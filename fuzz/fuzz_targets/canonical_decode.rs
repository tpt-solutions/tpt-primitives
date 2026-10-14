//! Feed arbitrary bytes to the canonical decoders.
//!
//! Two properties must hold for every input:
//! 1. decoding never panics, and
//! 2. anything that decodes is exactly the canonical encoding of its value,
//!    so the bytes and the object can never disagree (ADR 0001, ADR 0003).
//!
//! Run with: `cargo +nightly fuzz run canonical_decode`

#![no_main]

use libfuzzer_sys::fuzz_target;
use tpt_primitives::capability::{CapabilityGrant, Lease};
use tpt_primitives::computation::Computation;
use tpt_primitives::evidence::{Claim, Evidence};
use tpt_primitives::Canonical;
use tpt_sign::Signed;

fn check<T: Canonical>(data: &[u8]) {
    if let Ok(value) = T::from_canonical_bytes(data) {
        assert_eq!(
            value.canonical_bytes(),
            data,
            "accepted bytes must be the canonical encoding"
        );
    }
}

fuzz_target!(|data: &[u8]| {
    check::<Lease>(data);
    check::<CapabilityGrant>(data);
    check::<Computation>(data);
    check::<Evidence>(data);
    check::<Claim>(data);
    check::<Signed<Lease>>(data);
});
