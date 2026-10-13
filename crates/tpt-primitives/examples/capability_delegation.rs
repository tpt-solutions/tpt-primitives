//! Delegation: mint, attenuate, bind to a lease and epoch, then watch each
//! kind of invalidation fail with a specific reason.
//!
//! Run with: `cargo run -p tpt-primitives --example capability_delegation`

use tpt_primitives::Identified;
use tpt_primitives::capability::{
    AuthorityDomain, AuthorityError, Capability, Epoch, EpochBook, EpochRequirement, Grant, Lease,
    Principal, valid_authority, valid_authority_with_ancestors,
};
use tpt_primitives::time::LogicalTime;

fn report(label: &str, result: Result<(), AuthorityError>) {
    match result {
        Ok(()) => println!("{label:<28} valid"),
        Err(e) => println!("{label:<28} DENIED: {e}"),
    }
}

fn main() {
    let domain = AuthorityDomain::new("fabric.cluster-1").unwrap();
    let alice = Principal::new("alice").unwrap();
    let bob = Principal::new("bob").unwrap();

    let mut epochs = EpochBook::default();
    epochs.set_current(Epoch::initial(domain.clone()));
    let epoch = epochs.current(&domain).unwrap().clone();

    // Alice holds read+write, valid only in epoch 0.
    let root = Capability::mint(
        domain.clone(),
        alice.clone(),
        [
            Grant::new("db:orders", "read").unwrap(),
            Grant::new("db:orders", "write").unwrap(),
        ],
        true,
    )
    .unwrap()
    .bound_to_epoch(EpochRequirement {
        domain: domain.clone(),
        epoch: epoch.identity(),
    });

    // Alice hands Bob a read-only child. Authority can only shrink.
    let child = root.attenuate_for(&bob, |g| g.action == "read").unwrap();
    let t = LogicalTime::new(5);

    report(
        "child, fresh",
        valid_authority_with_ancestors(&child, &[&root], None, epochs.current(&domain), t),
    );

    // Revoking the parent kills the child too.
    let mut revoked_root = root.clone();
    revoked_root.revoke(t);
    report(
        "child, parent revoked",
        valid_authority_with_ancestors(&child, &[&revoked_root], None, epochs.current(&domain), t),
    );

    // An epoch transition invalidates everything bound to the old epoch.
    epochs.transition(&domain).unwrap();
    report(
        "root, after epoch change",
        valid_authority(&root, None, epochs.current(&domain), t),
    );

    // A lease limits authority in time.
    let lease = Lease::new(
        domain.clone(),
        alice.clone(),
        LogicalTime::ZERO,
        LogicalTime::new(10),
    )
    .unwrap();
    let leased = Capability::mint(domain, alice, [Grant::new("job", "run").unwrap()], false)
        .unwrap()
        .bound_to_lease(lease.identity());
    report(
        "leased, inside window",
        valid_authority(&leased, Some(&lease), None, t),
    );
    report(
        "leased, after expiry",
        valid_authority(&leased, Some(&lease), None, LogicalTime::new(10)),
    );
}
