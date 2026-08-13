#![forbid(unsafe_code)]

//! Spirit + Host identity types + FrameKind discriminator — wire-stable since v0.1-β.
//!
//! These are the v0.3-β identity primitives for per-Spirit mailbox routing
//! (Story 3.1) and cross-Host A2A addressing (Epic 6 Story 6.3).

use alloc::string::String;

/// Frame-kind discriminator — wire-stable since Story 1b.1.
///
/// The canonical source of truth for the IAC frame kind taxonomy per
/// architecture §7.1. Variants 0..=6 are IAC bus frame kinds; variants
/// 7/8/9 are kernel-internal audit kinds that do NOT flow through the
/// IAC router.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::identity::FrameKind;
///
/// let kind = FrameKind::from_u8(0);
/// assert_eq!(kind, Some(FrameKind::TaskAssign));
///
/// let unknown = FrameKind::from_u8(99);
/// assert_eq!(unknown, None);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum FrameKind {
    /// IAC bus frame kind: task assignment.
    TaskAssign = 0,
    /// IAC bus frame kind: task completion.
    TaskComplete = 1,
    /// IAC bus frame kind: decision dispatch.
    DecisionDispatch = 2,
    /// IAC bus frame kind: epistemic halt.
    EpistemicHalt = 3,
    /// IAC bus frame kind: telemetry event.
    TelemetryEvent = 4,
    /// IAC bus frame kind: consent request.
    ConsentRequest = 5,
    /// IAC bus frame kind: retract.
    Retract = 6,

    /// Kernel-internal audit kind: capability invocation.
    CapabilityInvocation = 7,
    /// Kernel-internal audit kind: sandbox block.
    SandboxBlock = 8,
    /// Kernel-internal audit kind: inference call.
    InferenceCall = 9,

    /// Lifecycle hook crossed 80% of its declared execution budget.  This is
    /// a targeted kernel-to-invoking-Spirit MPSC event, not merely an audit row.
    BudgetWarning = 12,
    /// Lifecycle hook exceeded its declared execution budget.  Like the
    /// warning, this is delivered to the invoking Spirit and logged first.
    BudgetExceeded = 13,

    /// Story 6.2 AC6 — FR52: a line of stdout/stderr captured from a
    /// CliWrapperSpirit's invoked CLI subprocess. Payload shape:
    /// `{ cli_binary_path, invoking_spirit_id, output_stream: "stdout"|"stderr",
    ///   line: String, line_no: u64 }`. The row carries `intent_lineage`
    /// inherited from the invoking Spirit's session-originating intent.
    CliSubprocessOutput = 21,
    /// Story 6.4 — ADR-034 binding-v0.9. Sender-approved / receiver-rejected
    /// mid-frame becomes a `ConsentRupture` event; the original frame is
    /// quarantined for the rejected slice and DELIVERED to the accepted slice;
    /// the sender's mailbox receives a typed ConsentRupture frame so the
    /// application can decide retry/escalate/halt.
    ConsentRupture = 22,
    /// Story 6.4 — NFR-Scale-4. Per-(provider, credential) token bucket
    /// exhaustion emits this typed frame to the invoking Spirit. The frame
    /// is NOT a stalled call; the inference router returns
    /// `InferenceError::RateLimited { retry_after_ms }` simultaneously.
    RateLimited = 23,
    /// Story 6.5 — FR54. Inbound message from an external gateway (Telegram,
    /// Slack, Discord, Signal, Email). The payload carries the external
    /// sender id + opaque message bytes. Routed to the Spirit's `on_frame`
    /// hook (or custom hook per manifest).
    GatewayInbound = 24,
    /// Story 6.5 — FR54. Outbound message to an external gateway. The
    /// payload carries the recipient address + opaque message bytes. Gated
    /// by `Scope::GatewaySend` cap-token verification.
    GatewayOutbound = 25,
}

impl FrameKind {
    /// Parse a `u8` discriminant into a `FrameKind`, returning `None` for unknown values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maos_spirit_abi::identity::FrameKind;
    ///
    /// assert_eq!(FrameKind::from_u8(0), Some(FrameKind::TaskAssign));
    /// assert_eq!(FrameKind::from_u8(25), Some(FrameKind::GatewayOutbound));
    /// assert_eq!(FrameKind::from_u8(99), None);
    /// ```
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::TaskAssign),
            1 => Some(Self::TaskComplete),
            2 => Some(Self::DecisionDispatch),
            3 => Some(Self::EpistemicHalt),
            4 => Some(Self::TelemetryEvent),
            5 => Some(Self::ConsentRequest),
            6 => Some(Self::Retract),
            7 => Some(Self::CapabilityInvocation),
            8 => Some(Self::SandboxBlock),
            12 => Some(Self::BudgetWarning),
            13 => Some(Self::BudgetExceeded),
            21 => Some(Self::CliSubprocessOutput),
            22 => Some(Self::ConsentRupture),
            23 => Some(Self::RateLimited),
            24 => Some(Self::GatewayInbound),
            25 => Some(Self::GatewayOutbound),
            _ => None,
        }
    }
}

/// Unique Spirit identifier — String newtype keyed on PID-rebind safety.
///
/// Architecture §7.1: "Every IAC frame carries a `spirit_id` field for
/// per-Spirit routing." The newtype exists so that a bare `String` cannot
/// be accidentally used where a `SpiritId` is required (address typing
/// per ADR-010).
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::identity::SpiritId;
///
/// let id = SpiritId::from("my-spirit");
/// let id2 = SpiritId::from(String::from("my-spirit"));
/// assert_eq!(id, id2);
/// assert_eq!(id.as_str(), "my-spirit");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SpiritId(pub String);

impl SpiritId {
    /// Borrow as string slice.
    ///
    /// # Example
    ///
    /// ```rust
    /// use maos_spirit_abi::identity::SpiritId;
    ///
    /// let id = SpiritId::from("my-spirit");
    /// assert_eq!(id.as_str(), "my-spirit");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for SpiritId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SpiritId {
    fn from(s: &str) -> Self {
        Self(String::from(s))
    }
}

/// Host identifier — stable across process restarts.
///
/// Same-Host routing at v0.3-β uses `None` for `FrameAddress.host_id`;
/// cross-Host A2A (Story 6.3) fills `Some(host_id)`.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::identity::HostId;
///
/// let host = HostId("node-east-1".into());
/// assert_eq!(host.as_str(), "node-east-1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HostId(pub String);

impl HostId {
    /// Borrow as string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Role for role-based IAC addressing.
///
/// Architecture §7.1 comment: per-frame-kind channel class router resolves
/// Spirit identity from a `SpiritRole` when the sender targets a role rather
/// than a specific Spirit. v0.3-β supports the four Director-surface roles
/// enumerated here; the full role ontology ships in Story 6.1.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::identity::SpiritRole;
///
/// let role = SpiritRole::Worker;
///
/// match role {
///     SpiritRole::Director => { /* orchestration logic */ }
///     SpiritRole::Observer => { /* monitoring logic */ }
///     SpiritRole::Worker => { /* task execution logic */ }
///     SpiritRole::Orchestrator => { /* multi-Spirit coordination */ }
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpiritRole {
    /// Director role: orchestration and decision authority.
    Director,
    /// Observer role: monitoring and telemetry.
    Observer,
    /// Worker role: task execution.
    Worker,
    /// Orchestrator role: multi-Spirit coordination.
    Orchestrator,
}
