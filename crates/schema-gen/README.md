# schema-gen

Generates JSON Schema and CDDL definitions for the canonical types of
`tpt-primitives`, written into `schemas/`.

## Usage

```sh
cargo run -p schema-gen            # regenerate schemas/ from the Rust types
cargo run -p schema-gen -- --check # fail with a diff if schemas/ has drifted
```

CI runs `--check`, so a change to a canonical type cannot merge without its
schemas being regenerated.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
