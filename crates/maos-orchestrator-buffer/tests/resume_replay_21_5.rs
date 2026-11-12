#![forbid(unsafe_code)]

//! Story 21-5 AC2.4 (D-21-5-07 B) — the leaf's resume replay, called through
//! the `OrchestratorResumePort` the kernel scheduler holds.
//!
//! * three queued instructions ⇒ replay ⇒ the same FIFO order, count 3;
//! * a Spirit that was never queued to ⇒ no buffer is created;
//! * a producer racing the replay of a full queue neither lands ahead of the
//!   queued instructions nor displaces them (the single guard).
//!
//! Proven red (story AC2.5): a replay that drops the last item reds the first
//! and third vectors.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use maos_domain::orchestrator::{
    OrchestratorInstruction, OrchestratorInstructionId, OrchestratorResumePort,
};
use maos_orchestrator_buffer::OrchestratorBufferRegistry;
use maos_spirit_abi::identity::SpiritId;

fn numbered(id: u64) -> OrchestratorInstruction {
    match OrchestratorInstruction::new(OrchestratorInstructionId(id), format!("goal-{id}"), id) {
        Ok(made) => made,
        Err(error) => panic!("instruction {id}: {error}"),
    }
}

fn ids(instructions: &[OrchestratorInstruction]) -> Vec<u64> {
    instructions.iter().map(|i| i.id.0).collect()
}

#[test]
fn three_queued_instructions_survive_a_replay_in_order() {
    let registry = OrchestratorBufferRegistry::new();
    let buffer = registry.get_or_create(&SpiritId::from("orch"));
    for id in 1..=3 {
        assert!(buffer.enqueue(numbered(id)).is_ok());
    }

    registry.replay_pending("orch");

    assert_eq!(buffer.pending_count(), 3, "the replay keeps the count");
    assert_eq!(ids(&buffer.recall_all_pending()), vec![1, 2, 3]);
}

#[test]
fn replaying_a_spirit_never_queued_to_creates_no_buffer() {
    let registry = OrchestratorBufferRegistry::new();

    registry.replay_pending("never-queued");

    assert!(registry.get(&SpiritId::from("never-queued")).is_none());
}

#[test]
fn a_producer_racing_a_replay_of_a_full_queue_changes_nothing() {
    const CYCLES: usize = 2_000;
    let registry = OrchestratorBufferRegistry::new();
    let buffer = registry.get_or_create(&SpiritId::from("orch"));
    let capacity = buffer.capacity() as u64;
    for id in 1..=capacity {
        assert!(buffer.enqueue(numbered(id)).is_ok());
    }

    let stop = Arc::new(AtomicBool::new(false));
    let producer = {
        let (buffer, stop) = (Arc::clone(&buffer), Arc::clone(&stop));
        std::thread::spawn(move || {
            let mut accepted = 0usize;
            while !stop.load(Ordering::Relaxed) {
                if buffer.enqueue(numbered(capacity + 1)).is_ok() {
                    accepted += 1;
                } else {
                    std::hint::spin_loop();
                }
            }
            accepted
        })
    };
    for _ in 0..CYCLES {
        registry.replay_pending("orch");
    }
    stop.store(true, Ordering::Relaxed);
    let accepted = match producer.join() {
        Ok(accepted) => accepted,
        Err(_) => panic!("the producer thread panicked"),
    };

    assert_eq!(accepted, 0, "a producer enqueue got in during a replay");
    assert_eq!(
        ids(&buffer.recall_all_pending()),
        (1..=capacity).collect::<Vec<_>>()
    );
}
