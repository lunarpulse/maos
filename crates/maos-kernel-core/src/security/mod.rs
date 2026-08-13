#![deny(unsafe_code)]

//! Security Manager — supervised service per §4.3.
//!
//! Enforces sandbox tiers, secret isolation, and approval-class
//! mediation. Story 1b.3 lands T0/T1/T2 tier enforcement and
//! per-Spirit resource caps.
//!
//! Story 2.1 adds capability-declaration → policy-table wiring
//! (replacing the hardcoded injection in `maos-bin::main`),
//! `OutputShapePredicate` scaffold, and a `DriftEvent` channel.

pub mod approval;
pub mod crypto;
pub mod drift;
pub mod manifest;
pub mod operator_config;
pub mod posture;
pub mod sandbox;

pub use crypto::RingCryptoProvider;
pub use manifest::{resolve_caps, ManifestError, ResolvedCaps, ResourceCaps, SandboxConfig};
pub use maos_domain::ports::SecurityManagerPort;
// Story 1b.5c — appended to preserve original re-export order so the
// signature_hash of each existing symbol remains stable under
// `check-service-boundary`'s use-item hashing (the gate hashes the
// whole `pub use` token-tree per member; reordering would falsely
// flag every existing symbol as removed-and-re-added).
pub use manifest::{
    Author, Budget, CapabilitiesRequired, ClassSection, OutputShape, Posture, PostureSection,
    ProviderCapabilities,
};
// Story 2.1 — appended to preserve re-export order (same discipline).
pub use manifest::{capabilities_required_to_scopes, OutputShapePredicate, OutputShapeViolation};
// Story 2.3 — appended for P2 port-pair completeness (RingCryptoProvider adapter → CryptoProvider Port).
pub use drift::{make_drift_channel, DriftEvent};
pub use maos_domain::ports::CryptoProvider;
pub use sandbox::{
    classify_exit, spawn_sandboxed, SandboxSpec, SandboxViolation, SandboxedChild, SpawnError,
};
// Story 3.2 — appended to preserve re-export order.
pub use manifest::{EpistemicAction, EpistemicPolicyRule, EpistemicPolicySection, ScalarPredicate};
// Story 5.1 — appended to preserve re-export order.
pub use manifest::{LifecycleSection, SchedulingSection};
// Story 5.3 — appended to preserve re-export order.
pub use manifest::{OnCrashSection, SupervisionSection};
// Story 5.5b — appended to preserve re-export order.
pub use manifest::{ProviderConfig, ProvidersSection};
// Story 5.5c — appended to preserve re-export order.
pub use manifest::{McpCapabilities, McpCapabilityServerEntry, McpSection, McpServerEntry};
pub use posture::{PostureError, PostureState};

use std::sync::Arc;

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::scheduler::SpiritSchedulerPort;
use tokio::sync::mpsc;

use crate::capability::cap_audit::{self, CapAuditEvent};
use crate::capability::cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable};

/// Security error raised during admission or enforcement.
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("sandbox tier unsupported at this version: requested {0}")]
    SandboxTierUnsupported(SandboxTier),
    #[error("sandbox unavailable on platform: {reason}")]
    SandboxUnavailable { reason: String },
    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
    #[error("policy table lookup failed for spirit {spirit_pid}")]
    PolicyLookup { spirit_pid: u32 },
    #[error("T3 admission failed: {0}")]
    T3AdmissionFailed(String),
    // ----- Story 7.5a — ABI Stability Triple enforcement (additive; kernel-internal,
    // NOT an ABI surface). The `E`-prefix follows the `CliWrapperAdmissionError`
    // taxonomy. Raised at the TOP of `admit_spirit`, before any policy-table
    // mutation — fail-loud before half-admitted state. -----
    /// FR8 — the running kernel is older than the Spirit's declared
    /// `min_substrate_version`. Raised whenever the kernel version
    /// (`env!("CARGO_PKG_VERSION")`) does NOT satisfy `>={declared_min}`, OR the
    /// declared minimum is unparseable (fail-loud, never a silent admit).
    #[error("substrate too old: spirit '{spirit_id}' requires kernel >= {declared_min}, running kernel is {kernel_version}")]
    ESubstrateTooOld {
        spirit_id: String,
        declared_min: String,
        kernel_version: String,
    },
    /// N-2 manifest schema — `manifest_schema_version < MIN_SUPPORTED`. Promotes
    /// the existing stringly below-window rejection (`manifest.rs`) to a typed
    /// error at the admission chokepoint (`ManifestError` stays frozen).
    #[error("manifest schema too old: declared schema {declared_schema} < min supported {min_supported} (N-2 hard refusal)")]
    EAbiTooOld {
        declared_schema: u32,
        min_supported: u32,
    },
    /// Forward/future manifest schema — `manifest_schema_version > MAX_SUPPORTED`.
    /// Fail-closed per the §LOCKED Design Decision (NO warn-and-ignore window):
    /// a future Spirit is told it needs a newer kernel, never silently admitted.
    #[error("manifest schema too new: declared schema {declared_schema} > max supported {max_supported} (kernel upgrade required)")]
    EAbiTooNew {
        declared_schema: u32,
        max_supported: u32,
    },
    /// Belt-and-suspenders gate: `admit_spirit` received `class: None`. The
    /// `[class]` section is required at parse time (`load_bundle_from_file`)
    /// AND at admission — a Spirit reaching admission without a class section
    /// indicates a malformed manifest or a code path circumventing the parser.
    #[error("spirit '{spirit_id}' admitted without required [class] section — manifest is malformed or admission path bypassed parser")]
    EClassRequired { spirit_id: String },
}

/// Adapter — implements `SecurityManagerPort` with sandbox tier
/// enforcement and approval mediation.
///
/// Promoted from ZST (v0.1-α) to hold `Arc<PolicyTable>` (Story 1b.3).
/// Story 2.1 adds an optional drift-event sender.
#[maos_attrs::i9_exempt(
    reason = "security manager adapter; holds Arc<PolicyTable> for runtime policy enforcement — structural-state caching per I9"
)]
#[derive(Debug, Clone)]
struct T3ImageVerificationConfig {
    lock_path: std::path::PathBuf,
    trust_anchor_pub: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct SecurityManagerAdapter {
    policy: Arc<PolicyTable>,
    t3_image_verification: Option<T3ImageVerificationConfig>,
    /// Drift-event channel sender (Story 2.1 AC4).
    /// The runtime detector that emits events ships in Story 9.x.
    drift_sender: Option<mpsc::Sender<DriftEvent>>,
    /// Story 5.5b — tracks last provider per spirit_id for ProviderSwitched
    /// emission.  Story 9.4b AC-8 — now **bounded** (see [`ProviderHistory`]).
    provider_history: Arc<std::sync::Mutex<ProviderHistory>>,
}

/// Story 9.4b AC-8 — bounded last-provider tracker for ProviderSwitched
/// emission.
///
/// Previously an unbounded `HashMap<String, String>` (`deferred-work.md:193`):
/// under high Spirit churn it grew without limit because nothing evicted
/// terminated Spirits.  Bounded at [`ProviderHistory::CAP`] entries with an
/// **evict-oldest-by-first-insertion** overflow policy — the latest provider is
/// always tracked (never *reject-new*).  Evicting a stale Spirit's entry only
/// means a later re-admission is treated as first-seen (no false
/// ProviderSwitched), which is benign: this is ephemeral switch-detection state
/// that is never serialized into a replayed artifact.
#[maos_attrs::i9_exempt(
    reason = "bounded last-provider tracker for ProviderSwitched emission (Story 9.4b AC-8); ephemeral switch-detection state keyed by spirit_id, capped at CAP=4096 with evict-oldest-by-first-insertion, never serialized into a replayed artifact; held inside Arc<Mutex<ProviderHistory>> in SecurityManagerAdapter"
)]
#[derive(Debug, Default)]
struct ProviderHistory {
    map: std::collections::HashMap<String, String>,
    order: std::collections::VecDeque<String>,
}

impl ProviderHistory {
    /// Soft cap on tracked Spirits — far above any realistic concurrent count.
    const CAP: usize = 4096;

    /// Insert/update the last provider for `spirit_id`, returning the previous
    /// value (so the caller can detect a switch).  Enforces the cap with
    /// evict-oldest on overflow.
    ///
    /// **Invariant**: `order` and `map` stay synchronised — every key in `map`
    /// appears exactly once in `order`, and vice-versa.  On an *update* (key
    /// already tracked) the old position is removed and the key is pushed to the
    /// back so it becomes the newest entry for eviction purposes.
    fn insert(&mut self, spirit_id: String, provider: String) -> Option<String> {
        let prev = self.map.insert(spirit_id.clone(), provider);
        if prev.is_some() {
            // Update case: remove the stale position so the key appears only
            // once in the deque (at the back after the push below).
            if let Some(pos) = self.order.iter().position(|k| k == &spirit_id) {
                self.order.remove(pos);
            }
        }
        self.order.push_back(spirit_id);
        while self.map.len() > Self::CAP {
            match self.order.pop_front() {
                Some(oldest) => {
                    self.map.remove(&oldest);
                }
                None => break,
            }
        }
        prev
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }
}

impl SecurityManagerAdapter {
    pub fn new(policy: Arc<PolicyTable>) -> Self {
        Self {
            policy,
            t3_image_verification: None,
            drift_sender: None,
            provider_history: Arc::new(std::sync::Mutex::new(ProviderHistory::default())),
        }
    }

    /// Set the drift-event sender (Story 2.1 AC4).
    ///
    /// The sender is consumed by the runtime drift detector at Story 9.x.
    pub fn with_drift_sender(mut self, sender: mpsc::Sender<DriftEvent>) -> Self {
        self.drift_sender = Some(sender);
        self
    }

    /// Inject an explicit verified-image lock boundary.
    ///
    /// Daemon defaults continue to read the operator-provided path and trust
    /// anchor from environment variables. Embedded composition roots and tests
    /// use this builder to avoid process-global state.
    pub fn with_t3_image_verification(
        mut self,
        lock_path: impl Into<std::path::PathBuf>,
        trust_anchor_pub: [u8; 32],
    ) -> Self {
        self.t3_image_verification = Some(T3ImageVerificationConfig {
            lock_path: lock_path.into(),
            trust_anchor_pub,
        });
        self
    }

    /// Access the underlying policy table (for tests and composition root).
    pub fn policy(&self) -> &Arc<PolicyTable> {
        &self.policy
    }

    /// Admit a Spirit: compute effective tier, reject T3+, journal Load,
    /// wire manifest-scoped capabilities into the policy table.
    ///
    /// Story 2.1: added `caps_required` parameter for capability-declaration
    /// → policy-table wiring (replaces hardcoded injection in `maos-bin`).
    pub fn admit_spirit(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        manifest: &SandboxConfig,
        caps: &ResourceCaps,
        caps_required: &CapabilitiesRequired,
        output_shape: Option<&OutputShape>,
        journal: &dyn SpiritSchedulerPort,
        posture_section: &PostureSection,
        epistemic_policy: Option<&EpistemicPolicySection>,
        _scheduling: Option<&SchedulingSection>,
        _lifecycle: Option<&LifecycleSection>,
        _on_crash: Option<&OnCrashSection>,
        _supervision: Option<&SupervisionSection>,
        providers: Option<&ProvidersSection>,
        // Story 7.5a — the `[class]` section carries the ABI Stability Triple
        // legs enforced at load: `min_substrate_version` (vs running kernel)
        // and `manifest_schema_version` (vs the kernel's supported window).
        // Required at parse time AND admission — None is rejected with
        // EClassRequired (no silent bypass).
        class: Option<&ClassSection>,
    ) -> Result<SandboxSpec, SecurityError> {
        // ---- Story 7.5a (AC2) — ABI Stability Triple enforcement.
        // Runs at the TOP of admit, BEFORE any policy-table mutation below, so a
        // rejected Spirit leaves NO half-admitted state. Reuses the hand-rolled
        // `maos_domain::revocation::semver_range_contains` comparator (no `semver`
        // crate dep) and the `maos-spirit-abi` schema-window constants.
        //
        // Belt-and-suspenders: `[class]` is required at parse time
        // (`load_bundle_from_file`), so production paths should never reach here
        // with None. If they do, it's a malformed manifest or a code path
        // circumventing the parser — reject loudly, never silently admit.
        let class = class.ok_or_else(|| SecurityError::EClassRequired {
            spirit_id: spirit_id.into(),
        })?;

        // Leg 1 — kernel_version vs declared min_substrate_version (FR8).
        // env!("CARGO_PKG_VERSION") inside kernel-core IS the kernel leg
        // (all crates share [workspace.package].version).
        let kernel_version = env!("CARGO_PKG_VERSION");
        let declared_min = class.min_substrate_version.clone();
        // Fail-loud on BOTH "too old" (Ok(false)) and "unparseable" (Err):
        // a version we cannot prove compatible is refused, never silently
        // admitted. No `unwrap_or_default()` — a silent default here would
        // admit a too-old kernel (a correctness/security bug).
        let satisfies_min = maos_domain::revocation::semver_range_contains(
            kernel_version,
            &format!(">={declared_min}"),
        )
        .map_err(|_| SecurityError::ESubstrateTooOld {
            spirit_id: spirit_id.into(),
            declared_min: declared_min.clone(),
            kernel_version: kernel_version.into(),
        })?;
        if !satisfies_min {
            return Err(SecurityError::ESubstrateTooOld {
                spirit_id: spirit_id.into(),
                declared_min,
                kernel_version: kernel_version.into(),
            });
        }

        // Leg 2 — manifest_schema_version vs the kernel's supported window.
        // Fail-closed in BOTH directions per the §LOCKED Design Decision:
        // below MIN → EAbiTooOld (N-2 hard refusal); above MAX → EAbiTooNew
        // (forward case; NO warn-and-ignore window). The manifest parser
        // keeps its shape-only window check as defense-in-depth.
        let declared_schema = class.manifest_schema_version;
        let min_supported = maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;
        let max_supported = maos_spirit_abi::MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION;
        if declared_schema < min_supported {
            return Err(SecurityError::EAbiTooOld {
                declared_schema,
                min_supported,
            });
        }
        if declared_schema > max_supported {
            return Err(SecurityError::EAbiTooNew {
                declared_schema,
                max_supported,
            });
        }
        // N-1 (declared < CURRENT but within window) — admit with documented
        // WARN-level degradation paths (NFR-Maint-9). The WARN emission lives
        // in maos-manifest so the same code path is exercised by the
        // manifest N-1 field-coverage test (AC4).
        let _degraded = maos_manifest::warn_n_minus_1_degradations(declared_schema);

        {
            let declared_scopes = capabilities_required_to_scopes(caps_required);
            let inner = self.policy.inner().load_full();

            let trust_tier = inner
                .manifest_scopes
                .get(&spirit_pid)
                .map(|m| m.trust_tier)
                .unwrap_or(TrustTier::Verified);

            let effective = self
                .policy
                .effective_sandbox_tier(spirit_pid, trust_tier, &inner);

            let mut new_inner = (*inner).clone();
            new_inner.manifest_scopes.insert(
                spirit_pid,
                ManifestCapabilityScope {
                    scopes: declared_scopes,
                    declared_tier: effective,
                    trust_tier,
                },
            );
            new_inner.spirit_postures.insert(
                spirit_pid,
                crate::security::posture::PostureState {
                    current: posture_section.default,
                    allowed_max: posture_section.allowed_max,
                    epistemic_policy: epistemic_policy
                        .cloned()
                        .unwrap_or_else(EpistemicPolicySection::default_open_fail),
                },
            );
            self.policy.update(new_inner);
        }

        let inner = self.policy.inner().load_full();

        let trust_tier = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.trust_tier)
            .unwrap_or(TrustTier::Verified);

        let effective = self
            .policy
            .effective_sandbox_tier(spirit_pid, trust_tier, &inner);

        // Story 5.5a: T3 is now admissible on Linux with container isolation.
        // T4 remains rejected (WASM tier scheduled for v2.0).
        if effective == SandboxTier::T3 {
            let crypto = crate::security::crypto::RingCryptoProvider;
            let lock = match &self.t3_image_verification {
                Some(config) => crate::security::sandbox::t3::image_lock::load_and_verify_lock_at(
                    &config.lock_path,
                    &config.trust_anchor_pub,
                    &crypto,
                ),
                None => {
                    let trust_anchor =
                        crate::security::sandbox::t3::image_verify::read_trust_anchor_pub()
                            .map_err(|error| SecurityError::T3AdmissionFailed(error.to_string()))?;
                    crate::security::sandbox::t3::image_lock::load_and_verify_lock(
                        &trust_anchor,
                        &crypto,
                    )
                }
            }
            .map_err(|error| SecurityError::T3AdmissionFailed(error.to_string()))?;
            if let Some(image_pin) = &manifest.image_pin {
                lock.resolve_pin(image_pin)
                    .map_err(|error| SecurityError::T3AdmissionFailed(error.to_string()))?;
            } else {
                lock.default_entry()
                    .map_err(|error| SecurityError::T3AdmissionFailed(error.to_string()))?;
            }
        } else if effective.0 > SandboxTier::T3.0 {
            return Err(SecurityError::SandboxTierUnsupported(effective));
        }

        // Fail-closed: T1 enforcement not yet implemented.
        if effective == SandboxTier::T1 {
            return Err(SecurityError::SandboxTierUnsupported(effective));
        }

        // Resolve resource caps (2-way strictest-of manifest vs operator).
        let operator_caps = inner
            .operator_policy
            .resource_cap_floor
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let resolved_caps = resolve_caps(caps, &operator_caps);

        let declared_scopes = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.scopes.clone())
            .unwrap_or_default();

        // Build OutputShapePredicate from manifest (Story 2.1 AC3).
        let predicate = output_shape.map(OutputShapePredicate::from);

        // Journal the Load transition with effective tier (monotonic clock).
        journal.journal_lifecycle(JournalEntry::Lifecycle(LifecycleEntry {
            timestamp: crate::capability::cap_tokens::monotonic_now_ns(),
            lifecycle_event: LifecycleEvent::Load,
            spirit_id: spirit_id.into(),
            payload: None,
            effective_sandbox_tier: Some(effective),
        }));

        // Backfill review (2026-05-25): scope the mutex narrowly. The earlier
        // shape held the lock across `serde_json::to_vec` AND
        // `journal.journal_lifecycle` — the journal call can perform I/O
        // (fsync on the lifecycle journal), and holding a global mutex across
        // I/O serializes every admission. Capture the from/to diff under the
        // lock, swap the value, drop the lock, then serialize + journal.
        if let Some(providers_section) = providers {
            let new_provider = providers_section.primary.id.clone();
            let from_provider_opt = {
                let mut history = self.provider_history.lock().unwrap();
                let prev = history.insert(spirit_id.into(), new_provider.clone());
                match prev {
                    Some(prev) if prev != new_provider => Some(prev),
                    _ => None,
                }
            };
            if let Some(from_provider) = from_provider_opt {
                let payload = maos_domain::invariants::i10::ProviderSwitchedPayload::new(
                    spirit_id.into(),
                    from_provider,
                    new_provider,
                    // manifest_path: synthetic — admit_spirit receives parsed
                    // sections, not the original path. Consumers should not
                    // parse this field as a real filesystem path.
                    format!("manifest:{spirit_id}"),
                    crate::capability::cap_tokens::monotonic_now_ns(),
                );
                let payload_bytes = serde_json::to_vec(&payload)
                    .map_err(|e| SecurityError::T3AdmissionFailed(e.to_string()))?;
                journal.journal_lifecycle(JournalEntry::Lifecycle(LifecycleEntry {
                    timestamp: payload.applied_at_ns,
                    lifecycle_event: LifecycleEvent::ProviderSwitched,
                    spirit_id: spirit_id.into(),
                    effective_sandbox_tier: None,
                    payload: Some(payload_bytes),
                }));
            }
        }

        Ok(SandboxSpec {
            tier: effective,
            resolved_caps,
            declared_scopes,
            spirit_id: spirit_id.into(),
            output_shape_predicate: predicate,
        })
    }

    /// Emit a sandbox-block audit event (non-blocking).
    pub fn emit_sandbox_block(
        &self,
        sender: &cap_audit::Sender,
        spirit_pid: u32,
        attempted_syscall: &str,
        sandbox_tier: SandboxTier,
    ) {
        let event = CapAuditEvent::SandboxBlock {
            spirit_pid,
            attempted_syscall: attempted_syscall.into(),
            sandbox_tier,
        };
        // Non-blocking; drop if channel saturated (ADR-030).
        if sender.try_send(event).is_err() {
            cap_audit::record_drop();
        }
    }
}

impl Default for SecurityManagerAdapter {
    fn default() -> Self {
        Self {
            policy: Arc::new(PolicyTable::new()),
            t3_image_verification: None,
            drift_sender: None,
            provider_history: Arc::new(std::sync::Mutex::new(ProviderHistory::default())),
        }
    }
}

impl SecurityManagerPort for SecurityManagerAdapter {
    fn sandbox_tier_floor(&self, _spirit_id: &str) -> SandboxTier {
        // v0.1-β: return the global operator floor.
        let inner = self.policy.inner().load_full();
        inner.operator_policy.global_sandbox_floor
    }

    fn effective_sandbox_tier(&self, spirit_pid: u32) -> Option<SandboxTier> {
        let inner = self.policy.inner().load_full();
        let trust_tier = inner
            .manifest_scopes
            .get(&spirit_pid)
            .map(|m| m.trust_tier)
            .unwrap_or(TrustTier::Verified);
        Some(
            self.policy
                .effective_sandbox_tier(spirit_pid, trust_tier, &inner),
        )
    }

    fn approval_class(&self, _capability: &str) -> maos_domain::invariants::i4::ApprovalDecision {
        // v0.1-β placeholder: auto-approve.
        maos_domain::invariants::i4::ApprovalDecision {
            actor: "kernel".into(),
            target: "spirit".into(),
            capability: _capability.into(),
            intent: "default".into(),
            decision: true,
            reasoning: Some("v0.1-β placeholder approval".into()),
        }
    }
}

#[cfg(test)]
mod provider_history_tests {
    use super::ProviderHistory;

    /// AC-8 — the tracker is bounded: it never exceeds CAP regardless of churn,
    /// and `insert` still returns the previous value so switch-detection works.
    #[test]
    fn provider_history_is_bounded_under_churn() {
        let mut h = ProviderHistory::default();
        // Insert far more distinct spirits than the cap.
        for i in 0..(ProviderHistory::CAP + 5_000) {
            let prev = h.insert(format!("spirit-{i}"), "anthropic".to_string());
            assert!(prev.is_none(), "distinct keys have no prior value");
        }
        assert!(
            h.len() <= ProviderHistory::CAP,
            "provider_history must stay bounded (was {}, cap {})",
            h.len(),
            ProviderHistory::CAP
        );
    }

    /// AC-8 — overflow policy is evict-oldest, NOT reject-new: re-inserting an
    /// existing key returns the previous provider (switch is detectable) and
    /// the newest entry always survives.
    #[test]
    fn provider_history_tracks_switch_and_keeps_newest() {
        let mut h = ProviderHistory::default();
        assert_eq!(h.insert("s1".into(), "anthropic".into()), None);
        // Same key, new provider → returns prior (a detectable switch).
        assert_eq!(
            h.insert("s1".into(), "openai".into()),
            Some("anthropic".to_string())
        );
        // Fill past the cap; the most-recently-inserted key must remain.
        for i in 0..(ProviderHistory::CAP + 10) {
            h.insert(format!("bulk-{i}"), "ollama".into());
        }
        let last = format!("bulk-{}", ProviderHistory::CAP + 10 - 1);
        // Re-inserting the newest returns its tracked value (not evicted).
        assert_eq!(h.insert(last, "ollama".into()), Some("ollama".to_string()));
    }

    /// Regression: re-inserting an already-tracked spirit must move it to the
    /// back of the eviction order, not leave a ghost entry at the old position.
    /// Without the fix, `order` would accumulate duplicates and desynchronise
    /// from `map`, causing either unbounded growth or premature eviction of
    /// live entries.
    #[test]
    fn provider_history_reinsertion_keeps_order_synced() {
        let mut h = ProviderHistory::default();

        // Seed three entries: A (oldest), B, C (newest).
        h.insert("A".into(), "p1".into());
        h.insert("B".into(), "p1".into());
        h.insert("C".into(), "p1".into());

        // Re-insert A — it should move to the back of the eviction order.
        h.insert("A".into(), "p2".into());

        // order must still have exactly 3 entries (no duplicates).
        assert_eq!(
            h.order.len(),
            h.map.len(),
            "order and map must stay in sync"
        );
        assert_eq!(h.order.len(), 3);

        // Eviction order is now B, C, A (B is oldest).
        assert_eq!(h.order[0], "B");
        assert_eq!(h.order[2], "A");
    }

    /// Regression: after eviction-then-reinsertion of the same spirit_id,
    /// order must contain exactly one entry for that id and the map must
    /// track it.
    #[test]
    fn provider_history_evict_then_reinsert_no_ghost() {
        let mut h = ProviderHistory::default();

        // Fill to exactly CAP entries.
        for i in 0..ProviderHistory::CAP {
            h.insert(format!("s-{i}"), "p".into());
        }
        assert_eq!(h.len(), ProviderHistory::CAP);

        // One more insert evicts s-0.
        h.insert("overflow".into(), "p".into());
        assert_eq!(h.len(), ProviderHistory::CAP);
        assert!(!h.map.contains_key("s-0"), "s-0 should have been evicted");

        // Re-insert s-0 (genuinely new again after eviction).
        let prev = h.insert("s-0".into(), "p".into());
        assert!(prev.is_none(), "evicted key re-insertion should be None");

        // order and map must stay perfectly in sync.
        assert_eq!(
            h.order.len(),
            h.map.len(),
            "order ({}) != map ({}) after evict-reinsert cycle",
            h.order.len(),
            h.map.len(),
        );
    }
}
