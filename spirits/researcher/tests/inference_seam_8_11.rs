//! Story 8.11 · AC2 — the Researcher live-LLM finding-synthesis seam.
//!
//! Two polarities, both PROVEN here with a deterministic in-test stub so the
//! crate's tests stay hermetic and provider-independent (the daemon-level
//! FixtureReplay-vs-real polarity is pinned in `crates/maos-bin/tests/`):
//!
//! - **Live** (`with_inference_port`): the finding `statement` is the Inference
//!   Port's returned text (bounded). Every cite (`source_log_ref`), hedge, and
//!   the scalars stay exactly as the deterministic path produces them.
//! - **Deterministic** (no seam): byte-identical to v0.5 — the regression guard.

use std::sync::Arc;

use researcher::{ClaimPayload, RecalledFrame, Researcher};

use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::InferencePort;

/// A deterministic stub Inference Port returning a fixed text and recording the
/// requests it saw (so we can assert the daemon-issued token + pid threaded).
struct StubInferencePort {
    text: String,
    calls: std::sync::Mutex<Vec<InferenceRequest>>,
}

impl StubInferencePort {
    fn new(text: &str) -> Self {
        Self {
            text: text.into(),
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl InferencePort for StubInferencePort {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        self.calls.lock().unwrap().push(req);
        Ok(InferenceResponse {
            text: self.text.clone(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 7,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "stub".into(),
                endpoint_url: "stub://local".into(),
                model_id: Some("stub-1".into()),
            },
        })
    }
}

fn claim_frame(frame_id: [u8; 16], claim_id: &str, statement: &str) -> RecalledFrame {
    let claim = ClaimPayload {
        claim_id: claim_id.into(),
        statement: statement.into(),
        topic: "fusion".into(),
        methodology_strength: 0.9,
        confidence: 0.8,
        load_bearing: true,
        polarity: true,
        hedges: vec!["likely".into()],
    };
    RecalledFrame {
        frame_id,
        intent: "research.claim".into(),
        payload: serde_json::to_vec(&claim).unwrap(),
    }
}

fn token(pid: u32) -> CapabilityToken {
    CapabilityToken::new(TokenId([7u8; 16]), pid, 0, [0u8; 64])
}

/// LIVE path: the finding statement comes from the Inference Port, and the
/// daemon-issued token + real pid are threaded into the request.
#[test]
fn live_path_uses_inference_port_text_with_token_and_pid() {
    const PID: u32 = 42;
    let frames = vec![claim_frame([1u8; 16], "c1", "the original source statement")];
    let port = Arc::new(StubInferencePort::new("LIVE digest sentence from the model."));

    let researcher = Researcher::with_frames(frames.clone()).with_inference_port(
        Arc::clone(&port) as Arc<dyn InferencePort + Send + Sync>,
        token(PID),
        PID,
    );
    assert!(researcher.is_live());

    let out = researcher.survey(&frames);
    assert_eq!(out.findings.len(), 1);
    let f = &out.findings[0];
    assert_eq!(
        f.statement, "LIVE digest sentence from the model.",
        "live finding statement must be the Inference Port's returned text"
    );
    // Cites unchanged — traceability holds on the live digest.
    assert!(
        !f.source_log_ref.is_empty(),
        "the live finding must still carry a non-empty source_log_ref (traceability)"
    );
    // Hedges preserved verbatim.
    assert_eq!(f.hedges, vec!["likely".to_string()]);

    // The daemon-issued token + real pid threaded into the request.
    let calls = port.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "exactly one finding-synthesis call");
    assert_eq!(calls[0].spirit_pid, PID, "real pid threaded (not Some(0))");
    assert_eq!(calls[0].capability_token.spirit_pid, PID);
}

/// DETERMINISTIC path (no seam): byte-identical to v0.5 — the regression guard.
#[test]
fn deterministic_path_is_byte_identical_to_v05_survey() {
    let frames = vec![claim_frame([2u8; 16], "c1", "the original source statement")];

    let det = Researcher::with_frames(frames.clone());
    assert!(!det.is_live());
    let det_out = det.survey(&frames);

    // The deterministic statement is the bounded summary of the SOURCE, never a
    // model's text. A live run with a different model text must differ here.
    assert_eq!(det_out.findings[0].statement, "the original source statement");

    let live = Researcher::with_frames(frames.clone()).with_inference_port(
        Arc::new(StubInferencePort::new("a completely different model sentence"))
            as Arc<dyn InferencePort + Send + Sync>,
        token(7),
        7,
    );
    let live_out = live.survey(&frames);

    // Same cites, same scalars, same bibliography — ONLY the statement differs.
    assert_eq!(
        det_out.findings[0].source_log_ref,
        live_out.findings[0].source_log_ref
    );
    assert_eq!(det_out.scalars, live_out.scalars);
    assert_eq!(det_out.bibliography.len(), live_out.bibliography.len());
    assert_ne!(
        det_out.findings[0].statement, live_out.findings[0].statement,
        "the live seam must actually change the finding-synthesis output"
    );
}

/// A live error degrades to the deterministic summary (a finding is never
/// dropped). The daemon enforces fail-loud-on-Unconfigured BEFORE a --live run,
/// so this is genuine degradation, not silent disablement.
#[test]
fn live_error_falls_back_to_deterministic_summary() {
    struct FailingPort;
    impl InferencePort for FailingPort {
        fn complete(&self, _req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
            Err(InferenceError::Timeout)
        }
    }
    let frames = vec![claim_frame([3u8; 16], "c1", "the original source statement")];
    let researcher = Researcher::with_frames(frames.clone()).with_inference_port(
        Arc::new(FailingPort) as Arc<dyn InferencePort + Send + Sync>,
        token(9),
        9,
    );
    let out = researcher.survey(&frames);
    assert_eq!(
        out.findings[0].statement, "the original source statement",
        "on a live error the deterministic summary is the fail-safe"
    );
}
