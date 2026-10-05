# Architecture: modules → spec sections

Every module in `crates/tpt-primitives/src` maps back to a section of
`spec.txt` (the cross-project design specification). This is the authoritative
map for reviewers; keep it in sync when modules change.

| Module | Spec | Responsibility |
|--------|------|----------------|
| `canonical.rs` | §4 | The canonical object contract: `Canonical` (schema version + deterministic bytes) and `Identified` (identity derivation). |
| `encoding.rs` | §4 | The deterministic CBOR envelope `["tpt-canonical", version, payload]` with strict decode. |
| `hash.rs` | §3, ADR 0001 | SHA-256 and hex helpers. |
| `id.rs` | §3 | `Id<T>` newtype, `IdClass` domain separation, `AnyId` class erasure. |
| `ids.rs` | §3 | The fifteen shared identity classes (+ lease, derivation, world, trace-event classes the primitives need). |
| `time.rs` | §3, §16, ADR 0002 | `LogicalTime` — no wall clock anywhere. |
| `value.rs` | §4 | `PrimitiveValue`/`PrimitiveMap` — canonical scalar content, no floats. |
| `resources.rs` | §7–9, §16 | `ResourceSet`/`ResourceKind`. |
| `environment.rs` | §9, §13, §16 | `EnvironmentSpec`, `PlatformSpec`, `ToolchainSpec`. |
| `capability.rs` | §5–6 | Capability (explicit, transferable, attenuable, revocable, domain-bound), Lease, Epoch, `EpochBook`, and the composed `valid_authority` check. |
| `intent.rs` | §7 | Intent: computation ref, capabilities, resources, locality, deadline, durability, failure policy. |
| `reservation.rs` | §8 | Reservation state machine `requested → reserved → committed → running → released` with expiry and failure semantics. |
| `computation.rs` | §9–10 | Computation (operation, inputs, dependencies, environment, platform, toolchain, resource policy, determinism policy, world relevance), Derivation, Provenance, content-addressed artifacts. |
| `execution.rs` | §11–12 | Execution (same computation ≠ same execution), Trace with stable event identity, normalization, replay divergence detection. |
| `evidence.rs` | §13–15, §18, §24 | Typed Evidence kinds, AssuranceLevel hierarchy, Claim (Assurance Graph) with the mechanical overstatement guard, Counterexample. |
| `world.rs` | §16–17 | ExecutionWorld, WorldRelevance projection/digest, ReproductionRequest, ReproductionOutcome (five-way), SubstitutionPolicy. |
| `schema.rs` | §4 | Registry consumed by `schema-gen`; JSON Schema + CDDL for the canonical types. |

## Ownership (spec §2)

- Concord owns specification, evidence and claim semantics → `evidence.rs`
  (plus `ids::SpecificationTag`, `ModelTag`, `ImplementationTag`).
- Repro owns computation, derivation and artifact identity →
  `computation.rs`.
- Fabric owns intent, reservation and execution-world semantics →
  `intent.rs`, `reservation.rs`, `world.rs` (execution/trace mechanics in
  `execution.rs`).

The central chain of §2 —
`SPECIFICATION → COMPUTATION → INTENT → RESERVATION → EXECUTION →
ARTIFACT → EVIDENCE → CLAIM` — is exercised end-to-end by
`crates/tpt-primitives/tests/interop.rs` (§20, §26).

## Deliberate exclusions

- No Fabric/Repro/Concord implementations (scope).
- No adapters: no Kubernetes/OCI/Git/cloud/CI/AI integrations (§21).
- No wall clock, no I/O, no threads: keeps the crate inside every
  project's trusted base for shape only (§19, `docs/trust-boundaries.md`).
