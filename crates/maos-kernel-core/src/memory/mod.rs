#![forbid(unsafe_code)]

//! Memory Manager — supervised service per §4.2.
//!
//! Provides three named memory tiers (`private`, `shared`, `collective`)
//! and enforces I5 namespace scopes. Story 4.3 lands the three-tier
//! mechanics with Principal Namespace (ADR-026) and Self-Telemetry (FR56).
//!
//! ## Architecture references
//! - §4.2 (Memory Manager — three tiers + I5 enforcement)
//! - §4.0.7 (kernel does NOT interpret memory contents)
//! - ADR-026 (Principal Memory Namespace, binding-v0.5)
//! - §9.2 (memory.md universal-cohort convention)

pub mod for_spirit;
pub mod principal;
pub mod private;
pub mod read_entry_point;
pub mod self_telemetry;
pub mod shared;
pub mod write_entry_point; // Story 9.4b AC-5/AC-9 — region-enforcement chokepoint // Story 9.4b R1-COND / AC-9 — region-enforcement chokepoint (read side)

pub use maos_domain::ports::MemoryManagerPort;

pub use principal::PrincipalNamespaceIndex;
pub use private::PrivateMemoryStore;
pub use self_telemetry::SelfTelemetryAggregator;
pub use shared::SharedMemoryStore;

/// Decision F — the registered set of storage backends the GDPR forget cascade
/// must account for.  This is the single source of truth shared with the
/// multi-backend erasure test: adding a backend here WITHOUT partitioning it
/// (proved-erased / proved-principal-empty) in that test FAILS the test, so a
/// new backend can never slip through unaudited.
pub const REGISTERED_ERASURE_BACKENDS: &[&str] = &["private", "principal_index", "shared"];

/// AC6 / R12 — the full set of erasure-class lineage ids the forget cascade
/// must stamp in the `ForgetReceipt`.  Mirrors the governance test
/// `erasure_class_lineage_ids` in `maos_domain::governance`.
pub const ERASURE_CLASS_LINEAGE_IDS: &[&str] = &[
    "compliance.claim.gdpr-erasure",
    "compliance.claim.legal-hold",
    "compliance.claim.retention-expiry",
];

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[cfg(feature = "spirit_test")]
use maos_spirit_sdk::spirit_test::{AttemptResult, IsolationHookPoint, ObservationResult};
#[cfg(feature = "spirit_test")]
use parking_lot::Mutex;

use crate::iac::transparency_log::TransparencyLogAdapter;
use maos_domain::cost::{CostAttributionPayload, PrincipalRef};
use maos_domain::governance::GovernanceEventPayload;
use maos_domain::memory::{
    CollectiveErrorKind, ExportEntry, ExportPayload, ForgetOutcome, ForgetReceipt, LegalHoldRecord,
    MemoryEntry, MemoryError, MemoryNamespace, MemoryTier, MemoryValue, PrincipalIndexRow,
};

// Re-exports above make PrivateMemoryStore, SharedMemoryStore,
// PrincipalNamespaceIndex available in this module scope.

#[allow(unused_imports)]
use maos_domain::invariants::i5::{MemoryScope, NamespaceKey};

/// Production Memory Manager adapter — implements `MemoryManagerPort`
/// with three-tier memory, Principal Namespace index, and I5 enforcement.
#[maos_attrs::i9_exempt(
    reason = "memory manager three-tier substrate — bounded by principal forget-cascade + per-Spirit memory budget; per-Spirit-keyed map / per-Spirit-namespaced filesystem / sqlite table for ADR-026 principal namespace + I5 isolation; parallel to the capability registry's per-Spirit token state, not pattern-learning"
)]
pub struct MemoryManagerAdapter {
    private: Arc<PrivateMemoryStore>,
    shared: Arc<SharedMemoryStore>,
    principal_index: Arc<PrincipalNamespaceIndex>,
    transparency_log: Arc<TransparencyLogAdapter>,
    next_frame_counter: AtomicU64,
    /// Story 9.4b AC-5 — configured home jurisdiction; `None` disables region
    /// pinning (legacy / default-region semantics).  Every store write routes
    /// through `write_entry_point::enforce_region` against this value.
    home_region: Option<maos_domain::region::Region>,
    /// Principal write enforcement (sec-redteam SR-1 de-stub).  When `Some`,
    /// only spirit pids in the set may write to `MemoryNamespace::Principal`
    /// namespaces — all others receive `PrincipalWriteUnauthorized`.  When
    /// `None` (v0.3-β default / test scaffolding), all pids are allowed.
    /// The daemon composition root calls `.with_principal_write_enforcement()`
    /// then `authorize_principal_writes(pid)` per admitted spirit.
    principal_write_enforcement: Option<std::sync::RwLock<HashSet<u32>>>,
    /// Story 10.4a — injected collective memory port (Loom-lite, user-space).
    /// When `Some`, collective-tier operations delegate to this port.
    /// When `None`, they return `MemoryError::CollectiveNotYetAvailable`.
    collective_port: Option<Arc<dyn maos_domain::ports::CollectiveMemoryPort>>,
    /// Story 10.4a — injected capability registry for I1 mediation of
    /// collective-tier ops.  When `Some`, the cap-gated `collective_*`
    /// methods verify+audit the Spirit's token (I2) and scope-check before
    /// the port call.  When `None`, the mediated path is unavailable.
    capabilities: Option<Arc<crate::capability::CapabilityRegistryAdapter>>,
    /// Story 4.3 — cross-Spirit isolation hook (Story 4.5 corpus).
    /// Feature-gated so production builds carry zero runtime cost.
    #[cfg(feature = "spirit_test")]
    isolation_hook: Option<Arc<Mutex<dyn IsolationHookPoint + Send>>>,
}

impl MemoryManagerAdapter {
    pub fn new(
        private: Arc<PrivateMemoryStore>,
        shared: Arc<SharedMemoryStore>,
        principal_index: Arc<PrincipalNamespaceIndex>,
        transparency_log: Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self {
            private,
            shared,
            principal_index,
            collective_port: None,
            capabilities: None,
            transparency_log,
            next_frame_counter: AtomicU64::new(0),
            home_region: None,
            principal_write_enforcement: None,
            #[cfg(feature = "spirit_test")]
            isolation_hook: None,
        }
    }

    /// Story 9.4b AC-5 — pin this memory manager to a home jurisdiction.  Set at
    /// the composition root from `RegionSection::resolve_from_env_and_disk`.
    /// When unset (default), region pinning is disabled.
    pub fn with_home_region(mut self, home_region: Option<maos_domain::region::Region>) -> Self {
        self.home_region = home_region;
        self
    }

    /// Enable principal namespace write enforcement (sec-redteam SR-1).
    /// Once enabled, only spirit pids registered via
    /// [`authorize_principal_writes`] may write to `Principal` namespaces.
    pub fn with_principal_write_enforcement(mut self) -> Self {
        self.principal_write_enforcement = Some(std::sync::RwLock::new(HashSet::new()));
        self
    }

    /// Story 10.4a — inject the collective memory port (Loom-lite adapter).
    /// Set at the daemon composition root.  When `None` (default), collective
    /// operations return `MemoryError::CollectiveNotYetAvailable`.
    pub fn with_collective_port(
        mut self,
        port: Option<Arc<dyn maos_domain::ports::CollectiveMemoryPort>>,
    ) -> Self {
        self.collective_port = port;
        self
    }

    /// Story 10.4a — inject the capability registry for I1/I2 mediation of
    /// collective-tier ops.  Set at the daemon composition root alongside
    /// [`with_collective_port`].  The cap-gated `collective_*` methods require
    /// this to enforce I1 (capability check before the port call) and I2
    /// (verify_and_audit journals the TL frame).
    pub fn with_capabilities(
        mut self,
        capabilities: Option<Arc<crate::capability::CapabilityRegistryAdapter>>,
    ) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Reject `Principal` namespace at the collective edge (Decision D).
    /// The collective tier holds cross-Spirit patterns, NOT subject-scoped
    /// PII; extending the principal_index / forget cascade into it would open
    /// a GDPR Art.15/17 hole.  Partitioned by construction.
    fn reject_principal_collective(namespace: &MemoryNamespace) -> Result<(), MemoryError> {
        if matches!(namespace, MemoryNamespace::Principal { .. }) {
            return Err(MemoryError::NamespaceViolation(
                "Principal namespace is partitioned out of the Collective tier \
                 (GDPR Art.15/17 — the collective tier holds cross-Spirit patterns, \
                 not subject-scoped PII)"
                    .into(),
            ));
        }
        Ok(())
    }

    /// Map a backing-store `CollectivePortError` into the typed
    /// `MemoryError::Collective` discriminant (P12), preserving the error
    /// category (Unreachable/Timeout/Transport/Other) instead of flattening to
    /// a bare string.  Shared by the cap-gated `collective_*` methods and the
    /// three `MemoryTier::Collective` trait arms.
    fn collective_port_error(e: maos_domain::ports::CollectivePortError) -> MemoryError {
        let kind = match &e {
            maos_domain::ports::CollectivePortError::Unreachable { .. } => {
                CollectiveErrorKind::Unreachable
            }
            maos_domain::ports::CollectivePortError::Timeout { .. } => CollectiveErrorKind::Timeout,
            maos_domain::ports::CollectivePortError::Transport(_) => CollectiveErrorKind::Transport,
            maos_domain::ports::CollectivePortError::Memory(_) => CollectiveErrorKind::Other,
        };
        MemoryError::Collective {
            kind,
            reason: e.to_string(),
        }
    }

    /// Story 10.4a — Spirit-facing collective-tier WRITE, I1/I2 mediated.
    ///
    /// I1: `verify_and_audit` checks the token (and I2: journals the
    /// `CapabilityInvocation` TL frame) BEFORE the port call.  The scope must
    /// be `LoomWrite` (high-privilege pattern write; TTL ≤ 60s per AC1).
    /// Principal namespace is rejected (Decision D).
    pub fn collective_write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
        token: &maos_domain::invariants::i1::CapabilityToken,
        posture_hash: [u8; 32],
        sandbox: maos_domain::invariants::i9::SandboxTier,
    ) -> Result<(), MemoryError> {
        Self::reject_principal_collective(namespace)?;
        let caps =
            self.capabilities
                .as_ref()
                .ok_or_else(|| MemoryError::CollectiveNotYetAvailable {
                    ship_target: "v1.5",
                    landing_story: "E10 Story 10.4",
                })?;
        let port = self.collective_port.as_ref().ok_or_else(|| {
            MemoryError::CollectiveNotYetAvailable {
                ship_target: "v1.5",
                landing_story: "E10 Story 10.4",
            }
        })?;
        caps.verify_and_audit(token, posture_hash, sandbox)
            .map_err(|e| MemoryError::Collective {
                kind: CollectiveErrorKind::CapabilityDenied,
                reason: format!("capability denied: {e}"),
            })?;
        match caps.get_token_scope(&token.token_id) {
            Some(maos_domain::invariants::i1::Scope::LoomWrite) => {}
            other => {
                return Err(MemoryError::Collective {
                    kind: CollectiveErrorKind::CapabilityDenied,
                    reason: format!("expected LoomWrite scope, got {other:?}"),
                })
            }
        }
        // P8 — LoomWrite TTL ≤ 60s enforcement (AC1: high-privilege pattern-write
        // tokens are capped at 60s).  The token carries an absolute `expiry_ns`;
        // the gap to `monotonic_now_ns()` is the live remaining TTL.  Token
        // issuance (`cap_tokens::issue`) already caps HighPrivilege TTL at 60s —
        // this is the runtime belt-and-braces re-check so a registry bug or a
        // forged/long-lived LoomWrite can never reach the port call.
        // TODO(issuance-side): assert the registry refuses to issue a LoomWrite
        // token with ttl_secs > 60 at the `issue_with_mediation` entry point.
        let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
        if token.expiry_ns > now_ns {
            let remaining_secs = (token.expiry_ns - now_ns) / 1_000_000_000;
            if remaining_secs > 60 {
                return Err(MemoryError::Collective {
                    kind: CollectiveErrorKind::CapabilityDenied,
                    reason: format!(
                        "LoomWrite token remaining TTL {remaining_secs}s exceeds the 60s cap (AC1)"
                    ),
                });
            }
        }
        port.write(spirit_pid, namespace, key, value)
            .map_err(Self::collective_port_error)
    }

    /// Story 10.4a — Spirit-facing collective-tier READ, I1/I2 mediated.
    /// Scope must be `LoomRead`.
    pub fn collective_read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        token: &maos_domain::invariants::i1::CapabilityToken,
        posture_hash: [u8; 32],
        sandbox: maos_domain::invariants::i9::SandboxTier,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        Self::reject_principal_collective(namespace)?;
        let caps =
            self.capabilities
                .as_ref()
                .ok_or_else(|| MemoryError::CollectiveNotYetAvailable {
                    ship_target: "v1.5",
                    landing_story: "E10 Story 10.4",
                })?;
        let port = self.collective_port.as_ref().ok_or_else(|| {
            MemoryError::CollectiveNotYetAvailable {
                ship_target: "v1.5",
                landing_story: "E10 Story 10.4",
            }
        })?;
        caps.verify_and_audit(token, posture_hash, sandbox)
            .map_err(|e| MemoryError::Collective {
                kind: CollectiveErrorKind::CapabilityDenied,
                reason: format!("capability denied: {e}"),
            })?;
        match caps.get_token_scope(&token.token_id) {
            Some(maos_domain::invariants::i1::Scope::LoomRead) => {}
            other => {
                return Err(MemoryError::Collective {
                    kind: CollectiveErrorKind::CapabilityDenied,
                    reason: format!("expected LoomRead scope, got {other:?}"),
                })
            }
        }
        port.read(spirit_pid, namespace, key)
            .map_err(Self::collective_port_error)
    }

    /// Story 10.4a — Spirit-facing collective-tier SCAN, I1/I2 mediated.
    /// Scope must be `LoomScan`.
    pub fn collective_scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
        token: &maos_domain::invariants::i1::CapabilityToken,
        posture_hash: [u8; 32],
        sandbox: maos_domain::invariants::i9::SandboxTier,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        Self::reject_principal_collective(namespace)?;
        let caps =
            self.capabilities
                .as_ref()
                .ok_or_else(|| MemoryError::CollectiveNotYetAvailable {
                    ship_target: "v1.5",
                    landing_story: "E10 Story 10.4",
                })?;
        let port = self.collective_port.as_ref().ok_or_else(|| {
            MemoryError::CollectiveNotYetAvailable {
                ship_target: "v1.5",
                landing_story: "E10 Story 10.4",
            }
        })?;
        caps.verify_and_audit(token, posture_hash, sandbox)
            .map_err(|e| MemoryError::Collective {
                kind: CollectiveErrorKind::CapabilityDenied,
                reason: format!("capability denied: {e}"),
            })?;
        match caps.get_token_scope(&token.token_id) {
            Some(maos_domain::invariants::i1::Scope::LoomScan) => {}
            other => {
                return Err(MemoryError::Collective {
                    kind: CollectiveErrorKind::CapabilityDenied,
                    reason: format!("expected LoomScan scope, got {other:?}"),
                })
            }
        }
        port.scan(spirit_pid, namespace, prefix, limit)
            .map_err(Self::collective_port_error)
    }

    /// Grant a spirit pid permission to write to `Principal` namespaces.
    /// No-op when enforcement is disabled (the v0.3-β / test default).
    pub fn authorize_principal_writes(&self, pid: u32) {
        if let Some(ref lock) = self.principal_write_enforcement {
            lock.write()
                .expect("principal_write_enforcement RwLock poisoned")
                .insert(pid);
        }
    }

    /// Create a pid-fused memory view for a Spirit.  This is the I5
    /// enforcement substrate — the `spirit_pid` is fused at construction
    /// and cannot be re-supplied.
    pub fn for_spirit(&self, spirit_pid: u32) -> for_spirit::SpiritMemoryView<'_> {
        for_spirit::SpiritMemoryView::new(self, spirit_pid)
    }

    /// Story 4.3 — attach an isolation hook for the `spirit_test` feature.
    /// Only available when the `spirit_test` feature is enabled.
    #[cfg(feature = "spirit_test")]
    pub fn with_isolation_hook(mut self, hook: Arc<Mutex<dyn IsolationHookPoint + Send>>) -> Self {
        self.isolation_hook = Some(hook);
        self
    }
    /// Story 9.2 — GDPR Art.17 forget cascade with optional legal-hold.
    ///
    /// * `reason` starting with `legal-hold` (case-insensitive) places a
    ///   **durable** per-principal-global hold (P29) and suspends erasure.
    /// * A principal already under a durable hold is suspended even without an
    ///   explicit reason — a second command cannot bypass a hold.
    /// * Otherwise the cascade runs and distillate bodies that reference the
    ///   principal are scrubbed + redaction-marker frames appended.
    pub fn forget_with_reason(
        &self,
        principal_id: &str,
        reason: Option<&str>,
    ) -> Result<ForgetOutcome, MemoryError> {
        // P2: case-insensitive legal-hold detection.  Both the bare form and
        // the `legal-hold:<ref>` form must match regardless of capitalization.
        let is_legal_hold = reason
            .map(|r| {
                let lower = r.trim().to_ascii_lowercase();
                lower == "legal-hold" || lower.starts_with("legal-hold:")
            })
            .unwrap_or(false);

        if is_legal_hold {
            // P29: place a DURABLE hold consulted by every later forget/uninstall.
            let requested_at_ns = Self::now_ns();
            let reason_str = reason.unwrap_or("legal-hold").to_string();
            let lower = reason_str.trim().to_ascii_lowercase();
            let case_ref = lower
                .strip_prefix("legal-hold:")
                .map(|s| s.trim().to_string());
            self.transparency_log
                .place_legal_hold(
                    principal_id,
                    &reason_str,
                    case_ref.as_deref(),
                    requested_at_ns,
                )
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            let payload = serde_json::json!({
                "principal_id": principal_id,
                "scope": "principal",
                "reason": reason_str,
                "case_ref": case_ref,
                "status": "suspended",
            });
            // P7: journal + capture frame_id atomically.
            self.transparency_log.insert_kernel_event_returning_id(
                0,
                "principal.forget.held",
                payload.to_string().as_bytes(),
            );
            let hold = LegalHoldRecord {
                principal_id: principal_id.to_string(),
                scope: "principal".to_string(),
                reason: reason_str,
                case_ref,
                requested_at_ns,
                status: "NOT ERASED — SUSPENDED UNDER LEGAL HOLD".to_string(),
            };
            return Ok(ForgetOutcome::Suspended { hold });
        }

        // P29: a prior durable hold blocks erasure even without an explicit
        // reason on THIS invocation.  This is the cross-command protection the
        // uninstall path relies on (run_uninstall_cascade calls with reason=None).
        if self
            .transparency_log
            .is_under_legal_hold(principal_id)
            .map_err(|e| MemoryError::Storage(e.to_string()))?
        {
            let requested_at_ns = Self::now_ns();
            let payload = serde_json::json!({
                "principal_id": principal_id,
                "scope": "principal",
                "status": "suspended-by-prior-hold",
            });
            self.transparency_log.insert_kernel_event_returning_id(
                0,
                "principal.forget.held",
                payload.to_string().as_bytes(),
            );
            let hold = LegalHoldRecord {
                principal_id: principal_id.to_string(),
                scope: "principal".to_string(),
                reason: "blocked by prior durable legal hold".to_string(),
                case_ref: None,
                requested_at_ns,
                status: "NOT ERASED — SUSPENDED UNDER PRIOR LEGAL HOLD".to_string(),
            };
            return Ok(ForgetOutcome::Suspended { hold });
        }

        // 1. Snapshot index rows (needed to identify affected Spirits/distillates).
        let rows = self.principal_index.lookup(principal_id)?;
        let writer_pids: HashSet<u32> = rows.iter().map(|r| r.writer_spirit_pid).collect();

        // 2. Body-scrub for distillates that REFERENCE the forgotten principal.
        //    P3: only distillates whose body embeds the principal_id are
        //    scrubbed — not every distillate by any writer Spirit (which would
        //    destroy unrelated principals' data).
        //    P5: a query error is propagated, not silently dropped.
        //    P4: a scrub/marker failure is propagated and the frame is NOT
        //    attested as redacted.
        let mut redacted_distillate_frame_ids: Vec<String> = Vec::new();
        if !writer_pids.is_empty() {
            let distillates = self
                .transparency_log
                .distillate_frames_for_pids(&writer_pids)
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            let needle = principal_id.as_bytes();
            for (frame_id, body) in distillates {
                if !body.windows(needle.len()).any(|w| w == needle) {
                    continue;
                }
                self.transparency_log
                    .scrub_distillate_body(frame_id, "gdpr-forget")
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                let _ = self
                    .transparency_log
                    .insert_distillate_redaction_marker(principal_id, frame_id);
                redacted_distillate_frame_ids.push(hex::encode(frame_id));
            }
        }

        // SR-3 (Story 9.3b) — scrub cost/governance frames that embed the
        // forgotten principal. Unlike distillates (content-based body scan),
        // these frames carry principal_id as a structured JSON field.
        let mut redacted_principal_frame_ids: Vec<String> = Vec::new();
        if !writer_pids.is_empty() && !principal_id.is_empty() {
            let principal_frames = self
                .transparency_log
                .principal_bearing_frames_for_pids(&writer_pids)
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            for (frame_id, body) in principal_frames {
                // Structured match: parse as typed cost/governance payloads
                // instead of fragile byte-substring scan (SR-3 review fix).
                let frame_matches = if let Ok(cost) =
                    serde_json::from_slice::<CostAttributionPayload>(&body)
                {
                    match &cost.principal {
                        PrincipalRef::Resolved { principal_id: pid } => pid == principal_id,
                        _ => false,
                    }
                } else if let Ok(_gov) = serde_json::from_slice::<GovernanceEventPayload>(&body) {
                    // Governance frames do not carry a principal_id field;
                    // they are never principal-bearing for forget purposes.
                    false
                } else {
                    // Unknown frame schema — fall back to byte scan so
                    // future frame kinds are not silently skipped.
                    body.windows(principal_id.len())
                        .any(|w| w == principal_id.as_bytes())
                };
                if !frame_matches {
                    continue;
                }
                self.transparency_log
                    .scrub_principal_bearing_frame(frame_id, "gdpr-forget")
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                redacted_principal_frame_ids.push(hex::encode(frame_id));
            }
        }

        // 3. Delete private-tier entries + principal_index rows (existing cascade).
        let deleted_entries = self.private.forget_principal(principal_id)?;
        let deleted_index_rows = self.principal_index.forget(principal_id)?;

        // 4. Journal the cascade with the redaction set and reason.  P7: the
        //    frame_id is captured atomically with the insert.
        let payload = serde_json::json!({
            "principal_id": principal_id,
            "deleted_entries": deleted_entries,
            "deleted_index_rows": deleted_index_rows,
            "redacted_distillate_frame_ids": redacted_distillate_frame_ids,
            "redacted_principal_frame_ids": redacted_principal_frame_ids,
            "reason": reason,
        });
        let frame_id = self.transparency_log.insert_kernel_event_returning_id(
            0,
            "principal.forget",
            payload.to_string().as_bytes(),
        );
        let timestamp_ns = Self::now_ns();
        let mut receipt = ForgetReceipt::new(
            principal_id,
            deleted_entries,
            deleted_index_rows,
            timestamp_ns,
            frame_id,
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;

        // Story 9.3b (AC6 / R12) — stamp the schema version in force at
        // erasure-execution time, read from the R10 schema-lifecycle registry.
        // Look up the full erasure-class set (R12), not just gdpr-erasure.
        for lineage_id in ERASURE_CLASS_LINEAGE_IDS {
            if let Ok(Some(entry)) = self.transparency_log.current_schema_version(lineage_id) {
                receipt.schema_id = Some(entry.schema_id);
                receipt.schema_version = Some(entry.version);
                break;
            }
        }
        Ok(ForgetOutcome::Erased {
            receipt,
            redacted_distillate_frame_ids,
            redacted_principal_frame_ids,
        })
    }

    /// Story 9.2 (P29) — release a durable legal hold so the principal may be
    /// erased again.  Returns whether a hold was actually removed.
    pub fn release_legal_hold(&self, principal_id: &str) -> Result<bool, MemoryError> {
        self.transparency_log
            .release_legal_hold(principal_id)
            .map_err(|e| MemoryError::Storage(e.to_string()))
    }

    #[cfg(feature = "spirit_test")]
    fn fire_isolation_hooks(&self, case_id: &str, attempt_ok: bool) {
        if let Some(hook) = &self.isolation_hook {
            let mut h = hook.lock();
            let _ = h.before_spirit_a_attempt(case_id);
            let attempt = AttemptResult {
                hooks_fired_during_attempt: vec![case_id.into()],
                frames_emitted: if attempt_ok { 1 } else { 0 },
            };
            let _ = h.after_spirit_a_attempt(case_id, &attempt);
            let _ = h.before_spirit_b_observe(case_id);
            let observation = ObservationResult {
                hooks_fired_during_observation: vec![case_id.into()],
                frames_emitted: 0,
                leaked_bytes: None,
            };
            let _ = h.after_spirit_b_observe(case_id, &observation);
        }
    }

    fn now_ns() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    fn mint_frame_id(&self) -> [u8; 16] {
        // Use the same monotonic-ULID mechanism as the Transparency Log:
        // increment a process-local counter and embed it in the low bits
        // of a ULID-based byte array.  This preserves uniqueness within
        // the process and monotonicity across calls.
        let counter = self.next_frame_counter.fetch_add(1, Ordering::Relaxed);
        let ulid = ulid::Ulid::new();
        let mut bytes = ulid.to_bytes();
        let cb = counter.to_le_bytes();
        // Overwrite the low 4 bytes (counter) — this is a deliberate,
        // documented trade-off: we sacrifice raw ULID randomness for
        // guaranteed process-local monotonicity.  The high 10 bytes
        // retain the ULID timestamp + randomness entropy.
        bytes[12] = cb[0];
        bytes[13] = cb[1];
        bytes[14] = cb[2];
        bytes[15] = cb[3];
        bytes
    }
}

impl MemoryManagerPort for MemoryManagerAdapter {
    fn validate_namespace_read(&self, key: &NamespaceKey<MemoryScope>) -> bool {
        // v0.3-β: all namespace reads are permitted at the port level;
        // real enforcement lives in the `for_spirit` reborrow + per-pid keying.
        let _ = key;
        true
    }

    fn validate_namespace_write(&self, key: &NamespaceKey<MemoryScope>) -> bool {
        let k = key.as_str();
        !k.is_empty() && !k.contains('\0')
    }

    fn write(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError> {
        #[cfg(feature = "spirit_test")]
        let case_id = format!(
            "memory.write:{spirit_pid}:{:?}:{key}",
            namespace.kind_label()
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&case_id, true);

        // AC-5/AC-9: every store write routes through the single region
        // chokepoint.  DirectWrite is home-by-construction (passes when pinning
        // matches or is disabled); future replay/restore paths MUST construct a
        // `WriteEntryPoint` carrying source-region provenance and will fail
        // closed here on a cross-region write.
        write_entry_point::enforce_region(
            &write_entry_point::WriteEntryPoint::DirectWrite,
            self.home_region.as_ref(),
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;

        // Sec-redteam SR-1: principal namespace writes require explicit
        // authorization when enforcement is enabled.
        if let MemoryNamespace::Principal { .. } = namespace {
            if let Some(ref lock) = self.principal_write_enforcement {
                let authorized = lock
                    .read()
                    .expect("principal_write_enforcement RwLock poisoned");
                if !authorized.contains(&spirit_pid) {
                    return Err(MemoryError::PrincipalWriteUnauthorized { spirit_pid });
                }
            }
        }

        match tier {
            MemoryTier::Private => {
                self.private
                    .write(spirit_pid, namespace, key, value.clone())?;
                // If this is a principal-tagged write, record in the index.
                if let MemoryNamespace::Principal {
                    principal_id,
                    schema,
                } = namespace
                {
                    let ts = Self::now_ns();
                    self.principal_index
                        .record_write(principal_id, spirit_pid, schema, key, ts)?;
                }
                Ok(())
            }
            MemoryTier::Shared => self.shared.write(spirit_pid, namespace, key, value),
            MemoryTier::Collective => {
                Self::reject_principal_collective(namespace)?;
                // P1 — I1/I2 fail-closed.  The trait path carries NO token /
                // posture_hash / sandbox (the `MemoryManagerPort` ABI cannot
                // change), so it CANNOT perform capability mediation.  When
                // I1/I2 mediation is wired (`self.capabilities` is `Some`, i.e.
                // the production composition root), the unmediated path is
                // DENIED — callers MUST use the cap-gated `collective_write`
                // (which verifies+audits the LoomWrite token before the port
                // call).  Only the legacy/test path (no capabilities injected)
                // falls through to the direct port delegation.
                if self.capabilities.is_some() {
                    return Err(MemoryError::Collective {
                        kind: CollectiveErrorKind::CapabilityDenied,
                        reason:
                            "unmediated collective write via the trait path denied while I1/I2 \
                                 capability mediation is wired; use the cap-gated collective_write \
                                 (carries the LoomWrite token + posture)"
                                .into(),
                    });
                }
                match &self.collective_port {
                    Some(port) => port
                        .write(spirit_pid, namespace, key, value)
                        .map_err(Self::collective_port_error),
                    None => Err(MemoryError::CollectiveNotYetAvailable {
                        ship_target: "v1.5",
                        landing_story: "E10 Story 10.4",
                    }),
                }
            }
        }
    }

    fn read(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        #[cfg(feature = "spirit_test")]
        let case_id = format!(
            "memory.read:{spirit_pid}:{:?}:{key}",
            namespace.kind_label()
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&case_id, true);

        // R1-COND / AC-9: every store read routes through the single region
        // chokepoint.  DirectRead is home-by-construction.
        read_entry_point::enforce_region(
            &read_entry_point::ReadEntryPoint::DirectRead,
            self.home_region.as_ref(),
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;

        match tier {
            MemoryTier::Private => self.private.read(spirit_pid, namespace, key),
            MemoryTier::Shared => self.shared.read(spirit_pid, namespace, key),
            MemoryTier::Collective => {
                Self::reject_principal_collective(namespace)?;
                // P1 — I1/I2 fail-closed (see the write arm for the rationale):
                // the trait path cannot carry the LoomRead token, so when
                // capability mediation is wired the unmediated read is DENIED.
                if self.capabilities.is_some() {
                    return Err(MemoryError::Collective {
                        kind: CollectiveErrorKind::CapabilityDenied,
                        reason: "unmediated collective read via the trait path denied while I1/I2 \
                                 capability mediation is wired; use the cap-gated collective_read \
                                 (carries the LoomRead token + posture)"
                            .into(),
                    });
                }
                match &self.collective_port {
                    Some(port) => port
                        .read(spirit_pid, namespace, key)
                        .map_err(Self::collective_port_error),
                    None => Err(MemoryError::CollectiveNotYetAvailable {
                        ship_target: "v1.5",
                        landing_story: "E10 Story 10.4",
                    }),
                }
            }
        }
    }

    fn scan(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        #[cfg(feature = "spirit_test")]
        let case_id = format!(
            "memory.scan:{spirit_pid}:{:?}:{prefix}",
            namespace.kind_label()
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&case_id, true);

        // R1-COND / AC-9: every store scan routes through the single region
        // chokepoint.  DirectRead is home-by-construction.
        read_entry_point::enforce_region(
            &read_entry_point::ReadEntryPoint::DirectRead,
            self.home_region.as_ref(),
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;

        match tier {
            MemoryTier::Private => self.private.scan(spirit_pid, namespace, prefix, limit),
            MemoryTier::Shared => self.shared.scan(spirit_pid, namespace, prefix, limit),
            MemoryTier::Collective => {
                Self::reject_principal_collective(namespace)?;
                // P1 — I1/I2 fail-closed (see the write arm for the rationale):
                // the trait path cannot carry the LoomScan token, so when
                // capability mediation is wired the unmediated scan is DENIED.
                if self.capabilities.is_some() {
                    return Err(MemoryError::Collective {
                        kind: CollectiveErrorKind::CapabilityDenied,
                        reason: "unmediated collective scan via the trait path denied while I1/I2 \
                                 capability mediation is wired; use the cap-gated collective_scan \
                                 (carries the LoomScan token + posture)"
                            .into(),
                    });
                }
                match &self.collective_port {
                    Some(port) => port
                        .scan(spirit_pid, namespace, prefix, limit)
                        .map_err(Self::collective_port_error),
                    None => Err(MemoryError::CollectiveNotYetAvailable {
                        ship_target: "v1.5",
                        landing_story: "E10 Story 10.4",
                    }),
                }
            }
        }
    }

    fn subject_access(&self, principal_id: &str) -> Result<Vec<PrincipalIndexRow>, MemoryError> {
        // R1-COND / AC-9: subject-access lookup routes through the read
        // chokepoint.  DirectRead is home-by-construction.
        read_entry_point::enforce_region(
            &read_entry_point::ReadEntryPoint::DirectRead,
            self.home_region.as_ref(),
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;
        self.principal_index.lookup(principal_id)
    }

    fn forget(&self, principal_id: &str) -> Result<ForgetReceipt, MemoryError> {
        match self.forget_with_reason(principal_id, None)? {
            ForgetOutcome::Erased { receipt, .. } => Ok(receipt),
            ForgetOutcome::Suspended { .. } => {
                // The trait-level `forget` is the legacy no-reason path;
                // a legal-hold is impossible without an explicit reason.
                Err(MemoryError::Storage(
                    "forget unexpectedly returned legal-hold without reason".to_string(),
                ))
            }
        }
    }

    fn export_redactable(
        &self,
        principal_id: &str,
        include_principal: bool,
    ) -> Result<Vec<ExportEntry>, MemoryError> {
        let rows = self.principal_index.lookup(principal_id)?;
        let mut entries = Vec::with_capacity(rows.len());

        for row in &rows {
            let ns = MemoryNamespace::principal(&row.principal_id, &row.schema)
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            let val_opt = self.private.read(row.writer_spirit_pid, &ns, &row.key)?;

            let payload = if include_principal {
                match val_opt {
                    Some(v) => ExportPayload::Raw(v),
                    None => ExportPayload::Redacted {
                        content_type: "unknown".into(),
                        principal_id: row.principal_id.clone(),
                        schema: row.schema.clone(),
                    },
                }
            } else {
                ExportPayload::Redacted {
                    content_type: "principal-namespace".into(),
                    principal_id: row.principal_id.clone(),
                    schema: row.schema.clone(),
                }
            };

            entries.push(ExportEntry {
                namespace: ns,
                key: row.key.clone(),
                payload,
            });
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_adapter() -> (Arc<MemoryManagerAdapter>, TempDir) {
        let tmp = TempDir::new().unwrap();
        let memory_root = tmp.path().join("memory");
        let db_path = tmp.path().join("audit.db");

        let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
        let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
        let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
        // Unique boot-nonce per test fixture to avoid in-memory SQLite collision.
        static TEST_NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let boot_nonce = TEST_NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(boot_nonce));
        let adapter = Arc::new(MemoryManagerAdapter::new(
            private,
            shared,
            principal_index,
            tl,
        ));
        (adapter, tmp)
    }

    #[test]
    fn write_read_private() {
        let (adapter, _tmp) = make_adapter();
        let val = MemoryValue::Text("hello".into());
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "k",
                val.clone(),
            )
            .unwrap();
        let got = adapter
            .read(1, MemoryTier::Private, &MemoryNamespace::Default, "k")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn write_read_shared() {
        let (adapter, _tmp) = make_adapter();
        let val = MemoryValue::Text("shared!".into());
        adapter
            .write(
                2,
                MemoryTier::Shared,
                &MemoryNamespace::Coordination,
                "s",
                val.clone(),
            )
            .unwrap();
        let got = adapter
            .read(2, MemoryTier::Shared, &MemoryNamespace::Coordination, "s")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn collective_returns_typed_error() {
        let (adapter, _tmp) = make_adapter();
        let err = adapter
            .write(
                1,
                MemoryTier::Collective,
                &MemoryNamespace::Default,
                "k",
                MemoryValue::Text("x".into()),
            )
            .unwrap_err();
        match err {
            MemoryError::CollectiveNotYetAvailable {
                ship_target,
                landing_story,
            } => {
                assert_eq!(ship_target, "v1.5");
                assert_eq!(landing_story, "E10 Story 10.4");
            }
            _ => panic!("expected CollectiveNotYetAvailable"),
        }
    }

    #[test]
    fn scan_private() {
        let (adapter, _tmp) = make_adapter();
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "a-1",
                MemoryValue::Text("a".into()),
            )
            .unwrap();
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "a-2",
                MemoryValue::Text("b".into()),
            )
            .unwrap();
        let results = adapter
            .scan(1, MemoryTier::Private, &MemoryNamespace::Default, "a-", 10)
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn spirit_view_fuses_pid() {
        let (adapter, _tmp) = make_adapter();
        let view = adapter.for_spirit(7);
        let val = MemoryValue::Text("pid-7".into());
        view.write(
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "k",
            val.clone(),
        )
        .unwrap();
        let got = view
            .read(MemoryTier::Private, &MemoryNamespace::Default, "k")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn i5_isolation_different_pids_dont_overlap() {
        let (adapter, _tmp) = make_adapter();
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "secret",
                MemoryValue::Text("spirit-1".into()),
            )
            .unwrap();
        // Spirit 2 reads Spirit 1's key — should return None.
        let got = adapter
            .read(2, MemoryTier::Private, &MemoryNamespace::Default, "secret")
            .unwrap();
        assert!(got.is_none());
    }
}
