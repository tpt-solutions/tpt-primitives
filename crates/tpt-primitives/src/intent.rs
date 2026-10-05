//! Intent primitive (spec §7).
//!
//! An intent describes the desired computation *before* execution. It can
//! reference a Repro `ComputationId`, carries the capabilities it will
//! exercise, the resources it needs, where it should run, and what should
//! happen on failure. Fabric owns execution semantics; this type is the
//! canonical description Fabric and the ecosystem share.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::ids::{CapabilityId, ComputationId, IntentTag};
use crate::resources::ResourceSet;
use crate::time::LogicalTime;

/// Where an intent wants to run (spec §7 "locality").
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case", tag = "locality")]
pub enum Locality {
    /// Any node willing and able.
    Any,
    /// Within a named region.
    Region {
        /// Region label.
        name: String,
    },
    /// On a specific node.
    Node {
        /// Node label.
        name: String,
    },
}

/// Durability requirements for the intent's outputs (spec §7).
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
pub enum Durability {
    /// Outputs may live in volatile storage.
    Ephemeral,
    /// Outputs must be durably recorded.
    Persistent,
}

/// What to do when the computation fails (spec §7 "failure policy").
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
#[serde(rename_all = "snake_case", tag = "failure_policy")]
pub enum FailurePolicy {
    /// Fail the intent on first failure.
    FailFast,
    /// Retry up to `max_attempts` total attempts, waiting `backoff_ticks`
    /// between attempts.
    Retry {
        /// Total attempts allowed (>= 1).
        max_attempts: u32,
        /// Logical ticks to wait between attempts.
        backoff_ticks: u64,
    },
    /// Record the failure but do not propagate it as an intent failure.
    BestEffort,
}

/// The intent primitive (spec §7): a desired computation before execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Intent {
    /// The Repro computation this intent wants to execute (§7: "An Intent
    /// can reference a Repro `ComputationID`).
    pub computation: Option<ComputationId>,
    /// Capabilities the execution will exercise (spec §5 authority).
    pub capabilities: BTreeSet<CapabilityId>,
    /// Resources the execution needs.
    pub resources: ResourceSet,
    /// Where the execution should run.
    pub locality: Locality,
    /// Logical tick after which the intent must no longer start.
    pub deadline: Option<LogicalTime>,
    /// Output durability requirement.
    pub durability: Durability,
    /// Failure policy.
    pub failure_policy: FailurePolicy,
}

impl Canonical for Intent {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Intent {
    type Id = IntentTag;
}

impl Intent {
    /// Start building an intent.
    pub fn build() -> IntentBuilder {
        IntentBuilder {
            intent: Intent {
                computation: None,
                capabilities: BTreeSet::new(),
                resources: ResourceSet::new(),
                locality: Locality::Any,
                deadline: None,
                durability: Durability::Ephemeral,
                failure_policy: FailurePolicy::FailFast,
            },
        }
    }
}

/// Builder for [`Intent`] values; construction order never affects the
/// canonical encoding or identity (set fields are `BTreeSet`s).
#[derive(Debug, Clone)]
pub struct IntentBuilder {
    intent: Intent,
}

impl IntentBuilder {
    /// Reference the computation to execute.
    pub fn computation(mut self, id: ComputationId) -> Self {
        self.intent.computation = Some(id);
        self
    }

    /// Add a capability the execution will exercise.
    pub fn capability(mut self, id: CapabilityId) -> Self {
        self.intent.capabilities.insert(id);
        self
    }

    /// Set the resource requirements.
    pub fn resources(mut self, resources: ResourceSet) -> Self {
        self.intent.resources = resources;
        self
    }

    /// Set the locality.
    pub fn locality(mut self, locality: Locality) -> Self {
        self.intent.locality = locality;
        self
    }

    /// Set the deadline.
    pub fn deadline(mut self, at: LogicalTime) -> Self {
        self.intent.deadline = Some(at);
        self
    }

    /// Set the durability requirement.
    pub fn durability(mut self, durability: Durability) -> Self {
        self.intent.durability = durability;
        self
    }

    /// Set the failure policy.
    pub fn failure_policy(mut self, policy: FailurePolicy) -> Self {
        self.intent.failure_policy = policy;
        self
    }

    /// Finish the intent.
    pub fn finish(self) -> Intent {
        self.intent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ComputationTag;
    use crate::resources::ResourceKind;

    fn computation_id(seed: &[u8]) -> Id<ComputationTag> {
        Id::derive(seed)
    }

    use crate::id::Id;

    #[test]
    fn builder_order_does_not_change_identity() {
        let a = Intent::build()
            .computation(computation_id(b"c"))
            .locality(Locality::Region { name: "eu".into() })
            .durability(Durability::Persistent)
            .finish();
        let b = Intent::build()
            .durability(Durability::Persistent)
            .locality(Locality::Region { name: "eu".into() })
            .computation(computation_id(b"c"))
            .finish();
        assert_eq!(a, b);
        assert_eq!(a.identity(), b.identity());
    }

    #[test]
    fn deadline_changes_identity() {
        let a = Intent::build().finish();
        let b = Intent::build().deadline(LogicalTime::new(100)).finish();
        assert_ne!(a.identity(), b.identity());
    }

    #[test]
    fn intent_round_trips() {
        let intent = Intent::build()
            .computation(computation_id(b"c"))
            .capability(Id::derive(b"cap"))
            .resources(ResourceSet::new().with(ResourceKind::CpuCores, 4))
            .locality(Locality::Node {
                name: "node-7".into(),
            })
            .deadline(LogicalTime::new(500))
            .durability(Durability::Persistent)
            .failure_policy(FailurePolicy::Retry {
                max_attempts: 3,
                backoff_ticks: 5,
            })
            .finish();
        let restored = Intent::from_canonical_bytes(&intent.canonical_bytes()).unwrap();
        assert_eq!(restored, intent);
        assert_eq!(restored.identity(), intent.identity());
    }
}
