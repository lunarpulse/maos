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

use std::sync::Arc;

use maos_a2a_core::{
    CrossTeamCrossingPort, CrossingOutcome, CrossingRefusal, COHORT_INTENT_COLLECTIVE_SHARE,
    CROSSING_EVENT_TYPE,
};
use maos_domain::frame::{FramePayload, IacFrame, TelemetryEventPayload};
use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::team::TeamId;
use maos_loom_lite::replication::bundle::{
    apply_replication_bundle, BundleError, CrossRegionReplicationBundle, CrossTeamApplyContext,
};
use maos_loom_lite::store::{LoomLiteStore, StoreError};

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
        bundle: CrossRegionReplicationBundle,
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
        if !intent.eq_ignore_ascii_case(COHORT_INTENT_COLLECTIVE_SHARE) {
            return None;
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
            serde_json::from_str(&payload.data)
                .map_err(|error| format!("crossing frame body is undecodable: {error}")),
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

/// The applier: the composition root's store-owning implementation of the A2A
/// crossing seam, and the **first production caller** of
/// `apply_replication_bundle` → `CrossTeamConsentAdapter::is_granted`.
pub struct CrossTeamCrossingAdapter {
    store: Arc<LoomLiteStore>,
    home_team: TeamId,
    base_seed: [u8; 32],
}

impl CrossTeamCrossingAdapter {
    pub fn new(store: Arc<LoomLiteStore>, home_team: TeamId, base_seed: [u8; 32]) -> Self {
        Self {
            store,
            home_team,
            base_seed,
        }
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
}

#[async_trait::async_trait]
impl CrossTeamCrossingPort for CrossTeamCrossingAdapter {
    async fn apply_crossing(&self, authenticated_team: &str, frame: &IacFrame) -> CrossingOutcome {
        let Some(decoded) = CrossTeamCrossingControl::from_frame(frame) else {
            return CrossingOutcome::NotCrossing;
        };
        let CrossTeamCrossingControl::Share { to_team, bundle } = match decoded {
            Ok(control) => control,
            Err(reason) => {
                return CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
                    reason,
                    from_team: authenticated_team.to_string(),
                    to_team: String::new(),
                    intent: COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
                })
            }
        };

        // ── AC3 / D-13: THE WELD ────────────────────────────────────────────
        // `authenticated_team` is the envelope claim the router has already
        // proven the TLS-verified peer speaks for under the signed V4 manifest.
        // `bundle.source_team` is the field `is_granted` will decide from. If
        // they differ, the emitter stamped a truthful envelope and signed a
        // lying payload — the seed-holding forger of D-10, which the shipped
        // relabel negative structurally cannot reach because its signature
        // verifies correctly. Refuse HERE, before any apply.
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

        let dest_region = self.store.config().home_region.clone();
        let context = CrossTeamApplyContext::new(&requested_team, COHORT_INTENT_COLLECTIVE_SHARE);
        match apply_replication_bundle(
            &bundle,
            &self.store,
            &dest_region,
            Some(context),
            &self.base_seed,
        )
        .await
        {
            Ok(result) => CrossingOutcome::Applied {
                applied_count: result.applied_count,
            },
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
///
/// `prepare_outbound` (`maos-a2a-core/src/router.rs`) stamps
/// `cohort_source_team` onto the wire request from **this host's own signed V4
/// declaration** — never from anything reachable here — so the emitter cannot
/// choose the team it speaks for even though it holds the signing seed. That is
/// what makes the applier's weld a real binding rather than a self-report.
pub fn crossing_frame(
    from: &maos_domain::frame::FrameAddress,
    peer: &maos_spirit_abi::identity::HostId,
    frame_seq: u64,
    to_team: &TeamId,
    bundle: CrossRegionReplicationBundle,
) -> Result<IacFrame, String> {
    let payload = CrossTeamCrossingControl::Share {
        to_team: to_team.as_str().to_string(),
        bundle,
    }
    .telemetry_payload()?;
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
        intent: maos_domain::invariants::i1::IntentClass::Readonly,
        payload: FramePayload::TelemetryEvent(payload),
        auto_marker: maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
        consent_envelope: Some(
            maos_domain::frame::ConsentEnvelope::with_fine_grained_intent(
                from.clone(),
                maos_domain::invariants::i8::A2AIntent::new(COHORT_INTENT_COLLECTIVE_SHARE),
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
