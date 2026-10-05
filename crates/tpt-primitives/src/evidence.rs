//! Evidence, claim and counterexample primitives (spec §13–15, §18, §24).
//!
//! Evidence is not a boolean: it is typed, scoped and addressable, with a
//! clearly distinguished evidence hierarchy (spec §13, §23). Claims are
//! machine-readable assurance statements that reference subjects,
//! properties, scopes, assumptions, obligations and supporting evidence,
//! and may depend on other claims — the Assurance Graph (spec §14, §18).
//!
//! The type system makes it mechanically difficult to overstate a claim
//! beyond its evidence (spec §24): [`Claim::assess`] computes the
//! assurance level each property is *actually* supported at and fails if
//! any required obligation is not met. Counterexamples represent failures
//! discovered through simulation, testing, fuzzing or proof uniformly
//! (spec §15), so they become regression/conformance artifacts rather
//! than disappearing into test logs.
//!
//! Concord owns these primitives.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::environment::{EnvironmentSpec, ToolchainSpec};
use crate::id::AnyId;
use crate::ids::{ClaimId, ClaimTag, CounterexampleTag, EvidenceId, EvidenceTag, TraceId, WorldId};
use crate::time::LogicalTime;

/// The typed evidence kinds (spec §13). Each carries its own
/// ecosystem-relevant metadata; result strength is mapped through
/// [`AssuranceLevel`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EvidenceKind {
    /// A formal proof, discharged by an external proof kernel.
    Proof {
        /// The external kernel trusted to have checked it (spec §19).
        kernel: String,
    },
    /// A model-checking run against a named model.
    ModelCheck {
        /// The model identifier.
        model: String,
    },
    /// A conformance run against a named suite.
    Conformance {
        /// The conformance suite identifier.
        suite: String,
    },
    /// A deterministic simulation run in a canonical execution world.
    Simulation {
        /// The world the simulation ran in.
        world: WorldId,
        /// Ticks simulated.
        ticks: u64,
    },
    /// A property-based test campaign.
    PropertyTest {
        /// Framework used.
        framework: String,
        /// Number of cases generated.
        cases: u64,
    },
    /// A fuzzing campaign.
    FuzzCampaign {
        /// Fuzzer used.
        fuzzer: String,
        /// Executions performed.
        executions: u64,
    },
    /// A differential check against a baseline implementation.
    DifferentialCheck {
        /// The baseline implementation identity.
        baseline: String,
    },
    /// A benchmark measurement (fixed-point: value plus unit).
    Benchmark {
        /// Metric name.
        metric: String,
        /// Measured value, scaled by `scale` to stay integral (ADR 0001).
        value: u64,
        /// Power of ten the value was scaled by.
        scale: u32,
        /// Unit of the metric, e.g. `"ms"`.
        unit: String,
    },
}

/// The result carried by an evidence object.
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
pub enum EvidenceResult {
    /// The property held under this evidence kind.
    Pass,
    /// The property was violated (a counterexample should exist).
    Fail,
    /// The campaign could not decide.
    Inconclusive,
}

/// An assumption a piece of evidence (or a claim) relies on. Assumptions
/// may themselves carry supporting evidence.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct Assumption {
    /// The assumed statement.
    pub statement: String,
    /// Evidence supporting the assumption, if any.
    pub evidence: Option<EvidenceId>,
}

impl Assumption {
    /// State an assumption without supporting evidence.
    pub fn of(statement: impl Into<String>) -> Self {
        Self {
            statement: statement.into(),
            evidence: None,
        }
    }
}

/// The evidence primitive (spec §13): typed, scoped and addressable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Evidence {
    /// What the evidence is about (any canonical object).
    pub subject: AnyId,
    /// The property/obligation evidenced.
    pub property: String,
    /// The environment the evidence was produced in.
    pub environment: EnvironmentSpec,
    /// The toolchain used to produce the evidence.
    pub toolchain: ToolchainSpec,
    /// Assumptions this evidence relies on.
    pub assumptions: BTreeSet<Assumption>,
    /// The kind of evidence and its metadata.
    pub kind: EvidenceKind,
    /// The result.
    pub result: EvidenceResult,
    /// Logical tick the evidence was produced (ADR 0002).
    pub produced_at: LogicalTime,
}

impl Canonical for Evidence {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Evidence {
    type Id = EvidenceTag;
}

/// The assurance hierarchy (spec §23, §24), ordered weakest → strongest.
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
pub enum AssuranceLevel {
    /// Nothing is known about the property.
    Unknown,
    /// Supported by benchmark measurement only.
    Benchmarked,
    /// Supported by a fuzzing campaign.
    FuzzTested,
    /// Supported by differential testing.
    DifferentiallyTested,
    /// Supported by property testing.
    PropertyTested,
    /// Supported by deterministic simulation.
    Simulated,
    /// Supported by a conformance suite.
    ConformanceTested,
    /// Supported by model checking.
    ModelChecked,
    /// Fully proven.
    FullyProven,
}

impl AssuranceLevel {
    /// The per-property status name from spec §24.
    pub const fn status_name(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Benchmarked => "benchmarked",
            Self::FuzzTested => "fuzz_tested",
            Self::DifferentiallyTested => "differentially_tested",
            Self::PropertyTested => "property_tested",
            Self::Simulated => "simulated",
            Self::ConformanceTested => "conformance_tested",
            Self::ModelChecked => "model_checked",
            Self::FullyProven => "fully_proven",
        }
    }

    /// The level a given evidence kind can support at best.
    pub const fn for_kind(kind: &EvidenceKind) -> Self {
        match kind {
            EvidenceKind::Proof { .. } => Self::FullyProven,
            EvidenceKind::ModelCheck { .. } => Self::ModelChecked,
            EvidenceKind::Conformance { .. } => Self::ConformanceTested,
            EvidenceKind::Simulation { .. } => Self::Simulated,
            EvidenceKind::PropertyTest { .. } => Self::PropertyTested,
            EvidenceKind::FuzzCampaign { .. } => Self::FuzzTested,
            EvidenceKind::DifferentialCheck { .. } => Self::DifferentiallyTested,
            EvidenceKind::Benchmark { .. } => Self::Benchmarked,
        }
    }
}

impl fmt::Display for AssuranceLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.status_name())
    }
}

/// An obligation a claim must discharge: a property held at (at least) a
/// given assurance level (spec §14 "required obligations").
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct Obligation {
    /// The property that must hold.
    pub property: String,
    /// Minimum assurance level required.
    pub min_level: AssuranceLevel,
}

impl Obligation {
    /// Require a property at a minimum level.
    pub fn requires(property: impl Into<String>, min_level: AssuranceLevel) -> Self {
        Self {
            property: property.into(),
            min_level,
        }
    }
}

/// The claim primitive (spec §14): a node in the Assurance Graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Claim {
    /// What the claim is about (any canonical object).
    pub subject: AnyId,
    /// The property asserted.
    pub property: String,
    /// The scope within which the property is asserted.
    pub scope: String,
    /// Assumptions the claim relies on.
    pub assumptions: BTreeSet<Assumption>,
    /// Obligations that must be discharged for this claim to stand.
    pub required_obligations: BTreeSet<Obligation>,
    /// Supporting evidence for this claim's own property.
    pub evidence: BTreeSet<EvidenceId>,
    /// Claims this claim depends on (the Assurance Graph, spec §18).
    pub depends_on: BTreeSet<ClaimId>,
}

impl Canonical for Claim {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Claim {
    type Id = ClaimTag;
}

/// Errors from claim assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssessmentError {
    /// Referenced evidence could not be resolved.
    MissingEvidence(EvidenceId),
    /// A referenced dependency claim could not be resolved.
    MissingClaim(ClaimId),
    /// The assurance graph contains a cycle.
    DependencyCycle(ClaimId),
    /// The claim asserts more than its evidence supports (spec §24).
    Overstated {
        /// Property whose support is insufficient.
        property: String,
        /// Level actually supported by the evidence.
        supported: AssuranceLevel,
        /// Level the claim requires.
        required: AssuranceLevel,
    },
}

impl fmt::Display for AssessmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEvidence(id) => write!(f, "evidence {id} not found"),
            Self::MissingClaim(id) => write!(f, "dependency claim {id} not found"),
            Self::DependencyCycle(id) => {
                write!(f, "assurance graph has a dependency cycle at {id}")
            }
            Self::Overstated {
                property,
                supported,
                required,
            } => write!(
                f,
                "claim overstates property {property:?}: evidence supports {supported}, obligation requires {required}"
            ),
        }
    }
}

impl std::error::Error for AssessmentError {}

/// Access to the evidence and claims referenced by a claim graph.
pub trait AssuranceResolver {
    /// Resolve an evidence object.
    fn evidence(&self, id: EvidenceId) -> Option<Evidence>;
    /// Resolve a claim object.
    fn claim(&self, id: ClaimId) -> Option<Claim>;
}

/// An in-memory [`AssuranceResolver`].
#[derive(Debug, Clone, Default)]
pub struct AssuranceStore {
    evidence: BTreeMap<EvidenceId, Evidence>,
    claims: BTreeMap<ClaimId, Claim>,
}

impl AssuranceStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Index an evidence object.
    pub fn put_evidence(&mut self, evidence: Evidence) {
        self.evidence.insert(evidence.identity(), evidence);
    }

    /// Index a claim.
    pub fn put_claim(&mut self, claim: Claim) {
        self.claims.insert(claim.identity(), claim);
    }
}

impl AssuranceResolver for AssuranceStore {
    fn evidence(&self, id: EvidenceId) -> Option<Evidence> {
        self.evidence.get(&id).cloned()
    }

    fn claim(&self, id: ClaimId) -> Option<Claim> {
        self.claims.get(&id).cloned()
    }
}

/// The outcome of assessing one claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimAssessment {
    /// Identity of the assessed claim.
    pub claim: ClaimId,
    /// Per-property assurance actually supported (spec §24: per-property
    /// status, never a blanket "verified"). Combines this claim's own
    /// evidence with what its dependencies support.
    pub supported: BTreeMap<String, AssuranceLevel>,
}

impl ClaimAssessment {
    /// The per-property status lines of spec §24, e.g.
    /// `"wal_replay: fully_proven"`.
    pub fn status_lines(&self) -> Vec<String> {
        self.supported
            .iter()
            .map(|(property, level)| format!("{property}: {}", level.status_name()))
            .collect()
    }
}

impl Claim {
    /// The stable identity of this claim.
    pub fn id(&self) -> ClaimId {
        self.identity()
    }

    /// The best assurance level the attached evidence actually supports
    /// for `property`: only `Pass` results with a matching subject and
    /// property count; the strongest kind wins (spec §24).
    pub fn supported_level<R: AssuranceResolver>(
        &self,
        property: &str,
        resolver: &R,
    ) -> AssuranceLevel {
        let mut best = AssuranceLevel::Unknown;
        for id in &self.evidence {
            let Some(evidence) = resolver.evidence(*id) else {
                continue;
            };
            if evidence.subject != self.subject || evidence.property != property {
                continue;
            }
            if evidence.result != EvidenceResult::Pass {
                continue;
            }
            let level = AssuranceLevel::for_kind(&evidence.kind);
            if level > best {
                best = level;
            }
        }
        best
    }

    /// Assess this claim *and its dependency closure* against the
    /// evidence store (spec §18, §24). Returns an error exactly when the
    /// claim would state more than the evidence supports — the
    /// overstatement guard is mechanical, not advisory.
    ///
    /// An obligation is discharged either by this claim's own evidence or
    /// by a dependency claim that supports the property at the required
    /// level; that is what makes the Assurance Graph composable.
    pub fn assess<R: AssuranceResolver>(
        &self,
        resolver: &R,
    ) -> Result<ClaimAssessment, AssessmentError> {
        let mut stack = BTreeSet::new();
        self.assess_inner(resolver, &mut stack)
    }

    fn assess_inner<R: AssuranceResolver>(
        &self,
        resolver: &R,
        stack: &mut BTreeSet<ClaimId>,
    ) -> Result<ClaimAssessment, AssessmentError> {
        let id = self.identity();
        if !stack.insert(id) {
            return Err(AssessmentError::DependencyCycle(id));
        }
        let result = self.assess_no_cycle(resolver, stack);
        stack.remove(&id);
        result
    }

    fn assess_no_cycle<R: AssuranceResolver>(
        &self,
        resolver: &R,
        stack: &mut BTreeSet<ClaimId>,
    ) -> Result<ClaimAssessment, AssessmentError> {
        let id = self.identity();
        // Dependencies assess first (spec §18); their supported levels
        // become available to discharge this claim's obligations.
        let mut dependency_levels: BTreeMap<String, AssuranceLevel> = BTreeMap::new();
        for dep_id in &self.depends_on {
            let dep = resolver
                .claim(*dep_id)
                .ok_or(AssessmentError::MissingClaim(*dep_id))?;
            let dep_assessment = dep.assess_inner(resolver, stack)?;
            for (property, level) in dep_assessment.supported {
                let entry = dependency_levels
                    .entry(property)
                    .or_insert(AssuranceLevel::Unknown);
                if level > *entry {
                    *entry = level;
                }
            }
        }

        // This claim's own evidence must resolve.
        for evidence_id in &self.evidence {
            if resolver.evidence(*evidence_id).is_none() {
                return Err(AssessmentError::MissingEvidence(*evidence_id));
            }
        }

        // Per-property support: the claim's own property plus every
        // obligation's property; the best of own evidence and
        // dependencies counts (spec §24).
        let mut properties: BTreeSet<String> = self
            .required_obligations
            .iter()
            .map(|o| o.property.clone())
            .collect();
        properties.insert(self.property.clone());

        let mut supported = BTreeMap::new();
        for property in properties {
            let own = self.supported_level(&property, resolver);
            let from_deps = dependency_levels
                .get(&property)
                .copied()
                .unwrap_or(AssuranceLevel::Unknown);
            supported.insert(property, own.max(from_deps));
        }

        // Mechanical overstatement guard (spec §24).
        for obligation in &self.required_obligations {
            let level = supported
                .get(&obligation.property)
                .copied()
                .unwrap_or(AssuranceLevel::Unknown);
            if level < obligation.min_level {
                return Err(AssessmentError::Overstated {
                    property: obligation.property.clone(),
                    supported: level,
                    required: obligation.min_level,
                });
            }
        }

        Ok(ClaimAssessment {
            claim: id,
            supported,
        })
    }
}

/// The counterexample primitive (spec §15): failures discovered through
/// simulation, testing, fuzzing or proof, represented uniformly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Counterexample {
    /// What failed (any canonical object).
    pub subject: AnyId,
    /// The specification that was violated.
    pub specification: AnyId,
    /// The property/obligation violated.
    pub property: String,
    /// The input or state that triggered the violation.
    pub input_state: crate::value::PrimitiveMap,
    /// The execution trace, when the violation was observed dynamically.
    pub trace: Option<TraceId>,
    /// The environment of the violation.
    pub environment: EnvironmentSpec,
    /// The observed violation description.
    pub observed_violation: String,
    /// Logical tick the counterexample was recorded.
    pub recorded_at: LogicalTime,
}

impl Canonical for Counterexample {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Counterexample {
    type Id = CounterexampleTag;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{AuthorityDomain, Principal};
    use crate::computation::{Computation, artifact_of_content};
    use crate::environment::EnvironmentSpec;
    use crate::id::Id;
    use crate::ids::ComputationTag;
    use crate::value::PrimitiveValue;

    fn subject_of_computation() -> AnyId {
        let c = Computation::build(crate::computation::Operation::new("op").unwrap()).finish();
        AnyId::from_id(&c.identity())
    }

    fn evidence_for(subject: &AnyId, property: &str, kind: EvidenceKind) -> Evidence {
        Evidence {
            subject: subject.clone(),
            property: property.into(),
            environment: EnvironmentSpec::new(),
            toolchain: crate::environment::ToolchainSpec::new("concord", "0.1"),
            assumptions: BTreeSet::new(),
            kind,
            result: EvidenceResult::Pass,
            produced_at: LogicalTime::new(1),
        }
    }

    fn fail_evidence_for(subject: &AnyId, property: &str) -> Evidence {
        let mut e = evidence_for(
            subject,
            property,
            EvidenceKind::FuzzCampaign {
                fuzzer: "cargo-fuzz".into(),
                executions: 1_000,
            },
        );
        e.result = EvidenceResult::Fail;
        e
    }

    #[test]
    fn evidence_round_trips_with_all_metadata() {
        let e = evidence_for(
            &subject_of_computation(),
            "no_data_loss",
            EvidenceKind::ModelCheck {
                model: "recovery-model".into(),
            },
        );
        let restored = Evidence::from_canonical_bytes(&e.canonical_bytes()).unwrap();
        assert_eq!(restored, e);
        assert_eq!(restored.identity(), e.identity());
    }

    #[test]
    fn evidence_identifies_subject_property_environment_toolchain_assumptions_result() {
        let subject = subject_of_computation();
        let mut e = evidence_for(
            &subject,
            "no_data_loss",
            EvidenceKind::Proof {
                kernel: "lean4".into(),
            },
        );
        e.assumptions.insert(Assumption {
            statement: "kernel soundness".into(),
            evidence: None,
        });
        assert_eq!(e.subject, subject);
        assert_eq!(e.property, "no_data_loss");
        assert!(!e.assumptions.is_empty());
        assert_eq!(e.result, EvidenceResult::Pass);
        assert!(matches!(e.kind, EvidenceKind::Proof { .. }));
    }

    #[test]
    fn hierarchy_orders_evidence_kinds() {
        let kinds = [
            EvidenceKind::Benchmark {
                metric: "latency".into(),
                value: 10,
                scale: 3,
                unit: "ms".into(),
            },
            EvidenceKind::FuzzCampaign {
                fuzzer: "f".into(),
                executions: 10,
            },
            EvidenceKind::DifferentialCheck {
                baseline: "b".into(),
            },
            EvidenceKind::PropertyTest {
                framework: "proptest".into(),
                cases: 10,
            },
            EvidenceKind::Simulation {
                world: Id::derive(b"w"),
                ticks: 10,
            },
            EvidenceKind::Conformance { suite: "s".into() },
            EvidenceKind::ModelCheck { model: "m".into() },
            EvidenceKind::Proof { kernel: "k".into() },
        ];
        let mut previous = AssuranceLevel::Unknown;
        for kind in kinds {
            let level = AssuranceLevel::for_kind(&kind);
            assert!(level > previous, "{level:?} should outrank {previous:?}");
            previous = level;
        }
    }

    #[test]
    fn supported_level_ignores_failures_and_foreign_subjects() {
        let subject = subject_of_computation();
        let other_subject = AnyId::from_id(&Id::<ComputationTag>::derive(b"other"));
        let mut store = AssuranceStore::new();
        // Fuzz pass: supports fuzz_tested.
        store.put_evidence(evidence_for(
            &subject,
            "no_data_loss",
            EvidenceKind::FuzzCampaign {
                fuzzer: "f".into(),
                executions: 10,
            },
        ));
        // A failing proof must not raise the level.
        let mut failed_proof = evidence_for(
            &subject,
            "no_data_loss",
            EvidenceKind::Proof { kernel: "k".into() },
        );
        failed_proof.result = EvidenceResult::Fail;
        store.put_evidence(failed_proof);
        // Evidence about a different subject must not count.
        store.put_evidence(evidence_for(
            &other_subject,
            "no_data_loss",
            EvidenceKind::Proof { kernel: "k".into() },
        ));

        let claim = Claim {
            subject: subject.clone(),
            property: "no_data_loss".into(),
            scope: "recovery path".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::from([Obligation::requires(
                "no_data_loss",
                AssuranceLevel::FuzzTested,
            )]),
            evidence: store.evidence.keys().copied().collect(),
            depends_on: BTreeSet::new(),
        };
        let assessment = claim.assess(&store).unwrap();
        assert_eq!(
            assessment.supported["no_data_loss"],
            AssuranceLevel::FuzzTested
        );
    }

    #[test]
    fn overstatement_is_mechanically_rejected() {
        let subject = subject_of_computation();
        let mut store = AssuranceStore::new();
        let fuzz = evidence_for(
            &subject,
            "no_data_loss",
            EvidenceKind::FuzzCampaign {
                fuzzer: "f".into(),
                executions: 10,
            },
        );
        store.put_evidence(fuzz.clone());

        // Requires full proof but only has fuzz evidence → rejected.
        let overstated = Claim {
            subject: subject.clone(),
            property: "no_data_loss".into(),
            scope: "all paths".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::from([Obligation::requires(
                "no_data_loss",
                AssuranceLevel::FullyProven,
            )]),
            evidence: BTreeSet::from([fuzz.identity()]),
            depends_on: BTreeSet::new(),
        };
        assert_eq!(
            overstated.assess(&store).unwrap_err(),
            AssessmentError::Overstated {
                property: "no_data_loss".into(),
                supported: AssuranceLevel::FuzzTested,
                required: AssuranceLevel::FullyProven,
            }
        );

        // Requires only fuzz-tested → accepted, and per-property status
        // reads exactly like spec §24.
        let honest = Claim {
            required_obligations: BTreeSet::from([Obligation::requires(
                "no_data_loss",
                AssuranceLevel::FuzzTested,
            )]),
            ..overstated
        };
        let assessment = honest.assess(&store).unwrap();
        assert_eq!(
            assessment.status_lines(),
            vec!["no_data_loss: fuzz_tested".to_string()]
        );
    }

    #[test]
    fn assurance_graph_composes_per_spec_section_18() {
        // Claim: "Transaction recovery is safe."
        //   depends on:
        //   Claim A: "WAL replay preserves committed records." (proof)
        //   Claim B: "Page checksum detects corruption under model X." (model check)
        //   Claim C: "Implementation conforms to recovery model." (conformance)
        let subject = subject_of_computation();

        let claim_a = Claim {
            subject: subject.clone(),
            property: "wal_replay_preserves_committed_records".into(),
            scope: "recovery".into(),
            assumptions: BTreeSet::from([Assumption::of("kernel soundness")]),
            required_obligations: BTreeSet::from([Obligation::requires(
                "wal_replay_preserves_committed_records",
                AssuranceLevel::FullyProven,
            )]),
            evidence: BTreeSet::new(),
            depends_on: BTreeSet::new(),
        };
        let claim_b = Claim {
            subject: subject.clone(),
            property: "page_checksum_detects_corruption".into(),
            scope: "storage".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::from([Obligation::requires(
                "page_checksum_detects_corruption",
                AssuranceLevel::ModelChecked,
            )]),
            evidence: BTreeSet::new(),
            depends_on: BTreeSet::new(),
        };
        let claim_c = Claim {
            subject: subject.clone(),
            property: "conforms_to_recovery_model".into(),
            scope: "implementation".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::from([Obligation::requires(
                "conforms_to_recovery_model",
                AssuranceLevel::ConformanceTested,
            )]),
            evidence: BTreeSet::new(),
            depends_on: BTreeSet::new(),
        };

        let top = Claim {
            subject: subject.clone(),
            property: "transaction_recovery_is_safe".into(),
            scope: "whole system".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::from([
                Obligation::requires(
                    "wal_replay_preserves_committed_records",
                    AssuranceLevel::FullyProven,
                ),
                Obligation::requires(
                    "page_checksum_detects_corruption",
                    AssuranceLevel::ModelChecked,
                ),
                Obligation::requires(
                    "conforms_to_recovery_model",
                    AssuranceLevel::ConformanceTested,
                ),
            ]),
            evidence: BTreeSet::new(),
            depends_on: BTreeSet::from([claim_a.id(), claim_b.id(), claim_c.id()]),
        };

        let mut store = AssuranceStore::new();

        // Each claim gets its own kind of evidence, wired explicitly.
        let proof = evidence_for(
            &subject,
            "wal_replay_preserves_committed_records",
            EvidenceKind::Proof {
                kernel: "lean4".into(),
            },
        );
        let model_check = evidence_for(
            &subject,
            "page_checksum_detects_corruption",
            EvidenceKind::ModelCheck {
                model: "page-model-x".into(),
            },
        );
        let conformance = evidence_for(
            &subject,
            "conforms_to_recovery_model",
            EvidenceKind::Conformance {
                suite: "recovery-conformance".into(),
            },
        );
        store.put_evidence(proof.clone());
        store.put_evidence(model_check.clone());
        store.put_evidence(conformance.clone());

        // The store holds the evidence-carrying versions; the top claim's
        // depends_on must reference exactly those versions' identities.
        let mut a_full = claim_a.clone();
        a_full.evidence.insert(proof.identity());
        let mut b_full = claim_b.clone();
        b_full.evidence.insert(model_check.identity());
        let mut c_full = claim_c.clone();
        c_full.evidence.insert(conformance.identity());
        let a_id = a_full.identity();
        let b_id = b_full.identity();
        let c_id = c_full.identity();
        store.put_claim(a_full);
        store.put_claim(b_full);
        store.put_claim(c_full);

        let mut top = top;
        top.depends_on = BTreeSet::from([a_id, b_id, c_id]);
        store.put_claim(top.clone());

        let assessment = top.assess(&store).unwrap();
        assert_eq!(
            assessment.supported["wal_replay_preserves_committed_records"],
            AssuranceLevel::FullyProven
        );
        assert_eq!(
            assessment.supported["page_checksum_detects_corruption"],
            AssuranceLevel::ModelChecked
        );

        // Now downgrade claim C to fuzz evidence: the top claim must fail
        // — the graph refuses to pretend the whole system is proven.
        let mut store2 = store.clone();
        let weak_evidence = evidence_for(
            &subject,
            "conforms_to_recovery_model",
            EvidenceKind::FuzzCampaign {
                fuzzer: "f".into(),
                executions: 1,
            },
        );
        let mut weak_c = claim_c.clone();
        weak_c.evidence = BTreeSet::from([weak_evidence.identity()]);
        store2.put_evidence(weak_evidence);
        store2.put_claim(weak_c.clone());
        let mut top2 = top.clone();
        top2.depends_on = BTreeSet::from([a_id, b_id, weak_c.identity()]);
        match top2.assess(&store2) {
            Err(AssessmentError::Overstated {
                property,
                supported,
                required,
            }) => {
                assert_eq!(property, "conforms_to_recovery_model");
                assert_eq!(supported, AssuranceLevel::FuzzTested);
                assert_eq!(required, AssuranceLevel::ConformanceTested);
            }
            other => panic!("expected overstatement error, got {other:?}"),
        }
    }

    #[test]
    fn dependency_cycles_are_detected() {
        let subject = subject_of_computation();
        let a = Claim {
            subject: subject.clone(),
            property: "a".into(),
            scope: "s".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::new(),
            evidence: BTreeSet::new(),
            depends_on: BTreeSet::new(),
        };
        // Content-addressed claims cannot literally contain their own id
        // (referencing it would change the content). A cycle therefore
        // arises through store-resolved references: x -> mid -> x.
        let mut x = a.clone();
        x.property = "x".into();
        let back_edge = Id::<ClaimTag>::derive(b"back-edge");
        x.depends_on = BTreeSet::from([back_edge]);
        let x_id = x.identity();
        let mut mid = a.clone();
        mid.property = "mid".into();
        mid.depends_on = BTreeSet::from([x_id]);
        let mut store = AssuranceStore::new();
        store.put_claim(x.clone());
        // The back edge: the id x references resolves to a claim that
        // depends on x again.
        store.claims.insert(back_edge, mid);
        assert!(matches!(
            x.assess(&store).unwrap_err(),
            AssessmentError::DependencyCycle(_)
        ));
        // A diamond (two claims depending on one subclaim) is legal.
        let mut base = a.clone();
        base.property = "base".into();
        let base_id = base.identity();
        let mut c = a.clone();
        c.property = "c".into();
        c.depends_on = BTreeSet::from([base_id]);
        let mut d = a.clone();
        d.property = "d".into();
        d.depends_on = BTreeSet::from([base_id]);
        let mut top = a.clone();
        top.property = "top".into();
        top.depends_on = BTreeSet::from([c.identity(), d.identity()]);
        store.put_claim(base);
        store.put_claim(c);
        store.put_claim(d);
        store.put_claim(top.clone());
        assert!(
            top.assess(&store).is_ok(),
            "diamond dependencies must not be flagged as cycles"
        );
    }

    #[test]
    fn missing_evidence_and_claims_are_errors() {
        let subject = subject_of_computation();
        let phantom = Id::<EvidenceTag>::derive(b"phantom");
        let store = AssuranceStore::new();
        let claim = Claim {
            subject: subject.clone(),
            property: "p".into(),
            scope: "s".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::new(),
            evidence: BTreeSet::from([phantom]),
            depends_on: BTreeSet::new(),
        };
        assert_eq!(
            claim.assess(&store).unwrap_err(),
            AssessmentError::MissingEvidence(phantom)
        );

        let phantom_claim = Id::<ClaimTag>::derive(b"phantom-claim");
        let claim2 = Claim {
            depends_on: BTreeSet::from([phantom_claim]),
            ..claim
        };
        assert_eq!(
            claim2.assess(&store).unwrap_err(),
            AssessmentError::MissingClaim(phantom_claim)
        );
    }

    #[test]
    fn failing_evidence_yields_counterexample_shape() {
        let subject = subject_of_computation();
        let mut store = AssuranceStore::new();
        store.put_evidence(fail_evidence_for(&subject, "no_data_loss"));
        let claim = Claim {
            subject,
            property: "no_data_loss".into(),
            scope: "recovery".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::from([Obligation::requires(
                "no_data_loss",
                AssuranceLevel::Unknown,
            )]),
            evidence: store.evidence.keys().copied().collect(),
            depends_on: BTreeSet::new(),
        };
        let assessment = claim.assess(&store).unwrap();
        assert_eq!(
            assessment.supported["no_data_loss"],
            AssuranceLevel::Unknown
        );

        // The failure becomes a first-class counterexample artifact.
        let cx = Counterexample {
            subject: claim.subject.clone(),
            specification: AnyId::from_id(&Id::<crate::ids::SpecificationTag>::derive(b"spec")),
            property: "no_data_loss".into(),
            input_state: BTreeMap::from([("crash_tick".into(), PrimitiveValue::Uint(1042))]),
            trace: Some(Id::derive(b"trace")),
            environment: EnvironmentSpec::new(),
            observed_violation: "committed record lost after crash at tick 1042".into(),
            recorded_at: LogicalTime::new(7),
        };
        let restored = Counterexample::from_canonical_bytes(&cx.canonical_bytes()).unwrap();
        assert_eq!(restored, cx);
        assert_eq!(restored.identity(), cx.identity());
    }

    #[test]
    fn evidence_subject_must_match_for_claim_use() {
        let subject = subject_of_computation();
        let other = AnyId::from_id(&Id::<crate::ids::SpecificationTag>::derive(
            b"other-subject",
        ));
        let mut store = AssuranceStore::new();
        let evidence = evidence_for(&other, "p", EvidenceKind::Proof { kernel: "k".into() });
        store.put_evidence(evidence.clone());
        let claim = Claim {
            subject,
            property: "p".into(),
            scope: "s".into(),
            assumptions: BTreeSet::new(),
            required_obligations: BTreeSet::new(),
            evidence: BTreeSet::from([evidence.identity()]),
            depends_on: BTreeSet::new(),
        };
        let assessment = claim.assess(&store).unwrap();
        assert_eq!(assessment.supported["p"], AssuranceLevel::Unknown);
    }

    #[test]
    fn unused_artifact_helper_still_valid() {
        // Cross-module sanity: evidence can reference artifacts.
        let artifact = artifact_of_content(b"binary");
        let subject = AnyId::from_id(&artifact);
        assert_eq!(subject.class, "tpt.artifact.v1");
        let _ = Principal::new("x").unwrap();
        let _ = AuthorityDomain::new("d").unwrap();
    }
}
