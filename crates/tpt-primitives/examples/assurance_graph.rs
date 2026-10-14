//! Assurance graph: a system claim is only as strong as the claims it
//! depends on, and a failing run refutes a property outright.
//!
//! Run with: `cargo run -p tpt-primitives --example assurance_graph`

use std::collections::BTreeSet;

use tpt_primitives::Identified;
use tpt_primitives::environment::{EnvironmentSpec, ToolchainSpec};
use tpt_primitives::evidence::{
    AssuranceLevel, AssuranceStore, Claim, Evidence, EvidenceKind, EvidenceResult, Obligation,
    ScopeRequirement,
};
use tpt_primitives::id::{AnyId, Id};
use tpt_primitives::ids::ComputationTag;
use tpt_primitives::time::LogicalTime;

fn evidence(
    subject: &AnyId,
    property: &str,
    kind: EvidenceKind,
    result: EvidenceResult,
) -> Evidence {
    Evidence {
        platform: None,
        subject: subject.clone(),
        property: property.into(),
        environment: EnvironmentSpec::new(),
        toolchain: ToolchainSpec::new("concord", "0.1"),
        assumptions: BTreeSet::new(),
        kind,
        result,
        produced_at: LogicalTime::new(1),
    }
}

fn main() {
    let wal = AnyId::from_id(&Id::<ComputationTag>::derive(b"wal"));
    let db = AnyId::from_id(&Id::<ComputationTag>::derive(b"database"));
    let mut store = AssuranceStore::new();

    // The WAL replay logic is proven.
    let proof = evidence(
        &wal,
        "replay_correct",
        EvidenceKind::Proof {
            kernel: "lean4".into(),
        },
        EvidenceResult::Pass,
    );
    let wal_claim = Claim {
        requirement: ScopeRequirement::default(),
        subject: wal.clone(),
        property: "replay_correct".into(),
        scope: "single node".into(),
        assumptions: BTreeSet::new(),
        required_obligations: BTreeSet::new(),
        evidence: BTreeSet::from([proof.identity()]),
        depends_on: BTreeSet::new(),
    };
    store.put_evidence(proof);
    store.put_claim(wal_claim.clone());

    // The database claim requires that property, discharged by the dependency.
    let db_claim = Claim {
        requirement: ScopeRequirement::default(),
        subject: db,
        property: "durable".into(),
        scope: "single node".into(),
        assumptions: BTreeSet::new(),
        required_obligations: BTreeSet::from([Obligation::requires(
            "replay_correct",
            AssuranceLevel::FullyProven,
        )]),
        evidence: BTreeSet::new(),
        depends_on: BTreeSet::from([wal_claim.id()]),
    };
    store.put_claim(db_claim.clone());
    println!(
        "before: {:?}",
        db_claim.assess(&store).unwrap().status_lines()
    );

    // A fuzz campaign finds a violation of the same property on the same
    // subject. Attaching it to the WAL claim refutes the proof.
    let failure = evidence(
        &wal,
        "replay_correct",
        EvidenceKind::FuzzCampaign {
            fuzzer: "afl".into(),
            executions: 1_000_000,
        },
        EvidenceResult::Fail,
    );
    let mut refuted = wal_claim;
    refuted.evidence.insert(failure.identity());
    store.put_evidence(failure);
    store.put_claim(refuted.clone());

    let db_claim = Claim {
        requirement: ScopeRequirement::default(),
        depends_on: BTreeSet::from([refuted.id()]),
        ..db_claim
    };
    store.put_claim(db_claim.clone());
    match db_claim.assess(&store) {
        Ok(a) => println!("after:  {:?}", a.status_lines()),
        Err(e) => println!("after:  rejected: {e}"),
    }
}
