//! Live-leg fix (run 43239794859) — the Anthropic driver over the REAL
//! `IoSubsystemAdapter` against a local fake HTTP server.
//!
//! A 400 `invalid_request_error` must reach the caller as a typed
//! `ProviderRejected` carrying the API's message (it used to be a body-less
//! `transport error ... status 400`), and the router must NOT walk the fallback
//! chain for it. 401 is also terminal; 429 and 5xx stay retryable.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceOptions, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_kernel_core::api::IoSubsystemAdapter;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_providers::{AnthropicProvider, Provider, ProviderError};

const KEY: &str = "sk-ant-api03-LIVE-LEG-SECRET";
const BODY_400: &str = r#"{"type":"error","error":{"type":"invalid_request_error","message":"`temperature` may not be set for this model"}}"#;

/// Serve exactly one request with `status` and `body`; the request headers and
/// body the server saw come back through the join handle.
fn serve_once(status: &'static str, body: &'static str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut seen = String::new();
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line.trim_end().is_empty() {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap();
            }
            seen.push_str(&line);
        }
        let mut request_body = vec![0u8; content_length];
        reader.read_exact(&mut request_body).unwrap();
        seen.push_str(&String::from_utf8_lossy(&request_body));
        let mut stream = stream;
        let head = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(body.as_bytes());
        let _ = stream.flush();
        seen
    });
    (url, handle)
}

fn request() -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0u8; 64]),
        "Answer with one word.".into(),
        InferenceOptions {
            max_tokens: 8,
            temperature: Some(0.0),
            model_id: Some("claude-sonnet-5-5".into()),
        },
        None,
        vec!["secondary".into()],
    )
}

fn anthropic(url: String) -> Arc<dyn Provider> {
    Arc::new(
        AnthropicProvider::with_api_key(
            Arc::new(IoSubsystemAdapter::new()),
            url,
            "claude-haiku-4-5-20251001".into(),
            KEY.into(),
        )
        .expect("construct anthropic provider"),
    )
}

struct Secondary(AtomicUsize);
impl Provider for Secondary {
    fn complete(&self, _: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(InferenceResponse {
            text: "from-secondary".into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "secondary".into(),
                endpoint_url: "http://secondary".into(),
                model_id: None,
            },
        })
    }
}

/// Route `req` through a router whose primary is the Anthropic driver at `url`
/// and whose only fallback counts its calls.
fn route(url: String) -> (Result<InferenceResponse, ProviderError>, usize) {
    let secondary = Arc::new(Secondary(AtomicUsize::new(0)));
    let mut providers: BTreeMap<String, Arc<dyn Provider>> = BTreeMap::new();
    providers.insert("anthropic".into(), anthropic(url));
    providers.insert("secondary".into(), secondary.clone());
    let router = MultiProviderRouter::new(providers, Some("anthropic".into()));
    let result = router.dispatch_with_fallback("anthropic", &["secondary".into()], &request());
    (result, secondary.0.load(Ordering::SeqCst))
}

#[test]
fn a_400_surfaces_the_api_message_and_is_not_retried() {
    let (url, server) = serve_once("400 Bad Request", BODY_400);
    let (result, fallback_calls) = route(url);
    let seen = server.join().unwrap();

    let Err(ProviderError::ProviderRejected { status, body }) = result else {
        panic!("a 400 must be a typed ProviderRejected, got {result:?}");
    };
    assert_eq!(status, 400);
    assert!(body.contains("invalid_request_error"), "{body}");
    assert!(body.contains("`temperature` may not be set"), "{body}");
    assert!(!body.contains(KEY));
    assert_eq!(fallback_calls, 0, "a 400 must not walk the fallback chain");

    // The wire request is the judge shape; the key travels only in the header.
    assert!(seen.contains("x-api-key: "), "{seen}");
    assert!(seen.contains(r#""model":"claude-sonnet-5-5""#), "{seen}");
    assert!(seen.contains(r#""max_tokens":8"#), "{seen}");
    // Sonnet 5.5 rejects sampling parameters; the requested 0.0 is dropped and
    // up-front thinking is switched off (anthropic-5-5-docs.md).
    assert!(!seen.contains("temperature"), "{seen}");
    assert!(
        seen.contains(r#""thinking":{"type":"between_tools"}"#),
        "{seen}"
    );
}

/// A rejection that echoes the credential it was sent must not leak it.
#[test]
fn a_401_that_echoes_the_key_never_leaks_it() {
    const ECHO: &str = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key: sk-ant-api03-LIVE-LEG-SECRET"}}"#;
    assert!(ECHO.contains(KEY));
    let (url, server) = serve_once("401 Unauthorized", ECHO);
    let (result, fallback_calls) = route(url);
    let seen = server.join().unwrap();
    assert!(
        seen.contains(KEY),
        "the key must travel in the request header"
    );
    let error = result.expect_err("a 401 is terminal");
    let ProviderError::ProviderRejected { status, body } = &error else {
        panic!("a 401 must be a typed ProviderRejected, got {error:?}");
    };
    assert_eq!(*status, 401);
    assert_eq!(
        body, "[body withheld: credential echo detected]",
        "an echoing body is withheld whole, status kept"
    );
    for rendered in [error.to_string(), format!("{error:?}"), body.clone()] {
        assert!(!rendered.contains(KEY), "{rendered}");
        assert!(!rendered.contains("LIVE-LEG-SECRET"), "{rendered}");
    }
    assert_eq!(fallback_calls, 0);
}

#[test]
fn a_401_is_terminal_and_carries_the_api_message() {
    let (url, server) = serve_once(
        "401 Unauthorized",
        r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#,
    );
    let (result, fallback_calls) = route(url);
    server.join().unwrap();
    let Err(ProviderError::ProviderRejected { status, body }) = result else {
        panic!("a 401 must be a typed ProviderRejected, got {result:?}");
    };
    assert_eq!(status, 401);
    assert!(body.contains("authentication_error"), "{body}");
    assert_eq!(fallback_calls, 0);
}

#[test]
fn a_429_stays_retryable_and_walks_to_the_fallback() {
    let (url, server) = serve_once(
        "429 Too Many Requests",
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#,
    );
    let (result, fallback_calls) = route(url);
    server.join().unwrap();
    assert_eq!(result.expect("fallback serves it").text, "from-secondary");
    assert_eq!(fallback_calls, 1);
}

#[test]
fn a_5xx_stays_retryable_and_walks_to_the_fallback() {
    for status in ["500 Internal Server Error", "529 Overloaded"] {
        let (url, server) = serve_once(
            status,
            r#"{"type":"error","error":{"type":"api_error","message":"boom"}}"#,
        );
        let (result, fallback_calls) = route(url);
        server.join().unwrap();
        assert_eq!(result.expect("fallback serves it").text, "from-secondary");
        assert_eq!(fallback_calls, 1, "{status}");
    }
}

#[test]
fn a_refused_connection_stays_a_retryable_transport_error() {
    // Bind then drop so the port is closed.
    let url = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let (result, fallback_calls) = route(url);
    assert_eq!(result.expect("fallback serves it").text, "from-secondary");
    assert_eq!(fallback_calls, 1);
}
