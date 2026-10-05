//! Execution and trace primitives (spec §11–12).
//!
//! An execution is one particular realisation of a computation and must
//! record enough information to distinguish *same computation* from
//! *same execution* (spec §11). A trace is a canonical representation of
//! relevant execution events with stable event identity, causal
//! relationships, deterministic normalization, replay support and
//! divergence detection (spec §12).
//!
//! Events are content-addressed, so comparing two traces is comparing two
//! sets of event identities — normalization is order-free and divergence
//! detection is exact.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Identified};
use crate::error::CanonicalError;
use crate::ids::{
    ArtifactId, ComputationId, ExecutionId, ExecutionTag, TraceEventId, TraceEventTag, TraceId,
    TraceTag, WorldId,
};
use crate::time::LogicalTime;
use crate::value::PrimitiveMap;

/// How an execution ended (spec §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum ExecutionOutcome {
    /// The execution produced artifacts.
    Succeeded {
        /// Content-addressed outputs.
        artifacts: BTreeSet<ArtifactId>,
    },
    /// The execution failed.
    Failed {
        /// Ecosystem-defined failure reason.
        reason: String,
    },
}

/// One realisation of a computation (spec §11).
///
/// `attempt` and `started_at` (plus the world) ensure that two runs of
/// the same computation are distinct executions with distinct identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Execution {
    /// The computation being realised.
    pub computation: ComputationId,
    /// The canonical execution world (spec §16) the computation ran in.
    pub world: WorldId,
    /// 1-based attempt number for this computation in this world.
    pub attempt: u32,
    /// Logical tick the execution started.
    pub started_at: LogicalTime,
    /// Logical tick the execution ended, once known.
    pub finished_at: Option<LogicalTime>,
    /// How the execution ended, once known.
    pub outcome: Option<ExecutionOutcome>,
    /// The trace of the execution, once recorded.
    pub trace: Option<TraceId>,
}

impl Canonical for Execution {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Execution {
    type Id = ExecutionTag;
}

impl Execution {
    /// Begin an execution: attempt 1 at the given logical tick.
    pub fn begin(computation: ComputationId, world: WorldId, at: LogicalTime) -> Self {
        Self::begin_attempt(computation, world, 1, at)
    }

    /// Begin a specific attempt of a computation.
    pub fn begin_attempt(
        computation: ComputationId,
        world: WorldId,
        attempt: u32,
        at: LogicalTime,
    ) -> Self {
        Self {
            computation,
            world,
            attempt,
            started_at: at,
            finished_at: None,
            outcome: None,
            trace: None,
        }
    }

    /// Record the end of the execution.
    pub fn finish(&mut self, at: LogicalTime, outcome: ExecutionOutcome) {
        self.finished_at = Some(at);
        self.outcome = Some(outcome);
    }

    /// Attach a trace reference.
    pub fn with_trace(mut self, trace: TraceId) -> Self {
        self.trace = Some(trace);
        self
    }
}

/// A single event inside a trace (spec §12). Its identity is derived
/// from its canonical content — payload plus causal parents — so event
/// identity is stable across normalization and replay.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, schemars::JsonSchema,
)]
pub struct TraceEvent {
    /// Event payload (ecosystem-defined, canonical).
    pub payload: PrimitiveMap,
    /// Events this one is causally downstream of.
    pub causal_parents: BTreeSet<TraceEventId>,
}

impl Canonical for TraceEvent {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for TraceEvent {
    type Id = TraceEventTag;
}

/// A trace: the canonical record of one execution's relevant events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Trace {
    /// The execution this trace belongs to.
    pub execution: ExecutionId,
    /// Events, stored in canonical (identity-sorted) order.
    pub events: Vec<TraceEvent>,
}

impl Canonical for Trace {
    const SCHEMA_VERSION: u16 = 1;
}

impl Identified for Trace {
    type Id = TraceTag;

    /// A trace's identity is derived from its *normalized* form, so the
    /// order events were recorded in cannot change identity
    /// (spec §12: deterministic normalization).
    fn identity(&self) -> crate::id::Id<TraceTag> {
        crate::id::Id::derive(&self.normalized().canonical_bytes())
    }
}

/// The result of comparing a trace against a replay (spec §12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceComparison {
    /// The replay produced exactly the same events.
    Identical,
    /// The replay diverged; the event identities list which events were
    /// expected but missing and which unexpected events appeared.
    Diverged {
        /// Events present in the original but missing from the replay.
        missing: BTreeSet<TraceEventId>,
        /// Events present in the replay but not in the original.
        unexpected: BTreeSet<TraceEventId>,
    },
}

impl Trace {
    /// Start a trace for an execution.
    pub fn for_execution(execution: crate::ids::ExecutionId) -> Self {
        Self {
            execution,
            events: Vec::new(),
        }
    }

    /// Append an event.
    pub fn push(&mut self, event: TraceEvent) {
        self.events.push(event);
    }

    /// Deterministic normalization (spec §12): events are ordered by
    /// their stable identity, independent of the order they were
    /// recorded in.
    pub fn normalized(&self) -> Self {
        let mut events = self.events.clone();
        events.sort_by_key(TraceEvent::identity);
        events.dedup_by_key(|e| e.identity());
        Self {
            execution: self.execution,
            events,
        }
    }

    /// Every event identity in canonical order.
    pub fn event_ids(&self) -> Vec<TraceEventId> {
        self.normalized()
            .events
            .iter()
            .map(TraceEvent::identity)
            .collect()
    }

    /// Compare this trace against a replay of the same execution.
    /// Divergence detection is exact: two traces match iff their event
    /// identity sets are equal.
    pub fn compare_replay(&self, replay: &Trace) -> TraceComparison {
        let a: BTreeSet<TraceEventId> = self.event_ids().into_iter().collect();
        let b: BTreeSet<TraceEventId> = replay.event_ids().into_iter().collect();
        if a == b {
            return TraceComparison::Identical;
        }
        TraceComparison::Diverged {
            missing: a.difference(&b).copied().collect(),
            unexpected: b.difference(&a).copied().collect(),
        }
    }

    /// Verify that every causal parent reference resolves to an event in
    /// this trace.
    pub fn causal_parents_resolved(&self) -> Result<(), CanonicalError> {
        let ids: BTreeSet<TraceEventId> = self.events.iter().map(TraceEvent::identity).collect();
        for event in &self.events {
            for parent in &event.causal_parents {
                if !ids.contains(parent) {
                    return Err(CanonicalError::InvalidContent(format!(
                        "trace of execution {} references event {parent} which is not present",
                        self.execution
                    )));
                }
            }
        }
        Ok(())
    }

    /// Replay support (spec §12): the events that must be fed to a
    /// re-execution, in causal order (normalized order is causal-total-
    /// order-agnostic; consumers order by their own dependency walk).
    pub fn replay_worklist(&self) -> Vec<TraceEventId> {
        self.event_ids()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;
    use crate::value::PrimitiveValue;

    fn computation() -> ComputationId {
        Id::derive(b"computation")
    }

    fn world() -> WorldId {
        Id::derive(b"world")
    }

    fn event(action: &str, parents: BTreeSet<TraceEventId>) -> TraceEvent {
        let mut payload = PrimitiveMap::new();
        payload.insert("action".into(), PrimitiveValue::Text(action.into()));
        TraceEvent {
            payload,
            causal_parents: parents,
        }
    }

    #[test]
    fn same_computation_is_not_same_execution() {
        let e1 = Execution::begin(computation(), world(), LogicalTime::new(10));
        let e2 = Execution::begin(computation(), world(), LogicalTime::new(20));
        let e3 = Execution::begin_attempt(computation(), world(), 2, LogicalTime::new(10));
        assert_ne!(
            e1.identity(),
            e2.identity(),
            "different start ticks are different executions"
        );
        assert_ne!(
            e1.identity(),
            e3.identity(),
            "different attempts are different executions"
        );
        let e4 = Execution::begin(computation(), world(), LogicalTime::new(10));
        assert_eq!(
            e1.identity(),
            e4.identity(),
            "same computation, world, attempt, tick = same execution"
        );
    }

    #[test]
    fn execution_records_distinction_explicitly() {
        let mut e = Execution::begin(computation(), world(), LogicalTime::new(1));
        e.finish(
            LogicalTime::new(5),
            ExecutionOutcome::Succeeded {
                artifacts: BTreeSet::from([crate::computation::artifact_of_content(b"out")]),
            },
        );
        assert!(matches!(
            e.outcome,
            Some(ExecutionOutcome::Succeeded { .. })
        ));
        assert_eq!(e.computation, computation());
    }

    #[test]
    fn event_identity_is_content_based() {
        let a = event("open", BTreeSet::new());
        let b = event("open", BTreeSet::new());
        let c = event("close", BTreeSet::new());
        assert_eq!(a.identity(), b.identity());
        assert_ne!(a.identity(), c.identity());
        let d = event("open", BTreeSet::from([a.identity()]));
        assert_ne!(
            a.identity(),
            d.identity(),
            "causal parents are part of event identity"
        );
    }

    #[test]
    fn normalization_is_order_free() {
        let a = event("a", BTreeSet::new());
        let b = event("b", BTreeSet::from([a.identity()]));
        let c = event("c", BTreeSet::from([b.identity()]));

        let mut t1 = Trace::for_execution(Id::derive(b"exec"));
        t1.push(a.clone());
        t1.push(b.clone());
        t1.push(c.clone());

        let mut t2 = Trace::for_execution(Id::derive(b"exec"));
        t2.push(c.clone());
        t2.push(a.clone());
        t2.push(b.clone());

        assert_eq!(
            t1.normalized(),
            t2.normalized(),
            "recording order is irrelevant presentation"
        );
        assert_eq!(t1.identity(), t2.identity());
    }

    #[test]
    fn duplicate_events_collapse() {
        let a = event("a", BTreeSet::new());
        let mut t = Trace::for_execution(Id::derive(b"exec"));
        t.push(a.clone());
        t.push(a.clone());
        assert_eq!(t.normalized().events.len(), 1);
    }

    #[test]
    fn replay_divergence_detected() {
        let a = event("a", BTreeSet::new());
        let b = event("b", BTreeSet::from([a.identity()]));

        let mut original = Trace::for_execution(Id::derive(b"exec"));
        original.push(a.clone());
        original.push(b.clone());

        // Faithful replay.
        let mut faithful = Trace::for_execution(Id::derive(b"exec"));
        faithful.push(b.clone());
        faithful.push(a.clone());
        assert_eq!(
            original.compare_replay(&faithful),
            TraceComparison::Identical
        );

        // Divergent replay: event "b" changed.
        let b2 = event("b-prime", BTreeSet::from([a.identity()]));
        let b2_id = b2.identity();
        let mut divergent = Trace::for_execution(Id::derive(b"exec"));
        divergent.push(a.clone());
        divergent.push(b2);
        match original.compare_replay(&divergent) {
            TraceComparison::Diverged {
                missing,
                unexpected,
            } => {
                assert_eq!(missing, BTreeSet::from([b.identity()]));
                assert_eq!(unexpected, BTreeSet::from([b2_id]));
            }
            TraceComparison::Identical => panic!("expected divergence"),
        }
    }

    #[test]
    fn unresolved_causal_parents_rejected() {
        let a = event("a", BTreeSet::new());
        let orphan = event("child", BTreeSet::from([Id::derive(b"missing-event")]));
        let mut t = Trace::for_execution(Id::derive(b"exec"));
        t.push(a);
        t.push(orphan);
        assert!(t.causal_parents_resolved().is_err());

        let parent = event("p", BTreeSet::new());
        let child = event("c", BTreeSet::from([parent.identity()]));
        let mut t2 = Trace::for_execution(Id::derive(b"exec"));
        t2.push(parent);
        t2.push(child);
        assert!(t2.causal_parents_resolved().is_ok());
    }

    #[test]
    fn trace_round_trips() {
        let a = event("a", BTreeSet::new());
        let mut t = Trace::for_execution(Id::derive(b"exec"));
        t.push(a);
        let restored = Trace::from_canonical_bytes(&t.canonical_bytes()).unwrap();
        assert_eq!(restored, t);
        assert_eq!(restored.identity(), t.identity());
    }
}
