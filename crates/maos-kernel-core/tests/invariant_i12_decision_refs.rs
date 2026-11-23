//! I12 ledger proof: every `decision.*` frame carries the digests and inbound raw frames its
//! Spirit declared in context.
//!
//! Seeded Spirits record digest refs and raw inbound frame refs (plus unrelated keys) in their
//! private working memory through the real `MemoryManagerAdapter`; a `decision.*` frame is
//! decorated by the production `memory_backed_digest_provider` and its
//! `working_memory_digest_refs` are judged against what the Spirit declared by
//! `invariants::i12::check`, as is the adversarial corpus. Plant: have the provider return the
//! default (empty) refs or drop the `raw-` keys.

use std::sync::Arc;

use maos_domain::frame::{DecisionDispatchPayload, FrameAddress, FramePayload, IacFrame};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i12::{check, Decision, Observation, WorkingMemoryDigestRefs};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::decision_logger::{
    decorate_decision_frame, memory_backed_digest_provider, WORKING_MEMORY_DIGEST_KEY_PREFIX,
};
use maos_spirit_abi::identity::{FrameKind, SpiritId};
use smallvec::smallvec;

mod common;

const SEEDS: [u64; 4] = [12, 0x1212, 0xDEC15, 1200];
const SPIRITS: u32 = 4;

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
        intent_lineage: IntentLineage::default(),
    }
}

fn generated(seed: u64) -> Observation {
    let (memory, _tmp) = common::memory::make_memory();
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut declared: Vec<Vec<String>> = vec![Vec::new(); SPIRITS as usize];
    for pid in 1..=SPIRITS {
        memory
            .bind_storage_identity(pid, &format!("spirit-{pid}"))
            .expect("bind");
        for n in 0..next() % 6 {
            let name = if next() % 2 == 0 {
                format!("digest-{n}")
            } else {
                format!("raw-inbound-{n}")
            };
            let key = format!("{WORKING_MEMORY_DIGEST_KEY_PREFIX}{name}");
            memory
                .write(
                    pid,
                    MemoryTier::Private,
                    &MemoryNamespace::Default,
                    &key,
                    MemoryValue::Text("ref".into()),
                )
                .expect("declare in context");
            declared[pid as usize - 1].push(name);
        }
        memory
            .write(
                pid,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "note:unrelated",
                MemoryValue::Text("x".into()),
            )
            .expect("unrelated key");
    }
    let provider = memory_backed_digest_provider(
        Arc::clone(&memory) as Arc<dyn MemoryManagerPort + Send + Sync>,
        |sid: &SpiritId| {
            sid.as_str()
                .strip_prefix("spirit-")
                .and_then(|n| n.parse().ok())
        },
    );
    let decisions = (1..=SPIRITS)
        .map(|pid| {
            let frame =
                decorate_decision_frame(decision_frame(&format!("spirit-{pid}")), &provider);
            let refs = match frame.payload {
                FramePayload::DecisionDispatch(p) => {
                    p.working_memory_digest_refs.as_slice().to_vec()
                }
                _ => Vec::new(),
            };
            Decision {
                declared: declared[pid as usize - 1].clone(),
                refs,
            }
        })
        .collect();
    Observation { decisions }
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
        if !obs.decisions.iter().any(|d| !d.declared.is_empty()) {
            failures.push(format!(
                "seed {seed}: no Spirit declared a digest (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i12_a_decision_frame_carries_every_declared_digest_and_inbound_raw_frame() {
    let corpus = include_str!("fixtures/invariants/i12_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
