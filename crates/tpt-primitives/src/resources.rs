//! Canonical resource descriptions shared by intents (spec §7),
//! reservations (spec §8), computations (spec §9) and execution worlds
//! (spec §16).

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A kind of resource. Known kinds get canonical names; ecosystems may
/// add `Custom` kinds with their own labels.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// CPU cores (integer count).
    CpuCores,
    /// Memory in MiB.
    MemoryMiB,
    /// Disk in MiB.
    DiskMiB,
    /// Network throughput in Mbps.
    NetworkMbps,
    /// GPU devices (integer count).
    GpuDevices,
    /// An ecosystem-defined kind; the label carries the unit convention.
    Custom {
        /// Ecosystem-defined label.
        label: String,
    },
}

impl fmt::Display for ResourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CpuCores => f.write_str("cpu_cores"),
            Self::MemoryMiB => f.write_str("memory_mib"),
            Self::DiskMiB => f.write_str("disk_mib"),
            Self::NetworkMbps => f.write_str("network_mbps"),
            Self::GpuDevices => f.write_str("gpu_devices"),
            Self::Custom { label } => f.write_str(label),
        }
    }
}

/// A deterministic set of resource amounts. `BTreeMap` keeps canonical
/// order sorted regardless of construction order (ADR 0001).
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
pub struct ResourceSet {
    /// Amount per resource kind.
    pub amounts: BTreeMap<ResourceKind, u64>,
}

impl ResourceSet {
    /// An empty resource set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the amount for a kind (builder style).
    pub fn with(mut self, kind: ResourceKind, amount: u64) -> Self {
        self.amounts.insert(kind, amount);
        self
    }

    /// The amount for a kind, if any.
    pub fn get(&self, kind: &ResourceKind) -> Option<u64> {
        self.amounts.get(kind).copied()
    }

    /// True if every requirement of `self` is covered by `available`.
    pub fn is_covered_by(&self, available: &ResourceSet) -> bool {
        self.amounts
            .iter()
            .all(|(kind, need)| available.get(kind).is_some_and(|have| have >= *need))
    }

    /// The union of two sets, taking the maximum per kind.
    pub fn merge_max(&self, other: &ResourceSet) -> ResourceSet {
        let mut out = self.clone();
        for (kind, amount) in &other.amounts {
            out.amounts
                .entry(kind.clone())
                .and_modify(|e| *e = (*e).max(*amount))
                .or_insert(*amount);
        }
        out
    }

    /// True if nothing is requested.
    pub fn is_empty(&self) -> bool {
        self.amounts.is_empty()
    }
}
