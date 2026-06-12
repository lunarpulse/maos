//! cap-audit bridge — first production caller of `emit_sandbox_block`.
//!
//! On every `SandboxBlock` observation inside T3 (detected via container
//! runtime exit cause + stderr, or via in-container T2 stack's parent-side
//! `CapAuditEvent` forwarding), this bridge emits a
//! `CapAuditEvent::SandboxBlock` → Transparency Log `FrameKind::SandboxBlock = 8`.
//!
//! Uses `try_send` + `cap_audit::record_drop()` on saturation per
//! ADR-030: NEVER block on the audit channel.

use crate::capability::cap_audit::{self, CapAuditEvent};
use maos_domain::invariants::i9::SandboxTier;

pub fn emit_t3_escape_block(
    sender: &cap_audit::Sender,
    host_pid: u32,
    category: &str,
    vector: &str,
) {
    let event = CapAuditEvent::SandboxBlock {
        spirit_pid: host_pid,
        attempted_syscall: format!("container.escape.{category}.{vector}"),
        sandbox_tier: SandboxTier::T3,
    };
    if sender.try_send(event).is_err() {
        cap_audit::record_drop();
    }
}

pub fn emit_t3_escape_block_probe(host_pid: u32, category: &str, vector: &str) {
    eprintln!(
        "maos: T3 escape block probe: host_pid={}, category={}, vector={}",
        host_pid, category, vector
    );
}
