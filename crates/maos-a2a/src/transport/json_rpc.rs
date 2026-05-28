//! JSON-RPC 2.0 framing per FR23b.
//!
//! Per FR47 the framing is HAND-ROLLED via `serde_json` — NO
//! `jsonrpc-core` / `jsonrpsee` crate. At v0.5 the framing supports a SINGLE
//! method `"iac.deliver"`; the payload is the same `IacFrame` shape used on
//! the same-Host bus.
//!
//! Error code mapping (per AC3):
//!   * `-32700` JSON-RPC parse error (malformed payload)
//!   * `-32600` invalid request (missing `jsonrpc` / `method` / `params`)
//!   * `-32601` method not found
//!   * `-32001` `EIntentDenied`
//!   * `-32002` TOFU `EPinMismatch::NotPinned`
//!   * `-32003` Consent envelope expired
//!   * `-32004` `SpiritRestartDetected` — peer's `boot_nonce` rolled (NFR-Rel-6)
//!   * `-32099` Other A2AError catch-all (with the variant's message)

use maos_domain::frame::IacFrame;
use serde::{Deserialize, Serialize};

pub const JSONRPC_VERSION: &str = "2.0";

/// The only RPC method at v0.5.
pub const METHOD_IAC_DELIVER: &str = "iac.deliver";

/// Error code constants.
pub const CODE_PARSE_ERROR: i32 = -32700;
pub const CODE_INVALID_REQUEST: i32 = -32600;
pub const CODE_METHOD_NOT_FOUND: i32 = -32601;
pub const CODE_INTENT_DENIED: i32 = -32001;
pub const CODE_PIN_MISMATCH_NOT_PINNED: i32 = -32002;
pub const CODE_CONSENT_EXPIRED: i32 = -32003;
/// Story 6.3 §A1 P6 (Epic 6 retro 2026-05-28) — peer's Spirit `boot_nonce`
/// has rolled relative to the stored TOFU pin; the receiver MUST invalidate
/// the prior pin and refuse the frame. NFR-Rel-6 detection floor.
pub const CODE_SPIRIT_RESTART_DETECTED: i32 = -32004;
pub const CODE_INTERNAL: i32 = -32099;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AJsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: IacFrame,
    pub id: u64,
    /// Sender's Spirit `boot_nonce` at send time. `0` = unspecified
    /// (backward-compat with v0.5-α loopback callers that pre-date this
    /// field — they admit without restart-detection). Cross-Host v0.7+ MUST
    /// populate from `Spirit::boot_nonce()`; receivers compare against
    /// `TofuPin.boot_nonce` and fire `CODE_SPIRIT_RESTART_DETECTED` on
    /// mismatch. Added in Story 6.3 §A1 P6 (Epic 6 retro 2026-05-28).
    #[serde(default)]
    pub boot_nonce: u64,
}

/// JSON-RPC 2.0 response. Untagged (field-based) deserialization
/// matches JSON-RPC 2.0 wire format where Ack has `result` and Nack
/// has `error`. Nack is ordered first — if a malformed response
/// contains both fields (protocol violation), the error path wins
/// (defense-in-depth: reject rather than silently accept).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum A2AJsonRpcResponse {
    Nack(NackResponse),
    Ack(AckResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckResponse {
    pub jsonrpc: String,
    pub result: AckBody,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NackResponse {
    pub jsonrpc: String,
    pub error: NackError,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckBody {
    pub delivered: bool,
    /// Receiver's observed `logical_clock` value post-`recv_advance`.
    pub receiver_logical_clock: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NackError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl A2AJsonRpcRequest {
    pub fn new(method: &str, params: IacFrame, id: u64) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.to_string(),
            params,
            id,
            boot_nonce: 0,
        }
    }

    /// Set the sender's Spirit `boot_nonce`. Cross-Host v0.7+ callers MUST
    /// invoke this with the live nonce so receivers can detect Spirit
    /// restarts (NFR-Rel-6). v0.5-α loopback callers may leave the default
    /// `0` — the receiver treats it as "unspecified" and skips the
    /// restart-detection check.
    pub fn with_boot_nonce(mut self, boot_nonce: u64) -> Self {
        self.boot_nonce = boot_nonce;
        self
    }

    /// Story 6.3 §A1 P7 (Epic 6 retro 2026-05-28) — parse a raw byte slice
    /// into an `A2AJsonRpcRequest`, emitting a JSON-RPC-compliant
    /// `CODE_PARSE_ERROR (-32700)` NACK on `serde_json::from_slice` failure.
    ///
    /// Cross-Host v0.7+ TCP transports MUST funnel inbound bytes through
    /// this helper before invoking `handle_intake`; the helper guarantees
    /// that malformed JSON yields a structured NACK (with `id = 0` per
    /// JSON-RPC 2.0 §5.1 since the offending request had no parseable id)
    /// rather than a raw `serde_json::Error` propagating up the stack.
    ///
    /// Loopback v0.5-α deliberately bypasses byte-level framing (the
    /// router routes typed values via `tokio::sync::mpsc`), so this helper
    /// is exercised by unit tests at v0.5 and by the cross-Host TCP
    /// connector at v0.7.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, NackResponse> {
        match serde_json::from_slice::<A2AJsonRpcRequest>(bytes) {
            Ok(req) => Ok(req),
            Err(e) => Err(NackResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                error: NackError {
                    code: CODE_PARSE_ERROR,
                    message: format!("JSON parse error: {e}"),
                    data: None,
                },
                // JSON-RPC 2.0 §5.1: id is null when not parseable from
                // request. We use `0` as the typed-zero sentinel since
                // `id: u64`. Transports concerned with strict spec
                // compliance should map this back to JSON `null` on the
                // outbound wire.
                id: 0,
            }),
        }
    }

    /// Validate the framing per JSON-RPC 2.0 + AC3.
    pub fn validate(&self) -> Result<(), NackError> {
        if self.jsonrpc != JSONRPC_VERSION {
            return Err(NackError {
                code: CODE_INVALID_REQUEST,
                message: format!("jsonrpc must be {JSONRPC_VERSION}; got {}", self.jsonrpc),
                data: None,
            });
        }
        if self.method != METHOD_IAC_DELIVER {
            return Err(NackError {
                code: CODE_METHOD_NOT_FOUND,
                message: format!("method must be {METHOD_IAC_DELIVER}; got {}", self.method),
                data: None,
            });
        }
        Ok(())
    }
}

impl A2AJsonRpcResponse {
    pub fn ack(id: u64, body: AckBody) -> Self {
        A2AJsonRpcResponse::Ack(AckResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: body,
            id,
        })
    }

    pub fn nack(id: u64, code: i32, message: impl Into<String>) -> Self {
        A2AJsonRpcResponse::Nack(NackResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            error: NackError {
                code,
                message: message.into(),
                data: None,
            },
            id,
        })
    }

    pub fn id(&self) -> u64 {
        match self {
            A2AJsonRpcResponse::Ack(a) => a.id,
            A2AJsonRpcResponse::Nack(n) => n.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::frame::{FrameAddress, FramePayload, TaskAssignPayload, PosturePreferences};
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_spirit_abi::identity::{FrameKind, SpiritId};
    use smallvec::smallvec;

    fn make_frame() -> IacFrame {
        IacFrame {
            frame_id: [1u8; 16],
            timestamp_ns: 0,
            logical_clock: 7,
            from: FrameAddress {
                spirit_id: SpiritId::from("a"),
                host_id: None,
                role: None,
            },
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from("b"),
                host_id: None,
                role: None,
            }],
            kind: FrameKind::TaskAssign,
            intent: IntentClass::Standard,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "g".into(),
                scope: vec![],
                success_criteria: "ok".into(),
                posture_preferences: PosturePreferences::default(),
                prior_distillate_ref: None,
            }),
            auto_marker: FrameOrigin::HumanAuthored,
            consent_envelope: None,
            intent_lineage: IntentLineage::default(),
        }
    }

    #[test]
    fn request_round_trip() {
        let frame = make_frame();
        let req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame, 42);
        req.validate().expect("valid");
        let json = serde_json::to_string(&req).expect("serialize");
        let back: A2AJsonRpcRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, 42);
        assert_eq!(back.method, METHOD_IAC_DELIVER);
        assert_eq!(back.params.logical_clock, 7);
    }

    #[test]
    fn request_validate_rejects_bad_jsonrpc_version() {
        let frame = make_frame();
        let mut req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame, 1);
        req.jsonrpc = "1.0".into();
        let err = req.validate().expect_err("must reject");
        assert_eq!(err.code, CODE_INVALID_REQUEST);
    }

    #[test]
    fn request_validate_rejects_unknown_method() {
        let frame = make_frame();
        let mut req = A2AJsonRpcRequest::new(METHOD_IAC_DELIVER, frame, 1);
        req.method = "iac.evict".into();
        let err = req.validate().expect_err("must reject");
        assert_eq!(err.code, CODE_METHOD_NOT_FOUND);
    }

    #[test]
    fn ack_response_round_trip() {
        let ack = A2AJsonRpcResponse::ack(
            1,
            AckBody {
                delivered: true,
                receiver_logical_clock: 99,
            },
        );
        let json = serde_json::to_string(&ack).expect("serialize");
        let back: A2AJsonRpcResponse = serde_json::from_str(&json).expect("deserialize");
        match back {
            A2AJsonRpcResponse::Ack(a) => {
                assert_eq!(a.id, 1);
                assert_eq!(a.result.receiver_logical_clock, 99);
            }
            _ => panic!("expected Ack"),
        }
    }

    #[test]
    fn nack_response_round_trip() {
        let nack = A2AJsonRpcResponse::nack(2, CODE_INTENT_DENIED, "intent x denied");
        let json = serde_json::to_string(&nack).expect("serialize");
        let back: A2AJsonRpcResponse = serde_json::from_str(&json).expect("deserialize");
        match back {
            A2AJsonRpcResponse::Nack(n) => {
                assert_eq!(n.error.code, CODE_INTENT_DENIED);
                assert_eq!(n.id, 2);
            }
            _ => panic!("expected Nack"),
        }
    }

    #[test]
    fn parse_error_serializes_minimally() {
        let nack = A2AJsonRpcResponse::nack(0, CODE_PARSE_ERROR, "bad json");
        let json = serde_json::to_string(&nack).expect("serialize");
        assert!(json.contains("-32700"));
    }
}
