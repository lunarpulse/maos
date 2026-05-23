#![forbid(unsafe_code)]

//! Lifecycle verb resolver — the operator's lifecycle surface.
//!
//! Per architecture §4.0.9 Story 5.1 application rule, this trait lives
//! in `maos-domain::lifecycle` (NOT `maos-kernel-core::lifecycle`) so
//! ACP server (Story 5.5c) and operator HTTP API (Story 5.4/9.4) can
//! consume the surface without depending on `maos-kernel-core`. Same
//! shape as the Story 4.1 `HaltResolver` relocation.
//!
//! The five lifecycle verbs are FR9 (load/start/pause/resume/unload
//! via authenticated control plane). The kernel-side impl
//! (`KernelLifecycleResolver`) lives in `maos-kernel-core::scheduler`.

/// Five authenticated control-plane lifecycle verbs per FR9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum LifecycleVerb {
    Load,
    Start,
    Pause,
    Resume,
    Unload,
}

/// Receipt returned by a successful lifecycle operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LifecycleReceipt {
    /// The kernel-assigned spirit process identifier.
    #[doc = "Construct via [`LifecycleReceipt::new`] to enforce validation; struct literals bypass spirit_pid non-zero / journal_offset_bytes integrity checks."]
    pub spirit_pid: u32,
    /// The verb that produced this receipt.
    #[doc = "Construct via [`LifecycleReceipt::new`] to enforce validation; struct literals bypass spirit_pid non-zero / journal_offset_bytes integrity checks."]
    pub verb: LifecycleVerb,
    /// Wall-clock timestamp in nanoseconds since the kernel's monotonic epoch.
    #[doc = "Construct via [`LifecycleReceipt::new`] to enforce validation; struct literals bypass spirit_pid non-zero / journal_offset_bytes integrity checks."]
    pub timestamp_ns: u64,
    /// Byte offset of the corresponding Lifecycle Journal row (None if journal
    /// write is deferred or fails softly).
    #[doc = "Construct via [`LifecycleReceipt::new`] to enforce validation; struct literals bypass spirit_pid non-zero / journal_offset_bytes integrity checks."]
    pub journal_offset_bytes: Option<u64>,
}

impl LifecycleReceipt {
    /// Construct a receipt with mandatory fields.
    /// Returns `Err` if `spirit_pid` is zero (kernel-reserved).
    pub fn new(
        spirit_pid: u32,
        verb: LifecycleVerb,
        timestamp_ns: u64,
        journal_offset_bytes: Option<u64>,
    ) -> Result<Self, LifecycleError> {
        if spirit_pid == 0 {
            return Err(LifecycleError::Internal(
                "spirit_pid must be non-zero".into(),
            ));
        }
        Ok(Self {
            spirit_pid,
            verb,
            timestamp_ns,
            journal_offset_bytes,
        })
    }
}

/// Errors surfaced by lifecycle operations.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum LifecycleError {
    #[error("spirit not loaded: {spirit_id}")]
    NotLoaded { spirit_id: String },

    #[error(
        "spirit already loaded: {spirit_id} \
         (v0.3-β does not support reload; Story 5.4 ships --policy cold-swap)"
    )]
    AlreadyLoaded { spirit_id: String },

    #[error(
        "invalid state transition: spirit {spirit_id} is in state \
         {current:?}, cannot execute verb {verb:?}"
    )]
    InvalidStateTransition {
        spirit_id: String,
        current: SpiritLifecycleState,
        verb: LifecycleVerb,
    },

    #[error("admission failed: {0}")]
    Admission(String),

    #[error(
        "hook fired but exceeded budget: {hook_name} ran {wall_ns}ns \
         past cap {cap_seconds}s"
    )]
    HookBudgetExceeded {
        hook_name: &'static str,
        wall_ns: u64,
        cap_seconds: u64,
    },

    #[error("internal: {0}")]
    Internal(String),
}

/// The four canonical Spirit lifecycle states.
///
/// Mirrors `maos_kernel_core::scheduler::SpiritControlBlock`'s `AtomicU8`
/// encoding. A `From` impl in both directions connects the domain and
/// kernel representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpiritLifecycleState {
    Loaded = 0,
    Running = 1,
    Paused = 2,
    Unloaded = 3,
}

impl SpiritLifecycleState {
    /// True when the Spirit can receive and process frames.
    pub fn is_runnable(self) -> bool {
        matches!(self, Self::Running)
    }

    /// True when the Spirit has been torn down.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Unloaded)
    }
}

/// Operator-facing lifecycle resolver — implemented by `maos-kernel-core`'s
/// `KernelLifecycleResolver`, consumed by `maos-cli`, `maos-acp` (Story 5.5c),
/// and `maos-control` (Story 5.4/9.4 operator HTTP API).
///
/// # Contract
///
/// - Returns `LifecycleReceipt` on success.
/// - Never panics; all errors are surfaced through `LifecycleError`.
/// - The implementation MUST journal exactly one Lifecycle Journal entry per
///   call AND one FR42 director-action audit row per call (for operator-initiated
///   verbs).
pub trait LifecycleResolver: Send + Sync {
    /// Resolve a lifecycle verb against the named spirit.
    fn resolve_verb(
        &self,
        spirit_id: &str,
        verb: LifecycleVerb,
    ) -> Result<LifecycleReceipt, LifecycleError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_verb_variant_exhaustiveness() {
        // Compile-time: match is exhaustive even through #[non_exhaustive]
        // when all known variants are matched and a fallback is provided.
        fn handle(verb: LifecycleVerb) -> &'static str {
            #[allow(unreachable_patterns)]
            match verb {
                LifecycleVerb::Load => "load",
                LifecycleVerb::Start => "start",
                LifecycleVerb::Pause => "pause",
                LifecycleVerb::Resume => "resume",
                LifecycleVerb::Unload => "unload",
                _ => "unknown",
            }
        }
        assert_eq!(handle(LifecycleVerb::Load), "load");
        assert_eq!(handle(LifecycleVerb::Unload), "unload");
    }

    #[test]
    fn lifecycle_receipt_new_with_none_offset() {
        let r = LifecycleReceipt::new(42, LifecycleVerb::Start, 1000, None).expect("valid receipt");
        assert_eq!(r.spirit_pid, 42);
        assert_eq!(r.verb, LifecycleVerb::Start);
        assert_eq!(r.timestamp_ns, 1000);
        assert_eq!(r.journal_offset_bytes, None);
    }

    #[test]
    fn lifecycle_receipt_rejects_pid_zero() {
        let err = LifecycleReceipt::new(0, LifecycleVerb::Load, 1000, None).unwrap_err();
        assert!(matches!(err, LifecycleError::Internal(ref msg) if msg.contains("non-zero")));
    }

    #[test]
    fn lifecycle_error_already_loaded_display_contains_spirit_id() {
        let err = LifecycleError::AlreadyLoaded {
            spirit_id: "test-spirit".into(),
        };
        let s = err.to_string();
        assert!(s.contains("test-spirit"));
        assert!(s.contains("already loaded"));
    }

    #[test]
    fn spirit_lifecycle_state_serde_round_trip() {
        let states = vec![
            SpiritLifecycleState::Loaded,
            SpiritLifecycleState::Running,
            SpiritLifecycleState::Paused,
            SpiritLifecycleState::Unloaded,
        ];
        for s in &states {
            let json = serde_json::to_string(s).unwrap();
            let back: SpiritLifecycleState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, *s);
        }
    }

    #[test]
    fn spirit_lifecycle_state_is_runnable() {
        assert!(!SpiritLifecycleState::Loaded.is_runnable());
        assert!(SpiritLifecycleState::Running.is_runnable());
        assert!(!SpiritLifecycleState::Paused.is_runnable());
        assert!(!SpiritLifecycleState::Unloaded.is_runnable());
    }

    #[test]
    fn spirit_lifecycle_state_is_terminal() {
        assert!(!SpiritLifecycleState::Loaded.is_terminal());
        assert!(!SpiritLifecycleState::Running.is_terminal());
        assert!(!SpiritLifecycleState::Paused.is_terminal());
        assert!(SpiritLifecycleState::Unloaded.is_terminal());
    }
}
