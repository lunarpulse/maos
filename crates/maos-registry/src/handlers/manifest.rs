//! Handler for `registry.manifest`.

use std::sync::Arc;

use crate::admission::extract_manifest_tier;
use crate::operations::ManifestArgs;
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::{SignedManifest, SpiritId, TrustTier};

/// Compute the server tier signature message using domain-separated hashing.
/// Format: sha256(spirit_id_len_u64 || spirit_id || version_len_u64 || version || tier_byte)
fn server_tier_signature_msg(spirit_id: &str, version: &str, server_tier: TrustTier) -> Vec<u8> {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&(spirit_id.len() as u64).to_le_bytes());
    hasher.update(spirit_id.as_bytes());
    hasher.update(&(version.len() as u64).to_le_bytes());
    hasher.update(version.as_bytes());
    hasher.update(&[server_tier as u8]);
    hasher.finalize().to_vec()
}

pub fn handle_manifest(
    storage: &Arc<dyn RegistryStorage>,
    args: &ManifestArgs,
    server_signing_key: Option<&[u8; 32]>,
) -> Result<serde_json::Value, String> {
    let spirit_id = SpiritId::from(args.spirit_id.as_str());
    let manifest = storage
        .get_manifest(&spirit_id, &args.version)
        .map_err(|e| e.to_string())?;

    // Story 7.2 — populate server-reported tier (Finding D2).
    let server_tier = extract_manifest_tier(&manifest.manifest_toml);
    let manifest = if let Some(key) = server_signing_key {
        let msg = server_tier_signature_msg(spirit_id.as_str(), &args.version, server_tier);
        let keypair = ring::signature::Ed25519KeyPair::from_seed_unchecked(key)
            .map_err(|e| format!("invalid server signing key: {e}"))?;
        let sig_bytes = keypair.sign(&msg);
        let mut sig = [0u8; 64];
        sig.copy_from_slice(sig_bytes.as_ref());
        manifest.with_server_tier(server_tier, sig)
    } else {
        manifest.with_server_tier(server_tier, [0u8; 64])
    };

    serde_json::to_value(manifest).map_err(|e| format!("serialize error: {e}"))
}
