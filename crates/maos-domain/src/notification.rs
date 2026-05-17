//! Notification surface types — domain-level per architecture §7.4.
//!
//! These types are shared between `maos-kernel-core` (Approval Manager
//! emits events) and `maos-director-surface` (dispatcher sends them to
//! channels). Putting them in `maos-domain` avoids a circular dependency.

/// The three notification levels from §7.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NotificationLevel {
    Immediate,
    Queue,
    Digest,
}

/// The notification surface a kernel event dispatches into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSurface {
    Terminal,
    AcpEditor,
    MobilePush,
}

/// What the kernel hands to a NotificationChannel.
///
/// Story 3.1 ships `TaskAssigned` + `ApprovalPrompt`;
/// Story 3.3 adds `Halt`; Story 3.4 adds `AnomalyFlagged`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum NotificationEvent {
    TaskAssigned {
        frame_id: [u8; 16],
        from: String,
        goal: String,
    },
    ApprovalPrompt {
        decision_id: u64,
        class: ApprovalClass,
        capability: String,
        reasoning: Option<String>,
    },
}

/// Maps to architecture §4.3.3's 6-class taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalClass {
    ReadonlyScoped,
    ReadonlySearch,
    Mutating,
    ExecCapable,
    ControlPlane,
    Interactive,
}
