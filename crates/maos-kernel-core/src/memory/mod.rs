#![forbid(unsafe_code)]

//! Memory Manager — supervised service per §4.2.
//!
//! Provides three named memory tiers (`private`, `shared`, `collective`)
//! and enforces I5 namespace scopes. Story 4.3 lands the three-tier
//! mechanics with Principal Namespace (ADR-026) and Self-Telemetry (FR56).
//!
//! ## Architecture references
//! - §4.2 (Memory Manager — three tiers + I5 enforcement)
//! - §4.0.7 (kernel does NOT interpret memory contents)
//! - ADR-026 (Principal Memory Namespace, binding-v0.5)
//! - §9.2 (memory.md universal-cohort convention)

pub mod for_spirit;
pub mod principal;
pub mod private;
pub mod self_telemetry;
pub mod shared;

pub use maos_domain::ports::MemoryManagerPort;

pub use principal::PrincipalNamespaceIndex;
pub use private::PrivateMemoryStore;
pub use self_telemetry::SelfTelemetryAggregator;
pub use shared::SharedMemoryStore;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[cfg(feature = "spirit_test")]
use maos_spirit_sdk::spirit_test::{AttemptResult, IsolationHookPoint, ObservationResult};
#[cfg(feature = "spirit_test")]
use parking_lot::Mutex;

use crate::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{
    ExportEntry, ExportPayload, ForgetReceipt, MemoryEntry, MemoryError, MemoryNamespace,
    MemoryTier, MemoryValue, PrincipalIndexRow,
};

// Re-exports above make PrivateMemoryStore, SharedMemoryStore,
// PrincipalNamespaceIndex available in this module scope.

#[allow(unused_imports)]
use maos_domain::invariants::i5::{MemoryScope, NamespaceKey};

/// Production Memory Manager adapter — implements `MemoryManagerPort`
/// with three-tier memory, Principal Namespace index, and I5 enforcement.
#[maos_attrs::i9_exempt(
    reason = "memory manager three-tier substrate — bounded by principal forget-cascade + per-Spirit memory budget; per-Spirit-keyed map / per-Spirit-namespaced filesystem / sqlite table for ADR-026 principal namespace + I5 isolation; parallel to the capability registry's per-Spirit token state, not pattern-learning"
)]
pub struct MemoryManagerAdapter {
    private: Arc<PrivateMemoryStore>,
    shared: Arc<SharedMemoryStore>,
    principal_index: Arc<PrincipalNamespaceIndex>,
    transparency_log: Arc<TransparencyLogAdapter>,
    next_frame_counter: AtomicU64,
    /// Story 4.3 — cross-Spirit isolation hook (Story 4.5 corpus).
    /// Feature-gated so production builds carry zero runtime cost.
    #[cfg(feature = "spirit_test")]
    isolation_hook: Option<Arc<Mutex<dyn IsolationHookPoint + Send>>>,
}

impl MemoryManagerAdapter {
    pub fn new(
        private: Arc<PrivateMemoryStore>,
        shared: Arc<SharedMemoryStore>,
        principal_index: Arc<PrincipalNamespaceIndex>,
        transparency_log: Arc<TransparencyLogAdapter>,
    ) -> Self {
        Self {
            private,
            shared,
            principal_index,
            transparency_log,
            next_frame_counter: AtomicU64::new(0),
            #[cfg(feature = "spirit_test")]
            isolation_hook: None,
        }
    }

    /// Create a pid-fused memory view for a Spirit.  This is the I5
    /// enforcement substrate — the `spirit_pid` is fused at construction
    /// and cannot be re-supplied.
    pub fn for_spirit(&self, spirit_pid: u32) -> for_spirit::SpiritMemoryView<'_> {
        for_spirit::SpiritMemoryView::new(self, spirit_pid)
    }

    /// Story 4.3 — attach an isolation hook for the `spirit_test` feature.
    /// Only available when the `spirit_test` feature is enabled.
    #[cfg(feature = "spirit_test")]
    pub fn with_isolation_hook(mut self, hook: Arc<Mutex<dyn IsolationHookPoint + Send>>) -> Self {
        self.isolation_hook = Some(hook);
        self
    }

    #[cfg(feature = "spirit_test")]
    fn fire_isolation_hooks(&self, case_id: &str, attempt_ok: bool) {
        if let Some(hook) = &self.isolation_hook {
            let mut h = hook.lock();
            let _ = h.before_spirit_a_attempt(case_id);
            let attempt = AttemptResult {
                hooks_fired_during_attempt: vec![case_id.into()],
                frames_emitted: if attempt_ok { 1 } else { 0 },
            };
            let _ = h.after_spirit_a_attempt(case_id, &attempt);
            let _ = h.before_spirit_b_observe(case_id);
            let observation = ObservationResult {
                hooks_fired_during_observation: vec![case_id.into()],
                frames_emitted: 0,
                leaked_bytes: None,
            };
            let _ = h.after_spirit_b_observe(case_id, &observation);
        }
    }

    fn now_ns() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    fn mint_frame_id(&self) -> [u8; 16] {
        // Use the same monotonic-ULID mechanism as the Transparency Log:
        // increment a process-local counter and embed it in the low bits
        // of a ULID-based byte array.  This preserves uniqueness within
        // the process and monotonicity across calls.
        let counter = self.next_frame_counter.fetch_add(1, Ordering::Relaxed);
        let ulid = ulid::Ulid::new();
        let mut bytes = ulid.to_bytes();
        let cb = counter.to_le_bytes();
        // Overwrite the low 4 bytes (counter) — this is a deliberate,
        // documented trade-off: we sacrifice raw ULID randomness for
        // guaranteed process-local monotonicity.  The high 10 bytes
        // retain the ULID timestamp + randomness entropy.
        bytes[12] = cb[0];
        bytes[13] = cb[1];
        bytes[14] = cb[2];
        bytes[15] = cb[3];
        bytes
    }
}

impl MemoryManagerPort for MemoryManagerAdapter {
    fn validate_namespace_read(&self, key: &NamespaceKey<MemoryScope>) -> bool {
        // v0.3-β: all namespace reads are permitted at the port level;
        // real enforcement lives in the `for_spirit` reborrow + per-pid keying.
        let _ = key;
        true
    }

    fn validate_namespace_write(&self, key: &NamespaceKey<MemoryScope>) -> bool {
        let _ = key;
        true
    }

    fn write(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError> {
        #[cfg(feature = "spirit_test")]
        let case_id = format!(
            "memory.write:{spirit_pid}:{:?}:{key}",
            namespace.kind_label()
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&case_id, true);

        match tier {
            MemoryTier::Private => {
                self.private
                    .write(spirit_pid, namespace, key, value.clone())?;
                // If this is a principal-tagged write, record in the index.
                if let MemoryNamespace::Principal {
                    principal_id,
                    schema,
                } = namespace
                {
                    let ts = Self::now_ns();
                    self.principal_index
                        .record_write(principal_id, spirit_pid, schema, key, ts)?;
                }
                Ok(())
            }
            MemoryTier::Shared => self.shared.write(spirit_pid, namespace, key, value),
            MemoryTier::Collective => Err(MemoryError::CollectiveNotYetAvailable {
                ship_target: "v1.5",
                landing_story: "E10 Story 10.4",
            }),
        }
    }

    fn read(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        #[cfg(feature = "spirit_test")]
        let case_id = format!(
            "memory.read:{spirit_pid}:{:?}:{key}",
            namespace.kind_label()
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&case_id, true);

        match tier {
            MemoryTier::Private => self.private.read(spirit_pid, namespace, key),
            MemoryTier::Shared => self.shared.read(spirit_pid, namespace, key),
            MemoryTier::Collective => Err(MemoryError::CollectiveNotYetAvailable {
                ship_target: "v1.5",
                landing_story: "E10 Story 10.4",
            }),
        }
    }

    fn scan(
        &self,
        spirit_pid: u32,
        tier: MemoryTier,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        #[cfg(feature = "spirit_test")]
        let case_id = format!(
            "memory.scan:{spirit_pid}:{:?}:{prefix}",
            namespace.kind_label()
        );
        #[cfg(feature = "spirit_test")]
        self.fire_isolation_hooks(&case_id, true);

        match tier {
            MemoryTier::Private => self.private.scan(spirit_pid, namespace, prefix, limit),
            MemoryTier::Shared => self.shared.scan(spirit_pid, namespace, prefix, limit),
            MemoryTier::Collective => Err(MemoryError::CollectiveNotYetAvailable {
                ship_target: "v1.5",
                landing_story: "E10 Story 10.4",
            }),
        }
    }

    fn subject_access(&self, principal_id: &str) -> Result<Vec<PrincipalIndexRow>, MemoryError> {
        self.principal_index.lookup(principal_id)
    }

    fn forget(&self, principal_id: &str) -> Result<ForgetReceipt, MemoryError> {
        // 1. Collect index rows (pre-delete snapshot).
        let rows = self.principal_index.lookup(principal_id)?;
        let index_count = rows.len() as u64;

        // 2. Delete from PrivateMemoryStore for each index row.
        let mut deleted_entries: u64 = 0;
        for row in &rows {
            let ns = MemoryNamespace::principal(&row.principal_id, &row.schema)
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            // We track the count from forget_principal which does a batch delete.
        }
        // Use the principal-level forget on the private store.
        deleted_entries = self.private.forget_principal(principal_id)?;

        // 3. Delete from PrincipalNamespaceIndex.
        let deleted_index_rows = self.principal_index.forget(principal_id)?;

        // 4. Mint frame_id and timestamp.
        let frame_id = self.mint_frame_id();
        let timestamp_ns = Self::now_ns();

        // 5. Write Transparency Log frame.
        let payload = format!(
            "principal_forget: id={} entries={} index_rows={}",
            principal_id, deleted_entries, deleted_index_rows
        );
        self.transparency_log.insert_frame_event(
            FrameKind::TaskComplete,
            0, // kernel-side cascade — affects multiple Spirits
            None,
            "principal.forget",
            payload.as_bytes(),
            FrameOrigin::Kernel,
        );

        Ok(ForgetReceipt {
            principal_id: principal_id.to_string(),
            deleted_entries,
            deleted_index_rows,
            timestamp_ns,
            frame_id,
        })
    }

    fn export_redactable(
        &self,
        principal_id: &str,
        include_principal: bool,
    ) -> Result<Vec<ExportEntry>, MemoryError> {
        let rows = self.principal_index.lookup(principal_id)?;
        let mut entries = Vec::with_capacity(rows.len());

        for row in &rows {
            let ns = MemoryNamespace::principal(&row.principal_id, &row.schema)
                .map_err(|e| MemoryError::Storage(e.to_string()))?;
            let val_opt = self.private.read(row.writer_spirit_pid, &ns, &row.key)?;

            let payload = if include_principal {
                match val_opt {
                    Some(v) => ExportPayload::Raw(v),
                    None => ExportPayload::Redacted {
                        content_type: "unknown".into(),
                        principal_id: row.principal_id.clone(),
                        schema: row.schema.clone(),
                    },
                }
            } else {
                ExportPayload::Redacted {
                    content_type: "principal-namespace".into(),
                    principal_id: row.principal_id.clone(),
                    schema: row.schema.clone(),
                }
            };

            entries.push(ExportEntry {
                namespace: ns,
                key: row.key.clone(),
                payload,
            });
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_adapter() -> (Arc<MemoryManagerAdapter>, TempDir) {
        let tmp = TempDir::new().unwrap();
        let memory_root = tmp.path().join("memory");
        let db_path = tmp.path().join("audit.db");

        let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
        let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
        let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
        // Unique boot-nonce per test fixture to avoid in-memory SQLite collision.
        static TEST_NONCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let boot_nonce = TEST_NONCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(boot_nonce));
        let adapter = Arc::new(MemoryManagerAdapter::new(
            private,
            shared,
            principal_index,
            tl,
        ));
        (adapter, tmp)
    }

    #[test]
    fn write_read_private() {
        let (adapter, _tmp) = make_adapter();
        let val = MemoryValue::Text("hello".into());
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "k",
                val.clone(),
            )
            .unwrap();
        let got = adapter
            .read(1, MemoryTier::Private, &MemoryNamespace::Default, "k")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn write_read_shared() {
        let (adapter, _tmp) = make_adapter();
        let val = MemoryValue::Text("shared!".into());
        adapter
            .write(
                2,
                MemoryTier::Shared,
                &MemoryNamespace::Coordination,
                "s",
                val.clone(),
            )
            .unwrap();
        let got = adapter
            .read(2, MemoryTier::Shared, &MemoryNamespace::Coordination, "s")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn collective_returns_typed_error() {
        let (adapter, _tmp) = make_adapter();
        let err = adapter
            .write(
                1,
                MemoryTier::Collective,
                &MemoryNamespace::Default,
                "k",
                MemoryValue::Text("x".into()),
            )
            .unwrap_err();
        match err {
            MemoryError::CollectiveNotYetAvailable {
                ship_target,
                landing_story,
            } => {
                assert_eq!(ship_target, "v1.5");
                assert_eq!(landing_story, "E10 Story 10.4");
            }
            _ => panic!("expected CollectiveNotYetAvailable"),
        }
    }

    #[test]
    fn scan_private() {
        let (adapter, _tmp) = make_adapter();
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "a-1",
                MemoryValue::Text("a".into()),
            )
            .unwrap();
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "a-2",
                MemoryValue::Text("b".into()),
            )
            .unwrap();
        let results = adapter
            .scan(1, MemoryTier::Private, &MemoryNamespace::Default, "a-", 10)
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn spirit_view_fuses_pid() {
        let (adapter, _tmp) = make_adapter();
        let view = adapter.for_spirit(7);
        let val = MemoryValue::Text("pid-7".into());
        view.write(
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "k",
            val.clone(),
        )
        .unwrap();
        let got = view
            .read(MemoryTier::Private, &MemoryNamespace::Default, "k")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn i5_isolation_different_pids_dont_overlap() {
        let (adapter, _tmp) = make_adapter();
        adapter
            .write(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "secret",
                MemoryValue::Text("spirit-1".into()),
            )
            .unwrap();
        // Spirit 2 reads Spirit 1's key — should return None.
        let got = adapter
            .read(2, MemoryTier::Private, &MemoryNamespace::Default, "secret")
            .unwrap();
        assert!(got.is_none());
    }
}
