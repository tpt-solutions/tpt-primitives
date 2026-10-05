# Design rules (spec §21–24)

The four design rules from the cross-project specification, and how this
crate enforces each one.

## Primitives before adapters (§21)

Do not begin with Kubernetes, OCI, Git, cloud storage, CI or AI-provider
integrations. First establish the semantic core; adapters are useful only
after it is stable.

**Enforced by scope:** this workspace contains exactly two crates —
`tpt-primitives` (semantic core) and `schema-gen` (a documentation tool).
There is no I/O dependency in the core crate, no feature flag for an
adapter, and no module named after an external system.

## Simulation is architectural (§22)

A deterministic simulator is not "just for tests". If a system cannot
represent its important state transitions independently of real-world
timing and infrastructure, its semantics are probably insufficiently
explicit.

**Enforced by construction:** the crate has no wall-clock dependency
(ADR 0002). All temporal content is `LogicalTime` ticks under a world's
`TimeModel`, so every operation in this crate is representable — and
byte-reproducible — inside a deterministic simulator. `ExecutionWorld`
(§16) makes the execution environment itself a first-class canonical
object.

## Proof is not the only evidence (§23)

Formal proof is preferred where feasible, but the architecture must support
properties that cannot reasonably be proven, using proof, model checking,
simulation, conformance, property testing, fuzzing and differential testing
in a clearly distinguished evidence hierarchy.

**Enforced by types:** `EvidenceKind` is a closed enum of exactly these
kinds, and `AssuranceLevel` is a total order over them
(`Unknown < Benchmarked < FuzzTested < … < FullyProven`). Nothing in the
type system can express "verified" as a bare boolean.

## Never claim more than the evidence (§24)

The system should make it mechanically difficult to say "this component is
verified" when only a subset of properties is proven. Instead: property P is
`fully_proven`, property Q is `model_checked`, property R is `fuzz_tested`,
property S is `unknown`.

**Enforced mechanically:** `Claim::assess` computes, per property, the
strongest level its attached evidence actually supports (ignoring `Fail`
results and evidence about other subjects) and returns
[`AssessmentError::Overstated`] if any required obligation exceeds it.
Status output is always per-property (`status_lines()`), never a blanket
verdict. The dependency closure is assessed too, so a high-level claim
cannot quietly inherit assurance a dependency no longer has (spec §18).
