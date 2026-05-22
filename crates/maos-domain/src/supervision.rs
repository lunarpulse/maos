#![forbid(unsafe_code)]

//! Supervision domain types — crash detection, hung-Spirit detection,
//! silent-failure detection, and dead-Spirit task disposition.
//!
//! Per architecture §4.0.9 dependency-triangle rule, these traits live in
//! `maos-domain` so `maos-kernel-core`, `maos-acp`, and `maos-control` can
//! all consume them without circular deps.
//!
//! Story 5.3 — v0.3-β lands the invariants; Story 5.5x lands the real
//! subprocess driver (`OsProcessChildSupervisor`).

use std::future::Future;
use std::pin::Pin;

// ── OnCrashAction (FR50 dead-Spirit task disposition) ─────────

/// Dead-Spirit task disposition policy declared in manifest `[on_crash]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum OnCrashAction {
    /// Nack every in-flight task (default).
    #[default]
    Nack,
    /// Reassign in-flight tasks to a replica Spirit.
    ReassignToReplica,
    /// Escalate to operator surface (notification + audit).
    EscalateToOperator,
}

impl std::fmt::Display for OnCrashAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnCrashAction::Nack => write!(f, "nack"),
            OnCrashAction::ReassignToReplica => write!(f, "reassign-to-replica"),
            OnCrashAction::EscalateToOperator => write!(f, "escalate-to-operator"),
        }
    }
}

// ── CrashCause + FaultCause (ADR-033) ────────────────────────

/// High-level classification of why a Spirit process ended.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum CrashCause {
    /// Clean EOF after last full frame (subprocess form only).
    Voluntary,
    /// Any fault condition — always produces a halt receipt.
    Fault(FaultCause),
}

impl CrashCause {
    /// String representation used in TL payload JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            CrashCause::Voluntary => "voluntary",
            CrashCause::Fault(f) => f.as_str(),
        }
    }

    pub fn exit_signal(&self) -> Option<i32> {
        match self {
            CrashCause::Fault(FaultCause::SignaledByKernel { signal, .. }) => Some(*signal),
            _ => None,
        }
    }

    pub fn exit_code(&self) -> Option<i32> {
        match self {
            CrashCause::Fault(FaultCause::NonZeroExit { code, .. }) => Some(*code),
            _ => None,
        }
    }

    pub fn stderr_tail(&self) -> Option<String> {
        match self {
            CrashCause::Fault(FaultCause::NonZeroExit { stderr_tail, .. }) => stderr_tail.clone(),
            CrashCause::Fault(FaultCause::SignaledByKernel { stderr_tail, .. }) => stderr_tail.clone(),
            CrashCause::Fault(FaultCause::OomKilled { stderr_tail, .. }) => stderr_tail.clone(),
            _ => None,
        }
    }
}

/// Specific fault reason — emitted in `task.orphaned` payload.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FaultCause {
    /// Hook panic in rust-inproc form.
    Panic { hook_name: String, payload_preview: String },
    /// Subprocess received a fatal signal.
    SignaledByKernel { signal: i32, stderr_tail: Option<String> },
    /// Subprocess exited with non-zero code.
    NonZeroExit { code: i32, stderr_tail: Option<String> },
    /// OOM killer terminated the subprocess.
    OomKilled { stderr_tail: Option<String> },
    /// Hook exceeded its budget cap.
    Timeout { hook_name: String, cap_seconds: u64 },
    /// CBOR or wire-format truncation.
    Truncated { reason: String },
}

impl FaultCause {
    pub fn as_str(&self) -> &'static str {
        match self {
            FaultCause::Panic { .. } => "fault.panic",
            FaultCause::SignaledByKernel { .. } => "fault.signaled",
            FaultCause::NonZeroExit { .. } => "fault.non-zero-exit",
            FaultCause::OomKilled { .. } => "fault.oom_killed",
            FaultCause::Timeout { .. } => "fault.timeout",
            FaultCause::Truncated { .. } => "fault.truncated",
        }
    }
}

// ── ChildExitStatus (subprocess form) ─────────────────────────

/// Typed exit status from a subprocess Spirit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum ChildExitStatus {
    /// Clean EOF — Spirit shut down gracefully.
    CleanEof,
    /// Fatal signal from kernel.
    SignaledByKernel { signal: i32, stderr_tail: Option<String> },
    /// Non-zero exit code.
    NonZeroExit { code: i32, stderr_tail: Option<String> },
    /// OOM killer.
    OomKilled { stderr_tail: Option<String> },
    /// Budget timeout.
    Timeout { hook_name: String, cap_seconds: u64 },
}

// ── SubprocessSupervisor trait ────────────────────────────────

/// Handle to a spawned child — opaque outside the trait impl.
pub type ChildHandle = u64;

/// Trait for supervising subprocess Spirits. v0.3-β uses a test double;
/// Story 5.5x lands the production `OsProcessChildSupervisor`.
pub trait SubprocessSupervisor: Send + Sync + 'static {
    /// Spawn the Spirit subprocess.
    fn spawn_child(
        &self,
        spirit_id: &str,
    ) -> Result<ChildHandle, SupervisionError>;

    /// Future the supervisor awaits to observe child exit.
    fn wait_for_exit(
        &self,
        child: ChildHandle,
    ) -> Pin<Box<dyn Future<Output = ChildExitStatus> + Send>>;
}

// ── ReplicaResolver trait ─────────────────────────────────────

/// Resolve a replica Spirit for task reassignment (FR50).
/// v0.3-β default is `NullReplicaResolver` (always `None`).
pub trait ReplicaResolver: Send + Sync + 'static {
    /// Return the `spirit_pid` of a healthy replica for the given `intent_class`,
    /// or `None` if no replica is available.
    fn find_replica(&self, intent_class: &str) -> Option<u32>;
}

/// Default v0.3-β resolver — always returns `None`.
pub struct NullReplicaResolver;

impl ReplicaResolver for NullReplicaResolver {
    fn find_replica(&self, _intent_class: &str) -> Option<u32> {
        None
    }
}

// ── SupervisionError ──────────────────────────────────────────

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum SupervisionError {
    #[error("spawn failed: {0}")]
    SpawnFailed(String),
    #[error("unknown child handle: {0}")]
    UnknownChild(u64),
    #[error("supervisor already shut down")]
    ShutDown,
    #[error("heartbeat not wired: {0}")]
    HeartbeatNotWired(String),
    #[error("SCB not found for pid {0}")]
    ScbNotFound(u32),
    #[error("lock poisoned: {0}")]
    LockPoisoned(String),
}

// ── DispositionOutcome ────────────────────────────────────────

/// Result of applying FR50 disposition policy.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct DispositionOutcome {
    pub nacked: usize,
    pub reassigned: usize,
    pub escalated: usize,
    pub reassignment_failed: usize,
}

// ── HandleCrashReport / Error ─────────────────────────────────

/// Report produced by `CrashDetector::handle_crash`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HandleCrashReport {
    pub spirit_pid: u32,
    pub spirit_id: String,
    pub cause: CrashCause,
    pub detection_latency_ns: u64,
    pub task_orphaned_emitted_at_ns: u64,
    pub halt_receipts_produced: usize,
    pub tokens_revoked: usize,
    pub disposition_outcome: DispositionOutcome,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum HandleCrashError {
    #[error("spirit not loaded: pid={0}")]
    NotLoaded(u32),
    #[error("internal error: {0}")]
    Internal(String),
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_crash_action_default_is_nack() {
        assert_eq!(OnCrashAction::default(), OnCrashAction::Nack);
    }

    #[test]
    fn on_crash_action_serde_roundtrip_kebab_case() {
        let actions = [OnCrashAction::Nack, OnCrashAction::ReassignToReplica, OnCrashAction::EscalateToOperator];
        for a in actions {
            let json = serde_json::to_string(&a).unwrap();
            let back: OnCrashAction = serde_json::from_str(&json).unwrap();
            assert_eq!(a, back);
        }
    }

    #[test]
    fn on_crash_action_display() {
        assert_eq!(OnCrashAction::Nack.to_string(), "nack");
        assert_eq!(OnCrashAction::ReassignToReplica.to_string(), "reassign-to-replica");
        assert_eq!(OnCrashAction::EscalateToOperator.to_string(), "escalate-to-operator");
    }

    #[test]
    fn crash_cause_voluntary_str() {
        assert_eq!(CrashCause::Voluntary.as_str(), "voluntary");
    }

    #[test]
    fn fault_cause_signaled_by_kernel_str() {
        let cause = CrashCause::Fault(FaultCause::SignaledByKernel { signal: 9, stderr_tail: None });
        assert_eq!(cause.as_str(), "fault.signaled");
        assert_eq!(cause.exit_signal(), Some(9));
    }

    #[test]
    fn child_exit_status_signaled_debug() {
        let status = ChildExitStatus::SignaledByKernel { signal: 9, stderr_tail: Some("killed".into()) };
        let s = format!("{:?}", status);
        assert!(s.contains("SignaledByKernel"));
    }

    #[test]
    fn trait_object_safety_subprocess_supervisor() {
        // Compile-time check: trait object safety.
        let _: Option<Box<dyn SubprocessSupervisor>> = None;
    }

    #[test]
    fn trait_object_safety_replica_resolver() {
        let _: Option<Box<dyn ReplicaResolver>> = None;
    }

    #[test]
    fn null_replica_resolver_always_none() {
        let resolver = NullReplicaResolver;
        assert_eq!(resolver.find_replica("any"), None);
    }
}
