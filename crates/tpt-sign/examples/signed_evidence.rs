//! A build service signs evidence; a verifier checks it, and tampering is
//! caught both by `verify` and when decoding the envelope.
//!
//! Run with: `cargo run -p tpt-sign --example signed_evidence`

use std::collections::BTreeSet;

use tpt_primitives::Canonical;
use tpt_primitives::computation::artifact_of_content;
use tpt_primitives::environment::{EnvironmentSpec, ToolchainSpec};
use tpt_primitives::evidence::{Evidence, EvidenceKind, EvidenceResult};
use tpt_primitives::id::AnyId;
use tpt_primitives::time::LogicalTime;
use tpt_sign::{Signed, Signer};

fn main() {
    // In real use the seed comes from a key store, never from source.
    let build_service = Signer::from_seed([42; 32]);
    println!("signer key: {}", build_service.public_key());

    let evidence = Evidence {
        platform: None,
        subject: AnyId::from_id(&artifact_of_content(b"binary-v1")),
        property: "reproducible".into(),
        environment: EnvironmentSpec::new(),
        toolchain: ToolchainSpec::new("repro", "0.1"),
        assumptions: BTreeSet::new(),
        kind: EvidenceKind::DifferentialCheck {
            baseline: "build-a".into(),
        },
        result: EvidenceResult::Pass,
        produced_at: LogicalTime::new(1),
    };

    let signed = build_service.sign(evidence);
    signed.verify().expect("fresh signature verifies");
    println!(
        "signed:     ok ({} bytes on the wire)",
        signed.canonical_bytes().len()
    );

    // An attacker flips the result in transit.
    let mut forged = signed.clone();
    forged.object.result = EvidenceResult::Fail;
    println!("tampered:   {}", forged.verify().unwrap_err());

    // The same attack on the encoded form never becomes a value.
    let bytes = forged.canonical_bytes();
    println!(
        "decoding:   {}",
        Signed::<Evidence>::from_canonical_bytes(&bytes).unwrap_err()
    );
}
