//! Errors produced by canonical encoding and decoding.

use std::fmt;

/// Errors from the canonical encoding layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    /// The payload could not be serialized to canonical CBOR.
    Encode(String),
    /// The bytes are not a well-formed canonical envelope.
    NotAnEnvelope,
    /// The envelope carries a different schema version than the reading
    /// type understands. See `docs/evolution.md`.
    SchemaVersionMismatch {
        /// Version expected by the reading type.
        expected: u16,
        /// Version found in the envelope.
        found: u16,
    },
    /// The envelope magic does not match this library.
    UnknownMagic(String),
    /// The payload could not be deserialized as the requested type.
    Decode(String),
    /// Canonical bytes must be consumed exactly; extra trailing bytes are
    /// rejected rather than ignored.
    TrailingBytes(usize),
    /// A value violates a content invariant (e.g. empty authority domain).
    InvalidContent(String),
    /// Hex decoding of an identity string failed.
    BadHex(String),
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(msg) => write!(f, "canonical encode error: {msg}"),
            Self::NotAnEnvelope => write!(
                f,
                "not a canonical envelope (expected a 3-element CBOR array)"
            ),
            Self::SchemaVersionMismatch { expected, found } => {
                write!(
                    f,
                    "schema version mismatch: reader understands v{expected}, envelope is v{found}"
                )
            }
            Self::UnknownMagic(magic) => write!(f, "unknown canonical envelope magic: {magic:?}"),
            Self::Decode(msg) => write!(f, "canonical decode error: {msg}"),
            Self::TrailingBytes(n) => write!(f, "{n} trailing byte(s) after canonical payload"),
            Self::InvalidContent(msg) => write!(f, "invalid canonical content: {msg}"),
            Self::BadHex(msg) => write!(f, "bad identity hex: {msg}"),
        }
    }
}

impl std::error::Error for CanonicalError {}
