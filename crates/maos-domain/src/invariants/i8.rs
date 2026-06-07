//! I8: Cross-Host A2A interactions require explicit consent at both ends.
//!
//! Scoped to the typed intent of the message (not just the channel).
//! Channel-consent does not imply transaction-consent.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3 / v0.5**: `—` (unchanged).
//! - **v0.9**: `runtime` — A2A Gateway rejects frames with intent not in
//!   send-allowlist or accept-allowlist.
//! - **v1.0 / v1.5**: `runtime` (unchanged; promoted to `fuzz` at v1.5
//!   for cross-Host adversarial corpus).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i8::{InvariantI8, A2AIntent, IntentAllowlist};
//!
//! let _marker: InvariantI8 = InvariantI8;
//! let intent = A2AIntent::new("diagnosis-handoff");
//! let mut allowlist = IntentAllowlist::new();
//! allowlist.insert(intent.clone());
//! assert!(allowlist.contains(&A2AIntent::new("diagnosis-handoff")));
//! ```

/// I8 marker type — Cross-Host A2A requires explicit typed-intent consent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI8;

/// Typed intent for A2A consent — the granularity of cross-Host consent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct A2AIntent(String);

/// Maximum byte length of a canonical A2A intent. ADR-012 keeps the intent
/// vocabulary *open* but not unbounded; the canonical-form check rejects
/// pathologically long strings (Story 8.7 / AC5).
/// Maximum byte length of a canonical A2A intent string.
///
/// Chosen as 128 bytes (arbitrary but generous) to prevent unbounded memory
/// pressure from malicious or misconfigured intent strings on the wire. The
/// bound is documented here for operator visibility; operators with legitimate
/// intents exceeding this should file an ADR-012 revisit request.
pub const MAX_CANONICAL_INTENT_LEN: usize = 128;

impl A2AIntent {
    /// Create a new A2A intent.
    ///
    /// **Free-form by design** — ADR-012 deliberately chooses an open
    /// vocabulary, so this performs no validation. Use [`A2AIntent::parse`] on
    /// paths that want fail-closed canonical-form validation, and
    /// [`A2AIntent::is_canonical`] for hygiene checks.
    pub fn new(intent: impl Into<String>) -> Self {
        Self(intent.into())
    }

    /// Return the intent string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Story 8.7 / AC5 — is this intent in **canonical fine-grained form**?
    ///
    /// Canonical form is the de-facto shape the reference fleet already uses —
    /// `rca-summary`, `diagnosis-handoff:read-only-evidence`,
    /// `code-mutation-directive` — formalized as the grammar
    /// `^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$`: an optional single
    /// `namespace:verb` split, each side a `-`-joined run of non-empty
    /// lowercase-alphanumeric segments, bounded to [`MAX_CANONICAL_INTENT_LEN`]
    /// bytes.
    ///
    /// The 3 coarse band tokens (`highprivilege`/`standard`/`readonly`, the
    /// `IntentClass::a2a_consent_intent_str` projection) are a subset of this
    /// grammar, so a *canonical* intent is exactly the set that can ever match a
    /// frame's consent key (fine-grained OR band-fallback). An allowlist entry
    /// that is NOT canonical can never match any frame — `maos-a2a-core`'s router
    /// emits a `tracing::warn!` for such unreachable entries so the original
    /// "silent never-match" failure mode becomes loud.
    ///
    /// **Story 8.8 note:** an empty string (`A2AIntent::new("")`) fails this
    /// check because the grammar requires at least one character (`+` quantifier).
    /// In the fail-closed router (`A2ARouterCore::consent_decision`), empty
    /// strings are therefore classified as `UnclassifiedReason::NonCanonical`,
    /// NOT `Absent` — they are present-but-invalid, not missing.
    pub fn is_canonical(&self) -> bool {
        let s = self.0.as_str();
        if s.is_empty() || s.len() > MAX_CANONICAL_INTENT_LEN {
            return false;
        }
        // `[a-z0-9]+(-[a-z0-9]+)*` — a `-`-joined run of non-empty
        // lowercase-alphanumeric segments.
        fn is_segment_run(seg: &str) -> bool {
            !seg.is_empty()
                && seg.split('-').all(|run| {
                    !run.is_empty()
                        && run
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                })
        }
        // At most one `namespace:verb` split.
        let mut parts = s.split(':');
        let namespace = parts.next().unwrap_or_default();
        let verb = parts.next();
        if parts.next().is_some() {
            return false; // more than one ':'
        }
        if !is_segment_run(namespace) {
            return false;
        }
        match verb {
            None => true,
            Some(v) => is_segment_run(v),
        }
    }

    /// Story 8.7 / AC5 — parse a string into a **canonical** [`A2AIntent`],
    /// rejecting non-canonical shapes with [`NonCanonicalIntent`].
    ///
    /// Use this on config-load paths that want fail-closed vocabulary
    /// validation; [`A2AIntent::new`] remains the free-form constructor
    /// ADR-012's open vocabulary requires.
    pub fn parse(intent: impl Into<String>) -> Result<Self, NonCanonicalIntent> {
        let candidate = Self(intent.into());
        if candidate.is_canonical() {
            Ok(candidate)
        } else {
            Err(NonCanonicalIntent(candidate.0))
        }
    }
}

/// Story 8.7 / AC5 — [`A2AIntent::parse`] rejection: the string is not in
/// canonical fine-grained form
/// (`^[a-z0-9]+(-[a-z0-9]+)*(:[a-z0-9]+(-[a-z0-9]+)*)?$`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonCanonicalIntent(pub String);

impl std::fmt::Display for NonCanonicalIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "non-canonical A2A intent {:?}: expected lowercase `namespace:verb` \
             (e.g. `diagnosis-handoff:read-only-evidence`) or a band token \
             (`highprivilege`/`standard`/`readonly`)",
            self.0
        )
    }
}

impl std::error::Error for NonCanonicalIntent {}

/// Typed allowlist wrapper around a set of A2A intents.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IntentAllowlist {
    intents: std::collections::BTreeSet<A2AIntent>,
}

impl IntentAllowlist {
    /// Create an empty allowlist.
    pub fn new() -> Self {
        Self {
            intents: std::collections::BTreeSet::new(),
        }
    }

    /// Insert an intent into the allowlist.
    pub fn insert(&mut self, intent: A2AIntent) {
        self.intents.insert(intent);
    }

    /// Check whether the allowlist contains the given intent.
    pub fn contains(&self, intent: &A2AIntent) -> bool {
        self.intents.contains(intent)
    }
}

impl Default for IntentAllowlist {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_allowlist_membership() {
        let mut list = IntentAllowlist::new();
        list.insert(A2AIntent::new("consult"));
        assert!(list.contains(&A2AIntent::new("consult")));
        assert!(!list.contains(&A2AIntent::new("delegate")));
    }

    // ── Story 8.7 / AC5 — canonical-form hygiene ──────────────────────────────

    #[test]
    fn is_canonical_accepts_reference_fleet_shapes() {
        // The de-facto fleet vocabulary + the 3 band tokens.
        for s in [
            "rca-summary",
            "diagnosis-handoff:read-only-evidence",
            "code-mutation-directive",
            "cross-environment-telemetry-query",
            "highprivilege",
            "standard",
            "readonly",
            "a",
            "a1-b2:c3-d4",
        ] {
            assert!(A2AIntent::new(s).is_canonical(), "{s} should be canonical");
        }
    }

    #[test]
    fn is_canonical_rejects_typo_class() {
        // Exactly the "silent never-match" class the warn must catch.
        for s in [
            "",                              // empty
            "Diagnosis-Handoff",            // uppercase
            "diagnosis handoff",            // space
            "diagnosis-handoff:",           // trailing colon (empty verb)
            ":read-only",                   // empty namespace
            "diagnosis--handoff",           // empty segment between dashes
            "-leading-dash",                // leading dash → empty segment
            "trailing-dash-",               // trailing dash → empty segment
            "ns:verb:extra",                // more than one colon
            "snake_case",                   // underscore not allowed
        ] {
            assert!(
                !A2AIntent::new(s).is_canonical(),
                "{s:?} should NOT be canonical"
            );
        }
        // Length bound.
        let too_long = "a".repeat(MAX_CANONICAL_INTENT_LEN + 1);
        assert!(!A2AIntent::new(too_long).is_canonical());
    }

    #[test]
    fn parse_round_trips_canonical_and_rejects_non_canonical() {
        let ok = A2AIntent::parse("diagnosis-handoff:read-only-evidence").expect("canonical");
        assert_eq!(ok.as_str(), "diagnosis-handoff:read-only-evidence");

        let err = A2AIntent::parse("Not Canonical").expect_err("must reject");
        assert!(format!("{err}").contains("non-canonical"));
        // `new` stays free-form (ADR-012 open vocabulary).
        assert_eq!(A2AIntent::new("Not Canonical").as_str(), "Not Canonical");
    }
}
