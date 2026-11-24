//! Story 22-8 — Mira's **directed task**: the native leg of `maosctl ask mira "<incident>"`.
//!
//! The host hands [`Mira::handle_task`] one incident text. Mira turns it into an
//! [`AnomalySignal`] (the JSON itself, or — only for other text — one `provider.complete`
//! call through the host-bound [`InferencePort`]), diagnoses it **deterministically**
//! ([`Mira::diagnose`], unchanged: NFR-Testability-1), keeps one Markdown record per diagnosed
//! subject through the host-bound [`DiagnosisMemory`], raises her diagnostic-confidence scalar at
//! her halt boundary, hands the read-only advisory to the host-bound [`AdvisoryRoute`], and
//! answers one JSON document ([`TaskAnswer`]).
//!
//! Everything kernel-side (the pid-bound memory port, the minted inference token, the A2A
//! router and its consent) is a port the host binds: this crate stays free of kernel and A2A
//! crates (Story 0.2).

use crate::{AnomalySignal, DiagnosticAdvisory, Mira, DIAGNOSTIC_CONFIDENCE_TAG};
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{InferenceOptions, InferencePort, InferenceRequest};
use serde::{Deserialize, Serialize};

/// Prefix of the Private-tier key one diagnosed subject's record lives under
/// (`mira.diagnosis.<subject>`, the subject percent-encoded to a legal memory key).
pub const MEMORY_KEY_PREFIX: &str = "mira.diagnosis.";

/// The longest encoded subject a memory key carries (the store caps a whole key).
const MAX_ENCODED_SUBJECT: usize = 120;

/// The instruction prefix of the one completion Mira asks for free text.
pub const SIGNAL_PROMPT: &str =
    "You are a prod-edge incident parser. Convert the incident report below into ONE JSON \
object with the keys \"subject\" (string), \"metric\" (string), \"observed\" (number), \
\"baseline\" (number), \"detail\" (string) and \"source_log_ref\" (string). Reply with the JSON \
object only.\n\nIncident report:\n";

/// Durable per-subject memory, pid-bound by the host (the Private tier, Markdown).
pub trait DiagnosisMemory: Send + Sync {
    fn read(&self, key: &str) -> Result<Option<String>, String>;
    fn write(&self, key: &str, markdown: &str) -> Result<(), String>;
}

/// Where Mira's read-only advisory goes. The host decides the recipient and the consent intent;
/// `Err(code)` is the typed denial Mira reports as `denied:<code>`.
pub trait AdvisoryRoute: Send + Sync {
    fn deliver(&self, advisory: &DiagnosticAdvisory) -> Result<(), String>;
}

/// Why a task gave no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// The text is not an `AnomalySignal` JSON and no signal could be made of it.
    Signal(String),
    /// The inference port refused or returned nothing usable.
    Inference(String),
    /// The memory port refused the read or the write.
    Memory(String),
    /// The diagnostic-confidence scalar could not be raised at the halt boundary.
    Halt(String),
}

impl TaskError {
    /// A stable, typed reason for the host's task record.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Signal(_) => "invalid_task",
            Self::Inference(_) => "inference_refused",
            Self::Memory(_) => "memory_refused",
            Self::Halt(_) => "halt_refused",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::Signal(detail)
            | Self::Inference(detail)
            | Self::Memory(detail)
            | Self::Halt(detail) => detail,
        }
    }
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.detail())
    }
}

impl std::error::Error for TaskError {}

/// Mira's answer to one directed task (the JSON `task show` returns as `result`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskAnswer {
    pub subject: String,
    pub finding: String,
    pub severity: f64,
    pub confidence: f64,
    /// `delivered`, `denied:<code>` or `not_routed` (the Host declares no A2A peers).
    pub advisory: String,
    /// How many diagnoses of this subject Mira had recorded before this one.
    pub prior_diagnoses: u64,
}

impl TaskAnswer {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("a TaskAnswer is plain data")
    }
}

/// The legal memory key of `subject`'s record.
pub fn memory_key(subject: &str) -> String {
    let mut encoded = String::with_capacity(subject.len());
    for byte in subject.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' => encoded.push(byte as char),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    if encoded.len() > MAX_ENCODED_SUBJECT {
        // FNV-1a over the whole subject keeps two long subjects apart.
        let hash = subject
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
            });
        encoded.truncate(MAX_ENCODED_SUBJECT);
        encoded.push_str(&format!("~{hash:016x}"));
    }
    format!("{MEMORY_KEY_PREFIX}{encoded}")
}

fn parse_prior(record: &str) -> u64 {
    record
        .lines()
        .find_map(|line| line.strip_prefix("diagnoses: "))
        .and_then(|count| count.trim().parse().ok())
        .unwrap_or(0)
}

fn render_record(
    subject: &str,
    count: u64,
    finding: &str,
    severity: f64,
    confidence: f64,
) -> String {
    format!(
        "# mira.diagnosis — {subject}\n\ndiagnoses: {count}\nlast_severity: {severity:.3}\n\
         last_confidence: {confidence:.3}\nlast_finding: {finding}\n"
    )
}

/// The `AnomalySignal` a task text stands for: the JSON itself, else one completion.
fn signal_of(
    text: &str,
    inference: Option<&dyn InferencePort>,
) -> Result<AnomalySignal, TaskError> {
    if let Ok(signal) = serde_json::from_str::<AnomalySignal>(text.trim()) {
        return check_signal(signal);
    }
    let Some(port) = inference else {
        return Err(TaskError::Signal(
            "the task is not an AnomalySignal JSON and this Host binds no inference provider \
             (set inference_mode, or send the signal as JSON)"
                .into(),
        ));
    };
    // The host re-binds the request's pid, token and provider; Mira holds none of them.
    let request = InferenceRequest::new(
        0,
        CapabilityToken::new(TokenId([0; 16]), 0, 0, [0; 64]),
        format!("{SIGNAL_PROMPT}{text}"),
        InferenceOptions {
            max_tokens: 400,
            temperature: Some(0.0),
            model_id: None,
        },
        None,
        Vec::new(),
    );
    let response = port
        .complete(request)
        .map_err(|error| TaskError::Inference(error.to_string()))?;
    let reply = response.text.as_str();
    let (Some(start), Some(end)) = (reply.find('{'), reply.rfind('}')) else {
        return Err(TaskError::Inference(
            "the completion holds no JSON object".into(),
        ));
    };
    let signal = serde_json::from_str::<AnomalySignal>(&reply[start..=end]).map_err(|error| {
        TaskError::Inference(format!("the completion is not an AnomalySignal: {error}"))
    })?;
    check_signal(signal)
}

fn check_signal(signal: AnomalySignal) -> Result<AnomalySignal, TaskError> {
    if signal.subject.trim().is_empty() || signal.metric.trim().is_empty() {
        return Err(TaskError::Signal(
            "a signal names a subject and a metric".into(),
        ));
    }
    if !signal.observed.is_finite() || !signal.baseline.is_finite() {
        return Err(TaskError::Signal(
            "a signal's values are finite numbers".into(),
        ));
    }
    Ok(signal)
}

impl Mira {
    /// Answer one directed task. `inference` is `None` under the Deterministic mode (ADR-064);
    /// `route` is `None` when the Host declares no A2A peers.
    pub fn handle_task(
        &self,
        text: &str,
        inference: Option<&dyn InferencePort>,
        memory: &dyn DiagnosisMemory,
        route: Option<&dyn AdvisoryRoute>,
    ) -> Result<TaskAnswer, TaskError> {
        let signal = signal_of(text, inference)?;
        let diagnosis = self.diagnose(&signal);
        let key = memory_key(&diagnosis.subject);
        let prior = memory
            .read(&key)
            .map_err(TaskError::Memory)?
            .map_or(0, |record| parse_prior(&record));
        memory
            .write(
                &key,
                &render_record(
                    &diagnosis.subject,
                    prior + 1,
                    &diagnosis.finding,
                    diagnosis.severity,
                    diagnosis.confidence,
                ),
            )
            .map_err(TaskError::Memory)?;
        if diagnosis.requires_halt {
            if let Some(port) = self.scalar_port.as_ref() {
                let derived_from = if diagnosis.source_log_ref.is_empty() {
                    format!("mira:diagnose:{}", diagnosis.subject)
                } else {
                    diagnosis.source_log_ref.clone()
                };
                port.write_scalar(
                    0,
                    self.spirit_id(),
                    DIAGNOSTIC_CONFIDENCE_TAG,
                    diagnosis.confidence,
                    &derived_from,
                )
                .map_err(|error| TaskError::Halt(error.to_string()))?;
            }
        }
        let advisory = match route {
            None => "not_routed".to_owned(),
            Some(route) => match route.deliver(&self.advisory(&diagnosis)) {
                Ok(()) => "delivered".to_owned(),
                Err(code) => format!("denied:{code}"),
            },
        };
        Ok(TaskAnswer {
            subject: diagnosis.subject,
            finding: diagnosis.finding,
            severity: diagnosis.severity,
            confidence: diagnosis.confidence,
            advisory,
            prior_diagnoses: prior,
        })
    }
}
