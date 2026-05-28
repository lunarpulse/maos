#![forbid(unsafe_code)]

//! Gateway Dispatcher integration tests — Story 6.5 / FR54.

use maos_kernel_core::orchestrator::{
    gateway_dispatcher::{GatewayDispatcher, GatewaySubmoduleRegistry},
    echo_gateway::EchoGatewayFactory,
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

fn make_gateways_section() -> GatewaysSection {
    GatewaysSection {
        entries: vec![make_entry("echo-1")],
    }
}

#[tokio::test]
async fn gateway_dispatcher_admit_single_spirit() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(1, "test-spirit-1", "principal-1", &make_gateways_section())
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(1, "test-spirit-1").await;
    assert_eq!(record.spirit_pid, 1);
    assert_eq!(record.gateways.len(), 1);
    assert_eq!(record.gateways[0].gateway_id, "echo-1");
}

#[tokio::test]
async fn gateway_dispatcher_unload_returns_record() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(42, "spirit-42", "principal-42", &make_gateways_section())
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(42, "spirit-42").await;
    assert_eq!(record.spirit_pid, 42);
    assert_eq!(record.gateways.len(), 1);
    assert!(
        matches!(
            record.gateways[0].disconnect_outcome,
            maos_domain::frame::DisconnectOutcome::Clean
                | maos_domain::frame::DisconnectOutcome::Timeout
        )
    );
}

#[tokio::test]
async fn gateway_dispatcher_admit_multiple_gateways() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    let gateways = GatewaysSection {
        entries: vec![make_entry("gw-a"), make_entry("gw-b")],
    };
    dispatcher
        .admit_spirit_gateways(7, "multi-gw", "principal-7", &gateways)
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(7, "multi-gw").await;
    assert_eq!(record.gateways.len(), 2);
}

#[tokio::test]
async fn gateway_dispatcher_unregistered_type_fails_admission() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    let gateways = GatewaysSection {
        entries: vec![GatewayEntry {
            id: "tg-1".into(),
            gateway_type: GatewayType::Telegram,
            auth_secret_ref: "secret:tg:key".into(),
            inbound_allowlist: vec![],
            outbound_allowlist: vec![],
            on_inbound: maos_manifest::OnInboundHook::OnFrame,
            reconnect_backoff_secs: 5,
            max_message_bytes: 4096,
        }],
    };
    let result = dispatcher
        .admit_spirit_gateways(5, "tg-only", "principal-5", &gateways)
        .await;
    assert!(result.is_err());
    let record = dispatcher.unload_spirit_gateways(5, "tg-only").await;
    assert!(record.gateways.is_empty());
}

#[tokio::test]
async fn gateway_dispatcher_no_factory_fails_admission() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    let dispatcher = GatewayDispatcher::new(registry);

    let result = dispatcher
        .admit_spirit_gateways(3, "no-factory", "principal-3", &make_gateways_section())
        .await;
    assert!(result.is_err());
    let record = dispatcher.unload_spirit_gateways(3, "no-factory").await;
    assert!(record.gateways.is_empty());
}

#[tokio::test]
async fn gateway_dispatcher_unload_nonexistent_spirit() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    let dispatcher = GatewayDispatcher::new(registry);

    let record = dispatcher.unload_spirit_gateways(99, "spirit-99").await;
    assert!(record.gateways.is_empty());
    assert_eq!(record.spirit_pid, 99);
}

#[tokio::test]
async fn gateway_dispatcher_admit_two_spirits_isolated() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    let mut gw_b = make_gateways_section();
    gw_b.entries[0].id = "echo-b".into();

    dispatcher
        .admit_spirit_gateways(10, "spirit-a", "principal-a", &make_gateways_section())
        .await
        .unwrap();
    dispatcher
        .admit_spirit_gateways(11, "spirit-b", "principal-b", &gw_b)
        .await
        .unwrap();

    let record_a = dispatcher.unload_spirit_gateways(10, "spirit-a").await;
    assert_eq!(record_a.gateways.len(), 1);
    assert_eq!(record_a.gateways[0].gateway_id, "echo-1");

    let record_b = dispatcher.unload_spirit_gateways(11, "spirit-b").await;
    assert_eq!(record_b.gateways.len(), 1);
    assert_eq!(record_b.gateways[0].gateway_id, "echo-b");
}

#[tokio::test]
async fn gateway_dispatcher_empty_gateways_section() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(20, "empty", "principal-20", &GatewaysSection::default())
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(20, "empty").await;
    assert!(record.gateways.is_empty());
}

#[tokio::test]
async fn gateway_dispatcher_duplicate_gateway_id_rejected() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(30, "dup-test", "principal-30", &make_gateways_section())
        .await
        .unwrap();

    let result = dispatcher
        .admit_spirit_gateways(30, "dup-test", "principal-30", &make_gateways_section())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn gateway_dispatcher_gateway_type_in_uninstall_record() {
    init_clock();
    let registry = Arc::new(GatewaySubmoduleRegistry::new());
    registry.register(GatewayType::Echo, Arc::new(EchoGatewayFactory));
    let dispatcher = GatewayDispatcher::new(registry);

    dispatcher
        .admit_spirit_gateways(50, "type-test", "principal-50", &make_gateways_section())
        .await
        .unwrap();
    let record = dispatcher.unload_spirit_gateways(50, "type-test").await;
    assert_eq!(record.gateways[0].gateway_type, "echo");
}
