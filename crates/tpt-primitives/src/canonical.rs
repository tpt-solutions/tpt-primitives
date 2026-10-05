//! The canonical object contract (spec §4).
//!
//! An object that participates in identity must have:
//!
//! - a canonical representation ([`Canonical`] serialization)
//! - an explicit schema version ([`Canonical::SCHEMA_VERSION`])
//! - a deterministic encoding ([`Canonical::canonical_bytes`], ADR 0001)
//! - a deterministic identity derivation ([`Identified::identity`])
//! - explicit evolution rules (`docs/evolution.md`)
//!
//! Changing irrelevant presentation must not change identity; changing
//! semantics must.

use serde::{Serialize, de::DeserializeOwned};

use crate::encoding;
use crate::error::CanonicalError;
use crate::id::Id;

/// A type with a canonical representation and an explicit schema version.
///
/// Canonical bytes are the schema-versioned CBOR envelope produced by
/// [`crate::encoding`]; they are the identity preimage for [`Identified`].
pub trait Canonical: Serialize + DeserializeOwned + Sized {
    /// Schema version pinned by this Rust type. Bump rules are defined in
    /// `docs/evolution.md`.
    const SCHEMA_VERSION: u16;

    /// Deterministic canonical bytes including the schema-version envelope.
    ///
    /// Serialization of in-memory canonical types is infallible, so the
    /// result is unwrapped.
    fn canonical_bytes(&self) -> Vec<u8> {
        encoding::encode_envelope(Self::SCHEMA_VERSION, self)
    }

    /// Decode from canonical bytes, strictly checking the schema version.
    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, CanonicalError> {
        let (version, payload) = encoding::decode_envelope::<Self>(bytes)?;
        if version != Self::SCHEMA_VERSION {
            return Err(CanonicalError::SchemaVersionMismatch {
                expected: Self::SCHEMA_VERSION,
                found: version,
            });
        }
        Ok(payload)
    }
}

/// A canonical type from which a stable [`Id`] is derived.
///
/// The default derivation hashes the full canonical bytes with class domain
/// separation (ADR 0001). A type whose identity must exclude mutable state
/// (e.g. a capability's revocation status) models the identity-bearing
/// content as a separate inner type and implements [`Identified`] there.
pub trait Identified: Canonical {
    /// The identity class marker of the derived ID.
    type Id: crate::id::IdClass;

    /// Derive the stable identity of this object.
    fn identity(&self) -> Id<Self::Id> {
        Id::derive(&self.canonical_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Demo {
        name: String,
        value: u64,
    }

    struct DemoTag;
    impl crate::id::IdClass for DemoTag {
        const DOMAIN: &'static str = "tpt.demo.v1";
    }

    impl Canonical for Demo {
        const SCHEMA_VERSION: u16 = 1;
    }

    impl Identified for Demo {
        type Id = DemoTag;
    }

    #[test]
    fn round_trip_preserves_identity() {
        let demo = Demo {
            name: "demo".into(),
            value: 42,
        };
        let id = demo.identity();
        let bytes = demo.canonical_bytes();
        let restored = Demo::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(restored, demo);
        assert_eq!(restored.identity(), id);
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let demo = Demo {
            name: "demo".into(),
            value: 42,
        };
        let bytes = crate::encoding::encode_envelope(999, &demo);
        let err = Demo::from_canonical_bytes(&bytes).unwrap_err();
        assert_eq!(
            err,
            CanonicalError::SchemaVersionMismatch {
                expected: 1,
                found: 999
            }
        );
    }

    #[test]
    fn semantic_change_changes_identity() {
        let a = Demo {
            name: "demo".into(),
            value: 42,
        };
        let b = Demo {
            name: "demo".into(),
            value: 43,
        };
        assert_ne!(a.identity(), b.identity());
    }
}
