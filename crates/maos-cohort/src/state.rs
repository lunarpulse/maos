#![forbid(unsafe_code)]

use maos_a2a_core::{CohortManifestGate, CohortReissueDisposition, CohortReissueRejection};
use maos_spirit_abi::identity::HostId;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audit::{CohortAuditEvent, CohortAuditSink};
use crate::control::CohortManifestControl;
use crate::error::{CohortError, CohortManifestForkReason};
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

    pub fn canonical_hash(&self) -> Result<[u8; 32], CohortError> {
        Ok(self
            .cached
            .lock()
            .map_err(|_| CohortError::EStatePoisoned)?
            .canonical_hash)
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

        let rejection = if candidate.version < seen_version {
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
    fn is_current(&self, peer: &HostId) -> bool {
        let Ok(cached) = self.cached.lock() else {
            return false;
        };
        self.clock
            .now_secs()
            .saturating_sub(cached.confirmed_at_secs)
            <= cached.manifest.t_stale_secs
            && cached
                .manifest
                .members
                .iter()
                .any(|member| member.host_id == self.local_host.as_str())
            && cached
                .manifest
                .members
                .iter()
                .any(|member| member.host_id == peer.as_str())
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::audit::InMemoryCohortAuditSink;
    use crate::manifest::{
        CohortAuthority, CohortMember, ConsentMatrix, ManifestSignature,
        RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE, SCHEMA_VERSION,
    };

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
            schema_version: SCHEMA_VERSION,
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
            signature: ManifestSignature { sig: String::new() },
        }
        .signed_with(signer);
        toml::to_string(&manifest).expect("serializable test manifest")
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
    fn peer_absent_from_current_manifest_is_revoked_mesh_wide() {
        let signer = signing_key(13);
        let state = state(1, &signer, Arc::new(InMemoryCohortAuditSink::default()));
        assert!(
            !CohortManifestGate::is_current(&state, &HostId("host-b".into())),
            "a peer absent from the accepted roster must be denied while the manifest is fresh"
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
}
