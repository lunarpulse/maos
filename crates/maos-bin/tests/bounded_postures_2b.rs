#![forbid(unsafe_code)]

//! Machine-asserted claim boundaries for `j1-crosshost-2b`.

use std::net::SocketAddr;
use std::sync::Arc;

use maos_a2a_core::consent::ConsentAllowlists;
use maos_a2a_core::error::A2AError;
use maos_a2a_core::identity::{PeerCertFingerprint, PeerId};
use maos_a2a_core::transport::json_rpc::{A2AJsonRpcResponse, CODE_SPIRIT_RESTART_DETECTED};
use maos_a2a_core::{A2APeerConfig, A2AProfile, A2ARouterCore, InMemoryTofuPinStore, TofuPinStore};
use maos_a2a_tcp::PinnedFingerprint;
use maos_bin::delegation::{FROM_HOST, FROM_SPIRIT, RECIPIENT_SPIRIT, TO_HOST};
use maos_domain::frame::IacFrame;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{HostId, SpiritRole};
use orchestrator::{Orchestrator, DELEGATION_CONSENT_INTENT};

const ROUTER_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-a2a-core/src/router.rs"
));
const LOOPBACK_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-a2a/src/adapter.rs"
));
const TCP_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-a2a-tcp/src/transport.rs"
));
const JSON_RPC_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../maos-a2a-core/src/transport/json_rpc.rs"
));

fn code_window_after(source: &str, signature: &str, lines: usize) -> String {
    source
        .split_once(signature)
        .unwrap_or_else(|| panic!("missing source seam {signature}"))
        .1
        .lines()
        .take(lines)
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn peer_config(endpoint: &str) -> A2APeerConfig {
    A2APeerConfig {
        peer_id: PeerId::new(TO_HOST),
        endpoint: endpoint.to_string(),
        cert_fingerprint: PeerCertFingerprint {
            algo: "sha256".to_string(),
            hex: "00".repeat(32),
        },
        profile: A2AProfile::Loopback,
        allowlists: ConsentAllowlists {
            send_allowlist: vec![A2AIntent::new(DELEGATION_CONSENT_INTENT)],
            accept_allowlist: Vec::new(),
        },
        partition_timeout_secs: 30,
        consent_ttl_secs: 300,
    }
}

async fn pinned_core(config: A2APeerConfig) -> A2ARouterCore {
    let tofu = Arc::new(InMemoryTofuPinStore::new());
    tofu.pin_first_contact(
        &config.peer_id,
        &config.cert_fingerprint,
        &config.cert_fingerprint,
        1,
    )
    .await
    .expect("test peer can be pre-pinned");
    A2ARouterCore::new(vec![config], tofu)
}

fn delegation_frame() -> IacFrame {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let emitter = Orchestrator::new(FROM_SPIRIT);
    emitter
        .assign_frame_remote(
            1,
            0x2B00_0001,
            RECIPIENT_SPIRIT,
            SpiritRole::Worker,
            emitter.build_task_assign("write the workspace", "exit 0", None),
            IntentLineage::new(vec![A2AIntent::new(DELEGATION_CONSENT_INTENT)]),
            TO_HOST,
            FROM_HOST,
            A2AIntent::new(DELEGATION_CONSENT_INTENT),
        )
        .expect("production delegation shape is canonical")
}

#[test]
fn peer_pin_toml_requires_an_operator_provisioned_boot_nonce() {
    let missing_nonce = r#"
peer_id = "developer-remote-host"

[fingerprint]
hex = "0000000000000000000000000000000000000000000000000000000000000000"
"#;

    let error = toml::from_str::<PinnedFingerprint>(missing_nonce)
        .expect_err("a peer pin without boot_nonce must not deserialize");
    assert!(
        error.to_string().contains("boot_nonce"),
        "the schema failure must name the omitted control: {error}"
    );
}

#[test]
fn cross_host_nonce_path_cannot_inherit_the_loopback_zero_sentinel() {
    let restart_check = code_window_after(ROUTER_SOURCE, "async fn handle_intake_inner(", 85);
    let loopback_route = code_window_after(LOOPBACK_SOURCE, "async fn route_outbound(", 30);
    let tcp_route = code_window_after(TCP_SOURCE, "async fn route_outbound(", 35);

    assert!(
        restart_check.contains("if request.boot_nonce != 0"),
        "restart detection must remain explicitly gated by the documented wire sentinel"
    );
    assert!(
        loopback_route.contains("self.core.prepare_outbound(frame, peer, 0).await?"),
        "only the in-process loopback router may stamp the zero sentinel"
    );
    assert!(
        tcp_route.contains(".prepare_outbound(frame, peer, self.own_boot_nonce)"),
        "the live transport must stamp its process boot nonce"
    );
    assert!(
        !tcp_route.contains("prepare_outbound(frame, peer, 0)"),
        "the live transport must never disable restart detection with the loopback sentinel"
    );
}

#[test]
fn restart_nack_is_a_pin_failure_never_a_transport_failure() {
    let core = A2ARouterCore::new(Vec::new(), Arc::new(InMemoryTofuPinStore::new()));
    let peer = HostId(TO_HOST.to_string());
    let error = core
        .interpret_response(
            &peer,
            A2AJsonRpcResponse::nack(7, CODE_SPIRIT_RESTART_DETECTED, "nonce changed"),
        )
        .expect_err("a restart NACK must reject the sender");

    assert!(
        matches!(
            error,
            A2AError::PinInvalidated {
                awaiting_repin: true,
                ..
            }
        ),
        "restart detection must surface the permanent pin failure: {error:?}"
    );
    assert!(
        !matches!(error, A2AError::TransportFailed(_)),
        "a restart NACK must never be presented as a retryable transport failure"
    );
}

#[test]
fn response_code_census_records_the_post_repair_scope_wall() {
    // Before `j1-crosshost-2b` added the restart arm the census was 9 typed codes
    // and 7 fall-throughs; 2b's arm made it 10 and 6.
    //
    // `j1-crosshost-2c` AC3.2 typed the two that 2b filed against it in-source:
    // `CODE_INTERNAL` (which 2b itself newly emits at `router.rs:1371` and
    // `:1549`) and `CODE_TIMEOUT` (which 2c's own bounded write path produces).
    // Both used to fall through to `TransportFailed`, making a dropped-receiver
    // internal NACK byte-identical at the sender to a genuine wire partition —
    // so AC3's fault windows could not tell apart the faults they inject.
    //
    // The census is therefore **12 typed, 4 fall-through**. The remaining four
    // are not reachable from anything 2b or 2c emits, and a nine-arm refactor
    // inside a frozen crate remains a different story.
    const ALL_CODES: [&str; 16] = [
        "CODE_PARSE_ERROR",
        "CODE_INVALID_REQUEST",
        "CODE_METHOD_NOT_FOUND",
        "CODE_INTENT_DENIED",
        "CODE_PIN_MISMATCH_NOT_PINNED",
        "CODE_CONSENT_EXPIRED",
        "CODE_SPIRIT_RESTART_DETECTED",
        "CODE_TIMEOUT",
        "CODE_FRAME_TOO_LARGE",
        "CODE_PEER_IDENTITY_MISMATCH",
        "CODE_CONSENT_GRANTER_MISMATCH",
        "CODE_CONSENT_UNCLASSIFIED",
        "CODE_TEAM_IDENTITY_MISMATCH",
        "CODE_CROSSING_SOURCE_TEAM_UNBOUND",
        "CODE_CROSS_TEAM_CROSSING_REFUSED",
        "CODE_INTERNAL",
    ];
    const TYPED_CODES: [&str; 12] = [
        "CODE_INTENT_DENIED",
        "CODE_PIN_MISMATCH_NOT_PINNED",
        "CODE_CONSENT_EXPIRED",
        "CODE_SPIRIT_RESTART_DETECTED",
        "CODE_PEER_IDENTITY_MISMATCH",
        "CODE_CONSENT_GRANTER_MISMATCH",
        "CODE_CONSENT_UNCLASSIFIED",
        "CODE_TEAM_IDENTITY_MISMATCH",
        "CODE_CROSSING_SOURCE_TEAM_UNBOUND",
        "CODE_CROSS_TEAM_CROSSING_REFUSED",
        // j1-crosshost-2c AC3.2
        "CODE_INTERNAL",
        "CODE_TIMEOUT",
    ];
    const FALL_THROUGHS: [&str; 4] = [
        "CODE_PARSE_ERROR",
        "CODE_INVALID_REQUEST",
        "CODE_METHOD_NOT_FOUND",
        "CODE_FRAME_TOO_LARGE",
    ];

    let definitions = JSON_RPC_SOURCE
        .lines()
        .filter(|line| line.trim_start().starts_with("pub const CODE_"))
        .collect::<Vec<_>>();
    assert_eq!(
        definitions.len(),
        ALL_CODES.len(),
        "the protocol defines 16 codes"
    );
    for code in ALL_CODES {
        assert!(
            definitions.iter().any(|line| line.contains(code)),
            "missing protocol code {code}"
        );
    }

    let response_mapping = code_window_after(ROUTER_SOURCE, "pub fn interpret_response(", 230);
    let mapped_arms = response_mapping
        .lines()
        .filter(|line| line.trim_start().starts_with("CODE_") && line.contains("=>"))
        .collect::<Vec<_>>();
    assert_eq!(
        mapped_arms.len(),
        TYPED_CODES.len(),
        "interpret_response must type exactly 12 of the 16 protocol codes"
    );
    for code in TYPED_CODES {
        assert!(
            mapped_arms.iter().any(|line| line.contains(code)),
            "{code} must have a typed response arm"
        );
    }
    for code in FALL_THROUGHS {
        assert!(
            !mapped_arms.iter().any(|line| line.contains(code)),
            "{code} must remain outside this story's response mapping"
        );
    }
}

#[tokio::test]
async fn explicit_granter_expiry_survives_outbound_preparation() {
    const GRANTER_EXPIRY_NS: u64 = 42_000_000_000;

    let core = pinned_core(peer_config("tls://127.0.0.1:7451")).await;
    let mut frame = delegation_frame();
    frame
        .consent_envelope
        .as_mut()
        .expect("production delegation frame carries a consent envelope")
        .valid_until_ns = Some(GRANTER_EXPIRY_NS);

    let (request, _, _) = core
        .prepare_outbound(frame, &HostId(TO_HOST.to_string()), 9)
        .await
        .expect("an allowlisted, pre-pinned delegation frame is outbound-admitted");
    assert_eq!(
        request.params.consent_envelope.unwrap().valid_until_ns,
        Some(GRANTER_EXPIRY_NS),
        "the transitional transport TTL must not overwrite a granter-authoritative expiry"
    );
}

#[test]
fn hostname_config_validates_but_the_live_dial_is_ip_only() {
    let hostname_endpoint = "tls://host-b.internal:7443";
    let config = peer_config(hostname_endpoint);
    config
        .validate()
        .expect("the current peer schema admits a hostname endpoint");

    let host_port = hostname_endpoint.strip_prefix("tls://").unwrap();
    assert!(
        host_port.parse::<SocketAddr>().is_err(),
        "a hostname cannot reach the IP-only live dial path"
    );

    let dial_addr = code_window_after(TCP_SOURCE, "fn dial_addr(&self", 12);
    assert!(
        dial_addr.contains("rest.parse::<SocketAddr>()"),
        "the live transport must parse the endpoint directly as SocketAddr, not resolve DNS"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// j1-crosshost-2b §A6 review P8 (AC4.1) — the LIVE-path half of the boot-nonce
// boundary. The tests above prove the wire stamping; these prove the transport
// itself REFUSES the zero sentinel at bind time, so the first cross-host path
// cannot ship with NFR-Rel-6 restart detection structurally dead. The refusal
// is checked before any identity work, so no certs are needed to reach it.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn cross_host_bind_refuses_the_zero_own_boot_nonce() {
    let config = maos_a2a_tcp::TcpA2AConfig {
        listen_addr: "127.0.0.1:0".parse().expect("loopback listen addr"),
        own_cert_chain: std::path::PathBuf::from("/nonexistent/dummy.pem"),
        own_private_key: std::path::PathBuf::from("/nonexistent/dummy.key"),
        peer_pins: vec![],
        handshake_timeout: std::time::Duration::from_secs(30),
        ca_roots: None,
    };
    let error = maos_a2a_tcp::TcpA2ATransport::bind_with_intake_sink(
        config,
        vec![],
        0, // the loopback sentinel — MUST be refused on the cross-host path
        maos_a2a_tcp::TcpTimeouts::production(std::time::Duration::from_secs(30)),
        maos_a2a_core::HandshakeRetryPolicy::default(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .err()
    .expect("bind must refuse boot_nonce = 0 before any identity work");
    assert!(
        error.to_string().contains("boot_nonce = 0"),
        "the refusal must name the sentinel: {error}"
    );
}

#[tokio::test]
async fn cross_host_bind_refuses_a_zero_peer_pin_nonce() {
    let zero_pin = toml::from_str::<PinnedFingerprint>(
        r#"
peer_id = "founder-loop-host"
boot_nonce = 0

[fingerprint]
hex = "0000000000000000000000000000000000000000000000000000000000000000"
"#,
    )
    .expect(
        "an explicit zero parses — the schema requires presence, the transport refuses the value",
    );
    let config = maos_a2a_tcp::TcpA2AConfig {
        listen_addr: "127.0.0.1:0".parse().expect("loopback listen addr"),
        own_cert_chain: std::path::PathBuf::from("/nonexistent/dummy.pem"),
        own_private_key: std::path::PathBuf::from("/nonexistent/dummy.key"),
        peer_pins: vec![zero_pin],
        handshake_timeout: std::time::Duration::from_secs(30),
        ca_roots: None,
    };
    let error = maos_a2a_tcp::TcpA2ATransport::bind_with_intake_sink(
        config,
        vec![],
        0x2B_B, // own nonce is fine; the PIN carries the sentinel
        maos_a2a_tcp::TcpTimeouts::production(std::time::Duration::from_secs(30)),
        maos_a2a_core::HandshakeRetryPolicy::default(),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .err()
    .expect("bind must refuse a zero-nonce peer pin");
    assert!(
        error.to_string().contains("boot_nonce = 0"),
        "the refusal must name the sentinel: {error}"
    );
}
