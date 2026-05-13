use clap::{Parser, Subcommand};
use std::process;

mod check_unsafe;
mod check_empty_kernel;
mod check_loom;
mod check_service_boundary;
mod fs_walk;
mod kloc_check;
mod abi_diff;
mod invariant_lock;
mod corpus_types;
mod check_corpus;
mod check_judge_config;
mod coverage_matrix;
mod corpus_staleness;
mod calibrate;
mod rebaseline_check;

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
    CheckUnsafe { #[arg(long, default_value = "crates/maos-kernel-core/capability")] path: String, #[arg(long)] json: bool },
    /// AC3 — KLOC budget check with alarm and hard-fail thresholds.
    KlocCheck { #[arg(long, default_value = "xtask/kloc.toml")] config: String, #[arg(long)] json: bool },
    /// AC4 — ABI diff against the previous tagged baseline.
    AbiDiff { #[arg(long, default_value = "HEAD~1")] base: String, #[arg(long)] json: bool },
    /// AC6 — I9 structural-state lint for empty-kernel invariant.
    CheckEmptyKernel { #[arg(long, default_value = "crates/maos-kernel-core")] path: String, #[arg(long, default_value = "xtask/i9-whitelist.toml")] whitelist: String, #[arg(long, default_value = "xtask/i9-denylist.toml")] denylist: String, #[arg(long, default_value = "docs/invariants/i9-exemptions.md")] exemptions: String, #[arg(long)] json: bool },
    /// AC7 — NFR-Test-9 Loom-not-in-kernel structural grep.
    CheckLoom { #[arg(long)] path: Option<String>, #[arg(long, default_value = "xtask/kernel-crates.toml")] crates: String, #[arg(long, default_value = "xtask/loom-blocklist.toml")] blocklist: String, #[arg(long, default_value = "xtask/loom-allowlist.toml")] allowlist: String, #[arg(long)] json: bool },
    /// AC8 — NFR-Test-2 service-boundary surface-diff stub.
    CheckServiceBoundary { #[arg(long)] path: Option<String>, #[arg(long, default_value = "docs/ci-baselines/kernel-surface-v0.1-alpha.json")] baseline: String, #[arg(long, default_value = "xtask/kernel-api-classes.toml")] classes: String, #[arg(long)] json: bool },
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
        #[arg(long)] changed_files: Option<String>,
        #[arg(long)] pr_number: Option<u64>,
        #[arg(long)] sha: Option<String>,
        #[arg(long)] write_journal: bool,
        #[arg(long, default_value = "docs/invariants/journal.jsonl")] journal_output: String,
        #[arg(long)] pr_body: Option<String>,
        #[arg(long)] json: bool,
    },
    /// AC1 — SHA-256-pinned JSONL corpus verification (NFR-Test-1).
    CheckCorpus { #[arg(long, default_value = "tests/corpora/MANIFEST.toml")] manifest: String, #[arg(long, default_value = "tests/corpora")] corpora_dir: String, #[arg(long)] register: Option<String>, #[arg(long)] json: bool },
    /// AC2 — Pinned-judge-LLM structural contract (NFR-Test-1).
    CheckJudgeConfig { #[arg(long, default_value = "tests/judge-config.toml")] config: String, #[arg(long, default_value = "xtask/judge-direct-call-identifiers.toml")] identifiers: String, #[arg(long)] json: bool },
    /// AC4 — Coverage-matrix delivered-phase enforcement (NFR-Meta-3).
    CoverageMatrix { #[arg(long, default_value = "tests/coverage-matrix.yaml")] config: String, #[arg(long, default_value = "tests/phase-config.toml")] phase_config: String, #[arg(long, default_value = "tests/corpora/MANIFEST.toml")] manifest: String, #[arg(long, default_value = "xtask/gate-registry.toml")] gate_registry: String, #[arg(long)] json: bool },
    /// AC5 — Corpus staleness `valid_until` enforcement (NFR-Meta-2).
    CorpusStaleness { #[arg(long, default_value = "tests/coverage-matrix.yaml")] config: String, #[arg(long, default_value = "tests/corpora/MANIFEST.toml")] manifest: String, #[arg(long, default_value = "30")] warn_window_days: i64, #[arg(long)] json: bool },
    /// AC6 — Calibration Wilson-CI math (NFR-Aud-8).
    Calibrate { #[arg(long)] corpus: String, #[arg(long)] n: u64, #[arg(long)] p: f64, #[arg(long, default_value = "tests/corpora/MANIFEST.toml")] manifest: String, #[arg(long, default_value = "tests/corpora")] corpora_dir: String, #[arg(long)] synthetic_pass_rate: Option<f64>, #[arg(long)] json: bool },
    /// AC3 — Quarterly rebaseline check (NFR-Test-1).
    RebaselineCheck { #[arg(long, default_value = "tests/corpora/MANIFEST.toml")] manifest: String, #[arg(long, default_value = "tests/corpora")] corpora_dir: String, #[arg(long, default_value = "tests/judge-config.toml")] judge_config: String, #[arg(long, default_value = "0.98")] threshold: f64, #[arg(long)] out: Option<String>, #[arg(long)] json: bool },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::CheckUnsafe { path, json } => check_unsafe::run(&path, json),
        Commands::CheckEmptyKernel { path, whitelist, denylist, exemptions, json } => check_empty_kernel::run(&path, &whitelist, &denylist, &exemptions, json),
        Commands::CheckLoom { path, crates, blocklist, allowlist, json } => check_loom::run(path.as_deref(), &crates, &blocklist, &allowlist, json),
        Commands::CheckServiceBoundary { path, baseline, classes, json } => check_service_boundary::run(path.as_deref(), &baseline, &classes, json),
        Commands::KlocCheck { config, json } => kloc_check::run(&config, json),
        Commands::AbiDiff { base, json } => abi_diff::run(&base, json),
        Commands::InvariantLock { changed_files, pr_number, sha, write_journal, journal_output, pr_body, json } => invariant_lock::run(changed_files.as_deref(), pr_number, sha.as_deref(), write_journal, &journal_output, pr_body.as_deref(), json),
        Commands::CheckCorpus { manifest, corpora_dir, register, json } => check_corpus::run(&manifest, &corpora_dir, register.as_deref(), json),
        Commands::CheckJudgeConfig { config, identifiers, json } => check_judge_config::run(&config, &identifiers, json),
        Commands::CoverageMatrix { config, phase_config, manifest, gate_registry, json } => coverage_matrix::run(&config, &phase_config, &manifest, &gate_registry, json),
        Commands::CorpusStaleness { config, manifest, warn_window_days, json } => corpus_staleness::run(&config, &manifest, warn_window_days, json),
        Commands::Calibrate { corpus, n, p, manifest, corpora_dir, synthetic_pass_rate, json } => calibrate::run(&corpus, n, p, &manifest, &corpora_dir, synthetic_pass_rate, json),
        Commands::RebaselineCheck { manifest, corpora_dir, judge_config, threshold, out, json } => rebaseline_check::run(&manifest, &corpora_dir, &judge_config, threshold, out.as_deref(), json),
    };
    if let Err(e) = result { eprintln!("{e}"); process::exit(1); }
}
