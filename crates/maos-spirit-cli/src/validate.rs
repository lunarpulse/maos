//! `maos-spirit validate --manifest <m> --artifact <a> [--json]` (Story 22-7): what would the Host refuse?
//! The package is signed in memory with a per-call ephemeral key (no registry, daemon or key file; nothing is
//! written); the checks are `maos_registry::validate`. Exit 0 admissible, 1 refused, 2 unreadable input or policy.

use std::path::PathBuf;

use clap::Parser;
use maos_domain::ports::registry::{SignedPackage, SpiritId};
use maos_registry::client::RegistryClientConfig;
use maos_registry::validate::ValidationReport;
use ring::rand::{SecureRandom, SystemRandom};

use crate::errors::CliError;
use crate::{publish, signing};

/// Arguments of the `validate` subcommand.
#[derive(Parser, Debug, Clone)]
pub struct ValidateArgs {
    /// Path to the Spirit manifest TOML.
    #[arg(long)]
    manifest: PathBuf,
    /// Path to the built component (`dist/spirit.wasm`).
    #[arg(long)]
    artifact: PathBuf,
    /// Print the report as JSON.
    #[arg(long, default_value_t = false)]
    json: bool,
}

/// The in-memory package `validate` checks, signed with a key generated for this call only.
pub fn ephemeral_package(manifest: Vec<u8>, artifact: Vec<u8>) -> Result<SignedPackage, CliError> {
    let mut seed = [0u8; 32];
    SystemRandom::new()
        .fill(&mut seed)
        .map_err(|_| CliError::SigningKeyDerive("system RNG unavailable".into()))?;
    let (pubkey, pair) = signing::derive_keypair(&seed)?;
    // A manifest with no readable identity still gets a package: the checks report why.
    let (id, version) = signing::extract_spirit_id_and_version(&manifest).unwrap_or_default();
    publish::sign_package(
        &pair,
        pubkey,
        manifest,
        artifact,
        SpiritId(id),
        version,
        None,
    )
}

/// Signs the in-memory package and runs every check.
pub fn validate_bytes(
    manifest: Vec<u8>,
    artifact: Vec<u8>,
    cfg: Option<&RegistryClientConfig>,
) -> Result<ValidationReport, CliError> {
    let pkg = ephemeral_package(manifest, artifact)?;
    Ok(maos_registry::validate::validate(&pkg, cfg))
}

fn render(report: &ValidationReport) -> String {
    let show = |value: Option<&str>| value.unwrap_or("-").to_owned();
    let mut out = format!(
        "spirit: {} {}\nform: {}\neffective tier: {}\nsandbox tier floor: {}\n",
        report.spirit_id,
        report.version,
        report.form,
        show(report.effective_tier),
        show(report.sandbox_tier_floor),
    );
    out.push_str(if report.admissible() {
        "result: admissible (install gate, boot gate and component shape)\n"
    } else {
        "result: refused\n"
    });
    for finding in &report.findings {
        out += &format!(
            "  [{}] {}: {}\n",
            finding.check, finding.code, finding.detail
        );
    }
    out
}

fn try_run(args: &ValidateArgs) -> Result<i32, String> {
    let (manifest, artifact) =
        publish::read_inputs(&args.manifest, &args.artifact).map_err(|e| e.to_string())?;
    let cfg =
        maos_registry::install::operator_config().map_err(|e| format!("operator policy: {e}"))?;
    let report = validate_bytes(manifest, artifact, Some(&cfg)).map_err(|e| e.to_string())?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
        );
    } else {
        print!("{}", render(&report));
    }
    Ok(i32::from(!report.admissible()))
}

/// Runs the verb and returns its exit code.
pub fn run(args: ValidateArgs) -> i32 {
    try_run(&args).unwrap_or_else(|error| {
        eprintln!("maos-spirit validate: {error}");
        2
    })
}
