//! `maos-spirit` — Spirit-author CLI per FR35 (Story 7.2 v1.0): `publish`, `vet`, `yank`, and
//! (Story 22-7) `validate`, which reports what the Host would refuse before anything is published.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use maos_spirit_cli::errors::CliError;
use maos_spirit_cli::publish::{run_publish, PublishArgs};

/// Spirit-author CLI for validating, publishing, vetting and yanking Spirit packages.
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
    /// Issue or revoke exact-artifact vetting attestations.
    Vet(maos_spirit_cli::vet::VetArgs),
    /// Withdraw a release as its publisher or an enrolled vetter.
    Yank(maos_spirit_cli::yank::YankArgs),
    /// Check a package against the install gate, the boot gate and the component shape, without publishing.
    Validate(maos_spirit_cli::validate::ValidateArgs),
}

#[derive(Parser, Debug)]
struct PublishArgsCli {
    /// Trust tier: local | org_internal | public_untrusted | public_vetted.
    /// `public_vetted` is a declared aspiration (FR37/ADR-056) — inert until a
    /// vetting attestation promotes it at admission.
    #[arg(long, value_parser = ["local", "org_internal", "public_untrusted", "public_vetted"])]
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

    /// URI precedence: --registry-uri > $MAOS_REGISTRY_URI > [registry].uri > loopback default.
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
        Command::Vet(args) => report_result(maos_spirit_cli::vet::run(args)),
        Command::Yank(args) => report_result(maos_spirit_cli::yank::run(args)),
        Command::Validate(args) => maos_spirit_cli::validate::run(args),
    };
    std::process::exit(exit);
}

fn report_result(result: anyhow::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("maos-spirit: {error:#}");
            1
        }
    }
}
