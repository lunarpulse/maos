//! Registry operations — the MCP-Streamable-HTTP "methods" per ADR-008.
//!
//! Each variant corresponds to a `registry.<op>` JSON-RPC method name.
//! All argument and response shapes carry `#[serde(rename_all = "snake_case")]`
//! for wire-consistency.

/// Top-level registry operation enum.
///
/// Serialized as `"method": "registry.<op>"` on the JSON-RPC wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryOperation {
    Search,
    Manifest,
    Artifact,
    Publish,
    Deprecate,
    YanksSince,
}

/// Arguments for `registry.search`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SearchArgs {
    #[doc = "Construct via [`SearchArgs::new`]."]
    pub text: String,
    #[doc = "Construct via [`SearchArgs::new`].  Default false."]
    #[serde(default)]
    pub include_yanked: bool,
    #[doc = "Construct via [`SearchArgs::new`].  Default 50; max 200."]
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    50
}

impl SearchArgs {
    pub fn new(text: String, include_yanked: bool, limit: u32) -> Self {
        Self {
            text,
            include_yanked,
            limit: limit.min(200).max(1),
        }
    }
}

/// Arguments for `registry.manifest`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ManifestArgs {
    #[doc = "Construct via [`ManifestArgs::new`]."]
    pub spirit_id: String,
    #[doc = "Construct via [`ManifestArgs::new`]."]
    pub version: String,
}

impl ManifestArgs {
    pub fn new(spirit_id: String, version: String) -> Self {
        Self { spirit_id, version }
    }
}

/// Arguments for `registry.artifact`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArtifactArgs {
    #[doc = "Construct via [`ArtifactArgs::new`]."]
    pub spirit_id: String,
    #[doc = "Construct via [`ArtifactArgs::new`]."]
    pub version: String,
}

impl ArtifactArgs {
    pub fn new(spirit_id: String, version: String) -> Self {
        Self { spirit_id, version }
    }
}

/// Arguments for `registry.publish`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishArgs {
    #[doc = "Construct via [`PublishArgs::new`]."]
    pub pkg: maos_domain::ports::registry::SignedPackage,
}

impl PublishArgs {
    pub fn new(pkg: maos_domain::ports::registry::SignedPackage) -> Self {
        Self { pkg }
    }
}

/// Arguments for `registry.deprecate`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeprecateArgs {
    #[doc = "Construct via [`DeprecateArgs::new`]."]
    pub spirit_id: String,
    #[doc = "Construct via [`DeprecateArgs::new`]."]
    pub version: String,
    #[doc = "Construct via [`DeprecateArgs::new`]."]
    pub reason: String,
}

impl DeprecateArgs {
    pub fn new(spirit_id: String, version: String, reason: String) -> Self {
        Self {
            spirit_id,
            version,
            reason,
        }
    }
}

/// Arguments for `registry.yanks_since` (kernel-internal op).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct YanksSinceArgs {
    #[doc = "Construct via [`YanksSinceArgs::new`]."]
    pub since_ns: u64,
}

impl YanksSinceArgs {
    pub fn new(since_ns: u64) -> Self {
        Self { since_ns }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_serialization_is_snake_case() {
        let op = RegistryOperation::Search;
        let json = serde_json::to_string(&op).unwrap();
        assert_eq!(json, r#""search""#);

        let op2 = RegistryOperation::YanksSince;
        let json2 = serde_json::to_string(&op2).unwrap();
        assert_eq!(json2, r#""yanks_since""#);
    }

    #[test]
    fn search_args_defaults() {
        let args = SearchArgs::new("hello".into(), false, 50);
        assert_eq!(args.text, "hello");
        assert!(!args.include_yanked);
        assert_eq!(args.limit, 50);
    }

    #[test]
    fn search_args_serde_roundtrip() {
        let args = SearchArgs::new("test".into(), true, 25);
        let json = serde_json::to_string(&args).unwrap();
        let args2: SearchArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(args, args2);
    }

    #[test]
    fn manifest_args_serde_roundtrip() {
        let args = ManifestArgs::new("spirit".into(), "1.0.0".into());
        let json = serde_json::to_string(&args).unwrap();
        let args2: ManifestArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(args, args2);
    }

    #[test]
    fn artifact_args_serde_roundtrip() {
        let args = ArtifactArgs::new("spirit".into(), "1.0.0".into());
        let json = serde_json::to_string(&args).unwrap();
        let args2: ArtifactArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(args, args2);
    }

    #[test]
    fn deprecate_args_serde_roundtrip() {
        let args = DeprecateArgs::new("spirit".into(), "1.0.0".into(), "buggy".into());
        let json = serde_json::to_string(&args).unwrap();
        let args2: DeprecateArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(args, args2);
    }

    #[test]
    fn yanks_since_args_serde_roundtrip() {
        let args = YanksSinceArgs::new(42);
        let json = serde_json::to_string(&args).unwrap();
        let args2: YanksSinceArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(args, args2);
    }
}
