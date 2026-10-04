# ADR 0002: Logical time instead of wall-clock time

Status: Accepted

Date: 2026-10-05

## Context

Spec §3 forbids deriving identity from wall-clock timestamps. Spec §16
requires a canonical ExecutionWorld with an explicit *time model*, and
spec §22 states that simulation is architectural: a system must be able to
represent its important state transitions independently of real-world
timing. Leases (§6) need temporal validity; reservations (§8) need
deadlines and expiry; executions (§11) need start/end ordering.

Using `SystemTime` anywhere in canonical content would make identities
irreproducible and would make simulation second-class — exactly what §22
warns against.

## Decision

This crate has no wall-clock dependency. All temporal content is expressed
as `LogicalTime(u64)` — a monotonically increasing tick counter whose unit
is fixed by the ExecutionWorld's `TimeModel` (e.g. "1 tick = 1 millisecond").

- Lease validity windows, reservation deadlines and expiry, execution
  start/finish markers, and provenance timestamps are all logical ticks.
- A real deployment maps ticks to wall-clock time at its boundary (Fabric
  owns that mapping); the primitive itself never consults a clock.

## Consequences

- Everything in this crate is deterministic and simulation-first: the same
  operation sequence in the same world yields byte-identical objects and
  identities.
- Ordering ("did the lease expire before the reservation committed?") is
  answerable from data alone, without clock-skew reasoning.
- Wall-clock observability can be layered on later as non-canonical
  metadata outside the identity preimage.
