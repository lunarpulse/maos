//! Private in-band lifecycle records (Story 20-9): one tagged CBOR item per ADR-032 body,
//! beside `recall_wire`. A request never carries a pid, class, token or author; the daemon
//! binds them. `Probe` asks whether the guest exports `maos:spirit/lifecycle`.
use ciborium::tag::Required;
use serde::{Deserialize, Serialize};

use crate::recall_wire::WireBytes;

/// Private, unregistered tags in RFC 8949's first-come-first-served range.
pub const LIFECYCLE_REQUEST_TAG: u64 = 60002;
pub const LIFECYCLE_REPLY_TAG: u64 = 60003;
/// Longest guest-supplied refusal text kept in a reply.
pub const MAX_DETAIL_CHARS: usize = 256;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LifecycleRequest {
    Probe,
    Idle,
    Consolidate {
        batch: WireBytes,
    },
    Telemetry {
        seq: u64,
        spirit_id: String,
        tag: String,
        value: f64,
        timestamp_ms: u64,
    },
    Schedule {
        payload: WireBytes,
    },
    Pause,
    Resume,
    Snapshot,
    SwapIn {
        state: WireBytes,
    },
    Migrate {
        state: WireBytes,
    },
}

impl LifecycleRequest {
    /// The imports (`recall`, `context`, `skills`) are callable inside exactly these exports;
    /// a swap step is import-free so its result never depends on the log (D-20-9-05).
    pub fn imports_allowed(&self) -> bool {
        matches!(
            self,
            Self::Idle | Self::Consolidate { .. } | Self::Telemetry { .. } | Self::Schedule { .. }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LifecycleReply {
    Supported(bool),
    Done,
    State(WireBytes),
    Declined(String),
    Failed(String),
    Unsupported,
}

/// Dispatch on the CBOR tag header (`0xD9` + the two tag bytes) without decoding the body.
fn has_tag(body: &[u8], tag: u64) -> bool {
    let [0xD9, high, low, ..] = body else {
        return false;
    };
    u64::from(u16::from_be_bytes([*high, *low])) == tag
}
pub fn is_request(body: &[u8]) -> bool {
    has_tag(body, LIFECYCLE_REQUEST_TAG)
}
pub fn is_reply(body: &[u8]) -> bool {
    has_tag(body, LIFECYCLE_REPLY_TAG)
}
pub fn encode_request(request: &LifecycleRequest) -> Result<Vec<u8>, String> {
    crate::codec::encode_cbor(&Required::<_, LIFECYCLE_REQUEST_TAG>(request))
}
pub fn decode_request(body: &[u8]) -> Result<LifecycleRequest, String> {
    crate::codec::decode_cbor::<Required<LifecycleRequest, LIFECYCLE_REQUEST_TAG>>(body)
        .map(|value| value.0)
}
pub fn encode_reply(reply: &LifecycleReply) -> Result<Vec<u8>, String> {
    crate::codec::encode_cbor(&Required::<_, LIFECYCLE_REPLY_TAG>(reply))
}
pub fn decode_reply(body: &[u8]) -> Result<LifecycleReply, String> {
    crate::codec::decode_cbor::<Required<LifecycleReply, LIFECYCLE_REPLY_TAG>>(body)
        .map(|value| value.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recall_wire as recall;

    #[test]
    fn every_request_and_reply_round_trips_under_its_own_tag() {
        let requests = [
            LifecycleRequest::Probe,
            LifecycleRequest::Idle,
            LifecycleRequest::Consolidate {
                batch: WireBytes(vec![1, 2, 3]),
            },
            LifecycleRequest::Telemetry {
                seq: 9,
                spirit_id: "probe".into(),
                tag: "queue.depth".into(),
                value: 0.5,
                timestamp_ms: 1_700_000_000_000,
            },
            LifecycleRequest::Schedule {
                payload: WireBytes(b"cron".to_vec()),
            },
            LifecycleRequest::Pause,
            LifecycleRequest::Resume,
            LifecycleRequest::Snapshot,
            LifecycleRequest::SwapIn {
                state: WireBytes(vec![0; 300]),
            },
            LifecycleRequest::Migrate {
                state: WireBytes(Vec::new()),
            },
        ];
        for request in requests {
            let body = encode_request(&request).unwrap();
            assert!(is_request(&body) && !is_reply(&body));
            assert_eq!(decode_request(&body).unwrap(), request);
            assert!(decode_reply(&body).is_err());
        }
        let replies = [
            LifecycleReply::Supported(true),
            LifecycleReply::Done,
            LifecycleReply::State(WireBytes(vec![7; 70_000])),
            LifecycleReply::Declined("no".into()),
            LifecycleReply::Failed("boom".into()),
            LifecycleReply::Unsupported,
        ];
        for reply in replies {
            let body = encode_reply(&reply).unwrap();
            assert!(is_reply(&body) && !is_request(&body));
            assert_eq!(decode_reply(&body).unwrap(), reply);
            assert!(decode_request(&body).is_err());
        }
    }

    #[test]
    fn lifecycle_tags_are_disjoint_from_recall_tags() {
        let recall_request =
            recall::encode_request(&recall::RecallRequest::Context { used_tokens: 1 }).unwrap();
        assert!(recall::is_tagged(&recall_request));
        assert!(!is_request(&recall_request) && !is_reply(&recall_request));
        assert!(recall::decode_request(&encode_request(&LifecycleRequest::Idle).unwrap()).is_err());
        assert!(!is_request(&[]) && !is_reply(&[0xD9, 0xEA]));
    }

    #[test]
    fn only_the_acting_exports_may_import() {
        assert!(LifecycleRequest::Idle.imports_allowed());
        assert!(LifecycleRequest::Telemetry {
            seq: 0,
            spirit_id: String::new(),
            tag: String::new(),
            value: 0.0,
            timestamp_ms: 0
        }
        .imports_allowed());
        for request in [
            LifecycleRequest::Probe,
            LifecycleRequest::Pause,
            LifecycleRequest::Resume,
            LifecycleRequest::Snapshot,
            LifecycleRequest::SwapIn {
                state: WireBytes(Vec::new()),
            },
            LifecycleRequest::Migrate {
                state: WireBytes(Vec::new()),
            },
        ] {
            assert!(!request.imports_allowed(), "{request:?}");
        }
    }
}
