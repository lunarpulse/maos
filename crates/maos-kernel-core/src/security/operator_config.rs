//! Operator configuration — operator-policy sections for [registry] and [region].
//!
//! Resolved at composition root in this priority order:
//! env-vars (MAOS_REGISTRY_* / MAOS_REGION_*) → ~/.config/maos/operator.toml →
//! built-in defaults.

use maos_domain::region::Region;
use maos_spirit_abi::compliance::TrustTier;

/// Operator-configurable registry section per ADR-009.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RegistrySection {
    #[doc = "Construct via [`RegistrySection::new`] — registry MCP endpoint URI."]
    pub uri: String,
    #[doc = "Construct via [`RegistrySection::new`] — minimum trust-tier floor enforced."]
    pub tier_floor: TrustTier,
    #[doc = "Construct via [`RegistrySection::new`] — escalate public-untrusted to T3 sandbox."]
    pub t3_for_public_untrusted: bool,
    #[doc = "Construct via [`RegistrySection::new`] — allow unsigned local Spirits (dev workflow)."]
    pub allow_unsigned_local: bool,
    #[doc = "Construct via [`RegistrySection::new`] — Ed25519 public key the registry-side org-internal anchor signs with."]
    pub org_signing_pubkey: Option<[u8; 32]>,
    #[doc = "Story 7.2 — reject manifests that lack server-reported tier + signature."]
    pub require_server_tier_signature: bool,
    #[doc = "Story 7.2 — allow `maosctl import --force-tier` to override tier for air-gapped imports."]
    pub allow_force_tier_at_import: bool,
}

impl RegistrySection {
    pub fn new(
        uri: String,
        tier_floor: TrustTier,
        t3_for_public_untrusted: bool,
        allow_unsigned_local: bool,
        org_signing_pubkey: Option<[u8; 32]>,
        require_server_tier_signature: bool,
        allow_force_tier_at_import: bool,
    ) -> Self {
        Self {
            uri,
            tier_floor,
            t3_for_public_untrusted,
            allow_unsigned_local,
            org_signing_pubkey,
            require_server_tier_signature,
            allow_force_tier_at_import,
        }
    }

    /// Resolve from env-vars, operator.toml, and built-in defaults.
    pub fn resolve_from_env_and_disk() -> Self {
        // 1. Start with built-in defaults
        let mut section = Self::defaults();

        // 2. Override from ~/.config/maos/operator.toml if present
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::PathBuf::from(home)
                .join(".config")
                .join("maos")
                .join("operator.toml");
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(toml_val) = contents.parse::<toml::Value>() {
                        if let Some(registry_table) = toml_val.get("registry") {
                            if let Some(uri) = registry_table.get("uri").and_then(|v| v.as_str()) {
                                section.uri = uri.to_string();
                            }
                            if let Some(tf) =
                                registry_table.get("tier_floor").and_then(|v| v.as_str())
                            {
                                section.tier_floor = parse_tier(tf);
                            }
                            if let Some(b) = registry_table
                                .get("t3_for_public_untrusted")
                                .and_then(|v| v.as_bool())
                            {
                                section.t3_for_public_untrusted = b;
                            }
                            if let Some(b) = registry_table
                                .get("allow_unsigned_local")
                                .and_then(|v| v.as_bool())
                            {
                                section.allow_unsigned_local = b;
                            }
                            if let Some(hex_key) = registry_table
                                .get("org_signing_pubkey")
                                .and_then(|v| v.as_str())
                            {
                                if let Ok(bytes) = hex::decode(hex_key) {
                                    if let Ok(arr) = bytes.try_into() {
                                        section.org_signing_pubkey = Some(arr);
                                    }
                                }
                            }
                            if let Some(b) = registry_table
                                .get("require_server_tier_signature")
                                .and_then(|v| v.as_bool())
                            {
                                section.require_server_tier_signature = b;
                            }
                            if let Some(b) = registry_table
                                .get("allow_force_tier_at_import")
                                .and_then(|v| v.as_bool())
                            {
                                section.allow_force_tier_at_import = b;
                            }
                        }
                    }
                }
            }
        }

        // 3. Override from env vars (highest priority)
        if let Ok(uri) = std::env::var("MAOS_REGISTRY_URI") {
            section.uri = uri;
        }
        if let Ok(tf) = std::env::var("MAOS_REGISTRY_TIER_FLOOR") {
            section.tier_floor = parse_tier(&tf);
        }
        if let Ok(v) = std::env::var("MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED") {
            section.t3_for_public_untrusted = v == "true" || v == "1";
        }
        if let Ok(v) = std::env::var("MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL") {
            section.allow_unsigned_local = !(v == "false" || v == "0");
        }
        if let Ok(hex_key) = std::env::var("MAOS_REGISTRY_ORG_SIGNING_PUBKEY") {
            if let Ok(bytes) = hex::decode(&hex_key) {
                if let Ok(arr) = bytes.try_into() {
                    section.org_signing_pubkey = Some(arr);
                }
            }
        }
        if let Ok(v) = std::env::var("MAOS_REGISTRY_REQUIRE_SERVER_TIER_SIGNATURE") {
            section.require_server_tier_signature = v == "true" || v == "1";
        }
        if let Ok(v) = std::env::var("MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT") {
            section.allow_force_tier_at_import = v == "true" || v == "1";
        }

        section
    }

    /// Built-in defaults: most-permissive, unconfigured.
    fn defaults() -> Self {
        Self {
            uri: String::new(),
            tier_floor: TrustTier::Local,
            t3_for_public_untrusted: false,
            allow_unsigned_local: true,
            org_signing_pubkey: None,
            require_server_tier_signature: false,
            allow_force_tier_at_import: false,
        }
    }
}

/// Story 9.4b AC-5 — operator region-pinning configuration (the `[region]`
/// section of `~/.config/maos/operator.toml`, with `MAOS_REGION_*` env
/// overrides mirroring the `MAOS_REGISTRY_*` precedence).
///
/// `home_region == None` means region pinning is **disabled** — artifacts are
/// signed with the raw (non-region-derived) key, exactly as pre-9.4b
/// (default-region / legacy semantics, AC-11).  When `Some`, every region-aware
/// signing/verify path binds to that canonical jurisdiction tag.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RegionSection {
    #[doc = "Canonical (`ascii-v1`) home jurisdiction tag; `None` disables region pinning."]
    pub home_region: Option<Region>,
}

impl RegionSection {
    pub fn new(home_region: Option<Region>) -> Self {
        Self { home_region }
    }

    /// Resolve from env-vars, operator.toml, and built-in defaults.
    /// Precedence: `MAOS_REGION_HOME` → `[region].home_region` → default (None).
    pub fn resolve_from_env_and_disk() -> Self {
        let mut section = Self::defaults();

        // 1. operator.toml `[region]` section.
        if let Ok(home) = std::env::var("HOME") {
            let path = std::path::PathBuf::from(home)
                .join(".config")
                .join("maos")
                .join("operator.toml");
            if path.exists() {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    if let Ok(toml_val) = contents.parse::<toml::Value>() {
                        if let Some(region_table) = toml_val.get("region") {
                            if let Some(tag) =
                                region_table.get("home_region").and_then(|v| v.as_str())
                            {
                                section.home_region = canonicalize_or_warn(tag);
                            }
                        }
                    }
                }
            }
        }

        // 2. env override (highest priority).
        if let Ok(tag) = std::env::var("MAOS_REGION_HOME") {
            // An explicitly-empty env var disables region pinning.
            section.home_region = if tag.trim().is_empty() {
                None
            } else {
                canonicalize_or_warn(&tag)
            };
        }

        section
    }

    /// Built-in default: region pinning disabled.
    fn defaults() -> Self {
        Self { home_region: None }
    }
}

/// Canonicalize a region tag, warning (and disabling) on an invalid tag rather
/// than silently binding to a wrong-or-unrecoverable region (AC-12 fail-safe).
fn canonicalize_or_warn(tag: &str) -> Option<Region> {
    match Region::canonicalize(tag) {
        Ok(r) => Some(r),
        Err(e) => {
            eprintln!("maos: warning: invalid region tag '{tag}' ({e}); region pinning DISABLED");
            None
        }
    }
}

fn parse_tier(s: &str) -> TrustTier {
    match s.trim().to_lowercase().as_str() {
        "local" => TrustTier::Local,
        "org_internal" => TrustTier::OrgInternal,
        "public_vetted" => TrustTier::PublicVetted,
        "public_untrusted" => TrustTier::PublicUntrusted,
        _ => {
            eprintln!(
                "maos: warning: unrecognized trust tier '{}', defaulting to 'local'",
                s
            );
            TrustTier::Local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn defaults_are_local_tier() {
        let section = RegistrySection::defaults();
        assert!(section.uri.is_empty());
        assert_eq!(section.tier_floor, TrustTier::Local);
        assert!(!section.t3_for_public_untrusted);
        assert!(section.allow_unsigned_local);
        assert!(section.org_signing_pubkey.is_none());
    }

    #[test]
    fn resolve_from_defaults_when_no_env() {
        let _guard = ENV_LOCK.lock().unwrap();
        let section = RegistrySection::resolve_from_env_and_disk();
        assert!(section.allow_unsigned_local);
    }

    #[test]
    fn parse_tier_maps_correctly() {
        assert_eq!(parse_tier("local"), TrustTier::Local);
        assert_eq!(parse_tier("org_internal"), TrustTier::OrgInternal);
        assert_eq!(parse_tier("public_vetted"), TrustTier::PublicVetted);
        assert_eq!(parse_tier("public_untrusted"), TrustTier::PublicUntrusted);
        assert_eq!(parse_tier("unknown"), TrustTier::Local);
    }

    #[test]
    fn env_overrides_disk_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("MAOS_REGISTRY_URI", "env://override");
        std::env::set_var("MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED", "true");
        std::env::set_var("MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL", "false");

        let section = RegistrySection::resolve_from_env_and_disk();
        assert_eq!(section.uri, "env://override");
        assert!(section.t3_for_public_untrusted);
        assert!(!section.allow_unsigned_local);

        std::env::remove_var("MAOS_REGISTRY_URI");
        std::env::remove_var("MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED");
        std::env::remove_var("MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL");
    }

    #[test]
    fn env_allows_negating_bools() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED", "false");
        std::env::set_var("MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL", "true");

        let section = RegistrySection::resolve_from_env_and_disk();
        assert!(!section.t3_for_public_untrusted);
        assert!(section.allow_unsigned_local);

        std::env::remove_var("MAOS_REGISTRY_T3_FOR_PUBLIC_UNTRUSTED");
        std::env::remove_var("MAOS_REGISTRY_ALLOW_UNSIGNED_LOCAL");
    }

    // ─── Story 9.4b AC-5 RegionSection ─────────────────────────────────────

    #[test]
    fn region_defaults_to_disabled() {
        let s = RegionSection::defaults();
        assert!(s.home_region.is_none());
    }

    #[test]
    fn region_env_sets_and_canonicalizes() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("MAOS_REGION_HOME", "US-EAST-1");
        let s = RegionSection::resolve_from_env_and_disk();
        assert_eq!(
            s.home_region.as_ref().map(|r| r.as_str()),
            Some("us-east-1")
        );
        std::env::remove_var("MAOS_REGION_HOME");
    }

    #[test]
    fn region_env_empty_disables() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("MAOS_REGION_HOME", "  ");
        let s = RegionSection::resolve_from_env_and_disk();
        assert!(s.home_region.is_none());
        std::env::remove_var("MAOS_REGION_HOME");
    }

    #[test]
    fn region_env_invalid_tag_disables_with_warning() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("MAOS_REGION_HOME", "eu_west"); // underscore invalid
        let s = RegionSection::resolve_from_env_and_disk();
        // Fail-safe: invalid tag DISABLES pinning rather than binding wrongly.
        assert!(s.home_region.is_none());
        std::env::remove_var("MAOS_REGION_HOME");
    }
}
