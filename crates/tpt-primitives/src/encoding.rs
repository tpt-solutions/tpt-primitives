//! Deterministic CBOR encoding for canonical objects (spec §4).
//!
//! Canonical bytes are always a 3-element CBOR array:
//!
//! ```text
//! ["tpt-canonical", schema_version, payload]
//! ```
//!
//! Carrying the schema version inside the identity preimage makes the
//! encoding self-describing and gives evolution rules a strict anchor
//! (see `docs/evolution.md`). The deterministic profile — declaration-order
//! fields, `BTreeSet`/`BTreeMap` only, no floats — is documented in
//! `docs/decisions/0001-canonical-encoding-and-identity.md`.
//!
//! Serde does the typed work through `cbor4ii`. The [`cbor`](crate::cbor)
//! module puts map keys into bytewise order and splits the envelope.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use serde::{Serialize, de::DeserializeOwned};

use crate::cbor::{self, Value};
use crate::error::CanonicalError;

/// Magic identifying bytes produced by this library's canonical encoder.
pub const ENVELOPE_MAGIC: &str = "tpt-canonical";

/// CBOR array header for a 3-element array (major type 4, length 3).
const ARRAY_OF_3: u8 = 0x83;

/// Serialize `payload` into canonical bytes under the given schema version.
///
/// For the types in this crate serialization is infallible (pure in-memory
/// CBOR, no floats, no maps with non-string keys), so the result is
/// unwrapped; a bug would panic loudly rather than corrupt identity.
pub fn encode_envelope<T: Serialize>(schema_version: u16, payload: &T) -> Vec<u8> {
    let payload_bytes = cbor4ii::serde::to_vec(Vec::new(), payload)
        .unwrap_or_else(|e| panic!("canonical CBOR encoding is infallible: {e}"));
    let payload_value = cbor::decode(&payload_bytes)
        .unwrap_or_else(|e| panic!("canonical CBOR encoding is infallible: {e}"));
    let envelope = Value::Array(alloc::vec![
        Value::Text(ENVELOPE_MAGIC.into()),
        Value::Integer(i128::from(schema_version)),
        payload_value,
    ]);
    cbor::encode_canonical(&envelope)
        .unwrap_or_else(|e| panic!("canonical CBOR encoding is infallible: {e}"))
}

/// Decode canonical bytes, returning the schema version and payload.
///
/// Strict by design: the magic and schema version are checked and any
/// trailing bytes after the payload are rejected.
pub fn decode_envelope<T: DeserializeOwned>(bytes: &[u8]) -> Result<(u16, T), CanonicalError> {
    if bytes.first() != Some(&ARRAY_OF_3) {
        return Err(CanonicalError::NotAnEnvelope);
    }
    let mut rest = &bytes[1..];
    let magic: String = take_item(&mut rest)?;
    if magic != ENVELOPE_MAGIC {
        return Err(CanonicalError::UnknownMagic(magic));
    }
    let version: u16 = take_item(&mut rest)?;
    let payload: T = take_item(&mut rest)?;
    if !rest.is_empty() {
        return Err(CanonicalError::TrailingBytes(rest.len()));
    }
    Ok((version, payload))
}

/// Splits one well-formed item off the front of `rest` and deserializes it.
fn take_item<T: DeserializeOwned>(rest: &mut &[u8]) -> Result<T, CanonicalError> {
    let len = cbor::item_len(rest).map_err(cbor_error)?;
    let (item, tail) = rest.split_at(len);
    *rest = tail;
    cbor4ii::serde::from_slice(item).map_err(|e| CanonicalError::Decode(e.to_string()))
}

fn cbor_error(e: cbor::Error) -> CanonicalError {
    match e {
        // Indefinite lengths have no canonical form, so report them as such.
        cbor::Error::Indefinite => CanonicalError::NonCanonical,
        other => CanonicalError::Decode(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cbor::Value;
    use alloc::{string::ToString, vec};

    #[test]
    fn envelope_round_trip() {
        let bytes = encode_envelope(7, &"hello");
        let (version, payload): (u16, String) = decode_envelope(&bytes).unwrap();
        assert_eq!(version, 7);
        assert_eq!(payload, "hello");
    }

    #[test]
    fn rejects_bad_header() {
        assert_eq!(
            decode_envelope::<String>(&[0x82, 0x01, 0x02]),
            Err(CanonicalError::NotAnEnvelope)
        );
        assert_eq!(
            decode_envelope::<String>(&[]),
            Err(CanonicalError::NotAnEnvelope)
        );
    }

    #[test]
    fn rejects_unknown_magic() {
        let buf = cbor4ii::serde::to_vec(Vec::new(), &("other-magic", 1u16, "x")).unwrap();
        assert!(matches!(
            decode_envelope::<String>(&buf),
            Err(CanonicalError::UnknownMagic(_))
        ));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = encode_envelope(1, &"x");
        bytes.push(0x00);
        assert!(matches!(
            decode_envelope::<String>(&bytes),
            Err(CanonicalError::TrailingBytes(1))
        ));
    }

    #[test]
    fn map_keys_use_rfc8949_order() {
        #[derive(Serialize)]
        struct Fields {
            zebra: u8,
            b: u8,
            aa: u8,
        }
        // Length-first bytewise order: "b" (1 byte) < "aa" < "zebra".
        let bytes = encode_envelope(
            1,
            &Fields {
                zebra: 1,
                b: 2,
                aa: 3,
            },
        );
        let (_, v): (u16, Value) = decode_envelope(&bytes).unwrap();
        let Value::Map(entries) = v else {
            panic!("expected a map")
        };
        let keys: Vec<_> = entries
            .iter()
            .map(|(k, _)| k.as_text().unwrap().to_string())
            .collect();
        assert_eq!(keys, vec!["b", "aa", "zebra"]);
    }

    #[test]
    fn deterministic_output() {
        assert_eq!(encode_envelope(1, &"x"), encode_envelope(1, &"x"));
    }
}
