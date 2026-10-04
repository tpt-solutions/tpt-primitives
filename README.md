# TPT Primitives

Foundational shared-primitives library for the **Fabric / Repro / Concord** ecosystem.

TPT Primitives defines the small, composable semantic core that the three
ecosystem projects share:

| Project | Question it answers | Primitives it owns |
|---------|--------------------|--------------------|
| **Concord** | "Why trust it?" | specification, evidence, claim |
| **Repro**   | "What happened?" | computation, derivation, artifact identity |
| **Fabric**  | "Where/how did it run?" | intent, reservation, execution world |

The central chain this library models is:

```text
SPECIFICATION → COMPUTATION → INTENT → RESERVATION → EXECUTION
             → ARTIFACT → EVIDENCE → CLAIM
```

## Scope

This crate contains **primitives only**: the canonical identity types,
the canonical-object framework, and the semantic types built on them
(capability, lease/epoch, intent, reservation, computation, derivation,
execution, trace, evidence, claim, counterexample, deterministic world,
reproduction).

No Fabric, Repro or Concord implementation lives here. No adapters
(Kubernetes, OCI, Git, cloud storage, CI, AI providers) live here —
per the design rule *primitives before adapters* (spec §21).

The authoritative cross-project design specification is [`spec.txt`](spec.txt).

## Canonical identity

Every important object has a stable canonical identity:

- Canonical representation with an explicit schema version.
- Deterministic CBOR encoding (see `docs/decisions/0001-canonical-encoding-and-identity.md`).
- Identity derived as SHA-256 over the canonical bytes — never from
  filenames, auto-increment IDs, memory addresses, mutable labels or
  wall-clock timestamps (spec §3).

The fifteen shared identity classes (spec §3) are exposed as
`SpecificationId`, `ModelId`, `ImplementationId`, `ComputationId`,
`IntentId`, `ReservationId`, `ExecutionId`, `ArtifactId`, `TraceId`,
`EvidenceId`, `ClaimId`, `CapabilityId`, `EpochId`, `CounterexampleId`
and `ProvenanceId`, all instances of the generic `Id<T>` newtype.

## Layout

- `crates/tpt-primitives` — the core library.
- `crates/schema-gen` — tool that generates JSON Schema / CDDL from the
  canonical types into `schemas/` (CI fails if the checked-in schemas drift).
- `docs/decisions/` — architecture decision records (encoding, hashing, time model).
- `docs/` — architecture mapping, trust boundaries, design rules, evolution policy.

## License

Dual-licensed under `MIT OR Apache-2.0`, at your option.
Copyright © 2026 TPT Solutions.
