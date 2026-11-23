//! I13 ledger proof: a digest's intent lineage is the kernel-computed union of its sources'
//! intents, and a consumer admits it only inside its allowed-promotion set.
//!
//! Seeded digests over raw frames of several intents (including digests of digests) go through
//! the real `DistillateWriter::write_distillate` and `admit_for_consumer`; the stamped lineage
//! against the flattened source intents, and each admission against a random consumer set, are
//! judged by `invariants::i13::check`, as is the adversarial corpus. Plant: stamp the lineage
//! from the request rather than the sources, or admit without comparing to the allowed set.

use std::collections::BTreeSet;

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::invariants::i13::{check, AllowedPromotionSet, Digest, Observation};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::ports::DistillationPort;
use maos_kernel_core::iac::transparency_log::FrameKind;

mod common;

const SEEDS: [u64; 4] = [13, 0x11AEA6E, 0x1313, 1300];
const INTENTS: [&str; 3] = ["consult", "delegate", "inform"];
const DIGESTS: usize = 12;

fn generated(seed: u64) -> Observation {
    let (writer, tl, _tmp) = common::distill::make_writer(seed);
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    // Every frame a digest may cite, with the raw intents it stands for.
    let mut frames: Vec<([u8; 16], BTreeSet<&str>)> = INTENTS
        .iter()
        .map(|intent| {
            tl.insert_frame_event(
                FrameKind::TaskAssign,
                1,
                None,
                intent,
                b"raw-payload",
                FrameOrigin::HumanAuthored,
            );
            (tl.last_frame_id(), BTreeSet::from([*intent]))
        })
        .collect();
    let mut digests = Vec::new();
    for _ in 0..DIGESTS {
        let cited: Vec<usize> = (0..1 + next() % 3)
            .map(|_| (next() % frames.len() as u64) as usize)
            .collect();
        let depth = 1 + (cited.iter().filter(|i| **i >= INTENTS.len()).count() as u32).min(1);
        let request = DistillationRequest::new(
            cited.iter().map(|i| frames[*i].0).collect(),
            depth,
            DigestPayload::Text("digest".into()),
            None,
        )
        .expect("valid request");
        let receipt = writer.write_distillate(1, request).expect("digest");
        let sources: BTreeSet<&str> = cited
            .iter()
            .flat_map(|i| frames[*i].1.iter().copied())
            .collect();
        let mut allowed = AllowedPromotionSet::new();
        let consumer_allowed: Vec<String> = INTENTS
            .iter()
            .filter(|_| next() % 2 == 0)
            .map(|s| s.to_string())
            .collect();
        for intent in &consumer_allowed {
            allowed.insert(A2AIntent::new(intent.as_str()));
        }
        let admitted = writer
            .admit_for_consumer(receipt.digest_frame_id, &allowed)
            .is_ok();
        digests.push(Digest {
            source_intents: sources.iter().map(|s| s.to_string()).collect(),
            lineage: receipt
                .intent_lineage
                .as_slice()
                .iter()
                .map(|i| i.as_str().to_string())
                .collect(),
            consumer_allowed,
            admitted,
        });
        frames.push((receipt.digest_frame_id, sources));
    }
    Observation { digests }
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
        let admitted = obs.digests.iter().filter(|d| d.admitted).count();
        if admitted == 0 || admitted == obs.digests.len() {
            failures.push(format!(
                "seed {seed}: needs admitted and refused digests (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i13_lineage_is_the_union_of_source_intents_and_admission_respects_the_allowed_set() {
    let corpus = include_str!("fixtures/invariants/i13_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
