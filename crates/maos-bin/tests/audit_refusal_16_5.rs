//! AC1's headline falsifier (review 2026-09-17): with a fault-injected audit
//! sink (receiver dropped, D-16-5-E), a collective write through
//! `LiveResearcherCollectivePort::collective_write` is REFUSED with the
//! distinct `AuditUnavailable` cause — never `Denied` (D-16-5-K) — and the
//! collective store is observably NOT written (E15-A6: read the state the
//! production code wrote, not the return value).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use maos_bin::cross_team_crossing::LiveResearcherCollectivePort;
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::memory::{MemoryEntry, MemoryNamespace, MemoryValue};
use maos_domain::ports::collective_memory::CollectiveMemoryPort;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit, cap_policy, cap_quota, cap_tokens, CapabilityRegistryAdapter,
    CapabilityRegistryPort, WorkingMemoryStore,
};
use maos_kernel_core::memory::principal::PrincipalNamespaceIndex;
use maos_kernel_core::memory::private::PrivateMemoryStore;
use maos_kernel_core::memory::shared::SharedMemoryStore;
use maos_kernel_core::memory::MemoryManagerAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use researcher::{ResearcherCollectiveError, ResearcherCollectivePort};

struct MockCryptoProvider;

impl CryptoProvider for MockCryptoProvider {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }

    fn seal_for_export(
        &self,
        _key: &[u8],
        _nonce: &[u8],
        _aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(plaintext.to_vec())
    }

    fn sign_capability_token(
        &self,
        _key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut signature = [0u8; 64];
        for (index, byte) in token_bytes.iter().enumerate() {
            signature[index % signature.len()] ^= byte;
        }
        Ok(signature.to_vec())
    }
}

fn capability_adapter(audit: cap_audit::Sender) -> CapabilityRegistryAdapter {
    let policy = cap_policy::PolicyTable::new();
    let mut inner = cap_policy::PolicyTableInner::default();
    // The port binds spirit_pid 0 until a scheduler pid is assigned.
    inner.manifest_scopes.insert(
        0,
        cap_policy::ManifestCapabilityScope {
            scopes: vec![Scope::LoomWrite, Scope::LoomRead, Scope::LoomScan],
            declared_tier: maos_domain::invariants::i9::SandboxTier(0),
            trust_tier: cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(inner);

    CapabilityRegistryAdapter::new(
        Arc::new(MockCryptoProvider),
        cap_tokens::Ed25519SigningKey::new([0u8; 32]),
        0x16_05,
        Arc::new(policy),
        audit,
        cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    )
}

/// Test double for the Loom-lite collective store: records exactly what the
/// production `MemoryManagerAdapter::collective_write` delivered, so the
/// refusal assertion observes the store, not the return value.
#[derive(Default)]
struct RecordingCollectivePort {
    entries: Mutex<HashMap<String, MemoryValue>>,
}

impl CollectiveMemoryPort for RecordingCollectivePort {
    fn write(
        &self,
        _spirit_pid: u32,
        _namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), maos_domain::ports::CollectivePortError> {
        self.entries
            .lock()
            .expect("recording port lock")
            .insert(key.to_string(), value);
        Ok(())
    }

    fn read(
        &self,
        _spirit_pid: u32,
        _namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, maos_domain::ports::CollectivePortError> {
        Ok(self
            .entries
            .lock()
            .expect("recording port lock")
            .get(key)
            .cloned())
    }

    fn scan(
        &self,
        _spirit_pid: u32,
        _namespace: &MemoryNamespace,
        _prefix: &str,
        _limit: usize,
    ) -> Result<Vec<MemoryEntry>, maos_domain::ports::CollectivePortError> {
        Ok(Vec::new())
    }

    fn erase(
        &self,
        _spirit_pid: u32,
        _namespace: &MemoryNamespace,
        _key: &str,
    ) -> Result<maos_domain::ports::CollectiveEraseReceipt, maos_domain::ports::CollectivePortError>
    {
        unimplemented!("erase is not exercised by this test")
    }
}

#[test]
fn closed_audit_sink_refuses_collective_write_without_touching_the_store() {
    cap_tokens::init_monotonic_base();
    let temp = tempfile::tempdir().expect("temp root");
    let db_path = temp.path().join("memory.sqlite3");

    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_in_memory(0x16_05),
    );
    let (audit, mut receiver) = cap_audit::channel();
    let capability = Arc::new(capability_adapter(audit));
    let recording_store = Arc::new(RecordingCollectivePort::default());
    let memory = Arc::new(
        MemoryManagerAdapter::new(
            Arc::new(PrivateMemoryStore::new(temp.path().to_path_buf(), 4 * 1024)),
            Arc::new(SharedMemoryStore::open(&db_path).expect("shared store")),
            Arc::new(PrincipalNamespaceIndex::open(&db_path).expect("principal index")),
            Arc::clone(&tl),
        )
        .with_collective_port(Some(
            Arc::clone(&recording_store) as Arc<dyn CollectiveMemoryPort>
        ))
        .with_capabilities(Some(Arc::clone(&capability))),
    );
    let port = LiveResearcherCollectivePort::new(Arc::clone(&memory), capability, None, None);

    // Healthy sink: the write lands (this is the baseline the refusal is
    // asserted against — a test that passes by refusing everything is vacuous,
    // obligation (g)).
    let namespace = MemoryNamespace::Default;
    port.collective_write(
        &namespace,
        "audit-refusal-key",
        MemoryValue::Text("v1".into()),
    )
    .expect("healthy sink accepts the collective write");
    let _ = receiver.try_recv();
    assert_eq!(
        recording_store
            .read(0, &namespace, "audit-refusal-key")
            .expect("store read"),
        Some(MemoryValue::Text("v1".into())),
        "the healthy write must reach the collective store"
    );

    // Fault injection (D-16-5-E): kill the audit writer for the process's
    // lifetime — every subsequent send is Closed.
    drop(receiver);

    // AC1: the refusal carries the distinct audit cause, and the audit
    // failure is never rendered as a policy `Denied` (D-16-5-K).
    let error = port
        .collective_write(
            &namespace,
            "audit-refusal-key",
            MemoryValue::Text("v2".into()),
        )
        .expect_err("a dead audit sink must refuse the collective write");
    assert!(
        matches!(error, ResearcherCollectiveError::AuditUnavailable),
        "the operator-visible cause must be the audit one, got {error:?}"
    );

    // E15-A6: the store was observably not written — the refused value never
    // landed and the healthy value is untouched.
    assert_eq!(
        recording_store
            .read(0, &namespace, "audit-refusal-key")
            .expect("store read"),
        Some(MemoryValue::Text("v1".into())),
        "the refused collective write must not touch the store"
    );
}
