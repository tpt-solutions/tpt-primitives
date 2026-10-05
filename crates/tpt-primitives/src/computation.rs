//! Computation and derivation primitives (spec §9–10).
//!
//! A computation is not merely a command: it is a canonical semantic
//! object describing operation, inputs, dependencies, environment,
//! platform, toolchain, resource policy and determinism policy (Repro
//! owns this primitive). A derivation is the relationship
//!
//! ```text
//! input artifacts + ComputationID → DerivationID → output artifact
//! ```
//!
//! that permits lineage and independent verification.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::capability::Principal;
use crate::environment::{EnvironmentSpec, PlatformSpec, ToolchainSpec};
use crate::error::CanonicalError;
use crate::ids::{
    ArtifactId, ComputationId, ComputationTag, DerivationId, DerivationTag, ExecutionId,
    ProvenanceTag,
};
use crate::resources::ResourceSet;
use crate::time::LogicalTime;
use crate::value::PrimitiveMap;

/// The operation a computation performs: an ecosystem-defined kind plus
/// canonical parameters. Kept open (rather than a closed enum) so
/// ecosystems can register operation kinds without forking the type;
/// parameters are `PrimitiveMap`s, so they stay deterministic.
#[derive(
    Debug,
    Clone,
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
pub struct Operation {
    /// Operation kind label, e.g. `"repro.build.rust"`.
    pub kind: String,
    /// Canonical operation parameters.
    pub parameters: PrimitiveMap,
}

impl Operation {
    /// Describe an operation, rejecting an empty kind.
    pub fn new(kind: impl Into<String>) -> Result<Self, CanonicalError> {
        let kind = kind.into();
        if kind.is_empty() {
            return Err(CanonicalError::InvalidContent(
                "operation kind must not be empty".into(),
            ));
        }
        Ok(Self {
            kind,
            parameters: PrimitiveMap::new(),
        })
    }

    /// Add a parameter (builder style).
    pub fn with(mut self, name: impl Into<String>, value: crate::value::PrimitiveValue) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }
}

/// What determinism guarantees the computation claims (spec §9).
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
pub enum DeterminismPolicy {
    /// Re-execution must produce byte-identical artifacts.
    BitIdentical,
    /// Re-execution must produce semantically equivalent artifacts
    /// (identity of outputs may differ).
    SemanticallyEquivalent,
}

/// The computation primitive (spec §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Computation {
    /// The operation to perform.
    pub operation: Operation,
    /// Input artifacts (content-addressed).
    pub inputs: BTreeSet<ArtifactId>,
    /// Other computations this one depends on.
    pub dependencies: BTreeSet<ComputationId>,
    /// Relevant environment variables.
    pub environment: EnvironmentSpec,
    /// Target platform.
    pub platform: PlatformSpec,
    /// Toolchain used to perform the operation.
    pub toolchain: ToolchainSpec,
    /// Resource limits/policy.
    pub resource_policy: ResourceSet,
    /// Claimed determinism policy.
    pub determinism_policy: DeterminismPolicy,
    /// Which parts of an execution world are semantically relevant to
    /// this computation (spec §16).
    pub world_relevance: crate::world::WorldRelevance,
}

impl Canonical for Computation {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Computation {
    type Id = ComputationTag;
}

impl Computation {
    /// Start building a computation.
    pub fn build(operation: Operation) -> ComputationBuilder {
        ComputationBuilder {
            computation: Computation {
                operation,
                inputs: BTreeSet::new(),
                dependencies: BTreeSet::new(),
                environment: EnvironmentSpec::new(),
                platform: PlatformSpec::new("linux", "x86_64"),
                toolchain: ToolchainSpec::new("rust", "stable"),
                resource_policy: ResourceSet::new(),
                determinism_policy: DeterminismPolicy::BitIdentical,
                world_relevance: crate::world::WorldRelevance::minimal(),
            },
        }
    }
}

/// Builder for [`Computation`]; construction order never affects identity.
#[derive(Debug, Clone)]
pub struct ComputationBuilder {
    computation: Computation,
}

impl ComputationBuilder {
    /// Add an input artifact.
    pub fn input(mut self, artifact: ArtifactId) -> Self {
        self.computation.inputs.insert(artifact);
        self
    }

    /// Add a computation dependency.
    pub fn dependency(mut self, computation: ComputationId) -> Self {
        self.computation.dependencies.insert(computation);
        self
    }

    /// Set the environment.
    pub fn environment(mut self, environment: EnvironmentSpec) -> Self {
        self.computation.environment = environment;
        self
    }

    /// Set the platform.
    pub fn platform(mut self, platform: PlatformSpec) -> Self {
        self.computation.platform = platform;
        self
    }

    /// Set the toolchain.
    pub fn toolchain(mut self, toolchain: ToolchainSpec) -> Self {
        self.computation.toolchain = toolchain;
        self
    }

    /// Set the resource policy.
    pub fn resource_policy(mut self, policy: ResourceSet) -> Self {
        self.computation.resource_policy = policy;
        self
    }

    /// Set the determinism policy.
    pub fn determinism_policy(mut self, policy: DeterminismPolicy) -> Self {
        self.computation.determinism_policy = policy;
        self
    }

    /// Set the world-relevance declaration (spec §16).
    pub fn world_relevance(mut self, relevance: crate::world::WorldRelevance) -> Self {
        self.computation.world_relevance = relevance;
        self
    }

    /// Finish the computation.
    pub fn finish(self) -> Computation {
        self.computation
    }
}

/// The identity-bearing core of a derivation (spec §10):
/// `input artifacts + ComputationID`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DerivationSpec {
    /// The computation performed.
    pub computation: ComputationId,
    /// The input artifacts consumed.
    pub inputs: BTreeSet<ArtifactId>,
}

impl Canonical for DerivationSpec {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for DerivationSpec {
    type Id = DerivationTag;
}

/// A derivation: the immutable spec plus the artifacts it produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Derivation {
    /// Immutable identity-bearing content (computation + inputs).
    pub spec: DerivationSpec,
    /// Output artifacts (content-addressed), recorded as they are produced.
    pub outputs: BTreeSet<ArtifactId>,
}

impl Canonical for Derivation {
    const SCHEMA_VERSION: u16 = 1;
}

impl Derivation {
    /// Open a derivation from its spec; outputs are filled in later.
    pub fn open(spec: DerivationSpec) -> Self {
        Self {
            spec,
            outputs: BTreeSet::new(),
        }
    }

    /// The stable derivation identity (spec content only).
    pub fn id(&self) -> DerivationId {
        self.spec.identity()
    }

    /// Record an output artifact.
    pub fn record_output(&mut self, artifact: ArtifactId) {
        self.outputs.insert(artifact);
    }
}

/// Provenance of an artifact (spec §10): how this artifact came to exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Provenance {
    /// The artifact being explained.
    pub artifact: ArtifactId,
    /// The derivation that produced it.
    pub derivation: DerivationId,
    /// The execution that ran the derivation, when applicable.
    pub execution: Option<ExecutionId>,
    /// Who/what produced the artifact.
    pub produced_by: Principal,
    /// Logical tick of production.
    pub produced_at: LogicalTime,
}

impl Canonical for Provenance {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Provenance {
    type Id = ProvenanceTag;
}

/// Content-address an artifact: its identity is the hash of its bytes
/// (spec §3: identity from canonical semantics, never filenames).
pub fn artifact_of_content(content: &[u8]) -> ArtifactId {
    ArtifactId::derive(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;
    use crate::resources::ResourceKind;
    use crate::value::PrimitiveValue;
    use crate::world::WorldRelevance;

    fn operation() -> Operation {
        Operation::new("repro.build.rust")
            .unwrap()
            .with("profile", PrimitiveValue::Text("release".into()))
    }

    #[test]
    fn same_computation_different_construction_order() {
        let a = Computation::build(operation())
            .input(artifact_of_content(b"in1"))
            .input(artifact_of_content(b"in2"))
            .environment(
                EnvironmentSpec::new()
                    .with("RUSTFLAGS", PrimitiveValue::Text("-C opt-level=3".into())),
            )
            .finish();
        let b = Computation::build(operation())
            .input(artifact_of_content(b"in2"))
            .input(artifact_of_content(b"in1"))
            .environment(
                EnvironmentSpec::new()
                    .with("RUSTFLAGS", PrimitiveValue::Text("-C opt-level=3".into())),
            )
            .finish();
        assert_eq!(a.identity(), b.identity());
    }

    #[test]
    fn semantic_changes_change_identity() {
        let base = Computation::build(operation()).finish();
        let different_platform = Computation::build(operation())
            .platform(PlatformSpec::new("linux", "aarch64"))
            .finish();
        let different_toolchain = Computation::build(operation())
            .toolchain(ToolchainSpec::new("rust", "1.90"))
            .finish();
        let different_inputs = Computation::build(operation())
            .input(artifact_of_content(b"x"))
            .finish();
        assert_ne!(base.identity(), different_platform.identity());
        assert_ne!(base.identity(), different_toolchain.identity());
        assert_ne!(base.identity(), different_inputs.identity());
    }

    #[test]
    fn computation_round_trips() {
        let c = Computation::build(operation())
            .dependency(Id::derive(b"dep"))
            .resource_policy(ResourceSet::new().with(ResourceKind::MemoryMiB, 512))
            .determinism_policy(DeterminismPolicy::SemanticallyEquivalent)
            .world_relevance(WorldRelevance::all())
            .finish();
        let restored = Computation::from_canonical_bytes(&c.canonical_bytes()).unwrap();
        assert_eq!(restored, c);
        assert_eq!(restored.identity(), c.identity());
    }

    #[test]
    fn derivation_identity_is_spec_only() {
        let spec = DerivationSpec {
            computation: Id::derive(b"comp"),
            inputs: BTreeSet::from([artifact_of_content(b"a"), artifact_of_content(b"b")]),
        };
        let mut d1 = Derivation::open(spec.clone());
        let mut d2 = Derivation::open(spec);
        assert_eq!(d1.id(), d2.id());
        d1.record_output(artifact_of_content(b"out1"));
        d2.record_output(artifact_of_content(b"out2"));
        assert_ne!(d1.outputs, d2.outputs);
        assert_eq!(
            d1.id(),
            d2.id(),
            "outputs are results, not identity content"
        );
    }

    #[test]
    fn artifact_identity_is_content_only() {
        assert_eq!(artifact_of_content(b"same"), artifact_of_content(b"same"));
        assert_ne!(artifact_of_content(b"a"), artifact_of_content(b"b"));
    }

    #[test]
    fn provenance_round_trips() {
        let p = Provenance {
            artifact: artifact_of_content(b"out"),
            derivation: Id::derive(b"deriv"),
            execution: Some(Id::derive(b"exec")),
            produced_by: Principal::new("repro-runner").unwrap(),
            produced_at: LogicalTime::new(42),
        };
        let restored = Provenance::from_canonical_bytes(&p.canonical_bytes()).unwrap();
        assert_eq!(restored, p);
        assert_eq!(restored.identity(), p.identity());
    }

    #[test]
    fn empty_operation_kind_rejected() {
        assert!(Operation::new("").is_err());
    }
}
