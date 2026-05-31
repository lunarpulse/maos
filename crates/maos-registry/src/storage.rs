//! Registry storage — content-addressed filesystem tree.
//!
//! The `RegistryStorage` trait provides the persistence layer for the
//! Spirit Registry server.  `LocalFsRegistryStorage` implements it using
//! a directory tree at `~/.local/share/maos/registry/`.
//!
//! # Directory layout
//!
//! ```text
//! ~/.local/share/maos/registry/
//!   spirits/
//!     hello-spirit/
//!       0.1.0/
//!         manifest.toml
//!         artifact.bin
//!         signed_package.json
//!         compliance_claim.envelope
//!   index.json
//!   yanks.json
//! ```

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use maos_domain::ports::registry::{
    SearchQuery, SearchResultItem, SearchResults, SignedArtifact, SignedManifest, SignedPackage,
    SpiritId, YankEntry, YankList, YankReason, YankReceipt,
};

/// Persistent storage for the Spirit Registry.
pub trait RegistryStorage: Send + Sync {
    /// Store a signed package.
    fn put(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
    ) -> Result<(), StorageError>;

    /// Retrieve a signed manifest.
    fn get_manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, StorageError>;

    /// Retrieve a signed artifact.
    fn get_artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, StorageError>;

    /// Search the registry index.
    fn search(&self, q: &SearchQuery) -> Result<SearchResults, StorageError>;

    /// Yank (deprecate) a version.
    fn yank(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, StorageError>;

    /// List yanks since a given monotonic timestamp.
    fn yanks_since(&self, since_ns: u64) -> Result<YankList, StorageError>;

    /// Story 7.2 — store a signed package with origin metadata.
    fn publish_with_origin(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
        origin: &crate::origin::RegistryOrigin,
    ) -> Result<(), StorageError>;
}

/// Filesystem-backed registry storage using `~/.local/share/maos/registry/`.
pub struct LocalFsRegistryStorage {
    root: PathBuf,
    index: Mutex<BTreeMap<String, Vec<SearchResultItem>>>,
    yanks: Mutex<Vec<YankEntry>>,
}

impl LocalFsRegistryStorage {
    /// Construct storage rooted at `~/.local/share/maos/registry/`.
    pub fn new() -> Result<Self, io::Error> {
        let home = dirs_fallback();
        let root = home
            .join(".local")
            .join("share")
            .join("maos")
            .join("registry");
        fs::create_dir_all(root.join("spirits"))?;
        let index = Self::load_index(&root);
        let yanks = Self::load_yanks(&root);
        Ok(Self {
            root,
            index: Mutex::new(index),
            yanks: Mutex::new(yanks),
        })
    }

    /// Construct storage at a custom root path (for testing).
    pub fn at_path(root: PathBuf) -> Result<Self, io::Error> {
        fs::create_dir_all(root.join("spirits"))?;
        let index = Self::load_index(&root);
        let yanks = Self::load_yanks(&root);
        Ok(Self {
            root,
            index: Mutex::new(index),
            yanks: Mutex::new(yanks),
        })
    }

    fn load_index(root: &Path) -> BTreeMap<String, Vec<SearchResultItem>> {
        let path = root.join("index.json");
        if path.exists() {
            match fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
            {
                Some(idx) => idx,
                None => {
                    eprintln!("maos-registry: warning: failed to load index.json, starting with empty index");
                    BTreeMap::new()
                }
            }
        } else {
            BTreeMap::new()
        }
    }

    fn load_yanks(root: &Path) -> Vec<YankEntry> {
        let path = root.join("yanks.json");
        if path.exists() {
            match fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
            {
                Some(yanks) => yanks,
                None => {
                    eprintln!("maos-registry: warning: failed to load yanks.json, starting with empty yank list");
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        }
    }

    fn save_index_data(root: &Path, index: &BTreeMap<String, Vec<SearchResultItem>>) {
        if let Ok(json) = serde_json::to_vec_pretty(index) {
            let _ = fs::write(root.join("index.json"), &json);
        }
    }

    fn save_yanks_data(root: &Path, yanks: &[YankEntry]) {
        if let Ok(json) = serde_json::to_vec_pretty(yanks) {
            let _ = fs::write(root.join("yanks.json"), &json);
        }
    }

    fn spirit_dir(&self, spirit_id: &SpiritId) -> PathBuf {
        self.root.join("spirits").join(spirit_id.as_str())
    }

    fn version_dir(&self, spirit_id: &SpiritId, version: &str) -> PathBuf {
        self.spirit_dir(spirit_id).join(version)
    }
}

impl LocalFsRegistryStorage {
    fn write_origin(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        origin: &crate::origin::RegistryOrigin,
    ) -> Result<(), StorageError> {
        let vdir = self.version_dir(spirit_id, version);
        let origin_json =
            serde_json::to_vec_pretty(origin).map_err(|e| StorageError::Serde(e.to_string()))?;
        fs::write(vdir.join("origin.json"), &origin_json)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(())
    }
}

impl RegistryStorage for LocalFsRegistryStorage {
    fn put(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
    ) -> Result<(), StorageError> {
        let vdir = self.version_dir(spirit_id, version);
        fs::create_dir_all(&vdir).map_err(|e| StorageError::Io(e.to_string()))?;

        // Write manifest.toml
        fs::write(vdir.join("manifest.toml"), &pkg.manifest_toml)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        // Write artifact.bin
        fs::write(vdir.join("artifact.bin"), &pkg.artifact_bytes)
            .map_err(|e| StorageError::Io(e.to_string()))?;
        // Write signed_package.json
        let pkg_json =
            serde_json::to_vec_pretty(pkg).map_err(|e| StorageError::Serde(e.to_string()))?;
        fs::write(vdir.join("signed_package.json"), &pkg_json)
            .map_err(|e| StorageError::Io(e.to_string()))?;

        // Update index
        let index_snapshot = {
            let mut idx = self
                .index
                .lock()
                .map_err(|e| StorageError::Io(format!("index lock poisoned: {e}")))?;
            let items = idx.entry(spirit_id.as_str().to_string()).or_default();
            items.retain(|i| i.version != version);
            items.push(SearchResultItem::new(
                spirit_id.clone(),
                version.to_string(),
                extract_summary(&pkg.manifest_toml),
            ));
            idx.clone()
        };
        Self::save_index_data(&self.root, &index_snapshot);

        Ok(())
    }

    fn publish_with_origin(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        pkg: &SignedPackage,
        origin: &crate::origin::RegistryOrigin,
    ) -> Result<(), StorageError> {
        self.put(spirit_id, version, pkg)?;
        self.write_origin(spirit_id, version, origin)
    }

    fn get_manifest(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedManifest, StorageError> {
        let vdir = self.version_dir(spirit_id, version);
        let manifest_toml =
            fs::read(vdir.join("manifest.toml")).map_err(|_| StorageError::VersionNotFound {
                spirit_id: spirit_id.as_str().to_string(),
                version: version.to_string(),
            })?;
        // Reconstruct from stored files.  Signature data is in signed_package.json.
        let pkg_json = fs::read_to_string(vdir.join("signed_package.json"))
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let pkg: SignedPackage =
            serde_json::from_str(&pkg_json).map_err(|e| StorageError::Serde(e.to_string()))?;
        Ok(SignedManifest::new(
            spirit_id.clone(),
            version.to_string(),
            manifest_toml,
            pkg.signature,
            pkg.publisher_pubkey,
        ))
    }

    fn get_artifact(
        &self,
        spirit_id: &SpiritId,
        version: &str,
    ) -> Result<SignedArtifact, StorageError> {
        let vdir = self.version_dir(spirit_id, version);
        let artifact_bytes =
            fs::read(vdir.join("artifact.bin")).map_err(|_| StorageError::VersionNotFound {
                spirit_id: spirit_id.as_str().to_string(),
                version: version.to_string(),
            })?;
        let pkg_json = fs::read_to_string(vdir.join("signed_package.json"))
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let pkg: SignedPackage =
            serde_json::from_str(&pkg_json).map_err(|e| StorageError::Serde(e.to_string()))?;
        Ok(SignedArtifact::new(
            spirit_id.clone(),
            version.to_string(),
            artifact_bytes,
            pkg.signature,
            pkg.publisher_pubkey,
        ))
    }

    fn search(&self, q: &SearchQuery) -> Result<SearchResults, StorageError> {
        let query_lower = q.text.to_lowercase();
        if query_lower.is_empty() {
            return Ok(SearchResults::new(Vec::new()));
        }

        // Story 7.2 (closes 5.5d Low #28) — snapshot yanks ONCE outside the index
        // lock so we don't re-acquire the yanks Mutex inside the index walk.
        // Pre-7.2: holding `idx` while repeatedly locking `yanks` produced O(N×M)
        // contention; the snapshot reduces it to O(N) + O(1) yanks lock acquisitions.
        let yanks_snapshot: Vec<YankEntry> = if !q.include_yanked {
            self.yanks
                .lock()
                .map_err(|e| StorageError::Io(format!("yanks lock poisoned: {e}")))?
                .clone()
        } else {
            Vec::new()
        };

        let idx = self
            .index
            .lock()
            .map_err(|e| StorageError::Io(format!("index lock poisoned: {e}")))?
            .clone();
        let mut all_items: Vec<SearchResultItem> = Vec::new();
        for items in idx.values() {
            for item in items {
                let matches = item
                    .spirit_id
                    .as_str()
                    .to_lowercase()
                    .contains(&query_lower);
                if !matches {
                    continue;
                }
                if !q.include_yanked {
                    let is_yanked = yanks_snapshot
                        .iter()
                        .any(|y| y.spirit_id == item.spirit_id && y.version == item.version);
                    if is_yanked {
                        continue;
                    }
                }
                all_items.push(item.clone());
            }
        }

        let limit = q.limit.min(200).max(1) as usize;
        all_items.truncate(limit);

        Ok(SearchResults::new(all_items))
    }

    fn yank(
        &self,
        spirit_id: &SpiritId,
        version: &str,
        reason: &YankReason,
    ) -> Result<YankReceipt, StorageError> {
        let yank_id = format!("yank-{}", monotonic_now_ns());
        let entry = YankEntry::new(
            spirit_id.clone(),
            version.to_string(),
            monotonic_now_ns(),
            reason.summary.clone(),
        );

        let yanks_snapshot = {
            let mut yanks = self
                .yanks
                .lock()
                .map_err(|e| StorageError::Io(format!("yanks lock poisoned: {e}")))?;
            yanks.push(entry);
            yanks.clone()
        };
        Self::save_yanks_data(&self.root, &yanks_snapshot);

        Ok(YankReceipt::new(
            yank_id,
            spirit_id.clone(),
            version.to_string(),
        ))
    }

    fn yanks_since(&self, since_ns: u64) -> Result<YankList, StorageError> {
        let yanks = self
            .yanks
            .lock()
            .map_err(|e| StorageError::Io(format!("yanks lock poisoned: {e}")))?;
        let entries: Vec<YankEntry> = yanks
            .iter()
            .filter(|e| e.yanked_at_ns >= since_ns)
            .cloned()
            .collect();
        Ok(YankList::new(entries))
    }
}

/// Truncate a string at a character boundary, keeping at most `max_chars` characters.
fn truncate_str(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

/// Extract a short summary from a manifest TOML.
fn extract_summary(manifest_toml: &[u8]) -> String {
    let text = String::from_utf8_lossy(manifest_toml);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("description") {
            if let Some(val) = trimmed.split('=').nth(1) {
                let s = val.trim().trim_matches('"').to_string();
                if !s.is_empty() {
                    return s;
                }
            }
        }
    }
    for line in text.lines() {
        if !line.trim().is_empty() && !line.trim().starts_with('[') && !line.trim().starts_with('#')
        {
            let s = line.trim().to_string();
            if s.len() <= 120 {
                return s;
            }
            return format!("{}...", truncate_str(&s, 117));
        }
    }
    String::from("(no description)")
}

/// Monotonic nanosecond counter (per Story 5.5c §1366).
pub(crate) fn monotonic_now_ns() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(|| std::time::Instant::now());
    start.elapsed().as_nanos() as u64
}

/// Resolve the user's home directory.
fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Storage errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("io error: {0}")]
    Io(String),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("version '{version}' not found for spirit '{spirit_id}'")]
    VersionNotFound { spirit_id: String, version: String },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};

    fn empty_envelope() -> ComplianceClaimEnvelope {
        ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![0xA1, 0x01],
            signing_alg: SigningAlg::Ed25519,
        }
    }

    fn test_pkg(id: &str, ver: &str, desc: &str) -> SignedPackage {
        let manifest = format!(
            "[spirit]\nname = \"{}\"\nversion = \"{}\"\ndescription = \"{}\"\n",
            id, ver, desc
        );
        SignedPackage::new(
            SpiritId::from(id),
            ver.to_string(),
            manifest.into_bytes(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        )
    }

    fn temp_storage() -> LocalFsRegistryStorage {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("maos-registry-test-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&dir);
        LocalFsRegistryStorage::at_path(dir).unwrap()
    }

    #[test]
    fn put_and_get_manifest() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "A friendly Spirit");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        let manifest = s.get_manifest(&sid, "0.1.0").unwrap();
        assert_eq!(manifest.manifest_toml, pkg.manifest_toml);
    }

    #[test]
    fn put_and_get_artifact() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "A friendly Spirit");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        let artifact = s.get_artifact(&sid, "0.1.0").unwrap();
        assert_eq!(artifact.artifact_bytes, pkg.artifact_bytes);
    }

    #[test]
    fn search_finds_spirit_by_name() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "A friendly Spirit");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        let q = SearchQuery::new("hello".into(), false, 50);
        let results = s.search(&q).unwrap();
        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].spirit_id.as_str(), "hello-spirit");
    }

    #[test]
    fn search_excludes_yanked_by_default() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "Test");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        s.yank(&sid, "0.1.0", &YankReason::new("buggy".into()))
            .unwrap();
        let q = SearchQuery::new("hello".into(), false, 50);
        let results = s.search(&q).unwrap();
        assert!(results.items.is_empty());
    }

    #[test]
    fn search_includes_yanked_when_requested() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "Test");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        s.yank(&sid, "0.1.0", &YankReason::new("buggy".into()))
            .unwrap();
        let q = SearchQuery::new("hello".into(), true, 50);
        let results = s.search(&q).unwrap();
        assert_eq!(results.items.len(), 1);
    }

    #[test]
    fn yanks_since_returns_filtered_entries() {
        let s = temp_storage();
        let pkg = test_pkg("hello-spirit", "0.1.0", "Test");
        let sid = SpiritId::from("hello-spirit");
        s.put(&sid, "0.1.0", &pkg).unwrap();
        s.yank(&sid, "0.1.0", &YankReason::new("buggy".into()))
            .unwrap();

        let list = s.yanks_since(0).unwrap();
        assert_eq!(list.entries.len(), 1);
    }

    #[test]
    fn version_not_found_error() {
        let s = temp_storage();
        let sid = SpiritId::from("nonexistent");
        let err = s.get_manifest(&sid, "1.0.0").unwrap_err();
        assert!(matches!(err, StorageError::VersionNotFound { .. }));
    }
}
