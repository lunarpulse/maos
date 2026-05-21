#![forbid(unsafe_code)]

//! Integration test: KernelCtx routes Spirit-side calls into Epic-4
//! adapters (AC5). Verifies the builder pattern and adapter wiring.

use std::sync::Arc;

use maos_kernel_core::scheduler::kernel_ctx::KernelCtx;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;

#[test]
fn kernel_ctx_builder_wires_all_adapters() {
    let mut ctx = maos_spirit_abi::ctx::Ctx::mock();
    let kctx = KernelCtx::new(&mut ctx)
        .with_memory_manager(Arc::new(
            {
                let tmp = tempfile::TempDir::new().unwrap();
                let db_path = tmp.path().join("audit.db");
                let memory_root = tmp.path().join("memory");
                maos_kernel_core::memory::MemoryManagerAdapter::new(
                    Arc::new(maos_kernel_core::memory::private::PrivateMemoryStore::new(memory_root, 4)),
                    Arc::new(maos_kernel_core::memory::shared::SharedMemoryStore::open(&db_path).unwrap()),
                    Arc::new(maos_kernel_core::memory::principal::PrincipalNamespaceIndex::open(&db_path).unwrap()),
                    Arc::new(maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0)),
                )
            }
        ))
        .with_capability(Arc::new(
            maos_kernel_core::capability::CapabilityRegistryAdapter::new(
                Arc::new(maos_kernel_core::api::RingCryptoProvider),
                maos_kernel_core::capability::cap_tokens::Ed25519SigningKey::new([0u8; 32]),
                0,
                Arc::new(maos_kernel_core::capability::cap_policy::PolicyTable::new()),
                maos_kernel_core::capability::cap_audit::channel().0,
                maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
                Arc::new(maos_kernel_core::capability::WorkingMemoryStore::new()),
                Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
            )
        ))
        .with_iac(Arc::new(
            maos_kernel_core::iac::IacBusAdapter::new(
                Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(IacRtMetrics::new()))),
                Arc::new(maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0)),
            )
        ))
        .with_halt_registry(Arc::new(maos_kernel_core::halt::HaltRegistry::new()));

    // Verify each wired adapter is reachable.
    assert!(kctx.memory().is_some(), "memory_manager must be wired");
    assert!(kctx.capability.is_some(), "capability must be wired");
    assert!(kctx.iac.is_some(), "iac must be wired");
    assert!(kctx.halt_registry.is_some(), "halt_registry must be wired");
}

#[test]
fn kernel_ctx_default_builder_has_none() {
    let mut ctx = maos_spirit_abi::ctx::Ctx::mock();
    let kctx = KernelCtx::new(&mut ctx);

    assert!(kctx.memory_manager.is_none());
    assert!(kctx.capability.is_none());
    assert!(kctx.iac.is_none());
    assert!(kctx.halt_registry.is_none());
    assert!(kctx.log_recall.is_none());
    assert!(kctx.distillate_writer.is_none());
    assert!(kctx.self_telemetry.is_none());
    assert!(kctx.working_memory_orchestrator.is_none());
}
