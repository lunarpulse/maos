//! Story 9.2 (AC4.6) — headline acceptance smoke.  Exercises the full erasure
//! demo path in-process: write principal data + a canary distillate → forget →
//! subject-access empty → canary probe clean → build + verify a signed
//! proof-of-erasure (bundle-only AND against the live TL), plus a legal-hold arm.

#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_audit::erasure::proof::{
    build_erasure_proof, verify_erasure_proof, verify_erasure_proof_against_log, CategoryStatus,
    ErasureCategory,
};
use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::{DistillationPort, MemoryManagerPort};
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::memory::MemoryManagerAdapter;
use tempfile::TempDir;

const PRINCIPAL: &str = "smoke-subject@example.org";
const CANARY: &str = "SMOKE-CANARY-9-2-HEADLINE";

fn fresh_adapter(
    dir: &TempDir,
) -> (
    Arc<MemoryManagerAdapter>,
    Arc<TransparencyLogAdapter>,
    std::path::PathBuf,
) {
    let fs_root = dir.path().join("memory");
    std::fs::create_dir_all(&fs_root).unwrap();
    let db_path = dir.path().join("audit.sqlite");
    let private = Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
        fs_root,
        4 * 1024,
    ));
    let shared = Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db_path).unwrap());
    let principal_index =
        Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&db_path, &db_path, 1).unwrap(),
    );
    let memory = Arc::new(MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        Arc::clone(&tl),
    ));
    (memory, tl, db_path)
}

fn canary_in_distillates(tl: &TransparencyLogAdapter, canary: &str) -> bool {
    let entries = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::Distillate),
            ..Default::default()
        })
        .unwrap();
    entries
        .iter()
        .any(|e| String::from_utf8_lossy(&e.payload_redacted).contains(canary))
}

#[test]
fn headline_erasure_demo_produces_verifiable_proof() {
    let dir = TempDir::new().unwrap();
    let (memory, tl, db_path) = fresh_adapter(&dir);

    // 1. Spirit writes principal data + a canary distillate.
    let ns = MemoryNamespace::Principal {
        principal_id: PRINCIPAL.into(),
        schema: "chat".into(),
    };
    memory
        .write(
            7,
            MemoryTier::Private,
            &ns,
            "msg1",
            MemoryValue::Text("hi".into()),
        )
        .unwrap();
    let source = {
        tl.insert_frame_event(
            FrameKind::TaskComplete,
            7,
            None,
            "task.complete",
            CANARY.as_bytes(),
            maos_domain::invariants::i3::FrameOrigin::Kernel,
        );
        tl.last_frame_id()
    };
    // Story 9.3b SR-3 setup: also emit a principal-bearing cost-attribution
    // frame so the forget cascade has a structured frame to redact.
    let cost_payload = maos_domain::cost::CostAttributionPayload {
        schema_version: 1,
        timestamp_ns: 1,
        spirit_pid: 7,
        provider: "anthropic".into(),
        model: "claude-3".into(),
        principal: maos_domain::cost::PrincipalRef::Resolved {
            principal_id: PRINCIPAL.into(),
        },
        attribution_source: maos_domain::cost::AttributionSource::WriteTargetProxy,
        attribution_confidence: maos_domain::cost::AttributionConfidence::Exact,
        dimensions: {
            let mut d = std::collections::BTreeMap::new();
            d.insert(maos_domain::cost::CostDimension::TokensIn, 100);
            d
        },
    };
    tl.insert_frame_event(
        FrameKind::CostAttribution,
        7,
        None,
        "cost.attribution",
        &serde_json::to_vec(&cost_payload).unwrap(),
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );
    let writer = DistillateWriter::new(
        Arc::clone(&tl),
        Arc::new(()) as Arc<dyn std::any::Any + Send + Sync>,
    );
    let distillate_body = format!("{CANARY} principal={PRINCIPAL}");
    let receipt = writer
        .write_distillate(
            7,
            DistillationRequest::new(vec![source], 1, DigestPayload::Text(distillate_body), None)
                .unwrap(),
        )
        .unwrap();
    let distillate_frame_id = receipt.digest_frame_id;

    // Pre-forget: subject access finds the row; canary present.
    assert!(!maos_audit::subject_access_query(&db_path, PRINCIPAL)
        .unwrap()
        .is_empty());
    assert!(canary_in_distillates(&tl, CANARY));

    let pre_frame_ids = tl.all_frame_ids().unwrap();

    // 2. Forget the principal.
    let outcome = memory.forget_with_reason(PRINCIPAL, None).unwrap();
    let (erased_distillate_frame_ids, erased_principal_frame_ids) = match outcome {
        maos_domain::memory::ForgetOutcome::Erased {
            redacted_distillate_frame_ids,
            redacted_principal_frame_ids,
            ..
        } => (redacted_distillate_frame_ids, redacted_principal_frame_ids),
        other => panic!("expected Erased, got {other:?}"),
    };

    // 3. Subject-access is empty + canary is gone.
    assert!(
        maos_audit::subject_access_query(&db_path, PRINCIPAL)
            .unwrap()
            .is_empty(),
        "subject-access must be empty after forget"
    );
    assert!(
        !canary_in_distillates(&tl, CANARY),
        "canary must not survive in distillate bodies"
    );

    // 4. Build + sign the proof-of-erasure.
    let post_frame_ids = tl.all_frame_ids().unwrap();
    let erased_frame_ids: Vec<[u8; 16]> = erased_distillate_frame_ids
        .iter()
        .filter_map(|h| hex::decode(h).ok())
        .filter(|b| b.len() == 16)
        .map(|b| {
            let mut a = [0u8; 16];
            a.copy_from_slice(&b);
            a
        })
        .collect();
    assert!(
        erased_frame_ids.contains(&distillate_frame_id),
        "the scrubbed distillate must be among the erased frames"
    );
    let erased_principal_frame_ids: Vec<[u8; 16]> = erased_principal_frame_ids
        .iter()
        .filter_map(|h| hex::decode(h).ok())
        .filter(|b| b.len() == 16)
        .map(|b| {
            let mut a = [0u8; 16];
            a.copy_from_slice(&b);
            a
        })
        .collect();
    let seed = [0x9_2u8; 32];
    let proof = build_erasure_proof(
        "smoke-spirit".into(),
        7,
        1,
        &pre_frame_ids,
        &post_frame_ids,
        &erased_frame_ids,
        &[PRINCIPAL.to_string()],
        &erased_principal_frame_ids,
        vec![
            ErasureCategory {
                name: "memory_namespace".into(),
                status: CategoryStatus::Removed { count: 1 },
            },
            ErasureCategory {
                name: "principal_frames".into(),
                status: CategoryStatus::Removed {
                    count: erased_principal_frame_ids.len() as u64,
                },
            },
        ],
        &seed,
    )
    .unwrap();

    // 5. Verify — both bundle-only and against the live TL.
    let pubkey = ed25519_dalek::SigningKey::from_bytes(&seed)
        .verifying_key()
        .to_bytes();
    assert!(verify_erasure_proof(&proof, &pubkey).is_ok());
    // SR-3: the proof must include inclusion proofs for any redacted
    // principal-bearing frames (e.g., cost/governance frames whose payloads
    // were scrubbed but which remain in the append-only TL).
    assert!(
        !proof.redacted_principal_frame_proofs.is_empty(),
        "SR-3: redacted principal-bearing frames must be wired into the proof"
    );
    assert!(
        verify_erasure_proof_against_log(&proof, &pubkey, &pre_frame_ids, &post_frame_ids).is_ok(),
        "against-log verification must pass with the real frame sets"
    );
}

#[test]
fn legal_hold_arm_suspends_and_blocks_second_forget() {
    let dir = TempDir::new().unwrap();
    let (memory, _tl, db_path) = fresh_adapter(&dir);

    let ns = MemoryNamespace::Principal {
        principal_id: PRINCIPAL.into(),
        schema: "chat".into(),
    };
    memory
        .write(
            7,
            MemoryTier::Private,
            &ns,
            "msg1",
            MemoryValue::Text("hi".into()),
        )
        .unwrap();
    assert!(!maos_audit::subject_access_query(&db_path, PRINCIPAL)
        .unwrap()
        .is_empty());

    // P29: a legal-hold suspends and persists.
    let held = memory
        .forget_with_reason(PRINCIPAL, Some("legal-hold:audit-2026"))
        .unwrap();
    assert!(matches!(
        held,
        maos_domain::memory::ForgetOutcome::Suspended { .. }
    ));
    // The principal is retained.
    assert!(!maos_audit::subject_access_query(&db_path, PRINCIPAL)
        .unwrap()
        .is_empty());

    // A second forget WITHOUT a reason is blocked by the durable hold.
    let second = memory.forget_with_reason(PRINCIPAL, None).unwrap();
    assert!(
        matches!(second, maos_domain::memory::ForgetOutcome::Suspended { .. }),
        "a prior durable hold must block a later reasonless forget"
    );
    assert!(!maos_audit::subject_access_query(&db_path, PRINCIPAL)
        .unwrap()
        .is_empty());

    // Release the hold → a subsequent forget erases.
    assert!(memory.release_legal_hold(PRINCIPAL).unwrap());
    let third = memory.forget_with_reason(PRINCIPAL, None).unwrap();
    assert!(matches!(
        third,
        maos_domain::memory::ForgetOutcome::Erased { .. }
    ));
    assert!(maos_audit::subject_access_query(&db_path, PRINCIPAL)
        .unwrap()
        .is_empty());
}

#[test]
fn zero_entry_forget_is_a_documented_noop() {
    // P28: forgetting a principal that was never written must still succeed,
    // journal a principal.forget frame, and report zero deletions — the
    // empty-principal path the corpus's zero_entry stratum is meant to cover.
    let dir = TempDir::new().unwrap();
    let (memory, tl, _db_path) = fresh_adapter(&dir);

    let pre_count = tl.all_frame_ids().unwrap().len();
    let outcome = memory
        .forget_with_reason("never-written@example.org", None)
        .unwrap();
    match outcome {
        maos_domain::memory::ForgetOutcome::Erased { receipt, .. } => {
            assert_eq!(receipt.deleted_entries, 0);
            assert_eq!(receipt.deleted_index_rows, 0);
        }
        other => panic!("zero-entry forget must Erased, got {other:?}"),
    }
    // The cascade still journals a principal.forget frame.
    assert!(
        tl.all_frame_ids().unwrap().len() > pre_count,
        "zero-entry forget must still append a journal frame"
    );
}
