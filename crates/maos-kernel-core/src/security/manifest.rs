#![forbid(unsafe_code)]

//! Manifest `[sandbox]` and `[resources]` section parsing.
//!
//! Story 1b.3 owns these two sections end-to-end. Story 1b.5c composes
//! them into the full manifest parser.

use maos_domain::invariants::i9::SandboxTier;

/// Error raised during manifest section parsing or validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    #[error("sandbox tier parse failed: {0}")]
    TierParse(String),
    #[error("resource cap out of range: {field} = {value}")]
    CapOutOfRange { field: String, value: u32 },
    #[error("TOML deserialization failed: {0}")]
    Toml(String),
}

/// The `[sandbox]` manifest section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SandboxConfig {
    pub tier: SandboxTier,
}

impl SandboxConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSandboxConfig = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        Ok(SandboxConfig { tier: raw.tier })
    }
}

/// The `[resources]` manifest section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceCaps {
    pub cpu_max_pct: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub fd_max: Option<u32>,
}

impl ResourceCaps {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawResourceCaps = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

/// Resolved resource caps — strictest-of (manifest, operator-policy).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedCaps {
    pub cpu_max_pct: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub fd_max: Option<u32>,
}

/// Take the stricter (lower/None-patched) of two cap sources.
pub fn resolve_caps(manifest: &ResourceCaps, operator: &ResourceCaps) -> ResolvedCaps {
    ResolvedCaps {
        cpu_max_pct: strictest_opt(manifest.cpu_max_pct, operator.cpu_max_pct),
        memory_max_mb: strictest_opt(manifest.memory_max_mb, operator.memory_max_mb),
        fd_max: strictest_opt(manifest.fd_max, operator.fd_max),
    }
}

fn strictest_opt(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

// ------------------------------------------------------------------
// serde helpers
// ------------------------------------------------------------------

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSandboxConfig {
    #[serde(default = "default_tier")]
    tier: SandboxTier,
}

fn default_tier() -> SandboxTier {
    SandboxTier::DEFAULT_FLOOR
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResourceCaps {
    cpu_max_pct: Option<u32>,
    memory_max_mb: Option<u32>,
    fd_max: Option<u32>,
}

impl RawResourceCaps {
    fn validate(self) -> Result<ResourceCaps, ManifestError> {
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        if let Some(v) = self.cpu_max_pct {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange { field: "cpu_max_pct".into(), value: v });
            }
            if v > 100 * num_cpus {
                return Err(ManifestError::CapOutOfRange { field: "cpu_max_pct".into(), value: v });
            }
        }
        if let Some(v) = self.memory_max_mb {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange { field: "memory_max_mb".into(), value: v });
            }
        }
        if let Some(v) = self.fd_max {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange { field: "fd_max".into(), value: v });
            }
        }
        Ok(ResourceCaps {
            cpu_max_pct: self.cpu_max_pct,
            memory_max_mb: self.memory_max_mb,
            fd_max: self.fd_max,
        })
    }
}

// ------------------------------------------------------------------
// Tests (NFR-Test-13 ≥3 cases per field)
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SandboxConfig ----

    #[test]
    fn sandbox_config_well_formed() {
        let cfg = SandboxConfig::from_toml_str(r#"tier = "T2""#).unwrap();
        assert_eq!(cfg.tier, SandboxTier::T2);
    }

    #[test]
    fn sandbox_config_malformed_rejected() {
        let err = SandboxConfig::from_toml_str(r#"tier = "t2""#).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(_)));
    }

    #[test]
    fn sandbox_config_edge_missing_uses_default() {
        let cfg = SandboxConfig::from_toml_str("").unwrap();
        assert_eq!(cfg.tier, SandboxTier::DEFAULT_FLOOR);
    }

    #[test]
    fn sandbox_config_t3_parseable_but_rejected_at_admission() {
        let cfg = SandboxConfig::from_toml_str(r#"tier = "T3""#).unwrap();
        assert_eq!(cfg.tier, SandboxTier::T3);
    }

    // ---- ResourceCaps ----

    #[test]
    fn resource_caps_well_formed() {
        let caps = ResourceCaps::from_toml_str("cpu_max_pct = 50\nmemory_max_mb = 512\nfd_max = 64").unwrap();
        assert_eq!(caps.cpu_max_pct, Some(50));
        assert_eq!(caps.memory_max_mb, Some(512));
        assert_eq!(caps.fd_max, Some(64));
    }

    #[test]
    fn resource_caps_malformed_zero_rejected() {
        let err = ResourceCaps::from_toml_str(r#"memory_max_mb = 0"#).unwrap_err();
        assert!(matches!(err, ManifestError::CapOutOfRange { field, .. } if field == "memory_max_mb"));
    }

    #[test]
    fn resource_caps_edge_missing_all_optional() {
        let caps = ResourceCaps::from_toml_str("").unwrap();
        assert_eq!(caps.cpu_max_pct, None);
        assert_eq!(caps.memory_max_mb, None);
        assert_eq!(caps.fd_max, None);
    }

    #[test]
    fn resource_caps_partial_present() {
        let caps = ResourceCaps::from_toml_str(r#"cpu_max_pct = 75"#).unwrap();
        assert_eq!(caps.cpu_max_pct, Some(75));
        assert_eq!(caps.memory_max_mb, None);
    }

    // ---- resolve_caps ----

    #[test]
    fn resolve_caps_manifest_wins_when_tighter() {
        let m = ResourceCaps { cpu_max_pct: Some(30), memory_max_mb: Some(256), fd_max: None };
        let o = ResourceCaps { cpu_max_pct: Some(50), memory_max_mb: Some(512), fd_max: None };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(30));
        assert_eq!(r.memory_max_mb, Some(256));
    }

    #[test]
    fn resolve_caps_operator_wins_when_tighter() {
        let m = ResourceCaps { cpu_max_pct: Some(80), memory_max_mb: None, fd_max: Some(128) };
        let o = ResourceCaps { cpu_max_pct: Some(40), memory_max_mb: Some(1024), fd_max: Some(64) };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(40));
        assert_eq!(r.memory_max_mb, Some(1024));
        assert_eq!(r.fd_max, Some(64));
    }

    #[test]
    fn resolve_caps_none_falls_through() {
        let m = ResourceCaps { cpu_max_pct: None, memory_max_mb: Some(256), fd_max: None };
        let o = ResourceCaps { cpu_max_pct: Some(50), memory_max_mb: None, fd_max: Some(32) };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(50));
        assert_eq!(r.memory_max_mb, Some(256));
        assert_eq!(r.fd_max, Some(32));
    }

    #[test]
    fn resolve_caps_both_none_none() {
        let m = ResourceCaps::default();
        let o = ResourceCaps::default();
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, None);
        assert_eq!(r.memory_max_mb, None);
        assert_eq!(r.fd_max, None);
    }
}
