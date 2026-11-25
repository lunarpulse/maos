#![forbid(unsafe_code)]

//! Distillation domain types per AC2 — `DistillationRequest`, `DigestPayload`,
//! `SegmentHint`, `DistillationReceipt`, `DistillationError`.
//!
//! These are the pure domain shape types consumed by `DistillationPort` and
//! implemented by `DistillateWriter` in `maos-kernel-core`.

use std::borrow::Cow;

use crate::invariants::i13::IntentLineage;
use thiserror::Error;

/// Story 28-3 (D-28-3-04 C) — the one strict reading of a survey completion,
/// shared by the Researcher Spirit and the eval scorer so they cannot
/// disagree: exactly one JSON object whose only field `finding` is a string.
/// Unknown, duplicate or missing fields, arrays, scalars, code fences and
/// trailing material are all `None` (never a panic). So is a finding that is
/// blank after trimming (it would be "faithful" to any claim and pass recall,
/// traceability and leakage) or that holds a control character, newline and
/// tab included: the prompt asks for ONE sentence, and a second paragraph is
/// the meta-text the format floor exists to count. The string borrows from
/// `text` unless it held escapes. Beyond that it checks SHAPE only: a
/// schema-valid single line may still hold several sentences. Format
/// characters that let a persisted finding spoof what a reader sees are
/// refused as well: bidi embeddings/overrides/isolates (U+202A–U+202E,
/// U+2066–U+2069), zero-width and joiner characters (U+200B–U+200D, U+2060,
/// U+FEFF) and the line/paragraph separators (U+2028, U+2029).
pub fn parse_finding_completion(text: &str) -> Option<Cow<'_, str>> {
    serde_json::from_str::<StrictFinding<'_>>(text)
        .ok()
        .map(|f| f.0)
        .filter(|finding| {
            !finding.trim().is_empty() && !finding.chars().any(|c| c.is_control() || spoofs(c))
        })
}

/// A format character a finding may not carry (see [`parse_finding_completion`]).
fn spoofs(c: char) -> bool {
    matches!(
        c,
        '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{200B}'..='\u{200D}'
            | '\u{2060}'
            | '\u{FEFF}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// A JSON string borrowed when escape-free.
#[derive(serde::Deserialize)]
struct JsonStr<'a>(#[serde(borrow)] Cow<'a, str>);

struct StrictFinding<'a>(Cow<'a, str>);

impl<'de> serde::Deserialize<'de> for StrictFinding<'de> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{Error, MapAccess, Visitor};

        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = StrictFinding<'de>;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an object with a single string field `finding`")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut finding = None;
                while let Some(JsonStr(key)) = map.next_key::<JsonStr<'de>>()? {
                    if key != "finding" {
                        return Err(A::Error::custom("unknown field"));
                    }
                    if finding.is_some() {
                        return Err(A::Error::duplicate_field("finding"));
                    }
                    finding = Some(map.next_value::<JsonStr<'de>>()?.0);
                }
                finding
                    .map(StrictFinding)
                    .ok_or_else(|| A::Error::missing_field("finding"))
            }
        }
        // `deserialize_map`, not `deserialize_struct`: a struct would also
        // accept a JSON array as positional fields.
        deserializer.deserialize_map(V)
    }
}

/// A Spirit's request to persist a digest with kernel-enforced I11 audit chain.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DistillationRequest {
    /// Source raw-frame IDs (flattened — see DistillateWriter::flatten_source_log_ref).
    /// MUST be non-empty; the kernel rejects empty with `DistillationError::AuditChainMissing`.
    #[doc = "Construct via [`DistillationRequest::new`] to enforce validation; struct literals bypass source-log-ref non-empty / depth-≥-1 / payload-checks."]
    pub source_log_ref: Vec<[u8; 16]>,

    /// Monotonic depth — 0 for raw, increases by 1 per distillation hop.
    /// Spirit-supplied; kernel rejects `< 1` (digest writes carry depth ≥ 1).
    #[doc = "Construct via [`DistillationRequest::new`] to enforce validation; struct literals bypass source-log-ref non-empty / depth-≥-1 / payload-checks."]
    pub distillation_depth: u32,

    /// The digest content (Spirit-side LLM-compressed payload).
    /// The kernel does NOT inspect, parse, or summarize this per §4.0.7.
    #[doc = "Construct via [`DistillationRequest::new`] to enforce validation; struct literals bypass source-log-ref non-empty / depth-≥-1 / payload-checks."]
    pub digest_payload: DigestPayload,

    /// Optional segment-granularity hint (architecture I11: segment-level is default).
    /// `None` means segment = full source_log_ref range.
    #[doc = "Construct via [`DistillationRequest::new`] to enforce validation; struct literals bypass source-log-ref non-empty / depth-≥-1 / payload-checks."]
    #[serde(default)]
    pub segment_hint: Option<SegmentHint>,
}

impl DistillationRequest {
    /// Construct a validated distillation request.
    /// Returns Err if `source_log_ref` is empty or `distillation_depth < 1`.
    /// This is a defensive author-side check; the kernel ALSO validates at write time.
    pub fn new(
        source_log_ref: Vec<[u8; 16]>,
        distillation_depth: u32,
        digest_payload: DigestPayload,
        segment_hint: Option<SegmentHint>,
    ) -> Result<Self, DistillationError> {
        if source_log_ref.is_empty() {
            return Err(DistillationError::AuditChainMissing {
                reason: "empty source_log_ref".into(),
            });
        }
        if distillation_depth < 1 {
            return Err(DistillationError::AuditChainMissing {
                reason: "distillation_depth < 1".into(),
            });
        }
        Ok(Self {
            source_log_ref,
            distillation_depth,
            digest_payload,
            segment_hint,
        })
    }
}

/// The digest content — discriminated payload so the kernel can serialize
/// appropriately without interpreting the contents.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DigestPayload {
    /// Spirit-authored text digest (LLM compression output).
    Text(String),
    /// Structured digest (e.g., a serde-Json summary).
    Json(serde_json::Value),
}

/// Optional segment-granularity hint per architecture I11.
/// `None` on `DistillationRequest` means segment = full source_log_ref range.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SegmentHint {
    #[doc = "Construct via [`SegmentHint::new`] to enforce validation; struct literals bypass segment-bounds checks."]
    pub segment_start_frame_id: [u8; 16],
    #[doc = "Construct via [`SegmentHint::new`] to enforce validation; struct literals bypass segment-bounds checks."]
    pub segment_end_frame_id: [u8; 16],
}

impl SegmentHint {
    pub fn new(segment_start_frame_id: [u8; 16], segment_end_frame_id: [u8; 16]) -> Self {
        Self {
            segment_start_frame_id,
            segment_end_frame_id,
        }
    }
}

/// Receipt returned after the kernel successfully persists a digest.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DistillationReceipt {
    #[doc = "Construct via [`DistillationReceipt::new`] to enforce validation; struct literals bypass consistency checks."]
    pub digest_frame_id: [u8; 16],

    /// Kernel-computed intent lineage (I13 — union of intent classes of
    /// every frame in source_log_ref, looked up at write time).
    #[doc = "Construct via [`DistillationReceipt::new`] to enforce validation; struct literals bypass consistency checks."]
    pub intent_lineage: IntentLineage,

    /// Effective source_log_ref after transitive flattening (digests-of-digests
    /// are flattened to original raws).
    #[doc = "Construct via [`DistillationReceipt::new`] to enforce validation; struct literals bypass consistency checks."]
    pub effective_source_log_ref: Vec<[u8; 16]>,

    /// Effective depth = max(input_frame_depths) + 1.
    #[doc = "Construct via [`DistillationReceipt::new`] to enforce validation; struct literals bypass consistency checks."]
    pub effective_distillation_depth: u32,

    /// Wall-clock timestamp at write time.
    #[doc = "Construct via [`DistillationReceipt::new`] to enforce validation; struct literals bypass consistency checks."]
    pub timestamp_ns: u64,
}

impl DistillationReceipt {
    pub fn new(
        digest_frame_id: [u8; 16],
        intent_lineage: IntentLineage,
        effective_source_log_ref: Vec<[u8; 16]>,
        effective_distillation_depth: u32,
        timestamp_ns: u64,
    ) -> Self {
        Self {
            digest_frame_id,
            intent_lineage,
            effective_source_log_ref,
            effective_distillation_depth,
            timestamp_ns,
        }
    }
}

/// Typed error for distillation operations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum DistillationError {
    /// I11 enforcement: required audit-chain field missing or invalid.
    /// Renders as `EDigestAuditChainMissing` in user-facing logs.
    #[error("E_DIGEST_AUDIT_CHAIN_MISSING — {reason}")]
    AuditChainMissing { reason: String },

    /// I13 enforcement: consumer's allowed_promotion_set does not contain
    /// the digest's intent_lineage. Renders as `EIntentPromotionDenied`.
    #[error("E_INTENT_PROMOTION_DENIED — digest {digest_frame_id:?} carries intents not allowed by consumer")]
    IntentPromotionDenied { digest_frame_id: [u8; 16] },

    /// A source frame_id in the request was not found in the Transparency Log.
    #[error("source frame {frame_id:?} not found in transparency log")]
    SourceFrameNotFound { frame_id: [u8; 16] },

    /// I11 citer-authorization (Story 8.10 AC2): the citing Spirit is not
    /// permitted to reference one of the (flattened) source frames — a
    /// cross-principal citation. The forged distillate is rejected and NO
    /// `Distillate` row is written. Closes the `researcher::to_distillation_request`
    /// forgery path.
    #[error(
        "E_DIGEST_CITER_UNAUTHORIZED — citer pid {citer_pid} may not cite frame \
         {frame_id:?} owned by principal pid {source_pid}"
    )]
    CiterUnauthorized {
        citer_pid: u32,
        source_pid: u32,
        frame_id: [u8; 16],
    },

    /// SQLite or IO error during digest write or source-frame lookup.
    #[error("storage error: {0}")]
    Storage(String),

    /// FR13 — a quarantined pid's distillate write is refused before any row is written.
    #[error("spirit_pid {spirit_pid} is quarantined; distillate write refused")]
    Quarantined { spirit_pid: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_new_rejects_empty_source() {
        let result = DistillationRequest::new(vec![], 1, DigestPayload::Text("test".into()), None);
        assert!(matches!(
            result,
            Err(DistillationError::AuditChainMissing { .. })
        ));
    }

    #[test]
    fn request_new_rejects_depth_zero() {
        let result =
            DistillationRequest::new(vec![[1u8; 16]], 0, DigestPayload::Text("test".into()), None);
        assert!(matches!(
            result,
            Err(DistillationError::AuditChainMissing { .. })
        ));
    }

    #[test]
    fn request_new_accepts_valid() {
        let result = DistillationRequest::new(
            vec![[1u8; 16], [2u8; 16]],
            2,
            DigestPayload::Json(serde_json::json!({"k": "v"})),
            Some(SegmentHint::new([1u8; 16], [2u8; 16])),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn receipt_serde_round_trip() {
        use crate::invariants::i8::A2AIntent;
        let receipt = DistillationReceipt::new(
            [0xAB; 16],
            IntentLineage::new(vec![A2AIntent::new("consult")]),
            vec![[1u8; 16]],
            1,
            500,
        );
        let json = serde_json::to_string(&receipt).unwrap();
        let back: DistillationReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(back.digest_frame_id, [0xAB; 16]);
        assert_eq!(back.effective_distillation_depth, 1);
    }

    #[test]
    fn digst_payload_serde_round_trip() {
        let payload = DigestPayload::Text("hello world".into());
        let json = serde_json::to_string(&payload).unwrap();
        let back: DigestPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(back, DigestPayload::Text("hello world".into()));
    }

    #[test]
    fn distillation_error_audit_chain_missing_display() {
        let err = DistillationError::AuditChainMissing {
            reason: "empty source_log_ref".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("E_DIGEST_AUDIT_CHAIN_MISSING"));
        assert!(display.contains("empty source_log_ref"));
    }

    #[test]
    fn distillation_error_distinguishes_variants() {
        assert_ne!(
            DistillationError::AuditChainMissing { reason: "a".into() },
            DistillationError::IntentPromotionDenied {
                digest_frame_id: [0u8; 16]
            },
        );
    }

    #[test]
    fn finding_completion_accepts_one_object_with_a_string_finding() {
        assert_eq!(
            parse_finding_completion(r#"{"finding":"plain"}"#).as_deref(),
            Some("plain")
        );
        // Escapes decode (owned); surrounding JSON whitespace is legal.
        assert_eq!(
            parse_finding_completion(" {\"finding\":\"a \\\"q\\\" \\u00e9\"}\n").as_deref(),
            Some("a \"q\" é")
        );
        assert!(matches!(
            parse_finding_completion(r#"{"finding":"plain"}"#),
            Some(Cow::Borrowed(_))
        ));
    }

    #[test]
    fn finding_completion_rejects_every_other_shape_without_panicking() {
        for bad in [
            "",
            "plain text",
            // Blank, whitespace-only, control-character and multi-line findings
            // are format errors (round 2, H).
            r#"{"finding":""}"#,
            r#"{"finding":"  "}"#,
            r#"{"finding":" \t "}"#,
            r#"{"finding":"\u0000"}"#,
            r#"{"finding":"a\u0007b"}"#,
            r#"{"finding":"one sentence.\nA second paragraph."}"#,
            r#"{"finding":"tab\tseparated"}"#,
            // Round 3 (9): format characters that spoof a persisted finding.
            r#"{"finding":"a\u202Eb"}"#,
            r#"{"finding":"a\u202Ab"}"#,
            r#"{"finding":"a\u2066b"}"#,
            r#"{"finding":"a\u2069b"}"#,
            r#"{"finding":"a\u200Bb"}"#,
            r#"{"finding":"a\u200Db"}"#,
            r#"{"finding":"a\u2060b"}"#,
            r#"{"finding":"a\uFEFFb"}"#,
            r#"{"finding":"a\u2028b"}"#,
            r#"{"finding":"a\u2029b"}"#,
            r#"{"finding":"\u200B"}"#,
            r#"{}"#,
            r#"{"summary":"x"}"#,
            r#"{"other":"x"}"#,
            r#"{"finding":"x","extra":1}"#,
            r#"{"extra":1,"finding":"x"}"#,
            r#"{"finding":"x","finding":"y"}"#,
            r#"{"finding":1}"#,
            r#"{"finding":null}"#,
            r#"["x"]"#,
            r#"[{"finding":"x"}]"#,
            r#""x""#,
            "```json\n{\"finding\":\"x\"}\n```",
            r#"{"finding":"x"} trailing"#,
            r#"{"finding":"x"}{"finding":"y"}"#,
            r#"{"finding":"x""#,
        ] {
            assert!(parse_finding_completion(bad).is_none(), "{bad}");
        }
    }
}
