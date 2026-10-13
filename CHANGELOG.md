# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `valid_authority_with_ancestors`: delegation-chain check, so revoking a
  parent capability invalidates its descendants.
- `Capability::transfer_with_lease` and `CapabilityError::LeaseBound`.
- `CanonicalError::NonCanonical` and `PrimitiveValue::int`.
- Examples: `quickstart`, `capability_delegation`, `assurance_graph`.

### Changed

- **Behavior:** `Canonical::from_canonical_bytes` now rejects bytes that
  decode but are not the canonical encoding (non-minimal integers,
  uppercase identity hex, indefinite lengths).
- **Behavior:** a matching `Fail` evidence result now refutes a property
  (supported level `Unknown`) instead of being ignored next to a passing run.
- **API:** `Epoch::transition` returns `Option<Epoch>`; the counter no
  longer overflows. `EpochBook::transition` errors on exhaustion.

### Fixed

- `Capability::transfer` succeeded on a revoked capability.
- Transferring or attenuating a lease-bound capability to another holder
  produced a child that could never pass `valid_authority`; it now needs a
  lease for the new holder.

## [0.1.0] - 2026-10-05

Initial release of the shared-primitives library.

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
- JSON Schema and CDDL generation for all canonical types (`schema-gen`),
  with checked-in outputs under `schemas/`.
- Documentation: architecture map (module → spec section), trust boundary
  statement (spec §19), design rules (spec §21–24), schema evolution policy.

[Unreleased]: https://tpt.example/tpt-primitives/compare/v0.1.0...HEAD
[0.1.0]: https://tpt.example/tpt-primitives/releases/tag/v0.1.0
