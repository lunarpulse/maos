//! Story 10.5 AC2 — JetBrains ACP bridge scripted-NDJSON protocol test.
//!
//! Validates the reusable `maos-acp` server framing with `editor_id: "jetbrains"`.
//! The binary launch surface is covered by
//! `maos-bin/tests/jetbrains_acp_server.rs`.

use std::sync::Arc;

use maos_acp::AcpServer;
use maos_domain::halt::{HaltId, HaltResolver, Resolution, ResolveError};
use maos_domain::lifecycle::{LifecycleError, LifecycleReceipt, LifecycleResolver, LifecycleVerb};

struct StubLifecycleResolver;
impl LifecycleResolver for StubLifecycleResolver {
    fn resolve_verb(
        &self,
        spirit_id: &str,
        _verb: LifecycleVerb,
    ) -> Result<LifecycleReceipt, LifecycleError> {
        Err(LifecycleError::NotLoaded {
            spirit_id: spirit_id.into(),
        })
    }
}

struct StubHaltResolver;
impl HaltResolver for StubHaltResolver {
    fn resolve(&self, _halt_id: &HaltId, _resolution: Resolution) -> Result<(), ResolveError> {
        Ok(())
    }
}

/// Scripted JetBrains NDJSON conversation: session_start → lifecycle_verb →
/// halt_resolve → session_end. Uses exact wire format with session_id and
/// decision_id fields as 16-byte JSON arrays.
#[test]
fn jetbrains_acp_bridge_scripted_ndjson() {
    let server = AcpServer::new(Arc::new(StubLifecycleResolver), Arc::new(StubHaltResolver));
    let sessions = server.session_registry();

    // Session ID: [10,5,0,...] — 10.5 marker for traceability.
    // Decision ID: [0xAC,2,...] — AC2 marker.
    let input = concat!(
        r#"{"kind":"session_start","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"editor_id":"jetbrains","editor_version":"2024.3"}"#,
        "\n",
        r#"{"kind":"lifecycle_verb","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"decision_id":[172,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"verb":"load","spirit_id":"test-spirit-jb"}"#,
        "\n",
        r#"{"kind":"halt_resolve","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"decision_id":[172,2,0,0,0,0,0,0,0,0,0,0,0,0,0,1],"halt_id":"halt-jb-1","resolution":"approve"}"#,
        "\n",
        r#"{"kind":"session_end","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#,
        "\n",
    );

    let mut output = Vec::new();
    let mut server = AcpServer {
        lifecycle: Arc::new(StubLifecycleResolver),
        halts: Arc::new(StubHaltResolver),
        sessions,
    };

    server
        .run(input.as_bytes(), &mut output)
        .expect("server run must succeed");

    let output_str = String::from_utf8(output).unwrap();
    let responses: Vec<serde_json::Value> = output_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert!(
        responses.len() >= 4,
        "expected at least 4 responses, got {}: {:?}",
        responses.len(),
        responses
    );

    // 1. SessionReady — response to session_start
    assert_eq!(responses[0]["kind"], "session_ready");
    let kinds = responses[0]["supported_kinds"]
        .as_array()
        .expect("supported_kinds must be array");
    assert!(kinds.iter().any(|k| k == "lifecycle_verb"));
    assert!(kinds.iter().any(|k| k == "halt_resolve"));

    // 2. LifecycleReceipt — stub returns NotLoaded (error receipt expected)
    assert_eq!(responses[1]["kind"], "lifecycle_receipt");

    // 3. HaltReceipt — stub resolves OK
    assert_eq!(responses[2]["kind"], "halt_receipt");

    // 4. SessionTerminated — response to session_end
    assert_eq!(responses[3]["kind"], "session_terminated");
    assert!(
        responses[3]["duration_ns"].as_u64().unwrap_or(0) > 0,
        "duration_ns must be > 0"
    );
}
