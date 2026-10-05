//! Reservation primitive (spec §8).
//!
//! A reservation is a first-class claim on resources/authority with an
//! explicit lifecycle:
//!
//! ```text
//! requested → reserved → committed → running → released
//! ```
//!
//! with explicit expiry and failure semantics. The reservation's identity
//! is derived from its immutable [`ReservationSpec`] only — the lifecycle
//! state lives beside it, so a reservation keeps its identity from
//! `requested` through `released`.

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::capability::Principal;
use crate::error::CanonicalError;
use crate::ids::{IntentId, ReservationTag};
use crate::resources::ResourceSet;
use crate::time::LogicalTime;

/// Lifecycle states of a reservation (spec §8).
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
pub enum ReservationState {
    /// The claim has been requested but not yet granted.
    Requested,
    /// Resources are held for the holder.
    Reserved,
    /// The holder has committed; resources are locked for execution.
    Committed,
    /// The execution is consuming the resources.
    Running,
    /// The execution finished and resources returned to the pool.
    Released,
    /// The reservation expired before it could be used.
    Expired,
    /// The reservation failed.
    Failed,
}

impl ReservationState {
    /// True for the three terminal states.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Released | Self::Expired | Self::Failed)
    }
}

/// The immutable identity-bearing content of a reservation: what was
/// claimed, for whom, until when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ReservationSpec {
    /// The intent this reservation claims resources for.
    pub intent: IntentId,
    /// The resources claimed.
    pub resources: ResourceSet,
    /// Who holds the claim.
    pub holder: Principal,
    /// Logical tick at which an unclaimed/uncommitted claim lapses.
    pub expires_at: LogicalTime,
}

impl Canonical for ReservationSpec {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for ReservationSpec {
    type Id = ReservationTag;
}

/// One recorded state transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StateTransition {
    /// State before the transition.
    pub from: ReservationState,
    /// State after the transition.
    pub to: ReservationState,
    /// Logical tick of the transition.
    pub at: LogicalTime,
}

/// A reservation: its immutable spec plus lifecycle state and history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Reservation {
    /// Immutable identity-bearing content.
    pub spec: ReservationSpec,
    /// Current lifecycle state (outside the identity).
    pub state: ReservationState,
    /// Recorded transitions, in order.
    pub history: Vec<StateTransition>,
    /// Reason, if the reservation reached [`ReservationState::Failed`].
    pub failure_reason: Option<String>,
}

/// Errors from reservation lifecycle operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationError {
    /// The requested transition is not part of the state machine.
    InvalidTransition {
        /// Current state.
        from: ReservationState,
        /// Attempted state.
        to: ReservationState,
    },
    /// An `Expired` transition was attempted before the spec's expiry.
    BeforeExpiry {
        /// The expiry in the spec.
        expires_at: LogicalTime,
        /// The attempted transition time.
        at: LogicalTime,
    },
    /// A failure reason is required (or was supplied for a non-failure
    /// transition).
    ReasonRequired,
}

impl std::fmt::Display for ReservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(f, "illegal reservation transition {from:?} → {to:?}")
            }
            Self::BeforeExpiry { expires_at, at } => {
                write!(
                    f,
                    "cannot expire at {at}: reservation expires at {expires_at}"
                )
            }
            Self::ReasonRequired => write!(f, "a failure reason is required"),
        }
    }
}

impl std::error::Error for ReservationError {}

impl Canonical for Reservation {
    const SCHEMA_VERSION: u16 = 1;
}

impl Reservation {
    /// Open a reservation in the `Requested` state.
    pub fn request(spec: ReservationSpec) -> Result<Self, CanonicalError> {
        if spec.expires_at.ticks() == 0 {
            return Err(CanonicalError::InvalidContent(
                "reservation must have a non-zero expiry".into(),
            ));
        }
        Ok(Self {
            spec,
            state: ReservationState::Requested,
            history: Vec::new(),
            failure_reason: None,
        })
    }

    /// The stable identity of this reservation (its spec content).
    pub fn id(&self) -> crate::ids::ReservationId {
        self.spec.identity()
    }

    /// Transition the reservation one step forward, enforcing the state
    /// machine of spec §8 (including expiry and failure paths).
    pub fn try_transition(
        &mut self,
        to: ReservationState,
        at: LogicalTime,
    ) -> Result<(), ReservationError> {
        let allowed = matches!(
            (self.state, to),
            (ReservationState::Requested, ReservationState::Reserved)
                | (ReservationState::Reserved, ReservationState::Committed)
                | (ReservationState::Committed, ReservationState::Running)
                | (ReservationState::Running, ReservationState::Released)
        );
        if to == ReservationState::Expired {
            if self.state.is_terminal()
                || matches!(
                    self.state,
                    ReservationState::Running | ReservationState::Committed
                )
            {
                return Err(ReservationError::InvalidTransition {
                    from: self.state,
                    to,
                });
            }
            if at.is_before(self.spec.expires_at) {
                return Err(ReservationError::BeforeExpiry {
                    expires_at: self.spec.expires_at,
                    at,
                });
            }
        } else if to == ReservationState::Failed {
            if self.state.is_terminal() {
                return Err(ReservationError::InvalidTransition {
                    from: self.state,
                    to,
                });
            }
        } else if !allowed {
            return Err(ReservationError::InvalidTransition {
                from: self.state,
                to,
            });
        }
        self.history.push(StateTransition {
            from: self.state,
            to,
            at,
        });
        self.state = to;
        Ok(())
    }

    /// Reserve previously requested resources.
    pub fn reserve(&mut self, at: LogicalTime) -> Result<(), ReservationError> {
        self.try_transition(ReservationState::Reserved, at)
    }

    /// Commit a reservation for execution.
    pub fn commit(&mut self, at: LogicalTime) -> Result<(), ReservationError> {
        self.try_transition(ReservationState::Committed, at)
    }

    /// Mark the execution as consuming the reservation.
    pub fn begin_running(&mut self, at: LogicalTime) -> Result<(), ReservationError> {
        self.try_transition(ReservationState::Running, at)
    }

    /// Release the reservation after a successful run.
    pub fn release(&mut self, at: LogicalTime) -> Result<(), ReservationError> {
        self.try_transition(ReservationState::Released, at)
    }

    /// Expire the reservation; only allowed at or after `expires_at` and
    /// only from `Requested` or `Reserved`.
    pub fn expire(&mut self, at: LogicalTime) -> Result<(), ReservationError> {
        self.try_transition(ReservationState::Expired, at)
    }

    /// Fail the reservation from any non-terminal state.
    pub fn fail(
        &mut self,
        reason: impl Into<String>,
        at: LogicalTime,
    ) -> Result<(), ReservationError> {
        let reason = reason.into();
        if reason.is_empty() {
            return Err(ReservationError::ReasonRequired);
        }
        self.try_transition(ReservationState::Failed, at)?;
        self.failure_reason = Some(reason);
        Ok(())
    }

    /// True if the reservation can no longer change state.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// True if the reservation's deadline has passed at `at` and it is
    /// still claimable (callers should treat it as expirable).
    pub fn is_expirable_at(&self, at: LogicalTime) -> bool {
        !self.state.is_terminal()
            && matches!(
                self.state,
                ReservationState::Requested | ReservationState::Reserved
            )
            && !at.is_before(self.spec.expires_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;
    use crate::ids::IntentTag;
    use crate::resources::ResourceKind;

    fn spec(expires_at: LogicalTime) -> ReservationSpec {
        ReservationSpec {
            intent: Id::<IntentTag>::derive(b"intent"),
            resources: ResourceSet::new().with(ResourceKind::CpuCores, 2),
            holder: Principal::new("alice").unwrap(),
            expires_at,
        }
    }

    #[test]
    fn full_lifecycle() {
        let mut r = Reservation::request(spec(LogicalTime::new(100))).unwrap();
        assert_eq!(r.state, ReservationState::Requested);
        r.reserve(LogicalTime::new(2)).unwrap();
        r.commit(LogicalTime::new(3)).unwrap();
        r.begin_running(LogicalTime::new(4)).unwrap();
        r.release(LogicalTime::new(5)).unwrap();
        assert!(r.is_terminal());
        // Opening the reservation is not a transition; the four forward
        // steps are recorded.
        assert_eq!(r.history.len(), 4);
        assert_eq!(r.history.last().unwrap().to, ReservationState::Released);
    }

    #[test]
    fn identity_is_stable_across_state_changes() {
        let mut r = Reservation::request(spec(LogicalTime::new(100))).unwrap();
        let id = r.id();
        r.reserve(LogicalTime::new(2)).unwrap();
        assert_eq!(r.id(), id, "lifecycle state is not part of the identity");
        r.fail("capacity lost", LogicalTime::new(3)).unwrap();
        assert_eq!(r.id(), id);
    }

    #[test]
    fn identity_changes_when_spec_changes() {
        let a = Reservation::request(spec(LogicalTime::new(100))).unwrap();
        let b = Reservation::request(spec(LogicalTime::new(101))).unwrap();
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn illegal_transitions_rejected() {
        let mut r = Reservation::request(spec(LogicalTime::new(100))).unwrap();
        // Cannot skip straight to running.
        assert_eq!(
            r.begin_running(LogicalTime::new(2)).unwrap_err(),
            ReservationError::InvalidTransition {
                from: ReservationState::Requested,
                to: ReservationState::Running,
            }
        );
        // Cannot release before running.
        assert!(r.release(LogicalTime::new(2)).is_err());
        // Terminal states are frozen.
        r.reserve(LogicalTime::new(2)).unwrap();
        r.commit(LogicalTime::new(3)).unwrap();
        r.begin_running(LogicalTime::new(4)).unwrap();
        r.release(LogicalTime::new(5)).unwrap();
        assert!(r.reserve(LogicalTime::new(6)).is_err());
        assert!(r.fail("late", LogicalTime::new(6)).is_err());
    }

    #[test]
    fn expiry_is_explicit() {
        let mut r = Reservation::request(spec(LogicalTime::new(100))).unwrap();
        // Not allowed before expiry.
        assert!(!r.is_expirable_at(LogicalTime::new(99)));
        assert_eq!(
            r.expire(LogicalTime::new(99)).unwrap_err(),
            ReservationError::BeforeExpiry {
                expires_at: LogicalTime::new(100),
                at: LogicalTime::new(99),
            }
        );
        assert!(r.is_expirable_at(LogicalTime::new(100)));
        r.expire(LogicalTime::new(100)).unwrap();
        assert_eq!(r.state, ReservationState::Expired);
        assert!(r.is_terminal());
    }

    #[test]
    fn expiry_only_from_requested_or_reserved() {
        let mut r = Reservation::request(spec(LogicalTime::new(10))).unwrap();
        r.reserve(LogicalTime::new(2)).unwrap();
        r.commit(LogicalTime::new(3)).unwrap();
        assert_eq!(
            r.expire(LogicalTime::new(20)).unwrap_err(),
            ReservationError::InvalidTransition {
                from: ReservationState::Committed,
                to: ReservationState::Expired,
            }
        );
    }

    #[test]
    fn failure_records_reason() {
        let mut r = Reservation::request(spec(LogicalTime::new(10))).unwrap();
        r.fail("node lost", LogicalTime::new(2)).unwrap();
        assert_eq!(r.state, ReservationState::Failed);
        assert_eq!(r.failure_reason.as_deref(), Some("node lost"));
        assert!(
            r.fail("again", LogicalTime::new(3)).is_err(),
            "terminal state is frozen"
        );
    }

    #[test]
    fn failure_requires_reason() {
        let mut r = Reservation::request(spec(LogicalTime::new(10))).unwrap();
        assert_eq!(
            r.fail("", LogicalTime::new(2)).unwrap_err(),
            ReservationError::ReasonRequired
        );
    }

    #[test]
    fn reservation_round_trips_canonically() {
        let mut r = Reservation::request(spec(LogicalTime::new(50))).unwrap();
        r.reserve(LogicalTime::new(2)).unwrap();
        let restored = Reservation::from_canonical_bytes(&r.canonical_bytes()).unwrap();
        assert_eq!(restored, r);
        assert_eq!(restored.id(), r.id());
    }
}
