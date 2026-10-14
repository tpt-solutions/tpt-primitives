//! Golden vectors: canonical bytes and identities for representative
//! objects, checked in at `testvectors/v1.json`.
//!
//! Any change to the canonical encoding or an identity class changes these
//! values; the test then fails until the change is deliberate. After a
//! `SCHEMA_VERSION` bump (docs/evolution.md), regenerate with
//! `TPT_BLESS=1 cargo test -p tpt-primitives --test vectors`.
//!
//! Other-language implementations should reproduce every `canonical_hex`
//! and `id` in the file; `id` is
//! `SHA-256(domain || 0x00 || canonical_bytes)`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Value, json};
use tpt_primitives::Identified;
use tpt_primitives::capability::{AuthorityDomain, Capability, Epoch, Grant, Lease, Principal};
use tpt_primitives::computation::{Computation, Operation, artifact_of_content};
use tpt_primitives::environment::{EnvironmentSpec, PlatformSpec, ToolchainSpec};
use tpt_primitives::evidence::{
    AssuranceLevel, Claim, Evidence, EvidenceKind, EvidenceResult, Obligation, ScopeRequirement,
};
use tpt_primitives::hash::to_hex;
use tpt_primitives::id::{AnyId, IdClass};
use tpt_primitives::time::LogicalTime;
use tpt_primitives::value::PrimitiveValue;

fn entry<T: Identified>(name: &str, object: &T) -> Value {
    json!({
        "name": name,
        "domain": <T::Id as IdClass>::DOMAIN,
        "schema_version": T::SCHEMA_VERSION,
        "canonical_hex": to_hex(&object.canonical_bytes()),
        "id": object.identity().hex(),
    })
}

fn vectors() -> Value {
    let domain = AuthorityDomain::new("fabric.cluster-1").unwrap();
    let alice = Principal::new("alice").unwrap();

    let epoch = Epoch::initial(domain.clone());
    let lease = Lease::new(
        domain.clone(),
        alice.clone(),
        LogicalTime::ZERO,
        LogicalTime::new(10),
    )
    .unwrap();
    let capability = Capability::mint(
        domain,
        alice,
        [
            Grant::new("db:orders", "write").unwrap(),
            Grant::new("db:orders", "read").unwrap(),
        ],
        true,
    )
    .unwrap()
    .bound_to_lease(lease.identity());

    let computation = Computation::build(
        Operation::new("repro.build.rust")
            .unwrap()
            .with("target", PrimitiveValue::Text("release".into()))
            .with("jobs", PrimitiveValue::Uint(4)),
    )
    .input(artifact_of_content(b"source-v1"))
    .platform(PlatformSpec::new("linux", "x86_64"))
    .toolchain(ToolchainSpec::new("rust", "1.97.1"))
    .finish();

    let subject = AnyId::from_id(&artifact_of_content(b"binary-v1"));
    let evidence = Evidence {
        platform: Some(PlatformSpec::new("linux", "x86_64")),
        subject: subject.clone(),
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
    let claim = Claim {
        requirement: ScopeRequirement::default(),
        subject,
        property: "reproducible".into(),
        scope: "linux/x86_64".into(),
        assumptions: BTreeSet::new(),
        required_obligations: BTreeSet::from([Obligation::requires(
            "reproducible",
            AssuranceLevel::DifferentiallyTested,
        )]),
        evidence: BTreeSet::from([evidence.identity()]),
        depends_on: BTreeSet::new(),
    };

    json!({
        "format": 1,
        "hash": "sha256",
        "vectors": [
            entry("lease", &lease),
            entry("epoch", &epoch),
            entry("capability_grant", &capability.grant),
            entry("computation", &computation),
            entry("evidence", &evidence),
            entry("claim", &claim),
        ],
    })
}

fn path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testvectors/v1.json")
}

#[test]
fn canonical_bytes_and_identities_match_golden_vectors() {
    let actual = vectors();
    let rendered = serde_json::to_string_pretty(&actual).unwrap() + "\n";
    if std::env::var_os("TPT_BLESS").is_some() {
        std::fs::create_dir_all(path().parent().unwrap()).unwrap();
        std::fs::write(path(), &rendered).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(path())
        .expect("testvectors/v1.json missing; run with TPT_BLESS=1 to create it");
    let expected: Value = serde_json::from_str(&expected).unwrap();
    assert_eq!(
        actual, expected,
        "canonical encoding or identity changed; bump SCHEMA_VERSION and re-bless if intended"
    );
}
