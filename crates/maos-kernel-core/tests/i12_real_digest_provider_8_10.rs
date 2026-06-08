//! Story 8.10 AC3 — I12 decision-context content, proven end-to-end against the
//! REAL Story-4.3 `MemoryManagerAdapter`.
//!
//! `memory_backed_digest_provider` queries the real Memory Manager for the
//! citing Spirit's in-context `digest:` working-memory refs; a `decision.*`
//! frame decorated through it carries a **non-empty** `working_memory_digest_refs`,
//! and the de-tautologized `frame_carries_i12_refs` reports it.

use std::sync::Arc;

use maos_domain::frame::{
    DecisionDispatchPayload, FrameAddress, FramePayload, IacFrame,
};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i12::WorkingMemoryDigestRefs;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::decision_logger::{
    decorate_decision_frame, frame_carries_i12_refs, memory_backed_digest_provider,
    WORKING_MEMORY_DIGEST_KEY_PREFIX,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

fn make_memory() -> (Arc<MemoryManagerAdapter>, tempfile::TempDir) {
    let tmp = tempfile::TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xD1235));
    let adapter = Arc::new(MemoryManagerAdapter::new(private, shared, principal, tl));
    (adapter, tmp)
}

fn decision_frame(spirit: &str) -> IacFrame {
    IacFrame {
        frame_id: [0u8; 16],
        timestamp_ns: 0,
        logical_clock: 0,
        from: FrameAddress {
            spirit_id: SpiritId::from(spirit),
            host_id: None,
            role: None,
        },
        to: smallvec![],
        kind: FrameKind::DecisionDispatch,
        intent: IntentClass::Standard,
        payload: FramePayload::DecisionDispatch(DecisionDispatchPayload {
            decision_id: 1,
            approved: true,
            working_memory_digest_refs: WorkingMemoryDigestRefs::default(),
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: maos_domain::invariants::i13::IntentLineage::default(),
    }
}

#[test]
fn real_provider_yields_nonempty_refs_for_decision_frame() {
    let (memory, _tmp) = make_memory();
    let pid: u32 = 7;

    // The Spirit recorded two digests it reasoned over in its private WM.
    for key in ["digest:frame-aaa", "digest:frame-bbb"] {
        memory
            .write(
                pid,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                key,
                MemoryValue::Text("ref".into()),
            )
            .unwrap();
    }
    assert!(WORKING_MEMORY_DIGEST_KEY_PREFIX.starts_with("digest:"));

    // The REAL provider, backed by the real MemoryManagerAdapter.
    let memory_port: Arc<dyn MemoryManagerPort + Send + Sync> =
        Arc::clone(&memory) as Arc<dyn MemoryManagerPort + Send + Sync>;
    let provider = memory_backed_digest_provider(memory_port, move |_sid| Some(pid));

    let decorated = decorate_decision_frame(decision_frame("researcher"), &provider);

    match &decorated.payload {
        FramePayload::DecisionDispatch(p) => {
            let refs = p.working_memory_digest_refs.as_slice();
            assert_eq!(
                refs,
                &["frame-aaa".to_string(), "frame-bbb".to_string()],
                "real provider surfaces the in-context digest refs (prefix stripped, sorted)"
            );
        }
        _ => panic!("expected DecisionDispatch"),
    }

    assert!(
        frame_carries_i12_refs(&decorated),
        "a decision frame decorated through the real provider carries I12 refs"
    );
}

#[test]
fn real_provider_empty_when_spirit_has_no_digests() {
    let (memory, _tmp) = make_memory();
    let memory_port: Arc<dyn MemoryManagerPort + Send + Sync> =
        Arc::clone(&memory) as Arc<dyn MemoryManagerPort + Send + Sync>;
    let provider = memory_backed_digest_provider(memory_port, |_sid| Some(99));

    let decorated = decorate_decision_frame(decision_frame("idle-spirit"), &provider);
    match &decorated.payload {
        FramePayload::DecisionDispatch(p) => {
            assert!(p.working_memory_digest_refs.as_slice().is_empty());
        }
        _ => panic!("expected DecisionDispatch"),
    }
    // And the de-tautologized assertion correctly reports it does NOT carry refs.
    assert!(
        !frame_carries_i12_refs(&decorated),
        "empty-refs decision frame must NOT satisfy I12 (de-tautologized)"
    );
}
