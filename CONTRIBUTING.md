# Contributing

TPT Primitives is primitives only: identity, capability, intent, computation,
execution, evidence and world. Adapters and runtimes belong in Fabric, Repro
and Concord (`docs/design-rules.md`, spec section 21).

## Before you push

```text
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p schema-gen          # regenerate schemas/ if a canonical type changed
python testvectors/verify.py     # needs: pip install cbor2
```

CI runs the same checks plus `cargo doc -D warnings` and the examples.

## Changing a canonical type

Identity follows semantics, so a change to a canonical type's encoding is a
versioned event (`docs/evolution.md`):

1. Bump the type's `SCHEMA_VERSION`.
2. Regenerate the golden vectors: `TPT_BLESS=1 cargo test -p tpt-primitives --test vectors`.
3. Regenerate schemas with `cargo run -p schema-gen`.
4. Describe the change under `## [Unreleased]` in `CHANGELOG.md`.

The vectors test fails on any unannounced encoding change; that is its job.

## Adding a canonical type

Implement `Canonical` and `Identified` (with a new identity class in
`ids.rs`), add a `SchemaEntry` in `schema.rs`, keep fields deterministic
(`BTreeSet`/`BTreeMap`, no floats, no wall-clock time), validate in
constructors, and add a golden vector if the type is identity-bearing.

## Tests worth copying

- `tests/interop.rs`: the cross-project chain.
- `tests/properties.rs`: round-trip and strict-decoding properties.
- `examples/`: runnable walkthroughs.
