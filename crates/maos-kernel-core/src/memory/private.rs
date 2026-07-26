#![forbid(unsafe_code)]

//! Private-tier memory store — per-Spirit `Arc<RwLock<HashMap>>` +
//! per-Spirit-namespaced filesystem area (Story 4.3).
//!
//! In-memory map holds small values inline; `Markdown` values and values
//! exceeding `inline_threshold` spill to the per-Spirit filesystem area
//! under `<fs_root>/<spirit_pid>/<ns_encoded>/<key>.<ext>`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue, ValueKind};

const MAX_KEY_LEN: usize = 1024;

/// Per-Spirit in-memory + filesystem-backed key-value store.
#[maos_attrs::i9_exempt(
    reason = "memory manager private tier — per-Spirit-keyed in-memory map + per-Spirit-namespaced filesystem area for ADR-026 + I5 isolation; bounded by principal forget-cascade and per-Spirit memory budget"
)]
pub struct PrivateMemoryStore {
    in_mem: RwLock<HashMap<(u32, MemoryNamespace, String), MemoryValue>>,
    fs_root: PathBuf,
    inline_threshold: usize,
}

impl PrivateMemoryStore {
    pub fn new(fs_root: PathBuf, inline_threshold: usize) -> Self {
        Self {
            in_mem: RwLock::new(HashMap::new()),
            fs_root,
            inline_threshold,
        }
    }

    // ------------------------------------------------------------------
    // key sanitization
    // ------------------------------------------------------------------

    fn sanitize_key(key: &str) -> Result<(), MemoryError> {
        if key.len() > MAX_KEY_LEN {
            return Err(MemoryError::KeyTooLong {
                len: key.len(),
                max: MAX_KEY_LEN,
            });
        }
        for ch in key.chars() {
            if ch == '/' || ch == '\\' || ch == '\0' || ch.is_ascii_control() {
                return Err(MemoryError::InvalidKey {
                    key: key.to_string(),
                });
            }
        }
        // Reject path-traversal sequence ".." only when it appears as a
        // path component (preceded by start or '/' and followed by end or
        // '/').  This preserves legitimate keys like "foo..bar".
        if key.split('/').any(|segment| segment == "..") {
            return Err(MemoryError::InvalidKey {
                key: key.to_string(),
            });
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // filesystem path helpers
    // ------------------------------------------------------------------

    fn namespace_to_dirname(ns: &MemoryNamespace) -> Result<String, MemoryError> {
        let json = serde_json::to_string(ns)
            .map_err(|e| MemoryError::Storage(format!("namespace serialize: {e}")))?;
        // Hex-encode for filesystem-safe directory names (I5-safe).
        let mut hex = String::with_capacity(json.len() * 2);
        for byte in json.as_bytes() {
            hex.push_str(&format!("{byte:02x}"));
        }
        Ok(hex)
    }

    fn file_ext_for_kind(kind: ValueKind) -> &'static str {
        match kind {
            ValueKind::Json => ".json",
            ValueKind::Markdown => ".md",
            ValueKind::Blob => ".bin",
            ValueKind::Text => ".txt",
        }
    }

    fn fs_path_for(
        fs_root: &Path,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        kind: ValueKind,
    ) -> Result<PathBuf, MemoryError> {
        let ns_dir = Self::namespace_to_dirname(namespace)?;
        let ext = Self::file_ext_for_kind(kind);
        // sanitize_key already rejected '/', '\', and '..' path components;
        // we do NOT further mangle the key so the on-disk name matches the
        // logical key exactly.
        Ok(fs_root
            .join(spirit_pid.to_string())
            .join(&ns_dir)
            .join(format!("{key}{ext}")))
    }

    fn read_from_disk(path: &Path, kind: ValueKind) -> Result<MemoryValue, MemoryError> {
        let bytes = fs::read(path).map_err(MemoryError::Io)?;
        match kind {
            ValueKind::Json => {
                let val: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(MemoryValue::Json(val))
            }
            ValueKind::Markdown => {
                let s =
                    String::from_utf8(bytes).map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(MemoryValue::Markdown(s))
            }
            ValueKind::Blob => Ok(MemoryValue::Blob(bytes)),
            ValueKind::Text => {
                let s =
                    String::from_utf8(bytes).map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(MemoryValue::Text(s))
            }
        }
    }

    fn write_to_disk(path: &Path, value: &MemoryValue) -> Result<(), MemoryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(MemoryError::Io)?;
        }
        match value {
            MemoryValue::Json(v) => {
                let bytes =
                    serde_json::to_vec(v).map_err(|e| MemoryError::Storage(e.to_string()))?;
                fs::write(path, &bytes).map_err(MemoryError::Io)?;
            }
            MemoryValue::Markdown(s) => {
                fs::write(path, s.as_bytes()).map_err(MemoryError::Io)?;
            }
            MemoryValue::Blob(b) => {
                fs::write(path, b).map_err(MemoryError::Io)?;
            }
            MemoryValue::Text(s) => {
                fs::write(path, s.as_bytes()).map_err(MemoryError::Io)?;
            }
        }
        Ok(())
    }

    fn should_spill_to_disk(
        value: &MemoryValue,
        inline_threshold: usize,
    ) -> Result<bool, MemoryError> {
        Ok(
            matches!(value, MemoryValue::Markdown(_))
                || value.approximate_len()? > inline_threshold,
        )
    }

    // ------------------------------------------------------------------
    // public API
    // ------------------------------------------------------------------

    pub(in crate::memory) fn write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError> {
        Self::sanitize_key(key)?;

        let needs_spill = Self::should_spill_to_disk(&value, self.inline_threshold)?;
        if needs_spill {
            let path = Self::fs_path_for(&self.fs_root, spirit_pid, namespace, key, value.kind())?;
            Self::write_to_disk(&path, &value)?;
        }

        // Markdown is filesystem-canonical (operator-editable).  Do NOT
        // cache it in-memory so that operator hand-edits on disk are visible
        // on the next read.
        if matches!(value, MemoryValue::Markdown(_)) {
            return Ok(());
        }

        // For all other values, update the in-memory map for cache-warm reads.
        let key_tuple = (spirit_pid, namespace.clone(), key.to_string());
        let mut map = self
            .in_mem
            .write()
            .expect("PrivateMemoryStore lock poisoned");
        map.insert(key_tuple, value);
        Ok(())
    }

    pub(in crate::memory) fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        Self::sanitize_key(key)?;

        let key_tuple = (spirit_pid, namespace.clone(), key.to_string());

        // Check in-memory cache first (non-Markdown values only).
        {
            let map = self
                .in_mem
                .read()
                .expect("PrivateMemoryStore lock poisoned");
            if let Some(v) = map.get(&key_tuple) {
                return Ok(Some(v.clone()));
            }
        }

        // Try filesystem — for Markdown this is the canonical source;
        // for other types it's the spill fallback.
        for kind in &[
            ValueKind::Markdown,
            ValueKind::Json,
            ValueKind::Blob,
            ValueKind::Text,
        ] {
            let path = Self::fs_path_for(&self.fs_root, spirit_pid, namespace, key, *kind)?;
            if path.exists() {
                let value = Self::read_from_disk(&path, *kind)?;
                // Populate cache for non-Markdown values only.
                if !matches!(value, MemoryValue::Markdown(_)) {
                    let mut map = self
                        .in_mem
                        .write()
                        .expect("PrivateMemoryStore lock poisoned");
                    map.insert(key_tuple, value.clone());
                }
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Scan entries matching a key prefix within the given namespace,
    /// up to `limit` entries.  Order is NOT deterministic (HashMap
    /// iteration).  Merges in-memory and filesystem entries.
    pub(in crate::memory) fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let mut entries: Vec<MemoryEntry> = Vec::new();
        let ns = namespace.clone();
        let pid = spirit_pid;

        // In-memory entries.
        {
            let map = self
                .in_mem
                .read()
                .expect("PrivateMemoryStore lock poisoned");
            for ((pid_k, ns_k, key), value) in map.iter() {
                if *pid_k == pid && *ns_k == ns && key.starts_with(prefix) {
                    if entries.len() >= limit {
                        break;
                    }
                    entries.push(MemoryEntry {
                        namespace: ns.clone(),
                        key: key.clone(),
                        value: value.clone(),
                        timestamp_ns: 0, // v0.3-β: no per-entry timestamp
                    });
                }
            }
        }

        // Filesystem entries (spilled Markdown and large values).
        let pid_dir = self.fs_root.join(pid.to_string());
        let ns_dir = Self::namespace_to_dirname(&ns)?;
        let ns_path = pid_dir.join(&ns_dir);
        if ns_path.is_dir() {
            for entry in std::fs::read_dir(&ns_path).map_err(MemoryError::Io)? {
                let entry = entry.map_err(MemoryError::Io)?;
                let path = entry.path();
                let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.starts_with(prefix) {
                    let kind = match ext {
                        "json" => ValueKind::Json,
                        "md" => ValueKind::Markdown,
                        "bin" => ValueKind::Blob,
                        "txt" => ValueKind::Text,
                        _ => continue,
                    };
                    let value = Self::read_from_disk(&path, kind)?;
                    if entries.len() >= limit {
                        break;
                    }
                    entries.push(MemoryEntry {
                        namespace: ns.clone(),
                        key: file_name.to_string(),
                        value,
                        timestamp_ns: 0,
                    });
                }
            }
        }

        Ok(entries)
    }

    /// Internal helper — remove all entries for a given principal_id
    /// from the private store.  Returns the count of deleted entries.
    pub fn forget_principal(&self, principal_id: &str) -> Result<u64, MemoryError> {
        let mut count: u64 = 0;

        // Collect keys to remove (in-memory).
        let to_remove: Vec<(u32, MemoryNamespace, String)> = {
            let map = self
                .in_mem
                .read()
                .expect("PrivateMemoryStore lock poisoned");
            map.iter()
                .filter(|((_pid, ns, _key), _v)| match ns {
                    MemoryNamespace::Principal {
                        principal_id: pid, ..
                    } => pid == principal_id,
                    _ => false,
                })
                .map(|((pid, ns, key), _)| (*pid, ns.clone(), key.clone()))
                .collect()
        };

        // Remove from in-memory map.
        {
            let mut map = self
                .in_mem
                .write()
                .expect("PrivateMemoryStore lock poisoned");
            for (pid, ns, key) in &to_remove {
                let removed = map.remove(&(*pid, ns.clone(), key.clone()));
                if removed.is_some() {
                    count += 1;
                }
            }
        }

        // Remove filesystem subtrees for the principal namespace only.
        let fs_root = self.fs_root.clone();
        let mut cleaned_pids = std::collections::HashSet::new();
        for (pid, ns, _key) in &to_remove {
            let dir_name = Self::namespace_to_dirname(ns)?;
            let subtree = fs_root.join(pid.to_string()).join(&dir_name);
            if subtree.exists() {
                fs::remove_dir_all(&subtree).map_err(MemoryError::Io)?;
            }
            cleaned_pids.insert(*pid);
        }

        // Clean up per-pid directories only if they are now empty.
        for pid in cleaned_pids {
            let pid_dir = fs_root.join(pid.to_string());
            if pid_dir.is_dir() {
                let is_empty = std::fs::read_dir(&pid_dir)
                    .map(|mut d| d.next().is_none())
                    .unwrap_or(false);
                if is_empty {
                    let _ = fs::remove_dir(&pid_dir);
                }
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (PrivateMemoryStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let store = PrivateMemoryStore::new(tmp.path().to_path_buf(), 4 * 1024);
        (store, tmp)
    }

    #[test]
    fn write_and_read_in_memory_json() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Json(serde_json::json!({"x": 1}));
        store
            .write(1, &MemoryNamespace::Default, "k1", val.clone())
            .unwrap();
        let got = store.read(1, &MemoryNamespace::Default, "k1").unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn write_and_read_text() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Text("hello world".into());
        store
            .write(2, &MemoryNamespace::Default, "greeting", val.clone())
            .unwrap();
        let got = store
            .read(2, &MemoryNamespace::Default, "greeting")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn key_traversal_rejected_slash() {
        let (store, _tmp) = make_store();
        let err = store
            .write(
                1,
                &MemoryNamespace::Default,
                "a/b",
                MemoryValue::Text("bad".into()),
            )
            .unwrap_err();
        assert!(matches!(err, MemoryError::InvalidKey { .. }));
    }

    #[test]
    fn key_traversal_rejected_backslash() {
        let (store, _tmp) = make_store();
        let err = store
            .write(
                1,
                &MemoryNamespace::Default,
                "a\\b",
                MemoryValue::Text("bad".into()),
            )
            .unwrap_err();
        assert!(matches!(err, MemoryError::InvalidKey { .. }));
    }

    #[test]
    fn key_traversal_rejected_dot_dot() {
        let (store, _tmp) = make_store();
        let err = store
            .write(
                1,
                &MemoryNamespace::Default,
                "../escape",
                MemoryValue::Text("bad".into()),
            )
            .unwrap_err();
        assert!(matches!(err, MemoryError::InvalidKey { .. }));
    }

    #[test]
    fn key_traversal_rejected_nul() {
        let (store, _tmp) = make_store();
        let err = store
            .write(
                1,
                &MemoryNamespace::Default,
                "bad\0evil",
                MemoryValue::Text("bad".into()),
            )
            .unwrap_err();
        assert!(matches!(err, MemoryError::InvalidKey { .. }));
    }

    #[test]
    fn key_traversal_rejected_control_char() {
        let (store, _tmp) = make_store();
        let err = store
            .write(
                1,
                &MemoryNamespace::Default,
                "bad\x01ctrl",
                MemoryValue::Text("bad".into()),
            )
            .unwrap_err();
        assert!(matches!(err, MemoryError::InvalidKey { .. }));
    }

    #[test]
    fn legitimate_key_with_dot_dot_substring_is_accepted() {
        let (store, _tmp) = make_store();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "foo..bar",
                MemoryValue::Text("ok".into()),
            )
            .unwrap();
        let got = store
            .read(1, &MemoryNamespace::Default, "foo..bar")
            .unwrap();
        assert_eq!(got, Some(MemoryValue::Text("ok".into())));
    }

    #[test]
    fn markdown_spills_to_disk_and_reads_back() {
        let (store, tmp) = make_store();
        let payload = "# Hello\n\nThis is **markdown**.\n".to_string();
        let val = MemoryValue::Markdown(payload.clone());
        store
            .write(7, &MemoryNamespace::Default, "memory.md", val.clone())
            .unwrap();

        // Verify file exists on disk
        let fs_path = PrivateMemoryStore::fs_path_for(
            tmp.path(),
            7,
            &MemoryNamespace::Default,
            "memory.md",
            ValueKind::Markdown,
        )
        .unwrap();
        assert!(fs_path.exists(), "markdown file should exist on disk");

        let got = store
            .read(7, &MemoryNamespace::Default, "memory.md")
            .unwrap();
        assert_eq!(got, Some(MemoryValue::Markdown(payload)));
    }

    #[test]
    fn cross_pid_isolation_read_returns_none() {
        let (store, _tmp) = make_store();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "secret",
                MemoryValue::Text("spirit-1 secret".into()),
            )
            .unwrap();
        let got = store.read(2, &MemoryNamespace::Default, "secret").unwrap();
        assert!(got.is_none());
    }

    #[test]
    fn scan_returns_matching_entries() {
        let (store, _tmp) = make_store();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "apple",
                MemoryValue::Text("a".into()),
            )
            .unwrap();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "apricot",
                MemoryValue::Text("b".into()),
            )
            .unwrap();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "banana",
                MemoryValue::Text("c".into()),
            )
            .unwrap();
        let results = store.scan(1, &MemoryNamespace::Default, "ap", 10).unwrap();
        assert_eq!(results.len(), 2);
        let keys: Vec<&str> = results.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"apple"));
        assert!(keys.contains(&"apricot"));
    }

    #[test]
    fn forget_principal_removes_targeted_entries() {
        let (store, _tmp) = make_store();
        let ns1 = MemoryNamespace::principal("alice@example.org", "calendar").unwrap();
        let ns2 = MemoryNamespace::principal("bob@example.org", "tasks").unwrap();

        store
            .write(1, &ns1, "event-1", MemoryValue::Text("alice 1".into()))
            .unwrap();
        store
            .write(1, &ns1, "event-2", MemoryValue::Text("alice 2".into()))
            .unwrap();
        store
            .write(2, &ns2, "task-1", MemoryValue::Text("bob 1".into()))
            .unwrap();

        let deleted = store.forget_principal("alice@example.org").unwrap();
        assert!(deleted >= 2, "should have deleted alice's entries");

        let got1 = store.read(1, &ns1, "event-1").unwrap();
        assert!(got1.is_none(), "alice's entry should be gone");

        let got2 = store.read(2, &ns2, "task-1").unwrap();
        assert!(got2.is_some(), "bob's entry should remain");
    }

    #[test]
    fn read_missing_returns_none() {
        let (store, _tmp) = make_store();
        let got = store
            .read(1, &MemoryNamespace::Default, "no-such-key")
            .unwrap();
        assert!(got.is_none());
    }
}
