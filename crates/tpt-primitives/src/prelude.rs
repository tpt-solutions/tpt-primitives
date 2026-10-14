//! One-line import of the types most programs need.
//!
//! ```
//! use tpt_primitives::prelude::*;
//!
//! let epoch = Epoch::initial(AuthorityDomain::new("fabric").unwrap());
//! let id: EpochId = epoch.identity();
//! assert_eq!(id, Epoch::from_canonical_bytes(&epoch.canonical_bytes()).unwrap().identity());
//! ```

pub use crate::canonical::{Canonical, Identified};
pub use crate::capability::{
    AuthorityDomain, Capability, Epoch, EpochBook, EpochRequirement, Grant, Lease, Principal,
    valid_authority, valid_authority_with_ancestors,
};
pub use crate::computation::{Computation, Derivation, Operation, Provenance, artifact_of_content};
pub use crate::environment::{EnvironmentSpec, PlatformSpec, ToolchainSpec};
pub use crate::error::CanonicalError;
pub use crate::evidence::{
    AssuranceLevel, AssuranceStore, Claim, Counterexample, Evidence, EvidenceKind, EvidenceResult,
    Obligation, ScopeRequirement,
};
pub use crate::execution::{Execution, ExecutionOutcome, Trace, TraceEvent};
pub use crate::id::{AnyId, Id};
pub use crate::ids::*;
pub use crate::intent::Intent;
pub use crate::reservation::{Reservation, ReservationSpec};
pub use crate::resources::{ResourceKind, ResourceSet};
pub use crate::time::LogicalTime;
pub use crate::value::PrimitiveValue;
pub use crate::world::ExecutionWorld;
