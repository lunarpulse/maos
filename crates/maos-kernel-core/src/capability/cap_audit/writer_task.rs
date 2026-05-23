#![forbid(unsafe_code)]

//! Audit writer task — spawned at composition root.

use std::sync::Arc;

use tokio::sync::mpsc::Receiver;

use super::{CapAuditEvent, VerifyOutcome};
use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_domain::invariants::i3::FrameOrigin;

fn token_id_to_capability_token(token_id: &maos_domain::invariants::i1::TokenId) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[0..16].copy_from_slice(&token_id.0);
    buf
}

/// The audit writer task state.
pub struct CapAuditWriter;

impl CapAuditWriter {
    /// Spawn the writer task. Returns the `JoinHandle`.
    pub fn spawn(
        mut receiver: Receiver<CapAuditEvent>,
        transparency_log: Arc<TransparencyLogAdapter>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                write_to_transparency_log(&event, &transparency_log);
            }
        })
    }
}

fn write_to_transparency_log(event: &CapAuditEvent, log: &TransparencyLogAdapter) {
    match event {
        CapAuditEvent::Issue {
            token_id,
            spirit_pid,
            scope,
            ttl_secs: _,
        } => {
            let intent = format!("cap.issue.{:?}", std::mem::discriminant(scope));
            let payload = serde_json::to_vec(scope).unwrap_or_default();
            log.insert_frame_event(
                FrameKind::CapabilityInvocation,
                *spirit_pid,
                Some(&token_id_to_capability_token(token_id)),
                &intent,
                &payload,
                FrameOrigin::Kernel,
            );
        }
        CapAuditEvent::Verify {
            token_id,
            spirit_pid,
            outcome,
        } => {
            let intent = match outcome {
                VerifyOutcome::Ok => "cap.verify.ok",
                _ => "cap.verify.fail",
            };
            log.insert_frame_event(
                FrameKind::CapabilityInvocation,
                *spirit_pid,
                Some(&token_id_to_capability_token(token_id)),
                intent,
                &[],
                FrameOrigin::Kernel,
            );
        }
        CapAuditEvent::Revoke { token_id, reason } => {
            let spirit_pid = match reason {
                crate::capability::cap_tokens::RevokeReason::SpiritUnload {
                    spirit_pid, ..
                } => *spirit_pid,
                _ => 0,
            };
            log.insert_frame_event(
                FrameKind::CapabilityInvocation,
                spirit_pid,
                Some(&token_id_to_capability_token(token_id)),
                "cap.revoke",
                &[],
                FrameOrigin::Kernel,
            );
        }
        CapAuditEvent::Invocation {
            token_id,
            spirit_pid,
            capability_token_bytes,
            intent,
            payload,
        } => {
            let mut combined = capability_token_bytes.clone();
            combined.extend_from_slice(payload);
            log.insert_frame_event(
                FrameKind::CapabilityInvocation,
                *spirit_pid,
                Some(&token_id_to_capability_token(token_id)),
                intent,
                &combined,
                FrameOrigin::Kernel,
            );
        }
        CapAuditEvent::SandboxBlock {
            spirit_pid,
            attempted_syscall,
            sandbox_tier,
        } => {
            let intent = format!("sandbox.block.{}", attempted_syscall);
            let payload = format!("tier={}", sandbox_tier.0).into_bytes();
            log.insert_frame_event(
                FrameKind::SandboxBlock,
                *spirit_pid,
                None,
                &intent,
                &payload,
                FrameOrigin::Kernel,
            );
        }
    }
}
