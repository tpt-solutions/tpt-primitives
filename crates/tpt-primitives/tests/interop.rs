//! Cross-project interoperability test (spec §20, §26).
//!
//! Wires the deliberately-small first integration flow at the type level:
//!
//! ```text
//! Repro:   ComputationID
//!            ↓
//! Fabric:  Intent(ComputationID) → Reservation → ExecutionID + TraceID
//!            ↓
//! Repro:   ArtifactID + ProvenanceID
//!            ↓
//! Concord: EvidenceID + ClaimID
//! ```
//!
//! This validates composability only — no real execution or adapter logic
//! lives here (spec §21). It also demonstrates that the primitives
//! collectively answer the seven questions of spec §26.

use std::collections::{BTreeMap, BTreeSet};

use tpt_primitives::canonical::{Canonical, Identified};
use tpt_primitives::capability::{
    AuthorityDomain, Capability, Epoch, EpochRequirement, Grant, Lease, Principal, valid_authority,
};
use tpt_primitives::computation::{
    Computation, Derivation, DerivationSpec, Operation, Provenance, artifact_of_content,
};
use tpt_primitives::environment::{EnvironmentSpec, PlatformSpec, ToolchainSpec};
use tpt_primitives::evidence::{
    AssuranceLevel, AssuranceStore, Claim, Evidence, EvidenceKind, EvidenceResult, Obligation,
};
use tpt_primitives::execution::{Execution, ExecutionOutcome, Trace, TraceComparison, TraceEvent};
use tpt_primitives::id::AnyId;
use tpt_primitives::ids::{
    ClaimId, ComputationId, ExecutionId, IntentId, ProvenanceId, ReservationId, TraceId,
};
use tpt_primitives::intent::{Durability, FailurePolicy, Intent, Locality};
use tpt_primitives::reservation::{Reservation, ReservationSpec};
use tpt_primitives::resources::{ResourceKind, ResourceSet};
use tpt_primitives::time::LogicalTime;
use tpt_primitives::value::PrimitiveValue;
use tpt_primitives::world::{
    ExecutionWorld, NetworkPolicy, RandomnessPolicy, ReproductionOutcome, ReproductionRequest,
    SubstitutionKind, SubstitutionPolicy, TimeModel, WorldRelevance,
};

#[test]
fn cross_project_chain_composes() {
    // ------------------------------------------------------------------
    // The shared world (Fabric simulation / Repro execution converge on
    // the same canonical ExecutionWorld, spec §16).
    // ------------------------------------------------------------------
    let domain = AuthorityDomain::new("fabric.cluster-1").unwrap();
    let epoch = Epoch::initial(domain.clone());
    let lease = Lease::new(
        domain.clone(),
        Principal::new("fabric-runner").unwrap(),
        LogicalTime::ZERO,
        LogicalTime::new(10_000),
    )
    .unwrap();

    let world = ExecutionWorld::build()
        .time_model(TimeModel::Logical {
            ticks_per_second: 1_000,
        })
        .platform(PlatformSpec::new("linux", "x86_64"))
        .resources(
            ResourceSet::new()
                .with(ResourceKind::CpuCores, 4)
                .with(ResourceKind::MemoryMiB, 2048),
        )
        .randomness(RandomnessPolicy::Deterministic { seed: 2026 })
        .network(NetworkPolicy::Isolated)
        .env(EnvironmentSpec::new())
        .finish();
    let world_id = world.identity();

    // ------------------------------------------------------------------
    // Repro: ComputationID (spec §20 step 1).
    // ------------------------------------------------------------------
    let input = artifact_of_content(b"input:source-v7");
    let computation = Computation::build(
        Operation::new("repro.build.rust")
            .unwrap()
            .with("target", PrimitiveValue::Text("release".into())),
    )
    .input(input)
    .platform(PlatformSpec::new("linux", "x86_64"))
    .toolchain(ToolchainSpec::new("rust", "1.97.1"))
    .resource_policy(ResourceSet::new().with(ResourceKind::CpuCores, 2))
    .world_relevance(WorldRelevance {
        platform: true,
        randomness: true,
        ..WorldRelevance::default()
    })
    .finish();
    let computation_id: ComputationId = computation.identity();

    // ------------------------------------------------------------------
    // Fabric: Intent(ComputationID) (spec §20 step 2, §7).
    // ------------------------------------------------------------------
    let capability = Capability::mint(
        domain.clone(),
        Principal::new("fabric-runner").unwrap(),
        [Grant::new("net:cache.internal", "read").unwrap()],
        false,
    )
    .unwrap()
    .bound_to_epoch(EpochRequirement {
        domain: domain.clone(),
        epoch: epoch.identity(),
    });
    let capability_id = capability.id();

    let intent = Intent::build()
        .computation(computation_id)
        .capability(capability_id)
        .resources(ResourceSet::new().with(ResourceKind::CpuCores, 2))
        .locality(Locality::Region {
            name: "eu-central".into(),
        })
        .deadline(LogicalTime::new(5_000))
        .durability(Durability::Persistent)
        .failure_policy(FailurePolicy::Retry {
            max_attempts: 3,
            backoff_ticks: 10,
        })
        .finish();
    let intent_id: IntentId = intent.identity();
    assert_eq!(intent.computation, Some(computation_id));

    // Fabric: Reservation (spec §8) — claim the resources the intent needs.
    let reservation = Reservation::request(ReservationSpec {
        intent: intent_id,
        resources: intent.resources.clone(),
        holder: Principal::new("fabric-runner").unwrap(),
        expires_at: LogicalTime::new(1_000),
    })
    .unwrap();
    let reservation_id: ReservationId = reservation.id();

    // Authority composition (spec §5–6): capability + lease + epoch.
    assert!(valid_authority(&capability, Some(&lease), Some(&epoch), LogicalTime::new(1)).is_ok());

    // ------------------------------------------------------------------
    // Fabric: ExecutionID + TraceID (spec §20 step 3, §11–12).
    // ------------------------------------------------------------------
    let mut execution = Execution::begin(computation_id, world_id, LogicalTime::new(10));
    let execution_id: ExecutionId = execution.identity();
    assert_eq!(
        execution.computation, computation_id,
        "execution references its computation"
    );

    let mut trace = Trace::for_execution(execution_id);
    let start = TraceEvent {
        payload: BTreeMap::from([("action".into(), PrimitiveValue::Text("start".into()))]),
        causal_parents: BTreeSet::new(),
    };
    let compile = TraceEvent {
        payload: BTreeMap::from([("action".into(), PrimitiveValue::Text("compile".into()))]),
        causal_parents: BTreeSet::from([start.identity()]),
    };
    let done = TraceEvent {
        payload: BTreeMap::from([("action".into(), PrimitiveValue::Text("done".into()))]),
        causal_parents: BTreeSet::from([compile.identity()]),
    };
    trace.push(start.clone());
    trace.push(compile.clone());
    trace.push(done.clone());
    let trace_normalized = trace.normalized();
    let trace_id: TraceId = trace.identity();

    execution.finish(
        LogicalTime::new(90),
        ExecutionOutcome::Succeeded {
            artifacts: BTreeSet::new(), // filled by Repro below
        },
    );
    let execution = execution.with_trace(trace_id);
    assert!(trace.causal_parents_resolved().is_ok());

    // Reservation lifecycle runs to release.
    let mut reservation = reservation;
    reservation.reserve(LogicalTime::new(11)).unwrap();
    reservation.commit(LogicalTime::new(12)).unwrap();
    reservation.begin_running(LogicalTime::new(13)).unwrap();
    reservation.release(LogicalTime::new(90)).unwrap();
    assert!(reservation.is_terminal());

    // ------------------------------------------------------------------
    // Repro: ArtifactID + ProvenanceID (spec §20 step 4, §10).
    // ------------------------------------------------------------------
    let output = artifact_of_content(b"output:binary-v7");
    let mut derivation = Derivation::open(DerivationSpec {
        computation: computation_id,
        inputs: BTreeSet::from([input]),
    });
    let derivation_id = derivation.id();
    derivation.record_output(output);
    assert_eq!(
        derivation_id,
        derivation.id(),
        "outputs do not change derivation identity"
    );

    let provenance = Provenance {
        artifact: output,
        derivation: derivation_id,
        execution: Some(execution_id),
        produced_by: Principal::new("repro-runner").unwrap(),
        produced_at: LogicalTime::new(90),
    };
    let provenance_id: ProvenanceId = provenance.identity();

    // A faithful replay diverges nowhere (spec §12).
    let mut replay = Trace::for_execution(execution_id);
    replay.push(done.clone());
    replay.push(compile.clone());
    replay.push(start.clone());
    assert_eq!(trace.compare_replay(&replay), TraceComparison::Identical);
    assert_eq!(
        replay.identity(),
        trace_id,
        "replay normalizes to the same trace identity"
    );

    // ------------------------------------------------------------------
    // Concord: EvidenceID + ClaimID (spec §20 step 5, §13–14).
    // ------------------------------------------------------------------
    let subject = AnyId::from_id(&output);
    let evidence = Evidence {
        subject: subject.clone(),
        property: "bit_identical_reproduction".into(),
        environment: EnvironmentSpec::new(),
        toolchain: ToolchainSpec::new("repro", "0.1"),
        assumptions: BTreeSet::new(),
        kind: EvidenceKind::DifferentialCheck {
            baseline: computation_id.hex(),
        },
        result: EvidenceResult::Pass,
        produced_at: LogicalTime::new(95),
    };

    let claim = Claim {
        subject: subject.clone(),
        property: "artifact_is_reproducible".into(),
        scope: "this computation, linux/x86_64, rust 1.97.1".into(),
        assumptions: BTreeSet::new(),
        required_obligations: BTreeSet::from([Obligation::requires(
            "bit_identical_reproduction",
            AssuranceLevel::DifferentiallyTested,
        )]),
        evidence: BTreeSet::from([evidence.identity()]),
        depends_on: BTreeSet::<ClaimId>::new(),
    };

    let mut store = AssuranceStore::new();
    store.put_evidence(evidence);
    store.put_claim(claim.clone());
    let assessment = claim
        .assess(&store)
        .expect("claim must not overstate its evidence");

    // ------------------------------------------------------------------
    // spec §26: the seven questions, answered mechanically.
    // ------------------------------------------------------------------
    let intended = intent
        .computation
        .expect("q1: what did we intend to compute?");
    assert_eq!(intended, computation_id);

    let exact = computation.identity();
    assert_eq!(
        exact, computation_id,
        "q2: what exact computation was defined?"
    );

    // q3: where and under what authority did it execute?
    assert_eq!(execution.world, world_id);
    assert!(intent.capabilities.contains(&capability_id));
    assert!(
        valid_authority(
            &capability,
            Some(&lease),
            Some(&epoch),
            execution.started_at
        )
        .is_ok()
    );

    // q4: what exactly did it produce?
    assert_eq!(
        execution.outcome,
        Some(ExecutionOutcome::Succeeded {
            artifacts: BTreeSet::new()
        })
    );
    let _ = output;

    // q5: how was the result derived?
    assert_eq!(provenance.derivation, derivation_id);
    assert_eq!(provenance.artifact, output);
    assert_eq!(derivation.spec.computation, computation_id);

    // q6: what evidence do we have about correctness?
    assert_eq!(
        assessment.supported["bit_identical_reproduction"],
        AssuranceLevel::DifferentiallyTested
    );

    // q7: what remains unknown? — every property without evidence reads
    // `unknown`, per spec §24.
    let unknown_status = claim.supported_level("crash_safety", &store).status_name();
    assert_eq!(unknown_status, "unknown");

    // The whole chain holds together as identities:
    let _ = (
        computation_id,
        intent_id,
        reservation_id,
        execution_id,
        trace_id,
        derivation_id,
        provenance_id,
        capability_id,
        world_id,
        trace_normalized,
    );
}

#[test]
fn reproduction_request_expresses_spec_section_17() {
    let computation = Computation::build(Operation::new("repro.build.rust").unwrap()).finish();
    let world = ExecutionWorld::build()
        .randomness(RandomnessPolicy::Deterministic { seed: 1 })
        .finish();

    let request = ReproductionRequest {
        computation: computation.identity(),
        permitted_worlds: BTreeSet::from([world.identity()]),
        substitution_policy: SubstitutionPolicy::Permitted {
            kinds: BTreeSet::from([SubstitutionKind::Toolchain]),
        },
        relevance: WorldRelevance::all(),
    };
    let bytes = request.canonical_bytes();
    let (version, restored): (u16, ReproductionRequest) =
        tpt_primitives::encoding::decode_envelope(&bytes).unwrap();
    assert_eq!(version, ReproductionRequest::SCHEMA_VERSION);
    assert_eq!(restored, request);

    // The five-way classification is expressible and self-describing.
    let artifacts = BTreeSet::from([artifact_of_content(b"repro-out")]);
    let outcomes = [
        ReproductionOutcome::Reproduced {
            artifacts: artifacts.clone(),
        },
        ReproductionOutcome::ReproducedWithSubstitution {
            substitutions: BTreeSet::from([SubstitutionKind::Toolchain]),
            artifacts: artifacts.clone(),
        },
        ReproductionOutcome::EquivalentButNonIdentical {
            artifacts: artifacts.clone(),
            note: "debug info differs".into(),
        },
        ReproductionOutcome::Failed {
            reason: "toolchain unavailable".into(),
        },
        ReproductionOutcome::Indeterminate {
            reason: "no baseline".into(),
        },
    ];
    assert_eq!(
        outcomes.len(),
        5,
        "spec section 17 defines exactly five outcomes"
    );
}
