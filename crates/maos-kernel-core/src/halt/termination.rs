#![forbid(unsafe_code)]

//! Story 4.1 AC4 — `terminate_spirit` function.
//!
//! Called from the existing `MAOS_ONE_SHOT={stop, unload}` one-shot
//! arms in `crates/maos-bin/src/main.rs`. Drains all pending halts for
//! the Spirit via `HaltRegistry::drain_for_spirit(spirit_pid)`, writes
//! a `HaltReceipt` to the Transparency Log for each, and returns
//! `Vec<HaltReceipt>`.
//!
//! Story 5.3 will own the unplanned-termination paths (SIGKILL,
//! hung-Spirit detection) that AC4's 1000-termination corpus probes;
//! Story 4.1 scaffolds the planned-termination receipt path.

use crate::halt::HaltRegistry;
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_domain::halt::{HaltId, HaltReceipt, HaltState, TerminationKind};
use maos_domain::invariants::i3::FrameOrigin;
use std::time::{SystemTime, UNIX_EPOCH};

/// Drain all pending halts for a Spirit, produce HaltReceipts, and
/// write them to the Transparency Log. For Spirits with zero pending
/// halts, writes ONE "term-{spirit_pid}-{boot_nonce}" receipt.
///
/// Returns `Vec<HaltReceipt>` — the caller decides what to do with them.
pub fn terminate_spirit(
    tl: &TransparencyLogAdapter,
    registry: &HaltRegistry,
    spirit_pid: u32,
    spirit_id: &str,
    kind: TerminationKind,
    boot_nonce: u64,
) -> Vec<HaltReceipt> {
    let timestamp_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let pending = registry.drain_for_spirit(spirit_pid);
    let kind_str = kind.as_str();

    if pending.is_empty() {
        // No halts — write one termination-marker receipt
        let term_halt_id = format!("term-{spirit_id}-{spirit_pid}-{timestamp_ns}");
        let hid = match HaltId::new(&term_halt_id) {
            Ok(h) => h,
            Err(_) => {
                let fallback = format!("term-fallback-{spirit_pid}");
                HaltId::new(fallback).unwrap_or_else(|_| {
                    HaltId::new("term-unknown").expect("hardcoded fallback halt_id")
                })
            }
        };

        tl.insert_frame_event(
            FrameKind::EpistemicHalt,
            spirit_pid,
            None,
            kind_str,
            &[],
            FrameOrigin::Kernel,
        );
        let frame_id = tl.last_frame_id();

        let receipt = HaltReceipt::new(hid.clone(), timestamp_ns, spirit_pid, boot_nonce, frame_id)
            .with_resolution(HaltState::Terminated, kind_str, timestamp_ns);

        // Write serialized receipt to TL
        let payload = match serde_json::to_vec(&receipt) {
            Ok(p) => p,
            Err(e) => format!(r#"{{"error":"{}","halt_id":"{}"}}"#, e, hid.as_str()).into_bytes(),
        };
        tl.insert_frame_event(
            FrameKind::EpistemicHalt,
            spirit_pid,
            None,
            kind_str,
            &payload,
            FrameOrigin::Kernel,
        );

        vec![receipt]
    } else {
        let mut receipts = Vec::with_capacity(pending.len());
        for (halt_id, _state) in pending {
            tl.insert_frame_event(
                FrameKind::EpistemicHalt,
                spirit_pid,
                None,
                kind_str,
                &[],
                FrameOrigin::Kernel,
            );
            let frame_id = tl.last_frame_id();

            let receipt = HaltReceipt::new(
                halt_id.clone(),
                timestamp_ns,
                spirit_pid,
                boot_nonce,
                frame_id,
            )
            .with_resolution(HaltState::Terminated, kind_str, timestamp_ns);

            let payload = match serde_json::to_vec(&receipt) {
                Ok(p) => p,
                Err(e) => {
                    format!(r#"{{"error":"{}","halt_id":"{}"}}"#, e, halt_id.as_str()).into_bytes()
                }
            };
            tl.insert_frame_event(
                FrameKind::EpistemicHalt,
                spirit_pid,
                None,
                kind_str,
                &payload,
                FrameOrigin::Kernel,
            );
            receipts.push(receipt);
        }
        receipts
    }
}
