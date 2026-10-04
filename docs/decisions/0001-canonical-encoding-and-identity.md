# ADR 0001: Canonical encoding and identity derivation

Status: Accepted

Date: 2026-10-05

## Context

Spec §3–4 require that every important object has a stable canonical identity:

- canonical representation
- explicit schema version
- deterministic encoding
- deterministic identity derivation
- explicit evolution rules

Identity must never be derived from filenames, database auto-increment IDs,
memory addresses, mutable labels or wall-clock timestamps. The same object
must produce the same identity on every machine, in every process, forever.
Identity must change when semantics change and must not change when only
irrelevant presentation changes.

## Decision

### Encoding: deterministic CBOR profile

Canonical objects are serialized with CBOR (RFC 8949) under a deterministic
profile enforced by this crate:

1. **Struct fields are serialized in declaration order.** Field order is
   pinned by the schema version; readers must not depend on map reordering.
2. **Set-valued fields use `BTreeSet`, map-valued fields use `BTreeMap`.**
   Iteration order is therefore sorted and independent of insertion order.
   `HashMap` is forbidden in canonical content.
3. **No floating-point values in canonical content.** Quantities are
   integers (with an explicit unit) or fixed-point; this avoids the
   non-determinism of float formatting and the pitfalls of float equality.
4. **Explicit schema-version envelope.** Every canonical byte string is a
   CBOR array `["tpt-canonical", schema_version, payload]`, so the schema
   version is carried inside the identity preimage itself. Decoding checks
   the envelope version strictly against the version the reading type
   understands (see `docs/evolution.md`).
5. Text is UTF-8; byte strings are raw bytes.

The profile is implemented on top of `serde` + `ciborium`; determinism comes
from the rules above, which the crate's round-trip and identity tests enforce.

### Hashing: SHA-256

Identity is `SHA-256(canonical_bytes)`, stored as 32 raw bytes and rendered
as lowercase hex (64 characters).

Rationale: cross-language interoperability matters more than raw speed for a
shared ecosystem primitive. SHA-256 is available everywhere (every language,
HSM, OCI, git, TLS tooling), is hardware-accelerated on modern CPUs, and has
no collision attacks of practical concern for content addressing. BLAKE3 was
considered and rejected for this release because out-of-Rust verification
tooling is far less ubiquitous; switching hash functions later is an
explicit, planned evolution step (new identity class versions, see
`docs/evolution.md`), not an in-place change.

### Domain separation

`Id<T>` derivation prefixes a class label before hashing:
`SHA-256(class_domain || 0x00 || canonical_bytes)`, e.g.
`"tpt.specification.v1"`. Two objects with identical canonical bytes in
different identity classes therefore get different identities, and an ID of
one class can never be confused with an ID of another.

## Consequences

- Identity is verifiable independently of Rust: anyone can recompute
  `SHA-256` over the documented CBOR envelope.
- Presentation changes (whitespace in JSON views, insertion order in sets,
  builder construction order) cannot change identity.
- Any semantic field change changes identity — enforced by tests.
- All hash inputs are finite canonical byte strings; no implicit context.
