# TPT Primitives — Project Todo

Foundational shared-primitives library for the Fabric / Repro / Concord ecosystem.
Scope: primitives only (identity, capability, intent, computation, execution, evidence,
claim, deterministic world). No Fabric/Repro/Concord implementation lives here.

License: dual MIT OR Apache-2.0, © TPT Solutions.

---

## Phase 0 — Project Setup & Governance
- [ ] Initialize git repository
- [ ] Decide crate/workspace layout (e.g. `tpt-primitives` core crate + a schema-gen tool crate/xtask)
- [ ] Scaffold Cargo workspace (`Cargo.toml`, crate directories)
- [ ] Add `LICENSE-MIT` and `LICENSE-APACHE` (copyright TPT Solutions), set `license = "MIT OR Apache-2.0"` in Cargo.toml(s)
- [ ] Write root `README.md` (purpose, spec reference, scope boundary, license)
- [ ] Set up CI (build, test, `cargo fmt --check`, `cargo clippy`)
- [ ] Set up `CHANGELOG.md` and versioning policy (semver, starting at 0.1.0)
- [ ] Decide and document canonical encoding scheme (e.g. deterministic CBOR or JCS) and hash function (e.g. BLAKE3 or SHA-256) used for identity derivation (ADR / `docs/decisions/`)

## Phase 1 — Canonical Object Framework (spec §3–4)
- [ ] Define the "canonical object" contract: canonical representation, explicit schema version, deterministic encoding, deterministic identity derivation, explicit evolution rules
- [ ] Implement deterministic encoding module (type → canonical bytes)
- [ ] Implement identity derivation module (canonical bytes → stable ID, via chosen hash function)
- [ ] Define generic ID newtype pattern (e.g. `Id<T>`) shared by all identity classes
- [ ] Define schema-versioning/evolution strategy (what counts as a breaking vs. non-breaking change)
- [ ] Build schema-generation tool: derive JSON Schema/CDDL output from Rust types
- [ ] Tests: identity is stable across irrelevant presentation changes; identity changes when semantics change

## Phase 2 — Core Identity Types (spec §3)
- [ ] `SpecificationID`, `ModelID`, `ImplementationID`
- [ ] `ComputationID`, `IntentID`, `ReservationID`
- [ ] `ExecutionID`, `ArtifactID`, `TraceID`
- [ ] `EvidenceID`, `ClaimID`, `CapabilityID`
- [ ] `EpochID`, `CounterexampleID`, `ProvenanceID`
- [ ] Unit tests + generated schema entries for each ID type

## Phase 3 — Capability + Lease/Epoch Primitives (spec §5–6)
- [ ] `Capability` type: explicit, transferable, attenuable, revocable, bound to authority domain, optional epoch/lease binding
- [ ] `Lease` type (temporal authority)
- [ ] `Epoch` type (distributed invalidation)
- [ ] Compose capability + lease + epoch → "valid authority" check
- [ ] Epoch-transition invalidation semantics (old capability fails post-transition)
- [ ] Tests covering revocation, attenuation, and epoch-transition invalidation

## Phase 4 — Intent + Reservation Primitives (spec §7–8)
- [ ] `Intent` type: computation ref, capabilities, resources, locality, deadline, durability, failure policy
- [ ] `Reservation` type + state machine: requested → reserved → committed → running → released
- [ ] Explicit expiry and failure semantics on `Reservation`
- [ ] Tests for state transitions, expiry, and failure paths

## Phase 5 — Computation + Derivation Primitives (spec §9–10)
- [ ] `Computation` type: operation, inputs, dependencies, environment, platform, toolchain, resource policy, determinism policy
- [ ] `Derivation` type: input artifacts + `ComputationID` → `DerivationID` → output artifact
- [ ] Tests for derivation/lineage construction

## Phase 6 — Execution + Trace Primitives (spec §11–12)
- [ ] `Execution` type distinguishing "same computation" vs. "same execution"
- [ ] `Trace` type: stable event identity, causal relationships, deterministic normalization
- [ ] Replay support and divergence detection for traces
- [ ] Tests

## Phase 7 — Evidence, Claim & Counterexample Primitives (spec §13–15, §18, §24)
- [ ] `Evidence` type with subject/property/environment/toolchain/assumptions/result/identity
- [ ] Evidence kinds: Proof, ModelCheck, Conformance, Simulation, PropertyTest, FuzzCampaign, DifferentialCheck, Benchmark
- [ ] `Claim` type: subject, property, scope, assumptions, required obligations, supporting evidence, dependent claims (Assurance Graph)
- [ ] `Counterexample` type: subject, specification, input/state, trace, environment, observed violation
- [ ] Mechanism that makes it structurally difficult to overstate a claim beyond its evidence (e.g. per-property status: fully_proven / model_checked / fuzz_tested / unknown)
- [ ] Tests, incl. an assurance-graph composition example (per §18 example)

## Phase 8 — Deterministic World + Reproduction Primitives (spec §16–17)
- [ ] `ExecutionWorld` type: time model, platform, resources, capabilities, filesystem/object state, network policy, randomness policy, env vars, external dependencies
- [ ] Mechanism for a computation to declare which parts of the world are semantically relevant
- [ ] `ReproductionRequest` type
- [ ] Reproduction result classification: reproduced / reproduced-with-substitution / equivalent-but-non-identical / failed / indeterminate
- [ ] Tests

## Phase 9 — Trust Boundary Documentation (spec §19)
- [ ] Document what Fabric trusts (core semantics, declared transport guarantees)
- [ ] Document what Repro trusts (canonicalization, hashing, artifact storage integrity, execution records)
- [ ] Document what Concord trusts (evidence validity per type, external proof kernels, its own assurance model)
- [ ] Explicitly document: AI is never part of the trusted base

## Phase 10 — Cross-Project Interop Shape (spec §20, type-level only)
- [ ] Type-level example/integration test wiring: `ComputationID` → `Intent` → `ExecutionID`+`TraceID` → `ArtifactID`+`ProvenanceID` → `EvidenceID`+`ClaimID`
- [ ] Confirm this validates composability only — no real execution/adapter logic (per design rule §21)

## Phase 11 — Schema Export & Documentation
- [ ] Wire schema-generation tool into CI (fail build if generated schemas drift from types)
- [ ] Check generated JSON Schema/CDDL files into `schemas/`
- [ ] Rustdoc pass across all public types
- [ ] Architecture docs mapping each module back to its spec section
- [ ] Write up design rules as docs: primitives-before-adapters (§21), simulation-is-architectural (§22), proof-is-not-only-evidence (§23)

## Phase 12 — Release Readiness
- [ ] Full test suite green, clippy/fmt clean
- [ ] Review against the success criterion (§26): can the primitives collectively answer all 7 questions (intended computation, exact computation, where/under what authority, what was produced, how derived, what evidence, what remains unknown)?
- [ ] Finalize `CHANGELOG.md` for 0.1.0
- [ ] Tag `v0.1.0`
- [ ] Decide on crates.io publication (or hold private pending Fabric/Repro/Concord integration)
