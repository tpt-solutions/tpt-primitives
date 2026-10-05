//! Capability, lease and epoch primitives (spec §5–6).
//!
//! Capabilities represent authority. A capability is explicit,
//! transferable, attenuable, revocable, bound to an authority domain, and
//! optionally bound to a lease (temporal authority) and/or an epoch
//! (distributed invalidation):
//!
//! ```text
//! capability + lease + epoch → valid authority
//! ```
//!
//! Fabric owns runtime semantics; Concord can specify/prove capability
//! invariants over these canonical descriptions. After an epoch
//! transition, a capability bound to the old epoch must fail — enforced by
//! [`valid_authority`].
//!
//! Identity note: a capability's identity is derived from its immutable
//! [`CapabilityGrant`] only. Lifecycle state (revocation) is carried
//! separately in [`Capability::status`] so that revoking a capability does
//! not change its identity.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::error::CanonicalError;
use crate::ids::{CapabilityId, CapabilityTag, EpochId, EpochTag, LeaseId, LeaseTag};
use crate::time::LogicalTime;

/// A principal (holder) of authority. Non-empty opaque label; the
/// ecosystem maps principals to concrete actors at its boundary.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct Principal(String);

impl Principal {
    /// Create a principal, rejecting empty labels.
    pub fn new(label: impl Into<String>) -> Result<Self, CanonicalError> {
        let label = label.into();
        if label.is_empty() {
            return Err(CanonicalError::InvalidContent(
                "principal label must not be empty".into(),
            ));
        }
        Ok(Self(label))
    }

    /// The principal label.
    pub fn label(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Principal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// An authority domain (spec §5): the namespace a capability is bound to.
/// Non-empty opaque label.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct AuthorityDomain(String);

impl AuthorityDomain {
    /// Create an authority domain, rejecting empty labels.
    pub fn new(label: impl Into<String>) -> Result<Self, CanonicalError> {
        let label = label.into();
        if label.is_empty() {
            return Err(CanonicalError::InvalidContent(
                "authority domain must not be empty".into(),
            ));
        }
        Ok(Self(label))
    }

    /// The domain label.
    pub fn label(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AuthorityDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A single authorized action on a resource (spec §5). Capabilities carry
/// a set of grants; attenuation shrinks the set.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct Grant {
    /// The resource being authorized (opaque, ecosystem-defined).
    pub resource: String,
    /// The action authorized on the resource (opaque, ecosystem-defined).
    pub action: String,
}

impl Grant {
    /// Create a grant, rejecting empty resource/action labels.
    pub fn new(
        resource: impl Into<String>,
        action: impl Into<String>,
    ) -> Result<Self, CanonicalError> {
        let resource = resource.into();
        let action = action.into();
        if resource.is_empty() || action.is_empty() {
            return Err(CanonicalError::InvalidContent(
                "grant resource and action must not be empty".into(),
            ));
        }
        Ok(Self { resource, action })
    }
}

/// A lease: temporal authority over a domain (spec §6). Valid on
/// `[valid_from, expires_at)` in logical time (ADR 0002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Lease {
    /// Domain the lease governs.
    pub domain: AuthorityDomain,
    /// Principal the lease was issued to.
    pub holder: Principal,
    /// First logical tick at which the lease is valid (inclusive).
    pub valid_from: LogicalTime,
    /// First logical tick at which the lease has expired (exclusive).
    pub expires_at: LogicalTime,
}

impl Canonical for Lease {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Lease {
    type Id = LeaseTag;
}

impl Lease {
    /// Create a lease with explicit validity window; `expires_at` must be
    /// strictly after `valid_from`.
    pub fn new(
        domain: AuthorityDomain,
        holder: Principal,
        valid_from: LogicalTime,
        expires_at: LogicalTime,
    ) -> Result<Self, CanonicalError> {
        if !valid_from.is_before(expires_at) {
            return Err(CanonicalError::InvalidContent(format!(
                "lease expires_at {expires_at} must be after valid_from {valid_from}"
            )));
        }
        Ok(Self {
            domain,
            holder,
            valid_from,
            expires_at,
        })
    }

    /// True if the lease covers logical time `at`.
    pub fn covers(&self, at: LogicalTime) -> bool {
        self.valid_from.is_at_or_before(at) && at.is_before(self.expires_at)
    }
}

/// An epoch of a domain: a monotone invalidation counter (spec §6).
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct Epoch {
    /// Domain the epoch belongs to.
    pub domain: AuthorityDomain,
    /// Monotone counter; a transition increments it.
    pub counter: u64,
}

impl Canonical for Epoch {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Epoch {
    type Id = EpochTag;
}

impl Epoch {
    /// Create the initial epoch (counter 0) of a domain.
    pub fn initial(domain: AuthorityDomain) -> Self {
        Self { domain, counter: 0 }
    }

    /// The next epoch after a transition.
    pub fn transition(&self) -> Self {
        Self {
            domain: self.domain.clone(),
            counter: self.counter + 1,
        }
    }
}

/// Tracks the current epoch per authority domain so transitions can
/// invalidate capabilities (spec §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EpochBook {
    current: BTreeSet<Epoch>,
}

impl EpochBook {
    /// Insert (or replace) the current epoch of a domain.
    pub fn set_current(&mut self, epoch: Epoch) {
        self.current.retain(|e| e.domain != epoch.domain);
        self.current.insert(epoch);
    }

    /// The current epoch of a domain, if tracked.
    pub fn current(&self, domain: &AuthorityDomain) -> Option<&Epoch> {
        self.current.iter().find(|e| &e.domain == domain)
    }

    /// Perform an epoch transition for a domain, returning the new epoch.
    /// Any capability bound to the previous epoch now fails
    /// [`valid_authority`] (spec §6).
    pub fn transition(&mut self, domain: &AuthorityDomain) -> Result<Epoch, CanonicalError> {
        let next = match self.current(domain) {
            Some(current) => current.transition(),
            None => Epoch::initial(domain.clone()),
        };
        self.set_current(next.clone());
        Ok(next)
    }
}

/// Requirement that the *current* epoch of `domain` still be `epoch`.
/// After a transition the requirement can no longer be satisfied.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct EpochRequirement {
    /// The bound authority domain.
    pub domain: AuthorityDomain,
    /// The epoch the capability remains valid in.
    pub epoch: EpochId,
}

/// The immutable, identity-bearing content of a capability (spec §5).
///
/// Explicit, transferable, attenuable, bound to an authority domain, and
/// optionally bound to a lease and/or an epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CapabilityGrant {
    /// Authority domain this capability is bound to.
    pub domain: AuthorityDomain,
    /// Holder of the capability.
    pub holder: Principal,
    /// Authorized (resource, action) pairs. A set: canonical order is
    /// sorted, independent of construction order.
    pub grants: BTreeSet<Grant>,
    /// Whether the capability may be transferred or attenuated for
    /// another holder.
    pub transferable: bool,
    /// The capability this one was attenuated/transferred from.
    pub parent: Option<CapabilityId>,
    /// Optional lease binding (temporal authority, spec §6).
    pub lease: Option<LeaseId>,
    /// Optional epoch binding (distributed invalidation, spec §6).
    pub epoch_binding: Option<EpochRequirement>,
}

impl Canonical for CapabilityGrant {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for CapabilityGrant {
    type Id = CapabilityTag;
}

/// Lifecycle status of a capability, kept outside the identity preimage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum CapabilityStatus {
    /// Usable.
    Active,
    /// Permanently revoked (spec §5); revocation time for records.
    Revoked {
        /// Logical tick at which revocation was recorded.
        at: LogicalTime,
    },
}

/// A capability: an immutable grant plus its lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Capability {
    /// Immutable identity-bearing grant content.
    pub grant: CapabilityGrant,
    /// Lifecycle status (not part of the capability's identity).
    pub status: CapabilityStatus,
}

impl Capability {
    /// Mint a new root capability. `grants` must be non-empty.
    pub fn mint(
        domain: AuthorityDomain,
        holder: Principal,
        grants: impl IntoIterator<Item = Grant>,
        transferable: bool,
    ) -> Result<Self, CanonicalError> {
        let grants: BTreeSet<Grant> = grants.into_iter().collect();
        if grants.is_empty() {
            return Err(CanonicalError::InvalidContent(
                "a capability must carry at least one grant".into(),
            ));
        }
        Ok(Self {
            grant: CapabilityGrant {
                domain,
                holder,
                grants,
                transferable,
                parent: None,
                lease: None,
                epoch_binding: None,
            },
            status: CapabilityStatus::Active,
        })
    }

    /// The stable identity of this capability (its grant content).
    pub fn id(&self) -> CapabilityId {
        self.grant.identity()
    }

    /// Bind this capability to a lease (temporal authority).
    pub fn bound_to_lease(mut self, lease: LeaseId) -> Self {
        self.grant.lease = Some(lease);
        self
    }

    /// Bind this capability to an epoch requirement.
    pub fn bound_to_epoch(mut self, requirement: EpochRequirement) -> Self {
        self.grant.epoch_binding = Some(requirement);
        self
    }

    /// Revoke the capability; further use fails [`AuthorityError::Revoked`].
    pub fn revoke(&mut self, at: LogicalTime) {
        self.status = CapabilityStatus::Revoked { at };
    }

    /// Attenuate for the *same holder*: derive a child capability whose
    /// grants are the subset accepted by `keep`. Does not require
    /// [`CapabilityGrant::transferable`], since the holder is unchanged.
    pub fn attenuate(&self, keep: impl Fn(&Grant) -> bool) -> Result<Self, CapabilityError> {
        self.attenuate_inner(&self.grant.holder, keep)
    }

    /// Attenuate for a *different holder*: requires `transferable`.
    pub fn attenuate_for(
        &self,
        new_holder: &Principal,
        keep: impl Fn(&Grant) -> bool,
    ) -> Result<Self, CapabilityError> {
        if !self.grant.transferable {
            return Err(CapabilityError::NotTransferable(self.id()));
        }
        self.attenuate_inner(new_holder, keep)
    }

    /// Transfer verbatim to a new holder: requires `transferable`.
    pub fn transfer(&self, new_holder: &Principal) -> Result<Self, CapabilityError> {
        if !self.grant.transferable {
            return Err(CapabilityError::NotTransferable(self.id()));
        }
        if *new_holder == self.grant.holder {
            return Err(CapabilityError::SameHolder);
        }
        Ok(self.derive_child(new_holder.clone(), self.grant.grants.clone()))
    }

    fn attenuate_inner(
        &self,
        holder: &Principal,
        keep: impl Fn(&Grant) -> bool,
    ) -> Result<Self, CapabilityError> {
        if self.status != CapabilityStatus::Active {
            return Err(CapabilityError::NotActive(self.id()));
        }
        let grants: BTreeSet<Grant> = self
            .grant
            .grants
            .iter()
            .filter(|g| keep(g))
            .cloned()
            .collect();
        if grants.is_empty() {
            return Err(CapabilityError::EmptyAttenuation(self.id()));
        }
        Ok(self.derive_child(holder.clone(), grants))
    }

    fn derive_child(&self, holder: Principal, grants: BTreeSet<Grant>) -> Self {
        Self {
            grant: CapabilityGrant {
                domain: self.grant.domain.clone(),
                holder,
                grants,
                transferable: self.grant.transferable,
                parent: Some(self.id()),
                lease: self.grant.lease,
                epoch_binding: self.grant.epoch_binding.clone(),
            },
            status: CapabilityStatus::Active,
        }
    }
}

/// Errors from capability operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// The source capability is not active.
    NotActive(CapabilityId),
    /// The operation requires a transferable capability.
    NotTransferable(CapabilityId),
    /// Attenuation would remove every grant.
    EmptyAttenuation(CapabilityId),
    /// Transfer to the current holder.
    SameHolder,
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotActive(id) => write!(f, "capability {id} is not active"),
            Self::NotTransferable(id) => write!(f, "capability {id} is not transferable"),
            Self::EmptyAttenuation(id) => {
                write!(f, "attenuation of capability {id} would remove all grants")
            }
            Self::SameHolder => write!(f, "cannot transfer a capability to its current holder"),
        }
    }
}

impl std::error::Error for CapabilityError {}

/// Errors from the composed valid-authority check (spec §5–6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    /// The capability has been revoked.
    Revoked(CapabilityId),
    /// The capability requires a lease but none was supplied.
    LeaseRequired(CapabilityId),
    /// The supplied lease is not the one the capability is bound to.
    LeaseMismatch {
        /// The capability.
        capability: CapabilityId,
        /// The lease the capability requires.
        required: LeaseId,
    },
    /// The supplied lease is for a different domain or holder.
    LeaseForeign {
        /// Lease domain.
        domain: AuthorityDomain,
        /// Lease holder.
        holder: Principal,
    },
    /// Logical time `at` is before the lease becomes valid.
    LeaseNotYetValid {
        /// Lease start.
        valid_from: LogicalTime,
    },
    /// The lease has expired at logical time `at`.
    LeaseExpired {
        /// Evaluation time.
        at: LogicalTime,
        /// Expiry.
        expires_at: LogicalTime,
    },
    /// The capability requires an epoch binding but none was supplied.
    EpochRequired(CapabilityId),
    /// The current epoch of the bound domain does not match the
    /// requirement — the classic stale-authority failure after an epoch
    /// transition (spec §6).
    StaleEpoch {
        /// The required epoch.
        required: EpochId,
        /// The current epoch, if any.
        current: Option<EpochId>,
    },
}

impl fmt::Display for AuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Revoked(id) => write!(f, "capability {id} is revoked"),
            Self::LeaseRequired(id) => write!(f, "capability {id} requires a lease; none supplied"),
            Self::LeaseMismatch {
                capability,
                required,
            } => {
                write!(
                    f,
                    "capability {capability} is bound to lease {required}, a different lease was supplied"
                )
            }
            Self::LeaseForeign { domain, holder } => {
                write!(
                    f,
                    "lease belongs to domain {domain} holder {holder}, not to the capability's binding"
                )
            }
            Self::LeaseNotYetValid { valid_from } => {
                write!(f, "lease is not valid before {valid_from}")
            }
            Self::LeaseExpired { at, expires_at } => {
                write!(f, "lease expired at {expires_at}; now is {at}")
            }
            Self::EpochRequired(id) => {
                write!(f, "capability {id} requires an epoch; none supplied")
            }
            Self::StaleEpoch { required, current } => match current {
                Some(current) => write!(
                    f,
                    "capability requires epoch {required}, current epoch is {current}"
                ),
                None => write!(
                    f,
                    "capability requires epoch {required}, but the domain has no current epoch"
                ),
            },
        }
    }
}

impl std::error::Error for AuthorityError {}

/// The composed valid-authority check (spec §5–6):
/// `capability + lease + epoch → valid authority`.
///
/// A capability is valid authority at logical time `at` iff:
///
/// 1. it is [`CapabilityStatus::Active`],
/// 2. if bound to a lease, `lease` is that lease and covers `at`,
/// 3. if bound to an epoch, `current` is the current epoch of the bound
///    domain and still *is* the required epoch.
pub fn valid_authority(
    capability: &Capability,
    lease: Option<&Lease>,
    current: Option<&Epoch>,
    at: LogicalTime,
) -> Result<(), AuthorityError> {
    let grant = &capability.grant;
    if capability.status != CapabilityStatus::Active {
        return Err(AuthorityError::Revoked(grant.identity()));
    }

    if let Some(required) = grant.lease {
        let lease = lease.ok_or(AuthorityError::LeaseRequired(grant.identity()))?;
        if lease.identity() != required {
            return Err(AuthorityError::LeaseMismatch {
                capability: grant.identity(),
                required,
            });
        }
        if lease.domain != grant.domain || lease.holder != grant.holder {
            return Err(AuthorityError::LeaseForeign {
                domain: lease.domain.clone(),
                holder: lease.holder.clone(),
            });
        }
        if !lease.covers(at) {
            return Err(if at.is_before(lease.valid_from) {
                AuthorityError::LeaseNotYetValid {
                    valid_from: lease.valid_from,
                }
            } else {
                AuthorityError::LeaseExpired {
                    at,
                    expires_at: lease.expires_at,
                }
            });
        }
    }

    if let Some(requirement) = &grant.epoch_binding {
        if requirement.domain != grant.domain {
            return Err(AuthorityError::StaleEpoch {
                required: requirement.epoch,
                current: current.map(Epoch::identity),
            });
        }
        let current = current.ok_or(AuthorityError::EpochRequired(grant.identity()))?;
        if current.domain != requirement.domain || current.identity() != requirement.epoch {
            return Err(AuthorityError::StaleEpoch {
                required: requirement.epoch,
                current: Some(current.identity()),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical::Canonical;

    fn domain() -> AuthorityDomain {
        AuthorityDomain::new("fabric.test").unwrap()
    }

    fn alice() -> Principal {
        Principal::new("alice").unwrap()
    }

    fn bob() -> Principal {
        Principal::new("bob").unwrap()
    }

    fn root_capability() -> Capability {
        Capability::mint(
            domain(),
            alice(),
            [
                Grant::new("db:orders", "read").unwrap(),
                Grant::new("db:orders", "write").unwrap(),
                Grant::new("bucket:exports", "write").unwrap(),
            ],
            true,
        )
        .unwrap()
        .bound_to_epoch(EpochRequirement {
            domain: domain(),
            epoch: Epoch::initial(domain()).identity(),
        })
    }

    #[test]
    fn grant_insertion_order_does_not_change_identity() {
        let a = Capability::mint(
            domain(),
            alice(),
            [
                Grant::new("r1", "read").unwrap(),
                Grant::new("r2", "write").unwrap(),
            ],
            false,
        )
        .unwrap();
        let b = Capability::mint(
            domain(),
            alice(),
            [
                Grant::new("r2", "write").unwrap(),
                Grant::new("r1", "read").unwrap(),
            ],
            false,
        )
        .unwrap();
        assert_eq!(a.id(), b.id(), "grant set order is irrelevant presentation");
    }

    #[test]
    fn semantic_change_changes_identity() {
        let a = Capability::mint(
            domain(),
            alice(),
            [Grant::new("r1", "read").unwrap()],
            false,
        )
        .unwrap();
        let b = Capability::mint(
            domain(),
            alice(),
            [
                Grant::new("r1", "read").unwrap(),
                Grant::new("r2", "read").unwrap(),
            ],
            false,
        )
        .unwrap();
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn grant_round_trips_canonically() {
        let cap = root_capability();
        let bytes = cap.grant.canonical_bytes();
        let restored = CapabilityGrant::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(restored, cap.grant);
        assert_eq!(restored.identity(), cap.id());
    }

    #[test]
    fn attenuation_narrows_grants_and_records_parent() {
        let root = root_capability();
        let child = root
            .attenuate(|g| g.resource == "db:orders" && g.action == "read")
            .unwrap();
        assert_eq!(child.grant.grants.len(), 1);
        assert_eq!(child.grant.parent, Some(root.id()));
        assert_ne!(child.id(), root.id());
        assert_eq!(
            child.grant.epoch_binding, root.grant.epoch_binding,
            "bindings carry over"
        );
    }

    #[test]
    fn attenuation_requires_at_least_one_grant() {
        let root = root_capability();
        assert_eq!(
            root.attenuate(|_| false).unwrap_err(),
            CapabilityError::EmptyAttenuation(root.id())
        );
    }

    #[test]
    fn attenuation_of_revoked_capability_fails() {
        let mut root = root_capability();
        root.revoke(LogicalTime::new(9));
        assert_eq!(
            root.attenuate(|_| true).unwrap_err(),
            CapabilityError::NotActive(root.id())
        );
    }

    #[test]
    fn transfer_requires_transferable() {
        let root =
            Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], false).unwrap();
        assert_eq!(
            root.transfer(&bob()).unwrap_err(),
            CapabilityError::NotTransferable(root.id())
        );
        let transferable =
            Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], true).unwrap();
        let moved = transferable.transfer(&bob()).unwrap();
        assert_eq!(moved.grant.holder, bob());
        assert_eq!(moved.grant.parent, Some(transferable.id()));
    }

    #[test]
    fn transfer_to_same_holder_fails() {
        let root =
            Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], true).unwrap();
        assert_eq!(
            root.transfer(&alice()).unwrap_err(),
            CapabilityError::SameHolder
        );
    }

    #[test]
    fn revocation_fails_authority() {
        let mut cap = root_capability();
        let epoch = Epoch::initial(domain());
        assert!(valid_authority(&cap, None, Some(&epoch), LogicalTime::new(1)).is_ok());
        cap.revoke(LogicalTime::new(2));
        assert_eq!(
            valid_authority(&cap, None, Some(&epoch), LogicalTime::new(3)).unwrap_err(),
            AuthorityError::Revoked(cap.id())
        );
    }

    #[test]
    fn epoch_transition_invalidates_old_capability() {
        let mut book = EpochBook::default();
        let initial = book.transition(&domain()).unwrap();
        let cap = Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], false)
            .unwrap()
            .bound_to_epoch(EpochRequirement {
                domain: domain(),
                epoch: initial.identity(),
            });
        assert!(
            valid_authority(&cap, None, book.current(&domain()), LogicalTime::new(1)).is_ok(),
            "valid in the epoch it was bound to"
        );
        let next = book.transition(&domain()).unwrap();
        assert_ne!(next.identity(), initial.identity());
        assert_eq!(
            valid_authority(&cap, None, book.current(&domain()), LogicalTime::new(2)).unwrap_err(),
            AuthorityError::StaleEpoch {
                required: initial.identity(),
                current: Some(next.identity()),
            },
            "an old capability must fail after an epoch transition (spec section 6)"
        );
    }

    #[test]
    fn lease_windows_bound_authority() {
        let lease = Lease::new(
            domain(),
            alice(),
            LogicalTime::new(10),
            LogicalTime::new(20),
        )
        .unwrap();
        let cap = Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], false)
            .unwrap()
            .bound_to_lease(lease.identity());
        assert!(
            valid_authority(&cap, Some(&lease), None, LogicalTime::new(9)).is_err(),
            "before window"
        );
        assert!(
            valid_authority(&cap, Some(&lease), None, LogicalTime::new(10)).is_ok(),
            "first valid tick"
        );
        assert!(
            valid_authority(&cap, Some(&lease), None, LogicalTime::new(19)).is_ok(),
            "last valid tick"
        );
        assert!(
            valid_authority(&cap, Some(&lease), None, LogicalTime::new(20)).is_err(),
            "expiry is exclusive"
        );
    }

    #[test]
    fn lease_bound_capability_requires_its_lease() {
        let lease =
            Lease::new(domain(), alice(), LogicalTime::ZERO, LogicalTime::new(100)).unwrap();
        let cap = Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], false)
            .unwrap()
            .bound_to_lease(lease.identity());
        assert_eq!(
            valid_authority(&cap, None, None, LogicalTime::new(1)).unwrap_err(),
            AuthorityError::LeaseRequired(cap.id())
        );
        let other = Lease::new(domain(), bob(), LogicalTime::ZERO, LogicalTime::new(100)).unwrap();
        assert_eq!(
            valid_authority(&cap, Some(&other), None, LogicalTime::new(1)).unwrap_err(),
            AuthorityError::LeaseMismatch {
                capability: cap.id(),
                required: lease.identity(),
            }
        );
    }

    #[test]
    fn lease_and_epoch_compose() {
        let mut book = EpochBook::default();
        let epoch = book.transition(&domain()).unwrap();
        let lease = Lease::new(domain(), alice(), LogicalTime::ZERO, LogicalTime::new(50)).unwrap();
        let cap = Capability::mint(domain(), alice(), [Grant::new("r", "read").unwrap()], false)
            .unwrap()
            .bound_to_lease(lease.identity())
            .bound_to_epoch(EpochRequirement {
                domain: domain(),
                epoch: epoch.identity(),
            });
        assert!(
            valid_authority(
                &cap,
                Some(&lease),
                book.current(&domain()),
                LogicalTime::new(10)
            )
            .is_ok()
        );
        // Lease expires while the epoch is unchanged → fails on lease.
        let err = valid_authority(
            &cap,
            Some(&lease),
            book.current(&domain()),
            LogicalTime::new(50),
        )
        .unwrap_err();
        assert!(matches!(err, AuthorityError::LeaseExpired { .. }));
        // Epoch transitions while the lease is fresh → fails on epoch.
        book.transition(&domain()).unwrap();
        let err = valid_authority(
            &cap,
            Some(&lease),
            book.current(&domain()),
            LogicalTime::new(10),
        )
        .unwrap_err();
        assert!(matches!(err, AuthorityError::StaleEpoch { .. }));
    }

    #[test]
    fn lease_round_trips() {
        let lease =
            Lease::new(domain(), alice(), LogicalTime::new(3), LogicalTime::new(7)).unwrap();
        let restored = Lease::from_canonical_bytes(&lease.canonical_bytes()).unwrap();
        assert_eq!(restored, lease);
        assert_eq!(restored.identity(), lease.identity());
    }

    #[test]
    fn invalid_leases_rejected() {
        assert!(Lease::new(domain(), alice(), LogicalTime::new(7), LogicalTime::new(7)).is_err());
        assert!(Capability::mint(domain(), alice(), Vec::<Grant>::new(), false).is_err());
        assert!(AuthorityDomain::new("").is_err());
        assert!(Principal::new("").is_err());
    }
}
