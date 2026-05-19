#![forbid(unsafe_code)]

//! Per-halt output-marker registry. Story 4.2's predicate-firing path
//! consumes markers via `consume_for_halt(halt_id) -> Vec<OutputMarker>`.

use std::collections::VecDeque;
use std::sync::Mutex;
use dashmap::DashMap;
use maos_domain::halt::{HaltId, OutputMarker};

/// Per-halt output-marker registry. Story 4.2's predicate-firing path
/// consumes markers via `consume_for_halt(halt_id) -> Vec<OutputMarker>`.
#[maos_attrs::i9_exempt(reason = "halt mechanism — per-process override markers awaiting output_shape consumption; transient kernel state parallel to OrchestratorBuffer")]
#[derive(Debug, Default)]
pub struct OutputMarkerRegistry {
    by_halt: DashMap<HaltId, Mutex<VecDeque<OutputMarker>>>,
}

impl OutputMarkerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_for_halt(&self, halt_id: &HaltId, marker: OutputMarker) {
        let queue = self
            .by_halt
            .entry(halt_id.clone())
            .or_insert_with(|| Mutex::new(VecDeque::new()));
        queue.lock().expect("OutputMarkerRegistry lock poisoned").push_back(marker);
    }

    pub fn consume_for_halt(&self, halt_id: &HaltId) -> Vec<OutputMarker> {
        if let Some(queue) = self.by_halt.get(halt_id) {
            let mut q = queue.lock().expect("OutputMarkerRegistry lock poisoned");
            q.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    pub fn pending_count(&self, halt_id: &HaltId) -> usize {
        self.by_halt
            .get(halt_id)
            .map(|q| q.lock().expect("OutputMarkerRegistry lock poisoned").len())
            .unwrap_or(0)
    }
}
