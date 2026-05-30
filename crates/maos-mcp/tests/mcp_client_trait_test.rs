//! Story 7.2 AC5 §5.1–§5.2 — `McpClient` trait abstraction tests.

use std::collections::BTreeMap;
use std::sync::Arc;

use maos_domain::ports::mcp::{
    McpAttribution, McpCallResponse, McpError, McpRequest, McpResponse, McpTransportId,
};
use maos_mcp::fixture_replay::FixtureReplayMcpServer;
use maos_mcp::{McpClient, McpClientImpl, McpServerEntry};

fn make_replay_transport(
    responses: Vec<Result<McpResponse, maos_mcp::transport::McpTransportError>>,
) -> Arc<FixtureReplayMcpServer> {
    Arc::new(FixtureReplayMcpServer::new(responses, McpTransportId::StreamableHttp))
}

fn make_client_with_transport(
    transport: Arc<FixtureReplayMcpServer>,
) -> McpClientImpl {
    let mut transports: BTreeMap<McpTransportId, Arc<dyn maos_mcp::transport::McpTransport>> =
        BTreeMap::new();
    transports.insert(McpTransportId::StreamableHttp, transport.clone());

    let mut servers = BTreeMap::new();
    servers.insert(
        "spirit-registry".into(),
        McpServerEntry {
            name: "spirit-registry".into(),
            transport: McpTransportId::StreamableHttp,
            fallback_transport: None,
        },
    );

    McpClientImpl::new(transports, McpTransportId::StreamableHttp, servers).unwrap()
}

fn make_response(result: serde_json::Value) -> McpResponse {
    McpResponse::new(
        result,
        false,
        McpAttribution::new(
            "spirit-registry".into(),
            McpTransportId::StreamableHttp,
            "publish".into(),
        ),
    )
}

#[test]
fn trait_object_dispatches_correctly() {
    let response1 = make_response(serde_json::json!({"publish_id": "test-pub-1"}));
    let response2 = make_response(serde_json::json!({"publish_id": "test-pub-1"}));
    let transport = make_replay_transport(vec![Ok(response1), Ok(response2)]);
    let client = make_client_with_transport(transport.clone());

    // Direct call
    let direct = client.call("spirit-registry", "publish", serde_json::json!({"x": 1}));
    assert!(direct.is_ok());

    // Trait object call
    let trait_obj: Arc<dyn McpClient> = Arc::new(client);
    let via_trait = trait_obj.call("spirit-registry", "publish", serde_json::json!({"x": 1}));
    assert!(via_trait.is_ok());

    // Both should produce the same response shape
    let direct_resp = direct.unwrap();
    let trait_resp = via_trait.unwrap();
    assert_eq!(direct_resp.content, trait_resp.content);
}

#[test]
fn fixture_replay_impl_dispatches_correctly() {
    let response = make_response(serde_json::json!({"publish_id": "fixture-pub-42"}));
    let transport = make_replay_transport(vec![Ok(response)]);
    let client = make_client_with_transport(transport.clone());
    let trait_obj: Arc<dyn McpClient> = Arc::new(client);

    let result = trait_obj
        .call("spirit-registry", "publish", serde_json::json!({"y": 2}))
        .unwrap();

    assert_eq!(
        result.content,
        serde_json::json!({"publish_id": "fixture-pub-42"})
    );

    // Verify the transport recorded the call
    let calls = transport.take_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].server, "spirit-registry");
    assert_eq!(calls[0].tool, "publish");
}
