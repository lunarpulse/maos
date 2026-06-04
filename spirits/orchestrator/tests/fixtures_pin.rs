//! AC9 — the Orchestrator's wedge-scenario fixture is SHA-256-pinned (Story 0.3)
//! AND drives the real FR20 buffer to prove the safe-point FIFO drain order.
//! Mirrors `spirits/observer`'s fixtures_pin.

use maos_domain::orchestrator::{OrchestratorInstruction, OrchestratorInstructionId};
use maos_kernel_core::orchestrator::OrchestratorBuffer;
use orchestrator::Orchestrator;
use sha2::{Digest, Sha256};

const FIXTURE_FILES: [&str; 1] = ["wedge-scenario.json"];
const FIXTURES_PIN: &str = "ccc3aecd5f3e4ba97bad3a407d9758adc91221ef4b049e299897109f1af8cfad";

fn compute_pin() -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let mut hasher = Sha256::new();
    for f in FIXTURE_FILES {
        let path = format!("{dir}/{f}");
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        hasher.update(f.as_bytes());
        hasher.update([0u8]);
        hasher.update(&bytes);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn orchestrator_fixtures_are_sha_pinned() {
    let actual = compute_pin();
    assert_eq!(
        actual, FIXTURES_PIN,
        "Orchestrator fixtures changed — if intentional, update FIXTURES_PIN to {actual}"
    );
}

#[test]
fn wedge_scenario_drains_in_fifo_order_at_safe_points() {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/wedge-scenario.json"
    ))
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let buffer = OrchestratorBuffer::new();
    let orch = Orchestrator::new("orchestrator");

    let mut expected = Vec::new();
    for inst in v["instructions"].as_array().unwrap() {
        let id = inst["id"].as_u64().unwrap();
        let goal = inst["goal"].as_str().unwrap();
        buffer
            .enqueue(OrchestratorInstruction::new(OrchestratorInstructionId(id), goal, id).unwrap())
            .unwrap();
        expected.push(id);
    }

    let mut drained = Vec::new();
    while orch.is_safe_point() {
        match orch.drain_next(|| buffer.dequeue_at_safe_point()) {
            Some(i) => {
                drained.push(i.id.0);
                orch.begin_delegation();
                orch.complete_delegation();
            }
            None => break,
        }
    }
    assert_eq!(drained, expected, "wedge scenario drains in FIFO order");
}
