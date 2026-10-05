//! Logical time (ADR 0002).
//!
//! This crate never consults a wall clock. All temporal content is a
//! [`LogicalTime`]: a tick counter whose unit is fixed by the
//! [`ExecutionWorld`'s time model][crate::world::TimeModel]. Real
//! deployments map ticks to wall-clock time at their boundary; the
//! primitive itself stays deterministic and simulation-first (spec §22).

use serde::{Deserialize, Serialize};

/// A monotonically increasing tick count.
#[derive(
    Debug,
    Clone,
    Copy,
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
pub struct LogicalTime(u64);

impl LogicalTime {
    /// The origin (tick 0).
    pub const ZERO: LogicalTime = LogicalTime(0);

    /// Create from a raw tick count.
    pub const fn new(ticks: u64) -> Self {
        Self(ticks)
    }

    /// The raw tick count.
    pub const fn ticks(&self) -> u64 {
        self.0
    }

    /// `self + ticks`, saturating at `u64::MAX`.
    pub const fn plus(&self, ticks: u64) -> LogicalTime {
        LogicalTime(self.0.saturating_add(ticks))
    }

    /// `self <= other`.
    pub const fn is_at_or_before(&self, other: LogicalTime) -> bool {
        self.0 <= other.0
    }

    /// `self < other`.
    pub const fn is_before(&self, other: LogicalTime) -> bool {
        self.0 < other.0
    }
}

impl std::fmt::Display for LogicalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "t{}", self.0)
    }
}
