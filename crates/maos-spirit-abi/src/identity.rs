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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum FrameKind {
    TaskAssign = 0,
    TaskComplete = 1,
    DecisionDispatch = 2,
    EpistemicHalt = 3,
    TelemetryEvent = 4,
    ConsentRequest = 5,
    Retract = 6,
    CapabilityInvocation = 7,
    SandboxBlock = 8,
    InferenceCall = 9,
    /// Story 6.2 AC6 — FR52: a line of stdout/stderr captured from a
    /// CliWrapperSpirit's invoked CLI subprocess. Payload shape:
    /// `{ cli_binary_path, invoking_spirit_id, output_stream: "stdout"|"stderr",
    ///   line: String, line_no: u64 }`. The row carries `intent_lineage`
    /// inherited from the invoking Spirit's session-originating intent.
    CliSubprocessOutput = 21,
}

impl FrameKind {
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
            9 => Some(Self::InferenceCall),
            21 => Some(Self::CliSubprocessOutput),
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SpiritId(pub String);

impl SpiritId {
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct HostId(pub String);

impl HostId {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpiritRole {
    Director,
    Observer,
    Worker,
    Orchestrator,
}
