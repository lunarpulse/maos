#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::sync::Mutex;
use maos_domain::orchestrator::OrchestratorInstruction;

/// Bounded per-Spirit instruction buffer. The Orchestrator-class Spirit
/// owns the dequeue cadence; the kernel only enforces FIFO + capacity.
///
/// Capacity floor: 32 (matches `consent.request` mpsc capacity from
/// architecture §7.1.1's per-frame-kind channel-class table — same
/// "director-action, low-volume" tier).
///
/// Backpressure: `enqueue` returns `OrchestratorBufferError::QueueFull`
/// when at capacity; CLI surfaces this to the director rather than dropping.
#[maos_attrs::i9_exempt(reason = "orchestrator instruction buffer — transient per-process VecDeque for FR20 checkpoint/resume primitive; parallel to Mailbox routing state")]
#[derive(Debug)]
pub struct OrchestratorBuffer {
    queue: Mutex<VecDeque<OrchestratorInstruction>>,
    capacity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrchestratorBufferError {
    #[error("orchestrator buffer at capacity ({0}); director must wait for Orchestrator to drain")]
    QueueFull(usize),
}

impl OrchestratorBuffer {
    /// Construct with the v0.3-β capacity floor (32).
    pub fn new() -> Self {
        Self::with_capacity(32)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::with_capacity(capacity)),
            capacity,
        }
    }

    /// Enqueue an instruction. Returns `Err(QueueFull)` at capacity.
    /// FIFO ordering — `dequeue_at_safe_point` returns in insertion order.
    pub fn enqueue(
        &self,
        instruction: OrchestratorInstruction,
    ) -> Result<(), OrchestratorBufferError> {
        let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        if q.len() >= self.capacity {
            return Err(OrchestratorBufferError::QueueFull(self.capacity));
        }
        q.push_back(instruction);
        Ok(())
    }

    /// Dequeue the next instruction. Called by the Orchestrator-class
    /// Spirit between task completions — NEVER from kernel-internal hooks
    /// (that would violate FR20's "never preempt in-flight delegations").
    /// On a poisoned mutex, returns `None` (graceful degradation).
    pub fn dequeue_at_safe_point(&self) -> Option<OrchestratorInstruction> {
        match self.queue.lock() {
            Ok(mut q) => q.pop_front(),
            Err(_) => None,
        }
    }

    /// Drain ALL pending instructions in FIFO order. Used by the resume
    /// path (FR51 c) when the director resumes after a pause: the
    /// Orchestrator inherits the full buffered queue without losing order.
    pub fn recall_all_pending(&self) -> Vec<OrchestratorInstruction> {
        match self.queue.lock() {
            Ok(mut q) => {
                let drained: Vec<_> = q.drain(..).collect();
                drained
            }
            Err(_) => vec![],
        }
    }

    /// Current pending count — surfaced by `maosctl orchestrator status`
    /// (read-only inspection; not normative for the queue's correctness).
    pub fn pending_count(&self) -> usize {
        match self.queue.lock() {
            Ok(q) => q.len(),
            Err(_) => 0,
        }
    }

    /// Construction-time capacity (immutable per buffer instance).
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for OrchestratorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::orchestrator::{OrchestratorInstruction, OrchestratorInstructionId};

    fn make_instruction(id: u64, goal: impl Into<String>) -> OrchestratorInstruction {
        OrchestratorInstruction::new(
            OrchestratorInstructionId(id),
            goal,
            0,
        )
        .unwrap()
    }

    #[test]
    fn new_returns_buffer_with_capacity_32() {
        let buf = OrchestratorBuffer::new();
        assert_eq!(buf.capacity(), 32);
        assert_eq!(buf.pending_count(), 0);
    }

    #[test]
    fn enqueue_then_dequeue_returns_same_instruction_fifo() {
        let buf = OrchestratorBuffer::new();
        let instr = make_instruction(1, "test goal");
        buf.enqueue(instr.clone()).unwrap();

        let dequeued = buf.dequeue_at_safe_point().unwrap();
        assert_eq!(dequeued.id, instr.id);
        assert_eq!(dequeued.goal, "test goal");
    }

    #[test]
    fn enqueue_32_succeeds_33rd_returns_queue_full() {
        let buf = OrchestratorBuffer::new();
        for i in 0..32 {
            buf.enqueue(make_instruction(i as u64, format!("goal {}", i))).unwrap();
        }
        assert_eq!(buf.pending_count(), 32);
        let err = buf.enqueue(make_instruction(99, "overflow")).unwrap_err();
        assert!(matches!(err, OrchestratorBufferError::QueueFull(32)));
    }

    #[test]
    fn recall_all_pending_returns_fifo_order() {
        let buf = OrchestratorBuffer::new();
        buf.enqueue(make_instruction(1, "first")).unwrap();
        buf.enqueue(make_instruction(2, "second")).unwrap();
        buf.enqueue(make_instruction(3, "third")).unwrap();
        buf.enqueue(make_instruction(4, "fourth")).unwrap();
        buf.enqueue(make_instruction(5, "fifth")).unwrap();

        let all = buf.recall_all_pending();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].goal, "first");
        assert_eq!(all[1].goal, "second");
        assert_eq!(all[2].goal, "third");
        assert_eq!(all[3].goal, "fourth");
        assert_eq!(all[4].goal, "fifth");
    }

    #[test]
    fn recall_all_pending_empties_the_queue() {
        let buf = OrchestratorBuffer::new();
        buf.enqueue(make_instruction(1, "a")).unwrap();
        buf.enqueue(make_instruction(2, "b")).unwrap();

        let all = buf.recall_all_pending();
        assert_eq!(all.len(), 2);
        assert_eq!(buf.pending_count(), 0);
    }

    #[test]
    fn buffer_is_send_and_sync() {
        fn _assert_send_sync<T: Send + Sync>(_: T) {}
        _assert_send_sync(OrchestratorBuffer::new());
    }

    #[test]
    fn dequeue_at_safe_point_on_empty_returns_none() {
        let buf = OrchestratorBuffer::new();
        assert!(buf.dequeue_at_safe_point().is_none());
    }

    #[test]
    fn pending_count_after_enqueue_dequeue_is_correct() {
        let buf = OrchestratorBuffer::new();
        assert_eq!(buf.pending_count(), 0);
        buf.enqueue(make_instruction(1, "a")).unwrap();
        assert_eq!(buf.pending_count(), 1);
        buf.enqueue(make_instruction(2, "b")).unwrap();
        assert_eq!(buf.pending_count(), 2);
        buf.dequeue_at_safe_point();
        assert_eq!(buf.pending_count(), 1);
        buf.dequeue_at_safe_point();
        assert_eq!(buf.pending_count(), 0);
    }

    #[test]
    fn recall_and_re_enqueue_preserves_instructions() {
        let buf = OrchestratorBuffer::new();
        buf.enqueue(make_instruction(1, "first")).unwrap();
        buf.enqueue(make_instruction(2, "second")).unwrap();
        buf.enqueue(make_instruction(3, "third")).unwrap();

        let pending = buf.recall_all_pending();
        assert_eq!(pending.len(), 3);
        assert_eq!(buf.pending_count(), 0);

        for instr in pending {
            buf.enqueue(instr).unwrap();
        }
        assert_eq!(buf.pending_count(), 3);

        let first = buf.dequeue_at_safe_point().unwrap();
        assert_eq!(first.goal, "first");
        let second = buf.dequeue_at_safe_point().unwrap();
        assert_eq!(second.goal, "second");
        let third = buf.dequeue_at_safe_point().unwrap();
        assert_eq!(third.goal, "third");
    }
}
