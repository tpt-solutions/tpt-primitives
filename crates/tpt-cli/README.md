# tpt

Inspect, identify and diff TPT canonical objects from the command line, without
writing Rust.

## Commands

```text
tpt inspect <file|-> [--hex]          header, payload, canonical-form check
tpt id <domain> <file|-> [--hex]      identity: SHA-256(domain || 0x00 || bytes)
tpt diff <a> <b> [--hex]              which payload fields differ
```

Input is canonical bytes, read from a file or from stdin (`-`). With `--hex`,
the input is hex text instead, in the format used by `testvectors/v1.json`.

## Examples

```sh
cargo install --path crates/tpt-cli

tpt inspect object.cbor
tpt id claim object.cbor
tpt diff before.cbor after.cbor
```

## License

Dual-licensed under MIT or Apache-2.0, at your option.
