//! Deterministic world and reproduction primitives (spec §16–17).
//!
//! Fabric simulation and Repro execution both need controlled execution
//! environments. The canonical [`ExecutionWorld`] describes time model,
//! platform, resources, capabilities, filesystem/object state, network
//! policy, randomness policy, environment variables and external
//! dependencies. A computation states which parts of the world are
//! semantically relevant via [`WorldRelevance`] (spec §16).
//!
//! A [`ReproductionRequest`] means: recreate the derivation represented by
//! this `ComputationID` under this permitted world/policy — and the result
//! explicitly reports one of five outcomes (spec §17). This crate defines
//! the canonical shapes; running reproductions is Repro's job.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::environment::{EnvironmentSpec, PlatformSpec};
use crate::error::CanonicalError;
use crate::hash::sha256;
use crate::ids::{ArtifactId, CapabilityId, ComputationId, WorldId, WorldTag};
use crate::resources::ResourceSet;
use crate::time::LogicalTime;

/// How logical time advances in the world (spec §16 "time model",
/// ADR 0002). The world is deterministic: there is no wall clock.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", tag = "time_model")]
pub enum TimeModel {
    /// Logical ticks advance at a fixed rate, defined as ticks per second.
    Logical {
        /// How many ticks make up one simulated second.
        ticks_per_second: u64,
    },
}

/// What the world's randomness source provides (spec §16).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", tag = "randomness")]
pub enum RandomnessPolicy {
    /// A fixed seed: executions are bit-reproducible.
    Deterministic {
        /// The seed every execution starts from.
        seed: u64,
    },
    /// An entropy source: executions may differ run to run.
    EntropySource,
}

/// What network access the world grants (spec §16).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", tag = "network")]
pub enum NetworkPolicy {
    /// No network at all.
    Isolated,
    /// Loopback/local only.
    LocalOnly,
    /// Reachable hosts allowlist.
    Allowlist {
        /// The reachable hosts.
        hosts: BTreeSet<String>,
    },
    /// Unrestricted network.
    Unrestricted,
}

/// The canonical `ExecutionWorld` (spec §16): a controlled execution
/// environment with every dimension made explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExecutionWorld {
    /// How logical time advances.
    pub time_model: TimeModel,
    /// The platform computations run on.
    pub platform: PlatformSpec,
    /// Resources available to computations.
    pub resources: ResourceSet,
    /// Capabilities computations may exercise in this world.
    pub required_capabilities: BTreeSet<CapabilityId>,
    /// Filesystem/object state: path → content-addressed object.
    pub objects: BTreeMap<String, ArtifactId>,
    /// Network access policy.
    pub network: NetworkPolicy,
    /// Randomness policy.
    pub randomness: RandomnessPolicy,
    /// Environment variables visible to computations.
    pub env: EnvironmentSpec,
    /// External dependencies: name → content-addressed dependency object.
    pub external_dependencies: BTreeMap<String, ArtifactId>,
}

impl Canonical for ExecutionWorld {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for ExecutionWorld {
    type Id = WorldTag;
}

impl ExecutionWorld {
    /// Start building a world.
    pub fn build() -> ExecutionWorldBuilder {
        ExecutionWorldBuilder {
            world: ExecutionWorld {
                time_model: TimeModel::Logical {
                    ticks_per_second: 1_000,
                },
                platform: PlatformSpec::new("linux", "x86_64"),
                resources: ResourceSet::new(),
                required_capabilities: BTreeSet::new(),
                objects: BTreeMap::new(),
                network: NetworkPolicy::Isolated,
                randomness: RandomnessPolicy::Deterministic { seed: 0 },
                env: EnvironmentSpec::new(),
                external_dependencies: BTreeMap::new(),
            },
        }
    }
}

/// Builder for [`ExecutionWorld`].
#[derive(Debug, Clone)]
pub struct ExecutionWorldBuilder {
    world: ExecutionWorld,
}

impl ExecutionWorldBuilder {
    /// Set the time model.
    pub fn time_model(mut self, model: TimeModel) -> Self {
        self.world.time_model = model;
        self
    }

    /// Set the platform.
    pub fn platform(mut self, platform: PlatformSpec) -> Self {
        self.world.platform = platform;
        self
    }

    /// Set available resources.
    pub fn resources(mut self, resources: ResourceSet) -> Self {
        self.world.resources = resources;
        self
    }

    /// Require a capability to be present in the world.
    pub fn capability(mut self, capability: CapabilityId) -> Self {
        self.world.required_capabilities.insert(capability);
        self
    }

    /// Put a content-addressed object at a path.
    pub fn object(mut self, path: impl Into<String>, object: ArtifactId) -> Self {
        self.world.objects.insert(path.into(), object);
        self
    }

    /// Set the network policy.
    pub fn network(mut self, network: NetworkPolicy) -> Self {
        self.world.network = network;
        self
    }

    /// Set the randomness policy.
    pub fn randomness(mut self, randomness: RandomnessPolicy) -> Self {
        self.world.randomness = randomness;
        self
    }

    /// Set the environment.
    pub fn env(mut self, env: EnvironmentSpec) -> Self {
        self.world.env = env;
        self
    }

    /// Add an external dependency.
    pub fn external_dependency(mut self, name: impl Into<String>, reference: ArtifactId) -> Self {
        self.world
            .external_dependencies
            .insert(name.into(), reference);
        self
    }

    /// Finish the world.
    pub fn finish(self) -> ExecutionWorld {
        self.world
    }
}

/// Which parts of the world a computation declares semantically relevant
/// (spec §16). Irrelevant parts may vary across reproductions without
/// affecting equivalence.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
pub struct WorldRelevance {
    /// Logical time is semantically relevant.
    pub time_model: bool,
    /// Platform is semantically relevant.
    pub platform: bool,
    /// Available resources are semantically relevant.
    pub resources: bool,
    /// Capabilities are semantically relevant.
    pub capabilities: bool,
    /// Object state is semantically relevant.
    pub objects: bool,
    /// Network policy is semantically relevant.
    pub network: bool,
    /// Randomness policy is semantically relevant.
    pub randomness: bool,
    /// Environment variables are semantically relevant.
    pub env: bool,
    /// External dependencies are semantically relevant.
    pub external_dependencies: bool,
}

impl WorldRelevance {
    /// Everything relevant: reproductions must match the world exactly.
    pub fn all() -> Self {
        Self {
            time_model: true,
            platform: true,
            resources: true,
            capabilities: true,
            objects: true,
            network: true,
            randomness: true,
            env: true,
            external_dependencies: true,
        }
    }

    /// Only the platform is relevant (the minimal deterministic core).
    pub fn minimal() -> Self {
        Self {
            platform: true,
            ..Self::default()
        }
    }

    /// Project a world onto the relevant parts. Fields marked irrelevant
    /// become `None`; the projection's canonical bytes are suitable for
    /// equivalence digests.
    pub fn project(&self, world: &ExecutionWorld) -> RelevantWorld {
        RelevantWorld {
            time_model: self.time_model.then_some(world.time_model),
            platform: self.platform.then(|| world.platform.clone()),
            resources: self.resources.then(|| world.resources.clone()),
            capabilities: self
                .capabilities
                .then(|| world.required_capabilities.clone()),
            objects: self.objects.then(|| world.objects.clone()),
            network: self.network.then(|| world.network.clone()),
            randomness: self.randomness.then_some(world.randomness),
            env: self.env.then(|| world.env.clone()),
            external_dependencies: self
                .external_dependencies
                .then(|| world.external_dependencies.clone()),
        }
    }

    /// The canonical digest of the relevant view of `world`: two worlds
    /// have the same digest iff they agree on everything the computation
    /// declared relevant.
    pub fn digest(&self, world: &ExecutionWorld) -> [u8; 32] {
        let view = self.project(world);
        sha256(&view.canonical_bytes())
    }
}

/// The relevance-projected view of a world (spec §16). `None` marks a
/// dimension declared irrelevant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RelevantWorld {
    /// Time model, if relevant.
    pub time_model: Option<TimeModel>,
    /// Platform, if relevant.
    pub platform: Option<PlatformSpec>,
    /// Resources, if relevant.
    pub resources: Option<ResourceSet>,
    /// Capabilities, if relevant.
    pub capabilities: Option<BTreeSet<CapabilityId>>,
    /// Objects, if relevant.
    pub objects: Option<BTreeMap<String, ArtifactId>>,
    /// Network policy, if relevant.
    pub network: Option<NetworkPolicy>,
    /// Randomness policy, if relevant.
    pub randomness: Option<RandomnessPolicy>,
    /// Environment, if relevant.
    pub env: Option<EnvironmentSpec>,
    /// External dependencies, if relevant.
    pub external_dependencies: Option<BTreeMap<String, ArtifactId>>,
}

impl Canonical for RelevantWorld {
    const SCHEMA_VERSION: u16 = 1;
}

/// What may be substituted when re-executing a reproduction (spec §17
/// "reproduced with substitution").
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SubstitutionKind {
    /// A different (equivalent) platform was used.
    Platform,
    /// A different toolchain was used.
    Toolchain,
    /// Resources differed within policy.
    Resources,
    /// Environment variables differed within policy.
    Environment,
    /// An external dependency resolved to a permitted alternative.
    ExternalDependency,
}

/// Substitution policy of a reproduction request (spec §17).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", tag = "substitution_policy")]
pub enum SubstitutionPolicy {
    /// Only a bit-exact re-execution in an identical relevant world counts.
    ExactOnly,
    /// The listed substitutions may be reported instead of failing.
    Permitted {
        /// The permitted substitution kinds.
        kinds: BTreeSet<SubstitutionKind>,
    },
}

/// The reproduction request primitive (spec §17): recreate the derivation
/// represented by this `ComputationID` under this permitted world/policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReproductionRequest {
    /// The computation to recreate.
    pub computation: ComputationId,
    /// Worlds the reproduction may run in.
    pub permitted_worlds: BTreeSet<WorldId>,
    /// What substitutions are acceptable.
    pub substitution_policy: SubstitutionPolicy,
    /// The relevance declaration the equivalence check uses (usually the
    /// computation's own).
    pub relevance: WorldRelevance,
}

impl Canonical for ReproductionRequest {
    const SCHEMA_VERSION: u16 = 1;
}

/// The explicit five-way reproduction outcome classification (spec §17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "reproduction")]
pub enum ReproductionOutcome {
    /// Bit-identical artifacts in an equivalent relevant world.
    Reproduced {
        /// The artifacts of the reproduction.
        artifacts: BTreeSet<ArtifactId>,
    },
    /// The reproduction succeeded with declared substitutions.
    ReproducedWithSubstitution {
        /// What was substituted.
        substitutions: BTreeSet<SubstitutionKind>,
        /// The artifacts of the reproduction.
        artifacts: BTreeSet<ArtifactId>,
    },
    /// Semantically equivalent output, but not bit-identical.
    EquivalentButNonIdentical {
        /// The artifacts of the reproduction.
        artifacts: BTreeSet<ArtifactId>,
        /// How equivalence was established.
        note: String,
    },
    /// The reproduction failed.
    Failed {
        /// Ecosystem-defined reason.
        reason: String,
    },
    /// Equivalence could not be established either way.
    Indeterminate {
        /// Why no decision was possible.
        reason: String,
    },
}

impl ReproductionOutcome {
    /// True for the three success-shaped outcomes.
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Reproduced { .. }
                | Self::ReproducedWithSubstitution { .. }
                | Self::EquivalentButNonIdentical { .. }
        )
    }
}

/// Check whether a reproduction's substitutions are permitted by the
/// request's substitution policy. Helper for Repro's executor; the
/// classification itself remains Repro's decision.
pub fn substitutions_permitted(
    policy: &SubstitutionPolicy,
    used: &BTreeSet<SubstitutionKind>,
) -> Result<(), CanonicalError> {
    match policy {
        SubstitutionPolicy::ExactOnly => {
            if used.is_empty() {
                Ok(())
            } else {
                Err(CanonicalError::InvalidContent(format!(
                    "policy permits no substitutions but {used:?} were used"
                )))
            }
        }
        SubstitutionPolicy::Permitted { kinds: permitted } => {
            let unpermitted: Vec<&SubstitutionKind> =
                used.iter().filter(|s| !permitted.contains(s)).collect();
            if unpermitted.is_empty() {
                Ok(())
            } else {
                Err(CanonicalError::InvalidContent(format!(
                    "substitutions not permitted by policy: {unpermitted:?}"
                )))
            }
        }
    }
}

/// Logical-time window a reproduction must respect (convenience struct
/// for schedulers; logical ticks only, ADR 0002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReproductionWindow {
    /// Earliest tick the reproduction may start.
    pub not_before: LogicalTime,
    /// Tick the reproduction must be finished by.
    pub deadline: LogicalTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;
    use crate::ids::ComputationTag;
    use crate::resources::ResourceKind;
    use crate::value::PrimitiveValue;

    fn world(seed: u64) -> ExecutionWorld {
        ExecutionWorld::build()
            .time_model(TimeModel::Logical {
                ticks_per_second: 1_000,
            })
            .resources(ResourceSet::new().with(ResourceKind::CpuCores, 8))
            .randomness(RandomnessPolicy::Deterministic { seed })
            .network(NetworkPolicy::Allowlist {
                hosts: BTreeSet::from(["cache.internal".into()]),
            })
            .object(
                "/input/data.bin",
                crate::computation::artifact_of_content(b"data"),
            )
            .external_dependency(
                "toolchain",
                crate::computation::artifact_of_content(b"rust-1.97"),
            )
            .env(EnvironmentSpec::new().with("CI", PrimitiveValue::Bool(false)))
            .finish()
    }

    #[test]
    fn world_round_trips_and_has_stable_identity() {
        let w = world(42);
        let restored = ExecutionWorld::from_canonical_bytes(&w.canonical_bytes()).unwrap();
        assert_eq!(restored, w);
        assert_eq!(restored.identity(), w.identity());
        // Map insertion order does not affect identity.
        let w2 = ExecutionWorld::build()
            .object(
                "/input/data.bin",
                crate::computation::artifact_of_content(b"data"),
            )
            .external_dependency(
                "toolchain",
                crate::computation::artifact_of_content(b"rust-1.97"),
            )
            .finish();
        let w3 = ExecutionWorld::build()
            .external_dependency(
                "toolchain",
                crate::computation::artifact_of_content(b"rust-1.97"),
            )
            .object(
                "/input/data.bin",
                crate::computation::artifact_of_content(b"data"),
            )
            .finish();
        assert_eq!(w2.identity(), w3.identity());
    }

    #[test]
    fn world_identity_changes_on_relevant_state() {
        assert_ne!(world(1).identity(), world(2).identity(), "seed is semantic");
        let mut isolated = world(1);
        isolated.network = NetworkPolicy::Isolated;
        assert_ne!(isolated.identity(), world(1).identity());
    }

    #[test]
    fn relevance_digest_ignores_irrelevant_parts() {
        let relevance = WorldRelevance {
            platform: true,
            randomness: true,
            ..WorldRelevance::default()
        };
        let a = world(7);
        let mut b = world(7);
        b.resources = ResourceSet::new().with(ResourceKind::CpuCores, 64); // irrelevant
        assert_eq!(relevance.digest(&a), relevance.digest(&b));

        let mut c = world(8); // relevant: seed differs
        let _ = &mut c;
        assert_ne!(relevance.digest(&a), relevance.digest(&c));
    }

    #[test]
    fn all_relevance_is_exact_match() {
        let relevance = WorldRelevance::all();
        let a = world(1);
        let mut b = world(1);
        b.env
            .variables
            .insert("EXTRA".into(), PrimitiveValue::Text("1".into()));
        assert_ne!(relevance.digest(&a), relevance.digest(&b));
    }

    #[test]
    fn reproduction_request_round_trips() {
        let request = ReproductionRequest {
            computation: Id::<ComputationTag>::derive(b"comp"),
            permitted_worlds: BTreeSet::from([world(1).identity(), world(2).identity()]),
            substitution_policy: SubstitutionPolicy::Permitted {
                kinds: BTreeSet::from([SubstitutionKind::Platform]),
            },
            relevance: WorldRelevance::all(),
        };
        let restored =
            ReproductionRequest::from_canonical_bytes(&request.canonical_bytes()).unwrap();
        assert_eq!(restored, request);
    }

    #[test]
    fn outcome_classification_shape() {
        let artifacts = BTreeSet::from([crate::computation::artifact_of_content(b"out")]);
        let outcomes = [
            ReproductionOutcome::Reproduced {
                artifacts: artifacts.clone(),
            },
            ReproductionOutcome::ReproducedWithSubstitution {
                substitutions: BTreeSet::from([SubstitutionKind::Toolchain]),
                artifacts: artifacts.clone(),
            },
            ReproductionOutcome::EquivalentButNonIdentical {
                artifacts: artifacts.clone(),
                note: "same AST, different metadata".into(),
            },
            ReproductionOutcome::Failed {
                reason: "toolchain unavailable".into(),
            },
            ReproductionOutcome::Indeterminate {
                reason: "baseline artifact unavailable".into(),
            },
        ];
        let success_flags = [true, true, true, false, false];
        for (outcome, expected) in outcomes.iter().zip(success_flags) {
            assert_eq!(outcome.is_success(), expected);
            // All outcomes round-trip canonically.
            let bytes = crate::encoding::encode_envelope(1, outcome);
            let (v, back): (u16, ReproductionOutcome) =
                crate::encoding::decode_envelope(&bytes).unwrap();
            assert_eq!(v, 1);
            assert_eq!(&back, outcome);
        }
    }

    #[test]
    fn substitution_policy_enforced() {
        let exact = SubstitutionPolicy::ExactOnly;
        let none: BTreeSet<SubstitutionKind> = BTreeSet::new();
        assert!(substitutions_permitted(&exact, &none).is_ok());
        assert!(
            substitutions_permitted(&exact, &BTreeSet::from([SubstitutionKind::Platform])).is_err()
        );
        let permitted = SubstitutionPolicy::Permitted {
            kinds: BTreeSet::from([SubstitutionKind::Platform, SubstitutionKind::Toolchain]),
        };
        assert!(
            substitutions_permitted(&permitted, &BTreeSet::from([SubstitutionKind::Toolchain]))
                .is_ok()
        );
        assert!(
            substitutions_permitted(&permitted, &BTreeSet::from([SubstitutionKind::Resources]))
                .is_err()
        );
    }

    #[test]
    fn world_id_references_are_stable() {
        let w = world(3);
        let id: WorldId = w.identity();
        let restored = ExecutionWorld::from_canonical_bytes(&w.canonical_bytes()).unwrap();
        assert_eq!(restored.identity(), id);
    }
}
