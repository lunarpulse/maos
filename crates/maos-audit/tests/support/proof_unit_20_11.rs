use super::*;

fn post_ids(pre: &[[u8; 16]]) -> Vec<[u8; 16]> {
    let mut post = pre.to_vec();
    post.push([42; 16]);
    post.extend((100..116).map(|id| [id; 16]));
    post.push([199; 16]);
    post
}

fn fixture(
    seed: &[u8; 32],
    pre: &[[u8; 16]],
    erased: &[[u8; 16]],
    subjects: &[String],
) -> Result<ErasureProof, ErasureProofError> {
    let observations = REQUIRED_CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let (status, removals) = match *name {
                "memory_namespace" => (
                    CategoryStatus::Removed { count: 3 },
                    vec![RemovalEvidence::MemoryNamespaceBatch {
                        receipt_frame_id: format_frame_id(&[42; 16]),
                        deleted_entries: 3,
                    }],
                ),
                "intent_lineage" => (
                    CategoryStatus::Removed {
                        count: erased.len() as u64,
                    },
                    erased
                        .iter()
                        .map(|id| RemovalEvidence::Frame {
                            frame_id: format_frame_id(id),
                        })
                        .collect(),
                ),
                "scheduled_invocations" => (
                    CategoryStatus::CoverageGap {
                        reason: "unit fixture retains schedules".into(),
                    },
                    vec![],
                ),
                _ => (CategoryStatus::VerifiedEmpty, vec![]),
            };
            CategoryObservation::new(
                ErasureCategory {
                    name: (*name).into(),
                    status,
                },
                [100 + index as u8; 16],
                removals,
            )
        })
        .collect();
    build_erasure_proof(
        "spirit-1".into(),
        1,
        1234,
        pre,
        &post_ids(pre),
        erased,
        subjects,
        &[],
        observations,
        Some(CompletionObservation {
            frame_id: [199; 16],
            through_rowid: 99,
        }),
        seed,
    )
}

#[test]
fn sign_and_verify_round_trip() {
    let seed = [7; 32];
    let pre: Vec<_> = (0..4).map(|id| [id; 16]).collect();
    let proof = fixture(
        &seed,
        &pre,
        &[pre[0], pre[2]],
        &["alice@example.org".into()],
    )
    .expect("signed fixture");
    let key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    assert!(verify_erasure_proof(&proof, &key).is_ok());
    assert!(verify_erasure_proof_against_log(&proof, &key, &pre, &post_ids(&pre)).is_ok());
}

#[test]
fn erased_frame_absent_from_pre_tree_is_rejected() {
    let pre: Vec<_> = (0..4).map(|id| [id; 16]).collect();
    assert!(
        fixture(&[9; 32], &pre, &[[250; 16]], &[]).is_err(),
        "cannot prove inclusion of a frame absent from pre-tree"
    );
}

#[test]
fn tampered_post_root_is_rejected() {
    let seed = [3; 32];
    let pre: Vec<_> = (0..4).map(|id| [id; 16]).collect();
    let mut proof =
        fixture(&seed, &pre, &[pre[0]], &["bob@example.org".into()]).expect("signed fixture");
    proof.post_root[0] ^= 0xff;
    let key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    assert!(
        verify_erasure_proof(&proof, &key).is_err(),
        "tampered post_root must fail"
    );
}

#[test]
fn against_log_catches_withheld_frames() {
    let seed = [4; 32];
    let pre: Vec<_> = (0..4).map(|id| [id; 16]).collect();
    let proof =
        fixture(&seed, &pre, &[pre[0]], &["carol@example.org".into()]).expect("signed fixture");
    let key = SigningKey::from_bytes(&seed).verifying_key().to_bytes();
    assert!(verify_erasure_proof(&proof, &key).is_ok());
    let mut lying_post = post_ids(&pre);
    lying_post.push([99; 16]);
    assert!(
        verify_erasure_proof_against_log(&proof, &key, &pre, &lying_post).is_err(),
        "against-log verification must catch a signer withholding frames"
    );
}
