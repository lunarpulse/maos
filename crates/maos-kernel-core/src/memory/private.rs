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

    /// Every value kind.  `write`, `scan` and `forget_principal` all walk this
    /// one list so they cannot drift about which on-disk names this store could
    /// have produced.
    const ALL_KINDS: [ValueKind; 4] = [
        ValueKind::Json,
        ValueKind::Markdown,
        ValueKind::Blob,
        ValueKind::Text,
    ];

    /// Reverse of the file-name half of `fs_path_for`: strip a recognized
    /// value-kind extension to recover the logical key **verbatim** (the key
    /// is deliberately not mangled on the way out, `:100-102`) together with
    /// the kind that produced it.  `None` means the name is not a spill this
    /// store could have written, so it is not a readable entry and must never
    /// be attested or returned as one.  Stripping the extension rather than
    /// taking `file_stem()` is what makes the empty key round-trip (`.md` stems
    /// to `".md"`, but strips to `""`) and what keeps a dotted key like
    /// `report.2026` intact.  The four extensions are mutually non-suffixing,
    /// so the match is unambiguous whatever the order.
    fn spill_name_parts(name: &str) -> Option<(&str, ValueKind)> {
        Self::ALL_KINDS
            .into_iter()
            .find_map(|kind| Some((name.strip_suffix(Self::file_ext_for_kind(kind))?, kind)))
    }

    fn key_from_spill_name(name: &str) -> Option<&str> {
        Self::spill_name_parts(name).map(|(key, _)| key)
    }

    /// Unlink every spill for `(spirit_pid, namespace, key)` whose kind is not
    /// `keep`, leaving **at most one** on-disk file per logical key.
    ///
    /// Without this, a value that changes kind — or that shrinks below
    /// `inline_threshold` and stops spilling at all — leaves its predecessor
    /// behind, where `read`'s fixed-order kind probe (`:243-248`) resurrects it
    /// on the next cold read.  The in-memory cache hides that until restart.
    /// This does NOT make sub-threshold values durable (those are
    /// process-lifetime working memory by design); it makes the durable state
    /// non-contradictory — disk holds this value or none, never a previous one.
    fn remove_superseded_spills(
        fs_root: &Path,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        keep: Option<ValueKind>,
    ) -> Result<(), MemoryError> {
        // One stat on the hot path: a key that never spilled has no namespace
        // directory, so there is nothing to unlink.
        let ns_path = fs_root
            .join(spirit_pid.to_string())
            .join(Self::namespace_to_dirname(namespace)?);
        if !ns_path.is_dir() {
            return Ok(());
        }
        for kind in Self::ALL_KINDS {
            if Some(kind) == keep {
                continue;
            }
            let stale = ns_path.join(format!("{key}{}", Self::file_ext_for_kind(kind)));
            match fs::remove_file(&stale) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                // Never swallowed: a spill that outlives its supersession is a
                // value this store will serve again after a restart.
                Err(e) => return Err(MemoryError::Io(e)),
            }
        }
        Ok(())
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
        let spilled_kind = if needs_spill {
            let path = Self::fs_path_for(&self.fs_root, spirit_pid, namespace, key, value.kind())?;
            Self::write_to_disk(&path, &value)?;
            Some(value.kind())
        } else {
            None
        };
        // The new spill is durable BEFORE its predecessors are unlinked, so a
        // crash in between leaves a recoverable superset, never a gap.
        Self::remove_superseded_spills(&self.fs_root, spirit_pid, namespace, key, spilled_kind)?;

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
        // for other types it's the spill fallback.  Markdown is probed first
        // (it is authoritative), so this order deliberately differs from
        // `ALL_KINDS`; `remove_superseded_spills` guarantees at most one of
        // these paths exists, so the order no longer decides which value wins.
        for kind in &[
            ValueKind::Markdown,
            ValueKind::Json,
            ValueKind::Blob,
            ValueKind::Text,
        ] {
            let path = Self::fs_path_for(&self.fs_root, spirit_pid, namespace, key, *kind)?;
            // `exists()` follows symlinks and answers true for a directory: a
            // link here would be read from outside this Spirit's area (I5), and
            // a hand-made directory would fail the read with `IsADirectory`.
            // `symlink_metadata` does neither.
            if matches!(fs::symlink_metadata(&path), Ok(m) if m.is_file()) {
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
    /// iteration).
    ///
    /// Merges the in-memory and filesystem sources **by logical key**.  The map
    /// is the read-through cache for the very value `read` would return, so a
    /// key held in both is ONE entry and the cached copy wins — the same
    /// precedence `read` applies (`:236`).  A union without that identity
    /// returns one key twice, which reaches a signed `decision.*` frame's
    /// `working_memory_digest_refs` and halves its effective scan cap.
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

        // In-memory entries.  EVERY matching key is recorded, including the
        // ones `limit` keeps out of the result: `limit` truncates output, it
        // must not change which keys the filesystem pass sees as already held.
        let mut cached: std::collections::HashSet<String> = std::collections::HashSet::new();
        {
            let map = self
                .in_mem
                .read()
                .expect("PrivateMemoryStore lock poisoned");
            for ((pid_k, ns_k, key), value) in map.iter() {
                if *pid_k == pid && *ns_k == ns && key.starts_with(prefix) {
                    cached.insert(key.clone());
                    if entries.len() >= limit {
                        continue;
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
        let ns_path = self
            .fs_root
            .join(pid.to_string())
            .join(Self::namespace_to_dirname(&ns)?);
        // `is_dir()` follows symlinks; `symlink_metadata` does not.  A link
        // planted at the namespace path would otherwise let a scan read entries
        // from outside this Spirit's area (I5).  Unlike `forget_principal`,
        // which errors here because a miscount would be signed into an Art.17
        // receipt, a read reports nothing and stays safe.
        if matches!(fs::symlink_metadata(&ns_path), Ok(m) if m.is_dir()) {
            for entry in fs::read_dir(&ns_path).map_err(MemoryError::Io)? {
                let entry = entry.map_err(MemoryError::Io)?;
                // Mirror `forget_principal` (`:440`): only a regular file whose
                // name carries a value-kind extension is a logical entry.  A
                // hand-created sub-directory or an editor backup is residue —
                // skipping it keeps ONE junk node from failing the whole
                // namespace scan, which `read_from_disk` would otherwise do.
                if !entry.file_type().map_err(MemoryError::Io)?.is_file() {
                    continue;
                }
                let file_name = entry.file_name();
                let Some((key, kind)) = file_name.to_str().and_then(Self::spill_name_parts) else {
                    continue;
                };
                if !key.starts_with(prefix) || cached.contains(key) {
                    continue;
                }
                if entries.len() >= limit {
                    break;
                }
                entries.push(MemoryEntry {
                    namespace: ns.clone(),
                    key: key.to_string(),
                    value: Self::read_from_disk(&entry.path(), kind)?,
                    timestamp_ns: 0,
                });
            }
        }

        Ok(entries)
    }

    /// Internal helper — remove all entries for a given principal_id
    /// from the private store.  Returns the count of deleted entries.
    pub fn forget_principal(&self, principal_id: &str) -> Result<u64, MemoryError> {
        // Identity of an erased entry, shared by both sources so a value that
        // is BOTH cached and spilled is counted exactly once:
        // (`<pid>/<ns_dirname>`, key), the key recovered by stripping the
        // value-kind extension `fs_path_for` appended.
        let mut erased: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

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

        // Remove from in-memory map — now recording identity, not a counter.
        {
            let mut map = self
                .in_mem
                .write()
                .expect("PrivateMemoryStore lock poisoned");
            for (pid, ns, key) in &to_remove {
                if map.remove(&(*pid, ns.clone(), key.clone())).is_some() {
                    let dir = format!("{pid}/{}", Self::namespace_to_dirname(ns)?);
                    erased.insert((dir, key.clone()));
                }
            }
        }

        // Markdown never enters the in-memory map (see `write`), so the map
        // cannot name its residue; the on-disk tree is authoritative and is a
        // strict superset of the map-derived subtrees. Reverse
        // `namespace_to_dirname` on each directory name — hex-decode, then
        // deserialize the namespace JSON — and keep the `Principal` ones
        // naming this principal. Undecodable names are not namespace dirs.
        // Errors are NEVER swallowed: a skipped directory is silent
        // under-deletion, which would make the Art.17 receipt claim an erasure
        // that did not happen. `file_type` does not traverse symlinks, so a
        // link planted under `fs_root` is not a dir and the walk cannot escape.
        let fs_root = self.fs_root.clone();
        let pid_dirs = match fs::read_dir(&fs_root) {
            Ok(d) => Some(d),
            // Nothing ever spilled — not an error.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(MemoryError::Io(e)),
        };
        for pid_entry in pid_dirs.into_iter().flatten() {
            let pid_entry = pid_entry.map_err(MemoryError::Io)?;
            if !pid_entry.file_type().map_err(MemoryError::Io)?.is_dir() {
                continue;
            }
            let pid_dir = pid_entry.path();
            let pid_dir_name = pid_entry.file_name();
            let mut cleaned = false;
            for ns_entry in fs::read_dir(&pid_dir).map_err(MemoryError::Io)? {
                let ns_entry = ns_entry.map_err(MemoryError::Io)?;
                let name = ns_entry.file_name();
                let decoded = name
                    .to_str()
                    .and_then(|n| hex::decode(n).ok())
                    .and_then(|b| serde_json::from_slice::<MemoryNamespace>(&b).ok());
                if !matches!(decoded, Some(MemoryNamespace::Principal { principal_id: p, .. })
                    if p == principal_id)
                {
                    continue;
                }
                // Decode BEFORE the type check (Trap 4): a name that DOES
                // decode to this principal's namespace but is not a real
                // directory is corruption or a containment attack, never junk.
                // `file_type` does not traverse, so a symlink fails here
                // instead of being followed — `read_dir` WOULD follow it and
                // `remove_dir_all` would then unlink only the link, counting
                // bytes it did not erase into a signed Art.17 proof.
                if !ns_entry.file_type().map_err(MemoryError::Io)?.is_dir() {
                    return Err(MemoryError::Io(std::io::Error::from(
                        std::io::ErrorKind::NotADirectory,
                    )));
                }
                let ns_dir = ns_entry.path();
                let dir = format!(
                    "{}/{}",
                    pid_dir_name.to_string_lossy(),
                    name.to_string_lossy()
                );
                for file in fs::read_dir(&ns_dir).map_err(MemoryError::Io)? {
                    let file = file.map_err(MemoryError::Io)?;
                    // Only a regular file whose name matches a value-kind
                    // extension is a logical entry.  A hand-created
                    // sub-directory, a non-UTF-8 name or an editor backup is
                    // residue: `remove_dir_all` below destroys it, but
                    // counting it would inflate the signed receipt.
                    if !file.file_type().map_err(MemoryError::Io)?.is_file() {
                        continue;
                    }
                    let file_name = file.file_name();
                    if let Some(key) = file_name.to_str().and_then(Self::key_from_spill_name) {
                        erased.insert((dir.clone(), key.to_string()));
                    }
                }
                fs::remove_dir_all(&ns_dir).map_err(MemoryError::Io)?;
                cleaned = true;
            }

            // Clean up the per-pid directory only if it is now empty.  A late
            // iterator error is propagated, not read as "not empty".
            if cleaned {
                let mut rest = fs::read_dir(&pid_dir).map_err(MemoryError::Io)?;
                if rest.next().transpose().map_err(MemoryError::Io)?.is_none() {
                    let _ = fs::remove_dir(&pid_dir);
                }
            }
        }

        Ok(erased.len() as u64)
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
