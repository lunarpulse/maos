//! I11 ledger proof: a persisted digest has a non-empty, flattened `source_log_ref` and a
//! `distillation_depth`; anything else is refused `EDigestAuditChainMissing`.
//!
//! Seeded digest requests (empty, depth 0, raw, digest-of-digest and missing sources) go
//! through the real `DistillateWriter`; what it persisted and how it refused is judged by
//! `invariants::i11::check`, as is the adversarial corpus. The cargo-fuzz leg is the
//! `i11_digest_chain` target in `crates/maos-kernel-core/fuzz`. Plant: persist a request with an
//! empty `source_log_ref`, or skip the flattening of digest sources.

use maos_domain::distillation::{DigestPayload, DistillationError, DistillationRequest};
use maos_domain::invariants::i11::{check, Attempt, Observation};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::transparency_log::FrameKind;

mod common;

const SEEDS: [u64; 4] = [11, 0xD16E57, 0x1111, 1100];
const ATTEMPTS: usize = 40;

fn generated(seed: u64) -> Observation {
    let (writer, tl, _tmp) = common::distill::make_writer(seed);
    let raw = |intent: &str| {
        tl.insert_frame_event(
            FrameKind::TaskAssign,
            1,
            None,
            intent,
            b"raw-payload",
            FrameOrigin::HumanAuthored,
        );
        tl.last_frame_id()
    };
    let (r0, r1) = (raw("consult"), raw("delegate"));
    let seed_digest = DistillationRequest::new(vec![r0], 1, DigestPayload::Text("d0".into()), None)
        .expect("valid request");
    let d0 = writer
        .write_distillate(1, seed_digest)
        .expect("digest")
        .digest_frame_id;
    let pool = [r0, r1, d0, [0xDE; 16]];
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut attempts = Vec::new();
    for _ in 0..ATTEMPTS {
        let sources: Vec<[u8; 16]> = (0..next() % 4)
            .map(|_| pool[(next() % pool.len() as u64) as usize])
            .collect();
        let depth = (next() % 4) as u32;
        let request = DistillationRequest {
            source_log_ref: sources.clone(),
            distillation_depth: depth,
            digest_payload: DigestPayload::Text("digest".into()),
            segment_hint: None,
        };
        let mut attempt = Attempt {
            requested_sources: sources.len() as u32,
            requested_depth: depth,
            persisted: false,
            persisted_sources: 0,
            persisted_raw_only: true,
            refusal: None,
        };
        match writer.write_distillate(1, request) {
            Ok(receipt) => {
                attempt.persisted = true;
                attempt.persisted_sources = receipt.effective_source_log_ref.len() as u32;
                attempt.persisted_raw_only = receipt.effective_source_log_ref.iter().all(|id| {
                    matches!(tl.query_frame_by_id(*id), Ok(Some(row)) if row.kind != FrameKind::Distillate)
                });
            }
            Err(DistillationError::AuditChainMissing { .. }) => {
                attempt.refusal = Some("EDigestAuditChainMissing".into());
            }
            Err(other) => attempt.refusal = Some(other.to_string()),
        }
        attempts.push(attempt);
    }
    Observation { attempts }
}

fn run(corpus: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for (n, line) in corpus.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        match serde_json::from_str::<Observation>(line) {
            Ok(obs) if check(&obs).is_ok() => failures.push(format!("corpus line {n} accepted")),
            Ok(_) => {}
            Err(e) => failures.push(format!("corpus line {n} unparsable: {e}")),
        }
    }
    for seed in SEEDS {
        let obs = generated(seed);
        let persisted = obs.attempts.iter().filter(|a| a.persisted).count();
        let chainless = obs
            .attempts
            .iter()
            .filter(|a| a.requested_sources == 0 || a.requested_depth == 0)
            .count();
        if persisted == 0 || chainless == 0 {
            failures.push(format!(
                "seed {seed}: needs persisted and chainless attempts (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i11_a_persisted_digest_carries_a_flattened_chain_and_a_chainless_one_is_refused() {
    let corpus = include_str!("fixtures/invariants/i11_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
