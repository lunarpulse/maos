//! Story 16-4 — MAOS footprint, purge, retention, refusal, and key-source contracts.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use maos_bin::purge::{maos_roots, RootKind, WrittenBy};

const CHILD_ENV: &str = "MAOS_16_4_ROOT_CHILD";

#[test]
fn root_enumeration_honors_every_production_resolver_and_ownership_boundary() {
    let temp = tempfile::tempdir().expect("create root-enumeration fixture");
    let root = temp.path();
    let status = Command::new(std::env::current_exe().expect("resolve test executable"))
        .arg("--exact")
        .arg("root_enumeration_child")
        .arg("--nocapture")
        .env_clear()
        .env(CHILD_ENV, "1")
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_AUDIT_DB", root.join("stores/audit.sqlite"))
        .env("MAOS_JOURNAL_PATH", root.join("stores/journal.ndjson"))
        .env("MAOS_MEMORY_ROOT", root.join("stores/memory"))
        .env("MAOS_ARCHIVE_DIR", root.join("stores/archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", root.join("stores/proofs"))
        .env("MAOS_CRL_PATH", root.join("stores/crl"))
        .env(
            "MAOS_REGISTRY_YANK_CURSOR_PATH",
            root.join("stores/yank-cursor.json"),
        )
        .status()
        .expect("run isolated root-enumeration child");
    assert!(status.success(), "isolated root-enumeration child failed");
}

#[test]
fn root_enumeration_child() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return;
    }
    let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
    let maos_home = home.join(".maos");
    let config =
        PathBuf::from(std::env::var_os("XDG_CONFIG_HOME").expect("XDG_CONFIG_HOME")).join("maos");
    let roots = maos_roots();

    assert_root(&roots, &maos_home, RootKind::Directory, WrittenBy::Maos);
    for relative in [
        "stores/audit.sqlite",
        "stores/journal.ndjson",
        "stores/memory",
        "stores/archives",
        "stores/proofs",
        "stores/crl",
        "stores/yank-cursor.json",
    ] {
        assert!(
            roots
                .iter()
                .any(|entry| entry.path == home.parent().unwrap().join(relative)
                    && entry.written_by == WrittenBy::Maos),
            "missing MAOS-owned resolver output {relative}: {roots:#?}"
        );
    }
    assert_root(
        &roots,
        &config.join("audit-signing.key"),
        RootKind::File,
        WrittenBy::Maos,
    );
    assert_root(
        &roots,
        &config.join("spirit-signing.key"),
        RootKind::File,
        WrittenBy::Operator,
    );
    assert_root(
        &roots,
        &config.join("operator.toml"),
        RootKind::File,
        WrittenBy::Operator,
    );
    assert_root(
        &roots,
        &home.join(".local/share/maos"),
        RootKind::Directory,
        WrittenBy::Maos,
    );
}

fn assert_root(
    roots: &[maos_bin::purge::MaosRoot],
    path: &Path,
    kind: RootKind,
    written_by: WrittenBy,
) {
    assert!(
        roots.iter().any(|entry| {
            entry.path == path && entry.kind == kind && entry.written_by == written_by
        }),
        "missing {written_by:?} {kind:?} root {}: {roots:#?}",
        path.display()
    );
}

fn maos_command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .env_clear()
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("MAOS_HOME", root.join("maos-home"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_MEMORY_ROOT", root.join("memory"))
        .env("MAOS_ARCHIVE_DIR", root.join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", root.join("proofs"))
        .env("MAOS_CRL_PATH", root.join("crl"))
        .env("MAOS_REGISTRY_YANK_CURSOR_PATH", root.join("yank.json"))
        .env("MAOS_NOTIFY_DISABLE", "1");
    command
}

fn initialize_and_boot(root: &Path) {
    let init = maos_command(root)
        .args(["init", "--plain"])
        .output()
        .expect("run maos init");
    assert!(
        init.status.success(),
        "init failed: stdout={} stderr={}",
        String::from_utf8_lossy(&init.stdout),
        String::from_utf8_lossy(&init.stderr)
    );

    use std::io::Write;
    use std::process::Stdio;
    let mut child = maos_command(root)
        .args(["shell", "--plain"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start shell root");
    child
        .stdin
        .as_mut()
        .expect("shell stdin")
        .write_all(b"/quit\n")
        .expect("quit shell");
    let output = child.wait_with_output().expect("wait for shell root");
    assert!(
        output.status.success(),
        "shell failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn purge_dry_run_is_inert_then_removes_only_maos_owned_state() {
    let temp = tempfile::tempdir().expect("create purge fixture");
    let root = temp.path();
    initialize_and_boot(root);
    let run = maos_command(root)
        .args(["run", "spirits/researcher/manifest.toml", "--once"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_OLLAMA_URL", "skip")
        .output()
        .expect("run a production Spirit turn");
    assert!(
        run.status.success(),
        "production run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::write(root.join("memory/private-state"), b"private-memory")
        .expect("seed private memory fixture");

    let config = root.join("xdg-config/maos");
    std::fs::create_dir_all(&config).expect("create config fixture");
    std::fs::write(config.join("audit-signing.key"), b"maos-owned").expect("write audit key");
    std::fs::write(config.join("spirit-signing.key"), b"operator-owned")
        .expect("write Spirit signing key");
    std::fs::write(config.join("operator.toml"), b"[operator]\n").expect("write operator config");
    std::fs::create_dir_all(root.join("crl")).expect("create CRL tree");
    let outside = root.join("operator-outside");
    std::fs::create_dir_all(&outside).expect("create operator-owned outside tree");
    std::fs::write(outside.join("survive.txt"), b"survive").expect("write outside sentinel");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, root.join("maos-home/linked-outside"))
        .expect("link MAOS state to operator-owned outside tree");
    std::fs::write(root.join("crl/list.json"), b"{}").expect("write CRL");
    for tree in [
        root.join("maos-home"),
        root.join("memory"),
        root.join("xdg-config/maos"),
        root.join("crl"),
    ] {
        assert!(
            std::fs::read_dir(&tree)
                .unwrap_or_else(|error| panic!("read nonempty tree {}: {error}", tree.display()))
                .next()
                .is_some(),
            "purge control requires nonempty tree {}",
            tree.display()
        );
    }
    // AC1 — one root is pre-deleted after the boot so the destructive leg can
    // demand an explicit `absent:` stdout line for it. Tolerant of the boot
    // not having materialized the empty archive tree at all.
    match std::fs::symlink_metadata(root.join("archives")) {
        Ok(metadata) if metadata.is_dir() => {
            std::fs::remove_dir_all(root.join("archives")).expect("pre-delete the archive root");
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("inspect archive root before purge: {error}"),
    }

    // Non-unix vacuity: the filesystem-identity assertions only run where the
    // real identity walker compiles; the stub must never make them vacuous.
    #[cfg(unix)]
    let before = filesystem_identity(root);
    let dry_run = maos_command(root)
        .arg("purge")
        .output()
        .expect("run purge dry run");
    assert!(
        dry_run.status.success(),
        "dry run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&dry_run.stdout),
        String::from_utf8_lossy(&dry_run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&dry_run.stdout).contains("dry run"),
        "dry run must disclose its mode"
    );
    let dry_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(dry_stdout.contains("would remove credential: Anthropic API credential"));
    assert!(dry_stdout.contains("would remove credential: OpenAI API credential"));
    #[cfg(unix)]
    assert_eq!(
        filesystem_identity(root),
        before,
        "dry run must restore every path the lock acquisition materializes"
    );
    // Obligation (ab) — the receipt is a disposition record, never a secret
    // carrier. Read the operator door's bearer token out of the booted home
    // before the purge erases it; the written receipt must carry neither it
    // nor any `sk-` credential material.
    #[cfg(unix)]
    let bearer_token: String = {
        let control: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("maos-home/control.json"))
                .expect("read the booted control.json"),
        )
        .expect("decode control.json");
        control["token"]
            .as_str()
            .expect("control.json token")
            .to_owned()
    };
    let receipt = root.join("purge-receipt.json");
    let purge = maos_command(root)
        .args(["purge", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run destructive purge");
    assert!(
        purge.status.success(),
        "purge failed: stdout={} stderr={}",
        String::from_utf8_lossy(&purge.stdout),
        String::from_utf8_lossy(&purge.stderr)
    );
    // AC1 — every MAOS root the boot actually produced must be disclosed by
    // name on stdout, the pre-deleted archive root must be disclosed
    // `absent:`, and the operator-authored leftovers under the surviving
    // config tree must be named by `left behind:`.
    let purge_stdout = String::from_utf8_lossy(&purge.stdout);
    for line in [
        "maos purge: removed: MAOS home (maos-home)",
        "maos purge: removed: Transparency Log (transparency.sqlite)",
        "maos purge: removed: Lifecycle Journal (lifecycle.ndjson)",
        "maos purge: removed: private memory (memory)",
        "maos purge: removed: erasure proofs (proofs)",
        "maos purge: removed: certificate revocation lists (crl)",
        "maos purge: removed: audit signing key (audit-signing.key)",
        "maos purge: absent: Spirit archives (archives)",
        "maos purge: left (operator-authored): Spirit publisher signing key (spirit-signing.key)",
        "maos purge: left (operator-authored): operator configuration (operator.toml)",
        "maos purge: left behind: spirit-signing.key",
        "maos purge: left behind: operator.toml",
    ] {
        assert!(
            purge_stdout.contains(line),
            "purge stdout must disclose {line}: {purge_stdout}"
        );
    }
    assert!(
        !root.join("maos-home").exists(),
        "MAOS home must be removed"
    );
    assert!(!root.join("memory").exists(), "memory root must be removed");
    assert!(
        !root.join("archives").exists(),
        "archive root must be removed"
    );
    assert!(!root.join("proofs").exists(), "proof root must be removed");
    assert!(!root.join("crl").exists(), "CRL root must be removed");
    assert!(
        !config.join("audit-signing.key").exists(),
        "MAOS-written audit key must be removed"
    );
    assert_eq!(
        std::fs::read(config.join("spirit-signing.key")).expect("Spirit key survives"),
        b"operator-owned"
    );
    assert!(
        config.join("operator.toml").exists(),
        "operator configuration must survive"
    );
    assert_eq!(
        std::fs::read(outside.join("survive.txt")).expect("read outside sentinel"),
        b"survive",
        "purge must unlink symlinks without traversing their targets"
    );

    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read independent purge receipt"))
            .expect("decode purge receipt");
    assert_eq!(record["status"], "complete");
    assert!(record["signature"].is_null());
    assert!(record["proof"].is_null());
    assert_eq!(
        record["root_count"].as_u64(),
        record["roots"].as_array().map(|roots| roots.len() as u64)
    );
    let credential_roots: Vec<_> = record["roots"]
        .as_array()
        .expect("receipt roots")
        .iter()
        .filter(|root| {
            root["path"]
                .as_str()
                .is_some_and(|path| path.starts_with("keyring://maos/"))
        })
        .collect();
    assert_eq!(
        credential_roots.len(),
        2,
        "receipt must cover both known credentials"
    );
    assert!(credential_roots.iter().all(|root| {
        matches!(
            root["disposition"].as_str(),
            Some("removed" | "absent" | "backend-unavailable")
        )
    }));
    // The receipt must match the on-disk truth, not merely the plan.
    for entry in record["roots"].as_array().expect("receipt roots") {
        let Some(path) = entry["path"].as_str() else {
            continue;
        };
        let path = Path::new(path);
        match (entry["written_by"].as_str(), entry["disposition"].as_str()) {
            (Some("maos"), Some("removed")) => assert!(
                path.symlink_metadata().is_err(),
                "receipt claimed removed but {path:?} still exists"
            ),
            (Some("maos"), Some("absent")) => assert!(
                path.symlink_metadata().is_err(),
                "receipt claimed absent but {path:?} exists"
            ),
            (_, Some("left (operator-authored)")) => assert!(
                path.symlink_metadata().is_ok(),
                "receipt claimed left but {path:?} is gone"
            ),
            _ => {}
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&receipt)
                .expect("receipt metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let receipt_bytes = std::fs::read(&receipt).expect("read receipt bytes");
        assert!(
            !receipt_bytes
                .windows(bearer_token.len())
                .any(|window| window == bearer_token.as_bytes()),
            "the receipt must never carry the control.json bearer token"
        );
        assert!(
            !receipt_bytes.windows(3).any(|window| window == b"sk-"),
            "the receipt must never carry sk- credential material"
        );
    }

    // AC1 — re-resolve the footprint through the production resolvers in a
    // child with the SAME command environment: every MAOS-owned root the
    // resolvers still name must be gone from disk.
    let resolver_child = Command::new(std::env::current_exe().expect("resolve test executable"))
        .arg("--exact")
        .arg("assert_no_maos_roots_child")
        .arg("--nocapture")
        .env_clear()
        .current_dir(root)
        .env(NO_ROOTS_CHILD_ENV, "1")
        .env("HOME", root.join("home"))
        .env("MAOS_HOME", root.join("maos-home"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_MEMORY_ROOT", root.join("memory"))
        .env("MAOS_ARCHIVE_DIR", root.join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", root.join("proofs"))
        .env("MAOS_CRL_PATH", root.join("crl"))
        .env("MAOS_REGISTRY_YANK_CURSOR_PATH", root.join("yank.json"))
        .status()
        .expect("run isolated post-purge resolver check");
    assert!(
        resolver_child.success(),
        "the destructive purge left MAOS-owned state the production resolvers still name"
    );

    let reinit = maos_command(root)
        .args(["init", "--plain"])
        .output()
        .expect("re-initialize after purge");
    assert!(
        reinit.status.success(),
        "purge must leave a cleanly re-initializable home: {}",
        String::from_utf8_lossy(&reinit.stderr)
    );
}

/// Item 12 — the CLI's ndjson output is not merely "some parseable lines":
/// every line must BE one audit entry carrying the entry's key fields.
fn assert_audit_ndjson_entries(stdout: &str, expected_rows: usize, phase: &str) {
    let mut rows = 0;
    for line in stdout.lines().filter(|line| !line.is_empty()) {
        let entry: serde_json::Value =
            serde_json::from_str(line).expect("every ndjson line is one JSON value");
        let object = entry.as_object().expect("audit entry is a JSON object");
        for field in [
            "frame_id",
            "timestamp_ns",
            "spirit_pid",
            "boot_nonce",
            "kind",
            "intent",
        ] {
            assert!(
                object.contains_key(field),
                "{phase}: audit entry missing {field}: {line}"
            );
        }
        assert!(
            object["frame_id"].as_str().is_some_and(|id| {
                id.len() == 32 && id.chars().all(|character| character.is_ascii_hexdigit())
            }),
            "{phase}: frame_id must be 32 hex chars: {line}"
        );
        assert!(
            object["timestamp_ns"].is_u64()
                && object["spirit_pid"].is_u64()
                && object["boot_nonce"].is_u64(),
            "{phase}: frame counters must be JSON numbers: {line}"
        );
        assert!(
            object["kind"].as_str().is_some_and(|kind| !kind.is_empty()),
            "{phase}: kind must be a non-empty string: {line}"
        );
        assert!(
            object["intent"].is_string(),
            "{phase}: intent must be a string: {line}"
        );
        rows += 1;
    }
    assert_eq!(rows, expected_rows, "{phase}: audit row count mismatch");
}

#[test]
fn purge_keep_log_discloses_and_preserves_a_queryable_checkpointed_log() {
    let temp = tempfile::tempdir().expect("create keep-log fixture");
    let root = temp.path();
    let init = maos_command(root)
        .args(["init", "--plain"])
        .output()
        .expect("initialize one-shot fixture");
    assert!(init.status.success());
    let one_shot = maos_command(root)
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_OLLAMA_URL", "skip")
        .output()
        .expect("run WAL-heavy one-shot");
    assert!(
        one_shot.status.success(),
        "one-shot failed: {}",
        String::from_utf8_lossy(&one_shot.stderr)
    );

    let audit = root.join("maos-home/audit/transparency.sqlite");
    let entries = maos_audit::query(&audit, maos_audit::AuditFilter::default())
        .expect("query populated audit fixture");
    let before_rows = entries.len();
    let capability_tokens = entries
        .iter()
        .filter(|entry| entry.capability_token_hex.is_some())
        .count();
    let shell_turns = entries
        .iter()
        .filter(|entry| entry.intent == "shell.turn")
        .count();
    assert!(before_rows > 0, "keep-log control requires a non-empty log");
    let before_cli = maos_command(root)
        .args(["audit", "query", "--format", "ndjson"])
        .output()
        .expect("query audit log before purge");
    assert!(before_cli.status.success());
    assert_audit_ndjson_entries(
        &String::from_utf8_lossy(&before_cli.stdout),
        before_rows,
        "before the keep-log purge",
    );

    let dry_run = maos_command(root)
        .args(["purge", "--keep-log"])
        .output()
        .expect("run keep-log dry run");
    assert!(dry_run.status.success());
    let stdout = String::from_utf8_lossy(&dry_run.stdout);
    for disclosure in [
        format!("audit_rows={before_rows}"),
        format!("capability_token_rows={capability_tokens}"),
        format!("shell_turn_rows={shell_turns}"),
    ] {
        assert!(
            stdout.contains(&disclosure),
            "dry run must disclose exact queried counts: {stdout}"
        );
    }

    let receipt = root.join("keep-log-receipt.json");
    let purge = maos_command(root)
        .args(["purge", "--keep-log", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run keep-log purge");
    assert!(
        purge.status.success(),
        "keep-log purge failed: stdout={} stderr={}",
        String::from_utf8_lossy(&purge.stdout),
        String::from_utf8_lossy(&purge.stderr)
    );
    assert!(
        audit.exists(),
        "the Transparency Log must survive --keep-log"
    );
    assert!(!PathBuf::from(format!("{}-wal", audit.display())).exists());
    assert!(!PathBuf::from(format!("{}-shm", audit.display())).exists());
    assert_eq!(
        maos_audit::query_with_redaction(&audit, maos_audit::AuditFilter::default())
            .expect("ADR-028 export/replay reader accepts checkpointed log")
            .len(),
        before_rows
    );
    let after_cli = maos_command(root)
        .args(["audit", "query", "--format", "ndjson"])
        .output()
        .expect("query audit log after purge");
    assert!(after_cli.status.success());
    assert_audit_ndjson_entries(
        &String::from_utf8_lossy(&after_cli.stdout),
        before_rows,
        "after the keep-log purge",
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt).expect("read keep-log receipt"))
            .expect("decode keep-log receipt");
    let roots = record["roots"].as_array().expect("receipt roots");
    assert!(roots.iter().any(|entry| {
        entry["path"] == audit.display().to_string() && entry["disposition"] == "kept"
    }));
    for entry in roots {
        if entry["written_by"] == "maos" && entry["disposition"] == "removed" {
            let path = entry["path"].as_str().expect("receipt path");
            assert!(
                !Path::new(path).exists(),
                "--keep-log left unrelated MAOS state at {path}"
            );
        }
    }
}

#[test]
fn purge_keep_log_retains_distinct_tenant_audit_and_host_memory_databases() {
    let temp = tempfile::tempdir().expect("create tenant keep-log fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let host_memory = root.join("maos-home/audit/transparency.sqlite");
    let tenant_audit = root.join("maos-home/audit/teams/team-a/transparency.sqlite");
    let sibling_audit = root.join("maos-home/audit/teams/team-b/transparency.sqlite");
    std::fs::create_dir_all(tenant_audit.parent().expect("tenant audit parent"))
        .expect("create tenant audit parent");
    std::fs::create_dir_all(sibling_audit.parent().expect("sibling audit parent"))
        .expect("create sibling audit parent");
    std::fs::copy(&host_memory, &tenant_audit).expect("seed tenant audit schema");
    std::fs::copy(&host_memory, &sibling_audit).expect("seed sibling audit schema");
    for (path, marker) in [
        (&host_memory, "host-memory"),
        (&tenant_audit, "team-audit"),
        (&sibling_audit, "team-b-sibling"),
    ] {
        let connection = rusqlite::Connection::open(path).expect("open retained database");
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;\
                 CREATE TABLE IF NOT EXISTS purge_retention(marker TEXT NOT NULL);\
                 DELETE FROM purge_retention;",
            )
            .expect("create retention marker table");
        connection
            .execute("INSERT INTO purge_retention(marker) VALUES (?1)", [marker])
            .expect("insert retention marker");
    }

    // The booted root wrote production rows into the host log, and the tenant
    // and sibling copies carry the same rows. Read them back through the
    // production audit reader so survival is proven against real rows the
    // CODE produced, not just the test's marker table.
    let production_rows = |path: &Path| {
        maos_audit::query(path, maos_audit::AuditFilter::default()).expect("query production rows")
    };
    let host_entries = production_rows(&host_memory);
    assert!(
        !host_entries.is_empty(),
        "the booted root must have written production rows"
    );
    for path in [&tenant_audit, &sibling_audit] {
        assert_eq!(
            production_rows(path).len(),
            host_entries.len(),
            "seeded copy {} must carry the production rows",
            path.display()
        );
    }
    let retained_rows = [&host_memory, &tenant_audit]
        .into_iter()
        .map(|path| production_rows(path).len())
        .sum::<usize>();
    let sibling_entries = production_rows(&sibling_audit);
    let sibling_caps = sibling_entries
        .iter()
        .filter(|entry| entry.capability_token_hex.is_some())
        .count();
    let sibling_turns = sibling_entries
        .iter()
        .filter(|entry| entry.intent == "shell.turn")
        .count();

    let dry_run = maos_command(root)
        .env("MAOS_LOOM_POSTGRES", "postgresql://unused-by-purge")
        .env("MAOS_LOOM_HOME_TEAM", "team-a")
        .args(["purge", "--keep-log"])
        .output()
        .expect("run tenant keep-log dry run");
    assert!(dry_run.status.success());
    let dry_stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(
        dry_stdout.contains(&format!("audit_rows={retained_rows}")),
        "dry run must count both retained databases: {dry_stdout}"
    );
    // The sibling disclosure names the team and its exact queried counts.
    assert!(
        dry_stdout.contains(&format!(
            "--keep-log retains sibling team log team-b: \
             audit_rows={} capability_token_rows={sibling_caps} shell_turn_rows={sibling_turns}",
            sibling_entries.len()
        )),
        "dry run must disclose the sibling team log: {dry_stdout}"
    );

    let receipt = root.join("tenant-keep-log-receipt.json");
    let purge = maos_command(root)
        .env("MAOS_LOOM_POSTGRES", "postgresql://unused-by-purge")
        .env("MAOS_LOOM_HOME_TEAM", "team-a")
        .args(["purge", "--keep-log", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run tenant keep-log purge");
    assert!(
        purge.status.success(),
        "tenant keep-log purge failed: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    assert!(
        String::from_utf8_lossy(&purge.stdout)
            .contains("maos purge: kept: sibling team log (transparency.sqlite)"),
        "the confirmed run must disclose the retained sibling log"
    );
    for (path, marker) in [
        (&host_memory, "host-memory"),
        (&tenant_audit, "team-audit"),
        (&sibling_audit, "team-b-sibling"),
    ] {
        assert!(!PathBuf::from(format!("{}-wal", path.display())).exists());
        assert!(!PathBuf::from(format!("{}-shm", path.display())).exists());
        let connection = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .expect("reopen retained database");
        assert_eq!(
            connection
                .query_row("SELECT marker FROM purge_retention", [], |row| {
                    row.get::<_, String>(0)
                })
                .expect("read retained marker"),
            marker
        );
        // The production rows survive the checkpoint alongside the marker.
        assert_eq!(
            production_rows(path).len(),
            host_entries.len(),
            "{} must keep every production row",
            path.display()
        );
    }
    // The receipt must list the resolved team log, the default host DB and
    // the sibling log — all `kept`.
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read tenant keep-log receipt"))
            .expect("decode tenant keep-log receipt");
    let roots = record["roots"].as_array().expect("receipt roots");
    for kept in [
        tenant_audit.display().to_string(),
        host_memory.display().to_string(),
        sibling_audit.display().to_string(),
    ] {
        assert!(
            roots
                .iter()
                .any(|entry| entry["path"] == kept && entry["disposition"] == "kept"),
            "receipt must carry {kept} as kept: {roots:#?}"
        );
    }
}

/// Item 1(a) — with MAOS_AUDIT_DB set and NO MAOS_HOME to shadow it, the
/// custom database IS the resolved Transparency Log: `--keep-log` must
/// checkpoint it in place and retain every production row, while the explicit
/// memory override is still removed exactly (never widened to its parent).
#[test]
fn override_audit_db_without_maos_home_is_the_resolved_log_keep_log_retains_it() {
    let temp = tempfile::tempdir().expect("create override-scope fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let booted_log = root.join("maos-home/audit/transparency.sqlite");
    let production_rows = maos_audit::query(&booted_log, maos_audit::AuditFilter::default())
        .expect("query the booted Transparency Log");
    assert!(
        !production_rows.is_empty(),
        "the booted root must have written production rows"
    );

    let override_parent = root.join("operator-owned/maos");
    let memory = override_parent.join("runtime-memory");
    std::fs::create_dir_all(&memory).expect("create explicit memory override");
    std::fs::write(memory.join("state.bin"), b"maos-state").expect("write MAOS memory state");
    let sibling = override_parent.join("operator-sibling.txt");
    std::fs::write(&sibling, b"survive").expect("write operator sibling");

    let audit = root.join("custom-audit/custom-name.db");
    std::fs::create_dir_all(audit.parent().expect("custom audit parent"))
        .expect("create custom audit parent");
    std::fs::copy(&booted_log, &audit).expect("seed the custom database from the booted log");
    let connection = rusqlite::Connection::open(&audit).expect("open custom audit database");
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;\
             CREATE TABLE retained(marker TEXT NOT NULL);\
             INSERT INTO retained(marker) VALUES ('custom-audit');",
        )
        .expect("seed custom audit database");
    drop(connection);

    // Built by hand: unlike `maos_command`, this environment deliberately has
    // NO MAOS_HOME, so MAOS_AUDIT_DB is the resolved log, not a shadowed leg.
    let receipt = root.join("override-receipt.json");
    let purge = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env_clear()
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_AUDIT_DB", &audit)
        .env("MAOS_MEMORY_ROOT", &memory)
        .env("MAOS_CRL_PATH", root.join("crl"))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .args(["purge", "--keep-log", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run override-scope purge");
    assert!(
        purge.status.success(),
        "override-scope purge failed: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    // The custom name must appear as the RETAINED Transparency Log itself —
    // enumerated, checkpointed, kept — not merely tolerated as a bystander.
    assert!(
        String::from_utf8_lossy(&purge.stdout)
            .contains("maos purge: kept: Transparency Log (custom-name.db)"),
        "the resolved override log must be disclosed as kept: {}",
        String::from_utf8_lossy(&purge.stdout)
    );
    assert!(!memory.exists(), "the exact MAOS override must be removed");
    assert_eq!(
        std::fs::read(&sibling).expect("read surviving operator sibling"),
        b"survive",
        "an override containing a `maos` component must not widen deletion"
    );
    assert!(!PathBuf::from(format!("{}-wal", audit.display())).exists());
    assert!(!PathBuf::from(format!("{}-shm", audit.display())).exists());
    assert_eq!(
        maos_audit::query(&audit, maos_audit::AuditFilter::default())
            .expect("the checkpointed custom database stays queryable")
            .len(),
        production_rows.len(),
        "every production row must survive the checkpoint"
    );
    let retained = rusqlite::Connection::open_with_flags(
        &audit,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("reopen custom audit database");
    assert_eq!(
        retained
            .query_row("SELECT marker FROM retained", [], |row| row
                .get::<_, String>(0))
            .expect("read retained custom audit row"),
        "custom-audit"
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read override receipt"))
            .expect("decode override receipt");
    let roots = record["roots"].as_array().expect("receipt roots");
    assert!(
        roots
            .iter()
            .any(|entry| entry["path"] == audit.display().to_string()
                && entry["disposition"] == "kept"),
        "receipt must carry the override log as kept: {roots:#?}"
    );
    for entry in roots {
        if entry["written_by"] == "maos" && entry["disposition"] == "removed" {
            let path = Path::new(entry["path"].as_str().expect("receipt path"));
            assert!(
                path.symlink_metadata().is_err(),
                "--keep-log left unrelated MAOS state at {path:?}"
            );
        }
    }
}

/// Item 1(b) — with BOTH MAOS_HOME and MAOS_AUDIT_DB set, the override is
/// shadowed as the resolved log but still ENUMERATED (`Transparency Log
/// override`) together with its MAOS-written sidecars, and a confirmed purge
/// without --keep-log removes it.
#[test]
fn override_audit_db_under_maos_home_is_enumerated_and_removed() {
    let temp = tempfile::tempdir().expect("create override-enumeration fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let override_parent = root.join("operator-owned/maos");
    let memory = override_parent.join("runtime-memory");
    std::fs::create_dir_all(&memory).expect("create explicit memory override");
    std::fs::write(memory.join("state.bin"), b"maos-state").expect("write MAOS memory state");
    let sibling = override_parent.join("operator-sibling.txt");
    std::fs::write(&sibling, b"survive").expect("write operator sibling");

    let audit = root.join("custom-audit/custom-name.db");
    std::fs::create_dir_all(audit.parent().expect("custom audit parent"))
        .expect("create custom audit parent");
    let connection = rusqlite::Connection::open(&audit).expect("open custom audit database");
    connection
        .execute_batch("CREATE TABLE retained(marker TEXT NOT NULL);")
        .expect("seed custom audit database");
    drop(connection);
    // MAOS-written siblings of a relocated audit database
    // (purge.rs `audit_sidecar_roots`).
    std::fs::write(audit.with_extension("db.team"), b"team-binding").expect("write team binding");
    std::fs::write(audit.parent().unwrap().join("export-seq.json"), b"0")
        .expect("write export cursor");
    std::fs::write(
        audit.parent().unwrap().join("transparency.airgap-stub.log"),
        b"stub",
    )
    .expect("write air-gap stub");
    std::fs::write(
        audit
            .parent()
            .unwrap()
            .join("custom-name.siem-snapshot-42.sqlite"),
        b"leak",
    )
    .expect("write leaked SIEM snapshot");

    let dry_run = maos_command(root)
        .env("MAOS_MEMORY_ROOT", &memory)
        .env("MAOS_AUDIT_DB", &audit)
        .args(["purge"])
        .output()
        .expect("run override dry run");
    assert!(dry_run.status.success());
    let dry_stdout = String::from_utf8_lossy(&dry_run.stdout);
    for disclosure in [
        "maos purge: would remove: Transparency Log override (custom-name.db)",
        "audit team binding (custom-name.db.team)",
        "audit export residue (export-seq.json)",
        "audit export residue (transparency.airgap-stub.log)",
        "leaked audit SIEM snapshot (custom-name.siem-snapshot-42.sqlite)",
    ] {
        assert!(
            dry_stdout.contains(disclosure),
            "dry run must enumerate the shadowed override and its sidecars: {dry_stdout}"
        );
    }

    let receipt = root.join("override-enumeration-receipt.json");
    let purge = maos_command(root)
        .env("MAOS_MEMORY_ROOT", &memory)
        .env("MAOS_AUDIT_DB", &audit)
        .args(["purge", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run override purge");
    assert!(
        purge.status.success(),
        "override purge failed: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    assert!(
        audit.symlink_metadata().is_err(),
        "the shadowed override database must be removed by a confirmed purge"
    );
    for sidecar in [
        audit.with_extension("db.team"),
        audit.parent().unwrap().join("export-seq.json"),
        audit.parent().unwrap().join("transparency.airgap-stub.log"),
        audit
            .parent()
            .unwrap()
            .join("custom-name.siem-snapshot-42.sqlite"),
    ] {
        assert!(
            sidecar.symlink_metadata().is_err(),
            "sidecar must be removed: {}",
            sidecar.display()
        );
    }
    assert!(!memory.exists(), "the exact MAOS override must be removed");
    assert_eq!(
        std::fs::read(&sibling).expect("read surviving operator sibling"),
        b"survive",
        "an override containing a `maos` component must not widen deletion"
    );
    let purge_stdout = String::from_utf8_lossy(&purge.stdout);
    for line in [
        "maos purge: removed: Transparency Log override (custom-name.db)",
        "maos purge: removed: leaked audit SIEM snapshot (custom-name.siem-snapshot-42.sqlite)",
    ] {
        assert!(
            purge_stdout.contains(line),
            "stdout must disclose {line}: {purge_stdout}"
        );
    }
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read receipt"))
            .expect("decode receipt");
    assert_eq!(record["status"], "complete");
    let roots = record["roots"].as_array().expect("receipt roots");
    for removed in [
        audit.display().to_string(),
        audit.with_extension("db.team").display().to_string(),
        audit
            .parent()
            .unwrap()
            .join("export-seq.json")
            .display()
            .to_string(),
        audit
            .parent()
            .unwrap()
            .join("transparency.airgap-stub.log")
            .display()
            .to_string(),
        audit
            .parent()
            .unwrap()
            .join("custom-name.siem-snapshot-42.sqlite")
            .display()
            .to_string(),
    ] {
        assert!(
            roots
                .iter()
                .any(|entry| entry["path"] == removed && entry["disposition"] == "removed"),
            "receipt must carry {removed} as removed: {roots:#?}"
        );
    }
}

#[test]
fn purge_rejects_receipt_without_a_path_before_interpreting_later_options() {
    let temp = tempfile::tempdir().expect("create parser fixture");
    let root = temp.path();
    initialize_and_boot(root);
    // Non-unix vacuity: the identity assertions only run where the real
    // walker compiles; the stub must never make them vacuous.
    #[cfg(unix)]
    let before = filesystem_identity(root);
    let rejected = maos_command(root)
        .args(["purge", "--yes", "--receipt", "--keep-log"])
        .output()
        .expect("run malformed receipt invocation");
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr).contains("--receipt requires a non-option path")
    );
    #[cfg(unix)]
    assert_eq!(filesystem_identity(root), before);
    assert!(!root.join("--keep-log").exists());
}

#[test]
fn purge_refuses_a_top_level_maos_home_before_mutation() {
    let temp = tempfile::tempdir().expect("create unsafe-root fixture");
    let root = temp.path();
    let sentinel = root.join("sentinel");
    std::fs::write(&sentinel, b"survive").expect("write unsafe-root sentinel");
    let receipt = root.join("must-not-exist.json");
    let rejected = maos_command(root)
        .env("MAOS_HOME", "/")
        .args(["purge", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run unsafe-root purge");
    assert_eq!(rejected.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("refusing unsafe purge root /"));
    assert_eq!(std::fs::read(&sentinel).expect("read sentinel"), b"survive");
    assert!(!receipt.exists());
}

/// A relative MAOS_HOME is a typed 78 BEFORE any snapshot, lock, or deletion —
/// the resolvers' failure must never silently drop the home root from the
/// footprint.
#[test]
fn purge_refuses_a_relative_maos_home_before_any_filesystem_step() {
    let temp = tempfile::tempdir().expect("create relative-home fixture");
    let root = temp.path();
    let sentinel = root.join("sentinel");
    std::fs::write(&sentinel, b"survive").expect("write sentinel");
    let refused = maos_command(root)
        .env("MAOS_HOME", "relative-home")
        .args(["purge", "--yes"])
        .output()
        .expect("run relative-home purge");
    assert_eq!(refused.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("cannot resolve MAOS home") && stderr.contains("must be an absolute path"),
        "typed resolution refusal missing: {stderr}"
    );
    assert_eq!(std::fs::read(&sentinel).expect("read sentinel"), b"survive");
    assert!(
        root.join("relative-home").symlink_metadata().is_err(),
        "the relative home must never be materialized"
    );
    assert!(
        !std::fs::read_dir(root)
            .expect("list fixture root")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("maos-purge-receipt-")),
        "a refusal before the snapshot must not create a receipt"
    );
}

#[cfg(unix)]
#[test]
fn purge_refuses_unsafe_receipt_paths_before_mutating_state() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("create receipt-safety fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let linked_parent = root.join("linked-receipt-parent");
    symlink(root.join("maos-home"), &linked_parent).expect("link receipt parent into MAOS home");
    let before = filesystem_identity(root);
    for unsafe_receipt in [
        root.join("maos-home/receipt.json"),
        linked_parent.join("receipt.json"),
    ] {
        let refused = maos_command(root)
            .args(["purge", "--yes", "--receipt"])
            .arg(&unsafe_receipt)
            .output()
            .expect("run unsafe-receipt purge");
        assert_eq!(refused.status.code(), Some(78));
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("ReceiptPathUnsafe"),
            "receipt refusal must be typed: {}",
            String::from_utf8_lossy(&refused.stderr)
        );
        assert!(!unsafe_receipt.exists());
        assert_eq!(
            filesystem_identity(root),
            before,
            "receipt preflight refusal must not materialize or remove a path"
        );
    }

    let target = root.join("operator-owned.txt");
    std::fs::write(&target, b"survive").expect("write symlink target");
    let planted = root.join("preplanted-receipt.json");
    symlink(&target, &planted).expect("plant receipt symlink");
    let refused = maos_command(root)
        .args(["purge", "--yes", "--receipt"])
        .arg(&planted)
        .output()
        .expect("run preplanted-symlink purge");
    assert!(!refused.status.success());
    assert_eq!(refused.status.code(), Some(78));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("ReceiptPathUnsafe"),
        "a planted symlink at the receipt path is a typed refusal: {}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        std::fs::symlink_metadata(&planted)
            .expect("planted receipt path survives")
            .file_type()
            .is_symlink(),
        "the refusal must leave the planted symlink exactly as it was"
    );
    assert_eq!(
        std::fs::read(&target).expect("read symlink target"),
        b"survive",
        "secure receipt creation must never follow or overwrite a planted symlink"
    );
}

#[cfg(unix)]
#[test]
fn purge_refuses_live_store_without_changing_filesystem_identity() {
    use std::io::Write;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let temp = tempfile::tempdir().expect("create lock-refusal fixture");
    let root = temp.path();
    let init = maos_command(root)
        .args(["init", "--plain"])
        .output()
        .expect("run init");
    assert!(init.status.success());

    let mut live_root = maos_command(root)
        .args(["shell", "--plain"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start live MAOS root");
    let audit = root.join("maos-home/audit/transparency.sqlite");
    let journal = root.join("maos-home/journal/lifecycle.ndjson");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !audit.exists() || !journal.exists() {
        assert!(
            Instant::now() < deadline,
            "live root did not finish materializing its locked stores"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    let before = filesystem_identity(root);
    let receipt = root.join("must-not-exist.json");
    let refused = maos_command(root)
        .args(["purge", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run contending purge");
    assert_eq!(refused.status.code(), Some(69));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("StoreInUse") && stderr.contains("refusing to erase"),
        "typed refusal missing: {stderr}"
    );
    assert!(!receipt.exists(), "refusal must not create a receipt");
    assert_eq!(
        filesystem_identity(root),
        before,
        "refusal must restore paths created before lock contention is known"
    );

    live_root
        .stdin
        .as_mut()
        .expect("live root stdin")
        .write_all(b"/quit\n")
        .expect("stop live root");
    let output = live_root.wait_with_output().expect("wait for live root");
    assert!(
        output.status.success(),
        "live root failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Item 4(a) — a root that is itself a symlink is a typed refusal, and the
/// symlink's target must not be traversed or touched.
#[cfg(unix)]
#[test]
fn purge_refuses_a_symbolic_link_root_and_never_touches_its_target() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().expect("create symlink-root fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let outside = root.join("operator-outside");
    std::fs::create_dir_all(&outside).expect("create outside target");
    std::fs::write(outside.join("sentinel.txt"), b"survive").expect("write outside sentinel");
    let crl = root.join("crl");
    std::fs::create_dir_all(&crl).expect("create CRL root");
    std::fs::write(crl.join("list.json"), b"{}").expect("write CRL entry");
    std::fs::remove_dir_all(&crl).expect("clear the CRL root for the symlink plant");
    symlink(&outside, &crl).expect("plant a symlink at the CRL root");

    let refused = maos_command(root)
        .args(["purge", "--yes"])
        .output()
        .expect("run symlink-root purge");
    assert_eq!(refused.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("refusing symbolic-link root") && stderr.contains("crl"),
        "typed symlink refusal missing: {stderr}"
    );
    assert_eq!(
        std::fs::read(outside.join("sentinel.txt")).expect("read outside sentinel"),
        b"survive",
        "the symlink target must be untouched"
    );
    assert!(
        root.join("maos-home").symlink_metadata().is_ok(),
        "the refusal must happen before any deletion"
    );
    assert!(
        !std::fs::read_dir(root)
            .expect("list fixture root")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("maos-purge-receipt-")),
        "a preflight refusal must not create a receipt"
    );
}

/// Item 4(b) — a regular file planted at a Directory root is `left (type
/// mismatch)`: it survives, is named on stdout, and the receipt carries it.
#[test]
fn purge_reports_a_type_mismatched_directory_root_and_leaves_it_named() {
    let temp = tempfile::tempdir().expect("create type-mismatch fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let memory = root.join("memory");
    std::fs::remove_dir_all(&memory).expect("clear the memory root for the file plant");
    std::fs::write(&memory, b"operator file, not a directory").expect("plant a regular file");

    let receipt = root.join("type-mismatch-receipt.json");
    let purge = maos_command(root)
        .args(["purge", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run type-mismatch purge");
    assert!(
        purge.status.success(),
        "a type mismatch is a per-root disposition, not a run failure: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    let stdout = String::from_utf8_lossy(&purge.stdout);
    assert!(
        stdout.contains("maos purge: left (type mismatch): private memory (memory)"),
        "stdout must name the mismatched root: {stdout}"
    );
    assert!(
        stdout.contains("maos purge: left behind: memory"),
        "stdout must name the surviving file: {stdout}"
    );
    assert_eq!(
        std::fs::read(&memory).expect("the planted file must survive"),
        b"operator file, not a directory"
    );
    assert!(
        root.join("maos-home").symlink_metadata().is_err(),
        "every genuine MAOS root is still removed"
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read receipt"))
            .expect("decode receipt");
    assert_eq!(record["status"], "complete");
    let roots = record["roots"].as_array().expect("receipt roots");
    assert!(
        roots
            .iter()
            .any(|entry| entry["path"] == memory.display().to_string()
                && entry["disposition"] == "left (type mismatch)"),
        "receipt must carry the type mismatch: {roots:#?}"
    );
}

/// Item 4(c) — with HOME and MAOS_HOME both unset, the permanent legacy leg
/// (the producers' /tmp fallback) is still enumerated and the unresolvable
/// home is disclosed. The dry run must not create any /tmp artifact.
#[test]
fn purge_dry_run_enumerates_the_legacy_tmp_leg_when_home_is_unset() {
    let temp = tempfile::tempdir().expect("create legacy-leg fixture");
    let root = temp.path();
    let legacy_tree = PathBuf::from("/tmp/.local/share/maos");
    let import_cache = PathBuf::from("/tmp/.cache/maos/import");
    let legacy_before = legacy_tree.symlink_metadata().is_ok();
    let cache_before = import_cache.symlink_metadata().is_ok();

    // No HOME, no MAOS_HOME; the XDG legs stay inside the scratch root so the
    // store-lock candidates never leave the fixture.
    let dry_run = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env_clear()
        .current_dir(root)
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_CRL_PATH", root.join("crl"))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .arg("purge")
        .output()
        .expect("run home-unset dry run");
    assert!(
        dry_run.status.success(),
        "dry run failed: {}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    let stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(
        stdout.contains("maos purge: unresolvable: MAOS home (neither MAOS_HOME nor HOME set)"),
        "the unresolvable home must be disclosed: {stdout}"
    );
    assert!(
        stdout.contains("legacy registry and skills data tree"),
        "the /tmp legacy leg must be enumerated: {stdout}"
    );
    assert!(
        stdout.contains("registry import cache"),
        "the /tmp import cache must be enumerated: {stdout}"
    );

    // Cleanup tripwire: the dry run must have created neither /tmp artifact.
    for (path, existed) in [(&legacy_tree, legacy_before), (&import_cache, cache_before)] {
        if path.symlink_metadata().is_ok() && !existed {
            if path.is_dir() {
                std::fs::remove_dir_all(path).expect("remove dry-run /tmp artifact");
            } else {
                std::fs::remove_file(path).expect("remove dry-run /tmp artifact");
            }
            panic!("a dry run must not create {}", path.display());
        }
    }
}

/// Item 7 / AC4 — a store directory that cannot be opened is a typed 78
/// `LockUnavailable` before any deletion. The 000 mode sits ON the store
/// directory itself (the audit database's parent in lock-set terms): an
/// unopenable ANCESTOR of an enumerated root would trip the earlier
/// filesystem-snapshot walk as a plain IO error instead.
#[cfg(unix)]
#[test]
fn purge_refuses_an_unopenable_store_directory_as_lock_unavailable() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("create lock-unavailable fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let memory = root.join("memory");
    std::fs::write(memory.join("kept.bin"), b"survive").expect("seed memory content");
    std::fs::set_permissions(&memory, std::fs::Permissions::from_mode(0o000))
        .expect("close the store directory");
    // uid 0 can open anything: the fixture is not constructible there.
    if std::fs::File::open(&memory).is_ok() {
        std::fs::set_permissions(&memory, std::fs::Permissions::from_mode(0o700))
            .expect("restore store directory");
        return;
    }

    let refused = maos_command(root)
        .args(["purge", "--yes"])
        .output()
        .expect("run lock-unavailable purge");
    std::fs::set_permissions(&memory, std::fs::Permissions::from_mode(0o700))
        .expect("restore store directory");
    assert_eq!(refused.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("LockUnavailable") && stderr.contains("cannot lock store directory"),
        "typed lock refusal missing: {stderr}"
    );
    assert_eq!(
        std::fs::read(memory.join("kept.bin")).expect("read memory content"),
        b"survive",
        "nothing may be deleted when the lock set cannot be taken"
    );
    assert!(
        root.join("maos-home").symlink_metadata().is_ok(),
        "the home must survive a lock refusal"
    );
    assert!(
        !std::fs::read_dir(root)
            .expect("list fixture root")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("maos-purge-receipt-")),
        "a lock refusal must not create a receipt"
    );
}

/// Item 5 — FilesystemSnapshot::restore must undo the directories the lock
/// acquisition materializes, on BOTH the dry-run path and the live-holder
/// refusal path: the full-tree filesystem identity returns to exactly the
/// pre-purge state.
#[cfg(unix)]
#[test]
fn snapshot_restore_undoes_lock_acquisition_on_dry_run_and_contention_paths() {
    use std::io::Write;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let temp = tempfile::tempdir().expect("create snapshot-restore fixture");
    let root = temp.path();
    initialize_and_boot(root);

    // The boot's lock acquisition created the proofs dir eagerly; delete it
    // so every purge below must materialize it and restore must remove it.
    let proofs = root.join("proofs");
    std::fs::remove_dir_all(&proofs).expect("delete the proofs root");
    let before = filesystem_identity(root);

    let dry_run = maos_command(root)
        .arg("purge")
        .output()
        .expect("run restore dry run");
    assert!(dry_run.status.success());
    assert!(
        String::from_utf8_lossy(&dry_run.stdout)
            .contains("maos purge: absent: erasure proofs (proofs)"),
        "the deleted root must be disclosed as absent"
    );
    assert_eq!(
        filesystem_identity(root),
        before,
        "the dry run must materialize and then remove the missing store root"
    );

    let mut live_root = maos_command(root)
        .args(["shell", "--plain"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start live MAOS root");
    let audit = root.join("maos-home/audit/transparency.sqlite");
    let journal = root.join("maos-home/journal/lifecycle.ndjson");
    // The FIRST boot's stores already exist, so waiting on audit/journal
    // alone races the live shell's own lock acquisition; also wait for the
    // proofs dir it re-materializes before deleting it again.
    let deadline = Instant::now() + Duration::from_secs(10);
    while !audit.exists() || !journal.exists() || !proofs.exists() {
        assert!(
            Instant::now() < deadline,
            "live root did not finish materializing its locked stores"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    std::fs::remove_dir_all(&proofs).expect("delete the proofs root again");
    let contended = filesystem_identity(root);

    let refused = maos_command(root)
        .args(["purge", "--yes"])
        .output()
        .expect("run contending purge");
    assert_eq!(refused.status.code(), Some(69));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("StoreInUse"),
        "typed contention refusal missing: {stderr}"
    );
    assert_eq!(
        filesystem_identity(root),
        contended,
        "the refused purge must materialize and then remove the missing store root"
    );

    live_root
        .stdin
        .as_mut()
        .expect("live root stdin")
        .write_all(b"/quit\n")
        .expect("stop live root");
    let output = live_root.wait_with_output().expect("wait for live root");
    assert!(
        output.status.success(),
        "live root failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Item 8 — a second open connection keeps the retained database's WAL
/// sidecar alive through the checkpoint; `--keep-log` must then refuse the
/// whole run (typed audit error) and remove nothing.
#[test]
fn keep_log_refuses_to_finish_when_a_retained_sidecar_survives() {
    let temp = tempfile::tempdir().expect("create sidecar fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let audit = root.join("maos-home/audit/transparency.sqlite");
    let before_rows = maos_audit::query(&audit, maos_audit::AuditFilter::default())
        .expect("query the audit log")
        .len();
    assert!(before_rows > 0, "sidecar control needs a populated log");
    // Hold the database open across the purge: the checkpoint is then not the
    // last connection, so the -wal sidecar file survives it.
    let holder = rusqlite::Connection::open(&audit).expect("open the retaining connection");
    holder
        .query_row("SELECT count(*) FROM transparency_log", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("touch the retained database");

    let receipt = root.join("sidecar-receipt.json");
    let purge = maos_command(root)
        .args(["purge", "--keep-log", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run sidecar-refusing purge");
    assert_eq!(
        purge.status.code(),
        Some(1),
        "the sidecar refusal is the typed audit error: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    let stderr = String::from_utf8_lossy(&purge.stderr);
    assert!(
        stderr.contains("retained database sidecar still exists after checkpoint"),
        "typed sidecar refusal missing: {stderr}"
    );
    // The refusal fires in the preflight: no receipt, no deletion at all.
    assert!(
        receipt.symlink_metadata().is_err(),
        "a checkpoint refusal must not create a receipt"
    );
    assert!(root.join("maos-home").symlink_metadata().is_ok());
    assert_eq!(
        maos_audit::query(&audit, maos_audit::AuditFilter::default())
            .expect("query the untouched log")
            .len(),
        before_rows,
        "nothing may be removed when a sidecar survives"
    );
    drop(holder);
}

/// Item 9(z) — running from inside $MAOS_HOME with the DEFAULT
/// (cwd-relative) receipt path puts the receipt inside the tree being erased:
/// typed 78 refusal before any deletion.
#[test]
fn purge_refuses_a_default_receipt_inside_the_home_it_is_erasing() {
    let temp = tempfile::tempdir().expect("create cwd-receipt fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let home = root.join("maos-home");
    let refused = maos_command(root)
        .current_dir(&home)
        .args(["purge", "--yes"])
        .output()
        .expect("run in-home purge");
    assert_eq!(refused.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("ReceiptPathUnsafe") && stderr.contains("resolves inside deleted root"),
        "typed receipt refusal missing: {stderr}"
    );
    assert!(
        home.symlink_metadata().is_ok(),
        "the refusal must happen before any deletion"
    );
    assert!(
        !std::fs::read_dir(&home)
            .expect("list home")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("maos-purge-receipt-")),
        "the refused receipt must never exist"
    );
}

/// Item 9(aa) — a failure MID-deletion leaves the receipt on disk with status
/// `in-progress`, naming the root that never completed (`pending`), instead
/// of overwriting it with a finished verdict.
#[cfg(unix)]
#[test]
fn purge_failure_mid_deletion_leaves_an_in_progress_receipt() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("create mid-deletion fixture");
    let root = temp.path();
    initialize_and_boot(root);

    let memory = root.join("memory");
    let locked_dir = memory.join("sub");
    std::fs::create_dir_all(&locked_dir).expect("create locked subdirectory");
    std::fs::write(locked_dir.join("locked.bin"), b"cannot remove me")
        .expect("seed the unremovable file");
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o0555))
        .expect("remove write permission from the subdirectory");
    if std::fs::write(locked_dir.join(".probe"), b"").is_ok() {
        let _ = std::fs::remove_file(locked_dir.join(".probe"));
        std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o0700))
            .expect("restore subdirectory");
        return; // uid 0: the permission failure is not constructible.
    }

    let receipt = root.join("mid-deletion-receipt.json");
    let purge = maos_command(root)
        .args(["purge", "--yes", "--receipt"])
        .arg(&receipt)
        .output()
        .expect("run mid-deletion purge");
    std::fs::set_permissions(&locked_dir, std::fs::Permissions::from_mode(0o0700))
        .expect("restore subdirectory for cleanup");
    assert_eq!(
        purge.status.code(),
        Some(1),
        "a mid-deletion IO failure is a plain error exit: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    assert!(
        String::from_utf8_lossy(&purge.stderr).contains("remove file"),
        "the error must name the failed operation: {}",
        String::from_utf8_lossy(&purge.stderr)
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read surviving receipt"))
            .expect("decode receipt");
    assert_eq!(
        record["status"], "in-progress",
        "a mid-deletion failure must leave the receipt in-progress: {record}"
    );
    let roots = record["roots"].as_array().expect("receipt roots");
    assert!(
        roots
            .iter()
            .any(|entry| entry["path"] == memory.display().to_string()
                && entry["disposition"] == "pending"),
        "the receipt must name the root that never completed: {roots:#?}"
    );
    assert!(
        locked_dir.join("locked.bin").symlink_metadata().is_ok(),
        "the unremovable file must survive"
    );
}

/// Item 13 — the composition root selects the secret store BEFORE providers
/// boot: encrypted-file without a healthy master key is a typed 78, while
/// backend=env with an API key registers the Anthropic provider.
#[test]
fn composition_root_selects_the_secret_backend_before_providers_boot() {
    use std::io::Write;
    use std::process::Stdio;

    let temp = tempfile::tempdir().expect("create composition fixture");
    let root = temp.path();

    // encrypted-file with NO MAOS_KMS_MASTER_KEY: typed 78 at selection time.
    let mut refused_child = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env_clear()
        .current_dir(root)
        .env("HOME", root.join("home-encrypted"))
        .env("XDG_DATA_HOME", root.join("xdg-data-encrypted"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config-encrypted"))
        .env("MAOS_SECRETS_BACKEND", "encrypted-file")
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start encrypted-file boot");
    refused_child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"/quit\n")
        .ok();
    let refused = refused_child.wait_with_output().expect("wait for boot");
    assert_eq!(refused.status.code(), Some(78));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(
        stderr.contains("MAOS_SECRETS_BACKEND=encrypted-file requires a healthy")
            && stderr.contains("MAOS_KMS_MASTER_KEY"),
        "typed KMS refusal missing: {stderr}"
    );

    // backend=env with a key: the Anthropic provider registers on stderr.
    let mut env_child = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env_clear()
        .current_dir(root)
        .env("HOME", root.join("home-env"))
        .env("XDG_DATA_HOME", root.join("xdg-data-env"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config-env"))
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_ANTHROPIC_API_KEY", "sk-ant-composition-root-probe")
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start env-backend boot");
    env_child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"/quit\n")
        .expect("quit env-backend shell");
    let env_boot = env_child.wait_with_output().expect("wait for env boot");
    assert!(
        env_boot.status.success(),
        "env-backend boot failed: {}",
        String::from_utf8_lossy(&env_boot.stderr)
    );
    assert!(
        String::from_utf8_lossy(&env_boot.stderr).contains("maos: Anthropic provider registered"),
        "the env backend must register the Anthropic provider: {}",
        String::from_utf8_lossy(&env_boot.stderr)
    );
}

#[cfg(unix)]
fn filesystem_identity(root: &Path) -> std::collections::BTreeMap<PathBuf, (u64, u64, u32)> {
    use std::os::unix::fs::MetadataExt;

    fn visit(
        root: &Path,
        path: &Path,
        entries: &mut std::collections::BTreeMap<PathBuf, (u64, u64, u32)>,
    ) {
        let metadata = std::fs::symlink_metadata(path).expect("snapshot metadata");
        entries.insert(
            path.strip_prefix(root).unwrap_or(path).to_path_buf(),
            (metadata.dev(), metadata.ino(), metadata.mode()),
        );
        if metadata.is_dir() {
            let mut children: Vec<PathBuf> = std::fs::read_dir(path)
                .expect("snapshot directory")
                .map(|entry| entry.expect("snapshot entry").path())
                .collect();
            children.sort();
            for child in children {
                visit(root, &child, entries);
            }
        }
    }

    let mut entries = std::collections::BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

// Item 10 — the stub exists only so non-unix builds compile; every caller is
// cfg(unix)-gated, so no identity assertion can ever run vacuously against an
// empty map, and on unix the real walker makes the vacuous path impossible.
#[cfg(not(unix))]
#[allow(dead_code)]
fn filesystem_identity(_root: &Path) -> std::collections::BTreeMap<PathBuf, (u64, u64, u32)> {
    std::collections::BTreeMap::new()
}

const NO_ROOTS_CHILD_ENV: &str = "MAOS_16_4_NO_ROOTS_CHILD";

#[test]
fn init_prints_an_executable_truthful_purge_instruction() {
    let temp = tempfile::tempdir().expect("create instruction fixture");
    let root = temp.path();
    let mut init = Command::new(env!("CARGO_BIN_EXE_maos"));
    apply_default_home_environment(&mut init, root);
    let output = init
        .args(["init", "--plain"])
        .output()
        .expect("run default-path init");
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("init stdout UTF-8");
    let prefix = "maos: to remove all data, run:  ";
    let instruction = stdout
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .expect("init removal instruction");
    let argv: Vec<&str> = instruction.split_whitespace().collect();
    assert_eq!(argv.first().copied(), Some("maos"));

    let mut purge = Command::new(env!("CARGO_BIN_EXE_maos"));
    apply_default_home_environment(&mut purge, root);
    let purge_output = purge
        .args(&argv[1..])
        .output()
        .expect("execute exactly the printed purge instruction");
    assert!(
        purge_output.status.success(),
        "printed instruction failed: stdout={} stderr={}",
        String::from_utf8_lossy(&purge_output.stdout),
        String::from_utf8_lossy(&purge_output.stderr)
    );
    let mut reinit = Command::new(env!("CARGO_BIN_EXE_maos"));
    apply_default_home_environment(&mut reinit, root);
    assert!(reinit
        .args(["init", "--plain"])
        .status()
        .expect("reinitialize")
        .success());
    let mut second_purge = Command::new(env!("CARGO_BIN_EXE_maos"));
    apply_default_home_environment(&mut second_purge, root);
    let second_output = second_purge
        .args(&argv[1..])
        .output()
        .expect("repeat the printed purge instruction");
    assert!(
        second_output.status.success(),
        "repeated printed instruction failed: {}",
        String::from_utf8_lossy(&second_output.stderr)
    );
    let receipt_count = std::fs::read_dir(root)
        .expect("list receipt directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("maos-purge-receipt-")
        })
        .count();
    assert_eq!(
        receipt_count, 2,
        "each purge must retain an independent receipt"
    );

    let status = Command::new(std::env::current_exe().expect("resolve test executable"))
        .arg("--exact")
        .arg("assert_no_maos_roots_child")
        .arg("--nocapture")
        .env_clear()
        .current_dir(root)
        .env(NO_ROOTS_CHILD_ENV, "1")
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_CRL_PATH", root.join("crl"))
        .status()
        .expect("run isolated post-purge resolver check");
    assert!(
        status.success(),
        "printed instruction left MAOS state behind"
    );
}

#[test]
fn assert_no_maos_roots_child() {
    if std::env::var_os(NO_ROOTS_CHILD_ENV).is_none() {
        return;
    }
    for root in maos_roots()
        .into_iter()
        .filter(|root| root.written_by == WrittenBy::Maos)
    {
        assert!(
            std::fs::symlink_metadata(&root.path).is_err(),
            "MAOS root survived printed purge instruction: {root:#?}"
        );
    }
}

fn apply_default_home_environment(command: &mut Command, root: &Path) {
    command
        .env_clear()
        .current_dir(root)
        .env("HOME", root.join("home"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("MAOS_CRL_PATH", root.join("crl"))
        .env("MAOS_NOTIFY_DISABLE", "1");
}
