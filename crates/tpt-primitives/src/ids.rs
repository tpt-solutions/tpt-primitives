//! The shared identity classes (spec §3).
//!
//! The fifteen classes from the cross-project specification are defined
//! here as [`Id<T>`] specializations. A few ecosystem-internal classes
//! (lease, derivation, world, trace event) live alongside them because the
//! semantic primitives in this crate need stable identities for those
//! objects too; they follow the same rules.
//!
//! Identity is always derived from canonical semantics with class domain
//! separation — never from filenames, auto-increment IDs, memory
//! addresses, mutable labels or wall-clock timestamps (spec §3, ADR 0001).

use crate::id::{AnyId, Id, IdClass};

macro_rules! identity_class {
    ($(#[$meta:meta])* $marker:ident, $alias:ident, $domain:literal;) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $marker;

        impl IdClass for $marker {
            const DOMAIN: &'static str = $domain;
        }

        $(#[$meta])*
        #[doc = concat!("Identity class `", $domain, "`.")]
        pub type $alias = Id<$marker>;
    };
}

identity_class!(
    /// Marker for `SpecificationID` (spec §3): a Concord-owned specification.
    SpecificationTag, SpecificationId, "tpt.specification.v1";
);
identity_class!(
    /// Marker for `ModelID` (spec §3): a Concord-owned model (e.g. a
    /// formal model used by a model checker).
    ModelTag, ModelId, "tpt.model.v1";
);
identity_class!(
    /// Marker for `ImplementationID` (spec §3): a concrete implementation
    /// of a specification.
    ImplementationTag, ImplementationId, "tpt.implementation.v1";
);
identity_class!(
    /// Marker for `ComputationID` (spec §3, §9): a Repro-owned canonical
    /// computation description.
    ComputationTag, ComputationId, "tpt.computation.v1";
);
identity_class!(
    /// Marker for `IntentID` (spec §3, §7): a Fabric-owned intent.
    IntentTag, IntentId, "tpt.intent.v1";
);
identity_class!(
    /// Marker for `ReservationID` (spec §3, §8): a Fabric-owned reservation.
    ReservationTag, ReservationId, "tpt.reservation.v1";
);
identity_class!(
    /// Marker for `ExecutionID` (spec §3, §11): one realisation of a
    /// computation.
    ExecutionTag, ExecutionId, "tpt.execution.v1";
);
identity_class!(
    /// Marker for `ArtifactID` (spec §3): a content-addressed artifact.
    ArtifactTag, ArtifactId, "tpt.artifact.v1";
);
identity_class!(
    /// Marker for `TraceID` (spec §3, §12): a canonical execution trace.
    TraceTag, TraceId, "tpt.trace.v1";
);
identity_class!(
    /// Marker for `EvidenceID` (spec §3, §13): a Concord-owned evidence
    /// object.
    EvidenceTag, EvidenceId, "tpt.evidence.v1";
);
identity_class!(
    /// Marker for `ClaimID` (spec §3, §14): a node in the Assurance Graph.
    ClaimTag, ClaimId, "tpt.claim.v1";
);
identity_class!(
    /// Marker for `CapabilityID` (spec §3, §5): an authority grant.
    CapabilityTag, CapabilityId, "tpt.capability.v1";
);
identity_class!(
    /// Marker for `EpochID` (spec §3, §6): an epoch of a domain.
    EpochTag, EpochId, "tpt.epoch.v1";
);
identity_class!(
    /// Marker for `CounterexampleID` (spec §3, §15): a uniform failure
    /// record.
    CounterexampleTag, CounterexampleId, "tpt.counterexample.v1";
);
identity_class!(
    /// Marker for `ProvenanceID` (spec §3): a record of how an artifact
    /// was produced.
    ProvenanceTag, ProvenanceId, "tpt.provenance.v1";
);

// Ecosystem-internal classes needed by the primitives themselves.

identity_class!(
    /// Marker for lease identities (spec §6). Leases are identified
    /// alongside the shared classes; they were not among the initial
    /// fifteen because leases are Fabric-internal authority state.
    LeaseTag, LeaseId, "tpt.lease.v1";
);
identity_class!(
    /// Marker for derivation identities (spec §10): the relationship
    /// `input artifacts + ComputationID → DerivationID → output artifact`.
    DerivationTag, DerivationId, "tpt.derivation.v1";
);
identity_class!(
    /// placeholder-noop
    WorldTag, WorldId, "tpt.world.v1";
);
identity_class!(
    /// Marker for trace-event identities (spec §12): a single event
    /// inside a trace, content-addressed for stable event identity.
    TraceEventTag, TraceEventId, "tpt.trace-event.v1";
);

/// Every identity class in this crate, for schema generation and tooling.
pub const CLASSES: &[(&str, &str)] = &[
    (SpecificationTag::DOMAIN, "SpecificationId"),
    (ModelTag::DOMAIN, "ModelId"),
    (ImplementationTag::DOMAIN, "ImplementationId"),
    (ComputationTag::DOMAIN, "ComputationId"),
    (IntentTag::DOMAIN, "IntentId"),
    (ReservationTag::DOMAIN, "ReservationId"),
    (ExecutionTag::DOMAIN, "ExecutionId"),
    (ArtifactTag::DOMAIN, "ArtifactId"),
    (TraceTag::DOMAIN, "TraceId"),
    (EvidenceTag::DOMAIN, "EvidenceId"),
    (ClaimTag::DOMAIN, "ClaimId"),
    (CapabilityTag::DOMAIN, "CapabilityId"),
    (EpochTag::DOMAIN, "EpochId"),
    (CounterexampleTag::DOMAIN, "CounterexampleId"),
    (ProvenanceTag::DOMAIN, "ProvenanceId"),
    (LeaseTag::DOMAIN, "LeaseId"),
    (DerivationTag::DOMAIN, "DerivationId"),
    (WorldTag::DOMAIN, "WorldId"),
    (TraceEventTag::DOMAIN, "TraceEventId"),
];

/// Convenience constructor: a class-erased view of any identity.
pub fn any<T: IdClass>(id: &Id<T>) -> AnyId {
    AnyId::from_id(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_fifteen_classes_are_distinct_domains() {
        let domains: std::collections::BTreeSet<&str> = CLASSES.iter().map(|(d, _)| *d).collect();
        assert_eq!(domains.len(), CLASSES.len(), "domain labels must be unique");
        // The fifteen shared classes from spec §3.
        for domain in [
            "tpt.specification.v1",
            "tpt.model.v1",
            "tpt.implementation.v1",
            "tpt.computation.v1",
            "tpt.intent.v1",
            "tpt.reservation.v1",
            "tpt.execution.v1",
            "tpt.artifact.v1",
            "tpt.trace.v1",
            "tpt.evidence.v1",
            "tpt.claim.v1",
            "tpt.capability.v1",
            "tpt.epoch.v1",
            "tpt.counterexample.v1",
            "tpt.provenance.v1",
        ] {
            assert!(domains.contains(domain), "missing class {domain}");
        }
    }

    #[test]
    fn same_content_different_classes_different_identity() {
        let spec = SpecificationId::derive(b"shared");
        let model = ModelId::derive(b"shared");
        let artifact = ArtifactId::derive(b"shared");
        // Distinct classes are distinct types; compare raw bytes.
        let bytes = [spec.as_bytes(), model.as_bytes(), artifact.as_bytes()];
        assert_ne!(bytes[0], bytes[1]);
        assert_ne!(bytes[0], bytes[2]);
        assert_ne!(bytes[1], bytes[2]);
    }

    #[test]
    fn ids_serialize_as_hex_text() {
        let id = ComputationId::derive(b"abc");
        let bytes = crate::encoding::encode_envelope(1, &id);
        let (v, back): (u16, ComputationId) = crate::encoding::decode_envelope(&bytes).unwrap();
        assert_eq!(v, 1);
        assert_eq!(back, id);
        assert_eq!(back.hex().len(), 64);
        assert!(
            back.hex()
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn any_id_erasure_round_trip() {
        let id = EvidenceId::derive(b"e1");
        let any = AnyId::from_id(&id);
        assert_eq!(any.class, "tpt.evidence.v1");
        assert_eq!(any.id, id.hex());
        assert_eq!(any, id);
        // A mismatched class label does not compare equal.
        let wrong = AnyId {
            class: "tpt.claim.v1".into(),
            id: id.hex(),
        };
        assert_ne!(wrong, id);
    }
}
