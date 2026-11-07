//! Attestation issuance and operator-root-signed attestation revocation.
use clap::{Args, Subcommand};
use maos_compliance::{RevocationSemantics, VetterKeyring, VettingClaim};
use maos_domain::ports::registry::SignedPackage;
use maos_domain::revocation::{
    canonical_entries_bytes, CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct VetArgs {
    #[command(subcommand)]
    pub command: VetCommand,
}
#[derive(Debug, Subcommand)]
pub enum VetCommand {
    Issue(IssueArgs),
    Revoke(RevokeArgs),
}
#[derive(Debug, Args)]
pub struct IssueArgs {
    #[arg(long)]
    pub package: PathBuf,
    #[arg(long)]
    pub signing_key: PathBuf,
    #[arg(long)]
    pub vetter_key_id: String,
    #[arg(long)]
    pub expires_at_unix_ms: u64,
    #[arg(long)]
    pub output: PathBuf,
}
#[derive(Debug, Args)]
pub struct RevokeArgs {
    pub spec: String,
    /// Operator root key authorizing the keyring's attestation revocations.
    #[arg(long)]
    pub signing_key: PathBuf,
    #[arg(long)]
    pub vetter_keyring: PathBuf,
    #[arg(long)]
    pub reason: String,
}

pub fn run(args: VetArgs) -> anyhow::Result<()> {
    match args.command {
        VetCommand::Issue(args) => {
            let pkg: SignedPackage = serde_json::from_slice(&std::fs::read(&args.package)?)?;
            anyhow::ensure!(
                maos_registry::admission::verify_publisher_sig(&pkg),
                "package publisher signature invalid"
            );
            let seed = crate::signing::load_signing_seed(&Some(args.signing_key), &None)?;
            let now = maos_registry::yank_auth::now_unix_ms().map_err(anyhow::Error::msg)?;
            anyhow::ensure!(
                args.expires_at_unix_ms > now,
                "attestation expiry must follow issuance"
            );
            let claim = VettingClaim {
                manifest_hash: Sha256::digest(&pkg.manifest_toml).into(),
                artifact_sha256: Sha256::digest(&pkg.artifact_bytes).into(),
                publisher_pubkey: pkg.publisher_pubkey,
                spirit_id: pkg.spirit_id.as_str().into(),
                spirit_version: pkg.version,
                from_tier: maos_spirit_abi::compliance::TrustTier::PublicUntrusted,
                to_tier: maos_spirit_abi::compliance::TrustTier::PublicVetted,
                vetter_key_id: args.vetter_key_id,
                issued_at_unix_ms: now,
                expires_at_unix_ms: args.expires_at_unix_ms,
                revocation_semantics: RevocationSemantics::RefuseAtNextLoad,
                successor_policy: None,
            };
            let bytes = serde_cbor::to_vec(&maos_compliance::issue_attestation(&seed, &claim))?;
            write_atomic(&args.output, &bytes)?;
        }
        VetCommand::Revoke(args) => {
            let (name, version) = args
                .spec
                .rsplit_once('@')
                .ok_or_else(|| anyhow::anyhow!("expected name@version"))?;
            let seed = crate::signing::load_signing_seed(&Some(args.signing_key), &None)?;
            let (root, key) = crate::signing::derive_keypair(&seed)?;
            let mut keyring: VetterKeyring =
                serde_cbor::from_slice(&std::fs::read(&args.vetter_keyring)?)?;
            keyring.ensure_operator_root(&root)?;
            let entries = vec![RevocationEntry::new(
                name,
                format!("={version}"),
                args.reason,
                None,
            )?];
            let signature = key
                .sign(&canonical_entries_bytes(&entries)?)
                .as_ref()
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid Ed25519 signature length"))?;
            let crl = SignedRevocationList::new(
                CrlId::from_entries(&entries)?,
                1,
                maos_registry::yank_auth::now_unix_ms()
                    .map_err(anyhow::Error::msg)?
                    .saturating_mul(1_000_000),
                RevocationOrigin::Operator,
                entries,
                signature,
                root,
            )?;
            keyring.push_attestation_revocation(crl);
            write_atomic(&args.vetter_keyring, &serde_cbor::to_vec(&keyring)?)?;
        }
    }
    Ok(())
}

fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}
