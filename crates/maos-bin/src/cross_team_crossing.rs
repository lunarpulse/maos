//! Story 13.6b — the production cross-team crossing: an **emitter** on team A's
//! host and an **applier** on team B's host, joined by the cohort A2A mesh.
//!
//! # Why this is necessarily two processes (D-5)
//!
//! The composition root constructs **exactly one** `LoomLiteStore`, pinned to
//! `MAOS_LOOM_HOME_TEAM`, and `connection_assignment_guard` proves
//! `datname_for(home_team) == current_database()` at boot. A second store in the
//! same process would defeat ADR-055's database-per-team wall in the very act of
//! demonstrating it. So a crossing is a **pair**: `maos_team_a`'s daemon
//! originates and emits; `maos_team_b`'s daemon applies. Neither ever holds two
//! stores.
//!
//! # Why the emitter lives inside the daemon runtime (D-14)
//!
//! `maos-a2a-tcp`'s `prepare_outbound` is the only non-test outbound A2A send in
//! the workspace, and the transport that owns it is built only inside
//! `build_cohort_a2a_daemon_runtime`. A sibling `MAOS_ONE_SHOT` arm returns from
//! the dispatch thousands of lines before any transport, peer pin, or manifest
//! gate exists, so it cannot send an authenticated cohort frame at all. The
//! emitter is therefore a boot-time arm of `run_cohort_a2a_daemon`, gated on the
//! operator's own `MAOS_CROSS_TEAM_SHARE_PEER` declaration.
//!
//! # Why the applier decides from the ENVELOPE's team, not the payload's (D-13)
//!
//! `apply_replication_bundle` reads `bundle.source_team` — a self-declared
//! payload field whose signature **any** seed-holding emitter can forge, because
//! `derive_team_signing_seed` works for every `(region, team)`. Story 13.6a
//! authenticates a different field: `request.cohort_source_team`, stamped from
//! the host's own operator-signed `CohortMember.team`. Nothing bound them.
//! [`CrossTeamCrossingAdapter::apply_crossing`] is that weld: it refuses, under
//! its own cause, **before** `apply_replication_bundle` is called, when the two
//! disagree. Without it a truthful envelope plus a lying payload lands a row
//! under an impersonated team with every shipped check green.

use sha2::{Digest, Sha256};
use std::sync::Arc;

use maos_a2a_core::{
    CrossTeamCrossingPort, CrossingOutcome, CrossingRefusal, COHORT_INTENT_COLLECTIVE_SHARE,
    CROSSING_EVENT_TYPE,
};
use maos_iac::FrameFilter;
use maos_domain::frame::{FramePayload, IacFrame, TelemetryEventPayload};
use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::team::TeamId;
use maos_loom_lite::cross_team_consent::{CrossTeamConsentError, CrossTeamConsentPort};
use maos_loom_lite::replication::bundle::{
    apply_replication_bundle, BundleError, CrossRegionReplicationBundle, CrossTeamApplyContext,
};
use maos_loom_lite::store::{LoomLiteStore, StoreError};
use crate::cross_team_consent::CROSS_TEAM_COLLECTIVE_ERASE_INTENT;

/// The wire body of a cross-team crossing frame.
///
/// Rides `FramePayload::TelemetryEvent` exactly like the shipped cohort control
/// and digest frames (`maos.cohort-manifest.v1`), so **no `maos-domain` frame
/// variant and no `FrameKind` is added** — `FrameKind` is a `repr` ABI enum and
/// `abi-diff` is a null control that cannot see an addition, so riding the
/// existing intent is the only honest option.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CrossTeamCrossingControl {
    Share {
        /// The destination team the emitter is asking for. The applier binds
        /// this value to its own `store.config().home_team` through
        /// `CrossTeamApplyContext` before consent or persistence.
        to_team: String,
        /// Unique share operation identity, pinned into every crossed row.
        op_id: String,
        /// The manifest host that emitted this operation.
        emitter_host: String,
        bundle: CrossRegionReplicationBundle,
    },
    /// Reconcile an operator erase from a crossed destination to the team that
    /// owns the origin row. `to_team` is validated against the receiving store;
    /// the authenticated envelope team—not a body claim—authorizes the action.
    Erase {
        to_team: String,
        spirit_pid: u32,
        namespace: String,
        key: String,
        op_id: String,
        locator_digest: String,
    },
}

impl CrossTeamCrossingControl {
    /// Decode a crossing body out of an intake frame, or `None` when the frame
    /// is not one. Mirrors `CohortManifestControl::from_frame`: the intent, the
    /// `FrameKind`, and the `event_type` must all agree before the JSON is read.
    pub fn from_frame(frame: &IacFrame) -> Option<Result<Self, String>> {
        let intent = frame
            .consent_envelope
            .as_ref()
            .and_then(|envelope| envelope.intent_class.as_ref())
            .map(|intent| intent.as_str())?;
        let expected_erase = if intent.eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE) {
            false
        } else if intent.eq_ignore_ascii_case(CROSS_TEAM_COLLECTIVE_ERASE_INTENT) {
            true
        } else {
            return None;
        };
        let expected_class = if expected_erase {
            maos_domain::invariants::i1::IntentClass::Standard
        } else {
            maos_domain::invariants::i1::IntentClass::Readonly
        };
        if frame.intent != expected_class {
            return Some(Err(
                "crossing control coarse intent class does not match its fine-grained intent"
                    .into(),
            ));
        }
        if frame.kind != maos_spirit_abi::identity::FrameKind::TelemetryEvent {
            return Some(Err("crossing frame kind is not a telemetry event".into()));
        }
        let FramePayload::TelemetryEvent(payload) = &frame.payload else {
            return Some(Err("crossing frame payload is not a telemetry event".into()));
        };
        if payload.event_type != CROSSING_EVENT_TYPE {
            return Some(Err(format!(
                "crossing frame carries unexpected event_type {}",
                payload.event_type
            )));
        }
        Some(
            serde_json::from_str::<Self>(&payload.data)
                .map_err(|error| format!("crossing frame body is undecodable: {error}"))
                .and_then(|control| {
                    if matches!((&control, expected_erase),
                        (Self::Erase { .. }, true) | (Self::Share { .. }, false))
                    {
                        Ok(control)
                    } else {
                        Err("crossing control kind does not match its fine-grained intent".into())
                    }
                }),
        )
    }

    /// Encode this body into the telemetry payload the frame carries.
    pub fn telemetry_payload(&self) -> Result<TelemetryEventPayload, String> {
        Ok(TelemetryEventPayload {
            event_type: CROSSING_EVENT_TYPE.into(),
            data: serde_json::to_string(self)
                .map_err(|error| format!("crossing frame body is unencodable: {error}"))?,
        })
    }
}


/// SHA-256 commitment to an erase locator. Length-prefixing makes the
/// canonical grammar unambiguous; the raw key is never used for audit matching.
pub fn erase_locator_digest(
    from_team: &str,
    to_team: &str,
    spirit_pid: u32,
    namespace: &str,
    key: &str,
) -> String {
    fn append(hasher: &mut Sha256, value: &[u8]) {
        hasher.update(u32::try_from(value.len()).unwrap_or(u32::MAX).to_be_bytes());
        hasher.update(value);
    }
    let mut hasher = Sha256::new();
    hasher.update(b"maos-cross-team-erase-locator-v1");
    append(&mut hasher, from_team.as_bytes());
    append(&mut hasher, to_team.as_bytes());
    append(&mut hasher, &spirit_pid.to_be_bytes());
    append(&mut hasher, namespace.as_bytes());
    append(&mut hasher, key.as_bytes());
    let digest = hex::encode(hasher.finalize());
    digest
        .as_bytes()
        .chunks(16)
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .expect("hex digest is UTF-8")
        .join("-")
}
/// The applier: the composition root's store-owning implementation of the A2A
/// crossing seam, and the **first production caller** of
/// `apply_replication_bundle` → `CrossTeamConsentAdapter::is_granted`.
pub struct CrossTeamCrossingAdapter {
    store: Arc<LoomLiteStore>,
    home_team: TeamId,
    base_seed: [u8; 32],
    erase_reconciliation: Option<EraseReconciliation>,
}

struct EraseReconciliation {
    cross_team_consent: Arc<dyn CrossTeamConsentPort>,
    tenant_map: Arc<crate::tenant_map::TenantMapAdapter>,
    local_control_spirit: maos_domain::ports::registry::SpiritId,
    transparency_log: Arc<maos_iac::TransparencyLogAdapter>,
}

impl CrossTeamCrossingAdapter {
    pub fn new(store: Arc<LoomLiteStore>, home_team: TeamId, base_seed: [u8; 32]) -> Self {
        Self {
            store,
            home_team,
            base_seed,
            erase_reconciliation: None,
        }
    }

    pub fn with_erase_reconciliation(
        mut self,
        cross_team_consent: Arc<dyn CrossTeamConsentPort>,
        tenant_map: Arc<crate::tenant_map::TenantMapAdapter>,
        local_control_spirit: maos_domain::ports::registry::SpiritId,
        transparency_log: Arc<maos_iac::TransparencyLogAdapter>,
    ) -> Self {
        self.erase_reconciliation = Some(EraseReconciliation {
            cross_team_consent,
            tenant_map,
            local_control_spirit,
            transparency_log,
        });
        self
    }

    /// Story 13.6b / AC2 — keep the five-cause matrix alive **inside the applier
    /// process**, which is the only boundary it has ever crossed (D-15 measured
    /// zero `TransportCause` occurrences in either A2A crate). The typed
    /// `StoreError` is mapped through the shipped
    /// `maos_loom_lite::adapter::store_error_to_port_error`, so a consent denial,
    /// a stale tenant map, and an invalid attestation stay distinguishable
    /// locally, and only THEN is the refusal projected onto the new wire cause.
    fn local_cause(error: &BundleError) -> Option<maos_domain::ports::CollectivePortError> {
        let store_error = match error {
            BundleError::ConsentDenied {
                from_team,
                to_team,
                intent,
            } => StoreError::ConsentDenied {
                from_team: from_team.clone(),
                to_team: to_team.clone(),
                intent: intent.clone(),
            },
            BundleError::ConsentStateStale { reason } => StoreError::TenantMapStale {
                team_id: None,
                reason: reason.clone(),
            },
            _ => return None,
        };
        Some(maos_loom_lite::adapter::store_error_to_port_error(
            store_error,
        ))
    }

    fn refusal_for(error: BundleError, from_team: &str, to_team: &str) -> CrossingRefusal {
        match error {
            BundleError::ConsentDenied {
                from_team,
                to_team,
                intent,
            } => CrossingRefusal::ConsentDenied {
                from_team: from_team.as_str().to_string(),
                to_team: to_team.as_str().to_string(),
                intent,
            },
            BundleError::ConsentStateStale { reason } => CrossingRefusal::ConsentStale {
                reason,
                from_team: from_team.to_string(),
                to_team: to_team.to_string(),
                intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
            },
            BundleError::ConsentStateUnavailable { reason } => CrossingRefusal::StateUnavailable {
                reason,
                from_team: from_team.to_string(),
                to_team: to_team.to_string(),
                intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
            },
            other => CrossingRefusal::ApplyFailed {
                reason: other.to_string(),
                from_team: from_team.to_string(),
                to_team: to_team.to_string(),
                intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
            },
        }
    }

    fn share_provenance(
        transparency_log: &maos_iac::TransparencyLogAdapter,
        spirit_pid: u32,
        source_team: &str,
        destination_team: &str,
        op_id: &str,
        locator_digest: &str,
    ) -> Result<Option<(i64, String)>, String> {
        let entries = transparency_log
            .query_frames(FrameFilter {
                spirit_pid: Some(spirit_pid),
                ..FrameFilter::default()
            })
            .map_err(|error| format!("cross-team share provenance query failed: {error}"))?;
        Ok(entries.into_iter().find_map(|entry| {
            if entry.intent != "collective.host.cross-team-share" {
                return None;
            }
            let payload = serde_json::from_slice::<serde_json::Value>(&entry.payload_redacted)
                .ok()?;
            (payload.get("from_team").and_then(serde_json::Value::as_str) == Some(source_team)
                && payload.get("to_team").and_then(serde_json::Value::as_str)
                    == Some(destination_team)
                && payload
                    .get("op_id")
                    .and_then(serde_json::Value::as_str)
                    .map(|recorded| recorded.replace('-', ""))
                    .as_deref()
                    == Some(op_id)
                && payload
                    .get("locator_digest")
                    .and_then(serde_json::Value::as_str)
                    == Some(locator_digest)
                && payload.get("status").and_then(serde_json::Value::as_str)
                    == Some("crossing_applied"))
            .then(|| {
                Some((
                    payload.get("source_ts")?.as_i64()?,
                    payload
                        .get("source_region")?
                        .as_str()?
                        .to_string(),
                ))
            })?
        }))
    }
}

#[async_trait::async_trait]
impl CrossTeamCrossingPort for CrossTeamCrossingAdapter {
    async fn apply_crossing(&self, authenticated_team: &str, frame: &IacFrame) -> CrossingOutcome {
        let Some(decoded) = CrossTeamCrossingControl::from_frame(frame) else {
            return CrossingOutcome::NotCrossing;
        };
        match decoded {
            Ok(CrossTeamCrossingControl::Share {
                to_team,
                op_id,
                emitter_host,
                bundle,
            }) => {
                // ── AC3 / D-13: THE WELD ────────────────────────────────────
                let payload_team = bundle
                    .source_team
                    .as_ref()
                    .map(|team| team.as_str().to_string())
                    .unwrap_or_default();
                if payload_team != authenticated_team {
                    return CrossingOutcome::Refused(CrossingRefusal::SourceTeamUnbound {
                        envelope_team: authenticated_team.to_string(),
                        payload_team,
                    });
                }

                let requested_team = match TeamId::new(&to_team) {
                    Ok(team) => team,
                    Err(error) => {
                        return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                            reason: format!("crossing destination team is not canonical: {error}"),
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
                        })
                    }
                };

                if requested_team != self.home_team {
                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: format!(
                            "destination team mismatch: store={}, requested={}",
                            self.home_team.as_str(),
                            requested_team.as_str()
                        ),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
                    });
                }

                if op_id.len() != 32
                    || !op_id.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || emitter_host.is_empty()
                {
                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: "crossing share has an invalid operation binding".to_string(),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
                    });
                }
                let dest_region = self.store.config().home_region.clone();
                let context =
                    CrossTeamApplyContext::new(&requested_team, COHORT_INTENT_COLLECTIVE_SHARE);
                match apply_replication_bundle(
                    &bundle,
                    &self.store,
                    &dest_region,
                    Some(context),
                    &self.base_seed,
                )
                .await
                {
                    Ok(result) => {
                        for leaf in &bundle.leaves {
                            let namespace = match maos_loom_lite::schema::parts_to_namespace(
                                &leaf.namespace_kind,
                                &leaf.namespace_detail,
                            ) {
                                Ok(namespace) => namespace,
                                Err(error) => {
                                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                                        reason: format!(
                                            "crossing share namespace cannot be operation-bound: {error}"
                                        ),
                                        from_team: authenticated_team.to_string(),
                                        to_team,
                                        intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
                                    })
                                }
                            };
                            let origin = maos_loom_lite::store::CrossedRowOrigin {
                                source_team: TeamId::new(authenticated_team)
                                    .expect("authenticated team was canonicalized by the router"),
                                emitter_host: emitter_host.clone(),
                                op_id: op_id.clone(),
                                source_ts: leaf.source_ts,
                                source_region: leaf.source_region.clone(),
                            };
                            if let Err(error) = self
                                .store
                                .annotate_crossed_row(
                                    u32::try_from(leaf.spirit_pid).unwrap_or(u32::MAX),
                                    &namespace,
                                    &leaf.key,
                                    &origin,
                                )
                                .await
                            {
                                return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                                    reason: format!(
                                        "crossing share operation binding failed: {error}"
                                    ),
                                    from_team: authenticated_team.to_string(),
                                    to_team,
                                    intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
                                });
                            }
                        }
                        CrossingOutcome::Applied {
                            applied_count: result.applied_count,
                        }
                    }
                    Err(error) => {
                        if let Some(cause) = Self::local_cause(&error) {
                            eprintln!("maos: cross-team crossing refused at the applier: {cause:?}");
                        }
                        CrossingOutcome::Refused(Self::refusal_for(
                            error,
                            authenticated_team,
                            requested_team.as_str(),
                        ))
                    }
                }
            }
            Ok(CrossTeamCrossingControl::Erase {
                to_team,
                spirit_pid,
                namespace,
                key,
                op_id,
                locator_digest,
            }) => {
                let requested_team = match TeamId::new(&to_team) {
                    Ok(team) => team,
                    Err(error) => {
                        return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                            reason: format!("erase destination team is not canonical: {error}"),
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: "collective:erase".to_string(),
                        })
                    }
                };
                if requested_team != self.home_team {
                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: format!(
                            "erase destination team mismatch: store={}, requested={}",
                            self.home_team.as_str(),
                            requested_team.as_str()
                        ),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: "collective:erase".to_string(),
                    });
                }
                let Some(reconciliation) = self.erase_reconciliation.as_ref() else {
                    return CrossingOutcome::Refused(CrossingRefusal::StateUnavailable {
                        reason: "collective erase reconciliation is not configured".to_string(),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: "collective:erase".to_string(),
                    });
                };
                let from_team = match TeamId::new(authenticated_team) {
                    Ok(team) => team,
                    Err(error) => {
                        return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                            reason: format!("authenticated erase source team is not canonical: {error}"),
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: "collective:erase".to_string(),
                        })
                    }
                };
                match reconciliation.cross_team_consent.is_granted(
                    &from_team,
                    &requested_team,
                    "collective:erase",
                ) {
                    Ok(true) => {}
                    Ok(false) => {
                        return CrossingOutcome::Refused(CrossingRefusal::ConsentDenied {
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: "collective:erase".to_string(),
                        })
                    }
                    Err(CrossTeamConsentError::Stale { reason }) => {
                        return CrossingOutcome::Refused(CrossingRefusal::ConsentStale {
                            reason,
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: "collective:erase".to_string(),
                        })
                    }
                    Err(CrossTeamConsentError::StateUnavailable { reason }) => {
                        return CrossingOutcome::Refused(CrossingRefusal::StateUnavailable {
                            reason,
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: "collective:erase".to_string(),
                        })
                    }
                }
                let namespace_name = namespace;
                let namespace = match namespace_name.as_str() {
                    "default" => MemoryNamespace::Default,
                    "coordination" => MemoryNamespace::Coordination,
                    "forgotten" => MemoryNamespace::Forgotten,
                    _ => {
                        return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                            reason: format!(
                                "unsupported collective erase namespace {namespace_name:?}"
                            ),
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                        })
                    }
                };
                let expected_digest = erase_locator_digest(
                    requested_team.as_str(),
                    authenticated_team,
                    spirit_pid,
                    &namespace_name,
                    &key,
                );
                if locator_digest != expected_digest
                    || op_id.len() != 32
                    || !op_id.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: "collective erase locator binding is invalid".to_string(),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                    });
                }
                let provenance = match Self::share_provenance(
                    reconciliation.transparency_log.as_ref(),
                    spirit_pid,
                    requested_team.as_str(),
                    authenticated_team,
                    &op_id,
                    &locator_digest,
                ) {
                    Ok(found) => found,
                    Err(reason) => {
                        return CrossingOutcome::Refused(CrossingRefusal::StateUnavailable {
                            reason,
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                        })
                    }
                };
                let Some((shared_source_ts, shared_source_region)) = provenance else {
                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: "collective erase provenance not found".to_string(),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                    });
                };
                // Provenance is verified before the guarded erase's tenant
                // binding is established. A locator may fill an absent binding,
                // but cannot overwrite a different receiver-owned binding.
                if let Err(error) = reconciliation.tenant_map.bind_spirit_if_unbound_or_same(
                    spirit_pid,
                    reconciliation.local_control_spirit.clone(),
                ) {
                    return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: format!("collective erase receiver binding refused: {error}"),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                    });
                }
                match self
                    .store
                    .native_row_generation(spirit_pid, &namespace, &key)
                    .await
                {
                    Ok(Some((source_ts, source_region)))
                        if source_ts != shared_source_ts || source_region != shared_source_region =>
                    {
                        CrossingOutcome::Refused(CrossingRefusal::StaleGeneration {
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                        })
                    }
                    Ok(Some(_)) => match self.store.erase(spirit_pid, &namespace, &key).await {
                        Ok(receipt) => CrossingOutcome::Applied {
                            applied_count: receipt.deleted_rows as usize,
                        },
                        Err(error) => CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                            reason: error.to_string(),
                            from_team: authenticated_team.to_string(),
                            to_team,
                            intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                        }),
                    },
                    // The source row is already tombstoned. This exact operation
                    // is therefore an idempotent replay, not a new deletion.
                    Ok(None) => CrossingOutcome::Applied { applied_count: 0 },
                    Err(error) => CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                        reason: error.to_string(),
                        from_team: authenticated_team.to_string(),
                        to_team,
                        intent: CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
                    }),
                }
            }
            Err(reason) => CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                reason,
                from_team: authenticated_team.to_string(),
                to_team: String::new(),
                intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
            }),
        }
    }
}

/// The operator's boot-time crossing declaration, read by the emitter arm of
/// `run_cohort_a2a_daemon`. Absence of `MAOS_CROSS_TEAM_SHARE_PEER` means the
/// daemon serves exactly as it did before this story — no emit, no journal row.
#[derive(Debug, Clone)]
pub struct CrossTeamShareRequest {
    pub peer: String,
    pub to_team: TeamId,
    pub spirit_pid: u32,
    pub namespace: MemoryNamespace,
    pub key: String,
    pub value: MemoryValue,
}

impl CrossTeamShareRequest {
    /// Read the declaration from the environment. `Ok(None)` = not requested.
    pub fn from_env() -> Result<Option<Self>, String> {
        let peer = match std::env::var("MAOS_CROSS_TEAM_SHARE_PEER") {
            Ok(value) if !value.trim().is_empty() => value.trim().to_string(),
            Ok(_) | Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err("MAOS_CROSS_TEAM_SHARE_PEER is not valid UTF-8".to_string())
            }
        };
        let to_team = std::env::var("MAOS_CROSS_TEAM_SHARE_TO_TEAM")
            .map_err(|_| "MAOS_CROSS_TEAM_SHARE_TO_TEAM is required to emit a crossing")?;
        let to_team = TeamId::new(to_team.trim())
            .map_err(|error| format!("MAOS_CROSS_TEAM_SHARE_TO_TEAM is not canonical: {error}"))?;
        let spirit_pid = std::env::var("MAOS_CROSS_TEAM_SHARE_PID")
            .map_err(|_| "MAOS_CROSS_TEAM_SHARE_PID is required to emit a crossing")?
            .trim()
            .parse::<u32>()
            .map_err(|_| "MAOS_CROSS_TEAM_SHARE_PID must be a u32".to_string())?;
        let namespace = match std::env::var("MAOS_CROSS_TEAM_SHARE_NAMESPACE") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => "default".to_string(),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err("MAOS_CROSS_TEAM_SHARE_NAMESPACE is not valid UTF-8".to_string())
            }
        };
        let namespace = match namespace.trim() {
            "default" => MemoryNamespace::Default,
            "coordination" => MemoryNamespace::Coordination,
            "forgotten" => MemoryNamespace::Forgotten,
            // `apply_replication_bundle` refuses the principal namespace at the
            // destination; refuse it at the source too so the operator learns
            // why here instead of from a remote NACK.
            "principal" => {
                return Err("principal namespace is partitioned out of collective storage".into())
            }
            other => {
                return Err(format!(
                    "unsupported MAOS_CROSS_TEAM_SHARE_NAMESPACE '{other}'"
                ))
            }
        };
        let key = std::env::var("MAOS_CROSS_TEAM_SHARE_KEY")
            .map_err(|_| "MAOS_CROSS_TEAM_SHARE_KEY is required to emit a crossing")?;
        if key.is_empty() {
            return Err("MAOS_CROSS_TEAM_SHARE_KEY must not be empty".to_string());
        }
        let value = std::env::var("MAOS_CROSS_TEAM_SHARE_VALUE")
            .map_err(|_| "MAOS_CROSS_TEAM_SHARE_VALUE is required to emit a crossing")?;
        Ok(Some(Self {
            peer,
            to_team,
            spirit_pid,
            namespace,
            key,
            value: MemoryValue::Text(value),
        }))
    }
}

/// Build the cohort A2A frame that carries a crossing to `peer`.
pub fn crossing_frame(
    from: &maos_domain::frame::FrameAddress,
    peer: &maos_spirit_abi::identity::HostId,
    frame_seq: u64,
    to_team: &TeamId,
    bundle: CrossRegionReplicationBundle,
) -> Result<IacFrame, String> {
    crossing_frame_with_binding(
        from,
        peer,
        frame_seq,
        to_team,
        "00000000000000000000000000000000".to_string(),
        from.host_id
            .as_ref()
            .map(|host| host.as_str().to_string())
            .unwrap_or_else(|| "test-host".to_string()),
        bundle,
    )
}

/// Build a crossing frame with the operation binding emitted in production.
pub fn crossing_frame_with_binding(
    from: &maos_domain::frame::FrameAddress,
    peer: &maos_spirit_abi::identity::HostId,
    frame_seq: u64,
    to_team: &TeamId,
    op_id: String,
    emitter_host: String,
    bundle: CrossRegionReplicationBundle,
) -> Result<IacFrame, String> {
    control_frame(
        from,
        peer,
        frame_seq,
        COHORT_INTENT_COLLECTIVE_SHARE,
        maos_domain::invariants::i1::IntentClass::Readonly,
        CrossTeamCrossingControl::Share {
            to_team: to_team.as_str().to_string(),
            op_id,
            emitter_host,
            bundle,
        },
    )
}

/// Build an authenticated erase-reconciliation control for an origin team.
pub fn erase_frame_with_binding(
    from: &maos_domain::frame::FrameAddress,
    peer: &maos_spirit_abi::identity::HostId,
    frame_seq: u64,
    to_team: &TeamId,
    spirit_pid: u32,
    namespace: &MemoryNamespace,
    key: String,
    op_id: String,
    locator_digest: String,
) -> Result<IacFrame, String> {
    control_frame(
        from,
        peer,
        frame_seq,
        CROSS_TEAM_COLLECTIVE_ERASE_INTENT,
        maos_domain::invariants::i1::IntentClass::Standard,
        CrossTeamCrossingControl::Erase {
            to_team: to_team.as_str().to_string(),
            spirit_pid,
            namespace: match namespace {
                MemoryNamespace::Default => "default",
                MemoryNamespace::Coordination => "coordination",
                MemoryNamespace::Forgotten => "forgotten",
                MemoryNamespace::Principal { .. } => {
                    return Err("principal namespace is not collective-erasable".to_string())
                }
            }
            .to_string(),
            key,
            op_id,
            locator_digest,
        },
    )
}

fn control_frame(
    from: &maos_domain::frame::FrameAddress,
    peer: &maos_spirit_abi::identity::HostId,
    frame_seq: u64,
    fine_grained_intent: &str,
    class: maos_domain::invariants::i1::IntentClass,
    control: CrossTeamCrossingControl,
) -> Result<IacFrame, String> {
    let payload = control.telemetry_payload()?;
    let mut frame_id = [0u8; 16];
    frame_id[8..].copy_from_slice(&frame_seq.to_be_bytes());
    let mut recipients = smallvec::SmallVec::new();
    recipients.push(maos_domain::frame::FrameAddress {
        spirit_id: from.spirit_id.clone(),
        host_id: Some(peer.clone()),
        role: None,
    });
    Ok(IacFrame {
        frame_id,
        timestamp_ns: 0,
        logical_clock: 0,
        from: from.clone(),
        to: recipients,
        kind: maos_spirit_abi::identity::FrameKind::TelemetryEvent,
        intent: class,
        payload: FramePayload::TelemetryEvent(payload),
        auto_marker: maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
        consent_envelope: Some(
            maos_domain::frame::ConsentEnvelope::with_fine_grained_intent(
                from.clone(),
                maos_domain::invariants::i8::A2AIntent::new(fine_grained_intent),
            ),
        ),
        intent_lineage: maos_domain::invariants::i13::IntentLineage::default(),
    })
}

/// Story 13.6b / AC4 — reconcile the two independent surfaces that name this
/// host's team, fail-closed, at boot.
///
/// `MAOS_LOOM_HOME_TEAM` decides `store.config().home_team`, which becomes
/// `bundle.source_team` on every crossing this host emits. The signed
/// `CohortMember.team` decides the envelope stamp a peer authenticates. Nothing
/// reconciled them, so a host whose two surfaces disagreed emitted crossings the
/// destination refuses under the AC3 weld — a **correctness** failure that reads
/// on the wire like an **attack**. Refuse the boot instead.
///
/// An empty or unset override is not a disagreement: the collective tier is
/// simply not configured, and the composition root already rejects an empty
/// value when `MAOS_LOOM_POSTGRES` is set.
///
/// ⚠ **This does NOT replace the AC3 weld and must never be cited as if it
/// did.** An attacker owns their own boot: they will set the environment
/// correctly and lie in the payload. AC4 is the correctness control against
/// misconfiguration; AC3 is the security control against a peer. Both sentences
/// are true and neither implies the other.
pub fn reconcile_home_team_with_manifest(
    manifest: &maos_cohort::CohortManifest,
    local_host: &str,
    env_team: &str,
) -> Result<(), String> {
    let env_team = env_team.trim();
    if env_team.is_empty() {
        return Ok(());
    }
    match manifest.team_of_host(local_host) {
        Some(signed) if signed.as_str() == env_team => Ok(()),
        Some(signed) => Err(format!(
            "MAOS_LOOM_HOME_TEAM={env_team} disagrees with the signed cohort manifest, which \
             declares team {} for host {local_host} — one host, one team: reconcile the \
             environment with the signed manifest",
            signed.as_str(),
        )),
        None => Err(format!(
            "MAOS_LOOM_HOME_TEAM={env_team} is set but the signed cohort manifest declares NO \
             team for host {local_host} (pre-V4 schema, or a V4 member with no team) — absence \
             never permits: declare the member's team or unset the environment override"
        )),
    }
}


#[cfg(test)]
mod tests {
    use super::{
        erase_frame_with_binding, erase_locator_digest, CrossTeamCrossingAdapter,
        CrossTeamCrossingControl,
    };
    use maos_domain::frame::FrameAddress;
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::memory::MemoryNamespace;
    use maos_domain::team::TeamId;
    use maos_spirit_abi::identity::{HostId, SpiritId};
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_iac::{FrameKind, TransparencyLogAdapter};

    #[test]
    fn erase_provenance_requires_the_complete_applied_share_locator() {
        let audit = TransparencyLogAdapter::open_in_memory(13_600);
        let digest = erase_locator_digest("team-a", "team-b", 7, "default", "k");
        audit.insert_kernel_event_returning_id(
            7,
            "collective.host.cross-team-share",
            br#"{"from_team":"team-a","to_team":"team-b","op_id":"0123456789abcdef0123456789abcdef","locator_digest":"wrong","source_ts":1,"source_region":"r","status":"crossing_applied"}"#,
        );
        assert_eq!(
            CrossTeamCrossingAdapter::share_provenance(
                &audit,
                7,
                "team-a",
                "team-b",
                "0123456789abcdef0123456789abcdef",
                &digest,
            )
            .expect("audit query"),
            None,
            "a different locator digest must not authorize an erase"
        );
        let payload = serde_json::json!({
            "from_team": "team-a",
            "to_team": "team-b",
            "op_id": "0123456789abcdef-0123456789abcdef",
            "locator_digest": digest,
            "source_ts": 11,
            "source_region": "region-a",
            "status": "crossing_applied",
        });
        audit.insert_kernel_event_returning_id(
            7,
            "collective.host.cross-team-share",
            payload.to_string().as_bytes(),
        );
        assert_eq!(
            CrossTeamCrossingAdapter::share_provenance(
                &audit,
                7,
                "team-a",
                "team-b",
                "0123456789abcdef0123456789abcdef",
                &erase_locator_digest("team-a", "team-b", 7, "default", "k"),
            )
            .expect("audit query"),
            Some((11, "region-a".to_string()))
        );
    }

    #[test]
    fn erase_frame_with_readonly_coarse_intent_is_refused() {
        let from = FrameAddress {
            spirit_id: SpiritId::from("control"),
            host_id: Some(HostId("host-b".to_string())),
            role: None,
        };
        let mut frame = erase_frame_with_binding(
            &from,
            &HostId("host-a".to_string()),
            1,
            &TeamId::new("team-a").expect("canonical team"),
            7,
            &MemoryNamespace::Default,
            "k".to_string(),
            "0123456789abcdef0123456789abcdef".to_string(),
            erase_locator_digest("team-a", "team-b", 7, "default", "k"),
        )
        .expect("erase control encodes");
        frame.intent = IntentClass::Readonly;
        assert!(matches!(
            CrossTeamCrossingControl::from_frame(&frame),
            Some(Err(reason)) if reason.contains("coarse intent class")
        ));
    }
}