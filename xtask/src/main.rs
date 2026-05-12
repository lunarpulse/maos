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
        /// Path to the capability subtree (default: crates/maos-kernel-core/capability).
        #[arg(long, default_value = "crates/maos-kernel-core/capability")]
        path: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// AC3 — KLOC budget check with alarm and hard-fail thresholds.
    KlocCheck {
        /// Path to kloc.toml budget file.
        #[arg(long, default_value = "xtask/kloc.toml")]
        config: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// AC4 — ABI diff against the previous tagged baseline.
    AbiDiff {
        /// Git base ref for diff (default: HEAD~1).
        #[arg(long, default_value = "HEAD~1")]
        base: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// AC6 — I9 structural-state lint for empty-kernel invariant.
    CheckEmptyKernel {
        /// Path to the kernel-core subtree (default: crates/maos-kernel-core).
        #[arg(long, default_value = "crates/maos-kernel-core")]
        path: String,
        /// Path to the I9 whitelist file.
        #[arg(long, default_value = "xtask/i9-whitelist.toml")]
        whitelist: String,
        /// Path to the I9 denylist file.
        #[arg(long, default_value = "xtask/i9-denylist.toml")]
        denylist: String,
        /// Path to the I9 exemptions documentation file.
        #[arg(long, default_value = "docs/invariants/i9-exemptions.md")]
        exemptions: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// AC7 — NFR-Test-9 Loom-not-in-kernel structural grep.
    CheckLoom {
        /// Direct path to scan (overrides --crates; used for integration tests).
        #[arg(long)]
        path: Option<String>,
        /// Path to the kernel-crates list file.
        #[arg(long, default_value = "xtask/kernel-crates.toml")]
        crates: String,
        /// Path to the Loom blocklist file.
        #[arg(long, default_value = "xtask/loom-blocklist.toml")]
        blocklist: String,
        /// Path to the Loom allowlist file.
        #[arg(long, default_value = "xtask/loom-allowlist.toml")]
        allowlist: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// AC8 — NFR-Test-2 service-boundary surface-diff stub.
    CheckServiceBoundary {
        /// Direct path to the crate to scan (overrides default; used for integration tests).
        #[arg(long)]
        path: Option<String>,
        /// Path to the kernel surface baseline JSON file.
        #[arg(long, default_value = "docs/ci-baselines/kernel-surface-v0.1-alpha.json")]
        baseline: String,
        /// Path to the kernel API classes TOML file.
        #[arg(long, default_value = "xtask/kernel-api-classes.toml")]
        classes: String,
        /// Emit JSON instead of human-readable output.
        #[arg(long)]
        json: bool,
    },
    /// AC5 — Invariant lock gate for constitutional amendments.
    InvariantLock {
        /// Path to file listing changed files (one per line).
        #[arg(long)]
        changed_files: Option<String>,
        /// PR number for journal entry.
        #[arg(long)]
        pr_number: Option<u64>,
        /// Current git SHA.
        #[arg(long)]
        sha: Option<String>,
        /// Emit JSON instead of human-readable output.
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
        Commands::CheckServiceBoundary {
            path,
            baseline,
            classes,
            json,
        } => check_service_boundary::run(path.as_deref(), &baseline, &classes, json),
        Commands::KlocCheck { config, json } => kloc_check::run(&config, json),
        Commands::AbiDiff { base, json } => abi_diff::run(&base, json),
        Commands::InvariantLock {
            changed_files,
            pr_number,
            sha,
            json,
        } => invariant_lock::run(changed_files.as_deref(), pr_number, sha.as_deref(), json),
    };

    if let Err(e) = result {
        eprintln!("{e}");
        process::exit(1);
    }
}
