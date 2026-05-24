#![forbid(unsafe_code)]

//! Policy decision types.

use maos_domain::invariants::i1::Scope;

/// Outcome of policy evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Capability is permitted.
    Allow,
    /// Capability is denied.
    Deny,
    /// Requires interactive approval.
    RequireApproval { class: ApprovalClass },
}

/// Approval class per architecture §4.3.3 taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApprovalClass {
    /// Read-only scoped operation.
    ReadonlyScoped,
    /// Read-only search operation.
    ReadonlySearch,
    /// Mutating operation.
    Mutating,
    /// Execution-capable operation.
    ExecCapable,
    /// Control plane operation.
    ControlPlane,
    /// Interactive operation.
    Interactive,
}

/// Intent classification for approval policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Intent {
    /// File system read.
    FsRead { subtree: String },
    /// File system write.
    FsWrite { subtree: String },
    /// Network HTTPS call.
    NetHttps { domain: String },
    /// Process execution.
    ProcExec { binary: String },
    /// Sub-Spirit spawn.
    SubSpiritSpawn { class: String },
    /// LLM provider inference.
    ProviderInfer { provider: String },
    /// IAC frame send.
    IacSend { peer_class: String },
    /// Memory read.
    MemRead { scope: String },
    /// Memory write.
    MemWrite { scope: String },
    /// Self-telemetry read (FR56) — Story 4.3.
    SelfTelemetryRead,
    /// Log recall (participant-scoped) — Story 4.4.
    LogRecall,
    /// Log fetch (single-frame payload) — Story 4.4.
    LogFetch,
    /// Distillate write (I11 audit chain) — Story 4.4.
    DistillateWrite,
    /// MCP tool invocation — Story 5.5c.
    McpCall { server: String, tool: String },
}

/// Trust tier classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TrustTier {
    /// Public/untrusted — forced to T2 floor.
    #[default]
    PublicUntrusted,
    /// Known — T1 floor.
    Known,
    /// Verified — T0 floor.
    Verified,
    /// Internal — T0 floor.
    Internal,
}

/// A capability request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub scope: Scope,
}
