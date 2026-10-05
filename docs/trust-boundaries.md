# Trust boundaries (spec §19)

The three projects preserve a small, explicit trust boundary. `tpt-primitives`
defines the shared *shapes*; each project trusts only what is listed here and
nothing more. Anything not in this document is untrusted input to be checked.

## What Fabric trusts

- **Core semantics** — the meaning of intents, reservations, execution worlds
  and capabilities as defined by the canonical types in this crate.
- **Transport implementations only for their declared guarantees** — a
  transport is trusted exactly as far as the guarantees it explicitly
  declares (delivery, ordering, authenticity); no transport is trusted
  beyond its declaration.

## What Repro trusts

- **Canonicalization** — that canonical encoding/decoding (ADR 0001) maps
  values to a unique byte string.
- **Hashing** — SHA-256 as specified in ADR 0001 for identity derivation.
- **Artifact storage integrity** — that stored artifact bytes match the
  content address they were stored under.
- **Execution records** — that executions, traces and provenance records
  faithfully describe what ran.

## What Concord trusts

- **Evidence validity according to its evidence type** — a `Proof` was
  actually checked by the named kernel; a `FuzzCampaign` actually ran the
  named number of executions; and so on.
- **External proof kernels** where used — the kernel's soundness is trusted
  (and named in the evidence, so the trust is explicit and auditable).
- **Its own canonical assurance model** — the assessment rules implemented
  by `Claim::assess` (per-property levels, obligations, dependency closure).

## What nobody trusts

- **AI is never part of the trusted base.** Output produced with AI
  assistance is untrusted input like any other: it must carry the same
  evidence, pass the same conformance checks, and be derivable through the
  same provenance chain as anything else. No primitive in this crate grants
  authority, evidence weight or assurance based on the *origin* of a
  suggestion — only on the artifacts and evidence behind it.

## Consequence for this crate

`tpt-primitives` itself is inside every project's trusted base for *shape*
only: encoding, identity derivation and state-machine rules. It deliberately
contains no I/O, no clock access, no network, no storage — the mechanics
that would require trusting an implementation cannot hide here.
