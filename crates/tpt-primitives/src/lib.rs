//! Foundational shared primitives for the Fabric / Repro / Concord ecosystem.
//!
//! This crate implements the canonical object framework and the shared
//! semantic primitives described in the cross-project design specification
//! ([`spec.txt`](https://tpt.example/tpt-primitives)):
//!
//! | Module | Spec | Contents |
//! |--------|------|----------|
//! | [`canonical`] | §4 | canonical object contract (representation, schema version, encoding, identity) |
//! | [`id`] | §3 | generic `Id<T>` newtype and identity classes |
//! | [`ids`] | §3 | the fifteen shared identity classes |
//! | [`capability`] | §5–6 | capability, lease, epoch, composed valid-authority check |
//! | [`intent`] | §7 | intent primitive |
//! | [`reservation`] | §8 | reservation primitive and its state machine |
//! | [`computation`] | §9–10 | computation and derivation primitives |
//! | [`execution`] | §11–12 | execution and trace primitives, divergence detection |
//! | [`evidence`] | §13–15, §18, §24 | evidence, claim (assurance graph), counterexample |
//! | [`world`] | §16–17 | execution world, relevance declaration, reproduction |
//!
//! Fabric, Repro and Concord implementations deliberately do **not** live
//! here; this crate is primitives only (spec §21, *primitives before
//! adapters*).
//!
//! Identity is `SHA-256` over a schema-versioned deterministic CBOR
//! envelope (see `docs/decisions/0001-canonical-encoding-and-identity.md`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod canonical;
pub mod encoding;
pub mod error;
pub mod hash;
pub mod id;
pub mod ids;
pub mod schema;
pub mod time;
pub mod value;

pub use canonical::{Canonical, Identified};
pub use error::CanonicalError;
pub use id::Id;
pub use time::LogicalTime;
