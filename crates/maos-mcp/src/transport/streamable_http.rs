//! Streamable HTTP Transport — single-shot HTTP POST + chunked response.
//!
//! Layered atop `IoSubsystemPort::http_post_exchange`. This is the **default
//! transport** at v0.5-α per ADR-008.
//!
//! Story 18-8 (D-18-1-06 B, dual-era): every first request speaks MCP
//! 2026-07-28 — `MCP-Protocol-Version`, `Mcp-Method`, `Mcp-Name` and
//! `params._meta`. A `400` whose FULL body is not an MCP-reserved JSON-RPC
//! error (−32099..=−32020) caches this origin as legacy (CR-1): `initialize` +
//! `notifications/initialized`, then the server-issued `Mcp-Session-Id` (when
//! one is issued) and the negotiated version ride every later request (CR-2).
//! The bearer never leaves the header map (AC2.3). No lock is held across an
//! exchange (review EC-2): a rare concurrent duplicate `initialize` is accepted.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use maos_domain::ports::io_subsystem::{HttpExchange, IoSubsystemPort};
use maos_domain::ports::mcp::{McpRequest, McpResponse, McpTransportId};
use serde_json::{json, Value};

use super::{McpTransport, McpTransportError};

pub const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";
pub const LEGACY_PROTOCOL_VERSION: &str = "2025-11-25";
/// The MCP-reserved JSON-RPC codes: a `400` carrying one comes from a modern
/// server (review BH-2: `MissingRequiredClientCapability` −32021 included).
const MODERN_ERROR_CODES: std::ops::RangeInclusive<i64> = -32099..=-32020;
const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;
const SESSION_HEADER: &str = "Mcp-Session-Id";
const MAX_ERROR_BODY: usize = 4096;
/// JSON-RPC ids: one request per POST, so the response is matched by them.
const INITIALIZE_ID: i64 = 0;
const CALL_ID: i64 = 1;

/// The session a cached-legacy origin speaks under (CR-2).
#[derive(Clone)]
struct LegacySession {
    version: String,
    session_id: Option<String>,
}

/// Streamable HTTP transport — DEFAULT at v0.5-α. Deliberately not `Debug`:
/// it holds the bearer.
pub struct StreamableHttpTransport {
    transport_inner: Arc<dyn IoSubsystemPort>,
    endpoint_url: String,
    bearer: Option<String>,
    /// CR-1 era cache for this transport's one origin: `None` = modern.
    legacy: Mutex<Option<LegacySession>>,
}

impl StreamableHttpTransport {
    /// `bearer`, when present, is sent as `Authorization: Bearer` on every
    /// request (CR-5); absent, no `Authorization` header is sent. Surrounding
    /// ASCII whitespace (a pasted or file-read `\r\n`) is trimmed (review BH-1).
    pub fn new(
        transport_inner: Arc<dyn IoSubsystemPort>,
        endpoint_url: String,
        bearer: Option<String>,
    ) -> Self {
        Self {
            transport_inner,
            endpoint_url,
            bearer: bearer.map(|bearer| bearer.trim_ascii().to_owned()),
            legacy: Mutex::new(None),
        }
    }

    fn cache(&self) -> MutexGuard<'_, Option<LegacySession>> {
        self.legacy.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// POST one JSON-RPC message; a 2xx is the exchange, 401 `Unauthorized`,
    /// a modern `400` `ModernRejection`, anything else `HttpStatus` — the era
    /// is judged on the full body, only the stored copy is truncated (EC-5).
    fn post(
        &self,
        body: &Value,
        extra: &[(&str, &str)],
    ) -> Result<HttpExchange, McpTransportError> {
        // Review BH-1/EC-1: a byte outside visible ASCII would fail ureq's
        // header validation with an error that echoes the header line.
        let authorization = match self.bearer.as_deref() {
            Some(bearer) if !bearer.bytes().all(|byte| byte.is_ascii_graphic()) => {
                return Err(McpTransportError::InvalidBearer)
            }
            bearer => bearer.map(|bearer| format!("Bearer {bearer}")),
        };
        let mut headers = vec![
            ("Content-Type", "application/json"),
            ("Accept", "application/json, text/event-stream"),
        ];
        headers.extend(
            authorization
                .as_deref()
                .map(|value| ("Authorization", value)),
        );
        headers.extend_from_slice(extra);
        let bytes = serde_json::to_vec(body)
            .map_err(|e| McpTransportError::Transport(format!("encode: {e}")))?;
        let exchange = self
            .transport_inner
            .http_post_exchange(&self.endpoint_url, &bytes, &headers, SESSION_HEADER)
            .map_err(|e| McpTransportError::Transport(format!("http_post: {e}")))?;
        if (200..=299).contains(&exchange.status) {
            return Ok(exchange);
        }
        let body = &exchange.body[..exchange.body.len().min(MAX_ERROR_BODY)];
        let body = String::from_utf8_lossy(body).into_owned();
        Err(match (exchange.status, modern_code(&exchange.body)) {
            (401, _) => McpTransportError::Unauthorized,
            (400, Some(code)) => McpTransportError::ModernRejection { code, body },
            (status, _) => McpTransportError::HttpStatus {
                status,
                body: Some(body),
            },
        })
    }

    fn modern_call(&self, request: &McpRequest) -> Result<McpResponse, McpTransportError> {
        let headers = [
            ("MCP-Protocol-Version", MODERN_PROTOCOL_VERSION),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", request.tool.as_str()),
        ];
        let exchange = self.post(&build_streamable_http_request_body(request, true), &headers)?;
        parse_exchange(exchange, CALL_ID, &request.server)
    }

    fn legacy_post(
        &self,
        session: &LegacySession,
        body: &Value,
    ) -> Result<HttpExchange, McpTransportError> {
        let mut headers = vec![("MCP-Protocol-Version", session.version.as_str())];
        headers.extend(session.session_id.as_deref().map(|id| (SESSION_HEADER, id)));
        self.post(body, &headers)
    }

    /// CR-2: `initialize`, keep the issued session id, `notifications/initialized`.
    fn initialize(&self, server: &str) -> Result<LegacySession, McpTransportError> {
        let initialize = json!({
            "jsonrpc": "2.0",
            "id": INITIALIZE_ID,
            "method": "initialize",
            "params": {
                "protocolVersion": LEGACY_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": client_info(),
            }
        });
        let exchange = self.post(&initialize, &[])?;
        let session_id = exchange.header.clone();
        let result = parse_exchange(exchange, INITIALIZE_ID, server)?.content;
        let session = LegacySession {
            version: result["protocolVersion"]
                .as_str()
                .unwrap_or(LEGACY_PROTOCOL_VERSION)
                .to_owned(),
            session_id,
        };
        let initialized = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        self.legacy_post(&session, &initialized)?;
        Ok(session)
    }

    fn legacy_call(
        &self,
        session: &mut LegacySession,
        request: &McpRequest,
    ) -> Result<McpResponse, McpTransportError> {
        let body = build_streamable_http_request_body(request, false);
        let exchange = match self.legacy_post(session, &body) {
            // CR-2: a 404 for a session means "start a new session" — once.
            Err(McpTransportError::HttpStatus { status: 404, .. })
                if session.session_id.is_some() =>
            {
                *session = self.initialize(&request.server)?;
                self.legacy_post(session, &body)
            }
            other => other,
        }?;
        parse_exchange(exchange, CALL_ID, &request.server)
    }
}

impl McpTransport for StreamableHttpTransport {
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError> {
        let cached = self.cache().clone();
        if let Some(mut session) = cached {
            let result = self.legacy_call(&mut session, &request);
            // CR-1: a modern rejection on a cached-legacy origin re-probes once.
            let reprobe = matches!(result, Err(McpTransportError::ModernRejection { .. }));
            *self.cache() = (!reprobe).then_some(session);
            if !reprobe {
                return result;
            }
        }
        match self.modern_call(&request) {
            Err(McpTransportError::HttpStatus { status: 400, .. }) => {
                let mut session = self.initialize(&request.server)?;
                let result = self.legacy_call(&mut session, &request);
                *self.cache() = Some(session);
                result
            }
            result => result,
        }
    }

    fn id(&self) -> McpTransportId {
        McpTransportId::StreamableHttp
    }
}

/// The MCP-reserved code of a modern server's `400`, judged on the full body;
/// `None` is the legacy trigger. Review ruling D-18-8-R2: an
/// `UnsupportedProtocolVersion` whose `data.supported` lists the legacy
/// version takes the legacy path (versioning page: the client SHOULD pick a
/// mutually supported version).
fn modern_code(body: &[u8]) -> Option<i64> {
    let error = serde_json::from_slice::<Value>(body).ok()?["error"].take();
    let code = error["code"]
        .as_i64()
        .filter(|code| MODERN_ERROR_CODES.contains(code))?;
    let supported = error["data"]["supported"].as_array();
    let legacy_ok = supported.is_some_and(|list| list.iter().any(|v| v == LEGACY_PROTOCOL_VERSION));
    (code != UNSUPPORTED_PROTOCOL_VERSION || !legacy_ok).then_some(code)
}

fn client_info() -> Value {
    json!({"name": "maos", "version": env!("CARGO_PKG_VERSION")})
}

/// Review TIA-7c: a 2026-07-28 Streamable HTTP server MAY answer a POST with
/// `text/event-stream`. The IO port exposes one response header (the session
/// id), not `Content-Type`, so the body is sniffed: a body that is not a JSON
/// object is an event stream, and its answer is the `data:` of the first event
/// that is the JSON-RPC response to `id` (requests/notifications skipped).
fn parse_exchange(
    exchange: HttpExchange,
    id: i64,
    server: &str,
) -> Result<McpResponse, McpTransportError> {
    let text = String::from_utf8(exchange.body)
        .map_err(|e| McpTransportError::Transport(format!("utf8: {e}")))?;
    if text.trim_start().starts_with('{') {
        return parse_streamable_http_response(&text, server);
    }
    let response = text.replace("\r\n", "\n").split("\n\n").find_map(|event| {
        let data: Vec<&str> = event
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(|data| data.strip_prefix(' ').unwrap_or(data))
            .collect();
        let message = serde_json::from_str::<Value>(&data.join("\n")).ok()?;
        (message["id"] == id && message.get("method").is_none()).then_some(message)
    });
    let response = response.ok_or_else(|| {
        McpTransportError::Transport("no JSON-RPC response in the event stream".into())
    })?;
    super::response_from(&response, server, McpTransportId::StreamableHttp)
}

/// Build the JSON-RPC 2.0 `tools/call` body; `modern` adds the 2026-07-28
/// per-request `params._meta` (version, capabilities, client info).
pub fn build_streamable_http_request_body(request: &McpRequest, modern: bool) -> Value {
    let mut params = json!({"name": request.tool, "arguments": request.args});
    if modern {
        params["_meta"] = json!({
            "io.modelcontextprotocol/protocolVersion": MODERN_PROTOCOL_VERSION,
            "io.modelcontextprotocol/clientCapabilities": {},
            "io.modelcontextprotocol/clientInfo": client_info(),
        });
    }
    json!({"jsonrpc": "2.0", "id": CALL_ID, "method": "tools/call", "params": params})
}

/// Parse a Streamable HTTP response body into an `McpResponse`.
///
/// Supports two response shapes:
/// - Single-shot JSON (Content-Type: application/json)
/// - SSE-format chunks (Content-Type: text/event-stream)
///
/// For now, we treat the body as a single-shot JSON response.
pub fn parse_streamable_http_response(
    body: &str,
    server_name: &str,
) -> Result<McpResponse, McpTransportError> {
    // Try streamed SSE first
    if !body.trim_start().starts_with('{') && (body.contains("event:") || body.starts_with("data:"))
    {
        return crate::transport::sse::parse_sse_response(body, server_name);
    }

    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| McpTransportError::Transport(format!("http parse: {e}")))?;
    super::response_from(&value, server_name, McpTransportId::StreamableHttp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_request_body_contains_tool_name() {
        let req = McpRequest::new("srv".into(), "tool1".into(), json!({"a":1}));
        let body = build_streamable_http_request_body(&req, false);
        assert_eq!(body["method"], "tools/call");
        assert_eq!(body["params"]["name"], "tool1");
        assert!(body["params"].get("_meta").is_none());
    }

    #[test]
    fn parse_single_shot_json_response() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"echo","content":[{"type":"text","text":"ok"}]}}"#;
        let resp = parse_streamable_http_response(body, "test-server").unwrap();
        assert!(!resp.is_error);
        assert_eq!(
            resp.attribution.transport_id,
            McpTransportId::StreamableHttp
        );
    }

    #[test]
    fn parse_error_response() {
        let body =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let err = parse_streamable_http_response(body, "srv").unwrap_err();
        match err {
            McpTransportError::ServerError { code, .. } => assert_eq!(code, -32600),
            other => panic!("expected ServerError, got {other:?}"),
        }
    }

    #[test]
    fn parse_sse_format_response_delegates_to_sse_parser() {
        let body = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"name\":\"echo\",\"content\":[]}}\n\n";
        let resp = parse_streamable_http_response(body, "test-server").unwrap();
        assert_eq!(resp.attribution.transport_id, McpTransportId::Sse);
    }
}
