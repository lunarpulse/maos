//! Story 18-8 AC1 / AC2 — a conformant, dual-era MCP client (CR-4).
//!
//! The real `StreamableHttpTransport` over the real `IoSubsystemAdapter`
//! against `ConformantMcp`: a modern (2026-07-28) server is spoken to without a
//! handshake; a legacy (2025-11-25) server gets exactly one `initialize` and
//! one persisted `Mcp-Session-Id`; a missing bearer is a typed `Unauthorized`
//! on both. Every fixture negative is first asserted against a hand-built
//! request, then surfaced typed from the real client (CR-3). The positive
//! cases take the bearer from an `EnvSecretStore` captured over
//! `SecretKey::ALL`, the composition root's shape (AC2.4).

use std::collections::BTreeMap;
use std::sync::Arc;

use maos_domain::ports::io_subsystem::{HttpExchange, IoSubsystemPort};
use maos_domain::ports::mcp::McpRequest;
use maos_domain::ports::{McpServer, SecretKey, SecretStore};
use maos_journey_test::{ConformantMcp, McpEra, McpRequestCapture, CONFORMANT_LEGACY_VERSION};
use maos_kernel_core::api::IoSubsystemAdapter;
use maos_mcp::transport::streamable_http::{StreamableHttpTransport, MODERN_PROTOCOL_VERSION};
use maos_mcp::transport::{McpTransport, McpTransportError};
use serde_json::json;

/// The canary bearer: it must reach the wire and nothing else (AC2.3).
const CANARY: &str = "mcp-bearer-canary-18-8";

/// The bearer as the composition root materializes it: an environment
/// captured over `SecretKey::ALL` into an `EnvSecretStore`.
fn bearer_from_secret_store() -> Option<String> {
    let environment = BTreeMap::from([("MAOS_MCP_CALENDAR_BEARER", CANARY)]);
    let store = maos_secrets::EnvSecretStore::new(SecretKey::ALL.into_iter().filter_map(|key| {
        environment
            .get(key.environment_variable())
            .map(|value| (key, value.to_string()))
    }));
    store
        .get(SecretKey::McpBearer(McpServer::Calendar))
        .unwrap()
}

fn client(server: &ConformantMcp, bearer: Option<String>) -> StreamableHttpTransport {
    StreamableHttpTransport::new(
        Arc::new(IoSubsystemAdapter::new()),
        server.url().to_string(),
        bearer,
    )
}

fn call(
    transport: &StreamableHttpTransport,
) -> Result<maos_domain::ports::mcp::McpResponse, McpTransportError> {
    transport.invoke(McpRequest::new(
        "calendar".into(),
        "calendar_events".into(),
        json!({"day": "tuesday"}),
    ))
}

/// A hand-built POST: exactly the headers given, nothing added.
fn raw_post(
    server: &ConformantMcp,
    body: serde_json::Value,
    headers: &[(&str, &str)],
) -> HttpExchange {
    IoSubsystemAdapter::new()
        .http_post_exchange(
            server.url(),
            body.to_string().as_bytes(),
            headers,
            "Mcp-Session-Id",
        )
        .expect("the fixture answers")
}

fn error_code(exchange: &HttpExchange) -> i64 {
    serde_json::from_slice::<serde_json::Value>(&exchange.body).unwrap()["error"]["code"]
        .as_i64()
        .expect("a JSON-RPC error body")
}

fn modern_tools_call(version: &str) -> serde_json::Value {
    json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {
        "name": "calendar_events", "arguments": {},
        "_meta": {"io.modelcontextprotocol/protocolVersion": version},
    }})
}

fn methods(requests: &[McpRequestCapture]) -> Vec<String> {
    requests
        .iter()
        .map(|request| {
            request.json()["method"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

/// No error text, `Debug` or captured request other than the bearer header
/// itself may carry the canary (AC2.3).
fn assert_no_canary_leak(error: Option<&McpTransportError>, requests: &[McpRequestCapture]) {
    if let Some(error) = error {
        assert!(
            !format!("{error} {error:?}").contains(CANARY),
            "error leaks the bearer: {error:?}"
        );
    }
    for request in requests {
        assert!(
            !String::from_utf8_lossy(&request.body).contains(CANARY),
            "body leaks the bearer"
        );
        for (name, value) in &request.headers {
            assert!(
                name == "authorization" || !value.contains(CANARY),
                "header {name} leaks the bearer"
            );
        }
    }
}

#[test]
fn modern_two_calls_succeed_without_a_handshake() {
    let server = ConformantMcp::start(McpEra::Modern, Some(CANARY));
    let transport = client(&server, bearer_from_secret_store());

    for _ in 0..2 {
        let response = call(&transport).expect("a modern server accepts the client");
        assert_eq!(response.attribution.tool, "calendar_events");
    }

    let requests = server.requests();
    assert_eq!(
        methods(&requests),
        ["tools/call", "tools/call"],
        "no initialize, no notification"
    );
    for request in &requests {
        assert_eq!(
            request.header("MCP-Protocol-Version"),
            Some(MODERN_PROTOCOL_VERSION)
        );
        assert_eq!(request.header("Mcp-Method"), Some("tools/call"));
        assert_eq!(request.header("Mcp-Name"), Some("calendar_events"));
        assert_eq!(request.header("Mcp-Session-Id"), None);
        assert_eq!(
            request.header("Authorization"),
            Some(format!("Bearer {CANARY}").as_str())
        );
        let meta = &request.json()["params"]["_meta"];
        assert_eq!(
            meta["io.modelcontextprotocol/protocolVersion"],
            MODERN_PROTOCOL_VERSION
        );
        assert_eq!(meta["io.modelcontextprotocol/clientInfo"]["name"], "maos");
    }
    assert_no_canary_leak(None, &requests);
}

#[test]
fn modern_fixture_rejects_bad_headers_and_the_client_surfaces_it_typed() {
    let server = ConformantMcp::start(McpEra::Modern, None);
    let named = [
        ("Mcp-Method", "tools/call"),
        ("Mcp-Name", "calendar_events"),
    ];

    // Hand-built: a missing MCP-Protocol-Version is 400 HeaderMismatch.
    let missing = raw_post(&server, modern_tools_call(MODERN_PROTOCOL_VERSION), &named);
    assert_eq!((missing.status, error_code(&missing)), (400, -32020));
    // Hand-built: header and `_meta` disagree — 400 HeaderMismatch.
    let mismatched = raw_post(
        &server,
        modern_tools_call(MODERN_PROTOCOL_VERSION),
        &[("MCP-Protocol-Version", "2025-11-25"), named[0], named[1]],
    );
    assert_eq!((mismatched.status, error_code(&mismatched)), (400, -32020));
    // Hand-built: a missing Mcp-Method or Mcp-Name is 400.
    for headers in [[named[0]], [named[1]]] {
        let mut headers = headers.to_vec();
        headers.push(("MCP-Protocol-Version", MODERN_PROTOCOL_VERSION));
        let rejected = raw_post(
            &server,
            modern_tools_call(MODERN_PROTOCOL_VERSION),
            &headers,
        );
        assert_eq!(rejected.status, 400, "{headers:?}");
    }

    // The real client: a modern server that does not support 2026-07-28
    // answers 400 UnsupportedProtocolVersion listing what it supports (the
    // spec's MUST, review TIA-7a) — a modern rejection without the legacy
    // version, so the client stays modern (no `initialize`), typed.
    server.set_modern_version("2099-01-01");
    match call(&client(&server, None)) {
        Err(McpTransportError::ModernRejection { code: -32022, body }) => {
            let body: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(body["error"]["data"]["supported"], json!(["2099-01-01"]));
            assert_eq!(body["error"]["data"]["requested"], MODERN_PROTOCOL_VERSION);
        }
        other => panic!("expected ModernRejection -32022, got {other:?}"),
    }
    assert!(!methods(&server.requests()).contains(&"initialize".to_string()));
}

#[test]
fn legacy_two_calls_share_one_initialize_and_one_session() {
    let server = ConformantMcp::start(McpEra::Legacy, Some(CANARY));
    let transport = client(&server, bearer_from_secret_store());

    for _ in 0..2 {
        let response = call(&transport).expect("a legacy server accepts the client");
        assert_eq!(response.attribution.tool, "calendar_events");
    }

    let requests = server.requests();
    // The modern probe (400, not a modern error) → handshake → two calls.
    assert_eq!(
        methods(&requests),
        [
            "tools/call",
            "initialize",
            "notifications/initialized",
            "tools/call",
            "tools/call"
        ]
    );
    let probe = &requests[0];
    assert_eq!(
        probe.header("MCP-Protocol-Version"),
        Some(MODERN_PROTOCOL_VERSION)
    );
    assert_eq!(
        requests[1].json()["params"]["protocolVersion"],
        CONFORMANT_LEGACY_VERSION
    );
    for request in &requests[2..] {
        assert_eq!(request.header("Mcp-Session-Id"), Some("legacy-session-1"));
        assert_eq!(
            request.header("MCP-Protocol-Version"),
            Some(CONFORMANT_LEGACY_VERSION)
        );
        assert_eq!(
            request.header("Authorization"),
            Some(format!("Bearer {CANARY}").as_str())
        );
    }
    assert_no_canary_leak(None, &requests);
}

#[test]
fn legacy_fixture_rejects_unsessioned_and_unknown_sessions_and_the_client_recovers_once() {
    let server = ConformantMcp::start(McpEra::Legacy, None);
    let legacy = [("MCP-Protocol-Version", CONFORMANT_LEGACY_VERSION)];
    let tools_call = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": "calendar_events", "arguments": {}}});

    // Hand-built: `tools/call` before `initialize` is 400; an unknown session is 404.
    let unsessioned = raw_post(&server, tools_call.clone(), &legacy);
    assert_eq!(unsessioned.status, 400);
    let unknown = raw_post(
        &server,
        tools_call,
        &[legacy[0], ("Mcp-Session-Id", "never-issued")],
    );
    assert_eq!(unknown.status, 404);

    // The real client: a session the server forgot is 404 → one
    // re-initialize and one retry.
    let transport = client(&server, None);
    call(&transport).expect("first legacy call");
    server.forget_sessions();
    call(&transport).expect("a forgotten session re-initializes once");
    let initializes = methods(&server.requests())
        .iter()
        .filter(|m| *m == "initialize")
        .count();
    assert_eq!(initializes, 2);

    // A server whose sessions expire on every `tools/call` (review TIA-3):
    // exactly one re-initialize and one retry, then typed — nothing more.
    server.set_sessions_expire(true);
    let before = server.requests().len();
    match call(&transport) {
        Err(McpTransportError::HttpStatus { status: 404, .. }) => {}
        other => panic!("expected HttpStatus 404 after one retry, got {other:?}"),
    }
    assert_eq!(
        methods(&server.requests()[before..]),
        [
            "tools/call",
            "initialize",
            "notifications/initialized",
            "tools/call"
        ]
    );
    let statuses: Vec<_> = server.requests()[before..]
        .iter()
        .map(|request| request.header("Mcp-Session-Id").map(str::to_owned))
        .collect();
    assert_ne!(
        statuses[0], statuses[3],
        "the retry carries the new session"
    );
}

#[test]
fn a_missing_or_wrong_bearer_is_unauthorized_on_both_eras() {
    for era in [McpEra::Modern, McpEra::Legacy, McpEra::DualEra] {
        let server = ConformantMcp::start(era, Some(CANARY));
        let hand_built = raw_post(&server, modern_tools_call(MODERN_PROTOCOL_VERSION), &[]);
        assert_eq!(hand_built.status, 401, "{era:?}: no Authorization header");

        for bearer in [None, Some("not-the-bearer".to_string())] {
            match call(&client(&server, bearer.clone())) {
                Err(error @ McpTransportError::Unauthorized) => {
                    assert_no_canary_leak(Some(&error), &server.requests());
                }
                other => panic!("{era:?} with {bearer:?}: expected Unauthorized, got {other:?}"),
            }
        }
        assert!(
            !methods(&server.requests()).contains(&"initialize".to_string()),
            "{era:?}: a 401 is not a legacy trigger"
        );
    }
}

#[test]
fn a_dual_era_server_is_spoken_to_modern() {
    let server = ConformantMcp::start(McpEra::DualEra, Some(CANARY));
    let transport = client(&server, bearer_from_secret_store());
    call(&transport).expect("first call");
    call(&transport).expect("second call");
    assert_eq!(methods(&server.requests()), ["tools/call", "tools/call"]);
}

#[test]
fn a_cached_legacy_origin_re_probes_once_when_the_server_turns_modern() {
    let server = ConformantMcp::start(McpEra::Legacy, None);
    let transport = client(&server, None);
    call(&transport).expect("legacy call caches the era");
    server.set_era(McpEra::Modern);
    let before = server.requests().len();
    call(&transport).expect("a modern-shaped rejection re-probes modern");
    call(&transport).expect("the re-probe cached modern");
    let requests = server.requests();
    let requests = &requests[before..];
    // Review TIA-6: one legacy attempt (rejected modern), one modern re-probe,
    // then the third call goes straight to modern.
    assert_eq!(
        methods(requests),
        ["tools/call", "tools/call", "tools/call"]
    );
    let shape = |request: &McpRequestCapture| {
        (
            request.header("MCP-Protocol-Version").map(str::to_owned),
            request.header("Mcp-Session-Id").is_some(),
        )
    };
    let modern = (Some(MODERN_PROTOCOL_VERSION.to_owned()), false);
    assert_eq!(
        requests.iter().map(shape).collect::<Vec<_>>(),
        [
            (Some(CONFORMANT_LEGACY_VERSION.to_owned()), true),
            modern.clone(),
            modern
        ]
    );
}

/// Review BH-2/EC-4/AA-1/TIA-1 — `MissingRequiredClientCapability` (−32021)
/// is a modern rejection: no legacy fallback, surfaced typed.
#[test]
fn a_missing_capability_rejection_stays_modern() {
    let server = ConformantMcp::start(McpEra::DualEra, None);
    server.set_required_capability(true);
    match call(&client(&server, None)) {
        Err(McpTransportError::ModernRejection { code: -32021, .. }) => {}
        other => panic!("expected ModernRejection -32021, got {other:?}"),
    }
    assert_eq!(methods(&server.requests()), ["tools/call"], "no initialize");
}

/// Review EC-5/AA-4/BH-8/TIA-10 — the era is judged on the full body: a
/// 5 KiB `UnsupportedProtocolVersion` (over the 4 KiB stored copy) stays modern.
#[test]
fn a_long_modern_rejection_body_stays_modern() {
    let server = ConformantMcp::start(McpEra::Modern, None);
    server.set_modern_version("2099-01-01");
    server.set_error_padding(5 * 1024);
    match call(&client(&server, None)) {
        Err(McpTransportError::ModernRejection { code: -32022, body }) => {
            assert_eq!(body.len(), 4096, "only the stored copy is truncated");
        }
        other => panic!("expected ModernRejection -32022, got {other:?}"),
    }
    assert_eq!(methods(&server.requests()), ["tools/call"], "no initialize");
}

/// Review ruling D-18-8-R2 (AA-5) — a `-32022` whose `data.supported` lists
/// the legacy version takes the legacy path, and the call succeeds there.
#[test]
fn an_unsupported_version_listing_the_legacy_one_takes_the_legacy_path() {
    let server = ConformantMcp::start(McpEra::DualEra, None);
    server.set_modern_version("2099-01-01");
    server.set_supported(&["2099-01-01", CONFORMANT_LEGACY_VERSION]);
    let transport = client(&server, None);
    call(&transport).expect("the legacy path serves the call");
    call(&transport).expect("the legacy verdict is cached");
    assert_eq!(
        methods(&server.requests()),
        [
            "tools/call",
            "initialize",
            "notifications/initialized",
            "tools/call",
            "tools/call"
        ]
    );
}

/// Review TIA-7b — a legacy server that issues no `Mcp-Session-Id` (the
/// spec's MAY): one handshake, then sessionless calls with the version.
#[test]
fn a_sessionless_legacy_server_is_spoken_to_without_a_session() {
    let server = ConformantMcp::start(McpEra::Legacy, None);
    server.set_issue_sessions(false);
    let transport = client(&server, None);
    call(&transport).expect("first sessionless call");
    call(&transport).expect("second sessionless call");
    let requests = server.requests();
    assert_eq!(
        methods(&requests),
        [
            "tools/call",
            "initialize",
            "notifications/initialized",
            "tools/call",
            "tools/call"
        ]
    );
    for request in &requests[2..] {
        assert_eq!(request.header("Mcp-Session-Id"), None);
        assert_eq!(
            request.header("MCP-Protocol-Version"),
            Some(CONFORMANT_LEGACY_VERSION)
        );
    }
}

/// Review TIA-7c — 2026-07-28 Streamable HTTP lets a server answer a POST
/// with `text/event-stream`: priming event, notification, a server request
/// reusing the id, and a multi-line `data:` response — on both eras.
#[test]
fn an_event_stream_answer_is_parsed_on_both_eras() {
    for era in [McpEra::Modern, McpEra::Legacy] {
        let server = ConformantMcp::start(era, None);
        server.set_sse(true);
        let transport = client(&server, None);
        for _ in 0..2 {
            let response = call(&transport).unwrap_or_else(|e| panic!("{era:?}: {e:?}"));
            assert_eq!(response.attribution.tool, "calendar_events");
            assert_eq!(
                response.attribution.transport_id,
                maos_domain::ports::mcp::McpTransportId::StreamableHttp
            );
            assert_eq!(response.content["content"][0]["text"], "ok");
        }
    }
}

/// Review BH-1/EC-1/AA-2 — a pasted `\r\n` around the bearer is trimmed and
/// the call succeeds with the exact bearer.
#[test]
fn a_bearer_with_surrounding_whitespace_is_trimmed() {
    let server = ConformantMcp::start(McpEra::Modern, Some(CANARY));
    call(&client(&server, Some(format!(" {CANARY}\r\n")))).expect("the trimmed bearer works");
    assert_eq!(
        server.requests()[0].header("Authorization"),
        Some(format!("Bearer {CANARY}").as_str())
    );
}

/// Review BH-1/EC-1/AA-2 — a bearer with an inner control or non-ASCII byte
/// would make the header validator echo the header line; it is refused typed
/// before any request, and the error carries no part of it.
#[test]
fn a_bearer_with_an_inner_control_or_non_ascii_byte_is_refused_without_leaking() {
    let server = ConformantMcp::start(McpEra::Modern, Some(CANARY));
    for bearer in [
        format!("{CANARY}\nx"),
        format!("{CANARY}é"),
        format!("a {CANARY}"),
    ] {
        match call(&client(&server, Some(bearer.clone()))) {
            Err(error @ McpTransportError::InvalidBearer) => {
                assert_no_canary_leak(Some(&error), &[]);
            }
            other => panic!("{bearer:?}: expected InvalidBearer, got {other:?}"),
        }
    }
    assert!(server.requests().is_empty(), "nothing reached the wire");
}

/// Review TIA-2 — the client HOLDS the canary and the server answers 404, a
/// non-modern 400 and a 500: no error text or `Debug` carries it.
#[test]
fn error_paths_never_carry_the_held_bearer() {
    let rpc_error = |code: i64| {
        json!({"jsonrpc": "2.0", "id": 1, "error": {"code": code, "message": "nope"}}).to_string()
    };
    for (status, expected) in [
        ("404 Not Found", 404u16),
        ("400 Bad Request", 400),
        ("500 Internal Server Error", 500),
    ] {
        let server = ConformantMcp::start(McpEra::Modern, Some(CANARY));
        server.set_forced(Some((status, rpc_error(-32600))));
        match call(&client(&server, bearer_from_secret_store())) {
            Err(error @ McpTransportError::HttpStatus { status, .. }) if status == expected => {
                assert_no_canary_leak(Some(&error), &server.requests());
            }
            other => panic!("{expected}: expected HttpStatus, got {other:?}"),
        }
    }
}
