//! Story 10.4a AC1 — proven-red tests for the Loom-lite collective tier.
//!
//! Per Epic 9 §A1, proven-red is a **DEV-PASS** gate: every check the story
//! relies on is proven REAL by showing it fails on bad input (RED) *before*
//! succeeding on good input (GREEN).  This file covers the six AC1 vectors.
//!
//! # Vectors
//!
//! 1. **NFR-Test-9 grep injection** — the `check-loom` structural grep gate
//!    catches orchestration/backing-store vocabulary leaking into kernel
//!    source.  Inject a blocklisted identifier → RED; clean source → GREEN.
//! 2. **Dependency-closure gate** — RED: the `check-fr47` manifest scan flags
//!    the backing-store crate `sqlx`.  GREEN: the REAL `check-dependency-closure`
//!    gate confirms `maos-kernel-core`'s transitive closure is clean.
//! 3. **I9 behavioral pattern-write** — N collective writes via the injected
//!    port leave ZERO retention in the kernel's own stores (only the backing
//!    store mock holds the data).  The kernel mediates, never learns.
//! 4. **Loom-down typed timeout** — an unreachable Loom-lite host yields a
//!    typed `CollectivePortError::Unreachable`/`::Timeout` (no panic, no hang)
//!    within a bounded wall-clock window.
//! 5. **RTO drill** — the REAL `check-rto-gate` (NFR-Ops-9) goes RED when the
//!    weekly drill's restore time exceeds the 4 h SLA and GREEN within it.
//! 6. **Weekly cross-check tamper** — a single tampered `frame_id`, re-derived
//!    through the live Merkle primitive, drives `MigrationResult::verify()` to
//!    `MerkleRootMismatch` (RED); matching roots → `Ok` (GREEN).

use std::sync::Arc;
use std::time::{Duration, Instant};

use maos_domain::memory::{
    CollectiveErrorKind, MemoryEntry, MemoryError, MemoryNamespace, MemoryTier, MemoryValue,
};
use maos_domain::ports::{CollectiveMemoryPort, CollectivePortError, MemoryManagerPort};
use maos_kernel_core::api::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
    TransparencyLogAdapter,
};
use maos_loom_lite::adapter::LoomLiteAdapter;
use maos_loom_lite::store::{LoomLiteStore, StoreConfig};

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i10::{JournalEntry, LifecycleEvent};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::ports::scheduler::SpiritSchedulerPort;
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::{
    cap_audit, cap_quota, CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::security::manifest::{
    CapabilitiesRequired, ClassSection, PostureSection, ResourceCaps, SandboxConfig,
};
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use parking_lot::Mutex;

// ═══════════════════════════════════════════════════════════════════════
// Shared helpers
// ═══════════════════════════════════════════════════════════════════════

fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask sits directly under the workspace root")
}

fn live_researcher_collective_method_body<'a>(source: &'a str, method: &str) -> &'a str {
    let implementation = source
        .find("impl researcher::ResearcherCollectivePort for LiveResearcherCollectivePort {")
        .expect("LiveResearcherCollectivePort implementation exists");
    let method_start = implementation
        + source[implementation..]
            .find(&format!("fn {method}("))
            .expect("collective method exists");
    let body_start = method_start
        + source[method_start..]
            .find('{')
            .expect("collective method body opens");
    let mut depth = 0;
    for (offset, byte) in source.as_bytes()[body_start..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[body_start + 1..body_start + offset];
                }
            }
            _ => {}
        }
    }
    panic!("collective method body closes");
}

/// Run the xtask binary with the given args (built by `cargo test`).
fn run_xtask(args: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("failed to spawn the xtask binary")
}

fn write_file(root: &std::path::Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 1 — NFR-Test-9 Loom-not-in-kernel structural grep
// ═══════════════════════════════════════════════════════════════════════

/// RED: a blocklisted orchestration identifier (`Planner`) leaking into
/// kernel-shaped source makes the NFR-Test-9 grep gate fail.
#[test]
fn story_10_4a_ac1_nfr_test_9_grep_red() {
    let dir = tempfile::tempdir().unwrap();
    // Inject a backing-store/orchestration symbol the gate is sworn to keep
    // out of kernel-core.  (The gate is a syn AST visitor, so the violation is
    // an identifier, not a comment.)
    write_file(
        dir.path(),
        "src/leak.rs",
        "pub struct Planner { id: u32 }\n",
    );

    let blocklist = workspace_root().join("xtask/loom-blocklist.toml");
    let allowlist = workspace_root().join("xtask/loom-allowlist.toml");
    let out = run_xtask(&[
        "check-loom",
        "--json",
        "--path",
        dir.path().to_str().unwrap(),
        "--blocklist",
        blocklist.to_str().unwrap(),
        "--allowlist",
        allowlist.to_str().unwrap(),
    ]);

    assert!(
        !out.status.success(),
        "RED: gate MUST fail when 'Planner' leaks into kernel source"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("gate emits JSON report");
    assert_eq!(report["passed"], false, "report.passed must be false");
    let violations = report["violations"]
        .as_array()
        .expect("violations is an array");
    assert!(
        violations
            .iter()
            .any(|v| v["identifier"] == "Planner" && v["kind"] == "ItemStruct"),
        "violation must name the blocklisted identifier, got: {violations:?}"
    );
}

/// RED (backing-store vocabulary): a forbidden backing-store crate name
/// (`sqlx`) leaking into kernel-shaped source — as a `use` path OR an
/// identifier — makes the NFR-Test-9 grep fail.  This is the load-bearing
/// expanded denominator (AC1 review: the original RED only injected the
/// orchestration symbol `Planner`, leaving `sqlx`/`postgres` leaks uncaught).
#[test]
fn story_10_4a_ac1_nfr_test_9_backing_store_red() {
    let dir = tempfile::tempdir().unwrap();
    // A `use sqlx::query;` — the gate now scans EVERY path segment, so the
    // leading `sqlx` is flagged (not just the rightmost `query`).
    write_file(dir.path(), "src/leak.rs", "use sqlx::query;\n");

    let blocklist = workspace_root().join("xtask/loom-blocklist.toml");
    let allowlist = workspace_root().join("xtask/loom-allowlist.toml");
    let out = run_xtask(&[
        "check-loom",
        "--json",
        "--path",
        dir.path().to_str().unwrap(),
        "--blocklist",
        blocklist.to_str().unwrap(),
        "--allowlist",
        allowlist.to_str().unwrap(),
    ]);

    assert!(
        !out.status.success(),
        "RED: gate MUST fail when 'sqlx' leaks into kernel source"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("gate emits JSON report");
    assert_eq!(report["passed"], false);
    let violations = report["violations"].as_array().expect("violations array");
    assert!(
        violations.iter().any(|v| v["identifier"] == "sqlx"),
        "violation must name sqlx, got: {violations:?}"
    );
}

/// GREEN: clean kernel source (no orchestration vocabulary) passes the gate.
#[test]
fn story_10_4a_ac1_nfr_test_9_grep_green() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "src/clean.rs",
        "pub fn add(a: u32, b: u32) -> u32 { a + b }\n",
    );

    let blocklist = workspace_root().join("xtask/loom-blocklist.toml");
    let allowlist = workspace_root().join("xtask/loom-allowlist.toml");
    let out = run_xtask(&[
        "check-loom",
        "--json",
        "--path",
        dir.path().to_str().unwrap(),
        "--blocklist",
        blocklist.to_str().unwrap(),
        "--allowlist",
        allowlist.to_str().unwrap(),
    ]);

    assert!(
        out.status.success(),
        "GREEN: clean source MUST pass — stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 2 — dependency-closure gate
// ═══════════════════════════════════════════════════════════════════════
//
// The kernel mediates collective-tier access via the injected
// CollectiveMemoryPort but must NEVER depend on a backing-store crate.
// Two real gates enforce this:
//   • check-fr47              — manifest dependency scan (injectable via --path);
//   • check-dependency-closure — runs `cargo tree -p maos-kernel-core` and flags
//     sqlx/tokio-postgres/pgvector/deadpool-postgres in the transitive closure.
// RED proves the scan catches the forbidden backing-store crate `sqlx`; GREEN
// proves the live kernel-core closure is clean of the entire stack.

/// RED: a manifest declaring the backing-store crate `sqlx`, scanned against a
/// denylist that names the stack, fails the dependency-closure scan gate.
#[test]
fn story_10_4a_ac1_dependency_closure_red() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "Cargo.toml",
        "[package]\n\
         name = \"fake-kernel\"\n\
         version = \"0.0.0\"\n\
         edition = \"2021\"\n\
         \n\
         [dependencies]\n\
         sqlx = \"0.8\"\n",
    );
    // A denylist naming the Postgres/pgvector stack the kernel must not pull.
    write_file(
        dir.path(),
        "denylist.toml",
        "vendor-sdk-denylist = [\"sqlx\", \"tokio-postgres\", \"pgvector\", \"deadpool-postgres\"]\n",
    );
    write_file(dir.path(), "allowlist.toml", "allowed = []\n");

    let out = run_xtask(&[
        "check-fr47",
        "--json",
        "--path",
        dir.path().to_str().unwrap(),
        "--denylist",
        dir.path().join("denylist.toml").to_str().unwrap(),
        "--allowlist",
        dir.path().join("allowlist.toml").to_str().unwrap(),
    ]);

    assert!(
        !out.status.success(),
        "RED: gate MUST fail when sqlx is declared"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("gate emits JSON report");
    assert_eq!(report["passed"], false);
    let violations = report["violations"].as_array().unwrap();
    assert!(
        violations.iter().any(|v| v["dependency"] == "sqlx"),
        "violation must name sqlx, got: {violations:?}"
    );
}

/// GREEN: the REAL check-dependency-closure gate on the live workspace —
/// maos-kernel-core's transitive closure is free of the backing-store stack.
#[test]
fn story_10_4a_ac1_dependency_closure_green() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-dependency-closure", "--json"])
        .current_dir(workspace_root())
        .output()
        .expect("failed to run xtask");

    assert!(
        out.status.success(),
        "GREEN: kernel-core closure MUST be free of sqlx/tokio-postgres/pgvector — stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("gate emits JSON report");
    assert_eq!(report["passed"], true);
    assert_eq!(report["violations"], serde_json::Value::Array(vec![]));
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 3 — I9 behavioral pattern-write (zero kernel retention)
// ═══════════════════════════════════════════════════════════════════════

/// Recording backing-store mock: captures every mediated write and lets the
/// kernel read it back through the collective tier — but it is the ONLY place
/// the data lives.  Uses `parking_lot::Mutex` per rule rs-parking-lot.
struct RecordingPort {
    write_count: Mutex<usize>,
    read_count: Mutex<usize>,
    scan_count: Mutex<usize>,
    kv: Mutex<Vec<(u32, MemoryNamespace, String, MemoryValue)>>,
}

impl RecordingPort {
    fn new() -> Self {
        Self {
            write_count: Mutex::new(0),
            read_count: Mutex::new(0),
            scan_count: Mutex::new(0),
            kv: Mutex::new(Vec::new()),
        }
    }

    fn writes(&self) -> usize {
        *self.write_count.lock()
    }

    fn reads(&self) -> usize {
        *self.read_count.lock()
    }

    fn scans(&self) -> usize {
        *self.scan_count.lock()
    }
}

impl CollectiveMemoryPort for RecordingPort {
    fn write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), CollectivePortError> {
        self.kv
            .lock()
            .push((spirit_pid, namespace.clone(), key.to_string(), value));
        *self.write_count.lock() += 1;
        Ok(())
    }

    fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, CollectivePortError> {
        *self.read_count.lock() += 1;
        let guard = self.kv.lock();
        Ok(guard
            .iter()
            .rev()
            .find(|(pid, ns, k, _)| *pid == spirit_pid && ns == namespace && k == key)
            .map(|(_, _, _, v)| v.clone()))
    }

    fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, CollectivePortError> {
        *self.scan_count.lock() += 1;
        let guard = self.kv.lock();
        let mut out = Vec::new();
        for (pid, ns, k, v) in guard.iter().rev() {
            if *pid == spirit_pid && ns == namespace && k.starts_with(prefix) {
                if let Ok(entry) = MemoryEntry::new(namespace.clone(), k.clone(), v.clone(), 0) {
                    out.push(entry);
                }
                if out.len() >= limit {
                    break;
                }
            }
        }
        Ok(out)
    }
}

/// Build a real `MemoryManagerAdapter` against tempdir-backed stores, with the
/// collective port injected.
fn adapter_with_port(port: Arc<dyn CollectiveMemoryPort>) -> Arc<MemoryManagerAdapter> {
    let tmp = tempfile::tempdir().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    // Leak the TempDir so the on-disk SQLite files survive the test.
    std::mem::forget(tmp);

    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(unique_nonce()));
    Arc::new(
        MemoryManagerAdapter::new(private, shared, principal_index, tl)
            .with_collective_port(Some(port)),
    )
}

fn unique_nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

/// I9: after N collective writes the kernel holds NONE of the content in its
/// own private/shared stores — the backing store (mock) is the sole holder.
/// The kernel mediates and audits; it does not learn or index patterns.
///
/// # P22 — known limitation of this vector (documented, not hidden)
///
/// The `None` assertions below are TRUE BY CONSTRUCTION: the kernel's
/// `MemoryTier::Collective` arm ONLY delegates to the injected port — it has
/// no code path that copies collective content into Private/Shared.  So this
/// vector proves the kernel does NOT retain today, but it cannot by itself
/// detect a FUTURE regression where someone adds a side write into a local
/// tier alongside the port call (the `None` reads would still hold as long as
/// the copy targeted a DIFFERENT key).  The structural guard against that is
/// `check-service-boundary` + the I9 structural-state lint (blocks new
/// persistent kernel fields), plus the proven-red companion
/// `i9_retention_detected_red` which shows the assertion DOES fire on a
/// kernel-local (Private) write.  A stronger future vector would inject a port
/// that also flips an `AtomicBool` side-channel and assert the kernel never
/// reads Private/Shared after a Collective write — tracked, not blocking.
#[test]
fn story_10_4a_ac1_i9_zero_kernel_retention() {
    let port = Arc::new(RecordingPort::new());
    let adapter = adapter_with_port(port.clone());

    let n = 8;
    for i in 0..n {
        adapter
            .write(
                1,
                MemoryTier::Collective,
                &MemoryNamespace::Default,
                &format!("pattern-{i}"),
                MemoryValue::Text(format!("payload-{i}")),
            )
            .expect("collective write delegates to the injected port");
    }

    // The backing store received every write.
    assert_eq!(
        port.writes(),
        n,
        "mock must observe all {n} collective writes"
    );

    // ── Zero retention in the kernel's own stores ───────────────────────
    // Private and Shared reads of the SAME keys return None: the kernel never
    // copied collective content into its local tiers.
    for i in 0..n {
        let key = format!("pattern-{i}");
        assert_eq!(
            adapter
                .read(1, MemoryTier::Private, &MemoryNamespace::Default, &key)
                .unwrap(),
            None,
            "Private tier must NOT retain collective content ({key})"
        );
        assert_eq!(
            adapter
                .read(1, MemoryTier::Shared, &MemoryNamespace::Default, &key)
                .unwrap(),
            None,
            "Shared tier must NOT retain collective content ({key})"
        );
    }

    // Scans of the kernel-local tiers return nothing for the collective keys.
    let private_scan = adapter
        .scan(
            1,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "pattern-",
            100,
        )
        .unwrap();
    let shared_scan = adapter
        .scan(
            1,
            MemoryTier::Shared,
            &MemoryNamespace::Default,
            "pattern-",
            100,
        )
        .unwrap();
    assert!(private_scan.is_empty(), "Private scan must be empty");
    assert!(shared_scan.is_empty(), "Shared scan must be empty");

    // ── The data lives ONLY behind the collective tier (round-trips via port) ──
    let read_back = adapter
        .read(
            1,
            MemoryTier::Collective,
            &MemoryNamespace::Default,
            "pattern-0",
        )
        .unwrap();
    assert_eq!(
        read_back,
        Some(MemoryValue::Text("payload-0".into())),
        "collective read must round-trip through the injected port"
    );

    let collective_scan = adapter
        .scan(
            1,
            MemoryTier::Collective,
            &MemoryNamespace::Default,
            "pattern-",
            100,
        )
        .unwrap();
    assert_eq!(
        collective_scan.len(),
        n,
        "collective scan must surface all {n} mediated writes"
    );
}

/// RED companion (§A1 "both branches"): if the kernel DID retain content in a
/// local tier, the zero-retention assertion would FIRE.  We prove the I9 check
/// is non-vacuous by writing to the Private tier (kernel-retained) and showing
/// the same key is readable there — the negative control that makes the GREEN
/// vector's `None` assertions meaningful.
#[test]
fn story_10_4a_ac1_i9_retention_detected_red() {
    let adapter = adapter_with_port(Arc::new(RecordingPort::new()));
    // Write to a KERNEL-LOCAL tier (Private) — this IS retained.
    adapter
        .write(
            1,
            MemoryTier::Private,
            &MemoryNamespace::Default,
            "pattern-0",
            MemoryValue::Text("payload-0".into()),
        )
        .expect("private write");
    // The Private read returns Some — proving the zero-retention assertion in
    // the GREEN vector is meaningful (it would fire here, not vacuously pass).
    assert_eq!(
        adapter
            .read(
                1,
                MemoryTier::Private,
                &MemoryNamespace::Default,
                "pattern-0"
            )
            .unwrap(),
        Some(MemoryValue::Text("payload-0".into())),
        "Private tier DOES retain — the I9 zero-retention check is non-vacuous"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 4 — Loom-down → typed timeout/unreachable error
// ═══════════════════════════════════════════════════════════════════════

/// When the Loom-lite backing store is unreachable, every sync port op returns
/// a typed `CollectivePortError::Unreachable`/`::Timeout` — no panic, no hang —
/// within a bounded wall-clock window.  Driven from a `spawn_blocking` thread,
/// matching the documented MCP→kernel→adapter topology.
#[test]
fn story_10_4a_ac1_loom_down_typed_timeout() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("multi-thread runtime");
    let handle = rt.handle().clone();

    // Pool creation is lazy (deadpool never connects until `get()`), so a
    // non-existent host constructs successfully.
    let store = rt
        .block_on(async {
            LoomLiteStore::new(StoreConfig {
                connection_string: "host=127.0.0.1 port=1 dbname=loom_lite".into(),
                vector_dim: 1536,
                pool_size: 2,
                timeout_ms: 800,
                home_region: String::new(),
                home_team: String::new(),
            })
            .await
        })
        .expect("store pool creation must NOT connect eagerly");

    let adapter = LoomLiteAdapter::new(Arc::new(store), handle, Duration::from_millis(2500));

    let start = Instant::now();
    // Drive the SYNC trait method from spawn_blocking — the production edge.
    let result = rt.block_on(async move {
        tokio::task::spawn_blocking(move || {
            adapter.write(
                1,
                &MemoryNamespace::Default,
                "k",
                MemoryValue::Text("x".into()),
            )
        })
        .await
        .expect("spawn_blocking task did not panic")
    });
    let elapsed = start.elapsed();

    // AC1: halt-safe, typed error — and it MUST be bounded (no hang).
    assert!(
        elapsed < Duration::from_secs(15),
        "Loom-down op must return within a bounded window, took {elapsed:?}"
    );

    let err = result.expect_err("unreachable host must yield a typed error, not Ok");
    assert!(
        matches!(
            err,
            CollectivePortError::Unreachable { .. } | CollectivePortError::Timeout { .. }
        ),
        "expected Unreachable or Timeout, got: {err:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 5 — RTO drill (check-rto-gate, NFR-Ops-9)
// ═══════════════════════════════════════════════════════════════════════
//
// The REAL `check-rto-gate` consumes a weekly-drill evidence ledger
// (`rto-evidence.toml`) and goes RED when the latest drill's restore time
// exceeds the 4 h SLA (14400 s), GREEN when within it.  Both vectors inject
// an evidence ledger into a tempdir and run the live gate binary.

fn rto_evidence_toml(rto_seconds: u64) -> String {
    // P9: use a relative date (today) so the 7-day recency check in
    // check_rto_gate does not expire this fixture.
    let today = chrono::Utc::now().format("%Y-%m-%d");
    format!(
        "[[evidence]]\n\
         drill_date = \"{today}\"\n\
         rto_seconds = {rto_seconds}\n\
         drill_success = true\n",
    )
}

/// RED: a drill that took 5 h (18000 s) breaches the 4 h (14400 s) SLA.
#[test]
fn story_10_4a_ac1_rto_drill_red() {
    let dir = tempfile::tempdir().unwrap();
    write_file(
        dir.path(),
        "rto-evidence.toml",
        &rto_evidence_toml(5 * 3600),
    );

    let out = run_xtask(&[
        "check-rto-gate",
        "--json",
        "--evidence",
        dir.path().join("rto-evidence.toml").to_str().unwrap(),
    ]);

    assert!(
        !out.status.success(),
        "RED: a 5 h drill MUST breach the 4 h RTO SLA"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).expect("gate emits JSON report");
    assert_eq!(report["passed"], false);
    assert_eq!(report["threshold_secs"], 14400);
    assert_eq!(report["latest_drill"]["rto_seconds"], 5 * 3600);
}

/// GREEN: a drill that took 2 h (7200 s) is within the 4 h SLA; the 4 h
/// boundary (14400 s) is the inclusive ≤ edge.
#[test]
fn story_10_4a_ac1_rto_drill_green() {
    let within = tempfile::tempdir().unwrap();
    write_file(
        within.path(),
        "rto-evidence.toml",
        &rto_evidence_toml(2 * 3600),
    );
    let out = run_xtask(&[
        "check-rto-gate",
        "--json",
        "--evidence",
        within.path().join("rto-evidence.toml").to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "GREEN: a 2 h drill is within the SLA — stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Boundary: exactly 4 h (14400 s) is the inclusive ≤ edge → GREEN.
    let boundary = tempfile::tempdir().unwrap();
    write_file(
        boundary.path(),
        "rto-evidence.toml",
        &rto_evidence_toml(4 * 3600),
    );
    let out = run_xtask(&[
        "check-rto-gate",
        "--json",
        "--evidence",
        boundary.path().join("rto-evidence.toml").to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "GREEN: a 4 h drill sits on the ≤ boundary — stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 6 — weekly backup cross-check (NFR-Ops-9, verify_backup_integrity)
// ═══════════════════════════════════════════════════════════════════════
//
// AC1's weekly cadence independently re-derives the Merkle root from BOTH the
// source TL and its backup and byte-compares (`maos_audit::backup::
// verify_backup_integrity`).  This is the AC1 weekly backup oracle — NOT the
// AC2 migration oracle (`MigrationResult::verify`), which has its own vectors.

const TL_SCHEMA_11: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id BLOB NOT NULL PRIMARY KEY,
    timestamp_ns INTEGER NOT NULL,
    spirit_pid INTEGER NOT NULL,
    from_spirit_id TEXT NOT NULL DEFAULT '',
    to_spirit_id TEXT NOT NULL DEFAULT '',
    boot_nonce INTEGER NOT NULL,
    capability_token BLOB,
    kind INTEGER NOT NULL,
    intent TEXT NOT NULL,
    payload_redacted BLOB NOT NULL,
    origin INTEGER NOT NULL
)";

/// Build a SQLite TL at `path` with `n` deterministic rows.  `tamper_idx`,
/// if Some, flips one byte of that row's frame_id (a different frame_id set).
fn build_tl(path: &std::path::Path, n: u8, tamper_idx: Option<u8>) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(TL_SCHEMA_11).unwrap();
    for i in 0..n {
        let mut fid = [i; 16];
        if Some(i) == tamper_idx {
            fid[0] ^= 0xFF;
        }
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, \
             from_spirit_id, to_spirit_id, boot_nonce, capability_token, kind, \
             intent, payload_redacted, origin) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                fid.as_slice(),
                1i64 + i as i64,
                1i64,
                "a",
                "b",
                1i64,
                None::<&[u8]>,
                0i64,
                "intent",
                b"p".as_ref(),
                0i64,
            ],
        )
        .unwrap();
    }
}

/// RED: a backup whose frame_id set was tampered yields a different
/// independently-re-derived Merkle root → `verify_backup_integrity` REDs.
#[test]
fn story_10_4a_ac1_weekly_backup_tamper_red() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.sqlite");
    let backup = dir.path().join("backup.sqlite");
    build_tl(&source, 8, None);
    build_tl(&backup, 8, Some(5)); // tamper row 5's frame_id

    let err = maos_audit::backup::verify_backup_integrity(&source, &backup)
        .expect_err("tampered backup MUST RED");
    assert!(
        matches!(
            err,
            maos_audit::backup::BackupError::MerkleRootMismatch { .. }
        ),
        "expected MerkleRootMismatch, got {err:?}"
    );
}

/// GREEN: a faithful backup (identical frame_id set) passes the cross-check.
#[test]
fn story_10_4a_ac1_weekly_backup_green() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.sqlite");
    let backup = dir.path().join("backup.sqlite");
    build_tl(&source, 8, None);
    build_tl(&backup, 8, None);

    maos_audit::backup::verify_backup_integrity(&source, &backup)
        .expect("faithful backup MUST be GREEN");
}

// ═══════════════════════════════════════════════════════════════════════
// Vector 7 — I1/I2 capability mediation (deny / allow) — P1 + P3
// ═══════════════════════════════════════════════════════════════════════
//
// AC1 mandates a Capability Registry check BEFORE every collective port call
// (I1) and a TL journal before delivery (I2).  The trait path
// (`MemoryManagerPort::write/read/scan`) carries NO token/posture (the ABI is
// stable), so it CANNOT mediate.  P1 makes that path DENY fail-closed when
// `self.capabilities` is wired, directing callers at the cap-gated
// `collective_write/read/scan`.  These vectors drive the REAL
// `CapabilityRegistryAdapter` (verify_and_audit + scope-check) the production
// composition root uses — not a mock — so the deny/allow logic is exercised
// end-to-end.

/// Construct a real `CapabilityRegistryAdapter` with spirit 1 admitted for the
/// three Loom scopes.  Mirrors `make_capability` in the kernel-core halt test.
/// `init_monotonic_base` is idempotent and must run before any token
/// issue/verify (the TTL clock's `debug_assert`).
fn make_capability_for_with_audit(
    spirit_pid: u32,
) -> (
    Arc<CapabilityRegistryAdapter>,
    tokio::sync::mpsc::Receiver<cap_audit::CapAuditEvent>,
) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
    let signing_key = Ed25519SigningKey::new([0u8; 32]);
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        spirit_pid,
        ManifestCapabilityScope {
            scopes: vec![Scope::LoomWrite, Scope::LoomRead, Scope::LoomScan],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    policy.update(inner);
    let (audit_tx, audit_rx) = cap_audit::channel();
    let quota = cap_quota::CapQuotaTracker::new();
    let adapter = Arc::new(CapabilityRegistryAdapter::new(
        crypto,
        signing_key,
        0x10A4,
        policy,
        audit_tx,
        quota,
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    (adapter, audit_rx)
}

fn make_capability_for(spirit_pid: u32) -> Arc<CapabilityRegistryAdapter> {
    make_capability_for_with_audit(spirit_pid).0
}

fn make_capability() -> Arc<CapabilityRegistryAdapter> {
    make_capability_for(1)
}

struct NoopJournal;

impl SpiritSchedulerPort for NoopJournal {
    fn journal_lifecycle(&self, _entry: JournalEntry) {}

    fn last_lifecycle_event(&self, _spirit_id: &str) -> Option<LifecycleEvent> {
        None
    }
}

fn make_manifest_admitted_capability(
    spirit_pid: u32,
) -> (
    Arc<CapabilityRegistryAdapter>,
    tokio::sync::mpsc::Receiver<cap_audit::CapAuditEvent>,
) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let required = CapabilitiesRequired::from_toml_str(
        r#"provider.complete = ["anthropic.default"]
loom.read = true
loom.write = true
loom.scan = true"#,
    )
    .expect("v4 Loom capability declaration parses");
    let security = SecurityManagerAdapter::default();
    let posture = PostureSection::from_toml_str(
        r#"default = "cautious"
allowed_max = "cautious""#,
    )
    .unwrap();
    security
        .admit_spirit(
            spirit_pid,
            "story-13-5d-researcher",
            &SandboxConfig::default(),
            &ResourceCaps::default(),
            &required,
            None,
            &NoopJournal,
            &posture,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&ClassSection {
                name: "researcher".into(),
                version: "0.5.0".into(),
                abi: "1.0".into(),
                manifest_schema_version: 4,
                min_substrate_version: "0.0.1".into(),
                forms: vec!["rust-inproc".into()],
                trust_tier: "local".into(),
                description: "Story 13.5d admission witness".into(),
            }),
        )
        .expect("real manifest admission succeeds");

    let policy = Arc::clone(security.policy());
    let admitted = policy.inner().load_full();
    let scopes = &admitted
        .manifest_scopes
        .get(&spirit_pid)
        .expect("admission inserts manifest scopes")
        .scopes;
    for expected in [Scope::LoomRead, Scope::LoomWrite, Scope::LoomScan] {
        assert!(
            scopes.contains(&expected),
            "admission must preserve declared {expected:?}"
        );
    }
    drop(admitted);

    let (audit_tx, audit_rx) = cap_audit::channel();
    let adapter = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0u8; 32]),
        0x135D,
        policy,
        audit_tx,
        cap_quota::CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    (adapter, audit_rx)
}

/// Build a `MemoryManagerAdapter` with BOTH the collective port AND the
/// capability registry injected (the production composition-root shape).
fn adapter_with_caps(
    port: Arc<dyn CollectiveMemoryPort>,
    caps: Arc<CapabilityRegistryAdapter>,
) -> Arc<MemoryManagerAdapter> {
    let tmp = tempfile::tempdir().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");
    std::mem::forget(tmp);
    let private = Arc::new(PrivateMemoryStore::new(memory_root, 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal_index = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(unique_nonce()));
    Arc::new(
        MemoryManagerAdapter::new(private, shared, principal_index, tl)
            .with_collective_port(Some(port))
            .with_capabilities(Some(caps)),
    )
}

/// RED (P1 fail-closed): when I1/I2 mediation is wired, the unmediated trait
/// path is DENIED — the trait carries no token, so it cannot mediate.  No write
/// reaches the backing store.
#[test]
fn story_10_4a_ac1_i1_unmediated_trait_path_denied_red() {
    let port = Arc::new(RecordingPort::new());
    let adapter = adapter_with_caps(port.clone(), make_capability());
    let err = adapter
        .write(
            1,
            MemoryTier::Collective,
            &MemoryNamespace::Default,
            "pattern",
            MemoryValue::Text("payload".into()),
        )
        .expect_err("unmediated trait path MUST DENY when I1/I2 is wired");
    match err {
        MemoryError::Collective { kind, reason } => {
            assert_eq!(
                kind,
                CollectiveErrorKind::CapabilityDenied,
                "wrong deny kind: {reason}"
            );
        }
        other => panic!("expected Collective::CapabilityDenied, got {other:?}"),
    }
    assert_eq!(port.writes(), 0, "no write may reach the port on a deny");
}

/// RED (I1 scope mediation): a valid LoomRead token is NOT a LoomWrite grant.
/// `verify_and_audit` succeeds (the token is real), but the scope check denies
/// — proving mediation keys on the SCOPE, not just token existence.
#[test]
fn story_10_4a_ac1_i1_wrong_scope_denied_red() {
    let port = Arc::new(RecordingPort::new());
    let caps = make_capability();
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    let token = caps
        .issue_with_mediation(1, Scope::LoomRead, 60, posture, IntentClass::Readonly)
        .expect("LoomRead token issues for an admitted spirit");
    let err = adapter
        .collective_write(
            1,
            &MemoryNamespace::Default,
            "pattern",
            MemoryValue::Text("payload".into()),
            &token,
            posture,
            SandboxTier(0),
        )
        .expect_err("collective write with a LoomRead token MUST be DENIED");
    match err {
        MemoryError::Collective { kind, .. } => {
            assert_eq!(kind, CollectiveErrorKind::CapabilityDenied);
        }
        other => panic!("expected Collective::CapabilityDenied, got {other:?}"),
    }
    assert_eq!(port.writes(), 0, "no write may reach the port on a deny");
}

/// GREEN (I1 allow + P8 TTL ≤ 60s): a valid ≤60s LoomWrite token plus an
/// injected port lets the mediated `collective_write` succeed and the write
/// reaches the backing store.  This is the deny/allow pair's allow branch.
#[test]
fn story_10_4a_ac1_i1_valid_loomwrite_allows_green() {
    let port = Arc::new(RecordingPort::new());
    let caps = make_capability();
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    // TTL 30s, HighPrivilege — under the 60s LoomWrite cap (P8).
    let token = caps
        .issue_with_mediation(1, Scope::LoomWrite, 30, posture, IntentClass::HighPrivilege)
        .expect("LoomWrite token issues for an admitted spirit");
    adapter
        .collective_write(
            1,
            &MemoryNamespace::Default,
            "pattern",
            MemoryValue::Text("payload".into()),
            &token,
            posture,
            SandboxTier(0),
        )
        .expect("a valid ≤60s LoomWrite token MUST allow the mediated write");
    assert_eq!(port.writes(), 1, "the write MUST reach the backing store");
}

/// RED (Story 13.5d P0): a token bound to pid 7 cannot be presented as pid 9.
/// The typed denial must occur before the backing port is reached; presenting
/// the same token as pid 7 remains the matching control.
#[test]
fn story_13_5d_forged_pid_is_denied_before_collective_write() {
    let port = Arc::new(RecordingPort::new());
    let (caps, mut audit_rx) = make_capability_for_with_audit(7);
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    let token = caps
        .issue_with_mediation(7, Scope::LoomWrite, 30, posture, IntentClass::HighPrivilege)
        .expect("LoomWrite token issues for pid 7");

    let err = adapter
        .collective_write(
            9,
            &MemoryNamespace::Default,
            "forged-pid",
            MemoryValue::Text("payload".into()),
            &token,
            posture,
            SandboxTier(0),
        )
        .expect_err("pid 9 MUST NOT spend a token issued for pid 7");
    match err {
        MemoryError::Collective { kind, reason } => {
            assert_eq!(kind, CollectiveErrorKind::CapabilityDenied);
            assert!(
                reason.contains("SpiritIdMismatch"),
                "expected SpiritIdMismatch reason, got {reason}"
            );
        }
        other => panic!("expected Collective::CapabilityDenied, got {other:?}"),
    }
    assert_eq!(port.writes(), 0, "forged pid MUST make zero port writes");
    let audits: Vec<_> = std::iter::from_fn(|| audit_rx.try_recv().ok())
        .filter_map(|event| match event {
            cap_audit::CapAuditEvent::Verify {
                token_id,
                spirit_pid,
                outcome,
            } if token_id == token.token_id => Some((spirit_pid, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        audits,
        vec![
            (7, cap_audit::VerifyOutcome::Ok),
            (7, cap_audit::VerifyOutcome::SpiritIdMismatch),
        ],
        "a valid token is verified before its caller/token pid refusal"
    );

    adapter
        .collective_write(
            7,
            &MemoryNamespace::Default,
            "matching-pid",
            MemoryValue::Text("payload".into()),
            &token,
            posture,
            SandboxTier(0),
        )
        .expect("matching token and caller pid MUST succeed");
    assert_eq!(port.writes(), 1, "matching pid MUST reach the backing port");
}

/// RED (D1): authentication wins over caller-pid attribution. A forged token
/// presented with a mismatched pid must record its verification failure, never
/// a caller/token `SpiritIdMismatch`.
#[test]
fn story_13_5d_forged_token_is_verified_before_pid_mismatch() {
    let port = Arc::new(RecordingPort::new());
    let (caps, mut audit_rx) = make_capability_for_with_audit(7);
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    let token = caps
        .issue_with_mediation(7, Scope::LoomWrite, 30, posture, IntentClass::HighPrivilege)
        .expect("LoomWrite token issues for pid 7");
    let mut forged = token.clone();
    forged.signature[0] ^= 0xFF;

    let err = adapter
        .collective_write(
            9,
            &MemoryNamespace::Default,
            "forged-token",
            MemoryValue::Text("payload".into()),
            &forged,
            posture,
            SandboxTier(0),
        )
        .expect_err("a forged token MUST be rejected before pid attribution");
    match err {
        MemoryError::Collective { kind, reason } => {
            assert_eq!(kind, CollectiveErrorKind::CapabilityDenied);
            assert!(
                reason.contains("signature integrity violation"),
                "expected verification-class signature failure, got {reason}"
            );
            assert!(
                !reason.contains("SpiritIdMismatch"),
                "forged token must not reach caller/token attribution: {reason}"
            );
        }
        other => panic!("expected Collective::CapabilityDenied, got {other:?}"),
    }
    assert_eq!(port.writes(), 0, "forged token MUST make zero port writes");

    let audits: Vec<_> = std::iter::from_fn(|| audit_rx.try_recv().ok())
        .filter_map(|event| match event {
            cap_audit::CapAuditEvent::Verify {
                token_id,
                spirit_pid,
                outcome,
            } if token_id == token.token_id => Some((spirit_pid, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        audits,
        vec![(7, cap_audit::VerifyOutcome::SignatureMismatch)],
        "forged token must emit only its verification failure"
    );
    assert!(
        !audits
            .iter()
            .any(|(_, outcome)| *outcome == cap_audit::VerifyOutcome::SpiritIdMismatch),
        "forged token must record zero SpiritIdMismatch audit events"
    );
}

#[test]
fn story_13_5d_forged_pid_is_denied_before_collective_read() {
    let port = Arc::new(RecordingPort::new());
    let (caps, mut audit_rx) = make_capability_for_with_audit(7);
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    let token = caps
        .issue_with_mediation(7, Scope::LoomRead, 30, posture, IntentClass::Readonly)
        .expect("LoomRead token issues for pid 7");

    let err = adapter
        .collective_read(
            9,
            &MemoryNamespace::Default,
            "forged-pid",
            &token,
            posture,
            SandboxTier(0),
        )
        .expect_err("pid 9 MUST NOT spend a LoomRead token issued for pid 7");
    match err {
        MemoryError::Collective { kind, reason } => {
            assert_eq!(kind, CollectiveErrorKind::CapabilityDenied);
            assert!(reason.contains("SpiritIdMismatch"));
        }
        other => panic!("expected Collective::CapabilityDenied, got {other:?}"),
    }
    assert_eq!(port.reads(), 0, "forged pid MUST make zero port reads");
    let audits: Vec<_> = std::iter::from_fn(|| audit_rx.try_recv().ok())
        .filter_map(|event| match event {
            cap_audit::CapAuditEvent::Verify {
                token_id,
                spirit_pid,
                outcome,
            } if token_id == token.token_id => Some((spirit_pid, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        audits,
        vec![
            (7, cap_audit::VerifyOutcome::Ok),
            (7, cap_audit::VerifyOutcome::SpiritIdMismatch),
        ],
        "read refusal must be audited under the token owner's pid"
    );

    assert_eq!(
        adapter
            .collective_read(
                7,
                &MemoryNamespace::Default,
                "matching-pid",
                &token,
                posture,
                SandboxTier(0),
            )
            .expect("matching token and caller pid MUST succeed"),
        None
    );
    assert_eq!(port.reads(), 1, "matching pid MUST reach the backing port");
}

#[test]
fn story_13_5d_forged_pid_is_denied_before_collective_scan() {
    let port = Arc::new(RecordingPort::new());
    let (caps, mut audit_rx) = make_capability_for_with_audit(7);
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    let token = caps
        .issue_with_mediation(7, Scope::LoomScan, 30, posture, IntentClass::Readonly)
        .expect("LoomScan token issues for pid 7");

    let err = adapter
        .collective_scan(
            9,
            &MemoryNamespace::Default,
            "forged-pid",
            10,
            &token,
            posture,
            SandboxTier(0),
        )
        .expect_err("pid 9 MUST NOT spend a LoomScan token issued for pid 7");
    match err {
        MemoryError::Collective { kind, reason } => {
            assert_eq!(kind, CollectiveErrorKind::CapabilityDenied);
            assert!(reason.contains("SpiritIdMismatch"));
        }
        other => panic!("expected Collective::CapabilityDenied, got {other:?}"),
    }
    assert_eq!(port.scans(), 0, "forged pid MUST make zero port scans");
    let audits: Vec<_> = std::iter::from_fn(|| audit_rx.try_recv().ok())
        .filter_map(|event| match event {
            cap_audit::CapAuditEvent::Verify {
                token_id,
                spirit_pid,
                outcome,
            } if token_id == token.token_id => Some((spirit_pid, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        audits,
        vec![
            (7, cap_audit::VerifyOutcome::Ok),
            (7, cap_audit::VerifyOutcome::SpiritIdMismatch),
        ],
        "scan refusal must be audited under the token owner's pid"
    );

    assert!(adapter
        .collective_scan(
            7,
            &MemoryNamespace::Default,
            "matching-pid",
            10,
            &token,
            posture,
            SandboxTier(0),
        )
        .expect("matching token and caller pid MUST succeed")
        .is_empty());
    assert_eq!(port.scans(), 1, "matching pid MUST reach the backing port");
}

#[test]
fn story_13_5d_loom_scope_reaches_policy_table() {
    let (caps, _audit_rx) = make_manifest_admitted_capability(7);
    caps.issue_with_mediation(
        7,
        Scope::LoomWrite,
        30,
        [0u8; 32],
        IntentClass::HighPrivilege,
    )
    .expect("manifest-declared LoomWrite must issue where the old policy denied");
}

#[test]
fn story_13_5d_request_route_row_audit_correlation() {
    let (caps, mut audit_rx) = make_manifest_admitted_capability(7);
    let port = Arc::new(RecordingPort::new());
    let adapter = adapter_with_caps(port.clone(), caps.clone());
    let posture = [0u8; 32];
    let token = caps
        .issue_with_mediation(7, Scope::LoomWrite, 30, posture, IntentClass::HighPrivilege)
        .expect("manifest-admitted token issues");
    adapter
        .collective_write(
            7,
            &MemoryNamespace::Default,
            "correlated-row",
            MemoryValue::Text("correlated payload".into()),
            &token,
            posture,
            SandboxTier(0),
        )
        .expect("mediated correlated write");

    let rows = port.kv.lock();
    assert_eq!(rows.len(), 1, "exactly one backing row must land");
    assert_eq!(rows[0].0, 7, "store row requester pid");
    assert_eq!(rows[0].2, "correlated-row", "store row identity");
    drop(rows);

    let kernel_frames: Vec<_> = std::iter::from_fn(|| audit_rx.try_recv().ok())
        .filter_map(|event| match event {
            cap_audit::CapAuditEvent::Verify {
                token_id,
                spirit_pid,
                outcome,
            } if token_id == token.token_id => Some((spirit_pid, outcome)),
            _ => None,
        })
        .collect();
    assert_eq!(
        kernel_frames,
        vec![(7, cap_audit::VerifyOutcome::Ok)],
        "the kernel's verify_and_audit frame must carry the store row token id"
    );

    let source = include_str!("../../crates/maos-bin/src/main.rs");
    let write_body = live_researcher_collective_method_body(source, "collective_write");
    let audit_call = write_body
        .find("record_invocation(")
        .expect("collective_write must record its invocation");
    let memory_call = write_body
        .find(".collective_write(")
        .expect("collective_write must call the kernel memory route");
    assert!(
        audit_call < memory_call,
        "composition root must audit collective_write before the store route"
    );
    for method in ["collective_read", "collective_scan"] {
        assert!(
            live_researcher_collective_method_body(source, method).contains("record_invocation("),
            "{method} must record its invocation"
        );
    }
}
