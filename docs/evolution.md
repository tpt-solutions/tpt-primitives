# Schema versioning and evolution policy

Every canonical object in `tpt-primitives` carries its schema version inside
its canonical encoding (ADR 0001): the preimage is
`["tpt-canonical", schema_version, payload]`. This document defines what
counts as a breaking change and how versions move.

## Version model

- Each canonical type pins a `SCHEMA_VERSION` constant (starting at 1).
- The version is part of the identity preimage: **a schema change that
  changes encoding semantics produces different identities for newly
  created objects.** This is intentional — identity follows semantics.
- Existing identities never change: bytes already written remain valid
  evidence of the object that produced them.

## Non-breaking changes (patch/minor release, no schema bump)

- Rust-side refactors that do not change the canonical bytes of any type
  (builder ergonomics, docs, accessors).
- Adding **new types** or new **variants** used only in new objects.
- Bug fixes where the old encoding was produced but never persisted
  (requires an explicit deprecation note in `CHANGELOG.md`).

## Compatible evolution (schema minor bump, e.g. 1 → 2)

- Adding an optional field with a default.
- Widening a set-valued field in a way old writers could also express
  (empty set).
- Old readers **must reject** the newer version cleanly (strict envelope
  check) rather than misread it; new readers may provide an explicit
  `from_vN` upgrade helper.

## Breaking evolution (schema major move within 0.x, or 1.x major bump)

- Removing or renaming fields, changing field types or ordering semantics,
  changing the normalization rules, or changing the hash function.
- Old identities remain valid for the old bytes; cross-version identity
  equivalence must never be claimed implicitly.

## Rules for contributors

1. Never edit a canonical type's meaning without bumping its
   `SCHEMA_VERSION`.
2. Identity-relevant behavior is locked by tests (`identity_stable`,
   `identity_changes_on_semantics`); a PR that changes encoding must show
   the version bump.
3. Generated schemas under `schemas/` must be regenerated in the same PR
   (`cargo run -p schema-gen`) — CI fails on drift.
