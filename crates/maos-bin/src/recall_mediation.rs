//! One governed recall path for native Spirits and the in-band WASM service.

use std::sync::Arc;

use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{LogFetchResponse, LogRecallError, LogRecallFilter, LogRecallPage};
use maos_domain::ports::capability::{CapError, CapabilityRegistryPort};
use maos_domain::ports::LogRecallPort;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;

use crate::enterprise_identity::EnterpriseRuntime;
use crate::enterprise_pdp_runtime::EnterprisePdpRuntime;
use crate::worker_spawn::issue_enterprise_governed_capability;

/// Trusted composition-root identity; a wire request never supplies a pid or sender.
pub struct MediatedLogRecall {
    adapter: Arc<LogRecallAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    security: Arc<SecurityManagerAdapter>,
    journal: Arc<TransparencyLogAdapter>,
    enterprise: Option<Arc<EnterpriseRuntime>>,
    pdp: Option<EnterprisePdpRuntime>,
    spirit_id: String,
}

/// Field order is part of the operator-only refusal receipt format.
#[derive(serde::Serialize)]
struct RefusalPayload<'a> {
    op: &'a str,
    code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    frame_id_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>,
    boot_nonce: u64,
}

impl MediatedLogRecall {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        adapter: Arc<LogRecallAdapter>,
        capability: Arc<CapabilityRegistryAdapter>,
        security: Arc<SecurityManagerAdapter>,
        journal: Arc<TransparencyLogAdapter>,
        enterprise: Option<Arc<EnterpriseRuntime>>,
        pdp: Option<EnterprisePdpRuntime>,
        spirit_id: String,
    ) -> Self {
        Self {
            adapter,
            capability,
            security,
            journal,
            enterprise,
            pdp,
            spirit_id,
        }
    }

    fn validate(filter: &LogRecallFilter) -> Result<(), LogRecallError> {
        if filter.limit > LogRecallFilter::MAX_LIMIT {
            return Err(LogRecallError::LimitExceeded {
                requested: filter.limit,
                max: LogRecallFilter::MAX_LIMIT,
            });
        }
        if filter
            .since_ns
            .into_iter()
            .chain(filter.until_ns)
            .chain(
                filter
                    .cursor
                    .as_ref()
                    .map(|cursor| cursor.last_timestamp_ns),
            )
            .any(|timestamp| timestamp > i64::MAX as u64)
        {
            return Err(LogRecallError::InvalidCursor(
                "timestamp exceeds i64::MAX".into(),
            ));
        }
        Ok(())
    }

    fn governed<T>(
        &self,
        pid: u32,
        scope: Scope,
        call: impl FnOnce(&TokenId) -> Result<T, LogRecallError>,
    ) -> Result<T, LogRecallError> {
        let policy = self.security.policy().inner().load_full();
        let posture = policy
            .spirit_postures
            .get(&pid)
            .ok_or(LogRecallError::CapabilityDenied)?;
        let token = issue_enterprise_governed_capability(
            &self.capability,
            self.enterprise.as_deref(),
            self.pdp.as_ref(),
            pid,
            scope,
            60,
            posture.posture_hash(),
            IntentClass::Standard,
        )
        .map_err(|_| LogRecallError::CapabilityDenied)?;
        let result = call(&token.token_id);
        let revoked = match self.capability.revoke(token.token_id) {
            Ok(()) | Err(CapError::Revoked | CapError::UnknownToken) => Ok(()),
            Err(error) => Err(LogRecallError::Storage(format!(
                "token rollback failed: {error}"
            ))),
        };
        // Preserve the original operation refusal, but never return data after a
        // failed rollback. Every minted token is revoked before the caller resumes.
        result.and_then(|value| revoked.map(|()| value))
    }

    fn recall_inner(
        &self,
        pid: u32,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        Self::validate(&filter)?;
        self.governed(pid, Scope::LogRecall, |token| {
            self.adapter.recall_attributed(pid, filter, token)
        })
    }

    fn receipt(
        &self,
        pid: u32,
        op: &str,
        code: &str,
        id: Option<[u8; 16]>,
        error: Option<&LogRecallError>,
    ) {
        let owner_pid = match error {
            Some(LogRecallError::ScopeViolation { owner_pid, .. }) => Some(*owner_pid),
            _ => None,
        };
        let detail = match error {
            Some(LogRecallError::Storage(detail) | LogRecallError::InvalidCursor(detail)) => {
                Some(detail.as_str())
            }
            _ => None,
        };
        let payload = serde_json::to_vec(&RefusalPayload {
            op,
            code,
            // Match the journal's frame-reference convention: a contiguous
            // 32-hex id is otherwise scrubbed as a capability-token secret.
            frame_id_hex: id.map(|id| researcher::encode_frame_id_hex(&id)),
            owner_pid,
            detail,
            boot_nonce: self.journal.boot_nonce(),
        })
        .expect("refusal receipt contains only serializable scalar fields");
        self.journal.insert_frame_event_with_sender(
            FrameKind::GovernanceEvent,
            pid,
            &self.spirit_id,
            "",
            None,
            &format!("log.{op}.refused:{code}"),
            &payload,
            FrameOrigin::Kernel,
        );
    }

    fn finish<T>(
        &self,
        pid: u32,
        op: &str,
        id: Option<[u8; 16]>,
        result: Result<T, LogRecallError>,
    ) -> Result<T, LogRecallError> {
        if let Err(error) = &result {
            self.receipt(pid, op, refusal_code(error), id, Some(error));
        }
        result
    }
}

fn refusal_code(error: &LogRecallError) -> &'static str {
    match error {
        LogRecallError::CapabilityDenied => "capability-denied",
        LogRecallError::ScopeViolation { .. } => "scope-violation",
        LogRecallError::ConsentWithheld { .. } => "consent-withheld",
        LogRecallError::FrameNotFound { .. } => "frame-not-found",
        LogRecallError::InvalidCursor(_) => "invalid-request",
        LogRecallError::LimitExceeded { .. } => "limit-exceeded",
        LogRecallError::Storage(_) | LogRecallError::ECrossWallRecallDenied { .. } => "unavailable",
    }
}

/// The Spirit-facing port has no cross-wall path: `recall_cross_wall` keeps the
/// trait's fail-closed default, and the composition root's operator traceback
/// remains the only production caller (Story 13.6d).
impl LogRecallPort for MediatedLogRecall {
    fn recall(&self, pid: u32, filter: LogRecallFilter) -> Result<LogRecallPage, LogRecallError> {
        self.finish(pid, "recall", None, self.recall_inner(pid, filter))
    }

    fn fetch(&self, pid: u32, id: [u8; 16]) -> Result<LogFetchResponse, LogRecallError> {
        let result = self.governed(pid, Scope::LogFetch, |token| {
            self.adapter.fetch_attributed(pid, id, token)
        });
        self.finish(pid, "fetch", Some(id), result)
    }
}

/// A tagged record that cannot be serviced without closing the session channel.
#[cfg(feature = "wasm-host")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolViolation(pub &'static str);

#[cfg(feature = "wasm-host")]
impl MediatedLogRecall {
    /// Journal channel closure exactly once at the session's dispatch boundary.
    pub fn channel_closed(&self, pid: u32, reason: &'static str) {
        #[derive(serde::Serialize)]
        struct ChannelClosed<'a> {
            op: &'static str,
            code: &'static str,
            reason: &'a str,
            boot_nonce: u64,
        }
        let payload = serde_json::to_vec(&ChannelClosed {
            op: "channel",
            code: "protocol-violation",
            reason,
            boot_nonce: self.journal.boot_nonce(),
        })
        .expect("channel receipt contains only serializable scalar fields");
        self.journal.insert_frame_event_with_sender(
            FrameKind::GovernanceEvent,
            pid,
            &self.spirit_id,
            "",
            None,
            "recall.channel.closed:protocol-violation",
            &payload,
            FrameOrigin::Kernel,
        );
    }

    /// Decode, validate, govern and encode one request. Operation refusals are
    /// journaled before their encoded reply can be delivered. Malformed records
    /// are returned to the session, which journals closure and ends the channel.
    pub fn serve_wire(&self, pid: u32, body: &[u8]) -> Result<Vec<u8>, ProtocolViolation> {
        use maos_iac::adapter::log_recall::BoundedFetch;
        use maos_wasm_host::recall_wire::{
            self as wire, RecallReply, RecallRequest, WireBytes, WireCursor, WireEntry, WireError,
        };
        let request = wire::decode_request(body)
            .map_err(|_| ProtocolViolation("unknown tag or undecodable recall request"))?;
        let (op, id, result) = match request {
            RecallRequest::Recall { filter } => {
                let result = wire_filter(filter).and_then(|filter| self.recall_inner(pid, filter));
                let result = result
                    .map_err(|error| (wire_error(&error), Some(error)))
                    .and_then(|page| {
                        let entries = page
                            .entries
                            .into_iter()
                            .map(|entry| WireEntry {
                                frame_id: WireBytes(entry.frame_id.to_vec()),
                                timestamp_ns: entry.timestamp_ns,
                                kind: kind_name(entry.kind),
                                intent: entry.intent,
                                payload_available: entry.payload_available,
                            })
                            .collect();
                        let cursor = page.next_cursor.map(|cursor| WireCursor {
                            last_timestamp_ns: cursor.last_timestamp_ns,
                            last_frame_id: WireBytes(cursor.last_frame_id.to_vec()),
                        });
                        wire::fit_page(entries, cursor).map_err(|error| (error, None))
                    })
                    .and_then(|reply| {
                        wire::encode_reply(&reply)
                            .map_err(|_| (WireError::ReplyTooLarge(wire::REPLY_BYTE_BOUND), None))
                    });
                ("recall", None, result)
            }
            RecallRequest::Fetch { frame_id } => {
                let id = <[u8; 16]>::try_from(frame_id.0.as_slice()).ok();
                let result = id
                    .ok_or_else(|| {
                        LogRecallError::InvalidCursor(
                            "frame id must contain exactly 16 bytes".into(),
                        )
                    })
                    .and_then(|id| {
                        self.governed(pid, Scope::LogFetch, |token| {
                            self.adapter.fetch_attributed_bounded(
                                pid,
                                id,
                                token,
                                wire::REPLY_BYTE_BOUND,
                            )
                        })
                    });
                let result = result
                    .map_err(|error| (wire_error(&error), Some(error)))
                    .and_then(|fetched| match fetched {
                        BoundedFetch::PayloadTooLarge { .. } => {
                            Err((WireError::ReplyTooLarge(wire::REPLY_BYTE_BOUND), None))
                        }
                        BoundedFetch::Fetched(frame) => {
                            wire::encode_fetched_reply(&RecallReply::Fetched {
                                frame_id: WireBytes(frame.frame_id.to_vec()),
                                timestamp_ns: frame.timestamp_ns,
                                kind: kind_name(frame.kind),
                                intent: frame.intent,
                                payload: WireBytes(frame.payload_redacted),
                                origin: maos_wasm_host::frame_bridge::origin_case_name(
                                    frame.origin,
                                )
                                .into(),
                            })
                            .map_err(|error| (error, None))
                        }
                    });
                ("fetch", id, result)
            }
        };
        match result {
            Ok(body) => Ok(body),
            Err((error, domain)) => {
                self.receipt(pid, op, error.code(), id, domain.as_ref());
                wire::encode_reply(&RecallReply::Error(error))
                    .map_err(|_| ProtocolViolation("recall refusal reply could not be encoded"))
            }
        }
    }
}

#[cfg(feature = "wasm-host")]
fn wire_filter(
    filter: maos_wasm_host::recall_wire::WireFilter,
) -> Result<LogRecallFilter, LogRecallError> {
    use maos_domain::log_recall::{FrameKindLabel, LogRecallCursor};
    // Check limit first, matching the native path even if another field is bad.
    if filter.limit as usize > LogRecallFilter::MAX_LIMIT {
        return Err(LogRecallError::LimitExceeded {
            requested: filter.limit as usize,
            max: LogRecallFilter::MAX_LIMIT,
        });
    }
    let kind = filter
        .kind
        .map(|label| {
            serde::Deserialize::deserialize(serde::de::value::StrDeserializer::<
                serde::de::value::Error,
            >::new(&label))
            .map_err(|_| LogRecallError::InvalidCursor("unknown frame kind label".into()))
        })
        .transpose()?;
    let kind: Option<FrameKindLabel> = kind;
    let cursor = filter
        .cursor
        .map(|cursor| {
            Ok::<_, LogRecallError>(LogRecallCursor {
                last_timestamp_ns: cursor.last_timestamp_ns,
                last_frame_id: cursor.last_frame_id.0.as_slice().try_into().map_err(|_| {
                    LogRecallError::InvalidCursor("cursor id must contain exactly 16 bytes".into())
                })?,
            })
        })
        .transpose()?;
    Ok(LogRecallFilter {
        kind,
        since_ns: filter.since_ns,
        until_ns: filter.until_ns,
        limit: filter.limit as usize,
        cursor,
        intent_filter: filter.intent_filter,
        order_by_insertion: false,
    })
}

#[cfg(feature = "wasm-host")]
fn kind_name(kind: maos_domain::log_recall::FrameKindLabel) -> String {
    match serde_json::to_value(kind) {
        Ok(serde_json::Value::String(name)) => name,
        _ => unreachable!("frame kind labels serialize as strings"),
    }
}

#[cfg(feature = "wasm-host")]
fn wire_error(error: &LogRecallError) -> maos_wasm_host::recall_wire::WireError {
    use maos_wasm_host::recall_wire::WireError;
    match refusal_code(error) {
        "capability-denied" => WireError::CapabilityDenied,
        "scope-violation" => WireError::ScopeViolation,
        "consent-withheld" => WireError::ConsentWithheld,
        "frame-not-found" => WireError::FrameNotFound,
        "invalid-request" => WireError::InvalidRequest(match error {
            LogRecallError::InvalidCursor(detail) => detail.clone(),
            _ => unreachable!("invalid-request is an InvalidCursor"),
        }),
        "limit-exceeded" => WireError::LimitExceeded(LogRecallFilter::MAX_LIMIT as u32),
        _ => WireError::Unavailable,
    }
}
