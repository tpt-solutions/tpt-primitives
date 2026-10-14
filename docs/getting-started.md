# Getting started

Ten minutes, no prior knowledge of the spec.

## 1. Run the examples

```text
cargo run -p tpt-primitives --example quickstart             # computation -> artifact -> claim
cargo run -p tpt-primitives --example capability_delegation  # attenuate, revoke, lease, epoch
cargo run -p tpt-primitives --example assurance_graph        # dependencies and refutation
```

Then read the code: each is about 100 lines.

## 2. The one idea

Every object has an **identity derived from its canonical bytes**. Same
meaning, same ID, on any machine. Change the meaning, the ID changes. IDs of
different classes never collide (`Id<T>` carries the class in its type).

```rust
use tpt_primitives::prelude::*;

let epoch = Epoch::initial(AuthorityDomain::new("fabric").unwrap());
let id: EpochId = epoch.identity();              // 32-byte SHA-256 identity
let bytes = epoch.canonical_bytes();             // schema-versioned deterministic CBOR
assert_eq!(Epoch::from_canonical_bytes(&bytes).unwrap(), epoch);
```

## 3. Which type do I need?

| I want to say... | Use | Identity changes when... |
|---|---|---|
| who may do what, in which domain | `Capability` (`CapabilityGrant`) | grants, holder, bindings change (not on revoke) |
| that authority is time-limited | `Lease` | window or holder changes |
| that old authority must die after a failover | `Epoch` + `EpochRequirement` | counter changes |
| what I want computed and with what resources | `Intent` | any field changes |
| that I am holding resources for an intent | `Reservation` (spec is the identity) | spec changes (not on state change) |
| exactly what a computation is | `Computation` | operation, inputs, environment, toolchain... change |
| how an output was derived | `Derivation`, `Provenance` | computation or inputs change |
| that one run happened | `Execution`, `Trace` | computation, world, attempt, times change |
| what world it ran in | `ExecutionWorld` | any declared world property changes |
| a measurement or proof | `Evidence` | subject, property, kind, result... change |
| a statement of trust | `Claim` | subject, property, scope, evidence, dependencies change |
| a failure someone found | `Counterexample` | any field changes |

## 4. Where to start, per project

- **Repro** (what happened): `Computation`, `artifact_of_content`, `Derivation`,
  `Provenance`, `ReproductionRequest`. Start with `examples/quickstart.rs`.
- **Fabric** (where and how it ran): `Capability`, `Lease`, `Epoch`,
  `valid_authority_with_ancestors`, `Intent`, `Reservation`. Start with
  `examples/capability_delegation.rs`.
- **Concord** (why trust it): `Evidence`, `Claim`, `Obligation`,
  `AssuranceStore`, `Claim::to_dot`. Start with `examples/assurance_graph.rs`.

## 5. Inspect objects from the shell

```text
cargo run -p tpt-cli -- inspect object.cbor           # header, payload, canonical-form check
cargo run -p tpt-cli -- id tpt.lease.v1 object.cbor   # recompute an identity
cargo run -p tpt-cli -- diff a.cbor b.cbor            # which fields differ
```

Add `--hex` when the input is hex text, as in `testvectors/v1.json`.

## 6. Sign things

`tpt-sign` adds `Signed<T>`: an object plus an Ed25519 signature over its
identity. Decoding a forged envelope fails.

```text
cargo run -p tpt-sign --example signed_evidence
```

Who a key *is* and what it may sign is policy for Fabric and Concord (ADR 0004).

## 7. Use it from another language

Canonical bytes are RFC 8949 section 4.2 deterministic CBOR in an envelope
`["tpt-canonical", schema_version, payload]`; identity is
`SHA-256(domain || 0x00 || bytes)` (ADR 0001, ADR 0003). `testvectors/v1.json`
holds expected bytes and IDs; `testvectors/verify.py` shows the checks in
about 20 lines.

## Glossary

- **Canonical bytes**: the deterministic encoding an identity is computed from.
- **Identity class**: the kind of thing an ID names (`ComputationId`, `ClaimId`...).
- **Authority domain**: the namespace a capability is valid in.
- **Epoch**: a per-domain counter; bumping it invalidates capabilities bound to the old value.
- **Lease**: authority limited to a logical-time window.
- **Logical time**: ticks, never wall-clock (ADR 0002).
- **Assurance level**: how strong the evidence for a property is (`unknown` ... `fully_proven`).
- **Overstatement**: a claim requiring more assurance than its evidence supports; always rejected.
- **Refutation**: a failing result for the same subject and property; the property reads `unknown`.
