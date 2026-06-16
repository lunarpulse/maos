#![forbid(unsafe_code)]

//! Story 7.1 v0.5 binding — deprecation warning channel surface.
//!
//! Spirit code that uses a deprecated ABI surface receives a tagged warning
//! observable via `Ctx::deprecation_warnings()`. The `spirit-test` SDK
//! surfaces these warnings in test output; Story 7.5a's ABI compatibility
//! matrix gate (NFR-Maint-3) consumes them at v1.0 to assert every deprecated
//! surface has a matching `STABILITY.md` entry.
//!
//! At v0.5 the ABI has ZERO deprecations to surface — the channel ships
//! EMPTY-PRESENT. The `Ctx::mock_with_deprecation_warnings(vec![...])`
//! test helper lets `spirit-test` verify the surfacing WORKS even though
//! no real deprecations exist at v0.5.

/// A deprecation warning observable from `Ctx::deprecation_warnings()`.
///
/// Populated by the kernel at hook-fire time from any ABI surface annotated
/// `#[maos_attrs::deprecated_since(version = "0.5", remove_at = "1.0", migration = "...")]`.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::DeprecationWarning;
///
/// const OLD_SEND_WARNING: DeprecationWarning = DeprecationWarning::new(
///     "Ctx::old_send_method",
///     "0.5",
///     "1.0",
///     "use Ctx::new_send_method instead",
/// );
///
/// assert_eq!(OLD_SEND_WARNING.surface, "Ctx::old_send_method");
/// assert_eq!(OLD_SEND_WARNING.since_version, "0.5");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeprecationWarning {
    /// The deprecated surface identifier — e.g., `"Ctx::old_send_method"`.
    pub surface: &'static str,
    /// The version the surface was deprecated in — e.g., `"0.5"`.
    pub since_version: &'static str,
    /// The version the surface is planned for removal in — e.g., `"1.0"`.
    pub planned_removal: &'static str,
    /// Migration hint — e.g., `"use Ctx::new_send_method instead"`.
    pub migration_hint: &'static str,
}

impl DeprecationWarning {
    /// Construct a new deprecation warning.
    pub const fn new(
        surface: &'static str,
        since_version: &'static str,
        planned_removal: &'static str,
        migration_hint: &'static str,
    ) -> Self {
        Self {
            surface,
            since_version,
            planned_removal,
            migration_hint,
        }
    }
}
