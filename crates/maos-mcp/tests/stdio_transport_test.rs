use maos_domain::ports::mcp::{McpRequest, McpTransportId};
use maos_mcp::transport::stdio::StdioTransport;
use maos_mcp::transport::McpTransport;

#[test]
fn stdio_transport_spawn_and_exchange() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[{"type":"text","text":"hello"}]}}"#;
    let transport = StdioTransport::new(
        "sh".into(),
        vec!["-c".into(), format!("echo '{}'", response)],
    )
    .unwrap();

    let req = McpRequest::new("test-server".into(), "echo".into(), serde_json::json!({}));
    let resp = transport.invoke(req).unwrap();

    assert!(!resp.is_error);
    assert_eq!(resp.attribution.transport_id, McpTransportId::Stdio);
    assert_eq!(resp.attribution.server_name, "test-server");
    assert_eq!(resp.attribution.tool, "echo");
}

#[test]
fn stdio_transport_nonexistent_binary_returns_transport_error() {
    let transport =
        StdioTransport::new("/nonexistent/binary/that/does/not/exist".into(), vec![]).unwrap();

    let req = McpRequest::new("test-server".into(), "echo".into(), serde_json::json!({}));
    let err = transport.invoke(req).unwrap_err();

    assert!(format!("{err}").contains("spawn failed"));
}

#[test]
fn stdio_transport_malformed_output_returns_transport_error() {
    let transport = StdioTransport::new(
        "sh".into(),
        vec!["-c".into(), "echo 'not json at all'".into()],
    )
    .unwrap();

    let req = McpRequest::new("test-server".into(), "echo".into(), serde_json::json!({}));
    let err = transport.invoke(req).unwrap_err();

    assert!(format!("{err}").contains("parse"));
}
