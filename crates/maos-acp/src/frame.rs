//! ACP wire-protocol frame types — NDJSON over stdio.
//!
//! Per architecture §7.5 + appendix-a-cohort-prior-art-map.md (convergent
//! across openclaw / opencode / hermes; ratified by Zed).

use serde::{Deserialize, Serialize};

/// Session identifier — ULID-shaped 16-byte id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub [u8; 16]);

/// Decision identifier — ULID-shaped 16-byte id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecisionId(pub [u8; 16]);

/// Tagged-union wire frame IN from editor to ACP server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AcpFrameIn {
    SessionStart {
        session_id: SessionId,
        editor_id: String,
        editor_version: String,
    },
    SessionEnd {
        session_id: SessionId,
    },
    LifecycleVerb {
        session_id: SessionId,
        decision_id: DecisionId,
        verb: String,
        spirit_id: String,
    },
    HaltResolve {
        session_id: SessionId,
        decision_id: DecisionId,
        halt_id: String,
        resolution: String,
        operator_note: Option<String>,
    },
}

/// Tagged-union wire frame OUT from ACP server to editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AcpFrameOut {
    SessionReady {
        session_id: SessionId,
        supported_kinds: Vec<String>,
    },
    SessionTerminated {
        session_id: SessionId,
        duration_ns: u64,
    },
    LifecycleReceipt {
        decision_id: DecisionId,
        spirit_pid: u32,
        verb: String,
        timestamp_ns: u64,
        outcome: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<AcpErrorBody>,
    },
    HaltReceipt {
        decision_id: DecisionId,
        halt_id: String,
        outcome: String,
        timestamp_ns: u64,
    },
    NotificationDispatch {
        level: String,
        event: serde_json::Value,
    },
    Error {
        code: i32,
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        decision_id: Option<DecisionId>,
    },
}

/// Error body for ACP error frames.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcpErrorBody {
    pub code: i32,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_start_round_trip() {
        let frame = AcpFrameIn::SessionStart {
            session_id: SessionId([1u8; 16]),
            editor_id: "zed".into(),
            editor_version: "0.142.0".into(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("session_start"));
        let back: AcpFrameIn = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, AcpFrameIn::SessionStart { .. }));
    }

    #[test]
    fn lifecycle_verb_round_trip() {
        let frame = AcpFrameIn::LifecycleVerb {
            session_id: SessionId([2u8; 16]),
            decision_id: DecisionId([3u8; 16]),
            verb: "load".into(),
            spirit_id: "hello-spirit".into(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        let back: AcpFrameIn = serde_json::from_str(&json).unwrap();
        assert!(matches!(back, AcpFrameIn::LifecycleVerb { .. }));
    }

    #[test]
    fn session_ready_outbound() {
        let out = AcpFrameOut::SessionReady {
            session_id: SessionId([1u8; 16]),
            supported_kinds: vec!["lifecycle_verb".into(), "halt_resolve".into()],
        };
        let json = serde_json::to_string(&out).unwrap();
        assert!(json.contains("session_ready"));
    }

    #[test]
    fn unknown_frame_kind_deser() {
        let json = r#"{"kind":"unknown","data":42}"#;
        let result: Result<AcpFrameIn, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn error_frame_round_trip() {
        let err = AcpFrameOut::Error {
            code: -32600,
            message: "invalid frame".into(),
            decision_id: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("error"));
        assert!(json.contains("-32600"));
    }
}
