//! Story 22-8 — Mira's directed-task handler against fake ports: the prompt adapter calls the
//! Inference Port only for text that is not already an `AnomalySignal` JSON, `diagnose` stays
//! deterministic, the per-subject record counts earlier diagnoses, the halt boundary raises the
//! scalar, and the advisory route's verdict is the answer's `advisory`.

use maos_domain::halt::HaltReceipt;
use maos_domain::ports::epistemic_scalar::ScalarPortError;
use maos_domain::ports::inference::{
    InferenceError, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_domain::ports::EpistemicScalarPort;
use mira::{AdvisoryRoute, DiagnosisMemory, DiagnosticAdvisory, Mira, TaskError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const KNOWN: &str =
    r#"{"subject":"payments-api","metric":"error_rate","observed":0.42,"baseline":0.02}"#;
const UNKNOWN_SEVERE: &str =
    r#"{"subject":"ledger-sync","metric":"queue_skew","observed":9.0,"baseline":1.0}"#;

/// A fake provider: counts calls and answers with a fixed reply, or refuses.
struct FakeInference {
    reply: Result<String, String>,
    calls: Mutex<Vec<String>>,
}

impl FakeInference {
    fn answering(text: &str) -> Self {
        Self {
            reply: Ok(text.into()),
            calls: Mutex::default(),
        }
    }

    fn refusing() -> Self {
        Self {
            reply: Err("provider transport down".into()),
            calls: Mutex::default(),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl InferencePort for FakeInference {
    fn complete(&self, request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.calls.lock().unwrap().push(request.prompt);
        match &self.reply {
            Ok(text) => Ok(InferenceResponse {
                text: text.clone(),
                stop_reason: StopReason::StopSequence,
                usage: TokenUsage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
                provider_attribution: ProviderAttribution {
                    provider_id: "fake".into(),
                    endpoint_url: "fake://".into(),
                    model_id: None,
                },
            }),
            Err(message) => Err(InferenceError::ProviderTransport(message.clone())),
        }
    }
}

#[derive(Default)]
struct FakeMemory {
    records: Mutex<HashMap<String, String>>,
    reads: Mutex<u32>,
}

impl DiagnosisMemory for FakeMemory {
    fn read(&self, key: &str) -> Result<Option<String>, String> {
        *self.reads.lock().unwrap() += 1;
        Ok(self.records.lock().unwrap().get(key).cloned())
    }

    fn write(&self, key: &str, markdown: &str) -> Result<(), String> {
        self.records
            .lock()
            .unwrap()
            .insert(key.to_owned(), markdown.to_owned());
        Ok(())
    }
}

struct FakeRoute(Result<(), String>);

impl AdvisoryRoute for FakeRoute {
    fn deliver(&self, _advisory: &DiagnosticAdvisory) -> Result<(), String> {
        self.0.clone()
    }
}

#[derive(Default)]
struct FakeScalar {
    writes: Mutex<Vec<(String, String, f64)>>,
}

impl EpistemicScalarPort for FakeScalar {
    fn write_scalar(
        &self,
        _spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        _derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError> {
        self.writes
            .lock()
            .unwrap()
            .push((spirit_id.to_owned(), tag.to_owned(), value));
        Ok(None)
    }
}

#[test]
fn a_json_signal_makes_no_inference_call() {
    let port = FakeInference::answering("never read");
    let memory = FakeMemory::default();
    let answer = Mira::default()
        .handle_task(KNOWN, Some(&port), &memory, None)
        .expect("a JSON signal is answered");
    assert!(port.calls().is_empty(), "no completion for a JSON signal");
    // `diagnose` is unchanged: the answer is exactly the deterministic diagnosis.
    let expected = Mira::default().diagnose(&serde_json::from_str(KNOWN).unwrap());
    assert_eq!(answer.subject, "payments-api");
    assert_eq!(answer.finding, expected.finding);
    assert_eq!(answer.severity, expected.severity);
    assert_eq!(answer.confidence, expected.confidence);
    assert_eq!(answer.advisory, "not_routed");
}

#[test]
fn free_text_is_turned_into_a_signal_through_the_port() {
    let port = FakeInference::answering(&format!("Here you go:\n```json\n{KNOWN}\n```"));
    let memory = FakeMemory::default();
    let answer = Mira::default()
        .handle_task(
            "payments-api is throwing 42% errors since the deploy",
            Some(&port),
            &memory,
            None,
        )
        .expect("free text is answered through the port");
    let calls = port.calls();
    assert_eq!(calls.len(), 1, "exactly one completion");
    assert!(
        calls[0].starts_with(mira::SIGNAL_PROMPT) && calls[0].contains("42% errors"),
        "the prompt carries the adapter instruction and the incident text: {}",
        calls[0]
    );
    assert_eq!(answer.subject, "payments-api");
    assert_eq!(
        answer.finding,
        Mira::default()
            .diagnose(&serde_json::from_str(KNOWN).unwrap())
            .finding,
        "the completion only made the signal; the diagnosis is the deterministic one"
    );
}

#[test]
fn a_refused_completion_is_a_typed_failure() {
    let refusing = FakeInference::refusing();
    let error = Mira::default()
        .handle_task(
            "the database is slow",
            Some(&refusing),
            &FakeMemory::default(),
            None,
        )
        .expect_err("a refused completion gives no answer");
    assert!(matches!(error, TaskError::Inference(_)), "{error:?}");
    assert_eq!(error.code(), "inference_refused");

    let junk = FakeInference::answering("I could not find a signal in that.");
    let error = Mira::default()
        .handle_task(
            "the database is slow",
            Some(&junk),
            &FakeMemory::default(),
            None,
        )
        .expect_err("a completion without a signal gives no answer");
    assert_eq!(error.code(), "inference_refused", "{error:?}");

    // Deterministic mode: no port, and text that is not a signal is a typed refusal too.
    let error = Mira::default()
        .handle_task("the database is slow", None, &FakeMemory::default(), None)
        .expect_err("free text needs a provider");
    assert_eq!(error.code(), "invalid_task", "{error:?}");
}

#[test]
fn the_prior_count_comes_from_the_memory_port() {
    let memory = FakeMemory::default();
    let mira = Mira::default();
    let first = mira.handle_task(KNOWN, None, &memory, None).unwrap();
    let second = mira.handle_task(KNOWN, None, &memory, None).unwrap();
    let other = mira
        .handle_task(UNKNOWN_SEVERE, None, &memory, None)
        .unwrap();
    assert_eq!(first.prior_diagnoses, 0);
    assert_eq!(
        second.prior_diagnoses, 1,
        "her own earlier answer is remembered"
    );
    assert_eq!(other.prior_diagnoses, 0, "memory is per diagnosed subject");
    // A fresh Mira over the same store (a restart) still counts it.
    let restarted = Mira::default();
    assert_eq!(
        restarted
            .handle_task(KNOWN, None, &memory, None)
            .unwrap()
            .prior_diagnoses,
        2
    );
    let records = memory.records.lock().unwrap();
    assert!(
        records
            .keys()
            .all(|key| !key.contains('/') && key.starts_with("mira.diagnosis.")),
        "every key is a legal memory key: {:?}",
        records.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_subject_with_path_characters_gets_a_legal_distinct_key() {
    let (a, b) = (mira::memory_key("a/b"), mira::memory_key("a%2Fb"));
    assert_ne!(a, b);
    assert!(!a.contains('/') && !a.contains('\\'));
    let long = "x".repeat(400);
    assert!(mira::memory_key(&long).len() < 200);
    assert_ne!(
        mira::memory_key(&long),
        mira::memory_key(&format!("{long}y"))
    );
}

#[test]
fn the_halt_boundary_raises_the_scalar_and_the_route_verdict_is_the_answer() {
    let scalar = Arc::new(FakeScalar::default());
    let mira = Mira::default()
        .with_id("mira")
        .with_scalar_port(scalar.clone());
    let denied = FakeRoute(Err("intent_denied".into()));
    let answer = mira
        .handle_task(UNKNOWN_SEVERE, None, &FakeMemory::default(), Some(&denied))
        .unwrap();
    assert_eq!(answer.advisory, "denied:intent_denied");
    let writes = scalar.writes.lock().unwrap().clone();
    assert_eq!(writes.len(), 1, "one scalar at the halt boundary");
    assert_eq!(writes[0].0, "mira");
    assert_eq!(writes[0].1, mira::DIAGNOSTIC_CONFIDENCE_TAG);
    assert!(writes[0].2 < mira::DIAGNOSTIC_CONFIDENCE_HALT_THRESHOLD);

    // A confident diagnosis raises nothing and a delivered advisory says so.
    let ok = FakeRoute(Ok(()));
    let answer = mira
        .handle_task(KNOWN, None, &FakeMemory::default(), Some(&ok))
        .unwrap();
    assert_eq!(answer.advisory, "delivered");
    assert_eq!(scalar.writes.lock().unwrap().len(), 1);
}
