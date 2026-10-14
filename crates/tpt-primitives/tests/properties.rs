//! Property tests for the canonical framework (spec §4).

use proptest::prelude::*;
use tpt_primitives::Canonical;
use tpt_primitives::capability::{AuthorityDomain, Capability, Grant, Lease, Principal};
use tpt_primitives::computation::Computation;
use tpt_primitives::encoding::{decode_envelope, encode_envelope};
use tpt_primitives::evidence::{Claim, Evidence};
use tpt_primitives::value::PrimitiveValue;

fn value() -> impl Strategy<Value = PrimitiveValue> {
    prop_oneof![
        ".*".prop_map(PrimitiveValue::Text),
        prop::collection::vec(any::<u8>(), 0..64).prop_map(PrimitiveValue::Bytes),
        any::<u64>().prop_map(PrimitiveValue::Uint),
        any::<i64>().prop_map(PrimitiveValue::int),
        any::<bool>().prop_map(PrimitiveValue::Bool),
        Just(PrimitiveValue::Null),
    ]
}

/// Bytes the decoder accepts must be exactly the canonical encoding of the
/// value they decode to; anything else must be an error, never a panic.
fn accepted_bytes_are_canonical<T: Canonical>(bytes: &[u8]) {
    if let Ok(value) = T::from_canonical_bytes(bytes) {
        assert_eq!(value.canonical_bytes(), bytes);
    }
}

proptest! {
    #[test]
    fn primitive_values_round_trip(v in value()) {
        let bytes = encode_envelope(1, &v);
        let (_, back): (u16, PrimitiveValue) = decode_envelope(&bytes).unwrap();
        prop_assert_eq!(back, v);
    }

    #[test]
    fn map_insertion_order_never_changes_bytes(
        entries in prop::collection::btree_map("[a-z]{1,8}", value(), 0..8),
    ) {
        let forward: Vec<_> = entries.iter().collect();
        let mut a = std::collections::BTreeMap::new();
        for (k, v) in &forward { a.insert((*k).clone(), (*v).clone()); }
        let mut b = std::collections::BTreeMap::new();
        for (k, v) in forward.iter().rev() { b.insert((*k).clone(), (*v).clone()); }
        prop_assert_eq!(encode_envelope(1, &a), encode_envelope(1, &b));
    }

    #[test]
    fn grant_order_never_changes_capability_identity(
        pairs in prop::collection::btree_set(("[a-z]{1,6}", "[a-z]{1,6}"), 1..6),
    ) {
        let grants: Vec<Grant> = pairs
            .iter()
            .map(|(r, a)| Grant::new(r.clone(), a.clone()).unwrap())
            .collect();
        let mint = |gs: Vec<Grant>| {
            Capability::mint(
                AuthorityDomain::new("d").unwrap(),
                Principal::new("p").unwrap(),
                gs,
                false,
            )
            .unwrap()
            .id()
        };
        let mut reversed = grants.clone();
        reversed.reverse();
        prop_assert_eq!(mint(grants), mint(reversed));
    }

    #[test]
    fn decoders_never_panic_and_accept_only_canonical_bytes(
        bytes in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        accepted_bytes_are_canonical::<Lease>(&bytes);
        accepted_bytes_are_canonical::<Computation>(&bytes);
        accepted_bytes_are_canonical::<Evidence>(&bytes);
        accepted_bytes_are_canonical::<Claim>(&bytes);
    }

    #[test]
    fn mutating_valid_bytes_never_yields_a_second_encoding(
        index in any::<prop::sample::Index>(),
        flip in 1u8..=255,
    ) {
        let lease = Lease::new(
            AuthorityDomain::new("d").unwrap(),
            Principal::new("p").unwrap(),
            tpt_primitives::time::LogicalTime::ZERO,
            tpt_primitives::time::LogicalTime::new(10),
        )
        .unwrap();
        let mut bytes = lease.canonical_bytes();
        let i = index.index(bytes.len());
        bytes[i] ^= flip;
        accepted_bytes_are_canonical::<Lease>(&bytes);
    }
}
