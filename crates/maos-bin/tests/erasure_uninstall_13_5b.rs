//! Story 13.5b — proven-red coverage for shipped uninstall false-success paths.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use tempfile::TempDir;

const PRINCIPAL: &str = "held-uninstall@example.org";

struct Fixture {
    _dir: TempDir,
    data_home: PathBuf,
    memory_root: PathBuf,
    key_path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("create fixture directory");
        let data_home = dir.path().join("data");
        let memory_root = dir.path().join("memory");
        let key_path = dir.path().join("audit-signing.key");
        std::fs::create_dir_all(&data_home).expect("create data home");
        std::fs::create_dir_all(&memory_root).expect("create memory root");
        std::fs::write(&key_path, hex::encode([0x5bu8; 32])).expect("write audit key");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                .expect("secure audit key");
        }
        Self {
            _dir: dir,
            data_home,
            memory_root,
            key_path,
        }
    }

    fn audit_db(&self) -> PathBuf {
        self.data_home
            .join("maos")
            .join("audit")
            .join("transparency.sqlite")
    }

    fn proof_dir(&self) -> PathBuf {
        self.data_home.join("maos").join("erasure-proofs")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
        command
            .env("HOME", self._dir.path())
            .env("XDG_CONFIG_HOME", self._dir.path().join("config"))
            .env("XDG_DATA_HOME", &self.data_home)
            .env("MAOS_MEMORY_ROOT", &self.memory_root)
            .env("MAOS_AUDIT_KEY", &self.key_path)
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("MAOS_ONE_SHOT", "uninstall")
            .env("MAOS_SPIRIT_ID", "hello-spirit");
        command
    }

    fn run_uninstall(&self, region: Option<&str>) -> Output {
        let mut command = self.command();
        if let Some(region) = region {
            command.env("MAOS_REGION_HOME", region);
        } else {
            command.env_remove("MAOS_REGION_HOME");
        }
        command.output().expect("run one-shot uninstall")
    }

    fn seed_named_principal(&self, principal: &str, held: bool) {
        let audit_db = self.audit_db();
        std::fs::create_dir_all(audit_db.parent().expect("audit parent"))
            .expect("create audit parent");
        let private = Arc::new(PrivateMemoryStore::new(self.memory_root.clone(), 4 * 1024));
        let shared = Arc::new(SharedMemoryStore::open(&audit_db).expect("open shared store"));
        let principal_index =
            Arc::new(PrincipalNamespaceIndex::open(&audit_db).expect("open principal index"));
        let transparency_log = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&audit_db, &audit_db, 1)
                .expect("open transparency log"),
        );
        let memory = MemoryManagerAdapter::new(
            private,
            shared,
            principal_index,
            Arc::clone(&transparency_log),
        );
        let namespace = MemoryNamespace::Principal {
            principal_id: principal.into(),
            schema: "profile".into(),
        };
        memory
            .write(
                0,
                MemoryTier::Private,
                &namespace,
                "record",
                MemoryValue::Text("principal payload".into()),
            )
            .expect("seed principal row");
        if held {
            let outcome = memory
                .forget_with_reason(principal, Some("legal-hold:case-13-5b"))
                .expect("place legal hold");
            assert!(matches!(
                outcome,
                maos_domain::memory::ForgetOutcome::Suspended { .. }
            ));
        }
    }

    fn seed_principal(&self, held: bool) {
        self.seed_named_principal(PRINCIPAL, held);
    }

    /// Seed a `Markdown` record: filesystem-canonical, never cached in memory.
    fn seed_named_markdown_principal(&self, principal: &str) {
        let audit_db = self.audit_db();
        std::fs::create_dir_all(audit_db.parent().expect("audit parent"))
            .expect("create audit parent");
        let private = Arc::new(PrivateMemoryStore::new(self.memory_root.clone(), 4 * 1024));
        let shared = Arc::new(SharedMemoryStore::open(&audit_db).expect("open shared store"));
        let principal_index =
            Arc::new(PrincipalNamespaceIndex::open(&audit_db).expect("open principal index"));
        let transparency_log = Arc::new(
            TransparencyLogAdapter::open_with_global_legal_holds(&audit_db, &audit_db, 1)
                .expect("open transparency log"),
        );
        let memory = MemoryManagerAdapter::new(
            private,
            shared,
            principal_index,
            Arc::clone(&transparency_log),
        );
        memory
            .write(
                0,
                MemoryTier::Private,
                &MemoryNamespace::Principal {
                    principal_id: principal.into(),
                    schema: "profile".into(),
                },
                "dossier",
                MemoryValue::Markdown("# dossier\n\nprincipal payload\n".into()),
            )
            .expect("seed markdown record");
    }

    fn seed_markdown_principal(&self) {
        self.seed_named_markdown_principal(PRINCIPAL);
    }

    fn seed_held_principal(&self) {
        self.seed_principal(true);
    }

    fn seed_unheld_principal(&self) {
        self.seed_principal(false);
    }

    /// Plant a principal-namespaced row directly in `shared_memory` via raw SQL.
    ///
    /// Deliberately NOT written through `MemoryManagerAdapter`: since the
    /// Story 13.5h partition the adapter refuses `(Shared, Principal)` at the
    /// entry point, which is the whole point of the guard. Raw SQL reproduces
    /// the only way such a row can still exist — a Host upgraded from a
    /// pre-partition build, whose rows the partition renders unreachable but
    /// cannot erase (13.5h Trap 4).
    fn plant_pre_partition_shared_row(&self, principal: &str) {
        let audit_db = self.audit_db();
        std::fs::create_dir_all(audit_db.parent().expect("audit parent"))
            .expect("create audit parent");
        drop(SharedMemoryStore::open(&audit_db).expect("open shared store"));
        let namespace = serde_json::to_string(&MemoryNamespace::Principal {
            principal_id: principal.into(),
            schema: "profile".into(),
        })
        .expect("serialize principal namespace");
        let conn = rusqlite::Connection::open(&audit_db).expect("open shared db");
        conn.execute(
            "INSERT INTO shared_memory \
             (writer_spirit_pid, namespace, key, value, kind, timestamp_ns) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                0i64,
                namespace,
                "legacy-record",
                b"pre-partition payload".to_vec(),
                "text",
                1i64
            ],
        )
        .expect("plant pre-partition shared row");
    }
}

fn regular_files(path: &Path) -> usize {
    std::fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .count()
        })
        .unwrap_or(0)
}

fn spilled_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read private-memory directory") {
            let entry = entry.expect("read private-memory entry");
            let file_type = entry.file_type().expect("read private-memory entry type");
            if file_type.is_dir() {
                walk(&entry.path(), out);
            } else if file_type.is_file() {
                out.push(entry.path());
            }
        }
    }

    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

fn private_tree_contains(root: &Path, needle: &str) -> bool {
    // An observation failure is NOT proof of absence: fail loud rather than
    // report "erased" over residue we merely could not read.
    spilled_files(root).iter().any(|path| {
        String::from_utf8_lossy(&std::fs::read(path).expect("read private-memory spill"))
            .contains(needle)
    })
}

fn terminal(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "decode terminal JSON: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Story 13.5h — replaces 13.5b's `..._with_shared_coverage_gap` contract.
///
/// Proven-red contract: a region-pinned Host that erases everything it can
/// erase reports `erased`/exit 0, writes BOTH the proof bundle and the regional
/// teardown receipt, and now records the Shared tier as `VerifiedEmpty` with
/// `"shared"` named in the receipt's `stores_covered`.
///
/// That status is EARNED, not asserted: the producer counts
/// principal-namespaced rows in `shared_memory` (filtering on the namespace
/// column) and only then attests. 13.5h Trap 4 is what makes the count
/// load-bearing rather than decorative — the partition stops NEW principal
/// rows entering the tier, but a Host upgraded from a pre-partition build
/// could still hold rows written before the guard existed, and those are
/// unreachable rather than erased. Emitting `VerifiedEmpty` unconditionally
/// would be a fresh null control of exactly the kind 13.5h exists to remove,
/// so a non-zero count must degrade to `CoverageGap` and withhold `"shared"`
/// from `stores_covered`, driving `completed` false.
///
/// History: 13.5b's version asserted a `CoverageGap` naming this story as the
/// still-open owner of the Shared-tier hole; it in
/// turn replaced `regional_uninstall_refuses_to_attest_uncovered_shared_store`,
/// which asserted only `!status.success()` and was green for the wrong reason
/// (`completed` was structurally false, so a fully successful erasure
/// terminated `failed`/exit 5 after its proof was already on disk).
#[test]
fn regional_uninstall_attests_shared_tier_verified_empty() {
    let fixture = Fixture::new();
    fixture.seed_unheld_principal();
    let output = fixture.run_uninstall(Some("eu-west"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "region-pinned uninstall must succeed; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let terminal = terminal(&output);
    assert_eq!(terminal["outcome"], "erased");

    // The regional teardown receipt is produced.
    let receipts: Vec<_> = std::fs::read_dir(fixture.proof_dir())
        .expect("read proof dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with("regional-teardown-eu-west-"))
        })
        .collect();
    assert_eq!(
        receipts.len(),
        1,
        "exactly one regional teardown receipt expected, found {receipts:?}"
    );

    // Story 13.5h: `"shared"` is now a REQUIRED store and must appear in the
    // attested coverage set, with the cascade reporting completion.
    let receipt: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipts[0]).expect("read teardown receipt"))
            .expect("decode teardown receipt");
    let covered = receipt["forget_cascade"]["stores_covered"]
        .as_array()
        .expect("stores_covered array");
    assert!(
        covered.iter().any(|s| s == "shared"),
        "the shared tier must be attested as covered: {covered:?}"
    );
    assert_eq!(
        receipt["forget_cascade"]["completed"], true,
        "cascade must complete once every required store is covered"
    );

    // The Shared tier is attested as verified-empty in the proof bundle.
    let proof_path = terminal["proof_path"].as_str().expect("proof_path");
    let proof: serde_json::Value =
        serde_json::from_slice(&std::fs::read(proof_path).expect("read proof bundle"))
            .expect("decode proof bundle");
    let shared = proof["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|category| category["name"] == "shared")
        .expect("shared category present");
    assert_eq!(
        shared["status"]["status"], "VerifiedEmpty",
        "shared must be verified-empty after the 13.5h partition: {shared}"
    );
    assert!(
        shared["status"]["reason"].is_null(),
        "VerifiedEmpty carries no reason payload: {shared}"
    );
}

/// Story 13.5h Trap 4 — the partition makes pre-existing Shared principal rows
/// UNREACHABLE, not erased.
///
/// Pins the fail-closed response for a Host upgraded from a pre-partition
/// build: the `shared` category degrades from `VerifiedEmpty` to a
/// `CoverageGap` naming the residue, `"shared"` is withheld from
/// `stores_covered`, and the region-pinned run refuses to attest a completed
/// teardown rather than signing a success that did not happen.
///
/// This test is also what proves its sibling
/// `regional_uninstall_attests_shared_tier_verified_empty` is not vacuous:
/// hard-code `CategoryStatus::VerifiedEmpty` in `run_uninstall_cascade_inner`
/// instead of counting principal-namespaced rows, and THIS test reds while the
/// sibling stays green. A `VerifiedEmpty` that is asserted rather than measured
/// is precisely the null control Story 13.5h exists to remove.
#[test]
fn regional_uninstall_refuses_to_attest_pre_partition_shared_residue() {
    let fixture = Fixture::new();
    fixture.seed_unheld_principal();
    fixture.plant_pre_partition_shared_row(PRINCIPAL);
    let output = fixture.run_uninstall(Some("eu-west"));

    assert_ne!(
        output.status.code(),
        Some(0),
        "a teardown cannot succeed while unerasable principal rows remain in the shared tier; \
         stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // No regional teardown receipt may be signed for an incomplete cascade.
    let receipts: Vec<_> = std::fs::read_dir(fixture.proof_dir())
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .filter(|name| name.starts_with("regional-teardown-"))
                .collect()
        })
        .unwrap_or_default();
    assert!(
        receipts.is_empty(),
        "no teardown receipt may be signed while the shared tier holds residue: {receipts:?}"
    );

    // The proof bundle still lands, and reports the residue honestly.
    let bundles: Vec<PathBuf> = std::fs::read_dir(fixture.proof_dir())
        .expect("read proof dir")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "bundle"))
        .collect();
    assert_eq!(
        bundles.len(),
        1,
        "expected one proof bundle, got {bundles:?}"
    );
    let proof: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&bundles[0]).expect("read proof bundle"))
            .expect("decode proof bundle");
    let shared = proof["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|category| category["name"] == "shared")
        .expect("shared category present");
    assert_eq!(
        shared["status"]["status"], "CoverageGap",
        "shared must degrade to CoverageGap when residue is present: {shared}"
    );
    let reason = shared["status"]["reason"].as_str().expect("gap reason");
    let reason_lower = reason.to_ascii_lowercase();
    assert!(
        reason_lower.contains("not erased") && reason_lower.contains("unreachable"),
        "the gap must state that residue is unreachable but NOT erased: {reason}"
    );
    assert!(
        reason.contains('1'),
        "the gap must report how many rows remain: {reason}"
    );
}

/// Shared residue is fail-closed even without a configured home region.
///
/// The proof category, not regional receipt construction, is the source of
/// truth. A non-regional uninstall must therefore reject the same residue.
#[test]
fn non_regional_uninstall_rejects_pre_partition_shared_residue() {
    let fixture = Fixture::new();
    fixture.seed_unheld_principal();
    fixture.plant_pre_partition_shared_row(PRINCIPAL);
    let output = fixture.run_uninstall(None);

    assert_ne!(
        output.status.code(),
        Some(0),
        "uninstall cannot succeed while Shared principal residue remains; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(terminal(&output)["outcome"], "failed");
    assert_eq!(
        regular_files(&fixture.proof_dir()),
        1,
        "the failed terminal must retain the signed partial proof"
    );
}

/// A pre-partition Shared write did not create a `principal_index` row.
///
/// The uninstall must inspect Shared residue before treating an empty private
/// principal set as `NotFound`, and must persist the resulting coverage gap.
#[test]
fn shared_only_pre_partition_residue_is_reported() {
    let fixture = Fixture::new();
    fixture.plant_pre_partition_shared_row(PRINCIPAL);
    let output = fixture.run_uninstall(None);

    let result = terminal(&output);
    assert_eq!(
        result["outcome"], "failed",
        "Shared-only residue must not disappear behind NotFound: {result}"
    );
    assert_eq!(
        regular_files(&fixture.proof_dir()),
        1,
        "Shared-only residue must be recorded in a signed partial proof"
    );
}

#[test]
fn held_uninstall_is_non_success_and_writes_no_complete_proof() {
    let fixture = Fixture::new();
    fixture.seed_held_principal();
    let output = fixture.run_uninstall(None);

    assert_eq!(output.status.code(), Some(3));
    let terminal = terminal(&output);
    assert_eq!(terminal["outcome"], "held");
    assert_eq!(terminal["held_principal_ids"][0], PRINCIPAL);
    // Nothing was destroyed, so there is nothing to attest.
    assert!(terminal["erased_principal_ids"]
        .as_array()
        .expect("erased_principal_ids array")
        .is_empty());
    assert!(terminal["proof_path"].is_null());
    assert_eq!(
        regular_files(&fixture.proof_dir()),
        0,
        "held uninstall must not sign a complete proof"
    );
}

/// Story 13.5b review, D2 (party-mode consensus, option (b)).
///
/// Proven-red contract for the mixed case the shipped code could not express:
/// principal A is erased and principal B is held in the SAME run. The terminal
/// must be `held`/exit 3 AND carry both sets plus a partial proof; the proof
/// must mark B as a legal-hold `CoverageGap`; A must be gone from subject
/// access and B must survive; and no regional receipt may be written.
/// Dropping the partial-proof path, or reverting to a bare held terminal that
/// discards the erasures, must red this test.
#[test]
fn mixed_held_uninstall_writes_partial_proof() {
    const ERASED: &str = "erased-alongside-hold@example.org";

    let fixture = Fixture::new();
    fixture.seed_named_markdown_principal(ERASED);
    fixture.seed_named_principal(PRINCIPAL, true);

    let output = fixture.run_uninstall(Some("eu-west"));
    assert_eq!(
        output.status.code(),
        Some(3),
        "a held principal keeps the run non-success; stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let terminal = terminal(&output);
    assert_eq!(terminal["outcome"], "held");
    assert_eq!(terminal["held_principal_ids"][0], PRINCIPAL);
    assert_eq!(
        terminal["erased_principal_ids"]
            .as_array()
            .expect("erased_principal_ids array"),
        &vec![serde_json::Value::String(ERASED.into())],
        "the terminal must name what it destroyed"
    );
    assert_eq!(
        terminal["deleted_entries"], 1,
        "the erased principal's durable Markdown entry must be counted"
    );

    // A partial proof exists and is explicit about the hold.
    let proof_path = terminal["proof_path"]
        .as_str()
        .expect("a run that destroyed data must attest it");
    let proof: serde_json::Value =
        serde_json::from_slice(&std::fs::read(proof_path).expect("read proof bundle"))
            .expect("decode proof bundle");
    let hold = proof["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|category| category["name"] == "legal_hold")
        .expect("legal_hold category present");
    assert_eq!(hold["status"]["status"], "CoverageGap");
    let reason = hold["status"]["reason"]
        .as_str()
        .expect("legal_hold must be a CoverageGap");
    assert!(
        reason.contains(PRINCIPAL),
        "the gap must name the held principal: {reason}"
    );

    // A held run is not a teardown.
    let receipts = std::fs::read_dir(fixture.proof_dir())
        .expect("read proof dir")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("regional-teardown-")
        })
        .count();
    assert_eq!(receipts, 0, "a held run must write no teardown receipt");

    // Effects, not claims: A is gone, B survives.
    let audit_db = fixture.audit_db();
    assert!(
        maos_audit::subject_access_query(&audit_db, ERASED)
            .expect("subject access for erased principal")
            .is_empty(),
        "the erased principal must be gone"
    );
    assert!(
        !maos_audit::subject_access_query(&audit_db, PRINCIPAL)
            .expect("subject access for held principal")
            .is_empty(),
        "the held principal must survive"
    );
}

#[test]
fn not_found_uninstall_has_distinct_terminal_code() {
    let fixture = Fixture::new();
    let output = fixture.run_uninstall(None);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(terminal(&output)["outcome"], "not_found");
    assert_eq!(regular_files(&fixture.proof_dir()), 0);
}

#[test]
fn erased_uninstall_is_success_with_machine_receipt() {
    let fixture = Fixture::new();
    fixture.seed_unheld_principal();
    let output = fixture.run_uninstall(None);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(terminal(&output)["outcome"], "erased");
    assert_eq!(regular_files(&fixture.proof_dir()), 1);
}

#[test]
fn proof_write_failure_has_failed_terminal_code() {
    let fixture = Fixture::new();
    fixture.seed_unheld_principal();
    std::fs::create_dir_all(fixture.proof_dir().parent().expect("proof parent"))
        .expect("create proof parent");
    std::fs::write(fixture.proof_dir(), b"not-a-directory").expect("plant proof directory failure");

    let output = fixture.run_uninstall(None);
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(terminal(&output)["outcome"], "failed");
}

#[test]
fn private_forget_reports_filesystem_removal_failure() {
    let fixture = Fixture::new();
    let audit_db = fixture.audit_db();
    std::fs::create_dir_all(audit_db.parent().expect("audit parent")).expect("create audit parent");
    let private = Arc::new(PrivateMemoryStore::new(
        fixture.memory_root.clone(),
        4 * 1024,
    ));
    let shared = Arc::new(SharedMemoryStore::open(&audit_db).expect("open shared store"));
    let principal_index =
        Arc::new(PrincipalNamespaceIndex::open(&audit_db).expect("open principal index"));
    let transparency_log = Arc::new(
        TransparencyLogAdapter::open_with_global_legal_holds(&audit_db, &audit_db, 1)
            .expect("open transparency log"),
    );
    let memory = MemoryManagerAdapter::new(
        Arc::clone(&private),
        shared,
        principal_index,
        transparency_log,
    );
    let namespace = MemoryNamespace::Principal {
        principal_id: PRINCIPAL.into(),
        schema: "profile".into(),
    };
    memory
        .write(
            7,
            MemoryTier::Private,
            &namespace,
            "record",
            MemoryValue::Blob(vec![0x5b; 8 * 1024]),
        )
        .expect("seed private row");

    let pid_dir = fixture.memory_root.join("7");
    let namespace_dir = std::fs::read_dir(&pid_dir)
        .expect("read pid directory")
        .next()
        .expect("principal namespace directory")
        .expect("read namespace entry")
        .path();
    std::fs::remove_dir_all(&namespace_dir).expect("replace namespace directory");
    std::fs::write(&namespace_dir, b"simulated undeletable subtree")
        .expect("plant non-directory removal failure");

    let error = private
        .forget_principal(PRINCIPAL)
        .expect_err("filesystem removal failure must be reported");
    assert!(error.to_string().contains("directory") || error.to_string().contains("Directory"));
}

/// AC3's authority boundary: operator erase must be reachable from exactly one
/// composition-root call site and from no Spirit-facing surface.
///
/// 13.5b review: the earlier form counted the literals `"port.erase(spirit_pid"`
/// and `"collective_port.erase"`, so renaming the binding defeated it silently.
/// It now keys on the *method name* — the one thing a caller cannot rename —
/// across every Spirit-facing file, and checks the trait method is absent from
/// both `SpiritMemoryView` and each `MemoryTier::Collective` arm.
#[test]
fn collective_erase_has_one_operator_route_and_zero_spirit_reach() {
    let composition_root = include_str!("../src/main.rs");
    let spirit_view = include_str!("../../maos-kernel-core/src/memory/for_spirit.rs");
    let kernel_memory = include_str!("../../maos-kernel-core/src/memory/mod.rs");
    let scope_vocabulary = include_str!("../../maos-domain/src/invariants/i1.rs");

    assert_eq!(
        composition_root.matches(".erase(spirit_pid").count(),
        1,
        "collective erase must have exactly one composition-root call site, \
         whatever the port binding is named"
    );
    assert_eq!(
        composition_root
            .matches("\"collective.operator.erase\"")
            .count(),
        1,
        "collective erase must append exactly one audit-side intent"
    );
    assert!(
        composition_root.contains("tokio::task::spawn_blocking(move ||"),
        "sync adapter must be invoked from spawn_blocking (Trap 3)"
    );

    // No Spirit-facing file may name the erase method at all, under ANY
    // binding: a helper delegation or a renamed field still has to write
    // `.erase(`. `SpiritMemoryView` additionally must not name the concept.
    for (label, source) in [
        ("SpiritMemoryView", spirit_view),
        ("kernel memory tiers", kernel_memory),
    ] {
        assert_eq!(
            source.matches(".erase(").count(),
            0,
            "{label} must contain no collective erase delegation"
        );
    }
    assert!(
        !spirit_view.contains("collective-erase") && !spirit_view.contains("collective_erase"),
        "SpiritMemoryView must expose no collective erase path"
    );

    // The port trait method must not be mentioned by any `MemoryTier::Collective`
    // arm in the kernel — those are the three Spirit-reachable tier dispatches.
    for (index, arm) in kernel_memory.match_indices("MemoryTier::Collective") {
        let tail = &kernel_memory[index..kernel_memory.len().min(index + 600)];
        assert!(
            !tail.contains("erase"),
            "a MemoryTier::Collective arm reaches erase (byte offset {index}): {}",
            arm
        );
    }

    assert!(
        !scope_vocabulary.contains("CollectiveErase"),
        "operator erase must not gain a Spirit capability Scope"
    );
}

#[test]
fn legal_hold_list_and_release_are_operable_without_auto_erasure() {
    let fixture = Fixture::new();
    fixture.seed_held_principal();

    let list = fixture
        .command()
        .env("MAOS_ONE_SHOT", "legal-hold-list")
        .output()
        .expect("list legal holds");
    assert!(
        list.status.success(),
        "list failed: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let holds: serde_json::Value =
        serde_json::from_slice(&list.stdout).expect("decode legal hold list");
    assert_eq!(holds.as_array().map(Vec::len), Some(1));
    assert_eq!(holds[0]["principal_id"], PRINCIPAL);

    let release = fixture
        .command()
        .env("MAOS_ONE_SHOT", "legal-hold-release")
        .env("MAOS_LEGAL_HOLD_PRINCIPAL", PRINCIPAL)
        .output()
        .expect("release legal hold");
    assert!(
        release.status.success(),
        "release failed: {}",
        String::from_utf8_lossy(&release.stderr)
    );
    let receipt: serde_json::Value =
        serde_json::from_slice(&release.stdout).expect("decode release receipt");
    assert_eq!(receipt["released"], true);
    assert_eq!(receipt["auto_erased"], false);
    assert!(
        !maos_audit::subject_access_query(&fixture.audit_db(), PRINCIPAL)
            .expect("subject access after release")
            .is_empty(),
        "release re-queues eligibility but must never auto-erase"
    );
}

/// Story 13.5i — the production one-shot uninstall constructs a fresh private
/// store with an empty cache. Principal erasure must therefore discover the
/// filesystem record itself, delete its content, and attest one removed entry.
#[test]
fn private_tier_markdown_is_erased_by_the_forget_cascade() {
    let fixture = Fixture::new();
    fixture.seed_markdown_principal();

    let before = spilled_files(&fixture.memory_root);
    assert_eq!(
        before.len(),
        1,
        "fixture must spill exactly one Markdown record under {}; found {before:?}",
        fixture.memory_root.display()
    );
    assert_eq!(
        before[0].extension().and_then(|ext| ext.to_str()),
        Some("md"),
        "expected a .md spill: {before:?}"
    );
    assert!(
        private_tree_contains(&fixture.memory_root, "principal payload"),
        "fixture must contain the principal payload before erasure"
    );

    let output = fixture.run_uninstall(None);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let terminal = terminal(&output);
    assert_eq!(terminal["outcome"], "erased");

    assert!(
        maos_audit::subject_access_query(&fixture.audit_db(), PRINCIPAL)
            .expect("subject access after erase")
            .is_empty(),
        "principal_index erasure is durable"
    );
    assert!(
        spilled_files(&fixture.memory_root).is_empty(),
        "the principal's filesystem record must be removed"
    );
    assert!(
        !private_tree_contains(&fixture.memory_root, "principal payload"),
        "principal content must be absent after the forget cascade"
    );

    let proof_path = terminal["proof_path"].as_str().expect("proof_path");
    let proof: serde_json::Value =
        serde_json::from_slice(&std::fs::read(proof_path).expect("read proof bundle"))
            .expect("decode proof bundle");
    let namespace_category = proof["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|category| category["name"] == "memory_namespace")
        .expect("memory_namespace category present");
    assert_eq!(namespace_category["status"]["status"], "Removed");
    assert_eq!(
        namespace_category["status"]["count"], 1,
        "the signed proof must carry the real private-tier effect count"
    );

    // AC3's newly-armed path: `claims_removal` is true for the first time now
    // that the private count is non-zero, so the P18 empty-proof-set rejection
    // at `proof.rs:383-395` is reachable. Prove the bundle still verifies.
    let typed: maos_audit::erasure::proof::ErasureProof =
        serde_json::from_slice(&std::fs::read(proof_path).expect("read proof bundle"))
            .expect("decode typed proof bundle");
    assert!(
        !typed.subject_exclusion_proofs.is_empty(),
        "a bundle claiming a non-zero removal must carry exclusion proofs"
    );
    maos_audit::erasure::proof::verify_erasure_proof(
        &typed,
        &maos_audit::sealed_export::derive_pubkey(&[0x5bu8; 32]),
    )
    .expect("the signed bundle must verify now that it claims a real removal");
}
