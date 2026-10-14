//! Signatures over TPT canonical object identities.
//!
//! `tpt-primitives` says *what* an object is; this crate lets a key holder
//! say *"I vouch for exactly that"*. A [`Signed<T>`] carries the object, the
//! signer's [`PublicKey`] and an Ed25519 [`Signature`] over a message that
//! binds the signature to the object's identity class and identity:
//!
//! ```text
//! message = "tpt-sign.v1" || 0x00 || identity_domain || 0x00 || identity (32 bytes)
//! ```
//!
//! Signing the identity (not a copy of the bytes) means a signature covers
//! precisely what the identity covers, and domain separation stops a
//! signature on one class of object being replayed as another.
//!
//! What this crate does **not** do: decide who a key *is*, or whether it
//! may sign a given kind of object. Mapping keys to principals, key
//! distribution and revocation belong to the ecosystem (Fabric capabilities
//! can express "this key may attest X"); trust boundaries stay as written in
//! `docs/trust-boundaries.md`.
//!
//! Decoding a [`Signed<T>`] from canonical bytes verifies the signature, so
//! a forged or corrupted envelope never becomes a value.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
use serde::de::{self, Deserializer, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use tpt_primitives::hash::{from_hex, to_hex};
use tpt_primitives::id::IdClass;
use tpt_primitives::{Canonical, CanonicalError, Identified};

/// Domain label that prefixes every signed message.
pub const SIGNATURE_DOMAIN: &str = "tpt-sign.v1";

/// Errors from verifying a signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignError {
    /// The public key bytes are not a valid Ed25519 key.
    InvalidKey,
    /// The signature does not match the object, class and key.
    BadSignature,
}

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => f.write_str("invalid Ed25519 public key"),
            Self::BadSignature => f.write_str("signature does not verify"),
        }
    }
}

impl std::error::Error for SignError {}

macro_rules! byte_array {
    ($(#[$meta:meta])* $name:ident, $len:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name([u8; $len]);

        impl $name {
            /// Wrap raw bytes.
            pub const fn from_bytes(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }

            /// The raw bytes.
            pub const fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }

            /// Lowercase hex rendering.
            pub fn hex(&self) -> String {
                to_hex(&self.0)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.hex())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.hex())
            }
        }

        // Raw bytes in canonical CBOR, hex in JSON (as for identities).
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                if serializer.is_human_readable() {
                    serializer.serialize_str(&self.hex())
                } else {
                    serializer.serialize_bytes(&self.0)
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct V;
                impl Visitor<'_> for V {
                    type Value = [u8; $len];
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{} bytes or their hex form", $len)
                    }
                    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                        <[u8; $len]>::try_from(v)
                            .map_err(|_| E::invalid_length(v.len(), &stringify!($len)))
                    }
                    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                        from_hex::<$len>(v).map_err(E::custom)
                    }
                }
                deserializer.deserialize_any(V).map(Self)
            }
        }
    };
}

byte_array!(
    /// An Ed25519 public key (32 bytes).
    PublicKey,
    32
);
byte_array!(
    /// An Ed25519 signature (64 bytes).
    Signature,
    64
);

/// The exact bytes that are signed for an object.
fn message<T: Identified>(object: &T) -> Vec<u8> {
    let domain = <T::Id as IdClass>::DOMAIN;
    let mut msg = Vec::with_capacity(SIGNATURE_DOMAIN.len() + domain.len() + 34);
    msg.extend_from_slice(SIGNATURE_DOMAIN.as_bytes());
    msg.push(0);
    msg.extend_from_slice(domain.as_bytes());
    msg.push(0);
    msg.extend_from_slice(object.identity().as_bytes());
    msg
}

/// A signing key. Created from a 32-byte seed supplied by the caller, so
/// this crate never touches a random number generator or the clock.
pub struct Signer {
    key: SigningKey,
}

impl fmt::Debug for Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never print key material.
        write!(f, "Signer({})", self.public_key())
    }
}

impl Signer {
    /// Derive a signing key from a 32-byte secret seed.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    /// The matching public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.key.verifying_key().to_bytes())
    }

    /// Sign an object's identity. Ed25519 is deterministic: the same key
    /// and object always give the same signature.
    pub fn sign<T: Identified>(&self, object: T) -> Signed<T> {
        let signature = Signature(self.key.sign(&message(&object)).to_bytes());
        Signed {
            object,
            signer: self.public_key(),
            signature,
        }
    }
}

/// An object together with a signature over its identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signed<T> {
    /// The signed object.
    pub object: T,
    /// The key that signed it.
    pub signer: PublicKey,
    /// Ed25519 signature over [`SIGNATURE_DOMAIN`], the identity class and
    /// the object's identity.
    pub signature: Signature,
}

impl<T: Identified> Signed<T> {
    /// Check the signature against the object's current identity.
    pub fn verify(&self) -> Result<(), SignError> {
        let key =
            VerifyingKey::from_bytes(self.signer.as_bytes()).map_err(|_| SignError::InvalidKey)?;
        let signature = ed25519_dalek::Signature::from_bytes(self.signature.as_bytes());
        // `verify_strict` also rejects small-order keys and non-canonical
        // signatures.
        key.verify_strict(&message(&self.object), &signature)
            .map_err(|_| SignError::BadSignature)
    }
}

/// A signed object is itself a canonical object; decoding verifies the
/// signature.
impl<T: Canonical + Identified> Canonical for Signed<T> {
    const SCHEMA_VERSION: u16 = 1;

    fn validate(&self) -> Result<(), CanonicalError> {
        self.object.validate()?;
        self.verify()
            .map_err(|e| CanonicalError::InvalidContent(format!("signed object: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tpt_primitives::capability::{AuthorityDomain, Epoch, Lease, Principal};
    use tpt_primitives::time::LogicalTime;

    fn seed(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn lease() -> Lease {
        Lease::new(
            AuthorityDomain::new("fabric").unwrap(),
            Principal::new("alice").unwrap(),
            LogicalTime::ZERO,
            LogicalTime::new(10),
        )
        .unwrap()
    }

    #[test]
    fn matches_rfc8032_test_vector_1_public_key() {
        // RFC 8032 section 7.1, TEST 1.
        let seed: [u8; 32] =
            from_hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60").unwrap();
        assert_eq!(
            Signer::from_seed(seed).public_key().hex(),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );
    }

    #[test]
    fn sign_then_verify() {
        let signed = Signer::from_seed(seed(7)).sign(lease());
        signed.verify().unwrap();
        assert_eq!(signed.signer, Signer::from_seed(seed(7)).public_key());
    }

    #[test]
    fn signatures_are_deterministic() {
        let a = Signer::from_seed(seed(7)).sign(lease());
        let b = Signer::from_seed(seed(7)).sign(lease());
        assert_eq!(a.signature, b.signature);
        assert_ne!(
            a.signature,
            Signer::from_seed(seed(8)).sign(lease()).signature
        );
    }

    #[test]
    fn changing_the_object_breaks_the_signature() {
        let mut signed = Signer::from_seed(seed(7)).sign(lease());
        signed.object.expires_at = LogicalTime::new(11);
        assert_eq!(signed.verify(), Err(SignError::BadSignature));
    }

    #[test]
    fn a_different_signer_key_breaks_the_signature() {
        let mut signed = Signer::from_seed(seed(7)).sign(lease());
        signed.signer = Signer::from_seed(seed(8)).public_key();
        assert_eq!(signed.verify(), Err(SignError::BadSignature));
    }

    #[test]
    fn signature_does_not_transfer_between_identity_classes() {
        // Same 32 identity bytes, different class: the message differs.
        let lease = lease();
        let signer = Signer::from_seed(seed(7));
        let as_lease = signer.sign(lease.clone());
        let epoch = Epoch::initial(AuthorityDomain::new("fabric").unwrap());
        let mut forged = signer.sign(epoch);
        forged.signature = as_lease.signature;
        assert_eq!(forged.verify(), Err(SignError::BadSignature));
    }

    #[test]
    fn canonical_round_trip_verifies_on_decode() {
        let signed = Signer::from_seed(seed(7)).sign(lease());
        let bytes = signed.canonical_bytes();
        assert_eq!(
            Signed::<Lease>::from_canonical_bytes(&bytes).unwrap(),
            signed
        );
    }

    #[test]
    fn decoding_rejects_a_forged_envelope() {
        let mut signed = Signer::from_seed(seed(7)).sign(lease());
        signed.object.expires_at = LogicalTime::new(99);
        let bytes = signed.canonical_bytes();
        assert!(matches!(
            Signed::<Lease>::from_canonical_bytes(&bytes),
            Err(CanonicalError::InvalidContent(_))
        ));
    }

    #[test]
    fn json_view_uses_hex() {
        let signed = Signer::from_seed(seed(7)).sign(lease());
        let json = serde_json::to_value(&signed).unwrap();
        assert_eq!(json["signer"], signed.signer.hex());
        let back: Signed<Lease> = serde_json::from_value(json).unwrap();
        assert_eq!(back, signed);
    }

    #[test]
    fn debug_never_prints_the_secret() {
        let text = format!("{:?}", Signer::from_seed(seed(7)));
        assert!(text.starts_with("Signer(") && !text.contains("0707070707"));
    }
}
