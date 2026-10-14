# ADR 0005: Absent optionals are omitted; claims carry checkable scope

Status: Accepted

Date: 2026-10-14

## Context

**Encoding mismatch.** The CDDL marks optional fields with `?`, meaning the
key is absent. serde encodes `None` as CBOR `null`, so 21 optional fields
disagreed with the spec. The byte-level verifier could not catch this: it
checks that bytes re-encode consistently, not that they match the schema.

**Unchecked scope.** `Claim::scope` is free text. Evidence was matched to a
claim by subject and property only, so a result from one platform could
support, or refute, a claim about another. Evidence also recorded no
platform at all.

## Decision

1. **Absent optionals are omitted.** Every `Option` field in a canonical
   type is annotated `skip_serializing_if = "Option::is_none"`. The encoding
   now matches the CDDL. A test asserts that no optional is ever encoded as
   `null`.
2. **Schema version 2 for every canonical type.** Item 1 changes the bytes of
   any object with an absent optional, so the version moves for all of them,
   not just the ones with new fields. This is one deliberate breaking event
   before anything is tagged or persisted.
3. **Evidence records its platform** as `Option<PlatformSpec>`. `None` means
   platform-independent evidence, such as a proof. A claim that requires a
   platform does not count such evidence.
4. **Claims carry a `ScopeRequirement`:** optional platform, optional
   toolchain, and required environment variables (exact match on the listed
   names; extra variables allowed). `Claim::scope` stays as human-readable
   text, but assessment checks only the requirement.
5. **Out-of-scope evidence is ignored entirely**, including failures. A
   failure on aarch64 cannot refute a claim scoped to x86_64. This is the same
   rule as "not about this subject", extended to scope.

## Alternatives considered

- *Keep `null` and change the CDDL instead.* Rejected: `null` for every absent
  optional makes canonical bytes larger and forces every implementer to
  special-case it. The `?` notation is the more common deterministic form.
- *Match scope by parsing the `scope` string.* Rejected: free text cannot be
  checked mechanically, which is the whole point.

## Consequences

- Every golden vector changed (`testvectors/v1.json`, `signed-v1.json`);
  `verify.py` still passes against an independent CBOR and Ed25519 check.
- Existing `Evidence` and `Claim` literals gained `platform` and `requirement`
  fields. Code that wants platform-specific evidence sets `platform`.
- Scope does not yet cover a claim's *dependencies*: a dependency is assessed
  under its own requirement, which is the intended composition.
