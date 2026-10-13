# TPT Primitives — Project Todo

Foundational shared-primitives library for the Fabric / Repro / Concord ecosystem.
Scope: primitives only (identity, capability, intent, computation, execution, evidence,
claim, deterministic world). No Fabric/Repro/Concord implementation lives here.

License: dual MIT OR Apache-2.0, © TPT Solutions.

---

## Phase 0 — Project Setup & Governance
- [x] Initialize git repository
- [x] Decide crate/workspace layout (e.g. `tpt-primitives` core crate + a schema-gen tool crate/xtask)
- [x] Scaffold Cargo workspace (`Cargo.toml`, crate directories)
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` (copyright TPT Solutions), set `license = "MIT OR Apache-2.0"` in Cargo.toml(s)
- [x] Write root `README.md` (purpose, spec reference, scope boundary, license)
- [x] Set up CI (build, test, `cargo fmt --check`, `cargo clippy`)
- [x] Set up `CHANGELOG.md` and versioning policy (semver, starting at 0.1.0)
- [x] Decide and document canonical encoding scheme (e.g. deterministic CBOR or JCS) and hash function (e.g. BLAKE3 or SHA-256) used for identity derivation (ADR / `docs/decisions/`)

## Phase 1 — Canonical Object Framework (spec §3–4)
- [x] Define the "canonical object" contract: canonical representation, explicit schema version, deterministic encoding, deterministic identity derivation, explicit evolution rules
- [x] Implement deterministic encoding module (type → canonical bytes)
- [x] Implement identity derivation module (canonical bytes → stable ID, via chosen hash function)
- [x] Define generic ID newtype pattern (e.g. `Id<T>`) shared by all identity classes
- [x] Define schema-versioning/evolution strategy (what counts as a breaking vs. non-breaking change)
- [x] Build schema-generation tool: derive JSON Schema/CDDL output from Rust types
- [x] Tests: identity is stable across irrelevant presentation changes; identity changes when semantics change

## Phase 2 — Core Identity Types (spec §3)
- [x] `SpecificationID`, `ModelID`, `ImplementationID`
- [x] `ComputationID`, `IntentID`, `ReservationID`
- [x] `ExecutionID`, `ArtifactID`, `TraceID`
- [x] `EvidenceID`, `ClaimID`, `CapabilityID`
- [x] `EpochID`, `CounterexampleID`, `ProvenanceID`
- [x] Unit tests + generated schema entries for each ID type

## Phase 3 — Capability + Lease/Epoch Primitives (spec §5–6)
- [x] `Capability` type: explicit, transferable, attenuable, revocable, bound to authority domain, optional epoch/lease binding
- [x] `Lease` type (temporal authority)
- [x] `Epoch` type (distributed invalidation)
- [x] Compose capability + lease + epoch → "valid authority" check
- [x] Epoch-transition invalidation semantics (old capability fails post-transition)
- [x] Tests covering revocation, attenuation, and epoch-transition invalidation

## Phase 4 — Intent + Reservation Primitives (spec §7–8)
- [x] `Intent` type: computation ref, capabilities, resources, locality, deadline, durability, failure policy
- [x] `Reservation` type + state machine: requested → reserved → committed → running → released
- [x] Explicit expiry and failure semantics on `Reservation`
- [x] Tests for state transitions, expiry, and failure paths

## Phase 5 — Computation + Derivation Primitives (spec §9–10)
- [x] `Computation` type: operation, inputs, dependencies, environment, platform, toolchain, resource policy, determinism policy
- [x] `Derivation` type: input artifacts + `ComputationID` → `DerivationID` → output artifact
- [x] Tests for derivation/lineage construction

## Phase 6 — Execution + Trace Primitives (spec §11–12)
- [x] `Execution` type distinguishing "same computation" vs. "same execution"
- [x] `Trace` type: stable event identity, causal relationships, deterministic normalization
- [x] Replay support and divergence detection for traces
- [x] Tests

## Phase 7 — Evidence, Claim & Counterexample Primitives (spec §13–15, §18, §24)
- [x] `Evidence` type with subject/property/environment/toolchain/assumptions/result/identity
- [x] Evidence kinds: Proof, ModelCheck, Conformance, Simulation, PropertyTest, FuzzCampaign, DifferentialCheck, Benchmark
- [x] `Claim` type: subject, property, scope, assumptions, required obligations, supporting evidence, dependent claims (Assurance Graph)
- [x] `Counterexample` type: subject, specification, input/state, trace, environment, observed violation
- [x] Mechanism that makes it structurally difficult to overstate a claim beyond its evidence (e.g. per-property status: fully_proven / model_checked / fuzz_tested / unknown)
- [x] Tests, incl. an assurance-graph composition example (per §18 example)

## Phase 8 — Deterministic World + Reproduction Primitives (spec §16–17)
- [x] `ExecutionWorld` type: time model, platform, resources, capabilities, filesystem/object state, network policy, randomness policy, env vars, external dependencies
- [x] Mechanism for a computation to declare which parts of the world are semantically relevant
- [x] `ReproductionRequest` type
- [x] Reproduction result classification: reproduced / reproduced-with-substitution / equivalent-but-non-identical / failed / indeterminate
- [x] Tests

## Phase 9 — Trust Boundary Documentation (spec §19)
- [x] Document what Fabric trusts (core semantics, declared transport guarantees)
- [x] Document what Repro trusts (canonicalization, hashing, artifact storage integrity, execution records)
- [x] Document what Concord trusts (evidence validity per type, external proof kernels, its own assurance model)
- [x] Explicitly document: AI is never part of the trusted base

## Phase 10 — Cross-Project Interop Shape (spec §20, type-level only)
- [x] Type-level example/integration test wiring: `ComputationID` → `Intent` → `ExecutionID`+`TraceID` → `ArtifactID`+`ProvenanceID` → `EvidenceID`+`ClaimID`
- [x] Confirm this validates composability only — no real execution/adapter logic (per design rule §21)

## Phase 11 — Schema Export & Documentation
- [x] Wire schema-generation tool into CI (fail build if generated schemas drift from types)
- [x] Check generated JSON Schema/CDDL files into `schemas/`
- [x] Rustdoc pass across all public types
- [x] Architecture docs mapping each module back to its spec section
- [x] Write up design rules as docs: primitives-before-adapters (§21), simulation-is-architectural (§22), proof-is-not-only-evidence (§23)

## Phase 12 — Release Readiness
- [x] Full test suite green, clippy/fmt clean
- [x] Review against the success criterion (§26): all 7 questions answered mechanically by the primitives — demonstrated in `tests/interop.rs` and tabulated in README (Success criterion section)
- [x] Finalize `CHANGELOG.md` for 0.1.0
- [x] Tag `v0.1.0`
- [x] Decide on crates.io publication — holding private pending Fabric/Repro/Concord integration; revisit at first cross-project integration

---

## Phase 13 — Post-0.1.0 Review Fixes
- [x] `Capability::transfer` rejects revoked capabilities
- [x] Lease-bound capabilities cannot move to another holder without a lease for them (`transfer_with_lease`)
- [x] Delegation-chain check: revoking a parent invalidates descendants (`valid_authority_with_ancestors`)
- [x] Epoch counter overflow is an error, not a panic/wrap
- [x] Strict decode: reject non-canonical encodings (`NonCanonical`)
- [x] `PrimitiveValue::int` normalizes Int/Uint
- [x] Failing evidence refutes a property instead of being ignored
- [x] Examples: quickstart, capability_delegation, assurance_graph
- [ ] Create a `v0.1.0` tag on the release commit (was marked done but did not exist) and tag the next release
- [ ] Remove or use `CanonicalError::Encode`; make `encode_envelope` fallible or document the panic

## Phase 14 — Semantics Still Open
- [ ] Evidence assumptions must cap assurance (unsupported assumption => lower level)
- [ ] Trusted-kernel allowlist for `Proof` evidence; check scope/environment match
- [ ] Dependency levels must match subject/scope, not only property name
- [ ] Replace the total `AssuranceLevel` order with a partial order or configurable lattice
- [ ] Traces: keep event order/multiplicity; causal-order replay worklist; detect reordering
- [ ] `Execution` identity should exclude mutable outcome/finish state (match Capability/Reservation); validate `finished_at >= started_at`, single `finish`
- [ ] Memoize `Claim::assess` on shared dependencies (diamond graphs are exponential)
- [ ] `validate()` on `Canonical`, called from decode, so deserialized values uphold constructor invariants
- [ ] Intent <-> Capability <-> Reservation coherence check

## Phase 15 — Encoding Decisions (breaking; decide before anything persists)
- [ ] Adopt RFC 8949 section 4.2 key ordering, or document the exact profile precisely
- [ ] Encode `Id` as 32-byte bstr instead of 64-char hex text (update CDDL)
- [ ] Cross-language test vectors (`testvectors/*.json`) and a second-language verifier
- [ ] Golden-file identity snapshot in CI: bytes change only with a `SCHEMA_VERSION` bump
- [ ] Check hand-written CDDL against the types, not only the checked-in files

## Phase 16 — Signing and Attestation
- [ ] `Signed<T>` envelope (Ed25519 + key id) for capabilities, evidence, provenance
- [ ] Replace free-text `produced_by` with a verifiable signer

## Phase 17 — Tooling and Adoption
- [ ] `tpt` CLI: `id`, `inspect`, `verify`, `diff`, `explain`
- [ ] `Claim::to_dot()` assurance-graph visualizer
- [ ] `prelude` module; `#[derive(Canonical)]` macro to remove boilerplate impls
- [ ] Feature flags: `schema` (drop schemars), `no_std`
- [ ] `proptest` round-trip/identity tests and a fuzz target for the decoder
- [ ] `templates/` (CI build attestation, model-check claim, reproducible-build record)
- [ ] Per-project 10-minute guides (Fabric, Repro, Concord), glossary, "which type do I need?" chart
- [ ] CI: cargo-semver-checks, cargo-deny/audit, `cargo doc -D warnings`; dependabot, MSRV, CONTRIBUTING.md
- [ ] Release automation (release-plz / cargo-release)
