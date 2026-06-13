use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AuditEntry;
use super::redaction_placeholder::render_placeholder;
#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// A single frame in the trace-shape output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceFrame {
    pub frame_id: String,
    pub timestamp_ns: u64,
    pub kind: String,
    pub intent: String,
    pub shape_class: String,
    pub placeholder: Option<String>,
}

/// The complete trace-shape document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceShape {
    pub schema_version: String,
    pub source_bundle_hash: String,
    pub frame_count: usize,
    pub frames: Vec<TraceFrame>,
    pub determinism_scope: String,
}

/// Classify a kind string into a trace-shape class.
fn classify_kind(kind: &str) -> &'static str {
    match kind {
        "task.assign" | "task.complete" | "retract" | "cli.subprocess.output"
        | "cli.wrapper.shape.mismatch" => "structural",

        "capability.invocation" | "spirit.revoked" | "mcp.invocation" => "capability",

        "epistemic.halt" => "halt",

        "decision" | "decision.dispatch" | "distillate" => "decision",

        "telemetry.event" | "budget.warning" | "budget.exceeded" | "task.stalled"
        | "silent.failure.suspect" => "telemetry",

        _ => "other",
    }
}

/// Replay audit entries into a trace-shape document.
///
/// Entries are expected to already be sorted by `(timestamp_ns, frame_id)` —
/// the caller guarantees this via `query_with_redaction()`'s ORDER BY.
pub fn replay(
    entries: &[AuditEntry],
    source_bundle_canonical_bytes: &[u8],
) -> Result<TraceShape, ReplayError> {
    // Compute source bundle hash
    let mut hasher = Sha256::new();
    hasher.update(source_bundle_canonical_bytes);
    let source_bundle_hash = hex::encode(hasher.finalize());

    let frames: Vec<TraceFrame> = entries
        .iter()
        .map(|entry| {
            let placeholder = entry.redaction.as_ref().map(render_placeholder);
            TraceFrame {
                frame_id: entry.frame_id_hex.clone(),
                timestamp_ns: entry.timestamp_ns,
                kind: entry.kind.clone(),
                intent: entry.intent.clone(),
                shape_class: classify_kind(&entry.kind).to_owned(),
                placeholder,
            }
        })
        .collect();

    Ok(TraceShape {
        schema_version: "maos.trace-shape.v1".to_owned(),
        source_bundle_hash,
        frame_count: frames.len(),
        frames,
        determinism_scope: "v1.0-single-platform".to_owned(),
    })
}

/// Serialize a `TraceShape` to deterministic canonical bytes.
///
/// Delegates to the shared canonicalizer in `sealed_export` so the trace-shape
/// surface and the bundle surface use identical key-ordering rules.
pub fn replay_to_canonical_bytes(shape: &TraceShape) -> Result<Vec<u8>, ReplayError> {
    Ok(crate::sealed_export::canonicalize_value(shape))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RedactionMeta;

    fn make_entry(
        frame_id: &str,
        ts: u64,
        kind: &str,
        intent: &str,
        redaction: Option<RedactionMeta>,
    ) -> AuditEntry {
        AuditEntry {
            frame_id_hex: frame_id.to_owned(),
            timestamp_ns: ts,
            spirit_pid: 1,
            boot_nonce: 42,
            capability_token_hex: None,
            kind: kind.to_owned(),
            intent: intent.to_owned(),
            redaction,
        }
    }

    #[test]
    fn replay_empty_entries() {
        let shape = replay(&[], b"test-bundle").unwrap();
        assert_eq!(shape.schema_version, "maos.trace-shape.v1");
        assert_eq!(shape.determinism_scope, "v1.0-single-platform");
        assert_eq!(shape.frame_count, 0);
        assert!(shape.frames.is_empty());
    }

    #[test]
    fn replay_classifies_kinds() {
        let entries = vec![
            make_entry("aa", 1, "task.assign", "assign work", None),
            make_entry("bb", 2, "capability.invocation", "invoke cap", None),
            make_entry("cc", 3, "epistemic.halt", "halt detected", None),
            make_entry("dd", 4, "decision", "made decision", None),
            make_entry("ee", 5, "telemetry.event", "metric", None),
            make_entry("ff", 6, "unknown.kind", "misc", None),
        ];
        let shape = replay(&entries, b"bundle").unwrap();
        assert_eq!(shape.frames[0].shape_class, "structural");
        assert_eq!(shape.frames[1].shape_class, "capability");
        assert_eq!(shape.frames[2].shape_class, "halt");
        assert_eq!(shape.frames[3].shape_class, "decision");
        assert_eq!(shape.frames[4].shape_class, "telemetry");
        assert_eq!(shape.frames[5].shape_class, "other");
    }

    #[test]
    fn replay_redaction_placeholder() {
        let entries = vec![make_entry(
            "aa",
            1,
            "decision",
            "redacted intent",
            Some(RedactionMeta {
                class: "pii".to_owned(),
                original_len_bucket: 128,
            }),
        )];
        let shape = replay(&entries, b"bundle").unwrap();
        assert_eq!(
            shape.frames[0].placeholder.as_deref(),
            Some("<REDACTED:type=pii, len=128>")
        );
    }

    #[test]
    fn replay_no_redaction_placeholder_is_none() {
        let entries = vec![make_entry("aa", 1, "task.assign", "work", None)];
        let shape = replay(&entries, b"bundle").unwrap();
        assert!(shape.frames[0].placeholder.is_none());
    }

    #[test]
    fn replay_source_bundle_hash_is_sha256() {
        let data = b"canonical-bundle-bytes";
        let expected = {
            let mut h = Sha256::new();
            h.update(data);
            hex::encode(h.finalize())
        };
        let shape = replay(&[], data).unwrap();
        assert_eq!(shape.source_bundle_hash, expected);
    }

    #[test]
    fn replay_to_canonical_bytes_deterministic() {
        let entries = vec![
            make_entry("bb", 2, "decision", "d1", None),
            make_entry("aa", 1, "task.assign", "t1", None),
        ];
        let shape = replay(&entries, b"bundle").unwrap();
        let bytes_a = replay_to_canonical_bytes(&shape).unwrap();
        let bytes_b = replay_to_canonical_bytes(&shape).unwrap();
        assert_eq!(bytes_a, bytes_b, "canonical bytes must be deterministic");
    }

    #[test]
    fn replay_canonical_bytes_keys_sorted() {
        let entries = vec![make_entry("aa", 1, "task.assign", "work", None)];
        let shape = replay(&entries, b"bundle").unwrap();
        let bytes = replay_to_canonical_bytes(&shape).unwrap();
        let text = String::from_utf8(bytes).unwrap();

        // Top-level keys must appear in sorted order
        let det_pos = text.find("\"determinism_scope\"").unwrap();
        let fc_pos = text.find("\"frame_count\"").unwrap();
        let fr_pos = text.find("\"frames\"").unwrap();
        let sv_pos = text.find("\"schema_version\"").unwrap();
        let sb_pos = text.find("\"source_bundle_hash\"").unwrap();

        assert!(det_pos < fc_pos, "determinism_scope before frame_count");
        assert!(fc_pos < fr_pos, "frame_count before frames");
        assert!(fr_pos < sv_pos, "frames before schema_version");
        assert!(sv_pos < sb_pos, "schema_version before source_bundle_hash");
    }

    #[test]
    fn replay_canonical_bytes_placeholder_null_when_none() {
        let entries = vec![make_entry("aa", 1, "task.assign", "work", None)];
        let shape = replay(&entries, b"bundle").unwrap();
        let bytes = replay_to_canonical_bytes(&shape).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        // placeholder is serialized as null (schema requires the field)
        assert!(
            text.contains("\"placeholder\":null"),
            "placeholder must be null when None, got: {text}"
        );
    }

    #[test]
    fn classify_all_structural_kinds() {
        assert_eq!(classify_kind("task.assign"), "structural");
        assert_eq!(classify_kind("task.complete"), "structural");
        assert_eq!(classify_kind("retract"), "structural");
        assert_eq!(classify_kind("cli.subprocess.output"), "structural");
        assert_eq!(classify_kind("cli.wrapper.shape.mismatch"), "structural");
    }

    #[test]
    fn classify_all_capability_kinds() {
        assert_eq!(classify_kind("capability.invocation"), "capability");
        assert_eq!(classify_kind("spirit.revoked"), "capability");
        assert_eq!(classify_kind("mcp.invocation"), "capability");
    }

    #[test]
    fn classify_all_decision_kinds() {
        assert_eq!(classify_kind("decision"), "decision");
        assert_eq!(classify_kind("decision.dispatch"), "decision");
        assert_eq!(classify_kind("distillate"), "decision");
    }

    #[test]
    fn classify_all_telemetry_kinds() {
        assert_eq!(classify_kind("telemetry.event"), "telemetry");
        assert_eq!(classify_kind("budget.warning"), "telemetry");
        assert_eq!(classify_kind("budget.exceeded"), "telemetry");
        assert_eq!(classify_kind("task.stalled"), "telemetry");
        assert_eq!(classify_kind("silent.failure.suspect"), "telemetry");
    }
}
