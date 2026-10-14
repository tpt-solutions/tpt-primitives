//! `tpt`: look inside TPT canonical objects without writing Rust.
//!
//! ```text
//! tpt inspect <file|-> [--hex]          header, payload, canonical-form check
//! tpt id <domain> <file|-> [--hex]      identity: SHA-256(domain || 0x00 || bytes)
//! tpt diff <a> <b> [--hex]              which payload fields differ
//! ```
//!
//! Input is canonical bytes (a file, or `-` for stdin); `--hex` reads them
//! as hex text instead, as found in `testvectors/v1.json`.

use std::io::Read;
use std::process::ExitCode;

use ciborium::value::Value;
use serde_json::{Map, json};
use tpt_primitives::encoding::{decode_envelope, encode_envelope};
use tpt_primitives::hash::{from_hex_vec, sha256, to_hex};

const USAGE: &str = "usage:
  tpt inspect <file|-> [--hex]
  tpt id <domain> <file|-> [--hex]
  tpt diff <a> <b> [--hex]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("tpt: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let hex = args.iter().any(|a| a == "--hex");
    let positional: Vec<&str> = args
        .iter()
        .filter(|a| *a != "--hex")
        .map(String::as_str)
        .collect();
    match positional.as_slice() {
        ["inspect", input] => inspect(&load(input, hex)?),
        ["id", domain, input] => Ok(identity(domain, &load(input, hex)?)),
        ["diff", a, b] => diff(&load(a, hex)?, &load(b, hex)?),
        _ => Err(USAGE.to_owned()),
    }
}

fn load(input: &str, hex: bool) -> Result<Vec<u8>, String> {
    let raw = if input == "-" {
        let mut buf = Vec::new();
        std::io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| format!("reading stdin: {e}"))?;
        buf
    } else {
        std::fs::read(input).map_err(|e| format!("reading {input}: {e}"))?
    };
    if hex {
        let text = String::from_utf8(raw).map_err(|_| "hex input is not UTF-8".to_owned())?;
        from_hex_vec(text.trim()).map_err(|e| e.to_string())
    } else {
        Ok(raw)
    }
}

/// Decode canonical bytes into `(schema_version, payload)`.
fn decode(bytes: &[u8]) -> Result<(u16, Value), String> {
    decode_envelope::<Value>(bytes).map_err(|e| e.to_string())
}

/// SHA-256 over domain separation and bytes, as `Id::derive` does.
fn identity(domain: &str, bytes: &[u8]) -> String {
    let mut input = Vec::with_capacity(domain.len() + 1 + bytes.len());
    input.extend_from_slice(domain.as_bytes());
    input.push(0);
    input.extend_from_slice(bytes);
    to_hex(&sha256(&input))
}

fn inspect(bytes: &[u8]) -> Result<String, String> {
    let (version, payload) = decode(bytes)?;
    let canonical = encode_envelope(version, &payload) == bytes;
    let view = json!({
        "schema_version": version,
        "canonical_form": canonical,
        "bytes": bytes.len(),
        "payload": to_json(&payload),
    });
    serde_json::to_string_pretty(&view).map_err(|e| e.to_string())
}

fn diff(a: &[u8], b: &[u8]) -> Result<String, String> {
    let (va, pa) = decode(a)?;
    let (vb, pb) = decode(b)?;
    let mut lines = Vec::new();
    if va != vb {
        lines.push(format!("schema_version: {va} -> {vb}"));
    }
    match (to_json(&pa), to_json(&pb)) {
        (serde_json::Value::Object(ma), serde_json::Value::Object(mb)) => {
            let mut keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                match (ma.get(key), mb.get(key)) {
                    (Some(x), Some(y)) if x == y => {}
                    (x, y) => lines.push(format!("{key}: {} -> {}", show(x), show(y))),
                }
            }
        }
        (x, y) if x != y => lines.push(format!("payload: {x} -> {y}")),
        _ => {}
    }
    if lines.is_empty() {
        Ok("identical".to_owned())
    } else {
        Ok(lines.join("\n"))
    }
}

fn show(value: Option<&serde_json::Value>) -> String {
    value.map_or_else(|| "(absent)".to_owned(), ToString::to_string)
}

/// Render a CBOR value as JSON; byte strings become `0x…` hex text.
fn to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Integer(i) => {
            let n = i128::from(*i);
            u64::try_from(n).map_or_else(
                |_| i64::try_from(n).map_or_else(|_| json!(n.to_string()), |v| json!(v)),
                |v| json!(v),
            )
        }
        Value::Bytes(b) => json!(format!("0x{}", to_hex(b))),
        Value::Text(t) => json!(t),
        Value::Bool(b) => json!(b),
        Value::Null => serde_json::Value::Null,
        Value::Array(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        Value::Map(entries) => {
            let mut map = Map::new();
            for (k, v) in entries {
                let key = match k {
                    Value::Text(t) => t.clone(),
                    other => to_json(other).to_string(),
                };
                map.insert(key, to_json(v));
            }
            serde_json::Value::Object(map)
        }
        Value::Tag(_, inner) => to_json(inner),
        other => json!(format!("{other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(a: u64, b: &str) -> Vec<u8> {
        let payload = Value::Map(vec![
            (Value::Text("count".into()), Value::Integer(a.into())),
            (Value::Text("name".into()), Value::Text(b.into())),
        ]);
        encode_envelope(1, &payload)
    }

    #[test]
    fn inspect_reports_version_payload_and_canonical_form() {
        let out = inspect(&bytes(3, "x")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["canonical_form"], true);
        assert_eq!(v["payload"]["count"], 3);
    }

    #[test]
    fn inspect_flags_non_canonical_bytes() {
        // Version 1 as the non-minimal integer 0x18 0x01.
        let mut raw = vec![0x83, 0x6d];
        raw.extend(b"tpt-canonical");
        raw.extend([0x18, 0x01, 0x61, b'x']);
        let v: serde_json::Value = serde_json::from_str(&inspect(&raw).unwrap()).unwrap();
        assert_eq!(v["canonical_form"], false);
    }

    #[test]
    fn identity_matches_the_documented_construction() {
        let raw = bytes(1, "a");
        let mut input = b"tpt.test.v1\0".to_vec();
        input.extend(&raw);
        assert_eq!(identity("tpt.test.v1", &raw), to_hex(&sha256(&input)));
    }

    #[test]
    fn diff_names_the_changed_fields_only() {
        assert_eq!(diff(&bytes(1, "a"), &bytes(1, "a")).unwrap(), "identical");
        let out = diff(&bytes(1, "a"), &bytes(2, "a")).unwrap();
        assert_eq!(out, "count: 1 -> 2");
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(inspect(&[1, 2, 3]).is_err());
        assert!(run(&["bogus".to_owned()]).is_err());
    }
}
