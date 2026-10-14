# ADR 0003: RFC 8949 key order and binary identities

Status: Accepted

Date: 2026-10-14

## Context

ADR 0001 described the encoding as "deterministic CBOR" but serialized struct
fields in declaration order. That is deterministic for Rust, yet it is not the
RFC 8949 section 4.2 profile, so an implementer in another language could not
reproduce an identity with a standard deterministic-CBOR library. Identities
were also embedded as 64-character hex text, doubling their size and making
the wire form differ from the digest they represent.

Nothing had been tagged, published or persisted, so changing the preimage was
free at this point and would not be later.

## Decision

1. **Map keys follow RFC 8949 section 4.2.1**: entries sorted bytewise by
   their encoded keys (so shorter keys sort first), recursively. This
   supersedes rule 1 of ADR 0001 ("declaration order"). Integers are
   shortest-form and lengths definite, as ciborium already emits. Floats stay
   forbidden.
2. **Identities are 32-byte CBOR byte strings** in canonical bytes. Human
   readable formats (JSON views, schemas) keep the 64-character lowercase hex
   form. `AnyId` keeps its hex `id` string.
3. **Decoding is strict.** `Canonical::from_canonical_bytes` re-encodes the
   decoded value and rejects any input that is not byte-identical
   (`CanonicalError::NonCanonical`).
4. **Golden vectors** in `testvectors/v1.json` pin canonical bytes and
   identities. `testvectors/verify.py` checks them with a standard CBOR
   library and plain SHA-256, independent of this crate.

## Consequences

- Every identity differs from builds before this change. Because no release
  existed, no schema version was bumped; from now on any change to these
  vectors needs a `SCHEMA_VERSION` bump (docs/evolution.md).
- Other languages can implement the profile with an off-the-shelf
  deterministic-CBOR encoder and verify against the vectors.
- Types that use internally tagged serde enums buffer through a path that
  reports itself human-readable, so `Id` deserialization accepts both forms.
  The strict re-encode check keeps hex-in-CBOR from becoming a second valid
  encoding.
