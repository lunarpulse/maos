//! Story 13.5i — restart-backed controls for private-tier principal erasure.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use maos_domain::memory::{MemoryError, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use tempfile::TempDir;

const PRINCIPAL: &str = "private-residue@example.org";
const BYSTANDER: &str = "private-bystander@example.org";
const MARKDOWN_CANARY: &str = "PRIVATE-MARKDOWN-RESIDUE-13-5I";
const SPILL_CANARY: &str = "PRIVATE-SPILL-RESIDUE-13-5I";
const BYSTANDER_CANARY: &str = "PRIVATE-BYSTANDER-RESIDUE-13-5I";
const DEFAULT_CANARY: &str = "PRIVATE-DEFAULT-RESIDUE-13-5I";

struct Fixture {
    _dir: TempDir,
    fs_root: PathBuf,
    db_path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("create fixture directory");
        let fs_root = dir.path().join("memory");
        let db_path = dir.path().join("audit.sqlite");
        std::fs::create_dir_all(&fs_root).expect("create memory root");
        Self {
            _dir: dir,
            fs_root,
            db_path,
        }
    }

    fn open(&self) -> (Arc<PrivateMemoryStore>, MemoryManagerAdapter) {
        let private = Arc::new(PrivateMemoryStore::new(self.fs_root.clone(), 4 * 1024));
        let shared = Arc::new(SharedMemoryStore::open(&self.db_path).expect("open shared store"));
        let principal_index =
            Arc::new(PrincipalNamespaceIndex::open(&self.db_path).expect("open principal index"));
        let transparency_log = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&self.db_path, &self.db_path, 1)
                .expect("open transparency log"),
        );
        let memory = MemoryManagerAdapter::new(
            Arc::clone(&private),
            shared,
            principal_index,
            transparency_log,
        );
        (private, memory)
    }

    fn fresh_private(&self) -> PrivateMemoryStore {
        PrivateMemoryStore::new(self.fs_root.clone(), 4 * 1024)
    }
}

fn principal_namespace(principal_id: &str) -> MemoryNamespace {
    MemoryNamespace::Principal {
        principal_id: principal_id.into(),
        schema: "profile".into(),
    }
}

fn write_private(
    memory: &MemoryManagerAdapter,
    spirit_pid: u32,
    namespace: &MemoryNamespace,
    key: &str,
    value: MemoryValue,
) {
    memory
        .write(spirit_pid, MemoryTier::Private, namespace, key, value)
        .expect("seed private value");
}

fn large_blob(canary: &str) -> MemoryValue {
    MemoryValue::Blob(
        canary
            .as_bytes()
            .iter()
            .copied()
            .chain(std::iter::repeat_n(b'x', 8 * 1024))
            .collect(),
    )
}

fn tree_contains(root: &Path, needle: &str) -> bool {
    if !root.exists() {
        return false;
    }
    for entry in std::fs::read_dir(root).expect("read private tree") {
        let entry = entry.expect("read private tree entry");
        let file_type = entry.file_type().expect("read private tree entry type");
        if file_type.is_dir() {
            if tree_contains(&entry.path(), needle) {
                return true;
            }
        } else if file_type.is_file()
            && String::from_utf8_lossy(&std::fs::read(entry.path()).expect("read spill"))
                .contains(needle)
        {
            return true;
        }
    }
    false
}

fn namespace_dirname(namespace: &MemoryNamespace) -> String {
    hex::encode(serde_json::to_vec(namespace).expect("serialize namespace"))
}

#[test]
fn restart_forget_erases_markdown_content() {
    let fixture = Fixture::new();
    {
        let (_private, memory) = fixture.open();
        write_private(
            &memory,
            7,
            &principal_namespace(PRINCIPAL),
            "dossier",
            MemoryValue::Markdown(MARKDOWN_CANARY.into()),
        );
    }
    assert!(tree_contains(&fixture.fs_root, MARKDOWN_CANARY));

    fixture
        .fresh_private()
        .forget_principal(PRINCIPAL)
        .expect("forget after restart");
    assert!(!tree_contains(&fixture.fs_root, MARKDOWN_CANARY));
}

#[test]
fn restart_forget_erases_non_markdown_spill_content() {
    let fixture = Fixture::new();
    {
        let (_private, memory) = fixture.open();
        write_private(
            &memory,
            7,
            &principal_namespace(PRINCIPAL),
            "large-blob",
            large_blob(SPILL_CANARY),
        );
    }
    assert!(tree_contains(&fixture.fs_root, SPILL_CANARY));

    fixture
        .fresh_private()
        .forget_principal(PRINCIPAL)
        .expect("forget after restart");
    assert!(!tree_contains(&fixture.fs_root, SPILL_CANARY));
}

#[test]
fn restart_forget_reports_distinct_persisted_entry_count() {
    let fixture = Fixture::new();
    {
        let (_private, memory) = fixture.open();
        let namespace = principal_namespace(PRINCIPAL);
        write_private(
            &memory,
            7,
            &namespace,
            "dossier",
            MemoryValue::Markdown(MARKDOWN_CANARY.into()),
        );
        write_private(
            &memory,
            7,
            &namespace,
            "large-blob",
            large_blob(SPILL_CANARY),
        );
    }

    let deleted = fixture
        .fresh_private()
        .forget_principal(PRINCIPAL)
        .expect("forget after restart");

    assert_eq!(deleted, 2, "the receipt counts persisted logical entries");
}

#[test]
fn forget_counts_cached_and_spilled_value_exactly_once() {
    let fixture = Fixture::new();
    let (private, memory) = fixture.open();
    write_private(
        &memory,
        7,
        &principal_namespace(PRINCIPAL),
        "cached-and-spilled",
        large_blob(SPILL_CANARY),
    );
    assert!(
        tree_contains(&fixture.fs_root, SPILL_CANARY),
        "the fixture must actually spill, or this control degenerates to a cache-only test"
    );

    let deleted = private
        .forget_principal(PRINCIPAL)
        .expect("forget cached and spilled value");

    assert_eq!(deleted, 1, "one logical key must never be counted twice");
    assert!(!tree_contains(&fixture.fs_root, SPILL_CANARY));
}

#[test]
fn forget_preserves_bystander_and_default_namespace_content() {
    let fixture = Fixture::new();
    {
        let (_private, memory) = fixture.open();
        write_private(
            &memory,
            7,
            &principal_namespace(PRINCIPAL),
            "target",
            MemoryValue::Markdown(MARKDOWN_CANARY.into()),
        );
        write_private(
            &memory,
            8,
            &principal_namespace(BYSTANDER),
            "bystander",
            MemoryValue::Markdown(BYSTANDER_CANARY.into()),
        );
        write_private(
            &memory,
            9,
            &MemoryNamespace::Default,
            "default",
            MemoryValue::Markdown(DEFAULT_CANARY.into()),
        );
    }

    fixture
        .fresh_private()
        .forget_principal(PRINCIPAL)
        .expect("forget target principal");

    assert!(!tree_contains(&fixture.fs_root, MARKDOWN_CANARY));
    assert!(tree_contains(&fixture.fs_root, BYSTANDER_CANARY));
    assert!(tree_contains(&fixture.fs_root, DEFAULT_CANARY));
}

#[test]
#[cfg_attr(not(unix), ignore = "pid symlink containment is unix-only")]
fn forget_does_not_follow_pid_directory_symlink() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        let outside = fixture._dir.path().join("outside");
        let namespace_dir = outside.join(namespace_dirname(&principal_namespace(PRINCIPAL)));
        std::fs::create_dir_all(&namespace_dir).expect("create outside namespace");
        let outside_file = namespace_dir.join("dossier.md");
        std::fs::write(&outside_file, MARKDOWN_CANARY).expect("plant outside canary");
        symlink(&outside, fixture.fs_root.join("7")).expect("plant pid symlink");

        let result = fixture.fresh_private().forget_principal(PRINCIPAL);
        assert!(
            result.is_err(),
            "erasure must fail closed on a numeric PID symlink: {result:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&outside_file).expect("outside file survives"),
            MARKDOWN_CANARY
        );
    }
}

#[test]
#[cfg_attr(not(unix), ignore = "mode-bit fail-closed I/O is unix-only")]
fn forget_fails_closed_when_pid_directory_is_unreadable() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let fixture = Fixture::new();
        let pid_dir = fixture.fs_root.join("7");
        let namespace_dir = pid_dir.join(namespace_dirname(&principal_namespace(PRINCIPAL)));
        std::fs::create_dir_all(&namespace_dir).expect("create principal namespace");
        std::fs::write(namespace_dir.join("dossier.md"), MARKDOWN_CANARY)
            .expect("plant principal spill");
        std::fs::set_permissions(&pid_dir, std::fs::Permissions::from_mode(0o000))
            .expect("make pid directory unreadable");
        if std::fs::read_dir(&pid_dir).is_ok() {
            std::fs::set_permissions(&pid_dir, std::fs::Permissions::from_mode(0o700))
                .expect("restore pid directory permissions");
            eprintln!(
                "skipped: this process bypasses directory mode bits (root?), so the \
                 fail-closed path cannot be provoked here"
            );
            return;
        }

        let result = fixture.fresh_private().forget_principal(PRINCIPAL);
        std::fs::set_permissions(&pid_dir, std::fs::Permissions::from_mode(0o700))
            .expect("restore pid directory permissions");

        assert!(
            matches!(result, Err(MemoryError::Io(_))),
            "unreadable residue must fail the erasure: {result:?}"
        );
    }
}

/// AC4(d) row **M8**, as specified: an implementation that counts *files*
/// instead of distinct logical keys reports `1` here, because two of these
/// three entries are RAM-only and never touch the filesystem.  The pre-13.5i
/// suite had no CI-executed detector for that mutant.
#[test]
fn forget_counts_inline_only_entries_that_never_spill() {
    let fixture = Fixture::new();
    let (private, memory) = fixture.open();
    let namespace = principal_namespace(PRINCIPAL);
    write_private(
        &memory,
        7,
        &namespace,
        "inline-1",
        MemoryValue::Text("a".into()),
    );
    write_private(
        &memory,
        7,
        &namespace,
        "inline-2",
        MemoryValue::Text("b".into()),
    );
    write_private(&memory, 7, &namespace, "spilled", large_blob(SPILL_CANARY));

    let deleted = private
        .forget_principal(PRINCIPAL)
        .expect("forget inline and spilled entries");

    assert_eq!(
        deleted, 3,
        "two RAM-only entries plus one spill are three distinct logical keys"
    );
}

/// **M9** (13.5i code review): pid-level containment is not enough.  A symlink
/// planted at the *namespace* level is followed by `read_dir`, while
/// `remove_dir_all` unlinks only the link — so a walk that does not type-check
/// the namespace entry counts external bytes it never erased into an
/// Ed25519-signed Article 17 proof.  Fail closed instead.
#[test]
#[cfg_attr(not(unix), ignore = "namespace symlink containment is unix-only")]
fn forget_does_not_follow_namespace_directory_symlink() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let fixture = Fixture::new();
        let outside = fixture._dir.path().join("outside");
        std::fs::create_dir_all(&outside).expect("create outside namespace");
        let outside_file = outside.join("dossier.md");
        std::fs::write(&outside_file, MARKDOWN_CANARY).expect("plant outside canary");

        let pid_dir = fixture.fs_root.join("7");
        std::fs::create_dir_all(&pid_dir).expect("create pid directory");
        symlink(
            &outside,
            pid_dir.join(namespace_dirname(&principal_namespace(PRINCIPAL))),
        )
        .expect("plant namespace symlink");

        let result = fixture.fresh_private().forget_principal(PRINCIPAL);

        assert!(
            matches!(result, Err(MemoryError::Io(_))),
            "a namespace symlink must fail the erasure, never be counted: {result:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&outside_file).expect("outside file survives"),
            MARKDOWN_CANARY
        );
    }
}

/// `deleted_entries` counts logical keys, not filesystem nodes.  The private
/// tier's Markdown area is deliberately operator-editable, so a namespace
/// directory can legitimately hold things the store never wrote: they are
/// destroyed with the subtree but must not be attested as erased entries.
/// The empty key is the boundary case — its spill is named `.bin`, whose
/// `file_stem()` is `".bin"`, so a stem-based identity double-counts it.
#[test]
fn forget_counts_logical_keys_not_filesystem_nodes() {
    let fixture = Fixture::new();
    let (private, memory) = fixture.open();
    write_private(
        &memory,
        7,
        &principal_namespace(PRINCIPAL),
        "",
        large_blob(SPILL_CANARY),
    );

    let ns_dir = fixture
        .fs_root
        .join("7")
        .join(namespace_dirname(&principal_namespace(PRINCIPAL)));
    // Named `notes.md` deliberately: a directory whose name matches a
    // value-kind extension is only excluded from the count by the
    // regular-file guard, not by the extension check.
    std::fs::create_dir_all(ns_dir.join("notes.md")).expect("plant sub-directory");
    std::fs::write(ns_dir.join("notes.md").join("a.md"), "nested residue")
        .expect("plant nested residue");
    std::fs::write(ns_dir.join("dossier.md~"), "editor backup").expect("plant backup file");

    let deleted = private
        .forget_principal(PRINCIPAL)
        .expect("forget principal with hand-created residue");

    assert_eq!(
        deleted, 1,
        "one spilled logical key at the empty key, whatever else sits beside it"
    );
    assert!(
        !ns_dir.exists(),
        "the whole namespace subtree is destroyed regardless of what it held"
    );
}
