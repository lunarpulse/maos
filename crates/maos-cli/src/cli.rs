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
    /// Run a one-shot Spirit invocation (Story 1b.5a / 1b.5b).
    Run(RunArgs),
    /// Audit-trail subcommands. `query` is the FR4 mechanical-verification surface
    /// (Story 1b.5b); FR42–44 sealed-export lands at v1.0 (Story 9.1).
    Audit(AuditArgs),
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
