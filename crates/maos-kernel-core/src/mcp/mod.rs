#![forbid(unsafe_code)]

//! MCP client adapter — kernel-side capability mediation + Transparency-Log
//! emission for MCP tool invocations (Story 5.5c).
//!
//! Mirrors `InferencePortAdapter` at `crates/maos-kernel-core/src/inference/mod.rs`.
//! Holds the wire-level `McpClient` from `maos-mcp` + the capability registry +
//! the Transparency Log adapter + the telemetry stream.

use std::sync::Arc;

use maos_domain::invariants::i1::{CapabilityToken, Scope, TokenId};
use maos_domain::ports::mcp::{McpCallResponse, McpClientPort, McpError, McpRequest};

use crate::capability::CapabilityRegistryAdapter;
use crate::iac::{FrameKind, TransparencyLogAdapter};
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};

/// Kernel-side adapter for the MCP client port.
#[maos_attrs::i9_exempt(
    reason = "mcp-client adapter aggregate; holds Arc references to wire-level client + audit infrastructure"
)]
pub struct McpClientAdapter {
    client: Arc<dyn maos_mcp::McpClient + Send + Sync>,
    capability: Arc<CapabilityRegistryAdapter>,
    transparency_log: Arc<TransparencyLogAdapter>,
    telemetry: Arc<IacRtMetrics>,
}

impl McpClientAdapter {
    pub fn new(
        client: Arc<dyn maos_mcp::McpClient + Send + Sync>,
        capability: Arc<CapabilityRegistryAdapter>,
        transparency_log: Arc<TransparencyLogAdapter>,
        telemetry: Arc<IacRtMetrics>,
    ) -> Self {
        Self {
            client,
            capability,
            transparency_log,
            telemetry,
        }
    }

    /// Verify the capability token authorizes `Scope::McpCall` for the given
    /// server + tool combination.
    fn check_capability(
        &self,
        token: &CapabilityToken,
        server: &str,
        tool: &str,
    ) -> Result<(), McpError> {
        // posture_hash and SandboxTier are hardcoded zeros — consistent with
        // InferencePortAdapter (inference/mod.rs) which uses the same pattern.
        // Real posture-hash and tier will be wired when the kernel tracks per-call posture state.
        let posture_hash = [0u8; 32];
        self.capability
            .verify_and_audit(token, posture_hash, maos_domain::invariants::i9::SandboxTier(0))
            .map_err(|_e| McpError::CapabilityDenied {
                server: server.into(),
                tool: tool.into(),
            })?;

        let scope = self.capability.get_token_scope(&token.token_id);
        match scope {
            Some(Scope::McpCall { server: s, tool: t }) if s == server && t == tool => Ok(()),
            _ => Err(McpError::CapabilityDenied {
                server: server.into(),
                tool: tool.into(),
            }),
        }
    }
}

impl McpClientPort for McpClientAdapter {
    fn call(
        &self,
        token: &CapabilityToken,
        server: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        self.check_capability(token, server, tool)?;

        let _inflight = self.telemetry.inflight(Service::Capability);

        let start_ns = crate::capability::cap_tokens::monotonic_now_ns();

        let response = self
            .client
            .call(server, tool, args.clone())
            .map_err(|e| McpError::Transport(e.to_string()))?;

        let end_ns = crate::capability::cap_tokens::monotonic_now_ns();

        // TODO: duration_ns not passed to TL — insert_frame_event API doesn't accept a duration parameter.
        // The spec requires duration_ns: Some(end_ns - start_ns) in the TL row.
        // Tracked as finding #12 in the review.

        // Emit Transparency Log row
        let intent = format!("mcp:{server}/{tool}");
        let payload = serde_json::to_vec(&args).map_err(McpError::Encode)?;
        let mut token_bytes = [0u8; 32];
        token_bytes[..16].copy_from_slice(&token.token_id.0);
        let _log_token = self.transparency_log.insert_frame_event(
            FrameKind::McpInvocation,
            token.spirit_pid,
            Some(&token_bytes),
            &intent,
            &payload,
            maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
        );

        // Telemetry round-trip
        let duration_us = (end_ns - start_ns) / 1000;
        if response.is_error {
            self.telemetry
                .record_iac_rt(Service::Capability, Outcome::Err, duration_us);
        } else {
            self.telemetry
                .record_iac_rt(Service::Capability, Outcome::Ok, duration_us);
        }

        Ok(response)
    }
}

#[cfg(all(test, feature = "fixture_replay"))]
mod tests {
    use super::*;
    use crate::capability::cap_policy::PolicyTable;
    use crate::capability::cap_quota::CapQuotaTracker;
    use crate::capability::cap_tokens::Ed25519SigningKey;
    use crate::security::crypto::tests::MockCryptoProvider;
    use maos_domain::invariants::i1::{CapabilityToken, IntentClass, TokenId};
    use maos_domain::ports::mcp::{McpAttribution, McpRequest, McpResponse};
    use maos_mcp::fixture_replay::FixtureReplayMcpServer;

    fn test_adapter() -> (McpClientAdapter, Arc<CapabilityRegistryAdapter>, Arc<TransparencyLogAdapter>) {
        let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> =
            Arc::new(MockCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let policy = Arc::new(PolicyTable::new());
        {
            let mut inner = crate::capability::cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(
                7,
                crate::capability::cap_policy::ManifestCapabilityScope {
                    scopes: vec![Scope::McpCall {
                        server: "test-server".into(),
                        tool: "echo".into(),
                    }],
                    declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                    trust_tier: crate::capability::cap_policy::decision::TrustTier::Verified,
                },
            );
            policy.update(inner);
        }
        let (audit_tx, _audit_rx) = crate::capability::cap_audit::channel();
        let quota = CapQuotaTracker::new();
        let working_memory = Arc::new(crate::capability::WorkingMemoryStore::new());
        let telemetry_ct = Arc::new(crate::telemetry::TelemetryStreamAdapter::default());
        let capabilities = Arc::new(CapabilityRegistryAdapter::new(
            crypto,
            signing_key,
            0xDEAD_BEEF,
            policy,
            audit_tx,
            quota,
            working_memory,
            telemetry_ct,
        ));
        let transparency_log = Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF));
        let telemetry = Arc::new(IacRtMetrics::new());

        let fake_response = McpResponse::new(
            serde_json::json!({"result": "ok"}),
            false,
            McpAttribution::new(
                "test-server".into(),
                maos_domain::ports::mcp::McpTransportId::Stdio,
                "echo".into(),
            ),
        );

        let mcp_transport = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(fake_response)],
            maos_domain::ports::mcp::McpTransportId::Stdio,
        )) as Arc<dyn maos_mcp::McpTransport>;

        use std::collections::BTreeMap;
        let mut transports: BTreeMap<maos_domain::ports::mcp::McpTransportId, Arc<dyn maos_mcp::McpTransport>> = BTreeMap::new();
        transports.insert(maos_domain::ports::mcp::McpTransportId::Stdio, mcp_transport);

        let mut servers: BTreeMap<String, maos_mcp::McpServerEntry> = BTreeMap::new();
        servers.insert(
            "test-server".into(),
            maos_mcp::McpServerEntry {
                name: "test-server".into(),
                transport: maos_domain::ports::mcp::McpTransportId::Stdio,
                fallback_transport: None,
            },
        );

        let client = Arc::new(
            maos_mcp::McpClientImpl::new(
                transports,
                maos_domain::ports::mcp::McpTransportId::StreamableHttp,
                servers,
            )
            .unwrap(),
        );

        let adapter = McpClientAdapter::new(
            client,
            Arc::clone(&capabilities),
            Arc::clone(&transparency_log),
            telemetry,
        );

        (adapter, capabilities, transparency_log)
    }

    fn make_token(
        capabilities: &CapabilityRegistryAdapter,
        spirit_pid: u32,
        scope: Scope,
    ) -> CapabilityToken {
        crate::capability::cap_tokens::init_monotonic_base();
        capabilities
            .issue_with_mediation(spirit_pid, scope, 60, [0u8; 32], IntentClass::Standard)
            .unwrap()
    }

    #[test]
    fn capability_denied_returns_capability_denied() {
        let (adapter, _capabilities, tl) = test_adapter();
        // Construct a token with wrong scope directly (no manifest check needed)
        let bad_token = CapabilityToken::new(TokenId::ZERO, 99, u64::MAX, [0u8; 64]);
        let err = adapter
            .call(&bad_token, "test-server", "echo", serde_json::json!({}))
            .unwrap_err();
        assert!(matches!(err, McpError::CapabilityDenied { .. }));

        // Verify NO McpInvocation TL row was emitted
        let entries = tl
            .query_frames(crate::iac::FrameFilter {
                kind: Some(FrameKind::McpInvocation),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn mock_client_round_trip_logs_mcp_invocation() {
        let (adapter, capabilities, tl) = test_adapter();
        let token = make_token(
            &capabilities,
            7,
            Scope::McpCall {
                server: "test-server".into(),
                tool: "echo".into(),
            },
        );
        let resp = adapter
            .call(&token, "test-server", "echo", serde_json::json!({"msg": "hi"}))
            .unwrap();
        assert!(!resp.is_error);

        let entries = tl
            .query_frames(crate::iac::FrameFilter {
                kind: Some(FrameKind::McpInvocation),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].spirit_pid, 7);
        assert!(entries[0].intent.starts_with("mcp:test-server"));
    }

    #[test]
    fn server_error_response_logs_outcome_server_error() {
        // We map server errors to McpError::Transport on the adapter side,
        // but to test the is_error path we need a client that returns
        // a response with is_error = true.
        let crypto: Arc<dyn maos_domain::ports::crypto::CryptoProvider> =
            Arc::new(MockCryptoProvider);
        let signing_key = Ed25519SigningKey::new([0u8; 32]);
        let policy = Arc::new(PolicyTable::new());
        {
            let mut inner = crate::capability::cap_policy::PolicyTableInner::default();
            inner.manifest_scopes.insert(
                7,
                crate::capability::cap_policy::ManifestCapabilityScope {
                    scopes: vec![Scope::McpCall {
                        server: "err-srv".into(),
                        tool: "faulty".into(),
                    }],
                    declared_tier: maos_domain::invariants::i9::SandboxTier(0),
                    trust_tier: crate::capability::cap_policy::decision::TrustTier::Verified,
                },
            );
            policy.update(inner);
        }
        let (audit_tx, _audit_rx) = crate::capability::cap_audit::channel();
        let quota = CapQuotaTracker::new();
        let wm = Arc::new(crate::capability::WorkingMemoryStore::new());
        let tel = Arc::new(crate::telemetry::TelemetryStreamAdapter::default());
        let capabilities = Arc::new(CapabilityRegistryAdapter::new(
            crypto,
            signing_key,
            0xDEAD_BEEF,
            policy,
            audit_tx,
            quota,
            wm,
            tel,
        ));
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xDEAD_BEEF));
        let telemetry = Arc::new(IacRtMetrics::new());

        let error_resp = McpResponse::new(
            serde_json::json!({"error": "something went wrong"}),
            true,
            McpAttribution::new(
                "err-srv".into(),
                maos_domain::ports::mcp::McpTransportId::Stdio,
                "faulty".into(),
            ),
        );
        let mcp_transport = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(error_resp)],
            maos_domain::ports::mcp::McpTransportId::Stdio,
        )) as Arc<dyn maos_mcp::McpTransport>;

        use std::collections::BTreeMap;
        let mut transports: BTreeMap<maos_domain::ports::mcp::McpTransportId, Arc<dyn maos_mcp::McpTransport>> = BTreeMap::new();
        transports.insert(maos_domain::ports::mcp::McpTransportId::Stdio, mcp_transport);
        let mut servers: BTreeMap<String, maos_mcp::McpServerEntry> = BTreeMap::new();
        servers.insert(
            "err-srv".into(),
            maos_mcp::McpServerEntry {
                name: "err-srv".into(),
                transport: maos_domain::ports::mcp::McpTransportId::Stdio,
                fallback_transport: None,
            },
        );
        let client = Arc::new(
            maos_mcp::McpClientImpl::new(
                transports,
                maos_domain::ports::mcp::McpTransportId::StreamableHttp,
                servers,
            ).unwrap(),
        );
        let adapter = McpClientAdapter::new(client, capabilities.clone(), tl.clone(), telemetry);

        crate::capability::cap_tokens::init_monotonic_base();
        let token = capabilities
            .issue_with_mediation(
                7,
                Scope::McpCall {
                    server: "err-srv".into(),
                    tool: "faulty".into(),
                },
                60,
                [0u8; 32],
                IntentClass::Standard,
            )
            .unwrap();

        let resp = adapter
            .call(&token, "err-srv", "faulty", serde_json::json!({}))
            .unwrap();
        assert!(resp.is_error);

        // Verify TL row exists (outcome = "server_error" is logged by the
        // adapter, not the TL row itself, so just verify the row is there)
        let entries = tl
            .query_frames(crate::iac::FrameFilter {
                kind: Some(FrameKind::McpInvocation),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn monotonic_now_ns_used_for_timestamp() {
        let (adapter, capabilities, tl) = test_adapter();
        let token = make_token(&capabilities, 7, Scope::McpCall {
            server: "test-server".into(),
            tool: "echo".into(),
        });
        let _resp = adapter
            .call(&token, "test-server", "echo", serde_json::json!({}))
            .unwrap();
        let entries = tl.query_frames(crate::iac::FrameFilter {
            kind: Some(FrameKind::McpInvocation),
            ..Default::default()
        }).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].timestamp_ns > 0, "TL timestamp should be non-zero");
    }

    #[test]
    fn encode_error_variant_is_constructable() {
        let err = McpError::Encode(serde_json::from_str::<serde_json::Value>("not json").unwrap_err());
        assert!(err.to_string().to_lowercase().contains("encode"));
    }
}
