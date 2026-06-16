//! TraceSink port trait — OTel-agnostic trace seam (Story 9.5b / R2-2).
//!
//! The kernel defines this minimal trait with a no-op default.  The actual
//! OTel implementation lives in `maos-telemetry` (a non-kernel-core
//! workspace crate).  Off-path (default `None`) = no allocation, no span.
//!
//! # Span kinds (AC-1)
//!
//! | Method               | Span kind            | Linkage to parent     |
//! |----------------------|----------------------|-----------------------|
//! | `iac_frame_span`     | `maos.iac_frame`     | Root (MAOS is root)   |
//! | `capability_span`    | `maos.capability`    | `parent_span_id` + shared `trace_id` |
//! | `halt_event`         | `maos.halt`          | `frame_id` attribute correlation |
//!
//! # Safety invariants
//!
//! - **Zero principal nexus** in any span attribute (AC-5 / R2-5).
//! - **No task-locals** for context propagation — parent is explicit (AC-1).
//! - **Attrs built inside the `Option` check** — nothing allocates when off.
//! - **Named `let _guard =` binding** — bare `let _ =` drops immediately.

/// Opaque span context — carries `trace_id` + `span_id` for explicit
/// parent propagation (AC-1: never task-locals).
///
/// The no-op / `None` path uses [`SpanContext::EMPTY`] (all zeroes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanContext {
    /// W3C trace-id: 16 bytes.
    pub trace_id: [u8; 16],
    /// W3C span-id: 8 bytes.
    pub span_id: [u8; 8],
}

impl SpanContext {
    /// Sentinel for the no-op / disabled path.
    pub const EMPTY: Self = Self {
        trace_id: [0; 16],
        span_id: [0; 8],
    };

    /// True when this context is the disabled sentinel.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.trace_id == [0; 16] && self.span_id == [0; 8]
    }
}

/// RAII guard that ends the span on drop.
///
/// Carries the [`SpanContext`] so callers can propagate it to child
/// spans **explicitly** (never via task-locals).
///
/// The no-op guard (returned when the sink is `None`) does nothing on
/// drop and carries [`SpanContext::EMPTY`].
pub struct SpanGuard {
    context: SpanContext,
    /// Boxed drop callback — `None` for the no-op guard.
    _drop: Option<Box<dyn FnOnce() + Send>>,
}

impl SpanGuard {
    /// No-op guard — zero cost, zero allocation (created inline).
    pub const fn noop() -> Self {
        Self {
            context: SpanContext::EMPTY,
            _drop: None,
        }
    }

    /// Construct a live guard with a drop callback.
    pub fn new(context: SpanContext, on_drop: impl FnOnce() + Send + 'static) -> Self {
        Self {
            context,
            _drop: Some(Box::new(on_drop)),
        }
    }

    /// The span's context — propagate to [`TraceSink::capability_span`].
    #[inline]
    pub fn context(&self) -> &SpanContext {
        &self.context
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        if let Some(f) = self._drop.take() {
            f();
        }
    }
}

impl std::fmt::Debug for SpanGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpanGuard")
            .field("context", &self.context)
            .field("live", &self._drop.is_some())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Span attribute structs — per-span-kind allowlists (AC-5 / R2-6)
// ---------------------------------------------------------------------------

/// IAC frame span attributes (AC-5 allowlist).
///
/// Zero principal nexus — `frame_id` is an opaque correlation key,
/// `kind` and `intent` are enum labels, not principal identifiers.
#[derive(Debug, Clone)]
pub struct IacFrameSpanAttrs {
    pub frame_id: [u8; 16],
    pub kind: &'static str,
    pub intent: &'static str,
}

/// Capability invocation span attributes (AC-5 allowlist).
///
/// Zero principal nexus — `scope_label` is the capability scope enum
/// variant name, not a principal identifier.
#[derive(Debug, Clone)]
pub struct CapabilitySpanAttrs {
    pub scope_label: String,
    pub spirit_pid: u32,
}

/// Halt event span attributes (AC-5 allowlist / R2-4).
///
/// Zero principal nexus — the raw `HaltTelemetryEntry.value` is
/// **bucketed to `value_band`** (R2-5: principal-correlatable scalar
/// → coarse band). `frame_id` is the halt→frame correlation key (R2-4).
#[derive(Debug, Clone)]
pub struct HaltSpanAttrs {
    pub halt_id: String,
    pub tag: String,
    pub predicate_kind: String,
    /// Bucketed value — `over` / `at` / `under` (R2-5).
    pub value_band: &'static str,
    pub threshold: Option<f32>,
    /// The causing IAC frame's id (R2-4 — correlation, not parent_span_id).
    pub frame_id: [u8; 16],
}

impl HaltSpanAttrs {
    /// Bucket a raw value relative to its threshold (R2-5).
    ///
    /// Returns `"over"` / `"at"` / `"under"` / `"unknown"`.
    pub fn bucket_value(value: f32, threshold: Option<f32>) -> &'static str {
        match threshold {
            Some(t) => {
                if value > t {
                    "over"
                } else if (value - t).abs() < f32::EPSILON {
                    "at"
                } else {
                    "under"
                }
            }
            None => "unknown",
        }
    }
}

// ---------------------------------------------------------------------------
// TraceSink trait
// ---------------------------------------------------------------------------

/// OTel-agnostic trace sink — the kernel's ONLY dependency for tracing.
///
/// Implementors live outside kernel-core (e.g. `maos-telemetry`).
/// The hot-path seam is `Option<Arc<dyn TraceSink>>`; when `None` the
/// branch is not taken and **no attributes are built** (closure pattern).
///
/// # Contract
///
/// - [`iac_frame_span`][TraceSink::iac_frame_span]: MAOS is the trace root.
/// - [`capability_span`][TraceSink::capability_span]: child of the frame span.
/// - [`halt_event`][TraceSink::halt_event]: post-hoc span (no guard) correlated
///   by `frame_id` attribute, NOT `parent_span_id` (R2-4).
/// - All attribute keys ⊆ per-span-kind allowlist (AC-5).
/// - **No `tokio::task_local!`** for context propagation.
pub trait TraceSink: Send + Sync + 'static {
    /// Start an IAC-frame span.  MAOS originates the trace root.
    ///
    /// The returned [`SpanGuard`] MUST be bound with `let _guard = …`
    /// (named binding); `let _ = …` drops immediately → zero-duration span.
    fn iac_frame_span(&self, attrs: IacFrameSpanAttrs) -> SpanGuard;

    /// Start a capability-invocation span as a **live child** of the
    /// IAC-frame span whose context is `parent`.
    ///
    /// Linkage: `capability.parent_span_id == parent.span_id` AND
    /// shared `trace_id` (AC-1).
    fn capability_span(&self, parent: &SpanContext, attrs: CapabilitySpanAttrs) -> SpanGuard;

    /// Emit a completed halt-event span.  No guard — the span is
    /// opened and immediately closed (post-hoc, halt already committed).
    ///
    /// Linkage to the causing IAC frame is by `frame_id` **attribute
    /// correlation**, NOT `parent_span_id` (R2-4: the frame span is
    /// normally closed by halt time).
    fn halt_event(&self, attrs: HaltSpanAttrs);
}
