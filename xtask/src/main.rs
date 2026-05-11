use clap::{Parser, Subcommand};
use std::process;

mod check_unsafe;
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
