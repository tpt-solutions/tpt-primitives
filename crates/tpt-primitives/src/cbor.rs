// SPDX-License-Identifier: MIT OR Apache-2.0
//! A small CBOR value tree (the RFC 8949 subset this crate uses).
//!
//! Typed (de)serialization goes through serde and `cbor4ii`. This module does
//! the two things serde does not: it parses bytes into a [`Value`] tree, and
//! it writes a tree back in the deterministic profile of ADR 0001:
//!
//! - integers and lengths use the shortest form;
//! - lengths are always definite;
//! - map entries are sorted bytewise by their encoded keys (RFC 8949 §4.2.1).
//!
//! Decoding is strict about structure (truncation, reserved values, excessive
//! nesting, impossible lengths) but accepts non-minimal heads. Callers check
//! canonical form by re-encoding and comparing bytes, which is how
//! `Canonical::from_canonical_bytes` works.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Maximum nesting depth accepted by the parser and the writer.
const MAX_DEPTH: usize = 128;

/// A CBOR data item, without the floating-point width distinctions.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Major types 0 and 1, the full 64-bit range of each.
    Integer(i128),
    /// Major type 2.
    Bytes(Vec<u8>),
    /// Major type 7 with a 16-, 32- or 64-bit float payload. Canonical output
    /// always uses the 64-bit form, and this crate never produces floats.
    Float(f64),
    /// Major type 3.
    Text(String),
    /// Simple values 20 and 21.
    Bool(bool),
    /// Simple value 22.
    Null,
    /// Major type 4.
    Array(Vec<Value>),
    /// Major type 5.
    Map(Vec<(Value, Value)>),
    /// Major type 6.
    Tag(u64, Box<Value>),
}

impl Value {
    /// Returns the text if this value is a text string.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
}

/// Why a byte sequence is not a CBOR data item this module accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The input ends before the item does.
    Truncated,
    /// An indefinite-length item (or a stray `break`). Never canonical.
    Indefinite,
    /// A well-formed item this crate does not use (undefined, simple values).
    Unsupported(&'static str),
    /// Malformed input: reserved additional information, bad UTF-8, excessive
    /// nesting, a length larger than the input, or trailing bytes.
    Invalid(&'static str),
    /// An integer outside the range of CBOR major types 0 and 1.
    IntegerOutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("CBOR item is truncated"),
            Self::Indefinite => f.write_str("indefinite-length CBOR is not canonical"),
            Self::Unsupported(what) => write!(f, "unsupported CBOR value: {what}"),
            Self::Invalid(what) => write!(f, "invalid CBOR: {what}"),
            Self::IntegerOutOfRange => f.write_str("integer outside the CBOR range"),
        }
    }
}

impl core::error::Error for Error {}

/// Decodes exactly one data item that spans all of `bytes`.
///
/// # Errors
///
/// Returns an [`Error`] if `bytes` is not one well-formed item, or if bytes
/// remain after the item.
pub fn decode(bytes: &[u8]) -> Result<Value, Error> {
    let mut reader = Reader::new(bytes);
    let value = reader.value(0)?;
    if reader.pos != bytes.len() {
        return Err(Error::Invalid("trailing bytes after the item"));
    }
    Ok(value)
}

/// Returns the length in bytes of the single data item at the start of `bytes`,
/// after checking that it is well formed. Trailing bytes are not an error.
///
/// # Errors
///
/// Returns an [`Error`] if the first item is malformed.
pub fn item_len(bytes: &[u8]) -> Result<usize, Error> {
    let mut reader = Reader::new(bytes);
    reader.value(0)?;
    Ok(reader.pos)
}

/// Encodes `value` in the deterministic profile: shortest heads, definite
/// lengths, and map entries sorted bytewise by their encoded keys.
///
/// # Errors
///
/// Returns [`Error::IntegerOutOfRange`] for an integer that CBOR cannot
/// represent, and [`Error::Invalid`] if the tree is nested too deeply.
pub fn encode_canonical(value: &Value) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    write_value(value, &mut out, 0)?;
    Ok(out)
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.remaining() < n {
            return Err(Error::Truncated);
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn uint(&mut self, n: usize) -> Result<u64, Error> {
        let bytes = self.take(n)?;
        Ok(bytes.iter().fold(0u64, |acc, &b| (acc << 8) | u64::from(b)))
    }

    /// Reads an initial byte and its argument. For major type 7 the argument
    /// holds the raw float bits.
    fn head(&mut self) -> Result<(u8, u64, u8), Error> {
        let initial = self.take(1)?[0];
        let major = initial >> 5;
        let info = initial & 0x1f;
        let arg = match info {
            0..=23 => u64::from(info),
            24 => self.uint(1)?,
            25 => self.uint(2)?,
            26 => self.uint(4)?,
            27 => self.uint(8)?,
            31 => return Err(Error::Indefinite),
            _ => return Err(Error::Invalid("reserved additional information")),
        };
        Ok((major, arg, info))
    }

    /// A length prefix checked against the bytes left, so a hostile count
    /// cannot force a large allocation.
    fn count(&self, n: u64) -> Result<usize, Error> {
        let n = usize::try_from(n).map_err(|_| Error::Truncated)?;
        if n > self.remaining() {
            return Err(Error::Truncated);
        }
        Ok(n)
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::Invalid("nesting too deep"));
        }
        let (major, arg, info) = self.head()?;
        match major {
            0 => Ok(Value::Integer(i128::from(arg))),
            1 => Ok(Value::Integer(-1 - i128::from(arg))),
            2 => {
                let n = self.count(arg)?;
                Ok(Value::Bytes(self.take(n)?.to_vec()))
            }
            3 => {
                let n = self.count(arg)?;
                let text = core::str::from_utf8(self.take(n)?)
                    .map_err(|_| Error::Invalid("text is not UTF-8"))?;
                Ok(Value::Text(text.into()))
            }
            4 => {
                let n = self.count(arg)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.value(depth + 1)?);
                }
                Ok(Value::Array(items))
            }
            5 => {
                let n = self.count(arg)?.checked_mul(2).ok_or(Error::Truncated)?;
                if n > self.remaining() {
                    return Err(Error::Truncated);
                }
                let mut entries = Vec::with_capacity(n / 2);
                for _ in 0..n / 2 {
                    let key = self.value(depth + 1)?;
                    let value = self.value(depth + 1)?;
                    entries.push((key, value));
                }
                Ok(Value::Map(entries))
            }
            6 => Ok(Value::Tag(arg, Box::new(self.value(depth + 1)?))),
            _ => match info {
                20 => Ok(Value::Bool(false)),
                21 => Ok(Value::Bool(true)),
                22 => Ok(Value::Null),
                23 => Err(Error::Unsupported("undefined")),
                24 => Err(Error::Unsupported("simple value")),
                25 => {
                    let bits = u16::try_from(arg).map_err(|_| Error::Invalid("float payload"))?;
                    Ok(Value::Float(half_to_f64(bits)))
                }
                26 => {
                    let bits = u32::try_from(arg).map_err(|_| Error::Invalid("float payload"))?;
                    Ok(Value::Float(f64::from(f32::from_bits(bits))))
                }
                27 => Ok(Value::Float(f64::from_bits(arg))),
                _ => Err(Error::Invalid("reserved simple value")),
            },
        }
    }
}

/// Converts an IEEE 754 binary16 bit pattern to `f64` without `std`.
fn half_to_f64(bits: u16) -> f64 {
    let negative = bits & 0x8000 != 0;
    let exp = (bits >> 10) & 0x1f;
    let frac = bits & 0x3ff;
    let magnitude = if exp == 0 {
        // Subnormal: frac * 2^-24, exact in binary64.
        f64::from(frac) * (1.0 / 16_777_216.0)
    } else if exp == 0x1f {
        if frac == 0 { f64::INFINITY } else { f64::NAN }
    } else {
        // Normal: rebias the exponent from 15 to 1023 and widen the fraction.
        let exp64 = u64::from(exp) + 1023 - 15;
        f64::from_bits((exp64 << 52) | (u64::from(frac) << 42))
    };
    if negative { -magnitude } else { magnitude }
}

fn write_head(out: &mut Vec<u8>, major: u8, arg: u64) {
    let major = major << 5;
    if let Ok(n) = u8::try_from(arg) {
        if n < 24 {
            out.push(major | n);
        } else {
            out.push(major | 0x18);
            out.push(n);
        }
    } else if let Ok(n) = u16::try_from(arg) {
        out.push(major | 0x19);
        out.extend_from_slice(&n.to_be_bytes());
    } else if let Ok(n) = u32::try_from(arg) {
        out.push(major | 0x1a);
        out.extend_from_slice(&n.to_be_bytes());
    } else {
        out.push(major | 0x1b);
        out.extend_from_slice(&arg.to_be_bytes());
    }
}

fn write_value(value: &Value, out: &mut Vec<u8>, depth: usize) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::Invalid("nesting too deep"));
    }
    match value {
        Value::Integer(i) => {
            if *i >= 0 {
                let n = u64::try_from(*i).map_err(|_| Error::IntegerOutOfRange)?;
                write_head(out, 0, n);
            } else {
                let n = u64::try_from(-1 - *i).map_err(|_| Error::IntegerOutOfRange)?;
                write_head(out, 1, n);
            }
        }
        Value::Bytes(bytes) => {
            write_head(out, 2, bytes.len() as u64);
            out.extend_from_slice(bytes);
        }
        Value::Text(text) => {
            write_head(out, 3, text.len() as u64);
            out.extend_from_slice(text.as_bytes());
        }
        Value::Float(f) => {
            out.push(0xfb);
            out.extend_from_slice(&f.to_bits().to_be_bytes());
        }
        Value::Bool(false) => out.push(0xf4),
        Value::Bool(true) => out.push(0xf5),
        Value::Null => out.push(0xf6),
        Value::Array(items) => {
            write_head(out, 4, items.len() as u64);
            for item in items {
                write_value(item, out, depth + 1)?;
            }
        }
        Value::Map(entries) => {
            // Encode each key and value separately, then sort the pairs by the
            // encoded key. Sorting on whole encoded keys is the RFC 8949
            // §4.2.1 bytewise order.
            let mut halves: Vec<(Vec<u8>, Vec<u8>)> = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let mut k = Vec::new();
                write_value(key, &mut k, depth + 1)?;
                let mut v = Vec::new();
                write_value(value, &mut v, depth + 1)?;
                halves.push((k, v));
            }
            halves.sort_by(|a, b| a.0.cmp(&b.0));
            write_head(out, 5, entries.len() as u64);
            for (k, v) in halves {
                out.extend(k);
                out.extend(v);
            }
        }
        Value::Tag(tag, inner) => {
            write_head(out, 6, *tag);
            write_value(inner, out, depth + 1)?;
        }
    }
    Ok(())
}

impl serde::Serialize for Value {
    /// Serializes through serde. CBOR tags cannot be expressed in serde's data
    /// model, so a [`Value::Tag`] is written as its inner value. Re-encoding a
    /// tagged tree with serde therefore does not reproduce the tag; use
    /// [`encode_canonical`] to keep tags.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self {
            Self::Integer(i) => match u64::try_from(*i) {
                Ok(n) => serializer.serialize_u64(n),
                Err(_) => match i64::try_from(*i) {
                    Ok(n) => serializer.serialize_i64(n),
                    Err(_) => Err(serde::ser::Error::custom("integer outside the CBOR range")),
                },
            },
            Self::Bytes(bytes) => serializer.serialize_bytes(bytes),
            Self::Float(f) => serializer.serialize_f64(*f),
            Self::Text(text) => serializer.serialize_str(text),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Null => serializer.serialize_none(),
            Self::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Self::Map(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Self::Tag(_, inner) => inner.serialize(serializer),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AnyValue;

        impl<'de> serde::de::Visitor<'de> for AnyValue {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("any CBOR value")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
                Ok(Value::Integer(i128::from(v)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
                Ok(Value::Integer(i128::from(v)))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
                Ok(Value::Float(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Value, E> {
                Ok(Value::Text(v.into()))
            }

            fn visit_string<E>(self, v: String) -> Result<Value, E> {
                Ok(Value::Text(v))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
                Ok(Value::Bytes(v.to_vec()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Value, E> {
                Ok(Value::Bytes(v))
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_some<D: serde::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Value, D::Error> {
                <Value as serde::Deserialize<'de>>::deserialize(deserializer)
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Value, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Value::Array(items))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Value, A::Error> {
                let mut entries = Vec::new();
                while let Some((key, value)) = map.next_entry::<Value, Value>()? {
                    entries.push((key, value));
                }
                Ok(Value::Map(entries))
            }
        }

        deserializer.deserialize_any(AnyValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn heads_are_shortest_form() {
        assert_eq!(encode_canonical(&Value::Integer(23)).unwrap(), [0x17]);
        assert_eq!(encode_canonical(&Value::Integer(24)).unwrap(), [0x18, 24]);
        assert_eq!(
            encode_canonical(&Value::Integer(256)).unwrap(),
            [0x19, 0x01, 0x00]
        );
        assert_eq!(encode_canonical(&Value::Integer(-1)).unwrap(), [0x20]);
        assert_eq!(
            encode_canonical(&Value::Integer(i128::from(u64::MAX))).unwrap(),
            [0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn non_minimal_integer_decodes_and_re_encodes_minimal() {
        let value = decode(&[0x18, 0x01]).unwrap();
        assert_eq!(value, Value::Integer(1));
        assert_eq!(encode_canonical(&value).unwrap(), [0x01]);
    }

    #[test]
    fn map_keys_sort_by_encoded_bytes() {
        // "bb" encodes as 62 62 62 (length 2 head 0x62), "a" as 61 61: the
        // shorter key sorts first under bytewise order of encodings.
        let map = Value::Map(vec![
            (Value::Text("bb".into()), Value::Integer(1)),
            (Value::Text("a".into()), Value::Integer(2)),
        ]);
        assert_eq!(
            encode_canonical(&map).unwrap(),
            [0xa2, 0x61, b'a', 0x02, 0x62, b'b', b'b', 0x01]
        );
    }

    #[test]
    fn round_trip_preserves_tree() {
        let value = Value::Array(vec![
            Value::Null,
            Value::Bool(true),
            Value::Bytes(vec![1, 2, 3]),
            Value::Tag(55799, Box::new(Value::Text("x".into()))),
            Value::Integer(-1000),
        ]);
        let bytes = encode_canonical(&value).unwrap();
        assert_eq!(decode(&bytes).unwrap(), value);
    }

    #[test]
    fn floats_decode_from_all_widths() {
        assert_eq!(decode(&[0xf9, 0x3c, 0x00]).unwrap(), Value::Float(1.0));
        assert_eq!(
            decode(&[0xfa, 0x3f, 0x80, 0, 0]).unwrap(),
            Value::Float(1.0)
        );
        assert_eq!(
            decode(&[0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0]).unwrap(),
            Value::Float(1.5)
        );
        assert_eq!(half_to_f64(0x0001), 1.0 / 16_777_216.0);
    }

    #[test]
    fn malformed_input_is_rejected() {
        assert_eq!(decode(&[]), Err(Error::Truncated));
        assert_eq!(decode(&[0x5f]), Err(Error::Indefinite));
        assert_eq!(
            decode(&[0x1c]),
            Err(Error::Invalid("reserved additional information"))
        );
        assert_eq!(decode(&[0x62, 0x61]), Err(Error::Truncated));
        assert_eq!(
            decode(&[0x61, 0xff]),
            Err(Error::Invalid("text is not UTF-8"))
        );
        assert_eq!(
            decode(&[0x01, 0x02]),
            Err(Error::Invalid("trailing bytes after the item"))
        );
        assert_eq!(decode(&[0xf7]), Err(Error::Unsupported("undefined")));
        assert_eq!(
            decode(&[0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(Error::Truncated)
        );
    }

    #[test]
    fn excessive_nesting_is_rejected() {
        let mut bytes = vec![0x81; MAX_DEPTH + 2];
        bytes.push(0x00);
        assert_eq!(decode(&bytes), Err(Error::Invalid("nesting too deep")));
    }

    #[test]
    fn item_len_reports_first_item_only() {
        assert_eq!(item_len(&[0x82, 0x01, 0x02, 0xff]).unwrap(), 3);
    }
}
