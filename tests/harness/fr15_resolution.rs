#![allow(dead_code)]

//! Story 18-2 AC4.5 — FR15's three resolution pathways for one shipped class
//! (`prd/functional-requirements.md:49`), shared by the Butler and Researcher
//! witnesses.
//!
//! One corpus row halts through the kernel path
//! (`WorkingMemoryOrchestrator::process_scalar_write` under the class's
//! `[epistemic_policy]`), once per kind, each in a fresh world. Each halt is
//! resolved through the operator door's composition — `HaltFlow` over the
//! kernel `KernelHaltResolver` (which transitions `HaltRegistry::resolve`)
//! and the Transparency Log's `HaltJournal` — and the resolution must be
//! journaled to the Approval Decision Log with its reasoning chain.

use std::sync::Arc;

use maos_director_surface::halt_ui::HaltFlow;
use maos_director_surface::notification::NotificationDispatcher;
use maos_domain::halt::{HaltJournal, HaltState, Resolution};
use maos_kernel_core::capability::cap_policy::PolicyTable;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::working_memory::orchestrator::WorkingMemoryOrchestrator;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::halt::{HaltRegistry, KernelHaltResolver, OutputMarkerRegistry};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::security::manifest::EpistemicPolicySection;
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

const PID: u32 = 7;
const BOOT_NONCE: u64 = 0x18_02_F15;

/// The three FR15 kinds, the label each journals and the terminal state.
fn kinds() -> [(Resolution, &'static str, HaltState, &'static str); 3] {
    [
        (
            Resolution::provided_context("the director supplied the missing context").unwrap(),
            "provided_context",
            HaltState::Resumed,
            "provided_context: the director supplied the missing context",
        ),
        (
            Resolution::AcceptedHalt,
            "accepted_halt",
            HaltState::Terminated,
            "accepted_halt",
        ),
        (
            Resolution::AuthorizedOverride {
                operator_policy_ref: "policy://fr15/18-2".into(),
            },
            "authorized_override",
            HaltState::Overridden,
            "authorized_override: operator_policy_ref=policy://fr15/18-2",
        ),
    ]
}

/// Halt `scalar` through the kernel path and resolve it once per FR15 kind.
pub fn resolve_each_kind(
    spirit_id: &str,
    policy: &EpistemicPolicySection,
    (tag, value, derived_from): (&str, f64, String),
) {
    for (resolution, label, terminal, reasoning) in kinds() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db = tmp.path().join("audit.db");
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(BOOT_NONCE));
        let registry = Arc::new(HaltRegistry::new());
        let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(RingCryptoProvider),
            Ed25519SigningKey::new([0u8; 32]),
            BOOT_NONCE,
            Arc::new(PolicyTable::new()),
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        let orchestrator = Arc::new(WorkingMemoryOrchestrator::new(
            Arc::clone(&capability),
            Arc::clone(&registry),
        ));
        let journal = JournalAdapter::open(&tmp.path().join("journal.ndjson")).unwrap();
        let receipt = orchestrator
            .process_scalar_write(
                &tl,
                &journal,
                PID,
                spirit_id,
                BOOT_NONCE,
                tag,
                value,
                &derived_from,
                policy,
            )
            .unwrap()
            .expect("the row halts through the kernel path");
        let halt_id = receipt.halt_id;
        assert_eq!(
            registry.lookup_state(&halt_id),
            Some(HaltState::PendingResolution)
        );

        let memory = Arc::new(MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(tmp.path().join("memory"), 4 * 1024)),
            Arc::new(SharedMemoryStore::open(&db).unwrap()),
            Arc::new(PrincipalNamespaceIndex::open(&db).unwrap()),
            Arc::clone(&tl),
        ));
        let resolver = KernelHaltResolver::new(
            Arc::clone(&registry),
            Arc::clone(&tl),
            Arc::new(OutputMarkerRegistry::new()),
            Arc::new(maos_kernel_core::iac::Mailbox::new(Arc::new(
                IacRtMetrics::new(),
            ))),
            BOOT_NONCE,
            memory,
            Arc::clone(&orchestrator),
        );
        let flow = HaltFlow::new(
            Arc::new(resolver),
            Arc::new(NotificationDispatcher::new()),
            Arc::clone(&tl) as Arc<dyn HaltJournal>,
        );
        flow.submit_resolution(halt_id.clone(), resolution, spirit_id)
            .unwrap_or_else(|e| panic!("{spirit_id} {label}: {e}"));

        assert_eq!(registry.lookup_state(&halt_id), Some(terminal), "{label}");
        let approvals = tl.query_approvals(None).unwrap();
        assert_eq!(
            approvals.len(),
            1,
            "{spirit_id} {label}: one journaled resolution"
        );
        let row = &approvals[0];
        assert_eq!(
            (
                row.capability.as_str(),
                row.intent.as_str(),
                row.target.as_str()
            ),
            ("halt.resolve", label, spirit_id)
        );
        assert!(row.decision);
        assert_eq!(
            row.reasoning.as_deref(),
            Some(format!("halt={}: {reasoning}", halt_id.as_str()).as_str()),
            "{label}: the reasoning chain names the halt and the resolution"
        );
    }
}
