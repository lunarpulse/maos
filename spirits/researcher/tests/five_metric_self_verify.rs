//! AC4 — Researcher is the primary reference producer for the five-metric
//! distillation gate (NFR-Aud-7). The N=100 synthetic-v0 gate itself is
//! unchanged and lives in `maos-eval`; here we MEASURE the two STRUCTURAL
//! metrics against Researcher's OWN real distillates (Decision D):
//!
//! - **traceability (100%)** — every cited `source_log_ref` resolves to a real
//!   frame via the participant-scoped walker; a fabricated cite does NOT.
//! - **secret-leakage (0%)** — Researcher's digest payloads pass the REAL
//!   `CorpusBackedRedactionPolicy`; a planted secret DOES fire (positive control).
//!
//! recall / faithfulness / hedge-preservation stay corpus-annotated against the
//! IAA-gold corpus (App F.5 — not live-recomputable in CI), so they are asserted
//! in `maos-eval`'s `distillate_five_metrics_floor`, not here.

use std::borrow::Cow;
use std::sync::Arc;

use researcher::{ClaimPayload, Researcher};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{LogRecallError, LogRecallFilter};
use maos_domain::ports::LogRecallPort;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::redaction::{CorpusBackedRedactionPolicy, RedactionPolicy};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};

const NONCE: u64 = 0x_5E1F_7E57;

fn claim_bytes(claim_id: &str, statement: &str) -> Vec<u8> {
    let claim = ClaimPayload {
        claim_id: claim_id.into(),
        statement: statement.into(),
        topic: "fusion".into(),
        methodology_strength: 0.9,
        confidence: 0.92,
        load_bearing: true,
        polarity: true,
        hedges: vec!["likely".into(), "uncertain".into()],
    };
    serde_json::to_vec(&claim).unwrap()
}

fn seed(tl: &Arc<TransparencyLogAdapter>, pid: u32, claims: &[(&str, &str)]) {
    for (id, statement) in claims {
        let _ = tl.insert_frame_event(
            FrameKind::InferenceCall,
            pid,
            None,
            "inform",
            &claim_bytes(id, statement),
            FrameOrigin::SpiritAuto,
        );
    }
}

#[test]
fn researcher_distillate_is_100pct_traceable() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE));
    seed(
        &tl,
        10,
        &[
            ("c0", "the effect is likely present in trial A"),
            ("c1", "the effect is uncertain in trial B"),
            ("c2", "the mechanism is plausibly mediated by X"),
        ],
    );
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let researcher = Researcher::new();

    let frames = researcher
        .walk(&adapter, 10, LogRecallFilter::default())
        .unwrap(); // pid 10 = researcher
    let survey = researcher.survey(&frames);
    assert!(!survey.findings.is_empty());

    // Traceability: every cited source_log_ref resolves to a REAL frame via the
    // walker (the same scoped fetch the kernel honors).
    let mut resolved = 0usize;
    for finding in &survey.findings {
        let id = researcher::decode_frame_id_hex(&finding.source_log_ref).expect("cite decodes");
        assert!(
            adapter.fetch(10, id).is_ok(),
            "cited frame {} must resolve",
            finding.source_log_ref
        );
        resolved += 1;
    }
    assert_eq!(
        resolved,
        survey.findings.len(),
        "traceability MUST be 100% — every cite resolves"
    );

    // Negative control: a fabricated cite does NOT resolve (so a real
    // untraceable digest would be caught, not silently passed).
    let err = adapter.fetch(10, [0xAB; 16]).unwrap_err();
    assert!(matches!(err, LogRecallError::FrameNotFound { .. }));
}

#[test]
fn researcher_digest_payload_has_zero_secret_leakage() {
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0x1));
    seed(
        &tl,
        10,
        &[
            ("c0", "the effect is likely present"),
            ("c1", "the result is plausibly robust to reanalysis"),
        ],
    );
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let researcher = Researcher::new();
    let frames = researcher
        .walk(&adapter, 10, LogRecallFilter::default())
        .unwrap();
    let survey = researcher.survey(&frames);

    // The digest payload Researcher would persist (the I11 request payload).
    let request = researcher.to_distillation_request(&survey, 1).unwrap();
    let payload_bytes = serde_json::to_vec(&request.digest_payload).unwrap();

    let policy = CorpusBackedRedactionPolicy::new();
    let redacted = policy.redact(&payload_bytes);
    // Defensive: require both allocation AND content change to count as fired,
    // avoiding false positives if the policy allocates without modifying.
    let fired = matches!(&redacted, Cow::Owned(b) if b != &payload_bytes[..]);
    assert!(
        !fired,
        "secret-leakage MUST be 0% — a clean Researcher digest must not trip redaction \
         (colon-separated frame-id cites keep hex runs < the 32-char token threshold)"
    );
}

#[test]
fn planted_secret_in_a_digest_fires_redaction() {
    // Positive control (mirrors the gate's planted-secret control): a digest
    // payload carrying a real secret pattern MUST trip the redaction filter —
    // proving the secret-leakage metric is LIVE, not a no-op. Built directly
    // (not routed through the TL — see the defense-in-depth test below for why a
    // TL-planted secret never even reaches the survey).
    use maos_domain::distillation::DigestPayload;
    use researcher::{Finding, SurveyOutput};
    use std::collections::BTreeMap;

    let secret = "sk-ant-api03-59c8e3e277515a4ee1fbde7cb68810a4";
    let out = SurveyOutput {
        findings: vec![Finding {
            claim_id: "leak".into(),
            statement: format!("the integration key {secret} appears in the source"),
            confidence: 0.9,
            hedges: vec![],
            source_log_ref: researcher::encode_frame_id_hex(&[0x11; 16]),
        }],
        open_questions: vec![],
        confidence_map: BTreeMap::new(),
        bibliography: vec![],
        scalars: BTreeMap::new(),
    };
    let payload = DigestPayload::Json(serde_json::to_value(&out).expect("survey serializes"));
    let payload_bytes = serde_json::to_vec(&payload).unwrap();

    let policy = CorpusBackedRedactionPolicy::new();
    let redacted = policy.redact(&payload_bytes);
    // Defensive: require both allocation AND content change to count as fired.
    let fired = matches!(
        &redacted, Cow::Owned(b) if b != &payload_bytes[..]
    );
    assert!(
        fired,
        "a planted sk-ant-api03- secret MUST fire redaction (live control)"
    );
}

#[test]
fn tl_write_time_redaction_scrubs_secrets_before_the_survey() {
    // Defense in depth: a secret planted in a SOURCE frame is redacted by the
    // Transparency Log's pre-write filter BEFORE Researcher ever fetches it, so
    // it cannot enter a survey. The recalled payload no longer parses as a
    // ClaimPayload (the redaction marker breaks the JSON), and crucially the raw
    // secret never appears in what the walker returns.
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(NONCE ^ 0x3));
    seed(
        &tl,
        10,
        &[(
            "leak",
            "key sk-ant-api03-59c8e3e277515a4ee1fbde7cb68810a4 leaked",
        )],
    );
    let adapter = LogRecallAdapter::new(Arc::clone(&tl));
    let frames = Researcher::new()
        .walk(&adapter, 10, LogRecallFilter::default())
        .unwrap();
    assert_eq!(frames.len(), 1);
    let payload_str = String::from_utf8_lossy(&frames[0].payload);
    assert!(
        !payload_str.contains("sk-ant-api03-59c8e3e277515a4ee1fbde7cb68810a4"),
        "the raw secret must be scrubbed by TL write-time redaction before the survey sees it"
    );
}
