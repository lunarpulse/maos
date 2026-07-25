#![forbid(unsafe_code)]

use maos_a2a_core::{
    CohortConsentDenial, CohortConsentSeam, CohortConsentVerdict, CohortManifestGate,
    CohortReissueDisposition, CohortReissueRejection, DigestFrameClass, DigestReadPort,
    DigestReplyObservation, HaltReceiptObserver, COHORT_INTENT_DIGEST_READ,
};
use maos_domain::frame::IacFrame;
use maos_spirit_abi::identity::HostId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audit::{CohortAuditEvent, CohortAuditSink};
use crate::consent::{accept_admits, send_context, OutboundConsentContext};
use crate::control::CohortManifestControl;
use crate::digest::{DigestReadControl, DigestSummary, DIGEST_DAILY_SCOPE};
use crate::error::{CohortError, CohortManifestForkReason};
use crate::halt_receipt::{AbsenceKind, HaltReceiptControl};
use crate::manifest::CohortManifest;
use crate::pin::PinnedAuthorityKeys;

pub trait CohortClock: Send + Sync {
    fn now_secs(&self) -> u64;
}

struct SystemCohortClock {
    base: Instant,
}

impl SystemCohortClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
        }
    }
}

impl CohortClock for SystemCohortClock {
    fn now_secs(&self) -> u64 {
        self.base.elapsed().as_secs()
    }
}

struct CachedManifest {
    manifest: CohortManifest,
    canonical_hash: [u8; 32],
    /// Exact signed bytes that passed pinned-key verification. Distribution
    /// reuses this artifact rather than reconstructing a body/signature pair.
    signed_toml: String,
    confirmed_at_secs: u64,
}

#[derive(Debug, Clone)]
struct DigestGrant {
    scope: String,
    manifest_version: u64,
}

const MAX_PENDING_DIGEST_READS: usize = 256;

fn schema_is_downgrade(current_schema: u64, candidate_schema: u64) -> bool {
    candidate_schema < current_schema
}

#[cfg(test)]
mod schema_floor_tests {
    use super::schema_is_downgrade;

    #[test]
    fn schema_downgrade_is_an_ordered_floor() {
        assert!(schema_is_downgrade(2, 1));
        assert!(schema_is_downgrade(3, 2));
        assert!(schema_is_downgrade(3, 1));
        assert!(!schema_is_downgrade(2, 2));
        assert!(!schema_is_downgrade(2, 3));
    }
}

/// The verified, local authority for a cohort manifest cache.
///
/// All state transitions verify a candidate under the operator-pinned genesis
/// key set before inspecting its version. This prevents unauthenticated input
/// from advancing or regressing the cached version.
pub struct CohortManifestState {
    pinned: PinnedAuthorityKeys,
    audit: Arc<dyn CohortAuditSink>,
    cached: Mutex<CachedManifest>,
    pull_requests: Mutex<Vec<HostId>>,
    clock: Arc<dyn CohortClock>,
    local_host: HostId,
    /// Story 12.3 — per-member received-receipt presence table: member
    /// `host_id` → the set of distinct `HaltReceipt.halt_id`s observed from
    /// that authenticated member. Dedup is by `halt_id` (P4), so a re-shipped
    /// receipt never inflates the count. Interior-mutable like `pull_requests`.
    halt_presence: Mutex<HashMap<String, HashSet<String>>>,
    /// Story 12.3 — per-member explicit transport-level absence marker recorded
    /// after a classified probe (P2a/P3). Observability only.
    halt_absence: Mutex<HashMap<String, AbsenceKind>>,
    /// Story 12.4a — reader-side live capabilities keyed by authenticated peer
    /// and globally unique request id. Each capability is bound to the admitted
    /// scope and manifest version, then consumed by the first accepted reply.
    outstanding_digest_reads: Mutex<HashMap<(String, String), DigestGrant>>,
    /// Target-side reply capabilities with the same immutable binding.
    admitted_digest_reads: Mutex<HashMap<(String, String), DigestGrant>>,
    /// Target-side bounded reply obligations.
    pending_digest_replies: Mutex<Vec<(String, String, String)>>,
    /// Reader-side immutable summaries keyed by `(member, request_id)`.
    received_digest_summaries: Mutex<HashMap<(String, String), DigestSummary>>,
}

impl CohortManifestState {
    /// Loads a manifest only after strict schema validation and cryptographic
    /// verification under the operator-provisioned authority pin set.
    pub fn load(
        local_host: HostId,
        manifest_toml: &str,
        pinned: PinnedAuthorityKeys,
        audit: Arc<dyn CohortAuditSink>,
    ) -> Result<Self, CohortError> {
        Self::load_with_clock(
            local_host,
            manifest_toml,
            pinned,
            audit,
            Arc::new(SystemCohortClock::new()),
        )
    }

    #[doc(hidden)]
    pub fn load_with_clock(
        local_host: HostId,
        manifest_toml: &str,
        pinned: PinnedAuthorityKeys,
        audit: Arc<dyn CohortAuditSink>,
        clock: Arc<dyn CohortClock>,
    ) -> Result<Self, CohortError> {
        let manifest = CohortManifest::parse_and_validate(manifest_toml, &pinned)?;
        manifest.verify_signature(&pinned)?;
        Ok(Self {
            local_host,
            pinned,
            audit,
            cached: Mutex::new(CachedManifest {
                canonical_hash: manifest.canonical_hash(),
                manifest,
                signed_toml: manifest_toml.to_string(),
                confirmed_at_secs: clock.now_secs(),
            }),
            pull_requests: Mutex::new(Vec::new()),
            halt_presence: Mutex::new(HashMap::new()),
            halt_absence: Mutex::new(HashMap::new()),
            outstanding_digest_reads: Mutex::new(HashMap::new()),
            admitted_digest_reads: Mutex::new(HashMap::new()),
            pending_digest_replies: Mutex::new(Vec::new()),
            received_digest_summaries: Mutex::new(HashMap::new()),
            clock,
        })
    }

    pub fn manifest(&self) -> Result<CohortManifest, CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .clone())
    }

    pub fn version(&self) -> Result<u64, CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .version)
    }

    pub(crate) fn accept_consent(
        &self,
        sender_peer: &str,
        acting_role: Option<&str>,
        intent: &str,
        sender_version: Option<u64>,
    ) -> Result<(), CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        accept_admits(
            &cached.manifest,
            sender_peer,
            acting_role,
            intent,
            sender_version,
        )
    }

    pub(crate) fn outbound_consent_context(
        &self,
        receiver_peer: &str,
        intent: &str,
    ) -> Result<OutboundConsentContext, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        send_context(
            &cached.manifest,
            self.local_host.as_str(),
            receiver_peer,
            intent,
        )
    }

    pub fn canonical_hash(&self) -> Result<[u8; 32], CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .canonical_hash)
    }

    /// Host identity bound when this verified state was loaded.
    pub fn local_host(&self) -> &HostId {
        &self.local_host
    }

    /// The exact verified, signed manifest artifact suitable for a push.
    pub fn signed_toml(&self) -> Result<String, CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .signed_toml
            .clone())
    }

    /// Drain verified peers awaiting a signed push. Router code only enqueues;
    /// composition owns outbound delivery and its cancellation lifecycle.
    pub fn take_pull_requests(&self) -> Result<Vec<HostId>, CohortError> {
        let mut requests = self
            .pull_requests
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(std::mem::take(&mut *requests))
    }

    /// Applies a signed reissue. A same-version, same-body manifest is an
    /// idempotent confirmation. A lower verified version or a divergent verified
    /// body at the same version is rejected with a specific fork discriminant.
    pub fn apply_reissue(&self, manifest_toml: &str) -> Result<ReissueOutcome, CohortError> {
        let candidate = match CohortManifest::parse_and_validate(manifest_toml, &self.pinned) {
            Ok(candidate) => candidate,
            Err(CohortError::ECohortAuthorityUnpinned { .. }) => {
                return self
                    .reject_non_authority(reissue_version(manifest_toml).unwrap_or_default());
            }
            Err(error) => return Err(error),
        };
        if candidate.verify_signature(&self.pinned).is_err() {
            return self.reject_non_authority(candidate.version);
        }
        let candidate_hash = candidate.canonical_hash();

        let mut cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let seen_version = cached.manifest.version;
        let rejected_version = candidate.version;

        if candidate.cohort_id != cached.manifest.cohort_id {
            let expected_cohort_id = cached.manifest.cohort_id.clone();
            drop(cached);
            let error = CohortError::ECohortIdMismatch {
                expected_cohort_id: expected_cohort_id.clone(),
                rejected_cohort_id: candidate.cohort_id,
                seen_version,
                rejected_version,
            };
            self.audit.append(&CohortAuditEvent::ReissueRejected {
                cohort_id: expected_cohort_id,
                seen_version,
                rejected_version,
                reason: error.to_string(),
            })?;
            return Err(error);
        }

        let rejection =
            if schema_is_downgrade(cached.manifest.schema_version, candidate.schema_version) {
                Some(CohortManifestForkReason::SchemaDowngrade)
            } else if candidate.version < seen_version {
                Some(CohortManifestForkReason::VersionRegression)
            } else if candidate.version == seen_version && candidate_hash != cached.canonical_hash {
                Some(CohortManifestForkReason::ConcurrentFork)
            } else {
                None
            };

        if let Some(reason) = rejection {
            let error = CohortError::ECohortManifestFork {
                reason,
                seen_version,
                rejected_version,
            };
            self.audit.append(&CohortAuditEvent::ReissueRejected {
                cohort_id: cached.manifest.cohort_id.clone(),
                seen_version,
                rejected_version,
                reason: error.to_string(),
            })?;
            return Err(error);
        }

        if candidate.version == seen_version {
            cached.confirmed_at_secs = self.clock.now_secs();
            return Ok(ReissueOutcome::Confirmed {
                version: seen_version,
            });
        }

        self.audit
            .append(&CohortAuditEvent::MemberReissueAccepted {
                cohort_id: candidate.cohort_id.clone(),
                version: candidate.version,
                canonical_hash: candidate_hash,
            })?;
        let version = candidate.version;
        *cached = CachedManifest {
            manifest: candidate,
            canonical_hash: candidate_hash,
            signed_toml: manifest_toml.to_string(),
            confirmed_at_secs: self.clock.now_secs(),
        };
        // Correlation capabilities are grants under one signed manifest
        // snapshot. A reissue invalidates every in-flight exemption; callers
        // must mint a fresh request under the new matrix.
        self.outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .clear();
        self.admitted_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .clear();
        self.pending_digest_replies
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .clear();
        Ok(ReissueOutcome::Applied { version })
    }

    /// Records an authority-originated signed reissue before applying it to the
    /// authority's own verified cache. The caller retains key custody and must
    /// supply a fully signed TOML artifact; this state never stores a signing
    /// key. Both origin and member-adoption events are therefore observable.
    pub fn issue_reissue(&self, manifest_toml: &str) -> Result<ReissueOutcome, CohortError> {
        let candidate = match CohortManifest::parse_and_validate(manifest_toml, &self.pinned) {
            Ok(candidate) => candidate,
            Err(CohortError::ECohortAuthorityUnpinned { .. }) => {
                return self
                    .reject_non_authority(reissue_version(manifest_toml).unwrap_or_default());
            }
            Err(error) => return Err(error),
        };
        if candidate.verify_signature(&self.pinned).is_err() {
            return self.reject_non_authority(candidate.version);
        }
        if candidate.cohort_id != self.manifest()?.cohort_id {
            return self.apply_reissue(manifest_toml);
        }
        let hash = candidate.canonical_hash();
        let seen_version = self.version()?;
        if candidate.version <= seen_version {
            return Err(CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::VersionRegression,
                seen_version,
                rejected_version: candidate.version,
            });
        }
        self.audit
            .append(&CohortAuditEvent::AuthorityReissueIssued {
                cohort_id: candidate.cohort_id,
                version: candidate.version,
                canonical_hash: hash,
            })?;
        self.apply_reissue(manifest_toml)
    }

    /// Staleness is evaluated against the signed, code-clamped lease. At the
    /// precise lease boundary the state remains current; it becomes stale only
    /// after the elapsed duration exceeds the lease.
    pub fn is_fresh(&self) -> bool {
        let Ok(cached) = self.cached.lock() else {
            return false;
        };
        self.clock
            .now_secs()
            .saturating_sub(cached.confirmed_at_secs)
            <= cached.manifest.t_stale_secs
    }

    /// Freshness and the manifest from ONE locked snapshot: the lease check
    /// and the returned manifest are evaluated against the same
    /// `confirmed_at_secs`, so a lease turning stale between two separate
    /// lock acquisitions can no longer admit a grant on a stale snapshot
    /// (13.3 review). `Ok(None)` = the lease is stale (fail-closed,
    /// distinguishable from an unavailable state, which is `Err`).
    pub fn manifest_if_fresh(&self) -> Result<Option<CohortManifest>, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let fresh = self
            .clock
            .now_secs()
            .saturating_sub(cached.confirmed_at_secs)
            <= cached.manifest.t_stale_secs;
        Ok(fresh.then(|| cached.manifest.clone()))
    }

    /// Refresh before half of the signed stale lease elapses, leaving the other
    /// half to receive and verify the authority's signed confirmation.
    pub fn confirmation_interval(&self) -> Result<Duration, CohortError> {
        let cached = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        Ok(Duration::from_secs((cached.manifest.t_stale_secs + 1) / 2))
    }

    fn reject_non_authority(&self, rejected_version: u64) -> Result<ReissueOutcome, CohortError> {
        let (cohort_id, seen_version) = {
            let cached = self
                .cached
                .lock()
                .map_err(|_| CohortError::EStatePoisoned)?;
            (cached.manifest.cohort_id.clone(), cached.manifest.version)
        };
        let error = CohortError::ECohortManifestFork {
            reason: CohortManifestForkReason::NonAuthoritySigner,
            seen_version,
            rejected_version,
        };
        self.audit.append(&CohortAuditEvent::ReissueRejected {
            cohort_id,
            seen_version,
            rejected_version,
            reason: error.to_string(),
        })?;
        Err(error)
    }

    fn control_rejection(&self, error: CohortError) -> CohortReissueRejection {
        let (seen_version, rejected_version) = match &error {
            CohortError::ECohortManifestFork {
                seen_version,
                rejected_version,
                ..
            }
            | CohortError::ECohortIdMismatch {
                seen_version,
                rejected_version,
                ..
            } => (Some(*seen_version), Some(*rejected_version)),
            _ => (self.version().ok(), None),
        };
        CohortReissueRejection {
            reason: error.to_string(),
            seen_version,
            rejected_version,
        }
    }

    /// Story 12.3 — record an explicit transport-level absence marker for a
    /// probed member (P2a/P3). Observability only; the caller classifies the
    /// probe result (`HaltReceiptDistributor::classify_presence`) and records
    /// the induced-loss outcome here. Poisoned lock is a best-effort drop —
    /// absence is never a trust decision.
    pub fn record_absence(&self, member: &HostId, kind: AbsenceKind) {
        if let Ok(mut table) = self.halt_absence.lock() {
            table.insert(member.as_str().to_string(), kind);
        }
    }

    /// Story 12.3 — the count of DISTINCT halt receipts observed from `member`
    /// (dedup by `halt_id`, P4). Feeds the 12.4 digest. A poisoned lock reads 0.
    pub fn present_receipt_count(&self, member: &HostId) -> usize {
        self.halt_presence
            .lock()
            .ok()
            .and_then(|table| table.get(member.as_str()).map(HashSet::len))
            .unwrap_or(0)
    }

    /// Story 12.3 — whether a specific receipt (by `halt_id`) is present for a
    /// member.
    pub fn is_receipt_present(&self, member: &HostId, halt_id: &str) -> bool {
        self.halt_presence
            .lock()
            .ok()
            .and_then(|table| table.get(member.as_str()).map(|ids| ids.contains(halt_id)))
            .unwrap_or(false)
    }

    /// Story 12.3 — the recorded absence marker for a member, if any.
    pub fn absence_of(&self, member: &HostId) -> Option<AbsenceKind> {
        self.halt_absence
            .lock()
            .ok()
            .and_then(|table| table.get(member.as_str()).copied())
    }
}

fn reissue_version(manifest_toml: &str) -> Option<u64> {
    toml::from_str::<toml::Value>(manifest_toml)
        .ok()?
        .get("version")?
        .as_integer()
        .and_then(|version| u64::try_from(version).ok())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReissueOutcome {
    Applied { version: u64 },
    Confirmed { version: u64 },
}

impl CohortManifestGate for CohortManifestState {
    fn consent_decision(
        &self,
        seam: CohortConsentSeam,
        counterparty: &HostId,
        acting_role: Option<&str>,
        intent: &str,
        sender_manifest_version: Option<u64>,
    ) -> CohortConsentVerdict {
        let cached = match self.cached.lock() {
            Ok(cached) => cached,
            Err(_) => return CohortConsentVerdict::Deny(CohortConsentDenial::StateUnavailable),
        };
        let manifest = &cached.manifest;

        // A peer outside the roster is a mixed-deployment bilateral path, not a
        // cohort denial. This preserves the legacy defer behavior.
        if !manifest
            .members
            .iter()
            .any(|member| member.host_id == counterparty.as_str())
        {
            return CohortConsentVerdict::Defer;
        }
        let current = self
            .clock
            .now_secs()
            .saturating_sub(cached.confirmed_at_secs)
            <= manifest.t_stale_secs
            && manifest
                .members
                .iter()
                .any(|member| member.host_id == self.local_host.as_str());
        if !current {
            return CohortConsentVerdict::NotCurrent;
        }

        let decision = match seam {
            CohortConsentSeam::Send => send_context(
                manifest,
                self.local_host.as_str(),
                counterparty.as_str(),
                intent,
            )
            .map(|context| CohortConsentVerdict::AdmitOutbound {
                acting_role: context.acting_role,
                manifest_version: context.manifest_version,
            }),
            CohortConsentSeam::Accept => accept_admits(
                manifest,
                counterparty.as_str(),
                acting_role,
                intent,
                sender_manifest_version,
            )
            .map(|()| CohortConsentVerdict::Admit),
        };
        match decision {
            Ok(verdict) => verdict,
            Err(CohortError::EConsentPeerNotMember { ref peer, .. })
                if peer == counterparty.as_str() =>
            {
                CohortConsentVerdict::Defer
            }
            Err(CohortError::EConsentActingRoleAbsent) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::ActingRoleAbsent)
            }
            Err(CohortError::EConsentManifestVersionAbsent) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::ManifestVersionAbsent)
            }
            Err(CohortError::EConsentRoleNotEntitled { .. }) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::RoleNotEntitled)
            }
            Err(CohortError::EConsentTupleDenied { .. }) => {
                CohortConsentVerdict::Deny(CohortConsentDenial::NoGrant)
            }
            Err(CohortError::ECohortManifestSkew {
                sender_version,
                receiver_version,
                delta,
            }) => CohortConsentVerdict::Deny(CohortConsentDenial::ManifestSkew {
                sender_version,
                receiver_version,
                delta,
            }),
            Err(_) => CohortConsentVerdict::Deny(CohortConsentDenial::StateUnavailable),
        }
    }

    fn apply_reissue(
        &self,
        verified_peer: &HostId,
        frame: &maos_domain::frame::IacFrame,
    ) -> Result<CohortReissueDisposition, CohortReissueRejection> {
        match CohortManifestControl::from_frame(frame) {
            Ok(CohortManifestControl::Push { manifest_toml }) => self
                .apply_reissue(&manifest_toml)
                .map(|outcome| match outcome {
                    ReissueOutcome::Applied { version } => {
                        CohortReissueDisposition::Applied { version }
                    }
                    ReissueOutcome::Confirmed { version } => {
                        CohortReissueDisposition::Confirmed { version }
                    }
                })
                .map_err(|error| self.control_rejection(error)),
            Ok(CohortManifestControl::Pull { .. }) => {
                self.pull_requests
                    .lock()
                    .map_err(|_| CohortReissueRejection {
                        reason: CohortError::EStatePoisoned.to_string(),
                        seen_version: self.version().ok(),
                        rejected_version: None,
                    })?
                    .push(verified_peer.clone());
                Ok(CohortReissueDisposition::PullRequested)
            }
            Err(error) => Err(CohortReissueRejection {
                reason: error.to_string(),
                seen_version: self.version().ok(),
                rejected_version: None,
            }),
        }
    }
}

impl HaltReceiptObserver for CohortManifestState {
    /// Record receipt-presence for the authenticated `member` (P5r — the caller
    /// [`crate::A2ARouterCore::handle_intake_verified`] has proven
    /// `frame.from.host_id == verified_peer`, and the receipt payload carries no
    /// host identity, so `member` is the sole trustworthy emitter). Dedup is by
    /// the receipt's stable `halt_id` (P4). A frame that is not a well-formed
    /// halt-receipt control envelope is silently ignored (it never reaches this
    /// arm on the production path). Observability ONLY — this type has no method
    /// that resolves, resumes, or overrides a halt, and `maos-cohort` does not
    /// depend on `maos-kernel-core`, so the arbitration sink is unreachable (AC3).
    fn observe_receipt(&self, member: &HostId, frame: &IacFrame) {
        let Ok(control) = HaltReceiptControl::from_frame(frame) else {
            return;
        };
        let halt_id = control.halt_id().to_string();
        if let Ok(mut table) = self.halt_presence.lock() {
            table
                .entry(member.as_str().to_string())
                .or_default()
                .insert(halt_id);
        }
    }
}

impl CohortManifestState {
    pub fn note_digest_request_sent(
        &self,
        peer: &HostId,
        request_id: &str,
        scope: &str,
    ) -> Result<(), CohortError> {
        let version = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .manifest
            .version;
        let mut outstanding = self
            .outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        let received = self
            .received_digest_summaries
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        if outstanding.keys().any(|(_, id)| id == request_id)
            || received.keys().any(|(_, id)| id == request_id)
        {
            return Err(CohortError::EInvalidDigestRequest(
                "request_id must be globally unique for this reader".into(),
            ));
        }
        if outstanding.len() >= MAX_PENDING_DIGEST_READS {
            return Err(CohortError::EDigestCapacityExceeded(
                "reader has too many outstanding requests".into(),
            ));
        }
        outstanding.insert(
            (peer.as_str().to_string(), request_id.to_string()),
            DigestGrant {
                scope: scope.to_string(),
                manifest_version: version,
            },
        );
        Ok(())
    }

    pub fn cancel_digest_request(
        &self,
        peer: &HostId,
        request_id: &str,
    ) -> Result<(), CohortError> {
        self.outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .remove(&(peer.as_str().to_string(), request_id.to_string()));
        Ok(())
    }

    pub fn complete_admitted_digest_reply(
        &self,
        peer: &HostId,
        request_id: &str,
    ) -> Result<(), CohortError> {
        self.admitted_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .remove(&(peer.as_str().to_string(), request_id.to_string()));
        Ok(())
    }

    pub fn drain_pending_digest_replies(
        &self,
    ) -> Result<Vec<(String, String, String)>, CohortError> {
        Ok(std::mem::take(
            &mut *self
                .pending_digest_replies
                .lock()
                .map_err(|_| CohortError::EStatePoisoned)?,
        ))
    }

    pub fn requeue_pending_digest_reply(
        &self,
        requester: String,
        request_id: String,
        scope: String,
    ) -> Result<(), CohortError> {
        let mut queue = self
            .pending_digest_replies
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?;
        if queue.len() >= MAX_PENDING_DIGEST_READS {
            return Err(CohortError::EDigestCapacityExceeded(
                "target has too many pending replies".into(),
            ));
        }
        if !queue
            .iter()
            .any(|(peer, id, _)| peer == &requester && id == &request_id)
        {
            queue.push((requester, request_id, scope));
        }
        Ok(())
    }

    pub fn digest_summary(&self, peer: &HostId, request_id: &str) -> Option<DigestSummary> {
        self.received_digest_summaries.lock().ok().and_then(|map| {
            map.get(&(peer.as_str().to_string(), request_id.to_string()))
                .cloned()
        })
    }

    pub fn digest_summary_count(&self) -> usize {
        self.received_digest_summaries
            .lock()
            .map(|map| map.len())
            .unwrap_or(0)
    }
}

impl DigestReadPort for CohortManifestState {
    /// Parse a `cohort:digest-read` frame into its request/reply class. A frame
    /// that is not a well-formed digest-read envelope is `NotDigest` (the router
    /// then treats it as an ordinary consent-gated frame).
    fn classify(&self, frame: &IacFrame) -> DigestFrameClass {
        match DigestReadControl::from_frame(frame) {
            Ok(DigestReadControl::Request { request_id, .. }) => {
                DigestFrameClass::Request { request_id }
            }
            Ok(DigestReadControl::Reply { request_id, .. }) => {
                DigestFrameClass::Reply { request_id }
            }
            Err(_)
                if frame
                    .consent_envelope
                    .as_ref()
                    .and_then(|envelope| envelope.intent_class.as_ref())
                    .is_some_and(|intent| {
                        intent
                            .as_str()
                            .eq_ignore_ascii_case(COHORT_INTENT_DIGEST_READ)
                    }) =>
            {
                DigestFrameClass::Invalid
            }
            Err(_) => DigestFrameClass::NotDigest,
        }
    }

    fn note_admitted_request_guarded(
        &self,
        requester: &HostId,
        request_id: &str,
        frame: &IacFrame,
        before_commit: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        let DigestReadControl::Request {
            request_id: parsed_id,
            scope,
        } = DigestReadControl::from_frame(frame).map_err(|error| error.to_string())?
        else {
            return Err("admitted digest frame is not a request".into());
        };
        if parsed_id != request_id {
            return Err("classified request id changed before admission".into());
        }
        let version = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?
            .manifest
            .version;
        let key = (requester.as_str().to_string(), request_id.to_string());
        let mut admitted = self
            .admitted_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        if admitted.contains_key(&key) {
            return Ok(());
        }
        let mut queue = self
            .pending_digest_replies
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        if admitted.len() >= MAX_PENDING_DIGEST_READS || queue.len() >= MAX_PENDING_DIGEST_READS {
            return Err(CohortError::EDigestCapacityExceeded(
                "target has too many admitted digest reads".into(),
            )
            .to_string());
        }
        before_commit()?;
        self.audit
            .append(&CohortAuditEvent::DigestReadRequested {
                requester: requester.as_str().to_string(),
                request_id: request_id.to_string(),
                scope: scope.clone(),
            })
            .map_err(|error| error.to_string())?;
        admitted.insert(
            key.clone(),
            DigestGrant {
                scope: scope.clone(),
                manifest_version: version,
            },
        );
        queue.push((key.0, key.1, scope));
        Ok(())
    }

    fn authorize_reply_send(&self, peer: &HostId, request_id: &str) -> bool {
        let Ok(cached) = self.cached.lock() else {
            return false;
        };
        self.admitted_digest_reads
            .lock()
            .map(|grants| {
                grants
                    .get(&(peer.as_str().to_string(), request_id.to_string()))
                    .is_some_and(|grant| {
                        grant.manifest_version == cached.manifest.version
                            && grant.scope == DIGEST_DAILY_SCOPE
                    })
            })
            .unwrap_or(false)
    }

    fn observe_reply(
        &self,
        peer: &HostId,
        frame: &IacFrame,
    ) -> Result<DigestReplyObservation, String> {
        let DigestReadControl::Reply {
            request_id,
            summary,
        } = DigestReadControl::from_frame(frame).map_err(|error| error.to_string())?
        else {
            return Ok(DigestReplyObservation::Unauthorized);
        };
        let version = self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?
            .manifest
            .version;
        let key = (peer.as_str().to_string(), request_id.clone());
        let mut outstanding = self
            .outstanding_digest_reads
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        let mut received = self
            .received_digest_summaries
            .lock()
            .map_err(|_| CohortError::EStatePoisoned.to_string())?;
        if let Some(existing) = received.get(&key) {
            return if existing == &summary {
                Ok(DigestReplyObservation::Duplicate)
            } else {
                Err("conflicting replay attempted to replace a recorded digest summary".into())
            };
        }
        let Some(grant) = outstanding.get(&key) else {
            return Ok(DigestReplyObservation::Unauthorized);
        };
        if grant.manifest_version != version || grant.scope != DIGEST_DAILY_SCOPE {
            return Ok(DigestReplyObservation::Unauthorized);
        }
        self.audit
            .append(&CohortAuditEvent::DigestReplyReceived {
                member: peer.as_str().to_string(),
                request_id: request_id.clone(),
            })
            .map_err(|error| error.to_string())?;
        outstanding.remove(&key);
        received.insert(key, summary);
        Ok(DigestReplyObservation::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::audit::InMemoryCohortAuditSink;
    use crate::manifest::{
        CohortAuthority, CohortMember, ConsentMatrix, ConsentTuple, ManifestSignature, TeamEntry,
        COHORT_SCHEMA_V1, COHORT_SCHEMA_V2, COHORT_SCHEMA_V3, RESERVED_INTENT_HALT_RECEIPT,
        RESERVED_INTENT_REISSUE,
    };
    use maos_domain::frame::{ConsentEnvelope, FrameAddress, FramePayload, TelemetryEventPayload};
    use maos_domain::invariants::i1::IntentClass;
    use maos_domain::invariants::i13::IntentLineage;
    use maos_domain::invariants::i3::FrameOrigin;
    use maos_domain::invariants::i8::A2AIntent;
    use maos_domain::region::Region;
    use maos_domain::team::TeamId;
    use maos_spirit_abi::identity::{FrameKind, SpiritId, SpiritRole};
    use smallvec::smallvec;

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn signed_toml_for(
        version: u64,
        signer: &SigningKey,
        authority: &SigningKey,
        cohort_id: &str,
    ) -> String {
        let manifest = CohortManifest {
            schema_version: COHORT_SCHEMA_V1,
            cohort_id: cohort_id.to_string(),
            version,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![hex::encode(authority.verifying_key().to_bytes())],
            },
            members: vec![CohortMember {
                host_id: "host-a".into(),
                fingerprint: format!("sha256:{}", "ab".repeat(32)),
                roles: vec!["worker".into()],
            }],
            consent: ConsentMatrix::default(),
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.into(),
                RESERVED_INTENT_HALT_RECEIPT.into(),
            ],
            t_stale_secs: 120,
            teams: None,
            signature: ManifestSignature { sig: String::new() },
            cross_team_consent: Vec::new(),
        }
        .signed_with(signer);
        toml::to_string(&manifest).expect("serializable test manifest")
    }

    fn signed_tenant_toml(schema_version: u64, version: u64, signer: &SigningKey) -> String {
        let mut manifest: CohortManifest =
            toml::from_str(&signed_toml(version, signer, signer)).unwrap();
        manifest.schema_version = schema_version;
        manifest.teams = Some(vec![TeamEntry {
            team_id: TeamId::new("team-a").unwrap(),
            region: Region::canonicalize("region-a").unwrap(),
            datname: "maos_team_a".to_string(),
            members: vec![SpiritId::from("spirit-a")],
        }]);
        toml::to_string(&manifest.signed_with(signer)).unwrap()
    }

    fn digest_frame(from: &str, to: &str, control: DigestReadControl) -> IacFrame {
        let from_address = FrameAddress {
            spirit_id: SpiritId::from("digest"),
            host_id: Some(HostId(from.into())),
            role: Some(SpiritRole::Worker),
        };
        IacFrame {
            frame_id: [7; 16],
            timestamp_ns: 0,
            logical_clock: 1,
            from: from_address.clone(),
            to: smallvec![FrameAddress {
                spirit_id: SpiritId::from("digest"),
                host_id: Some(HostId(to.into())),
                role: None,
            }],
            kind: FrameKind::TelemetryEvent,
            intent: IntentClass::Readonly,
            payload: FramePayload::TelemetryEvent(
                control.telemetry_payload().expect("valid digest control"),
            ),
            auto_marker: FrameOrigin::SpiritAuto,
            consent_envelope: Some(ConsentEnvelope::with_fine_grained_intent(
                from_address,
                A2AIntent::new(COHORT_INTENT_DIGEST_READ),
            )),
            intent_lineage: IntentLineage::default(),
        }
    }

    fn signed_toml_with_stale_secs(
        version: u64,
        signer: &SigningKey,
        authority: &SigningKey,
        t_stale_secs: u64,
    ) -> String {
        let mut manifest: CohortManifest =
            toml::from_str(&signed_toml_for(version, signer, authority, "cohort-test"))
                .expect("fixture manifest parses");
        manifest.t_stale_secs = t_stale_secs;
        toml::to_string(&manifest.signed_with(signer)).expect("resigned fixture serializes")
    }

    fn signed_toml(version: u64, signer: &SigningKey, authority: &SigningKey) -> String {
        signed_toml_for(version, signer, authority, "cohort-test")
    }

    fn state(
        version: u64,
        signer: &SigningKey,
        audit: Arc<dyn CohortAuditSink>,
    ) -> CohortManifestState {
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        CohortManifestState::load(
            HostId("host-a".into()),
            &signed_toml(version, signer, signer),
            pins,
            audit,
        )
        .unwrap()
    }

    fn consent_state(version: u64, signer: &SigningKey) -> CohortManifestState {
        let manifest = CohortManifest {
            schema_version: COHORT_SCHEMA_V1,
            cohort_id: "consent-state".into(),
            version,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![hex::encode(signer.verifying_key().to_bytes())],
            },
            members: vec![
                CohortMember {
                    host_id: "host-a".into(),
                    fingerprint: format!("sha256:{}", "aa".repeat(32)),
                    roles: vec!["architect".into()],
                },
                CohortMember {
                    host_id: "host-b".into(),
                    fingerprint: format!("sha256:{}", "bb".repeat(32)),
                    roles: vec!["receiver".into()],
                },
            ],
            consent: ConsentMatrix {
                send: vec![ConsentTuple {
                    peer: "host-b".into(),
                    role: "receiver".into(),
                    intent: "cohort-work:write".into(),
                }],
                accept: vec![ConsentTuple {
                    peer: "host-a".into(),
                    role: "architect".into(),
                    intent: "cohort-work:write".into(),
                }],
            },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.into(),
                RESERVED_INTENT_HALT_RECEIPT.into(),
            ],
            t_stale_secs: 120,
            teams: None,
            signature: ManifestSignature { sig: String::new() },
            cross_team_consent: Vec::new(),
        }
        .signed_with(signer);
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        CohortManifestState::load(
            HostId("host-b".into()),
            &toml::to_string(&manifest).unwrap(),
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap()
    }

    #[test]
    fn consent_reads_the_live_manifest_version_for_each_frame() {
        let signer = signing_key(14);
        let state = consent_state(4, &signer);
        state
            .accept_consent("host-a", Some("architect"), "cohort-work:write", Some(4))
            .expect("same-version frame admits");

        let replacement = consent_state(6, &signer).signed_toml().unwrap();
        state.apply_reissue(&replacement).unwrap();
        assert!(matches!(
            state.accept_consent("host-a", Some("architect"), "cohort-work:write", Some(4),),
            Err(CohortError::ECohortManifestSkew {
                sender_version: 4,
                receiver_version: 6,
                delta: 2,
            })
        ));
    }

    #[test]
    fn applies_higher_verified_reissue_after_auditing() {
        let signer = signing_key(7);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &signer, audit.clone());

        assert_eq!(
            state
                .apply_reissue(&signed_toml(2, &signer, &signer))
                .unwrap(),
            ReissueOutcome::Applied { version: 2 }
        );
        assert_eq!(state.version().unwrap(), 2);
        assert!(matches!(
            audit.events().as_slice(),
            [CohortAuditEvent::MemberReissueAccepted { version: 2, .. }]
        ));
    }

    #[test]
    fn peer_absent_from_current_manifest_defers_to_bilateral_path() {
        let signer = signing_key(13);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        assert_eq!(
            CohortManifestGate::consent_decision(
                &state,
                CohortConsentSeam::Send,
                &HostId("host-b".into()),
                None,
                "cohort-work:write",
                None,
            ),
            CohortConsentVerdict::Defer,
            "an unknown peer remains eligible for the legacy bilateral path"
        );
    }

    #[test]
    fn authority_issue_is_journaled_before_member_adoption() {
        let signer = signing_key(6);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &signer, audit.clone());

        assert_eq!(
            state
                .issue_reissue(&signed_toml(2, &signer, &signer))
                .unwrap(),
            ReissueOutcome::Applied { version: 2 }
        );
        assert!(matches!(
            audit.events().as_slice(),
            [
                CohortAuditEvent::AuthorityReissueIssued { version: 2, .. },
                CohortAuditEvent::MemberReissueAccepted { version: 2, .. },
            ]
        ));
    }

    #[test]
    fn same_verified_body_confirms_but_divergent_body_forks() {
        let signer = signing_key(8);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(2, &signer, audit);
        let same = signed_toml(2, &signer, &signer);

        assert_eq!(
            state.apply_reissue(&same).unwrap(),
            ReissueOutcome::Confirmed { version: 2 }
        );

        let divergent = signed_toml_with_stale_secs(2, &signer, &signer, 121);
        let error = state.apply_reissue(&divergent).unwrap_err();
        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::ConcurrentFork,
                seen_version: 2,
                rejected_version: 2,
            }
        ));
    }

    #[test]
    fn lower_verified_version_is_a_version_regression() {
        let signer = signing_key(9);
        let state = state(2, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        let error = state
            .apply_reissue(&signed_toml(1, &signer, &signer))
            .unwrap_err();

        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::VersionRegression,
                seen_version: 2,
                rejected_version: 1,
            }
        ));
    }

    #[test]
    fn wrong_signer_is_journaled_with_actual_versions() {
        let authority = signing_key(10);
        let intruder = signing_key(11);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &authority, audit.clone());
        let error = state
            .apply_reissue(&signed_toml(2, &intruder, &authority))
            .unwrap_err();

        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::NonAuthoritySigner,
                seen_version: 1,
                rejected_version: 2,
            }
        ));
        assert!(matches!(
            audit.events().as_slice(),
            [CohortAuditEvent::ReissueRejected {
                seen_version: 1,
                rejected_version: 2,
                ..
            }]
        ));
        assert_eq!(state.version().unwrap(), 1);
    }

    #[test]
    fn higher_version_cannot_replace_the_cohort_identity() {
        let authority = signing_key(11);
        let audit = Arc::new(InMemoryCohortAuditSink::default());
        let state = state(1, &authority, audit.clone());

        let error = state
            .apply_reissue(&signed_toml_for(2, &authority, &authority, "other-cohort"))
            .unwrap_err();

        assert!(matches!(
            error,
            CohortError::ECohortIdMismatch {
                seen_version: 1,
                rejected_version: 2,
                ..
            }
        ));
        assert!(matches!(
            audit.events().as_slice(),
            [CohortAuditEvent::ReissueRejected {
                seen_version: 1,
                rejected_version: 2,
                ..
            }]
        ));
        assert_eq!(state.manifest().unwrap().cohort_id, "cohort-test");
    }

    #[test]
    fn confirmation_interval_leaves_half_the_stale_lease_for_delivery() {
        let signer = signing_key(12);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));

        assert_eq!(
            state.confirmation_interval().unwrap(),
            Duration::from_secs(60),
            "a 120-second stale lease must pull at its halfway point"
        );
    }

    struct RejectingAudit;

    impl CohortAuditSink for RejectingAudit {
        fn append(&self, _event: &CohortAuditEvent) -> Result<(), CohortError> {
            Err(CohortError::EAuditAppendFailed(
                "simulated transparency-log failure".into(),
            ))
        }
    }

    #[test]
    fn audit_failure_prevents_reissue_publication() {
        let signer = signing_key(12);
        let state = state(1, &signer, Arc::new(RejectingAudit));

        assert!(matches!(
            state.apply_reissue(&signed_toml(2, &signer, &signer)),
            Err(CohortError::EAuditAppendFailed(_))
        ));
        assert_eq!(state.version().unwrap(), 1);
    }

    #[test]
    fn digest_audit_failure_publishes_no_reply_capability() {
        let signer = signing_key(13);
        let state = state(1, &signer, Arc::new(RejectingAudit));
        let request = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Request {
                request_id: "host-b:0001".into(),
                scope: DIGEST_DAILY_SCOPE.into(),
            },
        );

        assert!(DigestReadPort::note_admitted_request(
            &state,
            &HostId("host-b".into()),
            "host-b:0001",
            &request,
        )
        .is_err());
        assert!(!DigestReadPort::authorize_reply_send(
            &state,
            &HostId("host-b".into()),
            "host-b:0001",
        ));
        assert!(state
            .drain_pending_digest_replies()
            .expect("queue remains readable")
            .is_empty());
    }

    #[test]
    fn guarded_digest_admission_skips_side_effects_for_duplicates_and_capacity_rejections() {
        let signer = signing_key(14);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        let requester = HostId("host-b".into());

        let first_id = "host-b:guarded-0000";
        let first = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Request {
                request_id: first_id.into(),
                scope: DIGEST_DAILY_SCOPE.into(),
            },
        );
        let mut guard_calls = 0usize;
        DigestReadPort::note_admitted_request_guarded(
            &state,
            &requester,
            first_id,
            &first,
            &mut || {
                guard_calls += 1;
                Ok(())
            },
        )
        .expect("first request admits");
        DigestReadPort::note_admitted_request_guarded(
            &state,
            &requester,
            first_id,
            &first,
            &mut || {
                guard_calls += 1;
                Ok(())
            },
        )
        .expect("duplicate remains idempotent");
        assert_eq!(
            guard_calls, 1,
            "duplicate request must not repeat irreversible governance"
        );

        for index in 1..MAX_PENDING_DIGEST_READS {
            let request_id = format!("host-b:guarded-{index:04}");
            let frame = digest_frame(
                "host-b",
                "host-a",
                DigestReadControl::Request {
                    request_id: request_id.clone(),
                    scope: DIGEST_DAILY_SCOPE.into(),
                },
            );
            DigestReadPort::note_admitted_request(&state, &requester, &request_id, &frame)
                .expect("fill pending digest capacity");
        }
        let overflow_id = "host-b:guarded-overflow";
        let overflow = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Request {
                request_id: overflow_id.into(),
                scope: DIGEST_DAILY_SCOPE.into(),
            },
        );
        let mut overflow_guard_calls = 0usize;
        let error = DigestReadPort::note_admitted_request_guarded(
            &state,
            &requester,
            overflow_id,
            &overflow,
            &mut || {
                overflow_guard_calls += 1;
                Ok(())
            },
        )
        .expect_err("over-capacity request must fail closed");
        assert!(error.contains("too many admitted digest reads"));
        assert_eq!(
            overflow_guard_calls, 0,
            "capacity rejection must occur before irreversible governance"
        );
    }

    #[test]
    fn digest_reply_is_immutable_and_manifest_scoped() {
        let signer = signing_key(14);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        let peer = HostId("host-b".into());
        state
            .note_digest_request_sent(&peer, "host-a:0001", DIGEST_DAILY_SCOPE)
            .unwrap();
        let first = DigestSummary {
            frames: 1,
            halts: 2,
            conflicts: 3,
        };
        let reply = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0001".into(),
                summary: first.clone(),
            },
        );
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &reply).unwrap(),
            DigestReplyObservation::Accepted
        );
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &reply).unwrap(),
            DigestReplyObservation::Duplicate
        );
        let conflicting = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0001".into(),
                summary: DigestSummary {
                    frames: 99,
                    ..first.clone()
                },
            },
        );
        assert!(DigestReadPort::observe_reply(&state, &peer, &conflicting).is_err());
        assert_eq!(state.digest_summary(&peer, "host-a:0001"), Some(first));

        state
            .note_digest_request_sent(&peer, "host-a:0002", DIGEST_DAILY_SCOPE)
            .unwrap();
        state
            .apply_reissue(&signed_toml(2, &signer, &signer))
            .unwrap();
        let revoked = digest_frame(
            "host-b",
            "host-a",
            DigestReadControl::Reply {
                request_id: "host-a:0002".into(),
                summary: DigestSummary::default(),
            },
        );
        assert_eq!(
            DigestReadPort::observe_reply(&state, &peer, &revoked).unwrap(),
            DigestReplyObservation::Unauthorized
        );
    }
    #[test]
    fn signed_v3_cache_refuses_higher_version_v2_reissue() {
        let signer = signing_key(15);
        let pins = PinnedAuthorityKeys::from_keys(vec![signer.verifying_key()]).unwrap();
        let state = CohortManifestState::load(
            HostId("host-a".into()),
            &signed_tenant_toml(COHORT_SCHEMA_V3, 1, &signer),
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap();
        let error = state
            .apply_reissue(&signed_tenant_toml(COHORT_SCHEMA_V2, 2, &signer))
            .unwrap_err();
        assert!(matches!(
            error,
            CohortError::ECohortManifestFork {
                reason: CohortManifestForkReason::SchemaDowngrade,
                seen_version: 1,
                rejected_version: 2,
            }
        ));
    }
}
