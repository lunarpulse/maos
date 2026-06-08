#![forbid(unsafe_code)]

//! Epistemic-scalar **write** port (Story 8.10 AC1).
//!
//! This is the cognitive-Spirit *write* dual of the read-only hexagonal
//! ports (`LogRecallPort`, `MemoryManagerPort`, …): a cognitive Spirit
//! (Butler) computes an epistemic scalar in `on_idle` and needs to drive
//! it through the kernel's universal-arithmetic policy path so the
//! `[epistemic_policy]` halt can fire. The frozen ABI `Ctx` exposes no
//! scalar-write surface (`crates/maos-spirit-abi/src/ctx.rs`), so the
//! Spirit depends on THIS trait — not on `maos-kernel-core` — and the
//! kernel-backed adapter is injected at construction time.
//!
//! ## Zero kernel KLOC
//!
//! The trait returns the EXISTING domain [`crate::halt::HaltReceipt`]
//! (NOT a kernel-core type), so no kernel symbol crosses the boundary
//! and the cognitive Spirit's lib stays kernel-free. The kernel-backed
//! adapter (which wraps `WorkingMemoryOrchestrator::process_scalar_write`)
//! lives in a non-kernel crate as a dev-dep newtype per the orphan rule.

use crate::halt::HaltReceipt;

/// Error surfaced when an epistemic-scalar write cannot reach the policy path.
#[derive(Debug, thiserror::Error)]
pub enum ScalarPortError {
    /// The kernel-backed scalar-write/policy-eval/halt pipeline failed.
    #[error("epistemic scalar-write backend error: {0}")]
    Backend(String),
}

/// The cognitive-Spirit epistemic-scalar **write** port.
///
/// `Send + Sync` is **load-bearing** (NOT optional): a cognitive Spirit
/// (e.g. Butler) is `Send + Sync` and holds an `Arc<dyn EpistemicScalarPort>`;
/// dropping the supertrait would make the Spirit's `#[spirit]` vtable fail
/// to compile.
pub trait EpistemicScalarPort: Send + Sync {
    /// Class: supervision
    ///
    /// Write the Spirit's assessed epistemic scalar through the kernel's
    /// policy path. The kernel performs the universal-arithmetic comparison
    /// against the manifest `[epistemic_policy]` and returns
    /// `Some(HaltReceipt)` when the predicate fires, `None` otherwise.
    ///
    /// Primitives (`tag` / `value` / `derived_from`) match
    /// `process_scalar_write` and the `ScalarTapEvent` shape exactly — the
    /// kernel never receives a bespoke domain newtype for these.
    fn write_scalar(
        &self,
        spirit_pid: u32,
        spirit_id: &str,
        tag: &str,
        value: f64,
        derived_from: &str,
    ) -> Result<Option<HaltReceipt>, ScalarPortError>;
}
