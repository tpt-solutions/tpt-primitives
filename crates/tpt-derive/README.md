# tpt-derive

`#[derive(Canonical)]` for `tpt-primitives` canonical types.

Writing `Canonical` and `Identified` by hand takes about three lines of
boilerplate per type, and it is easy to get the schema version or identity
class wrong. This derive generates both from one attribute:

```rust
#[derive(serde::Serialize, serde::Deserialize, tpt_derive::Canonical)]
#[canonical(version = 2, id = ClaimTag, validate = check_claim)]
pub struct Claim {
    // ...
}
```

## Attributes

| Attribute | Required | Meaning |
|-----------|----------|---------|
| `version = N` | yes | The schema version (see `docs/evolution.md`). |
| `id = Tag` | no | The identity class. Generates `Identified`. Omit it for content types that are not identities. |
| `validate = f` | no | A `fn(&Self) -> Result<(), CanonicalError>` run on decode via `Canonical::validate`. |

The type must also derive `serde::Serialize` and `serde::Deserialize`. Generic
types are not supported; canonical types in this workspace are concrete.

## Dependency

This crate is a proc-macro and is used through `tpt-primitives`. Add it as a
dev-dependency or a direct dependency alongside `tpt-primitives`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
