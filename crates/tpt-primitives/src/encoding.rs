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

use serde::{Serialize, de::DeserializeOwned};

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
pub fn encode_envelope<T: Serialize + ?Sized>(schema_version: u16, payload: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::ser::into_writer(&(ENVELOPE_MAGIC, schema_version, payload), &mut buf)
        .unwrap_or_else(|e| panic!("canonical CBOR encoding is infallible: {e}"));
    buf
}

/// Decode canonical bytes, returning the schema version and payload.
///
/// Strict by design: the magic and schema version are checked and any
/// trailing bytes after the payload are rejected.
pub fn decode_envelope<T: DeserializeOwned>(bytes: &[u8]) -> Result<(u16, T), CanonicalError> {
    let mut cursor = CountingCursor::new(bytes);
    let header = cursor.read_u8().ok_or(CanonicalError::NotAnEnvelope)?;
    if header != ARRAY_OF_3 {
        return Err(CanonicalError::NotAnEnvelope);
    }
    let magic: String = ciborium::de::from_reader(&mut cursor).map_err(|e| cbor(&e))?;
    if magic != ENVELOPE_MAGIC {
        return Err(CanonicalError::UnknownMagic(magic));
    }
    let version: u16 = ciborium::de::from_reader(&mut cursor).map_err(|e| cbor(&e))?;
    let payload: T = ciborium::de::from_reader(&mut cursor).map_err(|e| cbor(&e))?;
    if cursor.position() != bytes.len() {
        return Err(CanonicalError::TrailingBytes(
            bytes.len() - cursor.position(),
        ));
    }
    Ok((version, payload))
}

fn cbor(e: &ciborium::de::Error<std::io::Error>) -> CanonicalError {
    CanonicalError::Decode(e.to_string())
}

/// A `Read` adapter over a byte slice that tracks how much was consumed,
/// so decoded payloads can be checked for exact consumption.
struct CountingCursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> CountingCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn position(&self) -> usize {
        self.pos
    }

    fn read_u8(&mut self) -> Option<u8> {
        let b = self.data.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }
}

impl std::io::Read for CountingCursor<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = &self.data[self.pos.min(self.data.len())..];
        let n = remaining.len().min(buf.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&("other-magic", 1u16, "x"), &mut buf).unwrap();
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
    fn deterministic_output() {
        assert_eq!(encode_envelope(1, &"x"), encode_envelope(1, &"x"));
    }
}
