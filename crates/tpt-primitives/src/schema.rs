//! Schema description registry for the canonical types.
//!
//! `schema-gen` (in `crates/schema-gen`) walks this registry to emit the
//! checked-in JSON Schema and CDDL definitions under `schemas/`. Hand-
//! written CDDL lives beside each type's JSON Schema entry so both views
//! evolve together.

/// A registry entry: type name, CDDL definition, and a JSON Schema factory.
#[derive(Debug)]
pub struct SchemaEntry {
    /// Type name, e.g. `"Computation"`.
    pub name: &'static str,
    /// Hand-written CDDL definition for the type.
    pub cddl: &'static str,
    /// Produce the JSON Schema for the type.
    pub json_schema: fn() -> schemars::Schema,
}

/// All schema entries contributed by this crate.
pub fn entries() -> Vec<SchemaEntry> {
    Vec::new() // populated by Phase 11 (schema export)
}
