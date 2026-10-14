# ADR 0004: Signing lives in a separate crate and signs identities

Status: Accepted

Date: 2026-10-14

## Context

Nothing in the primitives was authenticated. `Provenance::produced_by` is a
label, evidence carries no author, capabilities carry no issuer. A claim
that "the build service saw this pass" is only as good as the channel that
delivered it.

## Decision

1. **A separate optional crate, `tpt-sign`.** `tpt-primitives` stays free of
   cryptographic key handling; consumers that only need identities do not
   compile Ed25519.
2. **Ed25519** (`ed25519-dalek`, `verify_strict`). Signatures are
   deterministic, so they can be pinned as golden vectors and reproduced by
   any RFC 8032 implementation.
3. **Sign the identity, with domain separation:**
   `"tpt-sign.v1" || 0x00 || identity_domain || 0x00 || identity`.
   The identity already commits to the schema-versioned canonical bytes, and
   the class domain stops a signature on one kind of object being replayed
   as another.
4. **`Signed<T>` is itself canonical** and its `validate()` verifies the
   signature, so decoding a forged envelope fails rather than yielding a
   value. It is not an identity class: the identity of the thing signed is
   unchanged by who signed it.
5. **Seeds are supplied by the caller.** The crate never reads the clock or a
   random number generator (ADR 0002 spirit; key generation is a boundary
   concern).

## Not decided here

- Which keys may sign which kinds of object, how keys map to principals, key
  distribution and revocation. These are Fabric/Concord policy; a capability
  granting "attest evidence for domain X" is the intended expression.
- Multiple signatures per object (threshold or countersigning). `Signed<T>`
  nests (`Signed<Signed<T>>` countersigns the envelope) until a real need
  shapes a better type.

## Consequences

- `testvectors/signed-v1.json` pins the message construction and envelope,
  verified independently by `testvectors/verify.py`.
- Changing the message format is a breaking event: bump `SIGNATURE_DOMAIN`.
