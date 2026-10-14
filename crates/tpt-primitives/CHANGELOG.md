# Changelog

All notable changes to `tpt-primitives` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ScopeRequirement` (platform, toolchain and required environment variables)
  and `Claim::requirement`; `Evidence::platform`.
- `AssessmentPolicy` with `Claim::assess_with_policy`: optional trusted proof
  kernels and strict assumption discharge. `ClaimAssessment` now lists
  `undischarged_assumptions` (propagated through dependencies).
- `ReservationSpec::check_against(&Intent)`.
- `valid_authority_with_ancestors`: delegation-chain check, so revoking a
  parent capability invalidates its descendants.
- `Capability::transfer_with_lease` and `CapabilityError::LeaseBound`.
- `CanonicalError::NonCanonical` and `PrimitiveValue::int`.
- `Claim::to_dot` Graphviz export of the assurance graph.
- `prelude` module; `hash::from_hex_vec`.
- Golden vectors (`testvectors/v1.json`), property tests, and the
  `capability_delegation`, `assurance_graph` and `quickstart` examples.

### Changed

- **Breaking encoding (ADR 0005):** every canonical type is schema version 2.
  Absent optional fields are omitted from canonical CBOR instead of encoded as
  `null`, matching the CDDL `?` notation.
- **Breaking (ADR 0005):** `Evidence` has `platform: Option<PlatformSpec>`;
  `Claim` has `requirement: ScopeRequirement`. Evidence outside a claim's
  requirement neither supports nor refutes that claim.
- **Behavior:** `Execution::identity()` now derives from `ExecutionSpec`
  (computation, world, attempt, start) only; outcome, finish time and trace
  no longer change an execution's identity. New `Execution::spec()`.
- **Behavior:** trace normalization is now a deterministic causal order
  (parents first, ties by identity) that keeps repeated events; it no longer
  sorts by hash and collapses duplicates. `replay_worklist` is in causal
  order and `compare_replay` counts multiplicity.
- **Behavior:** `Canonical::from_canonical_bytes` runs the new
  `Canonical::validate`, enforced for `Lease`, `CapabilityGrant`,
  `ReservationSpec` and `Execution`; `Principal` and `AuthorityDomain`
  reject empty labels when deserialized.
- **Breaking encoding (pre-release, ADR 0003):** map keys now use RFC 8949
  section 4.2.1 order, and identities are 32-byte byte strings in canonical
  CBOR (hex only in JSON). Every identity differs from earlier builds; the
  CDDL `tpt-id` is now `bstr .size 32`.
- **Behavior:** `Canonical::from_canonical_bytes` now rejects bytes that
  decode but are not the canonical encoding (non-minimal integers,
  uppercase identity hex, indefinite lengths).
- **Behavior:** a matching `Fail` evidence result now refutes a property
  (supported level `Unknown`) instead of being ignored next to a passing run.
- **API:** `Execution::finish` returns `Result` and rejects a second finish or
  a finish before the start tick.
- **API:** `Epoch::transition` returns `Option<Epoch>`; the counter no
  longer overflows. `EpochBook::transition` errors on exhaustion.

### Fixed

- `Claim::assess` re-assessed shared dependencies on every path (exponential on
  diamond graphs); results are now memoized per call.
- `Capability::transfer` succeeded on a revoked capability.
- Transferring or attenuating a lease-bound capability to another holder
  produced a child that could never pass `valid_authority`; it now needs a
  lease for the new holder.

## [0.1.0] - 2026-10-05

Initial release.

### Added

- Canonical object framework (spec §4): deterministic CBOR encoding with an
  explicit schema-version envelope, SHA-256 identity derivation, `Id<T>`
  newtype shared by all identity classes, and documented evolution rules.
- The fifteen shared identity classes (spec §3): `SpecificationId`,
  `ModelId`, `ImplementationId`, `ComputationId`, `IntentId`,
  `ReservationId`, `ExecutionId`, `ArtifactId`, `TraceId`, `EvidenceId`,
  `ClaimId`, `CapabilityId`, `EpochId`, `CounterexampleId`, `ProvenanceId`.
- Capability, lease and epoch primitives with a composed
  valid-authority check and epoch-transition invalidation (spec §5–6).
- Intent primitive (spec §7) and Reservation primitive with an explicit
  `requested → reserved → committed → running → released` state machine,
  expiry and failure semantics (spec §8).
- Computation and Derivation primitives (spec §9–10) including lineage via
  `input artifacts + ComputationID → DerivationID → output artifact`.
- Execution and Trace primitives (spec §11–12) with stable event identity,
  deterministic normalization, and replay divergence detection.
- Evidence, Claim and Counterexample primitives (spec §13–15) with the
  typed evidence hierarchy, the Assurance Graph (spec §18) and a
  mechanical guard against claiming more than the evidence supports (spec §24).
- ExecutionWorld, world-relevance declaration, and ReproductionRequest
  primitives with the five-way reproduction outcome classification (spec §16–17).

[Unreleased]: https://tpt.example/tpt-primitives/compare/v0.1.0...HEAD
[0.1.0]: https://tpt.example/tpt-primitives/releases/tag/v0.1.0
