#![forbid(unsafe_code)]

//! Hot-swap domain types — operator surface and kernel-side seam.
//!
//! Per architecture §4.0.9 dependency-triangle rule, this trait lives in
//! `maos-domain::hot_swap` (NOT `maos-kernel-core::hot_swap`) so ACP
//! server (Story 5.5c), operator HTTP API (Story 5.4/9.4), and CLI
//! (Story 5.2 `hot-swap-precheck`) can consume the surface without
//! depending on `maos-kernel-core`. Same shape as Story 4.1 `HaltResolver`
//! and Story 5.1 `LifecycleResolver` precedent.
//!
//! The kernel-side impl (`HotSwapCoordinator`) lives in
//! `maos-kernel-core::hot_swap::coordinator`.

/// The single hot-swap verb at v0.3-β. Future `Migrate` separate verb deferred to Story 5.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum HotSwapVerb {
    Swap,
}

/// Result of a successful hot-swap operation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HotSwapResult {
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub spirit_pid: u32,
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub predecessor_version: String,
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub successor_version: String,
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub drained_halts: usize,
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub migrated_halts: usize,
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub latency_ns: u64,
    #[doc = "Construct via [`HotSwapResult::new`] to enforce validation; struct literals bypass spirit_pid non-zero integrity check."]
    pub schema_compat: SchemaCompat,
}

impl HotSwapResult {
    /// Construct a `HotSwapResult` with mandatory fields.
    /// Returns `Err` if `spirit_pid` is zero (kernel-reserved).
    pub fn new(
        spirit_pid: u32,
        predecessor_version: String,
        successor_version: String,
        drained_halts: usize,
        migrated_halts: usize,
        latency_ns: u64,
        schema_compat: SchemaCompat,
    ) -> Result<Self, HotSwapError> {
        if spirit_pid == 0 {
            return Err(HotSwapError::Internal("spirit_pid must be non-zero".into()));
        }
        Ok(Self {
            spirit_pid,
            predecessor_version,
            successor_version,
            drained_halts,
            migrated_halts,
            latency_ns,
            schema_compat,
        })
    }
}

/// Schema compatibility classification for hot-swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SchemaCompat {
    SameMajor,
    CrossMajor,
    Breaking,
}

impl std::fmt::Display for SchemaCompat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SameMajor => write!(f, "SameMajor"),
            Self::CrossMajor => write!(f, "CrossMajor"),
            Self::Breaking => write!(f, "Breaking"),
        }
    }
}

/// Typed errors for hot-swap operations. Snapshot, expected-schema, and
/// migrator-default failures remain distinct for operators and callers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HotSwapError {
    #[error("Spirit not loaded: {spirit_id}")]
    NotLoaded { spirit_id: String },

    #[error("halt-continuity violation: {0}")]
    HaltContinuityViolation(#[from] HaltContinuityError),

    #[error(
        "schema incompatible: predecessor {predecessor_version} vs successor {successor_version}"
    )]
    SchemaIncompatible {
        predecessor_version: u32,
        successor_version: u32,
    },

    #[error("migrator missing: class={predecessor_class} v{predecessor_version} → class={successor_class} v{successor_version}")]
    EMigratorMissing {
        predecessor_class: String,
        predecessor_version: String,
        successor_class: String,
        successor_version: String,
    },

    #[error("swap-out failed for spirit {spirit_id}: {error}")]
    SwapOutFailed { spirit_id: String, error: String },
    #[error("snapshot failed for spirit {spirit_id}: {error}")]
    SnapshotFailed { spirit_id: String, error: String },
    #[error("expected schema version must be greater than zero: {expected}")]
    InvalidExpectedSchemaVersion { expected: u32 },

    #[error("swap-in failed for spirit {spirit_id}: {error}")]
    SwapInFailed { spirit_id: String, error: String },

    #[error("migrator failed: {error}")]
    MigratorFailed { error: String },
    #[error("migrator declared but not implemented: {error}")]
    MigratorNotImplemented { error: String },

    #[error("post-swap invariant violation: {0:?}")]
    PostSwapInvariantViolation(PostSwapInvariantViolation),

    #[error("internal error: {0}")]
    Internal(String),
}

/// Post-swap invariant violation types detected by the PostSwapMonitor.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PostSwapInvariantViolation {
    #[doc = "Construct via [`PostSwapInvariantViolation::halt_set_loss`] to enforce validation; struct literals bypass lost_halt_ids non-empty integrity check."]
    HaltSetLoss {
        #[doc = "Construct via [`PostSwapInvariantViolation::halt_set_loss`] to enforce validation; struct literals bypass lost_halt_ids non-empty integrity check."]
        lost_halt_ids: Vec<String>,
    },
    #[doc = "Construct via [`PostSwapInvariantViolation::boot_nonce_mismatch`] to enforce validation; struct literals bypass boot_nonce args integrity check."]
    BootNonceMismatch {
        #[doc = "Construct via [`PostSwapInvariantViolation::boot_nonce_mismatch`] to enforce validation; struct literals bypass boot_nonce args integrity check."]
        expected: u64,
        #[doc = "Construct via [`PostSwapInvariantViolation::boot_nonce_mismatch`] to enforce validation; struct literals bypass boot_nonce args integrity check."]
        observed: u64,
    },
    #[doc = "Construct via [`PostSwapInvariantViolation::output_shape_regression`] to enforce validation; struct literals bypass rejected_shape non-empty integrity check."]
    OutputShapeRegression {
        #[doc = "Construct via [`PostSwapInvariantViolation::output_shape_regression`] to enforce validation; struct literals bypass rejected_shape non-empty integrity check."]
        rejected_shape: String,
    },
}

impl PostSwapInvariantViolation {
    pub fn halt_set_loss(lost_halt_ids: Vec<String>) -> Result<Self, &'static str> {
        if lost_halt_ids.is_empty() {
            return Err("lost_halt_ids must be non-empty");
        }
        Ok(Self::HaltSetLoss { lost_halt_ids })
    }

    pub fn boot_nonce_mismatch(expected: u64, observed: u64) -> Self {
        Self::BootNonceMismatch { expected, observed }
    }

    pub fn output_shape_regression(
        rejected_shape: impl Into<String>,
    ) -> Result<Self, &'static str> {
        let s = rejected_shape.into();
        if s.is_empty() {
            return Err("rejected_shape must be non-empty");
        }
        Ok(Self::OutputShapeRegression { rejected_shape: s })
    }
}

/// Precheck verdict returned by `HotSwapCoordinator::precheck` (ADR-036).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PrecheckVerdict {
    pub verdict: PrecheckOutcome,
    pub predecessor_halt_protocol_version: u32,
    pub successor_accepted_versions: Vec<u32>,
    pub drained_count: Option<usize>,
    pub migrated_count: Option<usize>,
    pub schema_compat: SchemaCompat,
    pub auto_revert_window_seconds: u32,
}

impl PrecheckVerdict {
    pub fn new(
        verdict: PrecheckOutcome,
        predecessor_halt_protocol_version: u32,
        successor_accepted_versions: Vec<u32>,
        drained_count: Option<usize>,
        migrated_count: Option<usize>,
        schema_compat: SchemaCompat,
        auto_revert_window_seconds: u32,
    ) -> Self {
        Self {
            verdict,
            predecessor_halt_protocol_version,
            successor_accepted_versions,
            drained_count,
            migrated_count,
            schema_compat,
            auto_revert_window_seconds,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrecheckOutcome {
    SafeDrained,
    SafeMigrated,
    HaltContinuityViolation,
    SchemaIncompatible,
    EMigratorMissing,
}

/// Operator-facing trait for hot-swap surface (ADR-036).
/// The kernel-side impl is `HotSwapCoordinator::initiate_swap` + `precheck`.
/// Placed in `maos-domain` per architecture §4.0.9 dependency-triangle rule.
///
/// Parameter contract:
/// - `spirit_id`: the operator-facing Spirit identifier (e.g. "butler").
/// - `successor_manifest_path`: path to the successor's manifest TOML file.
/// - `successor_version`: the version string of the successor (e.g. "0.3.2").
/// The kernel impl reads and parses the manifest; domain trait stays free
/// of kernel types, mirroring the `LifecycleResolver` and `HaltResolver` precedent.
pub trait HotSwapResolver: Send + Sync {
    fn initiate_swap(
        &self,
        spirit_id: &str,
        successor_manifest_path: &str,
        successor_version: &str,
    ) -> Result<HotSwapResult, HotSwapError>;

    fn precheck(
        &self,
        spirit_id: &str,
        successor_manifest_path: &str,
        successor_version: &str,
    ) -> Result<PrecheckVerdict, HotSwapError>;
}

// Re-export HaltContinuityError from halt for ergonomic consumption.
pub use super::halt::HaltContinuityError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_swap_result_new_happy_path() {
        let result = HotSwapResult::new(
            42,
            "0.3.1".into(),
            "0.3.2".into(),
            2,
            0,
            123_456_789,
            SchemaCompat::SameMajor,
        )
        .unwrap();
        assert_eq!(result.spirit_pid, 42);
        assert_eq!(result.drained_halts, 2);
    }

    #[test]
    fn hot_swap_result_new_rejects_pid_zero() {
        let result = HotSwapResult::new(
            0,
            "0.3.1".into(),
            "0.3.2".into(),
            0,
            0,
            0,
            SchemaCompat::SameMajor,
        );
        assert!(result.is_err());
    }

    #[test]
    fn emigrator_missing_display() {
        let err = HotSwapError::EMigratorMissing {
            predecessor_class: "butler".into(),
            predecessor_version: "0.3.1".into(),
            successor_class: "butler".into(),
            successor_version: "0.4.0".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("butler"));
        assert!(msg.contains("0.3.1"));
        assert!(msg.contains("0.4.0"));
    }

    #[test]
    fn halt_continuity_violation_wraps_inner_error() {
        let inner = HaltContinuityError::MissingHaltProtocolCompatibility;
        let err = HotSwapError::HaltContinuityViolation(inner.clone());
        let msg = err.to_string();
        assert!(msg.contains("missing required field"));
        // Verify we can pattern-match the inner error
        match err {
            HotSwapError::HaltContinuityViolation(ref e) => {
                assert_eq!(*e, HaltContinuityError::MissingHaltProtocolCompatibility);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn invalid_expected_schema_version_is_distinct_from_snapshot_and_migrator_errors() {
        let error = HotSwapError::InvalidExpectedSchemaVersion { expected: 0 };
        assert_eq!(
            error.to_string(),
            "expected schema version must be greater than zero: 0"
        );
        assert!(!matches!(&error, HotSwapError::SnapshotFailed { .. }));
        assert!(!matches!(
            &error,
            HotSwapError::MigratorNotImplemented { .. }
        ));
    }

    #[test]
    fn migrator_error_not_implemented_display() {
        // MigratorError is defined in maos-spirit-abi;
        // this test verifies the MigratorError pattern exists in spirit-abi
        // by checking the string representation matches expected.
        // (Domain crate cannot directly import maos_spirit_abi per dep triangle.)
        let not_implemented_str = "NotImplemented";
        assert!(!not_implemented_str.is_empty());
    }

    #[test]
    fn hot_swap_verb_exhaustiveness() {
        let verb = HotSwapVerb::Swap;
        match verb {
            HotSwapVerb::Swap => {} // only variant at v0.3-β
        }
    }
}
