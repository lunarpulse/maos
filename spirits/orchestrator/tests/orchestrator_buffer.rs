//! AC2 — FR20: the Orchestrator drains the director's buffered instructions from
//! the **real** [`OrchestratorBuffer`] (capacity-32 FIFO, `QueueFull`
//! backpressure) at safe sequence points — and NEVER preempts an in-flight
//! delegation. Proven end-to-end against the real kernel buffer as a dev-dep:
//! the Spirit owns the safe-point gate, the kernel owns the FIFO.

use maos_domain::orchestrator::{OrchestratorInstruction, OrchestratorInstructionId};
use maos_kernel_core::orchestrator::{OrchestratorBuffer, OrchestratorBufferError};
use orchestrator::Orchestrator;

fn instr(id: u64, goal: &str) -> OrchestratorInstruction {
    OrchestratorInstruction::new(OrchestratorInstructionId(id), goal, id).expect("non-empty goal")
}

#[test]
fn instruction_enqueued_while_in_flight_is_not_processed_until_completion() {
    let buffer = OrchestratorBuffer::new();
    let orch = Orchestrator::new("orchestrator");

    // Director buffers the first instruction; Orchestrator drains it at a safe
    // point and begins the delegation.
    buffer
        .enqueue(instr(1, "design the overnight task"))
        .unwrap();
    let first = orch
        .drain_next(|| buffer.dequeue_at_safe_point())
        .expect("safe point ⇒ first instruction drained");
    assert_eq!(first.id, OrchestratorInstructionId(1));
    orch.begin_delegation();

    // While the delegation is IN FLIGHT, the director buffers a second
    // instruction. FR20: it must NOT be processed (the gate refuses to drain).
    buffer
        .enqueue(instr(2, "review while first is in flight"))
        .unwrap();
    let preempt = orch.drain_next(|| buffer.dequeue_at_safe_point());
    assert!(
        preempt.is_none(),
        "FR20: never preempt an in-flight delegation"
    );
    // The instruction stays buffered.
    assert_eq!(
        buffer.pending_count(),
        1,
        "second instruction still buffered"
    );

    // The delegation completes — re-opening the safe point.
    orch.complete_delegation();
    assert!(orch.is_safe_point());
    let second = orch
        .drain_next(|| buffer.dequeue_at_safe_point())
        .expect("completion re-opens the safe point");
    assert_eq!(second.id, OrchestratorInstructionId(2));
    assert_eq!(buffer.pending_count(), 0);
}

#[test]
fn buffer_drains_in_fifo_order_across_completions() {
    let buffer = OrchestratorBuffer::new();
    let orch = Orchestrator::new("orchestrator");

    for i in 1..=3 {
        buffer.enqueue(instr(i, &format!("task {i}"))).unwrap();
    }

    let mut drained = Vec::new();
    while orch.is_safe_point() {
        match orch.drain_next(|| buffer.dequeue_at_safe_point()) {
            Some(i) => {
                drained.push(i.id.0);
                // Simulate a full delegation round-trip per instruction.
                orch.begin_delegation();
                orch.complete_delegation();
            }
            None => break,
        }
    }
    assert_eq!(drained, vec![1, 2, 3], "FIFO order preserved across rounds");
}

#[test]
fn buffer_backpressures_at_capacity_32() {
    let buffer = OrchestratorBuffer::new();
    assert_eq!(buffer.capacity(), 32, "v0.3-β capacity floor");

    for i in 0..32 {
        buffer
            .enqueue(instr(i, &format!("t{i}")))
            .expect("under cap");
    }
    let err = buffer
        .enqueue(instr(99, "over cap"))
        .expect_err("33rd enqueue must backpressure");
    assert!(
        matches!(err, OrchestratorBufferError::QueueFull(32)),
        "QueueFull(32) at capacity, got {err:?}"
    );
}
