//! Golden vector for a signed object (`testvectors/signed-v1.json`).
//!
//! Ed25519 is deterministic, so the signature is pinned too. Regenerate with
//! `TPT_BLESS=1 cargo test -p tpt-sign --test vectors` after a deliberate
//! change to the message construction or the envelope.

use std::path::PathBuf;

use serde_json::{Value, json};
use tpt_primitives::capability::{AuthorityDomain, Lease, Principal};
use tpt_primitives::hash::to_hex;
use tpt_primitives::time::LogicalTime;
use tpt_primitives::{Canonical, Identified};
use tpt_sign::{SIGNATURE_DOMAIN, Signer};

fn vector() -> Value {
    let lease = Lease::new(
        AuthorityDomain::new("fabric.cluster-1").unwrap(),
        Principal::new("alice").unwrap(),
        LogicalTime::ZERO,
        LogicalTime::new(10),
    )
    .unwrap();
    let signed = Signer::from_seed([7; 32]).sign(lease.clone());
    json!({
        "format": 1,
        "signature_domain": SIGNATURE_DOMAIN,
        "message": "signature_domain || 0x00 || identity_domain || 0x00 || identity",
        "seed_hex": to_hex(&[7u8; 32]),
        "object_domain": "tpt.lease.v1",
        "object_canonical_hex": to_hex(&lease.canonical_bytes()),
        "object_id": lease.identity().hex(),
        "signer": signed.signer.hex(),
        "signature": signed.signature.hex(),
        "signed_canonical_hex": to_hex(&signed.canonical_bytes()),
    })
}

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testvectors/signed-v1.json")
}

#[test]
fn signed_lease_matches_golden_vector() {
    let actual = vector();
    if std::env::var_os("TPT_BLESS").is_some() {
        std::fs::write(
            path(),
            serde_json::to_string_pretty(&actual).unwrap() + "\n",
        )
        .unwrap();
        return;
    }
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(path())
            .expect("testvectors/signed-v1.json missing; run with TPT_BLESS=1"),
    )
    .unwrap();
    assert_eq!(actual, expected);
}
