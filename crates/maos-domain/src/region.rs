//! Region-pinning domain primitive (Story 9.4b AC-5 / AC-12).
//!
//! A [`Region`] is a **jurisdiction label** (NOT a principal/operator identity
//! — re-ratification D3) that is cryptographically welded into the Transparency
//! Log signing-key derivation and the sealed-export AEAD AAD over in
//! `maos-audit`.  This module owns only the *pure* part of region-pinning: the
//! canonical byte form and the typed errors.  No crypto, no I/O — so the
//! `Region` value type stays reachable by every crate in the dep graph
//! (`maos-cli`, `maos-bin`, `maos-kernel-core`) without dragging a crypto dep
//! into the pure domain core (ADR-010).
//!
//! # AC-12 — the irreversible one
//!
//! The region tag is *welded into the key* (HKDF `info`).  Two spellings of one
//! region deriving two keys is the same unrecoverable failure class as a missed
//! AEAD site: artifacts permanently bound to the wrong-or-unrecoverable region
//! with no re-derivation path.  So the byte representation feeding HKDF MUST be
//! canonicalized **once** and **frozen**:
//!
//! 1. trim surrounding ASCII whitespace,
//! 2. ASCII-lowercase (so `US` and `us` derive the **same** key),
//! 3. validate the result against the frozen grammar `^[a-z0-9-]{2,32}$`.
//!
//! The frozen-encoding identifier is [`Region::ENCODING`] = `"ascii-v1"`.  The
//! grammar is ASCII-only on purpose: it removes the Unicode-normalization /
//! homoglyph surface from the keying material entirely (no
//! `unicode-normalization` dep — NFC is a no-op over `[a-z0-9-]`).

use thiserror::Error;

/// A canonicalized jurisdiction label welded into region-pinned key derivation.
///
/// Construct **only** via [`Region::canonicalize`] — the inner string is always
/// in frozen `ascii-v1` form, so any two [`Region`]s comparing equal derive the
/// identical key (AC-12).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Region(String);

impl Region {
    /// Frozen byte-encoding identifier for the region tag (AC-12).  Bumping this
    /// is an intentional, irreversible re-keying — never change it silently.
    pub const ENCODING: &'static str = "ascii-v1";

    /// Minimum canonical length (grammar `{2,32}`).
    pub const MIN_LEN: usize = 2;
    /// Maximum canonical length (grammar `{2,32}`).
    pub const MAX_LEN: usize = 32;

    /// Canonicalize an arbitrary operator-supplied tag into frozen `ascii-v1`
    /// form, or reject it with [`RegionError::ERegionTagInvalid`].
    ///
    /// Normalizes (trim + ASCII-lowercase) *then* validates, so `"  US-East "`
    /// and `"us-east"` both canonicalize to `"us-east"` and key identically.
    pub fn canonicalize(raw: &str) -> Result<Self, RegionError> {
        let trimmed = raw.trim_matches(|c: char| c.is_ascii_whitespace());
        let lowered = trimmed.to_ascii_lowercase();

        let len = lowered.len();
        if len < Self::MIN_LEN || len > Self::MAX_LEN {
            return Err(RegionError::ERegionTagInvalid {
                tag: raw.to_string(),
                reason: "length must be 2..=32 after trim+lowercase (frozen ascii-v1 grammar)",
            });
        }
        // `len` is the byte length; for the ASCII grammar below it equals the
        // char count.  A non-ASCII byte makes a char that fails `is_ascii_*`,
        // so the per-char check rejects it (and multi-byte chars are caught).
        if !lowered
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(RegionError::ERegionTagInvalid {
                tag: raw.to_string(),
                reason: "only [a-z0-9-] permitted (frozen ascii-v1 grammar)",
            });
        }
        Ok(Region(lowered))
    }

    /// Resolve the operator home region with unified precedence.
    ///
    /// Priority: `MAOS_REGION_HOME` env var → `disk_tag` (from `operator.toml`
    /// `[region].home_region`) → `None` (pinning disabled).
    ///
    /// Returns `Err(RegionError::ERegionTagInvalid)` on an invalid tag instead
    /// of silently disabling — callers decide whether to fail-closed or warn.
    pub fn resolve_home(disk_tag: Option<&str>) -> Result<Option<Region>, RegionError> {
        // 1. Base: operator.toml value (if any).
        let base = match disk_tag {
            Some(tag) if !tag.trim().is_empty() => Some(Region::canonicalize(tag)?),
            _ => None,
        };

        // 2. Env override (highest priority).
        match std::env::var("MAOS_REGION_HOME") {
            Ok(tag) if tag.trim().is_empty() => Ok(None), // explicitly empty disables
            Ok(tag) => Ok(Some(Region::canonicalize(&tag)?)),
            Err(_) => Ok(base), // env not set, fall through to disk
        }
    }

    /// Resolve the operator home region from the `MAOS_REGION_HOME` env var
    /// only (no `operator.toml` disk lookup).
    ///
    /// Convenience shorthand for `resolve_home(None)`.  Prefer
    /// [`resolve_home`](Self::resolve_home) in production paths so
    /// `operator.toml` is also respected.
    pub fn home_from_env() -> Result<Option<Region>, RegionError> {
        Self::resolve_home(None)
    }

    /// The canonical tag as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The canonical bytes fed (with a domain separator, in `maos-audit`) into
    /// HKDF `info`.  Stable across the frozen [`Region::ENCODING`].
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Region-pinning typed errors (Story 9.4b AC-5).  Both variants are
/// auto-discovered by `cargo xtask error-catalog-check` (maos-domain/src is in
/// `scan_dirs`); they MUST have matching entries in `xtask/error-catalog.toml`.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum RegionError {
    /// Foreign-region data detected on a read/verify/enforcement path — the
    /// fail-closed region boundary (re-ratification R1; NFR-Comp-4 / PIPL §40).
    /// `severity=security`, `recovery_class=reject`, `kernel`, since `1.0.0`.
    #[error("region violation: {detail} (expected region '{expected}', found '{found}')")]
    ERegionViolation {
        /// The configured home region the artifact was expected to bind to.
        expected: String,
        /// The region the artifact actually bound to (or `"<unverifiable>"`).
        found: String,
        /// Where the violation was detected (entry-point / verify site).
        detail: &'static str,
    },

    /// An operator-supplied region tag failed the frozen `ascii-v1` grammar
    /// (AC-12) — rejected at the config / write-entry boundary before any key
    /// derivation. `severity=policy`, `recovery_class=fix_config`, `kernel`.
    #[error("invalid region tag '{tag}': {reason}")]
    ERegionTagInvalid {
        /// The raw rejected tag (pre-canonicalization).
        tag: String,
        /// Why it was rejected.
        reason: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialize env-mutating tests to avoid `set_var`/`remove_var` UB under
    /// parallel test execution (Rust ≥1.66).  Mirrors `ENV_LOCK` in
    /// `crates/maos-kernel-core/src/security/operator_config.rs`.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // --- AC-12 canonicalization (the irreversible contract) ---

    #[test]
    fn canonicalize_accepts_simple_lowercase() {
        let r = Region::canonicalize("eu").unwrap();
        assert_eq!(r.as_str(), "eu");
        assert_eq!(r.as_bytes(), b"eu");
    }

    #[test]
    fn canonicalize_accepts_hyphenated_and_digits() {
        assert_eq!(
            Region::canonicalize("us-east-1").unwrap().as_str(),
            "us-east-1"
        );
        assert_eq!(
            Region::canonicalize("ap-northeast-2").unwrap().as_str(),
            "ap-northeast-2"
        );
    }

    #[test]
    fn canonicalize_lowercases_so_two_spellings_key_identically() {
        // The AC-12 failure class: two spellings must NOT derive two keys.
        let a = Region::canonicalize("US-EAST").unwrap();
        let b = Region::canonicalize("us-east").unwrap();
        let c = Region::canonicalize("Us-East").unwrap();
        assert_eq!(a, b);
        assert_eq!(b, c);
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn canonicalize_trims_surrounding_whitespace() {
        assert_eq!(Region::canonicalize("  eu \t\n").unwrap().as_str(), "eu");
        assert_eq!(
            Region::canonicalize("  US-EAST  ").unwrap(),
            Region::canonicalize("us-east").unwrap()
        );
    }

    #[test]
    fn canonicalize_rejects_too_short() {
        assert!(matches!(
            Region::canonicalize("e"),
            Err(RegionError::ERegionTagInvalid { .. })
        ));
        assert!(matches!(
            Region::canonicalize(""),
            Err(RegionError::ERegionTagInvalid { .. })
        ));
        // whitespace-only collapses to empty after trim -> too short
        assert!(matches!(
            Region::canonicalize("   "),
            Err(RegionError::ERegionTagInvalid { .. })
        ));
    }

    #[test]
    fn canonicalize_rejects_too_long() {
        let long = "a".repeat(33);
        assert!(matches!(
            Region::canonicalize(&long),
            Err(RegionError::ERegionTagInvalid { .. })
        ));
        // exactly 32 is allowed
        assert!(Region::canonicalize(&"a".repeat(32)).is_ok());
    }

    #[test]
    fn canonicalize_rejects_disallowed_ascii() {
        for bad in [
            "eu_west", "eu.west", "eu/west", "eu west", "eu:west", "eu;west",
        ] {
            assert!(
                matches!(
                    Region::canonicalize(bad),
                    Err(RegionError::ERegionTagInvalid { .. })
                ),
                "expected reject for {bad:?}"
            );
        }
    }

    #[test]
    fn canonicalize_rejects_non_ascii_and_homoglyphs() {
        // Cyrillic 'е' (U+0435) looks like ASCII 'e' — must be rejected so the
        // keying material has zero homoglyph surface.
        assert!(matches!(
            Region::canonicalize("\u{0435}u"),
            Err(RegionError::ERegionTagInvalid { .. })
        ));
        // Full-width latin, emoji, NUL, control chars.
        for bad in ["ＵＳ", "eu\u{0}", "eu\u{007f}", "café"] {
            assert!(
                matches!(
                    Region::canonicalize(bad),
                    Err(RegionError::ERegionTagInvalid { .. })
                ),
                "expected reject for {bad:?}"
            );
        }
    }

    #[test]
    fn canonicalize_is_idempotent() {
        let once = Region::canonicalize("US-East-1").unwrap();
        let twice = Region::canonicalize(once.as_str()).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn encoding_id_is_frozen() {
        // Tripwire: changing this constant is an intentional irreversible
        // re-key. If this assert is edited, AC-12 review is mandatory.
        assert_eq!(Region::ENCODING, "ascii-v1");
    }

    #[test]
    fn home_from_env_resolves_and_canonicalizes() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("MAOS_REGION_HOME", "US-EAST-1");
        assert_eq!(
            Region::home_from_env()
                .unwrap()
                .map(|r| r.as_str().to_string()),
            Some("us-east-1".to_string()),
        );
        std::env::set_var("MAOS_REGION_HOME", "  ");
        assert!(Region::home_from_env().unwrap().is_none());
        std::env::set_var("MAOS_REGION_HOME", "eu_west"); // invalid grammar
        assert!(matches!(
            Region::home_from_env(),
            Err(RegionError::ERegionTagInvalid { .. }),
        ));
        std::env::remove_var("MAOS_REGION_HOME");
        assert!(Region::home_from_env().unwrap().is_none());
    }

    #[test]
    fn resolve_home_env_overrides_disk() {
        let _guard = ENV_LOCK.lock().unwrap();
        // env wins over disk_tag
        std::env::set_var("MAOS_REGION_HOME", "eu");
        assert_eq!(
            Region::resolve_home(Some("us-west"))
                .unwrap()
                .unwrap()
                .as_str(),
            "eu",
        );
        // env unset: disk_tag used
        std::env::remove_var("MAOS_REGION_HOME");
        assert_eq!(
            Region::resolve_home(Some("us-west"))
                .unwrap()
                .unwrap()
                .as_str(),
            "us-west",
        );
        // both absent: None
        assert!(Region::resolve_home(None).unwrap().is_none());
    }

    #[test]
    fn region_error_variants_are_distinct() {
        let v = RegionError::ERegionViolation {
            expected: "eu".into(),
            found: "us".into(),
            detail: "test",
        };
        let i = RegionError::ERegionTagInvalid {
            tag: "EU_WEST".into(),
            reason: "test",
        };
        assert_ne!(v, i);
    }
}
