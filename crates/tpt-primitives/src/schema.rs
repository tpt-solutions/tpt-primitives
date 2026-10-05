//! Schema description registry for the canonical types.
//!
//! `schema-gen` (in `crates/schema-gen`) walks this registry to emit the
//! checked-in JSON Schema and CDDL definitions under `schemas/`. Each
//! entry pairs a generated JSON Schema (`schemars`) with a hand-written
//! CDDL definition, so both views evolve together and CI can fail on
//! drift between the Rust types and the checked-in schemas.
//!
//! Note: entries describe the *payload* of a canonical object. On the
//! wire, every canonical object is wrapped in the schema-version envelope
//! documented in ADR 0001:
//!
//! ```cddl
//! tpt-envelope<V> = ["tpt-canonical", version: uint, payload: V]
//! ```

/// A registry entry: type name, CDDL definition, and a JSON Schema factory.
#[derive(Debug)]
pub struct SchemaEntry {
    /// Type name, e.g. `"Computation"`.
    pub name: &'static str,
    /// Hand-written CDDL definition for the type.
    pub cddl: &'static str,
    /// Produce the JSON Schema for the type.
    pub json_schema: fn() -> schemars::Schema,
}

macro_rules! entry {
    ($name:literal, $cddl:literal, $ty:ty) => {
        SchemaEntry {
            name: $name,
            cddl: $cddl,
            json_schema: || schemars::schema_for!($ty),
        }
    };
}

/// Shared CDDL prelude: identities, scalars, and small common types.
pub const CDDL_PRELUDE: &str = r#"; TPT Primitives canonical types (payloads; wire form wraps each
; payload in tpt-envelope<V> = ["tpt-canonical", version: uint, payload: V])

tpt-id = text .regexp "[0-9a-f]{64}"
tpt-principal = { holder: text }
tpt-domain = { domain: text }

; value.rs
primitive-value = ({"text": text} // {"bytes": bstr} // {"uint": uint}
                 // {"int": int} // {"bool": bool} // {"null": null})
primitive-map = { * text => primitive-value }

; resources.rs
resource-kind = ("cpu_cores" // "memory_mib" // "disk_mib" // "network_mbps"
               // "gpu_devices" // {"custom": {label: text}})
resource-set = { amounts: { * resource-kind => uint } }

; environment.rs
environment-spec = { variables: primitive-map }
platform-spec = { os: text, arch: text }
toolchain-spec = { name: text, version: text }

; capability.rs
grant = { resource: text, action: text }
lease = tpt-envelope<{ domain: text, holder: text, valid_from: uint, expires_at: uint }>
epoch = tpt-envelope<{ domain: text, counter: uint }>
epoch-requirement = { domain: text, epoch: tpt-id }
capability-grant = tpt-envelope<{
  domain: text,
  holder: text,
  grants: [* grant],
  transferable: bool,
  ?parent: tpt-id,
  ?lease: tpt-id,
  ?epoch_binding: epoch-requirement,
}>
capability-status = ({status: "active"} // {status: "revoked", at: uint})
capability = { grant: capability-grant, status: capability-status }

; intent.rs
locality = ({locality: "any"} // {locality: "region", name: text}
          // {locality: "node", name: text})
durability = "ephemeral" / "persistent"
failure-policy = ({"failure_policy": "fail_fast"}
                // {"failure_policy": "retry", max_attempts: uint, backoff_ticks: uint}
                // {"failure_policy": "best_effort"})
intent = tpt-envelope<{
  ?computation: tpt-id,
  capabilities: [* tpt-id],
  resources: resource-set,
  locality: locality,
  ?deadline: uint,
  durability: durability,
  failure_policy: failure-policy,
}>

; reservation.rs
reservation-state = ("requested" / "reserved" / "committed" / "running"
                   / "released" / "expired" / "failed")
reservation-spec = tpt-envelope<{
  intent: tpt-id, resources: resource-set, holder: text, expires_at: uint,
}>
state-transition = { from: reservation-state, to: reservation-state, at: uint }
reservation = tpt-envelope<{
  spec: reservation-spec,
  state: reservation-state,
  history: [* state-transition],
  ?failure_reason: text,
}>

; computation.rs
operation = { kind: text, parameters: primitive-map }
determinism-policy = "bit_identical" / "semantically_equivalent"
world-relevance = { ?time_model: bool, ?platform: bool, ?resources: bool
                  , ?capabilities: bool, ?objects: bool, ?network: bool
                  , ?randomness: bool, ?env: bool, ?external_dependencies: bool }
computation = tpt-envelope<{
  operation: operation,
  inputs: [* tpt-id],
  dependencies: [* tpt-id],
  environment: environment-spec,
  platform: platform-spec,
  toolchain: toolchain-spec,
  resource_policy: resource-set,
  determinism_policy: determinism-policy,
  world_relevance: world-relevance,
}>
derivation-spec = tpt-envelope<{ computation: tpt-id, inputs: [* tpt-id] }>
derivation = tpt-envelope<{ spec: derivation-spec, outputs: [* tpt-id] }>
provenance = tpt-envelope<{
  artifact: tpt-id, derivation: tpt-id, ?execution: tpt-id,
  produced_by: text, produced_at: uint,
}>

; execution.rs
execution-outcome = ({"outcome": "succeeded", artifacts: [* tpt-id]}
                   // {"outcome": "failed", reason: text})
execution = tpt-envelope<{
  computation: tpt-id, world: tpt-id, attempt: uint,
  started_at: uint, ?finished_at: uint, ?outcome: execution-outcome,
  ?trace: tpt-id,
}>
trace-event = tpt-envelope<{ payload: primitive-map, causal_parents: [* tpt-id] }>
trace = tpt-envelope<{ execution: tpt-id, events: [* trace-event] }>

; evidence.rs
evidence-kind = ({"kind": "proof", kernel: text}
               // {"kind": "model_check", model: text}
               // {"kind": "conformance", suite: text}
               // {"kind": "simulation", world: tpt-id, ticks: uint}
               // {"kind": "property_test", framework: text, cases: uint}
               // {"kind": "fuzz_campaign", fuzzer: text, executions: uint}
               // {"kind": "differential_check", baseline: text}
               // {"kind": "benchmark", metric: text, value: uint, scale: uint, unit: text})
evidence-result = "pass" / "fail" / "inconclusive"
assumption = { statement: text, ?evidence: tpt-id }
assurance-level = ("unknown" / "benchmarked" / "fuzz_tested" / "differentially_tested"
                 / "property_tested" / "simulated" / "conformance_tested"
                 / "model_checked" / "fully_proven")
obligation = { property: text, min_level: assurance-level }
evidence = tpt-envelope<{
  subject: {class: text, id: text}, property: text,
  environment: environment-spec, toolchain: toolchain-spec,
  assumptions: [* assumption], kind: evidence-kind,
  result: evidence-result, produced_at: uint,
}>
any-id = { class: text, id: text }
claim = tpt-envelope<{
  subject: any-id, property: text, scope: text,
  assumptions: [* assumption],
  required_obligations: [* obligation],
  evidence: [* tpt-id], depends_on: [* tpt-id],
}>
counterexample = tpt-envelope<{
  subject: any-id, specification: any-id, property: text,
  input_state: primitive-map, ?trace: tpt-id,
  environment: environment-spec, observed_violation: text, recorded_at: uint,
}>

; world.rs
time-model = {"time_model": "logical", ticks_per_second: uint}
randomness-policy = ({"randomness": "deterministic", seed: uint}
                   // {"randomness": "entropy_source"})
network-policy = ({"network": "isolated"} // {"network": "local_only"}
                // {"network": "allowlist", hosts: [* text]}
                // {"network": "unrestricted"})
execution-world = tpt-envelope<{
  time_model: time-model, platform: platform-spec, resources: resource-set,
  required_capabilities: [* tpt-id], objects: { * text => tpt-id },
  network: network-policy, randomness: randomness-policy,
  env: environment-spec, external_dependencies: { * text => tpt-id },
}>
relevant-world = { ?time_model: time-model, ?platform: platform-spec
                 , ?resources: resource-set, ?capabilities: [* tpt-id]
                 , ?objects: { * text => tpt-id }, ?network: network-policy
                 , ?randomness: randomness-policy, ?env: environment-spec
                 , ?external_dependencies: { * text => tpt-id } }
substitution-kind = ("platform" / "toolchain" / "resources" / "environment"
                   / "external_dependency")
substitution-policy = ({"substitution_policy": "exact_only"}
                     // {"substitution_policy": "permitted", kinds: [* substitution-kind]})
reproduction-request = {
  computation: tpt-id, permitted_worlds: [* tpt-id],
  substitution_policy: substitution-policy, relevance: world-relevance,
}
reproduction-outcome = ({"reproduction": "reproduced", artifacts: [* tpt-id]}
  // {"reproduction": "reproduced_with_substitution", substitutions: [* substitution-kind], artifacts: [* tpt-id]}
  // {"reproduction": "equivalent_but_non_identical", artifacts: [* tpt-id], note: text}
  // {"reproduction": "failed", reason: text}
  // {"reproduction": "indeterminate", reason: text})
"#;

/// All schema entries contributed by this crate, in stable order.
pub fn entries() -> Vec<SchemaEntry> {
    use crate::capability as cap;
    use crate::computation as comp;
    use crate::environment as env;
    use crate::evidence as ev;
    use crate::execution as exe;
    use crate::id;
    use crate::ids;
    use crate::intent as int;
    use crate::reservation as res;
    use crate::resources as rsc;
    use crate::time;
    use crate::value;
    use crate::world as wld;

    vec![
        // Identity classes: all serialize as the shared hex-string form.
        SchemaEntry {
            name: "Id",
            cddl: "; All identity classes share this shape.\ntpt-id = text .regexp \"[0-9a-f]{64}\"\n",
            json_schema: || schemars::schema_for!(ids::ComputationId),
        },
        entry!(
            "LogicalTime",
            "; A logical tick count.\nlogical-time = uint\n",
            time::LogicalTime
        ),
        entry!(
            "PrimitiveValue",
            "; See prelude.\nprimitive-value = ({\"text\": text} // {\"bytes\": bstr} // {\"uint\": uint} // {\"int\": int} // {\"bool\": bool} // {\"null\": null})\n",
            value::PrimitiveValue
        ),
        entry!("ResourceSet", "; See prelude.\n", rsc::ResourceSet),
        entry!("ResourceKind", "; See prelude.\n", rsc::ResourceKind),
        entry!("EnvironmentSpec", "; See prelude.\n", env::EnvironmentSpec),
        entry!("PlatformSpec", "; See prelude.\n", env::PlatformSpec),
        entry!("ToolchainSpec", "; See prelude.\n", env::ToolchainSpec),
        entry!("Grant", "; See prelude.\n", cap::Grant),
        entry!("Lease", "; See prelude.\n", cap::Lease),
        entry!("Epoch", "; See prelude.\n", cap::Epoch),
        entry!(
            "EpochRequirement",
            "; See prelude.\n",
            cap::EpochRequirement
        ),
        entry!("CapabilityGrant", "; See prelude.\n", cap::CapabilityGrant),
        entry!(
            "CapabilityStatus",
            "; See prelude.\n",
            cap::CapabilityStatus
        ),
        entry!("Capability", "; See prelude.\n", cap::Capability),
        entry!("Locality", "; See prelude.\n", int::Locality),
        entry!("Durability", "; See prelude.\n", int::Durability),
        entry!("FailurePolicy", "; See prelude.\n", int::FailurePolicy),
        entry!("Intent", "; See prelude.\n", int::Intent),
        entry!(
            "ReservationState",
            "; See prelude.\n",
            res::ReservationState
        ),
        entry!("ReservationSpec", "; See prelude.\n", res::ReservationSpec),
        entry!("StateTransition", "; See prelude.\n", res::StateTransition),
        entry!("Reservation", "; See prelude.\n", res::Reservation),
        entry!("Operation", "; See prelude.\n", comp::Operation),
        entry!(
            "DeterminismPolicy",
            "; See prelude.\n",
            comp::DeterminismPolicy
        ),
        entry!("Computation", "; See prelude.\n", comp::Computation),
        entry!("DerivationSpec", "; See prelude.\n", comp::DerivationSpec),
        entry!("Derivation", "; See prelude.\n", comp::Derivation),
        entry!("Provenance", "; See prelude.\n", comp::Provenance),
        entry!(
            "ExecutionOutcome",
            "; See prelude.\n",
            exe::ExecutionOutcome
        ),
        entry!("Execution", "; See prelude.\n", exe::Execution),
        entry!("TraceEvent", "; See prelude.\n", exe::TraceEvent),
        entry!("Trace", "; See prelude.\n", exe::Trace),
        entry!("EvidenceKind", "; See prelude.\n", ev::EvidenceKind),
        entry!("EvidenceResult", "; See prelude.\n", ev::EvidenceResult),
        entry!("Assumption", "; See prelude.\n", ev::Assumption),
        entry!("AssuranceLevel", "; See prelude.\n", ev::AssuranceLevel),
        entry!("Obligation", "; See prelude.\n", ev::Obligation),
        entry!("Evidence", "; See prelude.\n", ev::Evidence),
        entry!("AnyId", "; See prelude.\n", id::AnyId),
        entry!("Claim", "; See prelude.\n", ev::Claim),
        entry!("Counterexample", "; See prelude.\n", ev::Counterexample),
        entry!("TimeModel", "; See prelude.\n", wld::TimeModel),
        entry!(
            "RandomnessPolicy",
            "; See prelude.\n",
            wld::RandomnessPolicy
        ),
        entry!("NetworkPolicy", "; See prelude.\n", wld::NetworkPolicy),
        entry!("ExecutionWorld", "; See prelude.\n", wld::ExecutionWorld),
        entry!("WorldRelevance", "; See prelude.\n", wld::WorldRelevance),
        entry!("RelevantWorld", "; See prelude.\n", wld::RelevantWorld),
        entry!(
            "SubstitutionKind",
            "; See prelude.\n",
            wld::SubstitutionKind
        ),
        entry!(
            "SubstitutionPolicy",
            "; See prelude.\n",
            wld::SubstitutionPolicy
        ),
        entry!(
            "ReproductionRequest",
            "; See prelude.\n",
            wld::ReproductionRequest
        ),
        entry!(
            "ReproductionOutcome",
            "; See prelude.\n",
            wld::ReproductionOutcome
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_have_unique_names() {
        let mut names: Vec<_> = entries().into_iter().map(|e| e.name).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len, "entry names must be unique");
    }

    #[test]
    fn json_schema_factories_produce_schemas() {
        for entry in entries() {
            let schema = (entry.json_schema)();
            let mut buf = Vec::new();
            ciborium::ser::into_writer(&schema, &mut buf).expect("schema serializes");
            assert!(!buf.is_empty(), "{} must produce a schema", entry.name);
        }
    }
}
