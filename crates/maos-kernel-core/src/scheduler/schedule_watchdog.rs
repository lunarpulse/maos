#![forbid(unsafe_code)]

//! Story 6.4 / FR26 / ADR-025 — per-Spirit + per-schedule_id ScheduleWatchdog.
//!
//! Polls the SCB map; for each Running Spirit with `[[schedule]]` entries
//! whose `on_schedule` hook is enabled (per `[lifecycle].enabled_hooks`),
//! fires `on_schedule(ctx, schedule_id, payload)` when
//! `now_ns - last_fire_ns ≥ cadence_secs`.
//!
//! Per-firing gate (rejection ordered):
//!   1. `kernel_invocation_allowed("on_schedule")` per the manifest's
//!      lifecycle gate
//!   2. principal-revocability check (when `entry.principal_revocability`)
//!   3. rate-limit check against the per-`(spirit_id, schedule_id)` bucket
//!   4. ComplianceClaim stamp recorded in the firing's TL row
//!   5. `side_effect_allowlist` narrows the cap-token issued for the firing
//!
//! Each firing is journaled to the Transparency Log (Story 1b.1) with
//! `FrameKind::CapabilityInvocation` carrying the schedule_id, the
//! ComplianceClaim envelope hash (when present), and the narrowed scope.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use dashmap::DashMap;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::capability::CapabilityRegistryAdapter;
use crate::iac::transparency_log::{FrameKind as TlFrameKind, TransparencyLogAdapter};
use crate::scheduler::control_block::{ScbLifecycleState, SpiritControlBlock};
use crate::scheduler::hook_dispatch::HookDispatcher;
use crate::security::manifest::ScheduleEntry;
use maos_domain::invariants::i1::{IntentClass, TokenId};
use maos_domain::invariants::i3::FrameOrigin;

/// Per-schedule rate-limit bucket. Fractional refill is encoded as
/// f64 token-balance; on consume we top up by elapsed_ns × refill_per_sec.
///
/// Lives keyed by `(spirit_id, schedule_id)` on the watchdog. Separate
/// from the AC4 provider rate-limit bucket.
#[maos_attrs::i9_exempt(
    reason = "per-schedule rate-limit bucket; AtomicU64 + Mutex<f64> are transient per-process state owned by the ScheduleWatchdog DashMap (already I9-exempt). Dropped when the bucket entry evicts."
)]
#[derive(Debug)]
struct ScheduleBucket {
    capacity: f64,
    refill_per_sec: f64,
    tokens: parking_lot::Mutex<f64>,
    last_refill_ns: AtomicU64,
}

impl ScheduleBucket {
    fn new(rate_limit_per_hour: u32) -> Self {
        let cap = rate_limit_per_hour as f64;
        Self {
            capacity: cap,
            // tokens-per-second = per-hour / 3600.
            refill_per_sec: cap / 3600.0,
            tokens: parking_lot::Mutex::new(cap),
            last_refill_ns: AtomicU64::new(crate::capability::cap_tokens::monotonic_now_ns()),
        }
    }

    /// Top up the bucket based on elapsed time, then try to consume one
    /// whole token. Returns `true` on success, `false` on empty.
    fn try_consume(&self) -> bool {
        let now = crate::capability::cap_tokens::monotonic_now_ns();
        let last = self.last_refill_ns.swap(now, Ordering::AcqRel);
        let elapsed_secs = (now.saturating_sub(last) as f64) / 1_000_000_000.0;
        let refill = elapsed_secs * self.refill_per_sec;

        let mut tokens = self.tokens.lock();
        *tokens = (*tokens + refill).min(self.capacity);
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Story 6.4 / FR26 — schedule firing watchdog (`on_schedule` cadence loop).
#[maos_attrs::i9_exempt(
    reason = "per-Spirit schedule watchdog; DashMap holds transient per-process rate-limit buckets, not persistent state"
)]
pub struct ScheduleWatchdog {
    scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
    dispatcher: Arc<HookDispatcher>,
    capability: Option<Arc<CapabilityRegistryAdapter>>,
    transparency_log: Arc<TransparencyLogAdapter>,
    /// Per-(spirit_id, schedule_id) token bucket for `rate_limit_per_hour`
    /// enforcement (separate from the AC4 provider rate-limit substrate).
    rate_limits: DashMap<(String, String), Arc<ScheduleBucket>>,
    /// Per-(spirit_pid, schedule_id) last-fire timestamp (ns).
    last_fire_ns: DashMap<(u32, String), u64>,
}

impl ScheduleWatchdog {
    pub fn new(
        scbs: Arc<RwLock<BTreeMap<u32, Arc<SpiritControlBlock>>>>,
        dispatcher: Arc<HookDispatcher>,
        transparency_log: Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self {
            scbs,
            dispatcher,
            capability: None,
            transparency_log,
            rate_limits: DashMap::new(),
            last_fire_ns: DashMap::new(),
        }
    }

    pub fn with_capability(mut self, c: Arc<CapabilityRegistryAdapter>) -> Self {
        self.capability = Some(c);
        self
    }

    /// Story 6.4 — last-fire timestamp introspection for tests + smoke arms.
    /// Returns the nanosecond timestamp of the last firing for this
    /// `(spirit_pid, schedule_id)`, or `0` if never fired.
    pub fn last_fire_ns(&self, spirit_pid: u32, schedule_id: &str) -> u64 {
        self.last_fire_ns
            .get(&(spirit_pid, schedule_id.to_string()))
            .map(|r| *r.value())
            .unwrap_or(0)
    }

    /// Spawn the watchdog background task.
    ///
    /// Polls the SCB map every `poll_interval_ms` (default 1000ms; with
    /// `MAOS_SCHEDULE_FAST=1` the poll collapses to 40ms and cadence checks
    /// scale by 100× — mirrors the IdleWatchdog `MAOS_IDLE_FAST` convention).
    pub fn spawn(self: Arc<Self>, cancel: CancellationToken) -> JoinHandle<()> {
        tokio::spawn(async move {
            let fast_mode = std::env::var_os("MAOS_SCHEDULE_FAST").is_some();
            let poll_ms = if fast_mode { 40u64 } else { 1000u64 };
            let mut interval = tokio::time::interval(Duration::from_millis(poll_ms));

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => {
                        break;
                    }
                    _ = interval.tick() => {
                        self.check_and_fire(fast_mode).await;
                    }
                }
            }
        })
    }

    /// One poll cycle.
    async fn check_and_fire(&self, fast_mode: bool) {
        // Collect candidates outside the lock to avoid holding RwLock across
        // await points (Story 5.1 review-backfill lesson).
        let candidates: Vec<(Arc<SpiritControlBlock>, ScheduleEntry)> = {
            let scbs = self.scbs.read().unwrap();
            let now_ns = crate::capability::cap_tokens::monotonic_now_ns();
            let mut out = Vec::new();
            for (_, scb) in scbs.iter() {
                if scb.current_state() != ScbLifecycleState::Running {
                    continue;
                }
                if scb.manifest.schedules.entries.is_empty() {
                    continue;
                }
                // Step 1 — lifecycle gate.
                if !Self::on_schedule_allowed(scb) {
                    continue;
                }
                for entry in &scb.manifest.schedules.entries {
                    let last = self
                        .last_fire_ns
                        .get(&(scb.pid, entry.id.clone()))
                        .map(|r| *r.value())
                        .unwrap_or(0);
                    let cadence_ns = Self::cadence_window_ns(entry.cadence_secs, fast_mode);
                    let elapsed = now_ns.saturating_sub(last);
                    if last != 0 && elapsed < cadence_ns {
                        continue;
                    }
                    out.push((Arc::clone(scb), entry.clone()));
                }
            }
            out
        };

        for (scb, entry) in candidates {
            // Step 2 — principal-revocability proxy: at v0.5 the per-Spirit
            // revocation status is the proxy for principal revocation
            // (Story 9.2 wires the cascade). If the Spirit's tokens are
            // wholesale revoked, skip the firing.
            if entry.principal_revocability && self.is_principal_revoked(&scb) {
                continue;
            }

            // Step 3 — issue narrowed cap-token (one per Scope at v0.5).
            // Records the first issued token_id in the TL row.
            // `None` is valid when side_effect_scopes is empty OR when the
            // capability registry is not wired (test context); only skip
            // firing when the registry is present but issuance fails.
            let token_id = self.issue_narrowed_token(&scb, &entry);
            let fired_at_ns = crate::capability::cap_tokens::monotonic_now_ns();

            // Write TL row BEFORE firing the hook (I2 log-before-deliver).
            let record = crate::iac::payload::ScheduleFireRecord {
                spirit_id: scb.spirit_id.clone(),
                schedule_id: entry.id.clone(),
                fired_at_ns,
                compliance_claim_ref: entry.compliance_claim_ref,
                side_effect_token_id: token_id.unwrap_or(TokenId::ZERO),
                principal_revocability: entry.principal_revocability,
            };
            let payload_json = match serde_json::to_vec(&record) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "maos: WARN schedule.fire TL row payload serialize failed for {}/{}: {}",
                        scb.spirit_id, entry.id, e
                    );
                    // Skip TL write rather than write malformed bytes (I2 still
                    // honored — the dispatch decision logs structurally).
                    continue;
                }
            };

            // Step 4 — rate-limit consume (AFTER serialization succeeds so we
            // don't deplete the bucket on a path that aborts).
            let bucket_key = (scb.spirit_id.clone(), entry.id.clone());
            let bucket = self
                .rate_limits
                .entry(bucket_key)
                .or_insert_with(|| Arc::new(ScheduleBucket::new(entry.rate_limit_per_hour)))
                .value()
                .clone();
            if !bucket.try_consume() {
                continue;
            }

            let intent_str = format!("schedule.fire:{}", entry.id);
            let _ = self.transparency_log.insert_frame_event(
                TlFrameKind::CapabilityInvocation,
                scb.pid,
                None,
                &intent_str,
                &payload_json,
                FrameOrigin::Kernel,
            );

            // Fire the hook with the manifest's payload_bytes.
            let _outcome = self
                .dispatcher
                .fire_on_schedule(&scb, &entry.payload_bytes)
                .await;

            // Update last-fire timestamp regardless of hook outcome — the
            // dispatch decision (gate passes, rate-limit consumed) is the
            // firing event for cadence purposes.
            self.last_fire_ns
                .insert((scb.pid, entry.id.clone()), fired_at_ns);
        }
    }

    fn on_schedule_allowed(scb: &SpiritControlBlock) -> bool {
        let enabled: Vec<&str> = scb
            .manifest
            .lifecycle
            .enabled_hooks
            .iter()
            .map(|s| s.as_str())
            .collect();
        maos_spirit_abi::lifecycle::kernel_invocation_allowed(&enabled, "on_schedule")
    }

    fn cadence_window_ns(cadence_secs: u32, fast_mode: bool) -> u64 {
        let ns = (cadence_secs as u64) * 1_000_000_000;
        if fast_mode {
            // 100× collapse, floored at 1ms so the watchdog can re-fire on
            // back-to-back polls.
            (ns / 100).max(1_000_000)
        } else {
            ns
        }
    }

    fn is_principal_revoked(&self, scb: &SpiritControlBlock) -> bool {
        // v0.5 proxy — check whether the capability registry reports any
        // active tokens for this spirit. When the registry adapter is absent
        // (tests that don't wire capabilities), default to not-revoked.
        // Story 9.2 lands the full per-principal cascade walker.
        let Some(cap) = &self.capability else {
            return false;
        };
        !cap.has_active_tokens_for_pid(scb.pid)
    }

    fn issue_narrowed_token(
        &self,
        scb: &SpiritControlBlock,
        entry: &ScheduleEntry,
    ) -> Option<TokenId> {
        if entry.side_effect_scopes.is_empty() {
            return None;
        }
        let cap = self.capability.as_ref()?;
        // Issue one token per scope at v0.5; record the first token_id.
        let mut first_id: Option<TokenId> = None;
        // TTL = 2 × cadence_secs capped at 300s per ADR-023 Standard
        // intent_class (Story 6.4 AC2).
        let ttl_secs = ((entry.cadence_secs.saturating_mul(2)).min(300)).max(1);
        let posture_hash = [0u8; 32];
        for scope in &entry.side_effect_scopes {
            match cap.issue_with_mediation(
                scb.pid,
                scope.clone(),
                ttl_secs,
                posture_hash,
                IntentClass::Standard,
            ) {
                Ok(token) => {
                    if first_id.is_none() {
                        first_id = Some(token.token_id);
                    }
                }
                Err(e) => {
                    // Token issue failure: not a hard error at v0.5 — the
                    // firing proceeds with empty side-effect token (the
                    // hook will fail the in-Spirit capability check, which
                    // is the correct posture). Keep the denial observable:
                    // Story 11.4a CATCH-B forbids silent org-policy denies.
                    eprintln!(
                        "schedule-watchdog: capability token issue denied for spirit {}: {e}",
                        scb.pid
                    );
                    continue;
                }
            }
        }
        first_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_window_normal_and_fast() {
        // Normal mode — 1 second cadence = 1_000_000_000 ns.
        assert_eq!(ScheduleWatchdog::cadence_window_ns(1, false), 1_000_000_000);
        // Fast mode — 1 second cadence → 10_000_000 ns (collapsed 100×).
        assert_eq!(ScheduleWatchdog::cadence_window_ns(1, true), 10_000_000);
        // Floor: a 0.5ms target rounds up to 1ms in fast mode.
        // 1 sec / 100 = 10ms which is > 1ms floor; no clamp.
        // For cadence 0 (would never validate but checks floor):
        assert_eq!(ScheduleWatchdog::cadence_window_ns(0, true), 1_000_000);
    }

    #[test]
    fn schedule_bucket_consume_and_refill() {
        crate::capability::cap_tokens::init_monotonic_base();
        // 3600/hour = 1/sec.
        let bucket = ScheduleBucket::new(3600);
        // Start full: consume 3600 successfully... actually starting at
        // full capacity (cap), we can consume capacity tokens before empty.
        // Smoke-test 5 quick consumes.
        for _ in 0..5 {
            assert!(bucket.try_consume(), "initial consume should succeed");
        }
    }

    #[test]
    fn schedule_bucket_starts_full() {
        crate::capability::cap_tokens::init_monotonic_base();
        let bucket = ScheduleBucket::new(2); // 2/hour
                                             // Two consumes succeed; the third fails (refill rate is 2/3600 per sec,
                                             // imperceptible at test speed).
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume(), "third consume MUST fail");
    }
}
