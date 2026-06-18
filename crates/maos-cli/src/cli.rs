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
    /// Install a Spirit or verify a release artifact (Story 1b.5b, Story 9.4).
    /// Supports legacy spirit install, local release verification (`--from-local`),
    /// and remote fetch stub (source = release tag like "v0.5.0").
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
    /// Story 7.2 (FR60) — import a signed Spirit bundle from offline media
    /// (air-gapped operator path). Verifies the Ed25519 signing chain and
    /// admits the Spirit through the same Story 5.5d admission code path
    /// network installs use.
    Import(ImportArgs),
    /// Story 7.4 (FR39) — the skill ecosystem operator surface. `list`
    /// discovers `maos.skill.v1` skills on the conventional search path and
    /// shows their admission state; `approve`/`reject` give the pending
    /// operator-admission queue its exit (no skill activates without `approve`).
    Skills(SkillsArgs),
    /// GDPR Article 17 — forget a principal and emit a receipt.
    Forget(ForgetArgs),
    /// Story 9.3b — governance operations on the schema-lifecycle registry.
    Governance(GovernanceArgs),
    /// Story 9.4 AC-3 — Transparency Log backup/DR (region-scoped).
    Backup(BackupArgs),
}

/// Story 9.4 — `maosctl backup <create|verify|restore>`.
#[derive(clap::Args, Debug)]
pub struct BackupArgs {
    #[command(subcommand)]
    pub op: BackupOp,
}

#[derive(clap::Subcommand, Debug)]
pub enum BackupOp {
    /// Create a WAL-checkpoint-consistent backup of the Transparency Log.
    Create {
        /// Destination path for the backup file.
        #[arg(long)]
        dest: String,
    },
    /// Verify backup integrity via Merkle root cross-check.
    Verify {
        /// Path to the backup file.
        #[arg(long)]
        backup: String,
    },
    /// Restore from backup to a target path.
    Restore {
        /// Path to the backup file.
        #[arg(long)]
        backup: String,
        /// Target path for the restored TL.
        #[arg(long)]
        target: String,
    },
}
#[derive(clap::Args, Debug)]
pub struct GovernanceArgs {
    #[command(subcommand)]
    pub op: GovernanceOp,
}

#[derive(clap::Subcommand, Debug)]
pub enum GovernanceOp {
    /// Admit a schema version into the lifecycle registry.
    ///
    /// Emits a `FrameKind::GovernanceEvent` frame with a `SchemaLifecyclePayload`
    /// and appends the ratified entry to the `schema_lifecycle_registry` table.
    Admit {
        /// Stable schema lineage id, e.g. `compliance.claim.gdpr-erasure`.
        #[arg(long)]
        schema_id: String,
        /// Schema version number.
        #[arg(long)]
        version: u32,
        /// SHA-256 hex of the canonical schema bytes.
        #[arg(long)]
        content_hash: String,
        /// Optional prior version content hash for the chain.
        #[arg(long)]
        supersedes: Option<String>,
        /// ADR reference that ratified this schema version.
        #[arg(long)]
        ratified_by: String,
        /// When the schema takes effect (nanoseconds since Unix epoch).
        #[arg(long)]
        effective_at_ns: u64,
    },
}

/// Story 9.2 — `maosctl forget --principal <id> [--reason <legal-hold>]`.
#[derive(clap::Args, Debug)]
pub struct ForgetArgs {
    /// Principal identifier to erase (e.g. `alice@example.org`).
    #[arg(long, value_name = "PRINCIPAL")]
    pub principal: String,
    /// Optional legal-hold reason, e.g. `legal-hold` or `legal-hold:<case-ref>`.
    #[arg(long, value_name = "REASON")]
    pub reason: Option<String>,
}

/// Story 7.4 — `maosctl skills <list|approve|reject>`.
#[derive(clap::Args, Debug)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub op: SkillsOp,
}

#[derive(clap::Subcommand, Debug)]
pub enum SkillsOp {
    /// Discover skills on the conventional `[skills.search_path]` roots and
    /// render each with its admission state (all freshly-discovered skills are
    /// `Pending` — discovery never auto-admits).
    List {
        /// Override the search roots (repeatable). Defaults to the three
        /// conventional paths (`~/.maos/skills/`, `_bmad/skills/`,
        /// `/usr/share/maos/skills/`).
        #[arg(long)]
        root: Vec<String>,
    },
    /// Operator-admit a pending skill by id (FR39 admission exit).
    Approve {
        /// The skill id (`maosctl skills list` shows ids).
        skill_id: String,
        /// Operator identity recorded in the audit journal (AC-3 `actor`).
        /// Defaults to `$USER`, then `operator`.
        #[arg(long, value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Operator-reject a pending skill by id (FR39 admission exit).
    Reject {
        /// The skill id (`maosctl skills list` shows ids).
        skill_id: String,
        /// Operator identity recorded in the audit journal (AC-3 `actor`).
        /// Defaults to `$USER`, then `operator`.
        #[arg(long, value_name = "ACTOR")]
        actor: Option<String>,
    },
}

#[derive(clap::Args, Debug)]
pub struct InstallArgs {
    /// Spirit registry URI, local path, or release tag (e.g., "v0.5.0").
    /// At v0.5 only the legacy spirit install path uses this argument.
    pub source: Option<String>,

    /// GitHub Releases URL override (default: repo from Cargo.toml metadata).
    /// Remote fetch is deferred to a v1.0/AC-2 follow-up.
    #[arg(long)]
    pub release_url: Option<String>,

    /// Hex-encoded Ed25519 public key for signature verification.
    /// Default: bundled release public key.
    #[arg(long)]
    pub release_pubkey: Option<String>,

    /// Verify an already-downloaded artifact without installing.
    #[arg(long)]
    pub verify_only: bool,

    /// Path to a locally-staged release artifact directory.
    /// Must contain SHA256SUMS, SHA256SUMS.sig, and the binary.
    #[arg(long)]
    pub from_local: Option<String>,

    /// Installation prefix directory. Default: parent of the current executable.
    #[arg(long)]
    pub prefix: Option<std::path::PathBuf>,
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
        /// Filter by Spirit name. Resolved via TL-scan of lifecycle frames
        /// (Story 9.1, Decision E).
        #[arg(long)]
        spirit: Option<String>,

        /// Output format. `ndjson` (default) emits the FR4 schema, one JSON
        /// object per line. `plain` emits a human-readable tabular form.
        #[arg(long, value_enum, default_value_t = AuditFormat::Ndjson)]
        format: AuditFormat,

        /// FR41 — time range filter. Supports relative ("30d", "7d", "24h", "1h")
        /// from now or absolute nanosecond timestamp.
        #[arg(long)]
        range: Option<String>,

        /// FR41 — filter by frame kind (e.g. "capability.invocation").
        #[arg(long)]
        frame_kind: Option<String>,

        /// FR41 — substring match on intent column (param-bound, SQLi-safe).
        #[arg(long)]
        intent_contains: Option<String>,

        /// FR41 — hex-encoded capability token for exact-match filter.
        #[arg(long)]
        capability: Option<String>,

        /// FR41 — specific boot_nonce to scope the query.
        #[arg(long)]
        boot: Option<u64>,

        /// FR41 — union all boot incarnations for the resolved spirit.
        #[arg(long)]
        all_boots: bool,

        /// RESERVED — errors with pointer to --intent-contains.
        #[arg(long)]
        tag: Option<String>,
    },
    /// FR44 — sealed export bundle.
    SealedExport {
        /// Filter by Spirit name.
        #[arg(long)]
        spirit: Option<String>,
        /// Time range filter (e.g. "30d", "7d", "1h").
        #[arg(long)]
        range: Option<String>,
        /// Output file path for the signed bundle JSON.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        /// Explicit path to audit signing key file.
        #[arg(long)]
        audit_key: Option<std::path::PathBuf>,
    },
    /// Generate an Ed25519 audit signing key.
    Keygen {
        /// Output file path for the signing key.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Verify a sealed-export bundle.
    VerifyBundle {
        /// Path to bundle JSON file.
        bundle: std::path::PathBuf,
        /// Hex-encoded Ed25519 public key, or path to a .hex file.
        #[arg(long)]
        pubkey: String,
    },
    /// FR42 — subject-access query: retrieve all principal_index rows for a
    /// given principal, enriched with provenance and spirit-name resolution.
    SubjectAccess {
        /// Principal ID to look up (e.g. "user:alice").
        #[arg(long)]
        principal: String,

        /// Output format.
        #[arg(long, value_enum, default_value_t = AuditFormat::Ndjson)]
        format: AuditFormat,
    },
    /// FR43 — posture-delta report: classify composed log entries into
    /// capability changes, sandbox tier changes, and consent ruptures.
    ///
    /// **v0.5 limitation**: the consent dimension surfaces `ConsentRupture`
    /// events only (FrameKind 22). Allowlist configuration changes to
    /// `ConsentAllowlists` are not journaled in the Transparency Log at
    /// v0.5 and are therefore not tracked in this report. This will be
    /// addressed in a future release when consent-policy mutation events
    /// are persisted.
    PostureDelta {
        /// Time range for the report (required). Supports relative notation
        /// like "30d", "7d", "24h", "1h".
        #[arg(long)]
        range: String,

        /// Filter to a specific Spirit.
        #[arg(long)]
        spirit: Option<String>,

        /// Output format.
        #[arg(long, value_enum, default_value_t = AuditFormat::Ndjson)]
        format: AuditFormat,
    },
    /// FR46 — trajectory export: signed trajectory bundle with redaction.
    ///
    /// v1.0 HARD byte-identity over the signed/shape surface.
    /// v1.5 extends to cross-platform/cross-toolchain/cross-schema-revision envelope.
    Export {
        /// Filter by Spirit name.
        #[arg(long)]
        spirit: Option<String>,
        /// Time range filter (e.g. "30d", "7d", "1h").
        #[arg(long)]
        range: Option<String>,
        /// Output file path for the signed trajectory JSON.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
        /// Explicit path to audit signing key file.
        #[arg(long)]
        audit_key: Option<std::path::PathBuf>,
        /// Redaction policy name. Default: "none" (no redaction).
        #[arg(long, default_value = "none")]
        redaction_policy: String,
    },
    /// ADR-028 — replay a sealed-export or trajectory bundle as a
    /// deterministic trace-shape document.
    ///
    /// v1.0 HARD byte-identity: two replays of the same bundle produce
    /// byte-identical output.  v1.5 extends to cross-platform /
    /// cross-toolchain-version / cross-schema-revision envelope.
    Replay {
        /// Path to the bundle JSON file (audit-bundle.v1 or trajectory.v1).
        bundle: std::path::PathBuf,
        /// Output file path for the trace-shape JSON.
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// FR64 — cost-reconcile observability report.
    ///
    /// Groups cost-attribution frames by (month × principal × spirit × provider × model)
    /// and computes cost read-time in integer micro-USD. Only `Resolved(single)`
    /// rows are attributed to a principal; `Ambiguous` + `Unattributed` → `host-unallocated`.
    CostReconcile {
        /// Month to reconcile (YYYY-MM format).
        #[arg(long)]
        month: String,

        /// Output format.
        #[arg(long, value_enum, default_value_t = AuditFormat::Ndjson)]
        format: AuditFormat,

        /// Path to the pricing config TOML file.
        #[arg(long, default_value = "xtask/provider-pricing.toml")]
        pricing: String,
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

/// Story 7.2 (FR60) — `maosctl import --offline` subcommand args.
#[derive(clap::Args, Debug)]
pub struct ImportArgs {
    /// Path to the `.tar` offline bundle produced by `maos-spirit publish
    /// --offline-bundle` (uncompressed tar; gzip/zstd is v0.7+).
    #[arg(long)]
    pub offline: std::path::PathBuf,

    /// Override the default registry URI. The admission decision targets
    /// local storage regardless; this only customizes where the import is
    /// recorded for downstream tooling.
    #[arg(long)]
    pub registry_uri: Option<String>,

    /// Force the effective trust tier. Requires
    /// `[registry].allow_force_tier_at_import = true` in operator.toml
    /// (default false).
    #[arg(long)]
    pub force_tier: Option<String>,

    /// Verify-only mode: print the would-be admission decision and exit.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
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
