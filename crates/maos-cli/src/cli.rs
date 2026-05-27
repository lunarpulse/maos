//! Top-level command tree for `maosctl`.
//!
//! Per Story 1a.4 epic AC1: six v0.1 verbs (`install`, `start`, `stop`,
//! `unload`, `run`, `audit`) declared as subcommands. Every subcommand
//! body at v0.1-α emits a deterministic "not-yet-implemented" diagnostic
//! and exits with code 2 — the real bodies land at the cited stories.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "maosctl",
    version,
    about = "MAOS operator control plane CLI (v0.1-α scaffold)",
    long_about = None,
)]
pub struct Cli {
    /// Suppress all ANSI color sequences (per NFR-Ops-5).
    /// Also honored via NO_COLOR and TERM=dumb environment variables.
    #[arg(long, global = true)]
    pub plain: bool,

    /// Telemetry opt-in flag (per FR7). Default: `off` at v0.1-α
    /// (FR7 declares opt-in default; the actual telemetry surface
    /// lands at v0.5).
    #[arg(long, value_enum, default_value_t = TelemetryMode::Off, global = true)]
    pub telemetry: TelemetryMode,

    #[command(subcommand)]
    pub command: Subcommand,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum TelemetryMode {
    On,
    Off,
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Install a Spirit (Story 1b.5b lands the real body).
    Install(InstallArgs),
    /// Start a Spirit — writes one `LifecycleEvent::Start` Lifecycle Journal
    /// entry and exits (v0.1-β, Story 1b.5c). Supervised lifecycle with
    /// process spawn + mailbox lands at Epic 5 (Story 5.1).
    Start(StartArgs),
    /// Stop a Spirit — writes one `LifecycleEvent::Halt` Lifecycle Journal
    /// entry and exits (v0.1-β, Story 1b.5c). The supervisor that consumes
    /// the journal to actually signal a running Spirit ships at Epic 5
    /// (Story 5.1).
    Stop(StopArgs),
    /// Unload a Spirit — writes one `LifecycleEvent::Unload` Lifecycle
    /// Journal entry and exits (v0.1-β, Story 1b.5c). Graceful shutdown
    /// with mailbox drain lands at Epic 5 (Story 5.1).
    Unload(UnloadArgs),
    /// Uninstall a Spirit — removes the Spirit from the registry and emits
    /// a proof-of-erasure record enumerating all removed substrate state
    /// (Story 6.5 / FR65 v0.5 structural stub; full Merkle proof lands at
    /// Story 9.2).
    Uninstall(UninstallArgs),
    /// Run a one-shot Spirit invocation (Story 1b.5a / 1b.5b).
    Run(RunArgs),
    /// Audit-trail subcommands. `query` is the FR4 mechanical-verification surface
    /// (Story 1b.5b); FR42–44 sealed-export lands at v1.0 (Story 9.1).
    Audit(AuditArgs),
    /// Shift the runtime posture of a Spirit (Story 3.2).
    Posture(PostureArgs),
    /// Inspect or resolve a Spirit halt (Story 3.3).
    ///
    /// At v0.3-β `list` reads the Transparency Log for recent halts;
    /// `resolve` writes the director's resolution to the Approval Decision
    /// Log via `journal_halt_resolution`. The live pending-halt set + halt
    /// receipt lands at Story 4.1's `invoke_halt` mechanism.
    Halt(HaltArgs),
    /// Inspect or enqueue Orchestrator instructions (Story 3.4).
    ///
    /// At v0.3-β `status` reads the per-Spirit `OrchestratorBuffer` for
    /// pending counts; `queue` enqueues a natural-language instruction.
    /// The Orchestrator-class Spirit (Story 8.4 founder-loop) consumes
    /// queued instructions at safe sequence points between task completions
    /// via `OrchestratorBuffer::dequeue_at_safe_point`.
    Orchestrator(OrchestratorArgs),
    /// Pause a Spirit — interrupts in-flight autonomous actions, preserves
    /// state across pause/resume, recalls Orchestrator-buffered actions on
    /// resume (Story 3.4). P99 ≤2s interruption (FR51 a).
    Pause(PauseArgs),
    /// Resume a paused Spirit — restores from preserved state, replays
    /// Orchestrator-buffered pending actions per FR20 (Story 3.4, FR51 c).
    Resume(ResumeArgs),
    /// Revoke a capability token — in-flight operations using the token
    /// fail-safe with bounded time (Story 3.4, FR51 d). Revocation is
    /// journaled to the Approval Decision Log with director identity +
    /// reason per FR42.
    RevokeToken(RevokeTokenArgs),
    /// Spirit lifecycle operations (upgrade, hot-swap precheck, etc.).
    /// Story 5.2 ships `hot-swap-precheck`; Story 5.4 ships `upgrade`.
    Spirit(SpiritArgs),
    /// Revocation management — import signed CRLs, list applied CRLs (Story 5.4).
    Revocations(RevocationsArgs),
}

#[derive(clap::Args, Debug)]
pub struct InstallArgs {
    /// Spirit registry URI or local path (placeholder at v0.1-α).
    pub source: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct StartArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct StopArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct UnloadArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct UninstallArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct RunArgs {
    pub spirit: Option<String>,
    pub args: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub query: Option<AuditQuery>,
}

#[derive(clap::Subcommand, Debug)]
pub enum AuditQuery {
    /// Tail the local Transparency Log (Story 1b.5b).
    ///
    /// Per AC1, `--spirit <name>` filters to one Spirit and projects each row
    /// to the FR4 NDJSON schema (call_id, capability_token, spirit_pid,
    /// boot_nonce, call_type, timestamp_ns). `--format plain` produces
    /// human-readable tabular text. Both formats emit zero ANSI bytes
    /// when `NO_COLOR`, `TERM=dumb`, or `--plain` is set (NFR-Ops-5).
    Query {
        /// Filter by Spirit name. At v0.1-β only `hello-spirit` is resolvable
        /// (maps to `spirit_pid = 0` per Story 1b.5a's one-shot path).
        /// The full Spirit registry / scheduler lookup is Epic 5.
        #[arg(long)]
        spirit: Option<String>,

        /// Output format. `ndjson` (default) emits the FR4 schema, one JSON
        /// object per line. `plain` emits a human-readable tabular form.
        #[arg(long, value_enum, default_value_t = AuditFormat::Ndjson)]
        format: AuditFormat,
    },
}

/// Output format for `maosctl audit query`.
///
/// Both formats are accessibility-clean: zero ANSI escape bytes when the
/// NFR-Ops-5 cascade (`--plain` / `NO_COLOR=1` / `TERM=dumb`) is engaged.
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditFormat {
    /// FR4 NDJSON: one JSON object per line; AC1 mandatory-field schema.
    Ndjson,
    /// Human-readable tabular text; never emits ANSI escapes.
    Plain,
}

#[derive(clap::Args, Debug)]
pub struct PostureArgs {
    /// Spirit ID to shift.
    pub spirit: String,
    /// New runtime posture: cautious | assistive | autonomous-with-halt
    #[arg(long, value_enum)]
    pub shift: PostureChoice,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostureChoice {
    Cautious,
    Assistive,
    #[clap(name = "autonomous-with-halt")]
    AutonomousWithHalt,
}

/// Story 3.3 — halt inspection / resolution.
#[derive(clap::Args, Debug)]
pub struct HaltArgs {
    #[command(subcommand)]
    pub op: HaltOp,
}

#[derive(clap::Subcommand, Debug)]
pub enum HaltOp {
    /// List recent halts from the Transparency Log.
    List {
        /// Filter to halts emitted by a specific Spirit.
        #[arg(long)]
        spirit: Option<String>,
        /// Maximum number of halts to show (default: 20).
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Resolve a halt by ID with one of three documented kinds.
    Resolve {
        /// HaltId returned by `maosctl halt list`.
        halt_id: String,
        /// Spirit owning the halt (required — Story 4.1 will derive
        /// from halt_id, but at 3.3 the operator supplies it).
        #[arg(long)]
        spirit: String,
        /// Resolution kind.
        #[arg(long, value_enum)]
        kind: ResolutionKindChoice,
        /// Required when `--kind provided_context`: the missing context
        /// to append to Spirit working memory.
        #[arg(long, required_if_eq("kind", "provided-context"))]
        text: Option<String>,
        /// Required when `--kind authorized_override`: operator-policy
        /// reference authorizing the override.
        #[arg(long, required_if_eq("kind", "authorized-override"))]
        operator_policy: Option<String>,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolutionKindChoice {
    #[clap(name = "provided-context")]
    ProvidedContext,
    #[clap(name = "accepted-halt")]
    AcceptedHalt,
    #[clap(name = "authorized-override")]
    AuthorizedOverride,
}

/// Story 3.4 — Orchestrator instruction buffer subcommand.
#[derive(clap::Args, Debug)]
pub struct OrchestratorArgs {
    #[command(subcommand)]
    pub op: OrchestratorOp,
}

#[derive(clap::Subcommand, Debug)]
pub enum OrchestratorOp {
    /// Enqueue an instruction onto the per-Spirit Orchestrator buffer.
    Queue {
        /// Spirit ID to enqueue against (v0.3-β: only `hello-spirit`).
        #[arg(long)]
        spirit: String,
        /// Natural-language instruction (free-form at v0.3-β; typed
        /// structuring lands at Story 8.4).
        instruction: String,
    },
    /// Show pending instruction count for a Spirit (read-only).
    Status {
        #[arg(long)]
        spirit: String,
    },
}

/// Story 3.4 — pause a Spirit.
#[derive(clap::Args, Debug)]
pub struct PauseArgs {
    /// Spirit ID to pause (v0.3-β: only `hello-spirit`).
    pub spirit: String,
}

/// Story 3.4 — resume a Spirit.
#[derive(clap::Args, Debug)]
pub struct ResumeArgs {
    /// Spirit ID to resume.
    pub spirit: String,
}

/// Story 3.4 — revoke a capability token.
#[derive(clap::Args, Debug)]
pub struct RevokeTokenArgs {
    /// TokenId as 32-char lowercase hex (the wire format `CapabilityToken::token_id`
    /// renders to via `format!("{:032x}", ...)` — same shape as
    /// `cap_tokens/body.rs` golden tests).
    pub token_id: String,
    /// Optional director-supplied reason (free-form). Stored verbatim
    /// in the Approval Decision Log `reasoning` column per FR42.
    #[arg(long)]
    pub reason: Option<String>,
}

/// Story 5.2 — Spirit lifecycle operations.
#[derive(clap::Args, Debug)]
pub struct SpiritArgs {
    #[command(subcommand)]
    pub op: SpiritOp,
}

/// Spirit subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum SpiritOp {
    /// Run a hot-swap precondition check (ADR-036, REPORTING-ONLY at v0.3-β).
    /// Prints JSON verdict to stdout. Exit 0 = safe; exit 2 = violation.
    HotSwapPrecheck {
        /// Spirit ID to check (e.g. "butler").
        spirit: String,
        /// Predecessor version string (e.g. "0.3.1").
        #[arg(long)]
        from: String,
        /// Path to the successor's manifest TOML file.
        #[arg(long)]
        to: String,
    },
    /// Upgrade a Spirit to a successor version with a declared policy (Story 5.4, FR49).
    Upgrade {
        /// Spirit ID to upgrade (e.g. "butler").
        spirit: String,
        /// Path to the successor manifest TOML.
        #[arg(long)]
        to: String,
        /// Upgrade policy. Default: hot-swap.
        #[arg(long, value_enum, default_value_t = UpgradePolicyArg::HotSwap)]
        policy: UpgradePolicyArg,
    },
    /// Inspect a Spirit's sandbox report (Story 5.5a, AC5).
    Inspect {
        /// Spirit ID to inspect (e.g. "butler").
        spirit: String,
        /// Print sandbox isolation details.
        #[arg(long)]
        sandbox: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpgradePolicyArg {
    HotSwap,
    ColdSwap,
    Migrator,
}

/// Revocations subcommands.
#[derive(clap::Args, Debug)]
pub struct RevocationsArgs {
    #[command(subcommand)]
    pub op: RevocationsOp,
}

#[derive(clap::Subcommand, Debug)]
pub enum RevocationsOp {
    /// Import a signed CRL from offline media (FR60).
    Import {
        /// Path to the signed CRL JSON file.
        file: std::path::PathBuf,
        /// Re-apply even if this CRL was already imported.
        #[arg(long)]
        force: bool,
    },
    /// List already-applied CRLs by id + apply timestamp.
    List,
}
