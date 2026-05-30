//! `maos-spirit` — Spirit-author publish CLI per FR35 (Story 7.2 v1.0).
//!
//! At v1.0 ships the `publish` subcommand only. `validate` + `inspect`
//! are v0.7+ stubs that exit 1 with a deterministic "not yet implemented"
//! diagnostic.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use maos_spirit_cli::errors::CliError;
use maos_spirit_cli::publish::{run_publish, PublishArgs};

/// Spirit-author CLI for publishing, validating, and inspecting Spirit packages.
#[derive(Parser, Debug)]
#[command(name = "maos-spirit", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Publish a signed Spirit package to a registry.
    Publish(PublishArgsCli),
    /// Validate a Spirit package locally without publishing (v0.7+).
    Validate(StubArgs),
    /// Inspect a published Spirit's metadata (v0.7+).
    Inspect(StubArgs),
}

#[derive(Parser, Debug)]
struct PublishArgsCli {
    /// Trust tier: local | org_internal | public_untrusted (public_vetted deferred per FR37 v2.5).
    #[arg(long, value_parser = ["local", "org_internal", "public_untrusted"])]
    tier: String,

    /// Path to the Spirit manifest TOML.
    #[arg(long)]
    manifest: PathBuf,

    /// Path to the compiled Spirit artifact (binary blob).
    #[arg(long)]
    artifact: PathBuf,

    /// Path to the Ed25519 signing key (PEM-encoded or raw 32-byte hex).
    /// Precedence: --signing-key > --signing-key-env > ~/.config/maos/spirit-signing.key
    #[arg(long)]
    signing_key: Option<PathBuf>,

    /// Env var holding the Ed25519 signing key.
    #[arg(long)]
    signing_key_env: Option<String>,

    /// Registry URI override. Precedence: --registry-uri > $MAOS_REGISTRY_URI > built-in default.
    #[arg(long)]
    registry_uri: Option<String>,

    /// Path to a pre-baked ComplianceClaim envelope (CBOR). If absent, the
    /// CLI auto-populates structural fields from the manifest.
    #[arg(long)]
    compliance_claim: Option<PathBuf>,

    /// Print the would-be SignedPackage JSON without publishing.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

impl From<PublishArgsCli> for PublishArgs {
    fn from(c: PublishArgsCli) -> Self {
        PublishArgs {
            tier: c.tier,
            manifest: c.manifest,
            artifact: c.artifact,
            signing_key: c.signing_key,
            signing_key_env: c.signing_key_env,
            registry_uri: c.registry_uri,
            compliance_claim: c.compliance_claim,
            dry_run: c.dry_run,
        }
    }
}

#[derive(Parser, Debug)]
struct StubArgs {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let exit = match cli.command {
        Command::Publish(args) => match run_publish(PublishArgs::from(args)).await {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("maos-spirit: {e:#}");
                e.downcast_ref::<CliError>()
                    .map(CliError::exit_code)
                    .unwrap_or(1)
            }
        },
        Command::Validate(_) => {
            eprintln!("maos-spirit validate: not yet implemented (v0.7+)");
            1
        }
        Command::Inspect(_) => {
            eprintln!("maos-spirit inspect: not yet implemented (v0.7+)");
            1
        }
    };
    std::process::exit(exit);
}
