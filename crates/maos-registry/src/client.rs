//! Kernel-side MCP Spirit Registry client.
//!
//! Routes through Story 5.5c's `McpClient::call` to invoke registry
//! operations.  Registry calls are admission-time infrastructure —
//! there is no Spirit holding a capability token when the operator
//! installs a new Spirit, so registry calls route DIRECTLY through
//! `McpClient::call` rather than through the kernel-side
//! `McpClientAdapter` (which requires a `CapabilityToken`).

use std::sync::Arc;

use maos_domain::ports::mcp::McpError;
use maos_domain::ports::registry::{
    PublishReceipt, RegistryError, SearchQuery, SearchResults, SignedArtifact, SignedManifest,
    SignedPackage, SpiritId, SpiritRegistryClient, TrustTier, YankList, YankReason,
    YankReceipt,
};
use maos_mcp::McpClient;

/// Operator-configurable consumer-side verification policy.
#[derive(Debug, Clone)]
pub struct RegistryClientConfig {
    pub tier_floor: TrustTier,
    pub require_server_tier_signature: bool,
    pub org_signing_pubkey: Option<[u8; 32]>,
}

impl Default for RegistryClientConfig {
    fn default() -> Self {
        Self {
            tier_floor: TrustTier::Local,
            require_server_tier_signature: false,
            org_signing_pubkey: None,
        }
    }
}

/// Kernel-side registry client routing through `McpClient::call`.
pub struct McpSpiritRegistryClient {
    mcp_client: Arc<dyn McpClient + Send + Sync>,
    registry_server_name: String,
    config: RegistryClientConfig,
}

impl McpSpiritRegistryClient {
    /// Construct a new kernel-side registry client.
    pub fn new(mcp_client: Arc<dyn McpClient + Send + Sync>, registry_server_name: String) -> Self {
        Self {
            mcp_client,
            registry_server_name,
            config: RegistryClientConfig::default(),
        }
    }

    /// Story 7.2 — attach operator policy for consumer-side tier verification.
    pub fn with_config(mut self, config: RegistryClientConfig) -> Self {
        self.config = config;
        self
    }

    /// Fetch yanks since a given monotonic timestamp.
    /// Not part of the `SpiritRegistryClient` trait — used internally by YankPoller.
    pub fn yanks_since(&self,
        since_ns: u64,
    ) -> Result<YankList, RegistryError> {
        let args = serde_json::json!({"since_ns": since_ns});
        let resp = self
            .mcp_client
            .call(
                &self.registry_server_name,
                "registry.yanks_since",
                args,
            )
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
    }
}

impl SpiritRegistryClient for McpSpiritRegistryClient {
    fn search(&self, q: &SearchQuery) -> Result<SearchResults, RegistryError> {
        let args = serde_json::to_value(q).map_err(|e| RegistryError::Transport(e.to_string()))?;
        let resp = self
            .mcp_client
            .call(&self.registry_server_name, "registry.search", args)
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
    }

    fn manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, RegistryError> {
        let args = serde_json::json!({
            "spirit_id": spirit_id.as_str(),
            "version": version,
        });
        let resp = self
            .mcp_client
            .call(
                &self.registry_server_name,
                "registry.manifest",
                args,
            )
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        let manifest: SignedManifest = serde_json::from_value(resp.content)
            .map_err(|e| RegistryError::Transport(e.to_string()))?;

        // Story 7.2 — consumer-side tier verification.
        // If the server provides tier data, verify it regardless of require flag.
        // If require flag is set but data is missing, fail.
        match (
            manifest.server_reported_tier, manifest.server_signature_on_tier) {
            (Some(server_tier), Some(server_sig)) => {
                // Server provided tier data — verify signature and cross-check manifest.
                let manifest_tier = crate::admission::extract_manifest_tier(&manifest.manifest_toml);
                if manifest_tier != server_tier {
                    return Err(RegistryError::TrustTierServerMismatch {
                        manifest_tier,
                        server_reported_tier: server_tier,
                    });
                }
                
                if let Some(pubkey) = self.config.org_signing_pubkey {
                    let msg = server_tier_signature_msg(spirit_id.as_str(), version, server_tier);
                    let pk = ring::signature::UnparsedPublicKey::new(
                        &ring::signature::ED25519,
                        &pubkey[..],
                    );
                    pk.verify(&msg, &server_sig[..])
                        .map_err(|_| RegistryError::ServerTierSignatureInvalid)?;
                } else if self.config.require_server_tier_signature {
                    // Signature required but no pubkey configured — cannot verify.
                    return Err(RegistryError::ServerTierSignatureRequired {
                        reason: "require_server_tier_signature=true but org_signing_pubkey is not configured".into(),
                    });
                }
                
                // Apply tier floor check.
                if (server_tier as u8) > (self.config.tier_floor as u8) {
                    return Err(RegistryError::TierFloorViolation {
                        server_reported: server_tier,
                        operator_floor: self.config.tier_floor,
                    });
                }
            }
            (server_tier, server_sig) => {
                // Server did not provide complete tier data.
                if self.config.require_server_tier_signature {
                    let missing = if server_tier.is_none() {
                        "server_reported_tier"
                    } else {
                        "server_signature_on_tier"
                    };
                    return Err(RegistryError::ServerTierSignatureRequired {
                        reason: format!("manifest missing {missing}"),
                    });
                }
                // Migration window: warn but proceed.
                #[cfg(feature = "tracing")]
                tracing::warn!(
                    "spirit registry did not provide server_reported_tier/signature; \
                     consumer-side tier cross-verification skipped (v0.5→v1.0 migration)"
                );
            }
        }

        Ok(manifest)
    }

    fn artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, RegistryError> {
        let args = serde_json::json!({
            "spirit_id": spirit_id.as_str(),
            "version": version,
        });
        let resp = self
            .mcp_client
            .call(
                &self.registry_server_name,
                "registry.artifact",
                args,
            )
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
    }

    fn publish(
        &self,
        pkg: &SignedPackage,
    ) -> Result<PublishReceipt, RegistryError> {
        let args = serde_json::to_value(pkg).map_err(|e| RegistryError::Transport(e.to_string()))?;
        let resp = self
            .mcp_client
            .call(
                &self.registry_server_name,
                "registry.publish",
                args,
            )
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
    }

    fn deprecate(
        &self,
        _spirit_id: &SpiritId,
        _version: &str,
        _reason: &YankReason,
    ) -> Result<YankReceipt, RegistryError> {
        Err(RegistryError::Transport(
            "deprecate not implemented on MCP client".into(),
        ))
    }
}

/// Compute the server tier signature message using domain-separated hashing.
/// Matches the server-side implementation in `handlers/manifest.rs`.
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

fn map_mcp_err(e: McpError) -> RegistryError {
    RegistryError::Transport(e.to_string())
}

fn decode_typed_error(content: &serde_json::Value) -> RegistryError {
    if let Some(code) = content.get("error_code").and_then(|v| v.as_str()) {
        match code {
            "TrustTierFloorViolated" => {
                if let Some(msg) = content.get("message").and_then(|v| v.as_str()) {
                    RegistryError::Transport(format!("trust tier floor violated: {msg}"))
                } else {
                    RegistryError::Transport("trust tier floor violated".into())
                }
            }
            "SignatureInvalid" => RegistryError::SignatureInvalid,
            "OrgSignatureInvalid" => RegistryError::OrgSignatureInvalid,
            "ComplianceContextDrift" => {
                if let Some(msg) = content.get("message").and_then(|v| v.as_str()) {
                    RegistryError::Transport(format!("compliance context drift: {msg}"))
                } else {
                    RegistryError::Transport("compliance context drift".into())
                }
            }
            "PublicVettedDeferred" => RegistryError::PublicVettedDeferred,
            _ => RegistryError::Transport(format!("registry error: {code}")),
        }
    } else {
        RegistryError::Transport(format!("registry error: {content}"))
    }
}
