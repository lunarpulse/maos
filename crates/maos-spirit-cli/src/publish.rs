//! `maos-spirit publish --tier=<tier>` subcommand flow per Story 7.2 AC2.
//!
//! Builds a Story 5.5d-compatible `SignedPackage` and dispatches
//! `registry.publish` via `SpiritRegistryClient`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use maos_domain::ports::registry::{
    PublishReceipt, RegistryError, SignedPackage, SpiritId, SpiritRegistryClient,
};
use maos_registry::admission;
use maos_spirit_abi::compliance::TrustTier;

use crate::compliance_claim;
use crate::errors::CliError;
use crate::signing;

/// Maximum file size for manifest files (10 MiB).
const MAX_MANIFEST_SIZE: u64 = 10_485_760;

/// Maximum file size for artifact files (100 MiB).
const MAX_ARTIFACT_SIZE: u64 = 104_857_600;

/// Arguments parsed by the `publish` subcommand. Matches the clap derive
/// definition in `bin/maos-spirit.rs`.
#[derive(Debug, Clone)]
pub struct PublishArgs {
    pub tier: String,
    pub manifest: PathBuf,
    pub artifact: PathBuf,
    pub signing_key: Option<PathBuf>,
    pub signing_key_env: Option<String>,
    pub registry_uri: Option<String>,
    pub compliance_claim: Option<PathBuf>,
    pub dry_run: bool,
}

/// Outcome of the publish flow.
///
/// Test code consumes this directly to assert receipt shape without
/// reaching for stdout-capture machinery; the binary entrypoint serializes
/// the appropriate variant to stdout.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PublishOutcome {
    Receipt(PublishReceipt),
    DryRun {
        signed_package_summary: SignedPackageSummary,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPackageSummary {
    pub spirit_id: String,
    pub version: String,
    pub manifest_size: usize,
    pub artifact_size: usize,
    pub signature_hex: String,
    pub publisher_pubkey_hex: String,
    pub envelope_signing_alg: String,
}

/// Run the publish flow against an injected `SpiritRegistryClient` impl. The
/// real binary path resolves this from `--registry-uri`; tests inject
/// `FixtureReplaySpiritRegistryClient` directly.
pub fn run_publish_with_client(
    args: &PublishArgs,
    client: &dyn SpiritRegistryClient,
) -> Result<PublishOutcome, CliError> {
    let pkg = build_signed_package(args)?;

    if args.dry_run {
        return Ok(PublishOutcome::DryRun {
            signed_package_summary: SignedPackageSummary {
                spirit_id: pkg.spirit_id.as_str().to_string(),
                version: pkg.version.clone(),
                manifest_size: pkg.manifest_toml.len(),
                artifact_size: pkg.artifact_bytes.len(),
                signature_hex: hex::encode(pkg.signature),
                publisher_pubkey_hex: hex::encode(pkg.publisher_pubkey),
                envelope_signing_alg: format!("{:?}", pkg.compliance_envelope.signing_alg),
            },
        });
    }

    match client.publish(&pkg) {
        Ok(receipt) => Ok(PublishOutcome::Receipt(receipt)),
        Err(e) => Err(map_registry_error(e)),
    }
}

/// Build a `SignedPackage` from `args`. Public so tests can inspect the
/// pre-dispatch shape without driving a fixture-replay client.
pub fn build_signed_package(args: &PublishArgs) -> Result<SignedPackage, CliError> {
    // 1. Load signing key + derive keypair.
    let seed = signing::load_signing_seed(&args.signing_key, &args.signing_key_env)?;
    let (publisher_pubkey, signing_pair) = signing::derive_keypair(&seed)?;

    // 2. Read manifest + artifact with size limits.
    let manifest_toml = read_limited_file(&args.manifest, MAX_MANIFEST_SIZE, "manifest")?;
    let artifact_bytes = read_limited_file(&args.artifact, MAX_ARTIFACT_SIZE, "artifact")?;

    // 3. Extract spirit_id + version from manifest.
    let (spirit_id_str, version) = signing::extract_spirit_id_and_version(&manifest_toml)?;
    let spirit_id = SpiritId(spirit_id_str.clone());

    // 4. Verify --tier matches manifest-declared trust_tier.
    let manifest_tier = admission::extract_manifest_tier(&manifest_toml)
        .map_err(|error| CliError::InvalidTier(error.to_string()))?;
    let arg_tier = parse_tier_arg(&args.tier)?;
    if manifest_tier != arg_tier {
        return Err(CliError::TierMismatch {
            cli_tier: args.tier.clone(),
            manifest_tier: tier_to_cli_string(manifest_tier),
        });
    }

    // 5. Compute Ed25519 signature over sha256(manifest_len_u64 || manifest || artifact_len_u64 || artifact).
    // Domain-separated to prevent collision attacks. Uses u64::to_le_bytes for cross-arch compatibility.
    let mut hasher = sha2::Sha256::new();
    hasher.update(&(manifest_toml.len() as u64).to_le_bytes());
    hasher.update(&manifest_toml);
    hasher.update(&(artifact_bytes.len() as u64).to_le_bytes());
    hasher.update(&artifact_bytes);
    let msg = hasher.finalize();
    let signature_bytes = signing_pair.sign(&msg);
    let signature = ed25519_sig_to_array(signature_bytes.as_ref())?;

    // 6. Load or auto-populate ComplianceClaim envelope.
    let compliance_envelope = match &args.compliance_claim {
        Some(p) => compliance_claim::load_envelope(p)?,
        None => compliance_claim::auto_populate(
            &manifest_toml,
            &version,
            &publisher_pubkey,
            &signing_pair,
        )?,
    };

    // 7. Build SignedPackage.
    Ok(SignedPackage::new(
        spirit_id,
        version,
        manifest_toml,
        artifact_bytes,
        signature,
        publisher_pubkey,
        compliance_envelope,
    ))
}

/// Read a file with a size limit to prevent OOM from malicious inputs.
fn read_limited_file(path: &PathBuf, max_size: u64, label: &str) -> Result<Vec<u8>, CliError> {
    let metadata = std::fs::metadata(path)
        .map_err(|e| CliError::Other(format!("read {label} {:?}: {e}", path)))?;
    let size = metadata.len();
    if size > max_size {
        return Err(CliError::Other(format!(
            "{label} file too large: {} bytes (max {})",
            size, max_size
        )));
    }
    std::fs::read(path).map_err(|e| CliError::Other(format!("read {label} {:?}: {e}", path)))
}

/// Convert TrustTier to the CLI string representation.
fn tier_to_cli_string(tier: TrustTier) -> String {
    match tier {
        TrustTier::Local => "local".to_string(),
        TrustTier::OrgInternal => "org_internal".to_string(),
        TrustTier::PublicUntrusted => "public_untrusted".to_string(),
        TrustTier::PublicVetted => "public_vetted".to_string(),
    }
}

/// Parse the `--tier` string into a `TrustTier`.
///
/// Story 13.4 (FR37 / ADR-056) un-defers `public_vetted`: an author MAY declare
/// the vetted aspiration at publish, but the declaration is inert — promotion is
/// the **attestation artifact** (never this flag). A `public_vetted` package is
/// admitted only when a valid `VettingAttestation` walks the verify chain at
/// admission; absent one it defers with `PublicVettedDeferred`.
pub fn parse_tier_arg(tier_str: &str) -> Result<TrustTier, CliError> {
    match tier_str {
        "local" => Ok(TrustTier::Local),
        "org_internal" => Ok(TrustTier::OrgInternal),
        "public_untrusted" => Ok(TrustTier::PublicUntrusted),
        "public_vetted" => Ok(TrustTier::PublicVetted),
        other => Err(CliError::InvalidTier(format!(
            "{other}: expected one of: local, org_internal, public_untrusted, public_vetted"
        ))),
    }
}

/// Safely convert an Ed25519 signature slice to a fixed-size array.
fn ed25519_sig_to_array(sig: &[u8]) -> Result<[u8; 64], CliError> {
    if sig.len() != 64 {
        return Err(CliError::SigningKeyDerive(format!(
            "expected 64-byte Ed25519 signature, got {} bytes",
            sig.len()
        )));
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(sig);
    Ok(arr)
}

/// Map `RegistryError` to a CLI error with the right exit-code semantics.
fn map_registry_error(e: RegistryError) -> CliError {
    match e {
        RegistryError::TrustTierFloorViolated { .. } => {
            CliError::TrustTierFloorViolated(e.to_string())
        }
        RegistryError::OrgSignatureInvalid => CliError::OrgSignatureInvalid(e.to_string()),
        RegistryError::Unconfigured => CliError::Unconfigured(e.to_string()),
        RegistryError::Transport(s) => CliError::Transport(s),
        other => CliError::Other(other.to_string()),
    }
}

/// Public entry point used by the binary: resolves the registry URI, builds
/// a client, and runs the publish flow.
///
/// At v1.0 only `stub` (fixture-replay; for CI) and explicit
/// non-stub URIs are supported. Non-stub URIs return `Unconfigured` if the
/// in-process MCP client construction fails — operators should rely on the
/// `maos-bin` composition root for production publish flows.
pub async fn run_publish(args: PublishArgs) -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();

    let uri = resolve_registry_uri(args.registry_uri.as_deref());

    if uri == "stub" {
        #[cfg(feature = "fixture_replay")]
        {
            let synthetic_receipt = serde_json::json!({
                "publish_id": "stub-publish-id",
                "spirit_id": "stub-spirit",
                "version": "stub-version",
            });
            let client = maos_registry::fixture_replay::FixtureReplaySpiritRegistryClient::new(
                vec![Ok(synthetic_receipt)],
            );
            let outcome =
                run_publish_with_client(&args, &client).map_err(|e| anyhow::anyhow!(e))?;
            println!("{}", serde_json::to_string_pretty(&outcome)?);
            return Ok(());
        }
        #[cfg(not(feature = "fixture_replay"))]
        {
            return Err(anyhow::anyhow!(
                "registry URI 'stub' requires the `fixture_replay` cargo feature: \
                 cargo run -p maos-spirit-cli --features fixture_replay -- publish ..."
            ));
        }
    }

    Err(anyhow::anyhow!(
        "maos-spirit publish for non-`stub` registry URIs requires the kernel-side \
         composition root; invoke through `maosctl publish` (Story 7.4+) or pipe a \
         SignedPackage via --dry-run for downstream tooling. Resolved URI: '{}'",
        uri
    ))
}

fn resolve_registry_uri(arg: Option<&str>) -> String {
    if let Some(s) = arg {
        let trimmed = s.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Ok(v) = std::env::var("MAOS_REGISTRY_URI") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "http://127.0.0.1:6789/mcp".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tier_accepts_public_vetted() {
        // Story 13.4 (ADR-056) — un-deferred; the aspiration is inert until an
        // attestation promotes it at admission.
        assert!(matches!(
            parse_tier_arg("public_vetted"),
            Ok(TrustTier::PublicVetted)
        ));
    }

    #[test]
    fn parse_tier_rejects_unknown() {
        let err = parse_tier_arg("bogus").unwrap_err();
        assert!(err.to_string().contains("bogus"));
    }

    #[test]
    fn parse_tier_accepts_three_valid_tiers() {
        assert!(matches!(parse_tier_arg("local"), Ok(TrustTier::Local)));
        assert!(matches!(
            parse_tier_arg("org_internal"),
            Ok(TrustTier::OrgInternal)
        ));
        assert!(matches!(
            parse_tier_arg("public_untrusted"),
            Ok(TrustTier::PublicUntrusted)
        ));
    }

    #[test]
    fn tier_to_cli_string_round_trips() {
        assert_eq!(tier_to_cli_string(TrustTier::Local), "local");
        assert_eq!(tier_to_cli_string(TrustTier::OrgInternal), "org_internal");
        assert_eq!(
            tier_to_cli_string(TrustTier::PublicUntrusted),
            "public_untrusted"
        );
        assert_eq!(tier_to_cli_string(TrustTier::PublicVetted), "public_vetted");
    }

    #[test]
    fn resolve_registry_uri_trims_whitespace() {
        assert_eq!(
            resolve_registry_uri(Some("  http://example.com  ")),
            "http://example.com"
        );
    }

    #[test]
    fn resolve_registry_uri_rejects_whitespace_only() {
        assert_eq!(
            resolve_registry_uri(Some("   ")),
            "http://127.0.0.1:6789/mcp"
        );
    }
}
