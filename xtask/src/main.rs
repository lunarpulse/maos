use clap::{Parser, Subcommand};
use std::path::Path;
use std::process;

mod abi_diff;
mod bypass_scan;
mod calibrate;
mod cassette_age_gate;
mod check_a2a_sender_completeness;
mod check_abi_ratification;
mod check_adr_040_accepted;
mod check_air_gap;
mod check_bare_review_findings;
mod gate_common;
// Story 10.4a — dependency-closure gate (kernel-core artifact hygiene)
mod check_dependency_closure;
mod check_rto_gate;
// Story 10.4a — RTO drill (performs cold-restore + timing, writes evidence for check-rto-gate).
mod check_rto;
// Story 10.4a — SQLite→Postgres migration triple-oracle ship gate (NFR-Ops-10).
mod check_breaking_md;
mod check_composition_root_completeness;
mod check_corpus;
mod check_coverage_matrix_completeness;
mod check_cross_form_equiv;
mod check_deprecations_declared;
mod check_dev_model_used_populated;
mod check_dev_record_completeness;
mod check_empty_kernel;
mod check_env_contract;
mod check_epic_6_bridge;
mod check_epic_close_green;
mod check_error_catalog;
mod check_fr47;
mod check_governance_categories;
mod check_judge_config;
mod check_kernel_baseline;
mod check_literal_reappearance;
mod check_loom;
mod check_manifest_schema_version;
mod check_migration_merkle;
pub mod check_mock_not_in_release;
mod check_multi_provider_drift;
mod check_pentest_gate;
mod check_pub_field_constructors;
mod check_red_team_gate;
mod check_review_findings_resolved;
mod check_security_md;
mod check_ship_gate_completeness;
mod check_third_party_trial;
// Story 10.3 — v1.0 compliance ship-gates (export-control, CNA, fuzz-targets).
mod check_cna_registration;
mod check_export_control;
mod check_fuzz_floor;
mod check_fuzz_targets;
// Story 10.4c AC5 (D8) — FF-J6 guard: J6 latency harness revival trigger.
mod check_ff_j6;
mod check_serde_error_handling;
mod check_service_boundary;
mod check_skill_schema;
// Story 10.5 AC1 (NFR-Test-10) — skill-format conformance gate.
mod check_skill_conformance;
mod check_unsafe;
mod check_workspace_count;
mod corpus_staleness;
mod corpus_types;
mod coverage_matrix;
mod coverage_matrix_nfr_test_3;
mod example_spirit_regen;
mod fs_walk;
mod gen_abi_docs;
mod gen_isolation_corpus;
mod gen_termination_corpus;
mod invariant_lock;
mod kloc_check;
mod nfr_onb_1_gate;
mod rebaseline_check;
mod release_verify;
mod stability_matrix;
mod templates_regen;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "MAOS workspace automation — CI gates, linting, checks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// AC2 — Zero-unsafe gate for the capability-validation path.
    CheckUnsafe {
        #[arg(long, default_value = "crates/maos-kernel-core/capability")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// AC3 — KLOC budget check with alarm and hard-fail thresholds.
    KlocCheck {
        #[arg(long, default_value = "xtask/kloc.toml")]
        config: String,
        #[arg(long)]
        json: bool,
    },
    /// AC4 — ABI diff against the previous tagged baseline.
    AbiDiff {
        #[arg(long, default_value = "HEAD~1")]
        base: String,
        #[arg(long)]
        json: bool,
    },
    /// AC6 — I9 structural-state lint for empty-kernel invariant.
    CheckEmptyKernel {
        #[arg(long, default_value = "crates/maos-kernel-core")]
        path: String,
        #[arg(long, default_value = "xtask/i9-whitelist.toml")]
        whitelist: String,
        #[arg(long, default_value = "xtask/i9-denylist.toml")]
        denylist: String,
        #[arg(long, default_value = "docs/invariants/i9-exemptions.md")]
        exemptions: String,
        #[arg(long)]
        json: bool,
    },
    /// AC7 — NFR-Test-9 Loom-not-in-kernel structural grep.
    CheckLoom {
        #[arg(long)]
        path: Option<String>,
        #[arg(long, default_value = "xtask/kernel-crates.toml")]
        crates: String,
        #[arg(long, default_value = "xtask/loom-blocklist.toml")]
        blocklist: String,
        #[arg(long, default_value = "xtask/loom-allowlist.toml")]
        allowlist: String,
        #[arg(long)]
        json: bool,
    },
    /// AC8 — NFR-Test-2 service-boundary P1–P4 + Spirit-ABI reflection.
    CheckServiceBoundary {
        #[arg(long)]
        path: Option<String>,
        #[arg(
            long,
            default_value = "docs/ci-baselines/kernel-surface-v0.1-beta.json"
        )]
        baseline: String,
        #[arg(long, default_value = "xtask/kernel-api-classes.toml")]
        classes: String,
        #[arg(long, default_value = "xtask/p4-external-io-denylist.toml")]
        p4_denylist: String,
        #[arg(long, default_value = "xtask/p4-mediated-io-paths.toml")]
        p4_exemptions: String,
        #[arg(long, default_value = "crates/maos-spirit-abi/src/lifecycle.rs")]
        spirit_abi_lifecycle: String,
        #[arg(long, default_value = "crates/maos-spirit-derive/src/lib.rs")]
        spirit_abi_derive: String,
        #[arg(long)]
        json: bool,
    },
    /// AC2 — FR47 enforcement: vendor LLM SDK dependency scan.
    CheckFr47 {
        #[arg(long)]
        path: Option<String>,
        #[arg(long, default_value = "xtask/fr47-vendor-sdk-denylist.toml")]
        denylist: String,
        #[arg(long, default_value = "xtask/fr47-allowlist.toml")]
        allowlist: String,
        #[arg(long)]
        json: bool,
    },
    /// AC9 — NFR-Ops-4 + FR61 SECURITY.md section gate.
    CheckSecurityMd {
        #[arg(long)]
        json: bool,
    },
    /// AC5 — Invariant lock gate for constitutional amendments.
    ///
    /// `--write-journal` opts the run into persisting a journal entry; without
    /// this flag the gate validates only. `--journal-output` sets the file the
    /// entry is written to (default: `docs/invariants/journal.jsonl`). For
    /// Option (c) journal persistence (DF16), the merge-queue workflow sets
    /// `--journal-output /tmp/journal-entry.jsonl` and uploads the file as a
    /// CI artifact; the in-repo journal is rebuilt offline by the operator.
    /// `--pr-body` points at a file containing the PR body for revert detection;
    /// if the body matches the GitHub revert idiom, a paired "reverted" entry
    /// is appended after the primary entry.
    InvariantLock {
        #[arg(long)]
        changed_files: Option<String>,
        #[arg(long)]
        pr_number: Option<u64>,
        #[arg(long)]
        sha: Option<String>,
        #[arg(long)]
        write_journal: bool,
        #[arg(long, default_value = "docs/invariants/journal.jsonl")]
        journal_output: String,
        #[arg(long)]
        pr_body: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// AC1 — SHA-256-pinned JSONL corpus verification (NFR-Test-1).
    CheckCorpus {
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "tests/corpora")]
        corpora_dir: String,
        #[arg(long)]
        register: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// AC2 — Pinned-judge-LLM structural contract (NFR-Test-1).
    CheckJudgeConfig {
        #[arg(long, default_value = "tests/judge-config.toml")]
        config: String,
        #[arg(long, default_value = "xtask/judge-direct-call-identifiers.toml")]
        identifiers: String,
        #[arg(long)]
        json: bool,
    },
    /// AC4 — Coverage-matrix delivered-phase enforcement (NFR-Meta-3).
    CoverageMatrix {
        #[arg(long, default_value = "tests/coverage-matrix.yaml")]
        config: String,
        #[arg(long, default_value = "tests/phase-config.toml")]
        phase_config: String,
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "xtask/gate-registry.toml")]
        gate_registry: String,
        #[arg(long)]
        json: bool,
        /// Story 7.1 — Measure NFR-Test-3 coverage floor.
        #[arg(long)]
        measure_nfr_test_3: bool,
        /// Filter to a single Spirit name (requires --measure-nfr-test-3).
        #[arg(long)]
        spirit: Option<String>,
        /// Report without writing back to YAML.
        #[arg(long)]
        dry_run: bool,
    },
    /// AC5 — Corpus staleness `valid_until` enforcement (NFR-Meta-2).
    CorpusStaleness {
        #[arg(long, default_value = "tests/coverage-matrix.yaml")]
        config: String,
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "30")]
        warn_window_days: i64,
        #[arg(long)]
        json: bool,
    },
    /// AC6 — Calibration Wilson-CI math (NFR-Aud-8).
    Calibrate {
        #[arg(long)]
        corpus: String,
        #[arg(long)]
        n: u64,
        #[arg(long)]
        p: f64,
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "tests/corpora")]
        corpora_dir: String,
        #[arg(long)]
        synthetic_pass_rate: Option<f64>,
        #[arg(long)]
        json: bool,
    },
    /// AC3 — Quarterly rebaseline check (NFR-Test-1).
    RebaselineCheck {
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "tests/corpora")]
        corpora_dir: String,
        #[arg(long, default_value = "tests/judge-config.toml")]
        judge_config: String,
        #[arg(long, default_value = "0.98")]
        threshold: f64,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Story 2.3 — Template-to-example drift detector and regenerator.
    /// DEPRECATED: use `templates-regen` instead.
    ExampleSpiritRegen {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1 — Generalized template-to-example regenerator (Rust + TS).
    TemplatesRegen {
        #[arg(long)]
        lang: Option<String>,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 8.8 AC3 — cross-Host A2A sender-completeness gate: no reference
    /// cross-Host sender builds a frame with an unclassified `consent_envelope`.
    CheckA2aSenderCompleteness {
        #[arg(long, default_value = ".")]
        workspace_root: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 2.5 AC3 — workspace-member-count guard (Cargo.toml vs architecture doc).
    CheckWorkspaceCount {
        #[arg(long, default_value = "Cargo.toml")]
        cargo_toml: String,
        #[arg(
            long,
            default_value = "_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md"
        )]
        kernel_design: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 8.16 §A4 — kernel-core line-count single source of truth. Counts
    /// `crates/maos-kernel-core/src` and compares to `xtask/kernel-core-baseline.toml`;
    /// hard-fails on drift (so a multi-story phase cannot drift the kernel unsummed).
    #[command(name = "check-kernel-baseline")]
    CheckKernelBaseline {
        #[arg(long)]
        json: bool,
    },
    /// Story 8.16 §A5 — epic-close green gate. Fails if ANY workflow job is
    /// disabled with a job-level `if: false` (the Epic-8 fake-green mode). Makes
    /// "mark an epic retrospective done while gates are parked red" impossible.
    #[command(name = "check-epic-close-green")]
    CheckEpicCloseGreen {
        #[arg(long)]
        json: bool,
    },
    /// Story 4.1 AC4 — deterministic 1000-scenario termination-corpus generator.
    GenTerminationCorpus {
        #[arg(
            long,
            default_value = "crates/maos-eval/fixtures/termination-corpus-v0"
        )]
        out_dir: String,
    },
    /// Story 4.5 — deterministic 200-scenario cross-spirit isolation-corpus generator.
    GenIsolationCorpus {
        #[arg(long, default_value = "crates/maos-eval/fixtures/isolation-corpus-v0")]
        out_dir: String,
    },
    /// Story 4.1 A2 — check release binary for forbidden test-double symbols.
    CheckMockNotInRelease {
        #[arg(long, default_value = "target/release/maos")]
        binary: String,
        #[arg(long)]
        build_first: bool,
        #[arg(long)]
        json: bool,
    },
    /// Epic 4 retro §A4 — pub fields with `Construct via ::new` doc-attr MUST have a matching `::new` impl.
    CheckPubFieldConstructors {
        #[arg(long)]
        json: bool,
    },
    /// Epic 4 retro §A5 — every `*Adapter` re-exported from `api.rs` MUST be constructed exactly once in `main.rs`.
    CheckCompositionRootCompleteness {
        #[arg(long, default_value = "crates/maos-kernel-core/src/api.rs")]
        api_rs: String,
        #[arg(long, default_value = "crates/maos-bin/src/main.rs")]
        main_rs: String,
        #[arg(long, default_value = "xtask/composition-root-whitelist.toml")]
        whitelist: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 5.5b — multi-provider drift check across aggregated reports.
    CheckMultiProviderDrift {
        #[arg(long)]
        report: String,
        #[arg(long, default_value = "10")]
        threshold: f64,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 5.5e — v0.5 release-block: ADR-040 must be committed with status `accepted`.
    CheckAdr040Accepted {
        #[arg(long)]
        json: bool,
    },
    /// Epic 5 retro §A3 (closes Epic 4 retro §A6) — flag `.unwrap_or_default()` / `.unwrap()` / `.expect(...)` after serde calls.
    CheckSerdeErrorHandling {
        #[arg(long, default_value = "crates")]
        path: String,
        #[arg(long, default_value = "xtask/serde-error-allowlist.toml")]
        allowlist: String,
        #[arg(long)]
        json: bool,
    },
    /// Epic 5 retro §A5 — open Review Findings rows MUST block sprint-status `done`; closed rows MUST reference a File List entry.
    CheckReviewFindingsResolved {
        #[arg(long, default_value = "_bmad-output/implementation-artifacts")]
        stories_dir: String,
        #[arg(
            long,
            default_value = "_bmad-output/implementation-artifacts/sprint-status.yaml"
        )]
        sprint_status: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 6.1 AC1 — Epic 6 bridge precondition gate (9 mechanical checks).
    /// Story 6.2 AC1 — extended with `--story 6.2` flag adding 6.2-specific rows
    /// (D-2.10 retract-corpus, D-4.* iac-routing-budget, D-3.7/3.8 DRR fairness,
    /// D-5.1/5.2 smoke-iac-bus-6, §A4-Debt-2c hook-count drift).
    /// Story 6.3 AC1 — extended with `--story 6.3` flag adding 10 6.3-specific
    /// row classifications (§A3/§A5/§A6 gate-exists, 6.2 smoke-arm, iac-routing
    /// budget, retract-corpus, DRR carry-forward, cli_wrapper bench carry-forward,
    /// §A2 backfill carry-forward, 6.2 RF count, smoke-iac-bus chain, maos-a2a
    /// baseline). When `--story 6.X` is set, blocking_6_X rows must clear before
    /// the command exits 0.
    /// Story 6.4 AC1 — extended with `--story 6.4` flag adding 10 6.4-specific
    /// row classifications (§A3/§A5/§A6 gate-exists, 6.3 smoke-arm, 6.3-P4 CI
    /// test-target verification, 6.3 RF count, DRR carry-forward, cli_wrapper
    /// bench carry-forward, §A2 backfill carry-forward, maos-providers baseline,
    /// FrameKind baseline, ScheduleWatchdog baseline).
    #[command(name = "check-epic-6-bridge")]
    CheckEpic6Bridge {
        #[arg(long)]
        json: bool,
        /// Story scope — "6.2" / "6.3" / "6.4" extends with the story-specific
        /// rows. Unset = Story 6.1 legacy 9-check semantics.
        #[arg(long)]
        story: Option<String>,
    },
    /// Epic 5 retro §A6 (closes Epic 4 retro §A7) — `done` stories MUST have non-TBD model + non-empty dev record + File List files in `git diff`.
    CheckDevRecordCompleteness {
        #[arg(long, default_value = "_bmad-output/implementation-artifacts")]
        stories_dir: String,
        #[arg(
            long,
            default_value = "_bmad-output/implementation-artifacts/sprint-status.yaml"
        )]
        sprint_status: String,
        #[arg(long, default_value_t = false)]
        check_git_diff: bool,
        #[arg(long)]
        json: bool,
    },
    /// Epic 6 §A4 (retro 2026-05-28) — keep MANIFEST_SCHEMA_VERSION + MIN/MAX
    /// constants self-consistent and ban hardcoded `manifest_schema_version`
    /// comparisons in `maos-manifest` production code.
    #[command(name = "check-manifest-schema-version")]
    CheckManifestSchemaVersion {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1 — assert ZERO deprecation annotations at v0.5 HEAD.
    #[command(name = "check-deprecations-declared")]
    CheckDeprecationsDeclared {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1.5 — assert ZERO bare `_No review findings._` placeholders remain.
    #[command(name = "check-bare-review-findings")]
    CheckBareReviewFindings {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.4 — assert the maos.skill.v1 schema posture: valid round-trip +
    /// deny_unknown_fields (UnknownField, not a silent default) + semver validation.
    #[command(name = "check-skill-schema")]
    CheckSkillSchema {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1.5 — assert every story file has a populated `dev_model_used:` frontmatter field.
    #[command(name = "check-dev-model-used-populated")]
    CheckDevModelUsedPopulated {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.5a — generate (or `--check`) repo-root STABILITY.md from live
    /// workspace state (ABI Stability Triple + LTS + compliance/export STUBs).
    #[command(name = "stability-matrix")]
    StabilityMatrix {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 7.5a — enforce BREAKING.md dated-entry taxonomy (NFR-Maint-7).
    #[command(name = "check-breaking-md")]
    CheckBreakingMd {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.4a — dependency-closure gate (kernel-core artifact excludes Postgres/pgvector).
    #[command(name = "check-dependency-closure")]
    CheckDependencyClosure {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.4a — RTO ≤ 4h gate (drilled, not printed); NFR-Ops-9.
    #[command(name = "check-rto-gate")]
    CheckRtoGate {
        #[arg(long, default_value = "xtask/rto-evidence.toml")]
        evidence: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 10.4a — RTO ≤ 4h drill: performs cold-restore + timing, gates on
    /// threshold, and writes evidence consumed by `check-rto-gate`.
    /// NFR-Ops-2 (RTO) + NFR-Ops-1 (RPO), drilled not printed.
    #[command(name = "rto-drill")]
    RtoDrill {
        /// Path to an existing source TL (omit to create a synthetic one).
        #[arg(long)]
        source: Option<String>,
        /// Path to an existing backup (omit to create one from source).
        #[arg(long)]
        backup: Option<String>,
        /// Number of synthetic frames (default 10000; ignored if --source).
        #[arg(long)]
        frames: Option<usize>,
        /// RTO threshold in seconds (default 14400 = 4h).
        #[arg(long)]
        rto_threshold_secs: Option<u64>,
        /// Write evidence to this TOML file (consumed by check-rto-gate).
        #[arg(long)]
        evidence_output: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Story 7.5b — NFR-Onb-1 30-Minute First Spirit Validation Gate discipline
    /// rail: stratification + cohort evaluator + Butler-corpus seam over the
    /// committed example cohort/outcomes/self-trial; FAILs loudly on drift.
    #[command(name = "nfr-onb-1-gate")]
    NfrOnb1Gate {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 8.15 — cassette age gate: fail if any cassette's recorded_at exceeds 14 days.
    CassetteAgeGate {
        #[arg(long, default_value = "crates/maos-journey-test/cassettes")]
        cassette_dir: String,
        /// Directory containing the Tier-2 success stamp file (default: same as cassette_dir)
        #[arg(long)]
        stamp_dir: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Story 8.15 — env-contract gate: every MAOS_* env::var read must be registered.
    CheckEnvContract {
        #[arg(long, default_value = "crates/maos-bin")]
        maos_bin_dir: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 9.6 AC-5 — smoke-arm literal reappearance guard.
    #[command(name = "check-literal-reappearance")]
    CheckLiteralReappearance {
        #[arg(long, default_value = "crates")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 9.3 FR63 — typed error catalog CI gate: bijection check between
    /// `xtask/error-catalog.toml` and AST-discovered E*-prefixed error variants.
    #[command(name = "error-catalog-check")]
    ErrorCatalogCheck {
        #[arg(long, default_value = "xtask/error-catalog.toml")]
        catalog: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 9.3 FR63 — generate deterministic machine-readable error catalog
    /// artifact (per-error retryability + cause-chain + version-stability).
    #[command(name = "error-catalog-generate")]
    ErrorCatalogGenerate {
        #[arg(long, default_value = "xtask/error-catalog.toml")]
        catalog: String,
        #[arg(long, default_value = "docs/errors/error-catalog.json")]
        output: String,
    },
    /// Story 9.3b FR62 — abi-diff ⊆ ratified reconciliation gate (ADR-045 §4 / R1).
    /// Asserts every abi-diff-detected ABI change is covered by a ratified
    /// AbiExtensionProposal in the manifest and backed by a TL-ancestor frame.
    #[command(name = "check-abi-ratification")]
    CheckAbiRatification {
        #[arg(long, default_value = "xtask/abi-ratifications.toml")]
        manifest: String,
        /// ABI baseline to diff against. If the file does not exist,
        /// the base case (no changes) is assumed — born green.
        #[arg(long, default_value = "xtask/abi-baseline/ratification-baseline.txt")]
        baseline: String,
        /// Transparency Log SQLite path. Required when ABI changes are
        /// detected, so the gate can verify each ratification frame is a
        /// strict TL-ancestor of the delta.
        #[arg(long, default_value = "/var/lib/maos/audit/transparency.sqlite")]
        transparency_log: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 9.3b FR62 — governance category completeness cross-check (R9).
    #[command(name = "check-governance-categories")]
    CheckGovernanceCategories {
        #[arg(long)]
        json: bool,
    },
    /// Story 9.4b AC-9 — region write/read bypass scanner (R-RG3 runtime companion).
    #[command(name = "bypass-scan")]
    BypassScan {
        #[arg(long)]
        json: bool,
    },
    /// Story 9.4 AC-1 — release artifact signing and verification gate.
    #[command(name = "release-verify")]
    ReleaseVerify {
        /// Sign mode: generate .sig from SHA256SUMS
        #[arg(long)]
        sign: bool,
        /// Verify mode: check .sig + SHA256 integrity
        #[arg(long)]
        verify: bool,
        /// SHA256SUMS file path
        #[arg(long)]
        sha256sums: Option<String>,
        /// Signature file path (for verify)
        #[arg(long)]
        sig: Option<String>,
        /// Output path for generated signature (sign mode)
        #[arg(long)]
        output: Option<String>,
        /// Env var name containing hex-encoded signing key (sign mode)
        #[arg(long)]
        key_env: Option<String>,
        /// File containing signing key (sign mode, alternative to --key-env)
        #[arg(long)]
        key_file: Option<String>,
        /// Directory containing release artifacts (verify mode)
        #[arg(long)]
        artifacts_dir: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Story 9.4 R-AG1 — air-gap no-network-symbols CI gate + dirty-fixture bite.
    #[command(name = "check-air-gap")]
    CheckAirGap {
        /// Path to the air-gap binary to scan.
        #[arg(long, default_value = "target/debug/maos")]
        binary: String,
        /// Path to the dirty-fixture binary (must be rejected by the gate).
        #[arg(long)]
        dirty_fixture: Option<String>,
        /// Build the air-gap binary first (cargo build --no-default-features --features air-gap).
        #[arg(long)]
        build_first: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 9.5c — generate ABI reference `.md` from maos-spirit-abi rustdoc JSON.
    #[command(name = "gen-abi-docs")]
    GenAbiDocs {
        /// Output directory for generated `.md` files.
        #[arg(long, default_value = "docs-site/abi/v1")]
        out_dir: String,
        /// Check mode: fail if committed docs differ from generated output.
        #[arg(long)]
        check: bool,
    },
    /// Story 10.1a AC4 — assert expected gate job names present in v1.0-ship-gate needs.
    #[command(name = "check-ship-gate-completeness")]
    CheckShipGateCompleteness {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.1b AC1 — pen-test gate: parse summary.toml, assert p0/p1 == 0,
    /// advisory-if-absent (conditional per calibrate-per-commit).
    #[command(name = "check-pentest-gate")]
    CheckPentestGate {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.1b AC3 — coverage-matrix completeness: no v1.0 NFR has empty gates.
    #[command(name = "check-coverage-matrix-completeness")]
    CheckCoverageMatrixCompleteness {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.2 AC1 — third-party trial N=12 gate: parse trial-results.toml,
    /// assert successes >= 10 + stratification + per-participant validation,
    /// Wilson CI advisory (conditional per calibrate-per-commit).
    #[command(name = "check-third-party-trial")]
    CheckThirdPartyTrial {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.2 AC2 — CLI-wrapper cross-form distributional equivalence gate
    /// (ADVISORY per ADR-040 rust-inproc deferral). Validates pre-committed
    /// Mann-Whitney U-test artifact.
    #[command(name = "check-cross-form-equiv")]
    CheckCrossFormEquiv {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.2 AC3 — adversarial red-team 80-scenario gate (v1.5 phase).
    /// Per-class floor ≥9/10, aggregate ≥72/80, 0 unmitigated categories.
    /// Advisory at v1.0 with "WOULD HAVE BLOCKED SHIP" banner on failure.
    #[command(name = "check-red-team-gate")]
    CheckRedTeamGate {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.3 AC-1 (NFR-Comp-1) — export-control classification gate:
    /// ECCN doc present + STABILITY.md §Export non-stub + crypto enumeration.
    #[command(name = "check-export-control")]
    CheckExportControl {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.3 AC-5 (NFR-Ops-4) — CNA registration gate: blocking-when-present.
    #[command(name = "check-cna-registration")]
    CheckCnaRegistration {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.3 AC-2/AC-3 (NFR-Sec-5/6) — fuzz-target existence gate (mechanics).
    #[command(name = "check-fuzz-targets")]
    CheckFuzzTargets {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.3 NFR-Sec-5/6 — fuzz CPU-hour floor gate (release-time).
    #[command(name = "check-fuzz-floor")]
    CheckFuzzFloor {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.4a AC2 (NFR-Ops-10) — SQLite→Postgres migration triple-oracle
    /// ship gate: Merkle root + payload oracle + row count consistency across
    /// source/target backends. Advisory at v1.0, blocking at v1.5.
    #[command(name = "check-migration-merkle")]
    CheckMigrationMerkle {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.4c AC5 (D8) — FF-J6 guard: enforces J6 cold-start latency
    /// harness revival trigger. Greps for J6 latency bindings; fails if one
    /// appears with no J6 harness present.
    #[command(name = "check-ff-j6")]
    CheckFfJ6 {
        #[arg(long)]
        json: bool,
    },
    /// Story 10.5 AC1 (NFR-Test-10) — skill-format conformance gate: validates
    /// that ≥1 third-party skill format executes via Spirit-form adapter without
    /// kernel modification. Parses real Anthropic fixture + proven-red invalid.
    #[command(name = "check-skill-conformance")]
    CheckSkillConformance {
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::CheckUnsafe { path, json } => check_unsafe::run(&path, json),
        Commands::CheckEmptyKernel {
            path,
            whitelist,
            denylist,
            exemptions,
            json,
        } => check_empty_kernel::run(&path, &whitelist, &denylist, &exemptions, json),
        Commands::CheckLoom {
            path,
            crates,
            blocklist,
            allowlist,
            json,
        } => check_loom::run(path.as_deref(), &crates, &blocklist, &allowlist, json),
        Commands::CheckFr47 {
            path,
            denylist,
            allowlist,
            json,
        } => check_fr47::run(path.as_deref(), &denylist, &allowlist, json),
        Commands::CheckServiceBoundary {
            path,
            baseline,
            classes,
            p4_denylist,
            p4_exemptions,
            spirit_abi_lifecycle,
            spirit_abi_derive,
            json,
        } => check_service_boundary::run(
            path.as_deref(),
            &baseline,
            &classes,
            &p4_denylist,
            &p4_exemptions,
            &spirit_abi_lifecycle,
            &spirit_abi_derive,
            json,
        ),
        Commands::CheckSecurityMd { json } => {
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            let report = check_security_md::check_security_md(&workspace_root);
            if json {
                let payload = serde_json::json!({
                    "passed": report.passed,
                    "present_sections": report.present_sections,
                    "missing_sections": report.missing_sections,
                });
                println!("{}", payload);
            } else if report.passed {
                eprintln!(
                    "check-security-md: PASS ({} sections found)",
                    report.present_sections.len()
                );
            } else {
                eprintln!(
                    "check-security-md: FAIL — missing sections: {:?}",
                    report.missing_sections
                );
            }
            if report.passed {
                Ok(())
            } else {
                Err("SECURITY.md missing required sections".into())
            }
        }
        Commands::KlocCheck { config, json } => kloc_check::run(&config, json),
        Commands::AbiDiff { base, json } => abi_diff::run(&base, json),
        Commands::InvariantLock {
            changed_files,
            pr_number,
            sha,
            write_journal,
            journal_output,
            pr_body,
            json,
        } => invariant_lock::run(
            changed_files.as_deref(),
            pr_number,
            sha.as_deref(),
            write_journal,
            &journal_output,
            pr_body.as_deref(),
            json,
        ),
        Commands::CheckCorpus {
            manifest,
            corpora_dir,
            register,
            json,
        } => check_corpus::run(&manifest, &corpora_dir, register.as_deref(), json),
        Commands::CheckJudgeConfig {
            config,
            identifiers,
            json,
        } => check_judge_config::run(&config, &identifiers, json),
        Commands::CoverageMatrix {
            config,
            phase_config,
            manifest,
            gate_registry,
            json,
            measure_nfr_test_3,
            spirit,
            dry_run,
        } => {
            if measure_nfr_test_3 {
                coverage_matrix_nfr_test_3::run(&config, spirit.as_deref(), dry_run, json)
            } else {
                coverage_matrix::run(&config, &phase_config, &manifest, &gate_registry, json)
            }
        }
        Commands::CorpusStaleness {
            config,
            manifest,
            warn_window_days,
            json,
        } => corpus_staleness::run(&config, &manifest, warn_window_days, json),
        Commands::Calibrate {
            corpus,
            n,
            p,
            manifest,
            corpora_dir,
            synthetic_pass_rate,
            json,
        } => calibrate::run(
            &corpus,
            n,
            p,
            &manifest,
            &corpora_dir,
            synthetic_pass_rate,
            json,
        ),
        Commands::RebaselineCheck {
            manifest,
            corpora_dir,
            judge_config,
            threshold,
            out,
            json,
        } => rebaseline_check::run(
            &manifest,
            &corpora_dir,
            &judge_config,
            threshold,
            out.as_deref(),
            json,
        ),
        Commands::ExampleSpiritRegen { check, json } => {
            eprintln!(
                "WARN: example-spirit-regen is deprecated; use templates-regen --lang rust instead"
            );
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            templates_regen::run(
                &workspace_root,
                Some(templates_regen::Language::Rust),
                check,
                json,
            )
        }
        Commands::TemplatesRegen { lang, check, json } => {
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            let lang = match lang.as_deref().map(|s| s.parse()).transpose() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            templates_regen::run(&workspace_root, lang, check, json)
        }
        Commands::CheckA2aSenderCompleteness {
            workspace_root,
            json,
        } => check_a2a_sender_completeness::run(&workspace_root, json),
        Commands::CheckWorkspaceCount {
            cargo_toml,
            kernel_design,
            json,
        } => check_workspace_count::run(&cargo_toml, &kernel_design, json),
        Commands::GenTerminationCorpus { out_dir } => gen_termination_corpus::run(&out_dir),
        Commands::GenIsolationCorpus { out_dir } => gen_isolation_corpus::run(&out_dir),
        Commands::CheckMockNotInRelease {
            binary,
            build_first,
            json,
        } => check_mock_not_in_release::run(&binary, build_first, json),
        Commands::CheckPubFieldConstructors { json } => check_pub_field_constructors::run(json),
        Commands::CheckCompositionRootCompleteness {
            api_rs,
            main_rs,
            whitelist,
            json,
        } => check_composition_root_completeness::run(&api_rs, &main_rs, &whitelist, json),
        Commands::CheckMultiProviderDrift {
            report,
            threshold,
            strict,
            json,
        } => {
            let exit_code =
                check_multi_provider_drift::run(Path::new(&report), threshold, strict, json);
            if exit_code != 0 {
                process::exit(exit_code);
            }
            Ok(())
        }
        Commands::CheckAdr040Accepted { json } => check_adr_040_accepted::run(json),
        Commands::CheckSerdeErrorHandling {
            path,
            allowlist,
            json,
        } => check_serde_error_handling::run(&path, &allowlist, json),
        Commands::CheckReviewFindingsResolved {
            stories_dir,
            sprint_status,
            json,
        } => check_review_findings_resolved::run(&stories_dir, &sprint_status, json),
        Commands::CheckEpic6Bridge { json, story } => {
            check_epic_6_bridge::run_with_story(json, story.as_deref())
        }
        Commands::CheckDevRecordCompleteness {
            stories_dir,
            sprint_status,
            check_git_diff,
            json,
        } => check_dev_record_completeness::run(&stories_dir, &sprint_status, check_git_diff, json),
        Commands::CheckManifestSchemaVersion { json } => check_manifest_schema_version::run(json),
        Commands::CheckDeprecationsDeclared { json } => check_deprecations_declared::run(json),
        Commands::CheckBareReviewFindings { json } => check_bare_review_findings::run(json),
        Commands::CheckSkillSchema { json } => check_skill_schema::run(json),
        Commands::CheckDevModelUsedPopulated { json } => check_dev_model_used_populated::run(json),
        Commands::CheckKernelBaseline { json } => check_kernel_baseline::run(json),
        Commands::CheckDependencyClosure { json } => check_dependency_closure::run(json),
        Commands::CheckRtoGate { evidence, json } => check_rto_gate::run(&evidence, json),
        Commands::RtoDrill {
            source,
            backup,
            frames,
            rto_threshold_secs,
            evidence_output,
            json,
        } => check_rto::run(
            source.as_deref(),
            backup.as_deref(),
            frames,
            rto_threshold_secs,
            evidence_output.as_deref(),
            json,
        ),
        Commands::CheckEpicCloseGreen { json } => check_epic_close_green::run(json),
        Commands::StabilityMatrix { check, json } => {
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            stability_matrix::run(&workspace_root, check, json)
        }
        Commands::CheckBreakingMd { json } => check_breaking_md::run(json),
        Commands::NfrOnb1Gate { check, json } => {
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            nfr_onb_1_gate::run(&workspace_root, check, json)
        }
        Commands::CassetteAgeGate {
            cassette_dir,
            stamp_dir,
            json,
        } => cassette_age_gate::run(&cassette_dir, json, stamp_dir.as_deref()),
        Commands::CheckEnvContract { maos_bin_dir, json } => {
            check_env_contract::run(&maos_bin_dir, json)
        }
        Commands::CheckLiteralReappearance { path, json } => {
            check_literal_reappearance::run(&path, json)
        }
        Commands::ErrorCatalogCheck { catalog, json } => check_error_catalog::run(&catalog, json),
        Commands::ErrorCatalogGenerate { catalog, output } => {
            check_error_catalog::run_generate(&catalog, &output)
        }
        Commands::CheckAbiRatification {
            manifest,
            baseline,
            transparency_log,
            json,
        } => check_abi_ratification::run(&manifest, &baseline, &transparency_log, json),
        Commands::CheckGovernanceCategories { json } => check_governance_categories::run(json),
        Commands::BypassScan { json } => bypass_scan::run(json),
        Commands::ReleaseVerify {
            sign,
            verify,
            sha256sums,
            sig,
            output,
            key_env,
            key_file,
            artifacts_dir,
            json,
        } => release_verify::run(
            sign,
            verify,
            sha256sums.as_deref(),
            sig.as_deref(),
            output.as_deref(),
            key_env.as_deref(),
            key_file.as_deref(),
            artifacts_dir.as_deref(),
            json,
        ),
        Commands::CheckAirGap {
            binary,
            build_first,
            dirty_fixture,
            json,
        } => check_air_gap::run(&binary, build_first, dirty_fixture.as_deref(), json),
        Commands::GenAbiDocs { out_dir, check } => gen_abi_docs::run(Some(&out_dir), check),
        Commands::CheckShipGateCompleteness { json } => check_ship_gate_completeness::run(json),
        Commands::CheckPentestGate { json } => check_pentest_gate::run(json),
        Commands::CheckCoverageMatrixCompleteness { json } => {
            check_coverage_matrix_completeness::run(json)
        }
        Commands::CheckThirdPartyTrial { json } => check_third_party_trial::run(json),
        Commands::CheckCrossFormEquiv { json } => check_cross_form_equiv::run(json),
        Commands::CheckRedTeamGate { json } => check_red_team_gate::run(json),
        Commands::CheckExportControl { json } => check_export_control::run(json),
        Commands::CheckCnaRegistration { json } => check_cna_registration::run(json),
        Commands::CheckFuzzTargets { json } => check_fuzz_targets::run(json),
        Commands::CheckFuzzFloor { json } => check_fuzz_floor::run(json),
        Commands::CheckMigrationMerkle { json } => check_migration_merkle::run(json),
        Commands::CheckFfJ6 { json } => check_ff_j6::run(json),
        Commands::CheckSkillConformance { json } => check_skill_conformance::run(json),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        process::exit(1);
    }
}
