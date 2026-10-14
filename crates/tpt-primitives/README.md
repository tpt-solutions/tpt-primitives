# tpt-primitives

Foundational shared primitives for the Fabric / Repro / Concord ecosystem.

`tpt-primitives` defines the small, composable semantic core that the three
ecosystem projects share:

| Project | Question it answers | Primitives it owns |
|---------|--------------------|--------------------|
| **Concord** | "Why trust it?" | specification, evidence, claim |
| **Repro**   | "What happened?" | computation, derivation, artifact identity |
| **Fabric**  | "Where/how did it run?" | intent, reservation, execution world |

The central chain it models is:

```text
SPECIFICATION → COMPUTATION → INTENT → RESERVATION → EXECUTION
             → ARTIFACT → EVIDENCE → CLAIM
```

## What it provides

- **Canonical objects.** Deterministic CBOR encoding with an explicit schema
  version, and SHA-256 identities derived from the canonical bytes.
- **Identity classes.** Fifteen typed identifiers (`ClaimId`, `ExecutionId`,
  `CapabilityId`, …) built on a shared `Id<T>` newtype.
- **Capabilities, leases and epochs.** A composed valid-authority check with
  delegation chains and epoch-transition invalidation.
- **Intents and reservations.** A reservation state machine
  (`requested → reserved → committed → running → released`) with expiry and
  failure semantics.
- **Computations, executions and traces.** Stable event identity, deterministic
  trace normalization, and replay divergence detection.
- **Evidence, claims and counterexamples.** An assurance graph plus a mechanical
  guard against claiming more than the evidence supports.
- **Schemas.** JSON Schema and CDDL generation for every canonical type, with
  checked-in outputs under `schemas/`.

## Scope

This crate contains **primitives only**. It contains no Fabric, Repro or
Concord implementation and no adapters (Kubernetes, OCI, Git, cloud storage,
CI, AI providers). This follows the design rule *primitives before adapters*.

## Example

```sh
cargo run -p tpt-primitives --example quickstart
```

See `examples/` for capability delegation and the assurance graph.

## Feature status

Pre-1.0. Canonical encodings may change between minor releases; each change is
recorded in [CHANGELOG.md](CHANGELOG.md) together with the ADR that justifies it.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
