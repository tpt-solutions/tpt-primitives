//! Generic canonical identity type (spec §3).
//!
//! [`Id<T>`] is the single newtype shared by every identity class. The type
//! parameter `T` is a zero-cost marker implementing [`IdClass`], which
//! supplies the domain-separation label used during derivation
//! (ADR 0001): `SHA-256(class_domain || 0x00 || content)`.
//!
//! Identity is derived from canonical semantics only — never from
//! filenames, auto-increment counters, addresses, mutable labels or
//! wall-clock timestamps (spec §3).

use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CanonicalError;
use crate::hash;

/// Domain-separation label for an identity class.
///
/// Implemented by zero-sized marker types; the label must be stable for
/// the lifetime of the identity class (changing it changes every derived
/// identity and is a breaking event).
pub trait IdClass {
    /// Domain label, e.g. `"tpt.computation.v1"`.
    const DOMAIN: &'static str;
}

/// A 32-byte canonical identity tagged with its class.
pub struct Id<T: IdClass> {
    bytes: [u8; 32],
    _marker: PhantomData<fn() -> T>,
}

impl<T: IdClass> Id<T> {
    /// The zero identity, for tests and placeholders. Never derives from
    /// real content; do not persist.
    pub const ZERO: Self = Self {
        bytes: [0u8; 32],
        _marker: PhantomData,
    };

    /// Derive an identity from canonical content with class domain separation.
    pub fn derive(content: &[u8]) -> Self {
        let mut input = Vec::with_capacity(T::DOMAIN.len() + 1 + content.len());
        input.extend_from_slice(T::DOMAIN.as_bytes());
        input.push(0x00);
        input.extend_from_slice(content);
        Self::from_raw(hash::sha256(&input))
    }

    /// Wrap raw digest bytes (no derivation).
    pub const fn from_raw(bytes: [u8; 32]) -> Self {
        Self {
            bytes,
            _marker: PhantomData,
        }
    }

    /// The raw 32 digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Lowercase hex rendering (64 characters).
    pub fn hex(&self) -> String {
        hash::to_hex(&self.bytes)
    }

    /// Parse from a 64-character hex string.
    pub fn from_hex(hex: &str) -> Result<Self, CanonicalError> {
        Ok(Self::from_raw(hash::from_hex(hex)?))
    }

    /// True if this is [`Id::ZERO`].
    pub fn is_zero(&self) -> bool {
        self.bytes == [0u8; 32]
    }
}

impl<T: IdClass> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: IdClass> Copy for Id<T> {}

impl<T: IdClass> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl<T: IdClass> Eq for Id<T> {}

impl<T: IdClass> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: IdClass> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bytes.cmp(&other.bytes)
    }
}

impl<T: IdClass> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}

impl<T: IdClass> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

impl<T: IdClass> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", T::DOMAIN, self.hex())
    }
}

impl<T: IdClass> Serialize for Id<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex())
    }
}

impl<'de, T: IdClass> Deserialize<'de> for Id<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Self::from_hex(&hex).map_err(serde::de::Error::custom)
    }
}

// `schemars` support: every identity renders as a lowercase 64-char hex string.
impl<T: IdClass> schemars::JsonSchema for Id<T> {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!("TptId_{}", sanitize(T::DOMAIN)))
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^[0-9a-f]{64}$"
        })
    }
}

fn sanitize(domain: &str) -> String {
    domain
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// A class-erased identity: the identity plus the class label it belongs to.
///
/// Used where a primitive must reference "some canonical object of some
/// class" — e.g. the subject of an [`crate::evidence::Evidence`] or a
/// [`crate::evidence::Claim`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct AnyId {
    /// The identity class domain, e.g. `"tpt.computation.v1"`.
    pub class: String,
    /// Lowercase hex rendering of the identity.
    pub id: String,
}

impl AnyId {
    /// Erase the class of an [`Id`].
    pub fn from_id<T: IdClass>(id: &Id<T>) -> Self {
        Self {
            class: T::DOMAIN.to_owned(),
            id: id.hex(),
        }
    }
}

impl<T: IdClass> PartialEq<Id<T>> for AnyId {
    fn eq(&self, other: &Id<T>) -> bool {
        self.class == T::DOMAIN && self.id == other.hex()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTag;
    impl IdClass for TestTag {
        const DOMAIN: &'static str = "tpt.test.v1";
    }

    struct OtherTag;
    impl IdClass for OtherTag {
        const DOMAIN: &'static str = "tpt.other.v1";
    }

    type TestId = Id<TestTag>;
    type OtherId = Id<OtherTag>;

    #[test]
    fn derivation_is_deterministic() {
        let a = TestId::derive(b"content");
        let b = TestId::derive(b"content");
        assert_eq!(a, b);
        assert_ne!(TestId::derive(b"content"), TestId::derive(b"content "));
    }

    #[test]
    fn domain_separation() {
        // Same content in different classes must not collide.
        assert_ne!(
            TestId::derive(b"same").as_bytes(),
            OtherId::derive(b"same").as_bytes()
        );
        // And matches the documented construction.
        let expected = hash::sha256(b"tpt.test.v1\x00same");
        assert_eq!(TestId::derive(b"same").as_bytes(), &expected);
    }

    #[test]
    fn hex_round_trip() {
        let id = TestId::derive(b"hex");
        assert_eq!(Id::from_hex(&id.hex()).unwrap(), id);
        assert!(TestId::from_hex("nothex").is_err());
    }

    #[test]
    fn zero_id_is_detectable() {
        assert!(TestId::ZERO.is_zero());
        assert!(!TestId::derive(b"x").is_zero());
    }

    #[test]
    fn ordering_is_by_bytes() {
        let mut ids = [TestId::derive(b"b"), TestId::derive(b"a")];
        ids.sort();
        assert_eq!(ids[0], TestId::derive(b"a"));
    }
}
