//! Canonical primitive values.
//!
//! Canonical content must be deterministic, so this library defines its
//! own scalar value type instead of admitting arbitrary JSON-like values:
//! no floating-point numbers (ADR 0001), maps are `BTreeMap`s so key order
//! is sorted, and no `serde_json::Value` escape hatch.
//!
//! `PrimitiveValue` implements `serde` manually so each variant maps to a
//! fixed CBOR major type (text → string, bytes → byte string, etc.),
//! keeping the canonical encoding fully explicit.

use std::collections::BTreeMap;

use serde::de::{Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

/// A scalar value allowed in canonical content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, schemars::JsonSchema)]
#[schemars(rename_all = "snake_case")]
pub enum PrimitiveValue {
    /// A UTF-8 string.
    Text(String),
    /// Raw bytes.
    Bytes(Vec<u8>),
    /// An unsigned integer.
    Uint(u64),
    /// A signed integer.
    Int(i64),
    /// A boolean.
    Bool(bool),
    /// Absent value.
    Null,
}

impl Serialize for PrimitiveValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text(s) => serializer.serialize_str(s),
            Self::Bytes(b) => serializer.serialize_bytes(b),
            Self::Uint(u) => serializer.serialize_u64(*u),
            Self::Int(i) => serializer.serialize_i64(*i),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for PrimitiveValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl Visitor<'_> for ValueVisitor {
            type Value = PrimitiveValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a canonical primitive value")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Text(v.to_owned()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Text(v))
            }
            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Bytes(v.to_vec()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Bytes(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Uint(v))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Int(v))
            }
            fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                u64::try_from(v)
                    .map(PrimitiveValue::Uint)
                    .map_err(|_| E::custom("u128 value does not fit canonical u64"))
            }
            fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                i64::try_from(v)
                    .map(PrimitiveValue::Int)
                    .map_err(|_| E::custom("i128 value does not fit canonical i64"))
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Bool(v))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Null)
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(PrimitiveValue::Null)
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// A deterministic string-keyed map of primitive values.
pub type PrimitiveMap = BTreeMap<String, PrimitiveValue>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_ordering_is_sorted() {
        let mut map = PrimitiveMap::new();
        map.insert("zebra".into(), PrimitiveValue::Uint(1));
        map.insert("alpha".into(), PrimitiveValue::Uint(2));
        let a = crate::encoding::encode_envelope(1, &map);
        let mut map2 = PrimitiveMap::new();
        map2.insert("alpha".into(), PrimitiveValue::Uint(2));
        map2.insert("zebra".into(), PrimitiveValue::Uint(1));
        let b = crate::encoding::encode_envelope(1, &map2);
        assert_eq!(a, b, "insertion order must not affect canonical bytes");
    }

    #[test]
    fn all_variants_round_trip() {
        let values = vec![
            PrimitiveValue::Text("hi".into()),
            PrimitiveValue::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
            PrimitiveValue::Uint(u64::MAX),
            PrimitiveValue::Int(i64::MIN),
            PrimitiveValue::Bool(true),
            PrimitiveValue::Null,
        ];
        for value in values {
            let bytes = crate::encoding::encode_envelope(1, &value);
            let (v, back): (u16, PrimitiveValue) =
                crate::encoding::decode_envelope(&bytes).unwrap();
            assert_eq!(v, 1);
            assert_eq!(back, value);
        }
    }
}
