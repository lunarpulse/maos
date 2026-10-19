//! Private in-band recall records: exactly one tagged CBOR item per ADR-032 body.
//!
//! Struct fields and enum representations have fixed serialization order (no
//! unordered maps); preferred-width integers and definite lengths come from
//! ciborium. Both encode and decode pass through the bounded frame codec.
use ciborium::tag::Required;
use serde::{Deserialize, Serialize};

/// Private, unregistered tag in RFC 8949's first-come-first-served range.
pub const RECALL_REQUEST_TAG: u64 = 60000;
/// Private reply tag in the same range; not a registered interchange format.
pub const RECALL_REPLY_TAG: u64 = 60001;
pub const REPLY_BYTE_BOUND: u64 = maos_frame_codec::MAX_FRAME_BYTES as u64;

/// A CBOR byte string, never a node-per-byte serde sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireBytes(pub Vec<u8>);
impl Serialize for WireBytes {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}
impl<'de> Deserialize<'de> for WireBytes {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = WireBytes;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a CBOR byte string")
            }
            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<WireBytes, E> {
                Ok(WireBytes(bytes.to_vec()))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, bytes: Vec<u8>) -> Result<WireBytes, E> {
                Ok(WireBytes(bytes))
            }
        }
        deserializer.deserialize_byte_buf(Visitor)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireCursor {
    pub last_timestamp_ns: u64,
    pub last_frame_id: WireBytes,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireFilter {
    pub kind: Option<String>,
    pub since_ns: Option<u64>,
    pub until_ns: Option<u64>,
    pub limit: u32,
    pub cursor: Option<WireCursor>,
    pub intent_filter: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireEntry {
    pub frame_id: WireBytes,
    pub timestamp_ns: u64,
    pub kind: String,
    pub intent: String,
    pub payload_available: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RecallRequest {
    Recall { filter: WireFilter },
    Fetch { frame_id: WireBytes },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireError {
    ScopeViolation,
    FrameNotFound,
    InvalidRequest(String),
    LimitExceeded(u32),
    CapabilityDenied,
    ConsentWithheld,
    ReplyTooLarge(u64),
    Unavailable,
}
impl WireError {
    /// WIT case name, also used by the daemon's refusal receipts.
    pub fn code(&self) -> &'static str {
        match self {
            Self::ScopeViolation => "scope-violation",
            Self::FrameNotFound => "frame-not-found",
            Self::InvalidRequest(_) => "invalid-request",
            Self::LimitExceeded(_) => "limit-exceeded",
            Self::CapabilityDenied => "capability-denied",
            Self::ConsentWithheld => "consent-withheld",
            Self::ReplyTooLarge(_) => "reply-too-large",
            Self::Unavailable => "unavailable",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum RecallReply {
    Page {
        entries: Vec<WireEntry>,
        next_cursor: Option<WireCursor>,
    },
    Fetched {
        frame_id: WireBytes,
        timestamp_ns: u64,
        kind: String,
        intent: String,
        payload: WireBytes,
        /// WIT frame-origin case name, not a separate domain spelling table.
        origin: String,
    },
    Error(WireError),
}

/// Dispatch on CBOR major type 6, including unknown tags (which must be refused).
pub fn is_tagged(body: &[u8]) -> bool {
    body.first().is_some_and(|byte| byte >> 5 == 6)
}
pub fn encode_request(request: &RecallRequest) -> Result<Vec<u8>, String> {
    crate::codec::encode_cbor(&Required::<_, RECALL_REQUEST_TAG>(request))
}
pub fn decode_request(body: &[u8]) -> Result<RecallRequest, String> {
    crate::codec::decode_cbor::<Required<RecallRequest, RECALL_REQUEST_TAG>>(body)
        .map(|value| value.0)
}
pub fn encode_reply(reply: &RecallReply) -> Result<Vec<u8>, String> {
    crate::codec::encode_cbor(&Required::<_, RECALL_REPLY_TAG>(reply))
}
pub fn decode_reply(body: &[u8]) -> Result<RecallReply, String> {
    crate::codec::decode_cbor::<Required<RecallReply, RECALL_REPLY_TAG>>(body).map(|value| value.0)
}

/// Encode a fetched response without truncating its payload. Both the byte and
/// CBOR expansion budgets are enforced; budget failures are safe wire errors.
pub fn encode_fetched_reply(reply: &RecallReply) -> Result<Vec<u8>, WireError> {
    encode_reply(reply).map_err(|_| WireError::ReplyTooLarge(REPLY_BYTE_BOUND))
}

/// Longest fitting prefix under BOTH codec budgets. Search downward, not by
/// bisection: changing the resume cursor need not change encoded size monotonically.
/// Refuse an unencodable first entry rather than returning a non-progress cursor.
pub fn fit_page(
    mut entries: Vec<WireEntry>,
    next_cursor: Option<WireCursor>,
) -> Result<RecallReply, WireError> {
    #[derive(Serialize)]
    enum PageRef<'a> {
        Page {
            entries: &'a [WireEntry],
            next_cursor: &'a Option<WireCursor>,
        },
    }
    let cursor_for = |entries: &[WireEntry]| {
        entries.last().map(|entry| WireCursor {
            last_timestamp_ns: entry.timestamp_ns,
            last_frame_id: entry.frame_id.clone(),
        })
    };
    let original_len = entries.len();
    for count in (1..=original_len)
        .rev()
        .chain((original_len == 0).then_some(0))
    {
        let cursor = if count == original_len {
            next_cursor.clone()
        } else {
            cursor_for(&entries[..count])
        };
        if crate::codec::encode_cbor(&Required::<_, RECALL_REPLY_TAG>(PageRef::Page {
            entries: &entries[..count],
            next_cursor: &cursor,
        }))
        .is_ok()
        {
            entries.truncate(count);
            return Ok(RecallReply::Page {
                entries,
                next_cursor: cursor,
            });
        }
    }
    Err(WireError::ReplyTooLarge(REPLY_BYTE_BOUND))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: u8, intent_len: usize) -> WireEntry {
        WireEntry {
            frame_id: WireBytes(vec![id; 16]),
            timestamp_ns: id.into(),
            kind: "TaskAssign".into(),
            intent: "x".repeat(intent_len),
            payload_available: true,
        }
    }
    fn fetched(len: usize) -> RecallReply {
        RecallReply::Fetched {
            frame_id: WireBytes(vec![7; 16]),
            timestamp_ns: 1,
            kind: "TaskAssign".into(),
            intent: "Standard".into(),
            payload: WireBytes(vec![42; len]),
            origin: "kernel".into(),
        }
    }
    #[test]
    fn tags_discriminate_requests_replies_and_iac_frames() {
        use maos_domain::frame::*;
        let frame = IacFrame {
            frame_id: [7; 16],
            timestamp_ns: 1,
            logical_clock: 1,
            from: FrameAddress {
                spirit_id: "test".into(),
                host_id: None,
                role: None,
            },
            to: Default::default(),
            kind: maos_spirit_abi::identity::FrameKind::TaskAssign,
            intent: maos_domain::invariants::i1::IntentClass::Standard,
            payload: FramePayload::TaskAssign(TaskAssignPayload {
                goal: "test".into(),
                scope: vec![],
                success_criteria: "test".into(),
                posture_preferences: Default::default(),
                prior_distillate_ref: None,
            }),
            auto_marker: maos_domain::invariants::i3::FrameOrigin::Kernel,
            consent_envelope: None,
            intent_lineage: Default::default(),
        };
        assert!(!is_tagged(&crate::codec::encode_cbor(&frame).unwrap()));
        assert!(!is_tagged(&[]));
        let request = RecallRequest::Fetch {
            frame_id: WireBytes(vec![7; 16]),
        };
        let encoded = encode_request(&request).unwrap();
        assert!(is_tagged(&encoded));
        assert_eq!(decode_request(&encoded).unwrap(), request);
        assert!(decode_reply(&encoded).is_err());
        let reply = RecallReply::Error(WireError::ConsentWithheld);
        let encoded = encode_reply(&reply).unwrap();
        assert_eq!(decode_reply(&encoded).unwrap(), reply);
        assert!(decode_request(&encoded).is_err());
        assert!(decode_reply(&[0xd9, 0xea, 0x62, 0xf6]).is_err());
    }
    #[test]
    fn megabyte_payload_is_one_byte_string_not_a_sequence() {
        let reply = fetched(1024 * 1024);
        let bytes = encode_fetched_reply(&reply).unwrap();
        assert!(bytes.len() > 1024 * 1024 && bytes.len() < 1024 * 1024 + 256);
        assert_eq!(decode_reply(&bytes).unwrap(), reply);
        assert!(crate::codec::encode_cbor(&vec![42u8; 1024 * 1024])
            .unwrap_err()
            .contains("container expansion budget exceeded"));
        assert!(crate::codec::decode_cbor::<WireBytes>(&[0x82, 1, 2]).is_err());
    }
    #[test]
    fn page_prefix_at_exact_expansion_budget_boundary() {
        // Every item costs 1024 bytes plus its string/byte length. Find the
        // precise threshold once, then exercise the helper on either side.
        let fits = |len| {
            encode_reply(&RecallReply::Page {
                entries: vec![entry(1, len)],
                next_cursor: Some(WireCursor {
                    last_timestamp_ns: 1,
                    last_frame_id: WireBytes(vec![1; 16]),
                }),
            })
            .is_ok()
        };
        let mut low = 0;
        let mut high = REPLY_BYTE_BOUND as usize;
        while low + 1 < high {
            let mid = (low + high) / 2;
            if fits(mid) {
                low = mid;
            } else {
                high = mid;
            }
        }
        assert!(fits(low));
        assert!(!fits(low + 1));
        let page = fit_page(vec![entry(1, low), entry(2, 0)], None).unwrap();
        match page {
            RecallReply::Page {
                entries,
                next_cursor,
            } => {
                assert_eq!(entries.len(), 1);
                assert_eq!(next_cursor.unwrap().last_frame_id, WireBytes(vec![1; 16]));
            }
            _ => panic!("expected page"),
        }
        assert_eq!(
            fit_page(
                vec![entry(1, low + 1)],
                Some(WireCursor {
                    last_timestamp_ns: 1,
                    last_frame_id: WireBytes(vec![1; 16])
                })
            ),
            Err(WireError::ReplyTooLarge(REPLY_BYTE_BOUND))
        );
    }
    #[test]
    fn oversized_fetch_and_body_are_refused_at_bound_plus_one() {
        assert_eq!(
            encode_fetched_reply(&fetched(REPLY_BYTE_BOUND as usize + 1)),
            Err(WireError::ReplyTooLarge(REPLY_BYTE_BOUND))
        );
        assert!(decode_reply(&vec![0; REPLY_BYTE_BOUND as usize + 1]).is_err());
        let header = format!("Content-Length: {}\r\n\r\n", REPLY_BYTE_BOUND + 1);
        assert_eq!(
            maos_frame_codec::read_frame(&mut header.as_bytes())
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::InvalidData
        );
    }
    #[test]
    fn partial_header_is_protocol_error_not_clean_eof() {
        for bytes in [
            b"Content-Length: 12".as_slice(),
            b"Content-Length: 12\r\n",
            b"Content-Length: 12\r\n\r\nabc",
        ] {
            assert_eq!(
                maos_frame_codec::read_frame(&mut &*bytes)
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::UnexpectedEof
            );
        }
        assert_eq!(maos_frame_codec::read_frame(&mut &b""[..]).unwrap(), None);
    }
    #[test]
    fn empty_page_and_all_error_cases_round_trip() {
        assert_eq!(
            fit_page(vec![], None).unwrap(),
            RecallReply::Page {
                entries: vec![],
                next_cursor: None
            }
        );
        for error in [
            WireError::ScopeViolation,
            WireError::FrameNotFound,
            WireError::InvalidRequest("bad cursor".into()),
            WireError::LimitExceeded(1024),
            WireError::CapabilityDenied,
            WireError::ConsentWithheld,
            WireError::ReplyTooLarge(REPLY_BYTE_BOUND),
            WireError::Unavailable,
        ] {
            assert!(!error.code().is_empty());
            let reply = RecallReply::Error(error);
            assert_eq!(decode_reply(&encode_reply(&reply).unwrap()).unwrap(), reply);
        }
    }
}
