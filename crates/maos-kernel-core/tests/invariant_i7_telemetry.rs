//! I7 ledger proof: telemetry is broadcast, subscription is per-Spirit, and a halt's
//! `scalar.tap` run-up is observable.
//!
//! Seeded scalar writes go through the real `CapabilityRegistryAdapter::set_scalar` →
//! `TelemetryStreamAdapter` path: a topic only a Spirit subscribed to may deliver, and the
//! scalars written before a real policy-driven halt (`evaluate_after_set_scalar` →
//! `invoke_halt`) must all reach a `scalar.tap` observer. `invariants::i7::check` judges both,
//! as it does the adversarial corpus. Plant: publish to a topic channel nobody subscribed to, or
//! skip `publish_event` in `set_scalar`.

use std::sync::Arc;

use maos_domain::invariants::i7::{check, Delivery, Halt, Observation, TelemetryTopic};
use maos_domain::ports::TelemetryStreamPort;
use maos_kernel_core::capability::working_memory::policy_runtime::{
    evaluate_after_set_scalar, PolicyEvaluationOutcome,
};
use maos_kernel_core::capability::CapabilityRegistryPort;
use maos_kernel_core::halt::{invoke_halt, HaltRegistry};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::manifest::{
    EpistemicAction, EpistemicPolicyRule, EpistemicPolicySection, ScalarPredicate,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

mod common;

const SEEDS: [u64; 4] = [7, 0x7E1E, 0xBEEF7, 777];
const TAGS: [&str; 3] = ["uncertainty", "entropy", "confidence"];
const WRITES: usize = 24;
const HALTS: usize = 3;

fn deliveries(seed: u64) -> Vec<Delivery> {
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let adapter = common::registry::adapter_with_telemetry(&[], Arc::clone(&telemetry));
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let topic = |tag: &str| TelemetryTopic::new(format!("scalar.tap.{tag}"));
    let subscribed: Vec<bool> = TAGS
        .iter()
        .enumerate()
        .map(|(i, _)| i == 0 || (i == 2 && next() % 2 == 0))
        .collect();
    let mut receivers: Vec<_> = TAGS
        .iter()
        .zip(&subscribed)
        .map(|(tag, &on)| {
            if on {
                telemetry.subscribe_topic("observer", &topic(tag));
            }
            telemetry.subscribe(&topic(tag))
        })
        .collect();
    for n in 0..WRITES {
        let tag = TAGS[if n < TAGS.len() {
            n
        } else {
            (next() % TAGS.len() as u64) as usize
        }];
        adapter
            .set_scalar(1, "spirit-1", tag, n as f64 / 10.0, "frame")
            .expect("set_scalar");
    }
    TAGS.iter()
        .zip(&subscribed)
        .zip(&mut receivers)
        .map(|((tag, &subscribed), rx)| Delivery {
            topic: format!("scalar.tap.{tag}"),
            subscribed,
            delivered: rx.as_mut().is_some_and(|rx| rx.try_recv().is_ok()),
        })
        .collect()
}

/// A run-up of `steps` quiet writes, then a write above the threshold that trips a real halt.
fn halt_run(steps: usize) -> Halt {
    let telemetry = Arc::new(TelemetryStreamAdapter::default());
    let adapter = common::registry::adapter_with_telemetry(&[], Arc::clone(&telemetry));
    let mut tap = telemetry.subscribe_all();
    let policy = EpistemicPolicySection {
        rules: vec![EpistemicPolicyRule::new(
            "uncertainty".into(),
            EpistemicAction::Halt,
            None,
            None,
            Some(ScalarPredicate::Above { threshold: 0.7 }),
        )],
        default_action: EpistemicAction::VerbalizeOnly,
    };
    let tl = TransparencyLogAdapter::open_in_memory(0x17);
    let dir = tempfile::tempdir().expect("tempdir");
    let journal = JournalAdapter::open(&dir.path().join("journal.sqlite")).expect("journal");
    let registry = HaltRegistry::new();
    let mut written = 0;
    for value in (0..steps).map(|n| n as f64 / 100.0).chain([0.85]) {
        adapter
            .set_scalar(1, "spirit-1", "uncertainty", value, "frame")
            .expect("set_scalar");
        written += 1;
        let outcome = evaluate_after_set_scalar(
            "spirit-1",
            1,
            0x17,
            "uncertainty",
            value,
            "frame",
            &policy,
            &adapter as &dyn CapabilityRegistryPort,
        )
        .expect("policy");
        if let Some(PolicyEvaluationOutcome::Halt(payload)) = outcome {
            invoke_halt(&tl, &journal, &registry, payload, 1, "spirit-1", 0x17).expect("halt");
            break;
        }
    }
    let mut observed = 0;
    while tap.try_recv().is_ok() {
        observed += 1;
    }
    Halt { written, observed }
}

fn generated(seed: u64) -> Observation {
    Observation {
        deliveries: deliveries(seed),
        halts: (0..HALTS)
            .map(|n| halt_run(n * 3 + (seed % 3) as usize))
            .collect(),
    }
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
        if !obs.deliveries.iter().any(|d| d.subscribed && d.delivered) {
            failures.push(format!(
                "seed {seed}: no subscribed topic delivered (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i7_telemetry_reaches_only_subscribed_topics_and_a_halt_run_up_is_observable() {
    let corpus = include_str!("fixtures/invariants/i7_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
