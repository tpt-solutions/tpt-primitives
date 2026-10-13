//! Quickstart: define a computation, produce an artifact, and make a claim
//! that cannot say more than its evidence supports.
//!
//! Run with: `cargo run -p tpt-primitives --example quickstart`

use std::collections::BTreeSet;

use tpt_primitives::computation::{Computation, Operation, artifact_of_content};
use tpt_primitives::environment::{EnvironmentSpec, PlatformSpec, ToolchainSpec};
use tpt_primitives::evidence::{
    AssuranceLevel, AssuranceStore, Claim, Evidence, EvidenceKind, EvidenceResult, Obligation,
};
use tpt_primitives::id::AnyId;
use tpt_primitives::time::LogicalTime;
use tpt_primitives::{Canonical, Identified};

fn main() {
    // 1. A computation has a content-derived identity: the same definition
    //    gives the same ID on any machine.
    let computation = Computation::build(Operation::new("repro.build.rust").unwrap())
        .input(artifact_of_content(b"source-v1"))
        .platform(PlatformSpec::new("linux", "x86_64"))
        .toolchain(ToolchainSpec::new("rust", "1.97.1"))
        .finish();
    println!("computation  {}", computation.identity());
    println!("  canonical bytes: {}", computation.canonical_bytes().len());

    // 2. The artifact it produced is identified by its content.
    let artifact = artifact_of_content(b"binary-v1");
    println!("artifact     {artifact}");

    // 3. Evidence about the artifact, and claims that require a level.
    let subject = AnyId::from_id(&artifact);
    let evidence = Evidence {
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
    let claim = |level| Claim {
        subject: subject.clone(),
        property: "reproducible".into(),
        scope: "linux/x86_64, rust 1.97.1".into(),
        assumptions: BTreeSet::new(),
        required_obligations: BTreeSet::from([Obligation::requires("reproducible", level)]),
        evidence: BTreeSet::from([evidence.identity()]),
        depends_on: BTreeSet::new(),
    };

    let mut store = AssuranceStore::new();
    store.put_evidence(evidence.clone());

    // 4. An honest claim is assessed; an overstated one is rejected.
    let honest = claim(AssuranceLevel::DifferentiallyTested);
    println!(
        "honest claim:     {:?}",
        honest.assess(&store).unwrap().status_lines()
    );
    let overstated = claim(AssuranceLevel::FullyProven);
    println!(
        "overstated claim: {}",
        overstated.assess(&store).unwrap_err()
    );
}
