#![forbid(unsafe_code)]

//! `observer` — MAOS Observer v0.5, the third reference Spirit and the **first
//! read-only perceptual Spirit** (architecture §6.5). Story 8.3.
//!
//! Observer is a **watchdog**. It:
//!
//! 1. **Subscribes broadly** to the `scalar.tap` Telemetry Stream (I7 —
//!    runtime-operational at v0.5) and applies a **client-side principal-
//!    namespace filter** on the emitting Spirit's id (FR31 — the
//!    [`TelemetryStreamPort`](maos_domain::ports::TelemetryStreamPort) has only
//!    exact-topic subscription, so the namespace scoping lives Spirit-side;
//!    Decision C).
//! 2. **Watches for pre-halt scalar drift** ([`Observer::observe_scalar`]): it
//!    keeps a rolling per-`(spirit_id, tag)` trajectory and fires a **drift
//!    early-warning when a value enters a watch-band approaching (but has not yet
//!    crossed) the halt threshold** — so the operator can intervene BEFORE the
//!    halt fires (the v0.5 acceptance demo; the I7 "witness the runup, not just
//!    the alarm" obligation). The watch thresholds are Observer-side config
//!    (Decision D — `ScalarTapEvent` carries the value, never the threshold).
//! 3. **Classifies structural-anomaly suspects** ([`Observer::classify_signal`])
//!    from the NFR-Sec-3 divergence inputs (syscall-pattern divergence, fd-table
//!    growth, unexpected outbound IAC). NFR-Sec-3 is **v2.0 (ADR-024)** — at v0.5
//!    these inputs are **fixture-replayed, shaped as the real
//!    [`FrameKind::SandboxBlock`] discriminator](SANDBOX_BLOCK_FRAME_KIND)** the
//!    v2.0 wiring will deliver (Decision E).
//!
//! Both the drift early-warning and the structural-anomaly suspect become an
//! [`ObserverSurface`], which converts to the existing **[`NotificationEvent::
//! AnomalyFlagged`]** (the §7.4 operator surface §6.5 grants Observer — Story 3.4
//! shipped that variant expressly "for full Observer wiring at Story 8.3").
//! Observer adds **no new `FrameKind`/`FramePayload`** (Decision B; the ABI stays
//! frozen).
//!
//! ## Zero kernel KLOC (Story 0.2 invariant)
//! This crate depends only on the Spirit SDK/ABI and the PURE `maos-domain`
//! types (the i7 scalar.tap types, the §7.4 `NotificationEvent`, the `FrameKind`
//! discriminator). It NEVER reaches into `maos-kernel-core` /
//! `maos-director-surface`. The real `scalar.tap` stream and the real
//! `NotificationDispatcher` are exercised in `tests/`, which carry those crates
//! as dev-dependencies only (dev-deps do not enter the kernel-API surface the
//! boundary gate guards — Butler/Researcher's resolved pattern). The
//! *interpretation* of drift/anomaly (the thresholds, the confidence, the
//! "is this malice?" decision) lives entirely here; the kernel only ever carried
//! the structural signal — it never classifies (§4.0.7).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use maos_domain::invariants::i7::ScalarTapEvent;
use maos_domain::notification::{NotificationEvent, NotificationEventError};
use maos_spirit_sdk::{spirit, Ctx, Spirit};
use serde::{Deserialize, Serialize};

/// The frame-kind discriminator the NFR-Sec-3 structural alarms are carried on.
///
/// The kernel emits sandbox-escape *outcomes* as [`FrameKind::SandboxBlock`]
/// (`= 8`) today; the three v2.0 divergence inputs Observer classifies are
/// fixture-replayed SHAPED as this discriminator (Decision E) so the v2.0
/// source-swap is a wiring change, not a rewrite. Bound to the real ABI value
/// via [`maos_spirit_abi::identity::FrameKind::SandboxBlock`].
pub const SANDBOX_BLOCK_FRAME_KIND: u8 = maos_spirit_abi::identity::FrameKind::SandboxBlock as u8;

/// Confidence assigned to a reject-and-flag surface for a non-comparable
/// (NaN/Inf) measurement (Story 8.10 AC4a). Maximal: an un-decidable
/// measurement is treated as a definite anomaly so the watchdog raises rather
/// than going dark. In `[0.0, 1.0]` so the `anomaly_flagged` constructor accepts it.
pub const NON_COMPARABLE_FLAG_CONFIDENCE: f32 = 1.0;

// ───────────────────────────────────────────────────────────────────────────
// Cognitive posture (architecture §6.5). The manifest `[posture]` section is the
// AUTONOMY spectrum (`cautious`); THIS is the Observer cognitive posture.
// ───────────────────────────────────────────────────────────────────────────

/// Observer's cognitive posture. `PassiveObserver` is the only posture (§6.5):
/// silent allow on all reads; no exec; no mutating; no control-plane. Realized
/// Spirit-side (the manifest `[posture]` autonomy = `cautious`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObserverPosture {
    /// Read-only perceptual layer (§6.5).
    PassiveObserver,
}

// ───────────────────────────────────────────────────────────────────────────
// Principal-namespace scope — FR31 client-side filter (Decision C).
// ───────────────────────────────────────────────────────────────────────────

/// The set of principals (Spirit ids) Observer is scoped to watch (FR31).
///
/// A pattern is either an exact id or a `prefix*` wildcard. The
/// [`TelemetryStreamPort`](maos_domain::ports::TelemetryStreamPort) cannot scope
/// a subscription by namespace, so Observer drops out-of-scope events
/// **client-side** on the emitting `spirit_id` (Decision C).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrincipalScope {
    patterns: Vec<String>,
}

impl PrincipalScope {
    /// A scope that admits every principal (`["*"]`).
    pub fn all() -> Self {
        Self {
            patterns: vec!["*".to_string()],
        }
    }

    /// A scope from explicit id / `prefix*` patterns.
    pub fn from_patterns<I, S>(patterns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            patterns: patterns.into_iter().map(Into::into).collect(),
        }
    }

    /// Whether `spirit_id` falls under Observer's principal namespace.
    pub fn admits(&self, spirit_id: &str) -> bool {
        self.patterns.iter().any(|p| {
            if p == "*" {
                true
            } else if let Some(prefix) = p.strip_suffix('*') {
                spirit_id.starts_with(prefix)
            } else {
                p == spirit_id
            }
        })
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Drift watch config (Decision D) — Observer-side thresholds.
// ───────────────────────────────────────────────────────────────────────────

/// The direction a peer's `[epistemic_policy]` halt rule fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DriftDirection {
    /// Halt when the scalar rises ABOVE `threshold` (peer `on_value_above`).
    Above,
    /// Halt when the scalar falls BELOW `threshold` (peer `on_value_below`).
    Below,
}

/// A drift watch on one scalar `tag` (Decision D). Observer fires an
/// early-warning when the value enters the band `[threshold ∓ warn_margin)`
/// approaching `threshold`, BEFORE the peer's halt predicate fires.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchThreshold {
    /// The scalar tag to watch (e.g. `belief_variance`).
    pub tag: String,
    /// The peer's halt threshold (mirrors the peer `[epistemic_policy]` rule).
    pub threshold: f64,
    /// Which side the halt predicate fires on.
    pub direction: DriftDirection,
    /// The width of the pre-halt watch-band approaching `threshold` (> 0).
    pub warn_margin: f64,
}

/// Construction error for [`WatchThreshold`] (Story 8.10 AC4b — Fork B `Result`
/// shape, consistent with `EpistemicHaltPayload::new` / `DistillationRequest::new`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchThresholdError {
    /// `threshold` was NaN or ±∞ — a non-comparable threshold would silently
    /// disable the watch (the watchdog could never decide in/out of band).
    NonFiniteThreshold,
    /// `warn_margin` was NaN or ±∞ — a non-comparable band width.
    NonFiniteWarnMargin,
}

impl std::fmt::Display for WatchThresholdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteThreshold => write!(f, "watch threshold must be finite (got NaN/Inf)"),
            Self::NonFiniteWarnMargin => write!(f, "watch warn_margin must be finite (got NaN/Inf)"),
        }
    }
}
impl std::error::Error for WatchThresholdError {}

impl WatchThreshold {
    /// Construct a watch. Story 8.10 AC4b: **rejects** a NaN/Inf `threshold` or
    /// `warn_margin` (a non-comparable bound would silently disable the
    /// watchdog). A finite non-positive `warn_margin` is still clamped to a tiny
    /// epsilon so the band is always well-formed.
    pub fn new(
        tag: impl Into<String>,
        threshold: f64,
        direction: DriftDirection,
        warn_margin: f64,
    ) -> Result<Self, WatchThresholdError> {
        if !threshold.is_finite() {
            return Err(WatchThresholdError::NonFiniteThreshold);
        }
        if !warn_margin.is_finite() {
            return Err(WatchThresholdError::NonFiniteWarnMargin);
        }
        Ok(Self {
            tag: tag.into(),
            threshold,
            direction,
            warn_margin: if warn_margin > 0.0 {
                warn_margin
            } else {
                f64::EPSILON
            },
        })
    }

    /// `[low, high)` bounds of the pre-halt watch-band for this watch.
    fn band(&self) -> (f64, f64) {
        match self.direction {
            DriftDirection::Above => (self.threshold - self.warn_margin, self.threshold),
            DriftDirection::Below => (self.threshold, self.threshold + self.warn_margin),
        }
    }

    /// Whether `value` is inside the watch-band but has NOT yet crossed the
    /// threshold (the pre-halt early-warning window).
    fn in_band(&self, value: f64) -> bool {
        let (low, high) = self.band();
        match self.direction {
            // Approaching from below: low ≤ value < threshold.
            DriftDirection::Above => value >= low && value < high,
            // Approaching from above: threshold < value ≤ high.
            DriftDirection::Below => value > low && value <= high,
        }
    }

    /// Whether `value` is outside the watch-band entirely (drift episode reset).
    fn outside_band(&self, value: f64) -> bool {
        let (low, high) = self.band();
        match self.direction {
            DriftDirection::Above => value < low,
            DriftDirection::Below => value > high,
        }
    }

    /// Proximity-to-threshold confidence in `[0.0, 1.0]` for an in-band value:
    /// 0 at the band edge, →1 as the value nears the halt threshold.
    fn proximity(&self, value: f64) -> f32 {
        let (low, high) = self.band();
        let frac = match self.direction {
            DriftDirection::Above => (value - low) / (high - low),
            DriftDirection::Below => (high - value) / (high - low),
        };
        (frac as f32).clamp(0.0, 1.0)
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Structural-anomaly inputs (NFR-Sec-3, v2.0; fixture-replayed at v0.5).
// ───────────────────────────────────────────────────────────────────────────

/// The three NFR-Sec-3 structural-anomaly divergence kinds (§8 / NFR-Sec-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DivergenceKind {
    /// Syscall pattern diverges from the manifest's declared profile.
    SyscallPatternDivergence,
    /// The Spirit's file-descriptor table is growing unexpectedly.
    FdTableGrowth,
    /// An outbound IAC connection not declared in the manifest appeared.
    UnexpectedOutboundIac,
}

impl DivergenceKind {
    fn as_tag(self) -> &'static str {
        match self {
            DivergenceKind::SyscallPatternDivergence => "syscall-pattern-divergence",
            DivergenceKind::FdTableGrowth => "fd-table-growth",
            DivergenceKind::UnexpectedOutboundIac => "unexpected-outbound-iac",
        }
    }
}

/// A structural signal Observer classifies. At v0.5 these are fixture-replayed,
/// shaped as the real [`FrameKind::SandboxBlock`](SANDBOX_BLOCK_FRAME_KIND)
/// discriminator (Decision E). `magnitude` is the kernel-supplied structural
/// measurement (e.g. divergence score, fd growth ratio) in `[0.0, 1.0]`; the
/// kernel never interprets it — Observer decides whether it is a suspect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralSignal {
    /// The frame-kind discriminator the alarm rode in on. Expected to equal
    /// [`SANDBOX_BLOCK_FRAME_KIND`]; a mismatch is itself surfaced as suspect.
    pub frame_kind: u8,
    /// The Spirit the structural alarm is about.
    pub subject: String,
    /// Which structural divergence the kernel observed.
    pub kind: DivergenceKind,
    /// The structural measurement in `[0.0, 1.0]` (kernel-supplied, uninterpreted).
    #[doc = "Construct via [`StructuralSignal::new`] to enforce validation; struct literals bypass magnitude-range / NaN-Inf / non-empty-subject checks."]
    pub magnitude: f64,
    /// A human-readable structural detail (e.g. `"fd count 412 vs declared 64"`).
    #[serde(default)]
    pub detail: String,
}

/// Construction error for [`StructuralSignal`] (Story 8.10 AC5 — validating
/// constructor for the last invariant-bearing type lacking one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralSignalError {
    /// `subject` was empty — a structural alarm with no subject is unactionable.
    EmptySubject,
    /// `magnitude` was NaN or ±∞ — a non-comparable structural measurement.
    NonFiniteMagnitude,
    /// `magnitude` was outside `[0.0, 1.0]`.
    MagnitudeOutOfRange,
}

impl std::fmt::Display for StructuralSignalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySubject => write!(f, "structural signal subject must be non-empty"),
            Self::NonFiniteMagnitude => {
                write!(f, "structural signal magnitude must be finite (got NaN/Inf)")
            }
            Self::MagnitudeOutOfRange => {
                write!(f, "structural signal magnitude must be in [0.0, 1.0]")
            }
        }
    }
}
impl std::error::Error for StructuralSignalError {}

impl StructuralSignal {
    /// Validated constructor (Story 8.10 AC5). Rejects an empty `subject`, a
    /// NaN/Inf `magnitude`, and a `magnitude` outside `[0.0, 1.0]`. This is the
    /// only path that enforces the invariants; the `pub` fields remain (no ABI
    /// churn) but a struct literal is documented as bypass-prone.
    pub fn new(
        frame_kind: u8,
        subject: impl Into<String>,
        kind: DivergenceKind,
        magnitude: f64,
        detail: impl Into<String>,
    ) -> Result<Self, StructuralSignalError> {
        let subject = subject.into();
        if subject.trim().is_empty() {
            return Err(StructuralSignalError::EmptySubject);
        }
        if !magnitude.is_finite() {
            return Err(StructuralSignalError::NonFiniteMagnitude);
        }
        if !(0.0..=1.0).contains(&magnitude) {
            return Err(StructuralSignalError::MagnitudeOutOfRange);
        }
        Ok(Self {
            frame_kind,
            subject,
            kind,
            magnitude,
            detail: detail.into(),
        })
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Observer surface — the unified output (drift early-warning | structural suspect).
// ───────────────────────────────────────────────────────────────────────────

/// Which kind of anomaly an [`ObserverSurface`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnomalyKind {
    /// A pre-halt scalar-drift early-warning (AC3).
    DriftEarlyWarning,
    /// A structural-anomaly suspect (AC4).
    StructuralAnomalySuspect,
}

/// One operator-facing observation Observer produces. Serializes to exactly the
/// manifest `[output_shape]` `required_fields` (`subject` / `summary` /
/// `confidence` / `anomaly_kind`) and converts to the §7.4
/// [`NotificationEvent::AnomalyFlagged`] (Decision B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObserverSurface {
    /// The Spirit the observation is about.
    pub subject: String,
    /// Human-readable summary (drift early-warning / structural_anomaly_suspect).
    pub summary: String,
    /// Observer-computed confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Whether this is a drift early-warning or a structural-anomaly suspect.
    pub anomaly_kind: AnomalyKind,
}

impl ObserverSurface {
    /// Convert to the §7.4 operator notification (§6.5 notification-only
    /// authority). Routes `subject`/`summary`/`confidence` through the validated
    /// [`NotificationEvent::anomaly_flagged`] constructor (NaN / empty-summary /
    /// range-checked); `observer_id` is the flagging Observer's Spirit id.
    pub fn to_notification(
        &self,
        observer_id: &str,
    ) -> Result<NotificationEvent, NotificationEventError> {
        NotificationEvent::anomaly_flagged(
            observer_id,
            self.subject.clone(),
            self.summary.clone(),
            self.confidence,
        )
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The Observer Spirit.
// ───────────────────────────────────────────────────────────────────────────

/// Observer reference Spirit — a read-only telemetry watchdog. Holds its
/// principal-namespace scope, its drift watches, and a rolling per-`(spirit_id,
/// tag)` trajectory. `Arc<Mutex<...>>` interior state keeps Observer `Sync` as
/// the `#[spirit]` macro requires.
#[derive(Debug, Clone)]
pub struct Observer {
    /// This Observer's own Spirit id (the `observer` field of every surface).
    observer_id: String,
    /// FR31 principal-namespace scope (client-side filter, Decision C).
    namespace: PrincipalScope,
    /// Per-tag drift watch configs (Decision D).
    watches: Vec<WatchThreshold>,
    /// Rolling last value per `(spirit_id, tag)` + the set currently warned-for
    /// (so one drift episode produces one early-warning, not a flood).
    state: Arc<Mutex<DriftState>>,
    /// Optional seeded inputs an `on_idle` pass evaluates (fixture / harness path).
    pending_scalars: Option<Vec<ScalarTapEvent>>,
    pending_signals: Option<Vec<StructuralSignal>>,
    /// Surfaces produced by the most recent `on_idle` pass.
    last_surfaces: Arc<Mutex<Vec<ObserverSurface>>>,
}

#[derive(Debug, Default)]
struct DriftState {
    last_value: BTreeMap<(String, String), f64>,
    warned: BTreeSet<(String, String)>,
}

#[spirit]
impl Observer {
    /// Idle watchdog pass. Cancellation-aware; bounded (a single linear pass over
    /// any seeded scalar events + structural signals). Stores the resulting
    /// surfaces so the hook has a production-visible effect; the LIVE drift /
    /// structural paths are proven against the real `TelemetryStreamAdapter` /
    /// `NotificationDispatcher` in `tests/` (the ABI `Ctx` exposes no telemetry-
    /// receive surface — Butler/Researcher navigated the same gap).
    fn on_idle(&self, ctx: &mut Ctx) {
        if ctx.cancellation().is_cancelled() {
            return;
        }
        let mut surfaces = Vec::new();
        if let Some(scalars) = &self.pending_scalars {
            for ev in scalars {
                if let Some(s) = self.observe_scalar(ev) {
                    surfaces.push(s);
                }
            }
        }
        if let Some(signals) = &self.pending_signals {
            for sig in signals {
                if let Some(s) = self.classify_signal(sig) {
                    surfaces.push(s);
                }
            }
        }
        let mut guard = self.last_surfaces.lock().unwrap_or_else(|e| e.into_inner());
        *guard = surfaces;
    }
}

impl Default for Observer {
    fn default() -> Self {
        Self {
            observer_id: "observer".to_string(),
            namespace: PrincipalScope::all(),
            watches: Vec::new(),
            state: Arc::new(Mutex::new(DriftState::default())),
            pending_scalars: None,
            pending_signals: None,
            last_surfaces: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Observer {
    /// A watchdog scoped to `namespace`, watching `watches` (production default).
    pub fn watching(namespace: PrincipalScope, watches: Vec<WatchThreshold>) -> Self {
        Self {
            namespace,
            watches,
            ..Self::default()
        }
    }

    /// Override this Observer's own Spirit id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.observer_id = id.into();
        self
    }

    /// Seed scalar events an `on_idle` pass will evaluate (fixture / harness).
    pub fn with_pending_scalars(mut self, scalars: Vec<ScalarTapEvent>) -> Self {
        self.pending_scalars = Some(scalars);
        self
    }

    /// Seed structural signals an `on_idle` pass will classify (fixture / harness).
    pub fn with_pending_signals(mut self, signals: Vec<StructuralSignal>) -> Self {
        self.pending_signals = Some(signals);
        self
    }

    /// This Observer's own Spirit id.
    pub fn observer_id(&self) -> &str {
        &self.observer_id
    }

    /// The active cognitive posture — always `PassiveObserver` (§6.5).
    pub fn posture(&self) -> ObserverPosture {
        ObserverPosture::PassiveObserver
    }

    /// Whether `spirit_id` is under Observer's principal namespace (FR31).
    pub fn in_namespace(&self, spirit_id: &str) -> bool {
        self.namespace.admits(spirit_id)
    }

    /// Surfaces produced by the most recent `on_idle` pass.
    pub fn last_surfaces(&self) -> Vec<ObserverSurface> {
        self.last_surfaces
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    // ── the pre-halt scalar-drift watchdog (AC3) ─────────────────────────────

    /// Observe one `scalar.tap` event and, if it represents pre-halt drift,
    /// return a drift early-warning [`ObserverSurface`]. Returns `None` when:
    /// the emitter is outside Observer's principal namespace (FR31 drop); no
    /// watch covers the tag; the value is below the watch-band (no drift); the
    /// value already crossed the threshold (the kernel halt fires — too late for
    /// an *early* warning); or the drift episode was already warned (dedup).
    ///
    /// Updates the rolling trajectory as a side effect (so a future episode after
    /// the value leaves the band can warn again).
    pub fn observe_scalar(&self, event: &ScalarTapEvent) -> Option<ObserverSurface> {
        // FR31 — client-side principal-namespace filter (Decision C).
        if !self.in_namespace(&event.spirit_id) {
            return None;
        }
        let watch = self.watches.iter().find(|w| w.tag == event.tag)?;

        // Story 8.10 AC4a — a NaN/Inf scalar is ITSELF an anomaly: a
        // non-comparable measurement on a watched tag would silently disable
        // the watchdog (the old `return None` fail-open). Reject-and-flag so the
        // watchdog raises rather than going dark.
        if !event.value.is_finite() {
            return Some(ObserverSurface {
                subject: event.spirit_id.clone(),
                summary: format!(
                    "non-comparable scalar '{}' = {} (NaN/Inf) on watched tag — \
                     watchdog cannot compare against threshold {:.3}; flagging",
                    event.tag, event.value, watch.threshold
                ),
                confidence: NON_COMPARABLE_FLAG_CONFIDENCE,
                anomaly_kind: AnomalyKind::DriftEarlyWarning,
            });
        }
        let value = event.value;
        let key = (event.spirit_id.clone(), event.tag.clone());

        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.last_value.insert(key.clone(), value);

        // Reset the per-episode dedup once the value falls back out of the band.
        if watch.outside_band(value) {
            state.warned.remove(&key);
            return None;
        }
        if !watch.in_band(value) {
            // Either still far from the band, or already crossed the threshold:
            // not an early-warning. (A crossed value is the kernel's halt.)
            return None;
        }
        // In-band: warn ONCE per drift episode.
        if !state.warned.insert(key) {
            return None;
        }
        drop(state);

        let confidence = watch.proximity(value);
        let dir = match watch.direction {
            DriftDirection::Above => "approaching",
            DriftDirection::Below => "falling toward",
        };
        Some(ObserverSurface {
            subject: event.spirit_id.clone(),
            summary: format!(
                "drift early-warning: scalar '{}' = {:.3} {} halt threshold {:.3} (intervene before halt fires)",
                event.tag, value, dir, watch.threshold
            ),
            confidence,
            anomaly_kind: AnomalyKind::DriftEarlyWarning,
        })
    }

    // ── the structural-anomaly suspect path (AC4) ────────────────────────────

    /// Severity floor above which a structural signal becomes a suspect. The
    /// *interpretation* (this floor, the confidence) is Spirit-side (§4.0.7); the
    /// kernel only supplied the structural measurement.
    pub const STRUCTURAL_SUSPECT_FLOOR: f64 = 0.5;

    /// Classify a structural signal Spirit-side and, if it clears the suspect
    /// floor, return a `structural_anomaly_suspect` [`ObserverSurface`]. Returns
    /// `None` for an out-of-namespace subject (FR31) or a benign (sub-floor)
    /// signal (so Observer does not flood the operator with non-events).
    ///
    /// A `frame_kind` other than [`SANDBOX_BLOCK_FRAME_KIND`] is itself surfaced
    /// (an unexpected carrier is structurally suspicious), independent of
    /// magnitude.
    pub fn classify_signal(&self, signal: &StructuralSignal) -> Option<ObserverSurface> {
        if !self.in_namespace(&signal.subject) {
            return None;
        }
        // Story 8.10 AC4a — a NaN/Inf magnitude is ITSELF an anomaly (the old
        // `return None` fail-open silently disabled the structural watchdog).
        // Reject-and-flag the non-comparable measurement.
        if !signal.magnitude.is_finite() {
            return Some(ObserverSurface {
                subject: signal.subject.clone(),
                summary: format!(
                    "structural_anomaly_suspect: {} — non-comparable magnitude {} \
                     (NaN/Inf); watchdog cannot compare, flagging",
                    signal.kind.as_tag(),
                    signal.magnitude
                ),
                confidence: NON_COMPARABLE_FLAG_CONFIDENCE,
                anomaly_kind: AnomalyKind::StructuralAnomalySuspect,
            });
        }
        let magnitude = signal.magnitude;
        let unexpected_kind = signal.frame_kind != SANDBOX_BLOCK_FRAME_KIND;
        if magnitude < Self::STRUCTURAL_SUSPECT_FLOOR && !unexpected_kind {
            return None; // benign — below the Spirit-side suspect floor.
        }
        let confidence = (magnitude.clamp(0.0, 1.0)) as f32;
        let mut summary = format!(
            "structural_anomaly_suspect: {} (magnitude {:.3})",
            signal.kind.as_tag(),
            magnitude
        );
        if !signal.detail.is_empty() {
            summary.push_str(&format!(" — {}", signal.detail));
        }
        if unexpected_kind {
            summary.push_str(&format!(
                " [unexpected frame_kind {} ≠ SandboxBlock {}]",
                signal.frame_kind, SANDBOX_BLOCK_FRAME_KIND
            ));
        }
        Some(ObserverSurface {
            subject: signal.subject.clone(),
            // A structural suspect carries at least the floor confidence even
            // when the carrier (not the magnitude) is what is suspicious.
            confidence: if unexpected_kind {
                confidence.max(Self::STRUCTURAL_SUSPECT_FLOOR as f32)
            } else {
                confidence
            },
            summary,
            anomaly_kind: AnomalyKind::StructuralAnomalySuspect,
        })
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn scalar(spirit: &str, tag: &str, value: f64) -> ScalarTapEvent {
        ScalarTapEvent {
            spirit_id: spirit.into(),
            tag: tag.into(),
            value,
            timestamp: 1,
        }
    }

    fn belief_variance_watch() -> WatchThreshold {
        // Mirrors Butler's `belief_variance on_value_above 0.7`; band = [0.55, 0.7).
        WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, 0.15).unwrap()
    }

    fn obs() -> Observer {
        Observer::watching(
            PrincipalScope::from_patterns(["mira"]),
            vec![belief_variance_watch()],
        )
    }

    #[test]
    fn drift_warns_in_band_before_threshold() {
        let o = obs();
        // Below band → no warning.
        assert!(o
            .observe_scalar(&scalar("mira", "belief_variance", 0.40))
            .is_none());
        // In band, pre-threshold → warning.
        let s = o
            .observe_scalar(&scalar("mira", "belief_variance", 0.66))
            .expect("in-band pre-threshold drift warns");
        assert_eq!(s.anomaly_kind, AnomalyKind::DriftEarlyWarning);
        assert_eq!(s.subject, "mira");
        assert!(s.confidence > 0.0 && s.confidence <= 1.0);
        assert!(s.summary.contains("drift early-warning"));
        // Dedup: a second in-band value in the same episode does not re-warn.
        assert!(o
            .observe_scalar(&scalar("mira", "belief_variance", 0.69))
            .is_none());
        // Crossed the threshold → the kernel halt fires; no early-warning.
        assert!(o
            .observe_scalar(&scalar("mira", "belief_variance", 0.78))
            .is_none());
    }

    #[test]
    fn drift_drops_out_of_namespace_emitter() {
        let o = obs();
        // FR31: a value that WOULD warn, but from a non-scoped emitter → dropped.
        assert!(o
            .observe_scalar(&scalar("stranger", "belief_variance", 0.66))
            .is_none());
    }

    #[test]
    fn drift_ignores_unwatched_tag() {
        let o = obs();
        assert!(o
            .observe_scalar(&scalar("mira", "unrelated_metric", 0.66))
            .is_none());
    }

    #[test]
    fn drift_below_direction_warns_falling_toward_threshold() {
        // Halt when value < 0.6 (peer on_value_below); band = (0.6, 0.75].
        let o = Observer::watching(
            PrincipalScope::all(),
            vec![WatchThreshold::new(
                "user_preference_drift",
                0.6,
                DriftDirection::Below,
                0.15,
            )
            .unwrap()],
        );
        // Above band → no warning.
        assert!(o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.9))
            .is_none());
        // In band, pre-threshold → warning.
        let s = o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.66))
            .expect("falling-toward drift warns");
        assert!(s.summary.contains("falling toward"));
        // Crossed below threshold → halt; no early-warning.
        assert!(o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.55))
            .is_none());
    }

    #[test]
    fn structural_suspect_fires_above_floor() {
        let o = obs();
        let sig = StructuralSignal {
            frame_kind: SANDBOX_BLOCK_FRAME_KIND,
            subject: "mira".into(),
            kind: DivergenceKind::FdTableGrowth,
            magnitude: 0.82,
            detail: "fd count 412 vs declared 64".into(),
        };
        let s = o
            .classify_signal(&sig)
            .expect("above-floor structural signal is a suspect");
        assert_eq!(s.anomaly_kind, AnomalyKind::StructuralAnomalySuspect);
        assert!(s.summary.contains("structural_anomaly_suspect"));
        assert!(s.summary.contains("fd-table-growth"));
        assert!((s.confidence - 0.82).abs() < 1e-6);
    }

    #[test]
    fn structural_benign_below_floor_is_no_suspect() {
        let o = obs();
        let sig = StructuralSignal {
            frame_kind: SANDBOX_BLOCK_FRAME_KIND,
            subject: "mira".into(),
            kind: DivergenceKind::SyscallPatternDivergence,
            magnitude: 0.10,
            detail: String::new(),
        };
        assert!(
            o.classify_signal(&sig).is_none(),
            "benign signal is not a suspect"
        );
    }

    #[test]
    fn structural_drops_out_of_namespace_subject() {
        let o = obs();
        let sig = StructuralSignal {
            frame_kind: SANDBOX_BLOCK_FRAME_KIND,
            subject: "stranger".into(),
            kind: DivergenceKind::UnexpectedOutboundIac,
            magnitude: 0.95,
            detail: String::new(),
        };
        assert!(
            o.classify_signal(&sig).is_none(),
            "FR31: out-of-namespace subject dropped"
        );
    }

    #[test]
    fn surface_converts_to_validated_anomaly_notification() {
        let s = ObserverSurface {
            subject: "mira".into(),
            summary: "drift early-warning: scalar 'belief_variance' approaching".into(),
            confidence: 0.73,
            anomaly_kind: AnomalyKind::DriftEarlyWarning,
        };
        let ev = s.to_notification("observer").expect("valid anomaly event");
        match ev {
            NotificationEvent::AnomalyFlagged {
                observer,
                subject,
                confidence,
                ..
            } => {
                assert_eq!(observer, "observer");
                assert_eq!(subject, "mira");
                assert!((confidence - 0.73).abs() < 1e-6);
            }
            _ => panic!("expected AnomalyFlagged"),
        }
    }

    #[test]
    fn surface_serializes_with_required_output_fields() {
        let s = ObserverSurface {
            subject: "mira".into(),
            summary: "structural_anomaly_suspect: fd-table-growth".into(),
            confidence: 0.8,
            anomaly_kind: AnomalyKind::StructuralAnomalySuspect,
        };
        let v = serde_json::to_value(&s).unwrap();
        for field in ["subject", "summary", "confidence", "anomaly_kind"] {
            assert!(
                v.get(field).is_some(),
                "missing required output field {field}"
            );
        }
    }

    #[test]
    fn sandbox_block_discriminator_is_bound_to_the_abi() {
        assert_eq!(SANDBOX_BLOCK_FRAME_KIND, 8);
    }

    #[test]
    fn posture_is_passive_observer() {
        assert_eq!(
            Observer::default().posture(),
            ObserverPosture::PassiveObserver
        );
    }

    #[test]
    fn nan_scalar_is_flagged_not_dropped() {
        // Story 8.10 AC4a — a NaN scalar on a watched tag for an in-namespace
        // subject is reject-and-flagged, NOT silently dropped (the old fail-open).
        let o = obs();
        let s = o
            .observe_scalar(&scalar("mira", "belief_variance", f64::NAN))
            .expect("NaN scalar must surface a non-comparable anomaly flag");
        assert_eq!(s.subject, "mira");
        assert!(s.summary.contains("non-comparable"));
        assert!(s.confidence.is_finite() && (0.0..=1.0).contains(&s.confidence));
        // It must convert to a valid notification (NaN-confidence-rejecting ctor).
        assert!(s.to_notification("observer").is_ok());
    }

    #[test]
    fn inf_scalar_is_flagged_not_dropped() {
        // is_infinite() coverage (AC4a extends the guard beyond is_nan()).
        let o = obs();
        let s = o
            .observe_scalar(&scalar("mira", "belief_variance", f64::INFINITY))
            .expect("Inf scalar must surface a non-comparable anomaly flag");
        assert!(s.summary.contains("non-comparable"));
    }

    #[test]
    fn nan_magnitude_is_flagged_not_dropped() {
        // Story 8.10 AC4a — a NaN magnitude is reject-and-flagged, not dropped.
        let o = obs();
        let sig = StructuralSignal {
            frame_kind: SANDBOX_BLOCK_FRAME_KIND,
            subject: "mira".into(),
            kind: DivergenceKind::FdTableGrowth,
            magnitude: f64::NAN,
            detail: String::new(),
        };
        let s = o
            .classify_signal(&sig)
            .expect("NaN magnitude must surface a non-comparable structural suspect");
        assert_eq!(s.anomaly_kind, AnomalyKind::StructuralAnomalySuspect);
        assert!(s.summary.contains("non-comparable"));
        assert!(s.to_notification("observer").is_ok());
    }

    #[test]
    fn inf_magnitude_is_flagged_not_dropped() {
        let o = obs();
        let sig = StructuralSignal {
            frame_kind: SANDBOX_BLOCK_FRAME_KIND,
            subject: "mira".into(),
            kind: DivergenceKind::FdTableGrowth,
            magnitude: f64::NEG_INFINITY,
            detail: String::new(),
        };
        let s = o
            .classify_signal(&sig)
            .expect("Inf magnitude must surface a non-comparable structural suspect");
        assert!(s.summary.contains("non-comparable"));
    }

    #[test]
    fn watch_threshold_new_rejects_nan_threshold() {
        // Story 8.10 AC4b.
        assert!(matches!(
            WatchThreshold::new("belief_variance", f64::NAN, DriftDirection::Above, 0.15),
            Err(WatchThresholdError::NonFiniteThreshold)
        ));
        assert!(matches!(
            WatchThreshold::new("belief_variance", f64::INFINITY, DriftDirection::Above, 0.15),
            Err(WatchThresholdError::NonFiniteThreshold)
        ));
        assert!(matches!(
            WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, f64::NAN),
            Err(WatchThresholdError::NonFiniteWarnMargin)
        ));
        // A finite non-positive margin is still clamped (not rejected).
        let w = WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, -1.0)
            .expect("finite non-positive margin clamps, not rejects");
        assert!(w.warn_margin > 0.0);
    }

    #[test]
    fn structural_signal_new_validates() {
        // Story 8.10 AC5 — the validating constructor for StructuralSignal.
        assert!(matches!(
            StructuralSignal::new(SANDBOX_BLOCK_FRAME_KIND, "", DivergenceKind::FdTableGrowth, 0.5, ""),
            Err(StructuralSignalError::EmptySubject)
        ));
        assert!(matches!(
            StructuralSignal::new(SANDBOX_BLOCK_FRAME_KIND, "mira", DivergenceKind::FdTableGrowth, f64::NAN, ""),
            Err(StructuralSignalError::NonFiniteMagnitude)
        ));
        assert!(matches!(
            StructuralSignal::new(SANDBOX_BLOCK_FRAME_KIND, "mira", DivergenceKind::FdTableGrowth, 1.5, ""),
            Err(StructuralSignalError::MagnitudeOutOfRange)
        ));
        let sig = StructuralSignal::new(
            SANDBOX_BLOCK_FRAME_KIND,
            "mira",
            DivergenceKind::FdTableGrowth,
            0.82,
            "fd count 412 vs declared 64",
        )
        .expect("valid structural signal");
        assert_eq!(sig.subject, "mira");
        assert!((sig.magnitude - 0.82).abs() < 1e-9);
    }

    #[test]
    fn principal_scope_prefix_wildcard_matches() {
        let scope = PrincipalScope::from_patterns(["worker-*"]);
        assert!(scope.admits("worker-7"), "prefix wildcard matches");
        assert!(scope.admits("worker-"), "prefix wildcard matches empty suffix");
        assert!(!scope.admits("mira"), "prefix wildcard does not match unrelated");
    }

    #[test]
    fn unexpected_frame_kind_surfaces_as_suspect() {
        let o = Observer::watching(PrincipalScope::all(), vec![]);
        let sig = StructuralSignal {
            frame_kind: 42,
            subject: "mira".into(),
            kind: DivergenceKind::SyscallPatternDivergence,
            magnitude: 0.10,
            detail: String::new(),
        };
        let s = o
            .classify_signal(&sig)
            .expect("unexpected frame_kind is a suspect regardless of magnitude");
        assert_eq!(s.anomaly_kind, AnomalyKind::StructuralAnomalySuspect);
        assert!(s.summary.contains("unexpected frame_kind 42"));
        assert!(
            s.confidence >= (Observer::STRUCTURAL_SUSPECT_FLOOR as f32),
            "unexpected-kind confidence carries at least the suspect floor"
        );
    }

    #[test]
    fn drift_below_direction_reset_allows_rewarning() {
        let o = Observer::watching(
            PrincipalScope::all(),
            vec![WatchThreshold::new(
                "user_preference_drift",
                0.6,
                DriftDirection::Below,
                0.15,
            )
            .unwrap()],
        );
        // Safe (above band).
        assert!(o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.90))
            .is_none());
        // In band → warn.
        assert!(
            o.observe_scalar(&scalar("mira", "user_preference_drift", 0.66))
                .is_some()
        );
        // Still in band → dedup.
        assert!(o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.65))
            .is_none());
        // Back to safe (above band) → episode resets.
        assert!(o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.90))
            .is_none());
        // Re-enter band → re-warn (new episode).
        let s = o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.67))
            .expect("re-warning after reset");
        assert!(s.confidence > 0.0, "re-warning carries a real confidence");
    }

    #[test]
    fn drift_below_direction_confidence_value() {
        let o = Observer::watching(
            PrincipalScope::all(),
            vec![WatchThreshold::new(
                "user_preference_drift",
                0.6,
                DriftDirection::Below,
                0.15,
            )
            .unwrap()],
        );
        let s = o
            .observe_scalar(&scalar("mira", "user_preference_drift", 0.66))
            .expect("falling-toward drift warns");
        assert!(
            s.confidence > 0.0 && s.confidence < 1.0,
            "confidence is between 0 and 1: {}",
            s.confidence
        );
    }

    #[test]
    fn with_id_overrides_observer_id() {
        let o = Observer::default().with_id("custom-observer");
        assert_eq!(o.observer_id(), "custom-observer");
    }
}
