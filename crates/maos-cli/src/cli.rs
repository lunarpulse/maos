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
    /// Start a Spirit (Story 5.1 lifecycle verbs).
    Start(StartArgs),
    /// Stop a Spirit (Story 5.1 lifecycle verbs).
    Stop(StopArgs),
    /// Unload a Spirit (Story 5.1 lifecycle verbs).
    Unload(UnloadArgs),
    /// Run a one-shot Spirit invocation (Story 1b.5b).
    Run(RunArgs),
    /// Audit-trail subcommands (Story 1b.5b query subcommand; FR42–44 sealed-export at v1.0).
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
    Query,
}
