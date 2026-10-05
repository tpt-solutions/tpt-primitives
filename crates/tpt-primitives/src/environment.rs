//! Canonical environment, platform and toolchain descriptions.
//!
//! Shared by computations (spec §9), evidence (spec §13) and execution
//! worlds (spec §16). All content is deterministic: variables are a
//! `BTreeMap` of [`PrimitiveValue`]s (ADR 0001).

use crate::value::PrimitiveMap;
use serde::{Deserialize, Serialize};

/// Declared environment variables relevant to a computation or execution.
#[derive(
    Debug,
    Clone,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    schemars::JsonSchema,
)]
pub struct EnvironmentSpec {
    /// Environment variables by name.
    pub variables: PrimitiveMap,
}

impl EnvironmentSpec {
    /// An empty environment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable (builder style).
    pub fn with(mut self, name: impl Into<String>, value: crate::value::PrimitiveValue) -> Self {
        self.variables.insert(name.into(), value);
        self
    }
}

/// A target platform.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct PlatformSpec {
    /// Operating system label, e.g. `"linux"`.
    pub os: String,
    /// Architecture label, e.g. `"x86_64"`.
    pub arch: String,
}

impl PlatformSpec {
    /// Describe a platform, rejecting empty labels.
    pub fn new(os: impl Into<String>, arch: impl Into<String>) -> Self {
        Self {
            os: os.into(),
            arch: arch.into(),
        }
    }
}

/// A toolchain (compiler/interpreter/runtime) description.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct ToolchainSpec {
    /// Toolchain name, e.g. `"rust"`.
    pub name: String,
    /// Toolchain version, e.g. `"1.97.1"`.
    pub version: String,
}

impl ToolchainSpec {
    /// Describe a toolchain, rejecting empty labels.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}
