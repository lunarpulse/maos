//! Kernel-side MCP Spirit Registry client.
//!
//! Routes through Story 5.5c's `McpClient::call` to invoke registry
//! operations.  Registry calls are admission-time infrastructure —
//! there is no Spirit holding a capability token when the operator
//! installs a new Spirit, so registry calls route DIRECTLY through
//! `McpClient::call` rather than through the kernel-side
//! `McpClientAdapter` (which requires a `CapabilityToken`).

use std::sync::Arc;

use maos_domain::ports::registry::{
    PublishReceipt, RegistryError, SearchQuery, SearchResults, SignedArtifact, SignedManifest,
    SignedPackage, SpiritId, SpiritRegistryClient, TrustTier, YankList, YankReason,
    YankReceipt,
};
use maos_mcp::client::McpClient;

/// Kernel-side registry client routing through `McpClient::call`.
pub struct McpSpiritRegistryClient {
    // TODO: switch to `Arc<dyn McpClient>` once a trait abstraction is extracted;
    // McpClient is currently a concrete struct, so Arc<McpClient> is fine.
    mcp_client: Arc<McpClient>,
    registry_server_name: String,
}

impl McpSpiritRegistryClient {
    /// Construct a new kernel-side registry client.
    pub fn new(mcp_client: Arc<McpClient>, registry_server_name: String) -> Self {
        Self {
            mcp_client,
            registry_server_name,
        }
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
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
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

    fn publish(&self, pkg: &SignedPackage) -> Result<PublishReceipt, RegistryError> {
        let args = serde_json::json!({
            "spirit_id": pkg.spirit_id.as_str(),
            "version": pkg.version,
            "manifest_toml": pkg.manifest_toml,
            "artifact_bytes": pkg.artifact_bytes,
            "signature": hex::encode(&pkg.signature[..]),
            "publisher_pubkey": hex::encode(&pkg.publisher_pubkey[..]),
            "compliance_envelope": pkg.compliance_envelope,
        });
        let resp = self
            .mcp_client
            .call(&self.registry_server_name, "registry.publish", args)
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
    }

    fn deprecate(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, RegistryError> {
        let args = serde_json::json!({
            "spirit_id": spirit_id.as_str(),
            "version": version,
            "reason": reason.summary,
        });
        let resp = self
            .mcp_client
            .call(
                &self.registry_server_name,
                "registry.deprecate",
                args,
            )
            .map_err(map_mcp_err)?;
        if resp.is_error {
            return Err(decode_typed_error(&resp.content));
        }
        serde_json::from_value(resp.content).map_err(|e| RegistryError::Transport(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers for the kernel-side client
// ---------------------------------------------------------------------------

/// Internal method called by `YankPoller` (in `yank.rs`).
///
/// This is the kernel-internal `registry.yanks_since` MCP op —
/// NOT part of the public `SpiritRegistryClient` trait.
impl McpSpiritRegistryClient {
    pub fn yanks_since(&self, since_ns: u64) -> Result<YankList, RegistryError> {
        let args = serde_json::json!({ "since_ns": since_ns });
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

/// Map `McpError` → `RegistryError`, covering all variants exhaustively.
fn map_mcp_err(e: maos_domain::ports::mcp::McpError) -> RegistryError {
    match e {
        maos_domain::ports::mcp::McpError::UnknownServer(_) => RegistryError::Unconfigured,
        maos_domain::ports::mcp::McpError::CapabilityDenied { .. } => {
            RegistryError::Transport(
                "kernel-internal registry call lacks capability — composition root bug".into(),
            )
        }
        maos_domain::ports::mcp::McpError::Transport(inner) => {
            RegistryError::Transport(inner)
        }
        maos_domain::ports::mcp::McpError::Encode(e) => {
            RegistryError::Transport(format!("encode: {e}"))
        }
        maos_domain::ports::mcp::McpError::Decode(e) => {
            RegistryError::Transport(format!("decode: {e}"))
        }
        maos_domain::ports::mcp::McpError::Unconfigured => RegistryError::Unconfigured,
        maos_domain::ports::mcp::McpError::ServerError { code, message } => {
            RegistryError::Transport(format!("server error {code}: {message}"))
        }
    }
}

/// Decode a typed error from the JSON-RPC error shape.
///
/// Expected wire shape per FR63 typed-error convention:
/// `{"error_kind": "EVersionNotFound", "details": {"spirit_id": "...", "requested": "..."}}`.
/// Falls back to `RegistryError::Transport` for unknown error kinds.
fn decode_typed_error(v: &serde_json::Value) -> RegistryError {
    let kind = v
        .get("error_kind")
        .and_then(|k| k.as_str())
        .unwrap_or("unknown");
    let details = v.get("details");

    match kind {
        "EUnknownSpirit" => {
            let name = details
                .and_then(|d| d.get("spirit_id"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            RegistryError::UnknownSpirit(name)
        }
        "EVersionNotFound" => {
            let spirit_id = details
                .and_then(|d| d.get("spirit_id"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let requested = details
                .and_then(|d| d.get("requested"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            RegistryError::VersionNotFound {
                spirit_id,
                requested,
            }
        }
        "ESignatureInvalid" => RegistryError::SignatureInvalid,
        "ETrustTierFloorViolated" => {
            let manifest_tier = details
                .and_then(|d| d.get("manifest_tier"))
                .and_then(|t| serde_json::from_value::<TrustTier>(t.clone()).ok())
                .unwrap_or(TrustTier::PublicUntrusted);
            let floor = details
                .and_then(|d| d.get("floor"))
                .and_then(|t| serde_json::from_value::<TrustTier>(t.clone()).ok())
                .unwrap_or(TrustTier::Local);
            RegistryError::TrustTierFloorViolated {
                manifest_tier,
                floor,
            }
        }
        "EComplianceContextDrift" => {
            let actual = details
                .and_then(|d| d.get("actual_hex"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let claimed = details
                .and_then(|d| d.get("claimed_hex"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            RegistryError::ComplianceContextDrift {
                actual_hex: actual,
                claimed_hex: claimed,
            }
        }
        "EYanked" => {
            let spirit_id = details
                .and_then(|d| d.get("spirit_id"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let version = details
                .and_then(|d| d.get("version"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let reason = details
                .and_then(|d| d.get("reason"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            RegistryError::Yanked {
                spirit_id,
                version,
                reason,
            }
        }
        "ETransport" => {
            let msg = details
                .and_then(|d| d.get("message"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            RegistryError::Transport(msg)
        }
        "EUnconfigured" => RegistryError::Unconfigured,
        "EOrgSignatureInvalid" => RegistryError::OrgSignatureInvalid,
        "EPublicVettedDeferred" => RegistryError::PublicVettedDeferred,
        _ => RegistryError::Transport(format!("unknown typed error: {kind}")),
    }
}

// ---------------------------------------------------------------------------
// NullSpiritRegistryClient — null object for unconfigured registry
// ---------------------------------------------------------------------------

/// Returns `RegistryError::Unconfigured` on every call.
///
/// Used at composition root when `MAOS_REGISTRY_URI` is unset
/// AND no `operator.toml` exists — the kernel starts up cleanly
/// without a registry (only `maosctl install` requires it).
pub struct NullSpiritRegistryClient;

impl SpiritRegistryClient for NullSpiritRegistryClient {
    fn search(&self, _q: &SearchQuery) -> Result<SearchResults, RegistryError> {
        Err(RegistryError::Unconfigured)
    }

    fn manifest(
        &self,
        _spirit_id: &SpiritId,
        _version: &str,
    ) -> Result<SignedManifest, RegistryError> {
        Err(RegistryError::Unconfigured)
    }

    fn artifact(
        &self,
        _spirit_id: &SpiritId,
        _version: &str,
    ) -> Result<SignedArtifact, RegistryError> {
        Err(RegistryError::Unconfigured)
    }

    fn publish(&self, _pkg: &SignedPackage) -> Result<PublishReceipt, RegistryError> {
        Err(RegistryError::Unconfigured)
    }

    fn deprecate(
        &self,
        _spirit_id: &SpiritId,
        _version: &str,
        _reason: &YankReason,
    ) -> Result<YankReceipt, RegistryError> {
        Err(RegistryError::Unconfigured)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use maos_mcp::fixture_replay::FixtureReplayMcpServer;
    use maos_mcp::transport::McpTransportError;
    use std::collections::BTreeMap;

    use maos_domain::ports::mcp::{McpAttribution, McpTransportId};

    fn fake_response(content: serde_json::Value) -> maos_domain::ports::mcp::McpResponse {
        maos_domain::ports::mcp::McpResponse::new(
            content,
            false,
            McpAttribution::new(
                "spirit-registry".into(),
                McpTransportId::StreamableHttp,
                "registry.search".into(),
            ),
        )
    }

    fn fake_error_response(kind: &str, details: serde_json::Value) -> maos_domain::ports::mcp::McpResponse {
        maos_domain::ports::mcp::McpResponse::new(
            serde_json::json!({
                "error_kind": kind,
                "details": details,
            }),
            true,
            McpAttribution::new(
                "spirit-registry".into(),
                McpTransportId::StreamableHttp,
                "registry.search".into(),
            ),
        )
    }

    fn build_test_client(
        responses: Vec<Result<maos_domain::ports::mcp::McpResponse, McpTransportError>>,
    ) -> McpSpiritRegistryClient {
        let transport = Arc::new(FixtureReplayMcpServer::new(
            responses,
            McpTransportId::StreamableHttp,
        ));
        let mut transports: BTreeMap<McpTransportId, Arc<dyn maos_mcp::transport::McpTransport>> =
            BTreeMap::new();
        transports.insert(
            McpTransportId::StreamableHttp,
            transport as Arc<dyn maos_mcp::transport::McpTransport>,
        );

        let mut servers = BTreeMap::new();
        servers.insert(
            "spirit-registry".into(),
            maos_mcp::client::McpServerEntry {
                name: "spirit-registry".into(),
                transport: McpTransportId::StreamableHttp,
                fallback_transport: None,
            },
        );

        let mcp_client = Arc::new(
            McpClient::new(transports, McpTransportId::StreamableHttp, servers).unwrap(),
        );

        McpSpiritRegistryClient::new(mcp_client, "spirit-registry".into())
    }

    #[test]
    fn search_routes_to_registry_server() {
        let client = build_test_client(vec![Ok(fake_response(
            serde_json::json!({"items": []}),
        ))]);
        let q = SearchQuery::new("hello".into(), false, 50);
        let result = client.search(&q).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn publish_round_trip_with_signed_package() {
        use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};
        let pkg = SignedPackage::new(
            SpiritId::from("test-spirit"),
            "0.1.0".into(),
            b"[manifest]".to_vec(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            ComplianceClaimEnvelope {
                signature: [0u8; 64],
                attester_pubkey: [1u8; 32],
                claim_bytes: vec![0xA0],
                signing_alg: SigningAlg::Ed25519,
            },
        );
        let client = build_test_client(vec![Ok(fake_response(
            serde_json::json!({"publish_id": "pub-1", "spirit_id": "test-spirit", "version": "0.1.0"}),
        ))]);
        let receipt = client.publish(&pkg).unwrap();
        assert_eq!(receipt.publish_id, "pub-1");
        assert_eq!(receipt.version, "0.1.0");
    }

    #[test]
    fn version_not_found_maps_to_typed_error() {
        let client = build_test_client(vec![Ok(fake_error_response(
            "EVersionNotFound",
            serde_json::json!({"spirit_id": "test", "requested": "2.0.0"}),
        ))]);
        let q = SearchQuery::new("test".into(), false, 50);
        let err = client.search(&q).unwrap_err();
        assert!(matches!(err, RegistryError::VersionNotFound { .. }));
    }

    #[test]
    fn transport_error_maps_to_transport_error() {
        let client = build_test_client(vec![Err(McpTransportError::Transport("boom".into()))]);
        let q = SearchQuery::new("test".into(), false, 50);
        let err = client.search(&q).unwrap_err();
        assert!(matches!(err, RegistryError::Transport(_)));
    }

    #[test]
    fn unknown_typed_error_falls_back_to_transport() {
        let client = build_test_client(vec![Ok(fake_error_response(
            "EUnknownFutureVariant",
            serde_json::json!({}),
        ))]);
        let q = SearchQuery::new("test".into(), false, 50);
        let err = client.search(&q).unwrap_err();
        assert!(matches!(err, RegistryError::Transport(_)));
        assert!(err.to_string().contains("EUnknownFutureVariant"));
    }

    #[test]
    fn null_client_returns_unconfigured() {
        let client = NullSpiritRegistryClient;
        let q = SearchQuery::new("test".into(), false, 50);
        let err = client.search(&q).unwrap_err();
        assert!(matches!(err, RegistryError::Unconfigured));
    }

    #[test]
    fn null_client_all_methods_unconfigured() {
        let client = NullSpiritRegistryClient;
        assert!(matches!(
            client.search(&SearchQuery::new("x".into(), false, 1)),
            Err(RegistryError::Unconfigured)
        ));
        assert!(matches!(
            client.manifest(&SpiritId::from("x"), "1.0"),
            Err(RegistryError::Unconfigured)
        ));
        assert!(matches!(
            client.artifact(&SpiritId::from("x"), "1.0"),
            Err(RegistryError::Unconfigured)
        ));
        assert!(matches!(
            client.publish(&SignedPackage::new(
                SpiritId::from("x"),
                "1.0".into(),
                vec![],
                vec![],
                [0u8; 64],
                [0u8; 32],
                maos_spirit_abi::compliance::ComplianceClaimEnvelope {
                    signature: [0u8; 64],
                    attester_pubkey: [1u8; 32],
                    claim_bytes: vec![],
                    signing_alg: maos_spirit_abi::compliance::SigningAlg::Ed25519,
                },
            )),
            Err(RegistryError::Unconfigured)
        ));
        assert!(matches!(
            client.deprecate(&SpiritId::from("x"), "1.0", &YankReason::new("r".into())),
            Err(RegistryError::Unconfigured)
        ));
    }
}
