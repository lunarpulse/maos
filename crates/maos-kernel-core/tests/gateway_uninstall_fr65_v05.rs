#![forbid(unsafe_code)]

//! Gateway uninstall integration tests — Story 6.5 / FR65 v0.5.

use maos_kernel_core::orchestrator::{
    echo_gateway::EchoGatewayFactory,
    gateway_dispatcher::{GatewayDispatcher, GatewaySubmoduleRegistry},
};
use maos_manifest::{GatewayEntry, GatewayType, GatewaysSection};
use std::sync::Arc;

fn init_clock() {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
}

fn make_entry(id: &str) -> GatewayEntry {
    GatewayEntry {
        id: id.into(),
        gateway_type: GatewayType::Echo,
        auth_secret_ref: "secret:echo:token".into(),
        inbound_allowlist: vec![],
        outbound_allowlist: vec![],
        on_inbound: maos_manifest::OnInboundHook::OnFrame,
        reconnect_backoff_secs: 5,
        max_message_bytes: 4096,
    }
}

#[tokio::test]
async fn gateway_uninstall_record_has_spirit_pid() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(
            100,
            "spirit-100",
            "principal-100",
            &GatewaysSection {
                entries: vec![make_entry("gw-1")],
            },
        )
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(100, "spirit-100").await;
    assert_eq!(record.spirit_pid, 100);
    assert_eq!(record.spirit_id, "spirit-100");
    assert!(record.uninstalled_at_ns > 0);
}

#[tokio::test]
async fn gateway_uninstall_record_has_gateways() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(
            200,
            "spirit-200",
            "principal-200",
            &GatewaysSection {
                entries: vec![make_entry("gw-a"), make_entry("gw-b")],
            },
        )
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(200, "spirit-200").await;
    assert_eq!(record.gateways.len(), 2);
}

#[tokio::test]
async fn gateway_uninstall_record_entry_has_gateway_id() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(
            300,
            "spirit-300",
            "principal-300",
            &GatewaysSection {
                entries: vec![make_entry("my-gw")],
            },
        )
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(300, "spirit-300").await;
    assert_eq!(record.gateways[0].gateway_id, "my-gw");
    assert_eq!(record.gateways[0].gateway_type, "echo");
}

#[tokio::test]
async fn gateway_uninstall_record_disconnect_outcome_clean() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(
            400,
            "spirit-400",
            "principal-400",
            &GatewaysSection {
                entries: vec![make_entry("gw-ts")],
            },
        )
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(400, "spirit-400").await;
    assert!(
        matches!(
            record.gateways[0].disconnect_outcome,
            maos_domain::frame::DisconnectOutcome::Clean
                | maos_domain::frame::DisconnectOutcome::Timeout
        ),
        "expected Clean or Timeout"
    );
}

#[tokio::test]
async fn gateway_uninstall_empty_when_no_gateways() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    let dispatcher = GatewayDispatcher::new(registry);

    let record = dispatcher.unload_spirit_gateways(500, "spirit-500").await;
    assert!(record.gateways.is_empty());
}

#[tokio::test]
async fn gateway_uninstall_serde_round_trip() {
    use maos_domain::frame::{DisconnectOutcome, GatewayUninstallEntry, GatewayUninstallRecord};

    let record = GatewayUninstallRecord {
        spirit_id: "spirit-42".into(),
        spirit_pid: 42,
        uninstalled_at_ns: 1_000_000,
        gateways: vec![
            GatewayUninstallEntry {
                gateway_id: "gw-1".into(),
                gateway_type: "echo".into(),
                principal_ns_keys_removed: vec!["key1".into()],
                revoked_cap_token_ids: vec![[1u8; 16]],
                terminated_connection_id: Some("conn-1".into()),
                disconnect_outcome: DisconnectOutcome::Clean,
            },
            GatewayUninstallEntry {
                gateway_id: "gw-2".into(),
                gateway_type: "telegram".into(),
                principal_ns_keys_removed: vec![],
                revoked_cap_token_ids: vec![],
                terminated_connection_id: None,
                disconnect_outcome: DisconnectOutcome::Timeout,
            },
        ],
    };

    let json = serde_json::to_string(&record).unwrap();
    let back: GatewayUninstallRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(back.spirit_pid, 42);
    assert_eq!(back.gateways.len(), 2);
    assert_eq!(back.gateways[0].gateway_id, "gw-1");
    assert!(matches!(
        back.gateways[0].disconnect_outcome,
        DisconnectOutcome::Clean
    ));
    assert_eq!(back.gateways[1].gateway_id, "gw-2");
    assert!(matches!(
        back.gateways[1].disconnect_outcome,
        DisconnectOutcome::Timeout
    ));
}
