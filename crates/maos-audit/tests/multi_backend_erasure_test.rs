//! Story 9.2 — Decision F: every registered storage backend must prove erasure
//! OR prove non-applicability by construction.

#![forbid(unsafe_code)]

use std::path::Path;
use std::sync::Arc;

use maos_domain::memory::{MemoryError, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use rusqlite::OpenFlags;
use tempfile::TempDir;

const CANARY: &str = "DECISION-F-CANARY-9-2";
const TARGET_MARKDOWN_CANARY: &str = "PRIVATE-TARGET-MARKDOWN-13-5I";
const TARGET_SPILL_CANARY: &str = "PRIVATE-TARGET-SPILL-13-5I";
const BYSTANDER_CANARY: &str = "PRIVATE-BYSTANDER-13-5I";
const DEFAULT_CANARY: &str = "PRIVATE-DEFAULT-13-5I";

fn open_isolated_adapter(dir: &TempDir) -> Arc<maos_kernel_core::memory::MemoryManagerAdapter> {
    let fs_root = dir.path().join("memory");
    std::fs::create_dir_all(&fs_root).unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let private = Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
        fs_root,
        4 * 1024,
    ));
    let shared = Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db_path).unwrap());
    let principal_index =
        Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_with_global_legal_holds(
            &db_path, &db_path, 1,
        )
        .unwrap(),
    );
    Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        tl,
    ))
}

fn private_store_contains(fs_root: &Path, needle: &str) -> bool {
    // Fail loud: a `read_dir`/`read` failure swallowed into `false` would let
    // an unreadable surviving residue pass as content absence — the exact
    // shape of the null control this leg replaced.  `file_type` is used
    // instead of `Path::is_dir` so a symlink is not traversed.
    fn scan_dir(dir: &Path, needle: &str) -> bool {
        for entry in std::fs::read_dir(dir).expect("read private store directory") {
            let entry = entry.expect("read private store entry");
            let file_type = entry.file_type().expect("read private store entry type");
            if file_type.is_dir() {
                if scan_dir(&entry.path(), needle) {
                    return true;
                }
            } else if file_type.is_file()
                && String::from_utf8_lossy(
                    &std::fs::read(entry.path()).expect("read private store spill"),
                )
                .contains(needle)
            {
                return true;
            }
        }
        false
    }
    fs_root.exists() && scan_dir(fs_root, needle)
}

fn shared_store_contains(db_path: &Path, needle: &str) -> bool {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    let mut stmt = conn.prepare("SELECT value FROM shared_memory").unwrap();
    let rows: Vec<Vec<u8>> = stmt
        .query_map([], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(bytes)
        })
        .unwrap()
        .flatten()
        .collect();
    rows.iter()
        .any(|bytes| String::from_utf8_lossy(bytes).contains(needle))
}

/// Count `shared_memory` rows whose NAMESPACE COLUMN is a `Principal` variant.
///
/// Story 13.5h Trap 6: `shared_store_contains` above scans only the `value`
/// blob with no namespace filter, which is precisely why the pre-13.5h
/// discharge stayed green when its canary was swapped to a principal
/// namespace.  `MemoryNamespace` is serde externally tagged, so every
/// `Principal` variant serialises with the anchored prefix `{"Principal":`.
fn shared_store_principal_row_count(db_path: &Path) -> usize {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap();
    let count: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM shared_memory WHERE namespace LIKE '{"Principal":%'"#,
            [],
            |row| row.get(0),
        )
        .unwrap();
    usize::try_from(count).unwrap()
}

#[test]
fn multi_backend_erasure_partition_invariant() {
    let dir = TempDir::new().unwrap();
    let memory = open_isolated_adapter(&dir);
    let fs_root = dir.path().join("memory");
    let db_path = dir.path().join("audit.sqlite");
    let principal = "dave@example.org";
    let bystander = "bystander@example.org";

    let principal_ns = MemoryNamespace::Principal {
        principal_id: principal.into(),
        schema: "chat".into(),
    };
    memory
        .write(
            7,
            MemoryTier::Private,
            &principal_ns,
            "markdown",
            MemoryValue::Markdown(TARGET_MARKDOWN_CANARY.into()),
        )
        .unwrap();
    memory
        .write(
            7,
            MemoryTier::Private,
            &principal_ns,
            "spill",
            MemoryValue::Blob(
                TARGET_SPILL_CANARY
                    .as_bytes()
                    .iter()
                    .copied()
                    .chain(std::iter::repeat_n(b'x', 8 * 1024))
                    .collect(),
            ),
        )
        .unwrap();
    memory
        .write(
            8,
            MemoryTier::Private,
            &MemoryNamespace::Principal {
                principal_id: bystander.into(),
                schema: "chat".into(),
            },
            "bystander",
            MemoryValue::Markdown(BYSTANDER_CANARY.into()),
        )
        .unwrap();
    memory
        .write(
            9,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "default",
            MemoryValue::Markdown(DEFAULT_CANARY.into()),
        )
        .unwrap();

    // Plant the SAME canary in Shared tier under a non-principal namespace.
    // This represents legitimate cross-Spirit data that ADR-026 says must NOT
    // be erased by a principal forget.
    memory
        .write(
            7,
            MemoryTier::Shared,
            &MemoryNamespace::Coordination,
            "shared-marker",
            MemoryValue::Text(format!("{} shared", CANARY)),
        )
        .unwrap();

    // Reopen the adapter before forgetting: production one-shot uninstall
    // constructs a fresh PrivateMemoryStore whose cache is empty.
    drop(memory);
    let memory = open_isolated_adapter(&dir);
    let outcome = memory.forget_with_reason(principal, None).unwrap();
    let receipt = match outcome {
        maos_domain::memory::ForgetOutcome::Erased { receipt, .. } => receipt,
        other => panic!("principal forget unexpectedly suspended: {other:?}"),
    };

    // --- Backend enumeration + partition ---
    let mut proved_erased: Vec<&str> = Vec::new();
    let mut proved_principal_empty: Vec<&str> = Vec::new();

    // Private tier: both persistent value classes are gone after a restart.
    assert!(
        !private_store_contains(&fs_root, TARGET_MARKDOWN_CANARY),
        "Private tier still contains the target Markdown value after forget"
    );
    assert!(
        !private_store_contains(&fs_root, TARGET_SPILL_CANARY),
        "Private tier still contains the target non-Markdown spill after forget"
    );
    assert_eq!(
        receipt.deleted_entries, 2,
        "the signed effect count is distinct (pid, namespace, key) entries"
    );
    proved_erased.push("private");

    // ADR-026 positive retention: neither another principal nor a
    // non-principal namespace may be over-deleted.
    assert!(
        private_store_contains(&fs_root, BYSTANDER_CANARY),
        "forget over-deleted the bystander principal"
    );
    assert!(
        private_store_contains(&fs_root, DEFAULT_CANARY),
        "forget over-deleted the Default namespace"
    );

    // Principal index: address-only rows must be gone.
    let rows = maos_audit::subject_access_query(&db_path, principal).unwrap();
    assert!(
        rows.is_empty(),
        "principal_index still contains rows after forget"
    );
    proved_erased.push("principal_index");

    // --- Shared tier (Story 13.5h): principal-empty BY CONSTRUCTION ---
    //
    // (c) ADR-026 positive retention, asserted under its OWN name and kept
    // strictly separate from the principal-empty discharge below.  Fusing
    // these two claims into a single assertion is exactly what made this a
    // null control before 13.5h: surviving Coordination data proves no
    // OVER-erasure and says nothing at all about principal-emptiness.
    assert!(
        shared_store_contains(&db_path, CANARY),
        "ADR-026: legitimate cross-Spirit Coordination data must survive a principal forget"
    );

    // (a) The discharge proper: the Shared tier refuses subject-scoped PII at
    // its OWN entry point, on ALL THREE methods `SpiritMemoryView` exposes.
    // A partition that admits reads or scans of a planted row is not a
    // partition.  Assert the TYPED refusal, never the diagnostic sentence.
    let shared_write_err = memory
        .write(
            7,
            MemoryTier::Shared,
            &principal_ns,
            "must-not-land",
            MemoryValue::Text(CANARY.into()),
        )
        .expect_err("shared principal write must be refused");
    assert!(
        matches!(shared_write_err, MemoryError::NamespaceViolation(_)),
        "shared write refusal must be the typed partition: {shared_write_err:?}"
    );
    let shared_read_err = memory
        .read(7, MemoryTier::Shared, &principal_ns, "must-not-land")
        .expect_err("shared principal read must be refused");
    assert!(
        matches!(shared_read_err, MemoryError::NamespaceViolation(_)),
        "shared read refusal must be the typed partition: {shared_read_err:?}"
    );
    let shared_scan_err = memory
        .scan(7, MemoryTier::Shared, &principal_ns, "", 16)
        .expect_err("shared principal scan must be refused");
    assert!(
        matches!(shared_scan_err, MemoryError::NamespaceViolation(_)),
        "shared scan refusal must be the typed partition: {shared_scan_err:?}"
    );

    // (b) Namespace-column-filtered zero-row scan (Trap 6).
    assert_eq!(
        shared_store_principal_row_count(&db_path),
        0,
        "shared_memory must hold zero principal-namespaced rows"
    );
    proved_principal_empty.push("shared");

    // Collective/Loom tier: Decision D is the proof. A principal-shaped write
    // must be refused at the collective entry point before any store access.
    let collective_error = memory
        .write(
            7,
            MemoryTier::Collective,
            &principal_ns,
            "must-not-land",
            MemoryValue::Text(CANARY.into()),
        )
        .expect_err("collective principal write must be refused");
    assert!(
        collective_error.to_string().contains("partitioned out"),
        "refusal must be the Decision D partition: {collective_error}"
    );
    proved_principal_empty.push("loom");

    // Partition invariant: every registered backend is in exactly one bucket,
    // buckets are disjoint, and together they cover the registered set.
    let registered: std::collections::HashSet<&str> =
        maos_kernel_core::memory::REGISTERED_ERASURE_BACKENDS
            .iter()
            .copied()
            .collect();
    let covered: std::collections::HashSet<&str> = proved_erased.iter().copied().collect();
    let retained: std::collections::HashSet<&str> =
        proved_principal_empty.iter().copied().collect();

    assert!(
        covered.is_disjoint(&retained),
        "erased and principal-empty sets must be disjoint"
    );
    assert_eq!(
        covered
            .union(&retained)
            .copied()
            .collect::<std::collections::HashSet<_>>(),
        registered,
        "all registered backends must be accounted for"
    );
}
