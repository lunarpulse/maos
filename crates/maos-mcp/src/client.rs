//! MCP client — operator-configurable per-server transport selection.
//!
//! Public surface this crate exposes to callers (consumers: the kernel-side
//! adapter at `maos-kernel-core::mcp::McpClientAdapter`; Story 5.5d's
//! registry client; Epic 8 reference Spirits invoke this through the
//! capability/mediation path).
//!
//! ADR-010 sync trait — `call` returns synchronously; async callers wrap
//! in `spawn_blocking`.  Per the kernel-stays-small invariant, no Tokio
//! runtime is required inside this crate at v0.5-α.

use std::collections::BTreeMap;
use std::sync::Arc;

use maos_domain::ports::mcp::{McpCallResponse, McpError, McpRequest, McpResponse, McpTransportId};

use crate::transport::{McpTransport, McpTransportError};

/// MCP client — holds a map of transports and a map of server entries.
pub struct McpClientImpl {
    transports: BTreeMap<McpTransportId, Arc<dyn McpTransport>>,
    default_transport: McpTransportId,
    servers: BTreeMap<String, McpServerEntry>,
}

impl std::fmt::Debug for McpClientImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClient")
            .field("default_transport", &self.default_transport)
            .field("servers", &self.servers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Per-server MCP entry — maps a server name to its transport + fallback.
#[derive(Debug, Clone)]
pub struct McpServerEntry {
    pub name: String,
    pub transport: McpTransportId,
    pub fallback_transport: Option<McpTransportId>,
}

impl McpClientImpl {
    #[doc = "Construct a new [`McpClientImpl`] with the given transports, default transport, and server entries."]
    pub fn new(
        transports: BTreeMap<McpTransportId, Arc<dyn McpTransport>>,
        default_transport: McpTransportId,
        servers: BTreeMap<String, McpServerEntry>,
    ) -> Result<Self, McpError> {
        if transports.is_empty() {
            return Err(McpError::Unconfigured);
        }
        if !transports.contains_key(&default_transport) {
            return Err(McpError::Unconfigured);
        }
        // NOTE: server transport validation deferred to call-time per AC1 —
        // an operator may register a server before its transport is wired.
        for entry in servers.values() {
            if let Some(fb) = &entry.fallback_transport {
                if fb == &entry.transport {
                    return Err(McpError::Transport(format!(
                        "server '{}' fallback_transport must differ from primary",
                        entry.name
                    )));
                }
            }
        }
        Ok(Self {
            transports,
            default_transport,
            servers,
        })
    }

    /// Class: data-movement
    ///
    /// Invoke an MCP tool.  Caller is responsible for the capability-token
    /// check; that mediation lives in `McpClientAdapter` (kernel-side).
    pub fn call(
        &self,
        server_name: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<McpCallResponse, McpError> {
        let entry = self
            .servers
            .get(server_name)
            .ok_or_else(|| McpError::UnknownServer(server_name.into()))?;

        let transport = self.transports.get(&entry.transport).ok_or_else(|| {
            McpError::Transport(format!(
                "server '{}' transport {:?} not registered",
                entry.name, entry.transport
            ))
        })?;

        let request = McpRequest::new(server_name.into(), tool.into(), args);

        match transport.invoke(request.clone()) {
            Ok(resp) => Ok(resp),
            Err(McpTransportError::ServerError { code, message }) => {
                Err(McpError::ServerError { code, message })
            }
            Err(te) => {
                // Walk fallback if present
                if let Some(fb_id) = &entry.fallback_transport {
                    if let Some(fb_transport) = self.transports.get(fb_id) {
                        fb_transport
                            .invoke(request)
                            .map_err(|fb_err| McpError::Transport(fb_err.to_string()))
                    } else {
                        Err(McpError::Transport(format!(
                            "primary transport error: {te}"
                        )))
                    }
                } else {
                    Err(McpError::Transport(format!(
                        "primary transport error: {te}"
                    )))
                }
            }
        }
    }

    /// Registered server names — the operator-visible inventory.
    pub fn registered_servers(&self) -> Vec<String> {
        self.servers.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::fixture_replay::FixtureReplayMcpServer;
    use crate::transport::McpTransportError;
    use maos_domain::ports::mcp::{McpAttribution, McpResponse};

    fn fake_response(server: &str, tool: &str, tid: McpTransportId) -> McpResponse {
        McpResponse::new(
            json!({"result": "ok"}),
            false,
            McpAttribution::new(server.into(), tid, tool.into()),
        )
    }

    fn build_client(
        servers: BTreeMap<String, McpServerEntry>,
        transports: BTreeMap<McpTransportId, Arc<dyn McpTransport>>,
    ) -> McpClientImpl {
        let default = transports.keys().next().cloned().unwrap();
        McpClientImpl::new(transports, default, servers).unwrap()
    }

    #[test]
    fn call_routes_to_per_server_transport() {
        let t1 = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(fake_response(
                "server-a",
                "echo",
                McpTransportId::StreamableHttp,
            ))],
            McpTransportId::StreamableHttp,
        ));
        let t2 = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(fake_response("server-b", "echo", McpTransportId::Stdio))],
            McpTransportId::Stdio,
        ));

        let mut transports = BTreeMap::new();
        transports.insert(McpTransportId::StreamableHttp, t1 as Arc<dyn McpTransport>);
        transports.insert(McpTransportId::Stdio, t2 as Arc<dyn McpTransport>);

        let mut servers = BTreeMap::new();
        servers.insert(
            "server-a".into(),
            McpServerEntry {
                name: "server-a".into(),
                transport: McpTransportId::StreamableHttp,
                fallback_transport: None,
            },
        );
        servers.insert(
            "server-b".into(),
            McpServerEntry {
                name: "server-b".into(),
                transport: McpTransportId::Stdio,
                fallback_transport: None,
            },
        );

        let client = build_client(servers, transports);
        let resp_a = client.call("server-a", "echo", json!({})).unwrap();
        assert_eq!(
            resp_a.attribution.transport_id,
            McpTransportId::StreamableHttp
        );
        let resp_b = client.call("server-b", "echo", json!({})).unwrap();
        assert_eq!(resp_b.attribution.transport_id, McpTransportId::Stdio);
    }

    #[test]
    fn call_returns_unknown_server_for_unregistered() {
        let t = Arc::new(FixtureReplayMcpServer::new(vec![], McpTransportId::Stdio));
        let mut transports = BTreeMap::new();
        transports.insert(McpTransportId::Stdio, t as Arc<dyn McpTransport>);
        let err = build_client(BTreeMap::new(), transports)
            .call("nonexistent", "echo", json!({}))
            .unwrap_err();
        assert!(matches!(err, McpError::UnknownServer(_)));
    }

    #[test]
    fn call_walks_fallback_on_transport_error() {
        let primary = Arc::new(FixtureReplayMcpServer::new(
            vec![Err(McpTransportError::Transport("boom".into()))],
            McpTransportId::StreamableHttp,
        ));
        let fallback = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(fake_response("svc", "echo", McpTransportId::Stdio))],
            McpTransportId::Stdio,
        ));

        let mut transports = BTreeMap::new();
        transports.insert(
            McpTransportId::StreamableHttp,
            primary as Arc<dyn McpTransport>,
        );
        transports.insert(McpTransportId::Stdio, fallback as Arc<dyn McpTransport>);

        let mut servers = BTreeMap::new();
        servers.insert(
            "svc".into(),
            McpServerEntry {
                name: "svc".into(),
                transport: McpTransportId::StreamableHttp,
                fallback_transport: Some(McpTransportId::Stdio),
            },
        );

        let client = build_client(servers, transports);
        let resp = client.call("svc", "echo", json!({})).unwrap();
        assert_eq!(resp.attribution.transport_id, McpTransportId::Stdio);
    }

    #[test]
    fn call_does_not_walk_fallback_on_server_error() {
        let primary = Arc::new(FixtureReplayMcpServer::new(
            vec![Err(McpTransportError::ServerError {
                code: -32601,
                message: "Method not found".into(),
            })],
            McpTransportId::StreamableHttp,
        ));
        let fallback = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(fake_response("svc", "echo", McpTransportId::Stdio))],
            McpTransportId::Stdio,
        ));

        let mut transports = BTreeMap::new();
        transports.insert(
            McpTransportId::StreamableHttp,
            primary as Arc<dyn McpTransport>,
        );
        transports.insert(McpTransportId::Stdio, fallback as Arc<dyn McpTransport>);

        let mut servers = BTreeMap::new();
        servers.insert(
            "svc".into(),
            McpServerEntry {
                name: "svc".into(),
                transport: McpTransportId::StreamableHttp,
                fallback_transport: Some(McpTransportId::Stdio),
            },
        );

        let client = build_client(servers, transports);
        let err = client.call("svc", "echo", json!({})).unwrap_err();
        // Should have short-circuited; fallback was NOT used
        assert!(matches!(err, McpError::ServerError { code: -32601, .. }));
    }

    #[test]
    fn call_walks_fallback_on_transport_mapped_decode_error() {
        // Decode errors are mapped to Transport by our transports, so they
        // WILL walk fallback (which is correct for deterministically-bad
        // response parsing).  The McpError::Decode variant is reserved for
        // the kernel-side adapter's own serde path.
        let primary = Arc::new(FixtureReplayMcpServer::new(
            vec![Err(McpTransportError::Transport("decode error".into()))],
            McpTransportId::StreamableHttp,
        ));
        let fallback = Arc::new(FixtureReplayMcpServer::new(
            vec![Ok(fake_response("svc", "echo", McpTransportId::Stdio))],
            McpTransportId::Stdio,
        ));

        let mut transports = BTreeMap::new();
        transports.insert(
            McpTransportId::StreamableHttp,
            primary as Arc<dyn McpTransport>,
        );
        transports.insert(McpTransportId::Stdio, fallback as Arc<dyn McpTransport>);

        let mut servers = BTreeMap::new();
        servers.insert(
            "svc".into(),
            McpServerEntry {
                name: "svc".into(),
                transport: McpTransportId::StreamableHttp,
                fallback_transport: Some(McpTransportId::Stdio),
            },
        );

        let client = build_client(servers, transports);
        let resp = client.call("svc", "echo", json!({})).unwrap();
        assert_eq!(resp.attribution.transport_id, McpTransportId::Stdio);
    }

    #[test]
    fn call_with_unregistered_transport_returns_error() {
        let mut servers = BTreeMap::new();
        servers.insert(
            "svc".into(),
            McpServerEntry {
                name: "svc".into(),
                transport: McpTransportId::Stdio,
                fallback_transport: None,
            },
        );
        // No stdio transport registered
        let t = Arc::new(FixtureReplayMcpServer::new(
            vec![],
            McpTransportId::StreamableHttp,
        ));
        let mut transports = BTreeMap::new();
        transports.insert(McpTransportId::StreamableHttp, t as Arc<dyn McpTransport>);

        let client =
            McpClientImpl::new(transports, McpTransportId::StreamableHttp, servers).unwrap();
        let err = client.call("svc", "echo", json!({})).unwrap_err();
        assert!(matches!(err, McpError::Transport(_)));
        assert!(err.to_string().contains("not registered"));
    }

    #[test]
    fn call_with_unconfigured_transport_returns_unconfigured() {
        let err = McpClientImpl::new(
            BTreeMap::new(),
            McpTransportId::StreamableHttp,
            BTreeMap::new(),
        )
        .unwrap_err();
        assert!(matches!(err, McpError::Unconfigured));
    }

    // NOTE: Fix #33 — concurrent calls test deferred. Requires multi-threaded
    // test infrastructure to exercise overlapping invoke() calls safely.
}
