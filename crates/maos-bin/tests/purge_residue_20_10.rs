//! Story 20-10: runtime authority, receipt ordering, inertness, and shared custody.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

struct Fixture {
    dir: tempfile::TempDir,
    owned: String,
    other: String,
    missing: String,
    mismatch: String,
    impostor: String,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("scratch");
        let root = dir.path();
        for path in [
            "home",
            "bin",
            "runtime",
            "data/containers/storage",
            "maos/workspaces",
        ] {
            std::fs::create_dir_all(root.join(path)).expect("fixture directory");
        }
        std::fs::create_dir(root.join("maos/workspaces/maos-worker-7")).expect("owned workspace");
        std::fs::write(
            root.join("maos/workspaces/maos-worker-7/sentinel"),
            b"workspace must still exist at rm",
        )
        .expect("sentinel");
        std::fs::set_permissions(root.join("runtime"), std::fs::Permissions::from_mode(0o700))
            .expect("private runtime");
        let ids: Vec<String> = (1..=5).map(|i| format!("{i:064x}")).collect();
        let mut rows = Vec::new();
        for (index, id) in ids.iter().enumerate() {
            let name = if index == 3 {
                "maos-wrong"
            } else if index == 4 {
                "maos-impostor"
            } else {
                "maos-worker-7"
            };
            let source = if index == 1 {
                root.join("other/workspaces/maos-worker-7")
            } else {
                root.join("maos/workspaces/maos-worker-7")
            };
            rows.push(serde_json::json!({"Id":id,"Name":name,"Config":{"Labels":if index == 4 {serde_json::json!({})} else {serde_json::json!({"maos.spirit_id":"worker","maos.boot_nonce":"7"})}},"Mounts":if index == 2 {serde_json::json!([])} else {serde_json::json!([{"Destination":"/workspace","Source":source}])}}));
        }
        for (id, row) in ids.iter().zip(rows) {
            std::fs::write(
                root.join(format!("{id}.json")),
                serde_json::to_vec(&vec![row]).expect("fixture JSON"),
            )
            .expect("inspect fixture");
        }
        std::fs::write(root.join("candidates"), ids.join("\n")).expect("candidates");
        let absent_cid = format!("{:064x}", 6);
        let script = format!("#!/bin/sh\nroot='{}'\nprintf '%s\\n' \"$*\" >> \"$root/calls\"\n[ -z \"$AWS_SECRET_ACCESS_KEY\" ] || exit 99\ncase \"$1\" in\nps) cat \"$root/candidates\";;\ninspect) cat \"$root/$2.json\";;\nrm) [ -f \"$root/maos/workspaces/maos-worker-7/sentinel\" ] || {{ echo 'workspace removed before container' >&2; exit 96; }}; [ -f \"$root/receipt.json\" ] || exit 98; grep -q '\"pending\"' \"$root/receipt.json\" || exit 97; for cidfile in \"$root\"/runtime/maos-worker-state/*/*/cid \"$root\"/maos-worker-state-*/*/*/cid; do if [ -f \"$cidfile\" ] && [ \"$(cat \"$cidfile\")\" = \"$5\" ]; then rm -f -- \"$cidfile\" || exit 95; fi; done; touch \"$root/removed-$5\";;\ncontainer) [ -f \"$root/removed-$3\" ] && exit 1; [ \"$3\" = '{absent_cid}' ] && exit 1; exit 0;;\nunshare) exit 0;;\n*) exit 96;;\nesac\n",root.display());
        std::fs::write(root.join("bin/podman"), script).expect("stub");
        std::fs::set_permissions(
            root.join("bin/podman"),
            std::fs::Permissions::from_mode(0o700),
        )
        .expect("executable");
        Self {
            dir,
            owned: ids[0].clone(),
            other: ids[1].clone(),
            missing: ids[2].clone(),
            mismatch: ids[3].clone(),
            impostor: ids[4].clone(),
        }
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn command(&self) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_maos"));
        c.env_clear()
            .current_dir(self.root())
            .env("HOME", self.root().join("home"))
            .env("MAOS_HOME", self.root().join("maos"))
            .env("XDG_DATA_HOME", self.root().join("data"))
            .env("XDG_CONFIG_HOME", self.root().join("config"))
            .env("XDG_RUNTIME_DIR", self.root().join("runtime"))
            .env("TMPDIR", self.root())
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.root().join("bin").display()),
            )
            .env("AWS_SECRET_ACCESS_KEY", "must-not-reach-runtime");
        c
    }
    fn purge(&self, yes: bool) -> Output {
        let mut c = self.command();
        c.arg("purge");
        if yes {
            c.args(["--yes", "--receipt"])
                .arg(self.root().join("receipt.json"));
        }
        let out = c.output().expect("purge");
        assert!(
            out.status.success()
                || (yes
                    && out.status.code() == Some(1)
                    && String::from_utf8_lossy(&out.stdout)
                        .contains("maos purge: complete with residue:")),
            "stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
    fn calls(&self) -> String {
        std::fs::read_to_string(self.root().join("calls")).unwrap_or_default()
    }
    fn private(&self, relative: &str) -> PathBuf {
        let p = self.root().join("runtime/maos-worker-state").join(relative);
        std::fs::create_dir_all(&p).expect("state");
        let mut current = Some(p.as_path());
        while let Some(path) = current {
            if path == self.root().join("runtime") {
                break;
            }
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).expect("mode");
            current = path.parent();
        }
        p
    }
}
#[test]
fn ac1_owned_cid_removed_before_workspaces_and_receipted() {
    let f = Fixture::new();
    f.purge(true);
    let calls = f.calls();
    assert!(
        calls.contains(&format!("rm --force --volumes --time=0 {}", f.owned)),
        "{calls}"
    );
    assert!(
        calls.contains(&format!("container exists {}", f.owned)),
        "{calls}"
    );
    let text = std::fs::read_to_string(f.root().join("receipt.json")).expect("receipt");
    let receipt: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("receipt: {e}"));
    assert_eq!(receipt["schema_version"], 1);
    let rows = receipt["roots"]
        .as_array()
        .unwrap_or_else(|| panic!("roots"));
    assert!(
        rows.iter()
            .any(|r| r["path"] == format!("podman://{}", f.owned) && r["disposition"] == "removed"),
        "{text}"
    );
}

#[test]
fn ac1_listed_candidate_vanishing_before_inspect_is_skipped() {
    let f = Fixture::new();
    let vanished = format!("{:064x}", 6);
    std::fs::write(
        f.root().join("candidates"),
        format!("{}\n{vanished}\n", f.owned),
    )
    .expect("racing candidates");
    let out = f
        .command()
        .args(["purge", "--dry-run"])
        .output()
        .expect("dry purge");
    assert!(
        out.status.success(),
        "vanished listed candidate must not abort read-only purge: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let calls = f.calls();
    assert!(calls.contains(&format!("inspect {vanished}")), "{calls}");
    assert!(
        calls.contains(&format!("container exists {vanished}")),
        "{calls}"
    );
    assert!(!f.root().join("receipt.json").exists());
    for call in calls.lines() {
        assert!(
            call.starts_with("ps ")
                || call.starts_with("inspect ")
                || call.starts_with("container exists "),
            "dry-run mutation: {call}"
        );
    }
}

#[test]
fn ac1_failed_inspect_with_present_or_unconfirmed_candidate_remains_fatal() {
    for mode in ["present", "error", "timeout"] {
        let f = Fixture::new();
        std::fs::write(f.root().join("candidates"), &f.other).expect("candidate");
        std::fs::remove_file(f.root().join(format!("{}.json", f.other)))
            .expect("make inspect fail");
        if mode != "present" {
            let stub = f.root().join("bin/podman");
            let script = std::fs::read_to_string(&stub).expect("stub");
            let response = if mode == "error" {
                "container) exit 125; "
            } else {
                "container) sleep 10; exit 1; "
            };
            std::fs::write(&stub, script.replace("container) ", response)).expect("exists failure");
        }
        let out = f
            .command()
            .args(["purge", "--dry-run"])
            .output()
            .expect("dry refusal");
        let error = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(78), "{mode}: {error}");
        assert!(
            error.contains("status=") && error.contains("stderr=") && error.contains(".json"),
            "inspect exit/stderr evidence missing for {mode}: {error}"
        );
        let calls = f.calls();
        assert!(
            calls.contains(&format!("container exists {}", f.other)),
            "{calls}"
        );
        assert!(!calls
            .lines()
            .any(|call| call.starts_with("rm ") || call.starts_with("unshare ")));
        assert!(!f.root().join("receipt.json").exists());
    }
}
#[test]
fn ac2_other_home_missing_mount_mismatched_name_and_impostor_survive() {
    let f = Fixture::new();
    let out = f.purge(true);
    let report = String::from_utf8_lossy(&out.stdout);
    let calls = f.calls();
    for cid in [&f.other, &f.missing, &f.mismatch] {
        assert!(
            report.contains(&format!("podman rm --force {cid}")),
            "{report}"
        );
        assert!(!calls.contains(&format!("--time=0 {cid}")), "{calls}");
    }
    assert!(!report.contains(&f.impostor), "{report}");
    assert!(!report.contains("maos-impostor"), "{report}");
}
#[test]
fn ac3_dry_run_uses_only_readonly_verbs_and_preserves_custody() {
    let f = Fixture::new();
    let empty = f.private("process-2147483647-00000000");
    let out = f.purge(false);
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(
        report.contains(&format!(
            "would remove container {} (maos-worker-7)",
            f.owned
        )),
        "{report}"
    );
    assert!(report.contains("would remove T3 state"), "{report}");
    assert!(empty.exists());
    assert!(!f.root().join("receipt.json").exists());
    for call in f.calls().lines() {
        assert!(
            call.starts_with("ps ") || call.starts_with("inspect "),
            "dry-run mutation: {call}"
        );
    }
}
#[test]
fn ac5_only_absent_cid_and_empty_state_are_reclaimed() {
    let f = Fixture::new();
    let empty = f.private("process-2147483647-00000000");
    let legacy_short = f.private("00000001");
    let live_process = f.private(&format!("process-{}-00000002", std::process::id()));
    let absent = f.private("process/absent");
    let present = f.private("process/present");
    let unknown = f.private("process/unknown");
    let removed = f.private("process/removed");
    std::fs::write(removed.join("cid"), &f.owned).expect("removed CID");
    std::fs::write(removed.join("pid"), b"2147483647").expect("spawn pid");
    std::fs::write(absent.join("cid"), format!("{:064x}", 6)).expect("absent CID");
    std::fs::write(present.join("cid"), &f.other).expect("present CID");
    let out = f.purge(true);
    assert!(!empty.exists(), "empty process root leaked");
    assert!(!absent.exists(), "absent CID custody leaked");
    assert!(
        !removed.exists(),
        "removed container's custody leaked after runtime deleted its cidfile"
    );
    assert!(present.exists(), "live home's custody removed");
    assert!(unknown.exists(), "unknown custody removed");
    assert!(
        legacy_short.exists(),
        "nonce-only custody has no creator-absence proof"
    );
    assert!(
        live_process.exists(),
        "empty live creator root must remain usable"
    );
    let report = String::from_utf8_lossy(&out.stdout);
    let bytes = std::fs::read(f.root().join("receipt.json")).expect("receipt");
    let receipt: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("{e}"));
    let rows = receipt["roots"]
        .as_array()
        .unwrap_or_else(|| panic!("receipt rows"));
    for path in [&present, &unknown, &legacy_short, &live_process] {
        assert!(
            report.contains(&format!(
                "left (T3 custody residue): T3 state {}",
                path.display()
            )),
            "missing retained state report: {report}"
        );
        assert!(
            rows.iter()
                .any(|row| row["path"] == path.display().to_string()
                    && row["written_by"] == "maos"
                    && row["disposition"] == "left (T3 custody residue)"),
            "missing retained custody receipt row: {receipt}"
        );
    }
    for forbidden in [
        "termination_unconfirmed",
        "kill_failed",
        "runtime_client_unconfirmed",
    ] {
        assert!(!report.contains(forbidden), "{report}");
    }
}

#[test]
fn ac5_missing_inventory_cid_is_retained_after_container_removal() {
    let f = Fixture::new();
    let unknown = f.private("process/missing-at-inventory");
    std::fs::write(unknown.join("pid"), b"2147483647").expect("unconfirmed spawn pid");
    assert!(!unknown.join("cid").exists());
    f.purge(true);
    assert!(f.root().join(format!("removed-{}", f.owned)).exists());
    assert!(
        unknown.join("pid").is_file() && !unknown.join("cid").exists(),
        "a CID never captured at inventory must not authorize custody removal"
    );
    let text = std::fs::read_to_string(f.root().join("receipt.json")).expect("receipt");
    let receipt: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("receipt: {e}"));
    assert!(
        receipt["roots"].as_array().is_some_and(|rows| rows
            .iter()
            .any(|row| row["path"] == unknown.display().to_string()
                && row["disposition"] == "left (T3 custody residue)")),
        "missing conservative custody receipt: {text}"
    );
}

#[test]
fn ac5_inventory_directory_replacement_retains_both_inodes() {
    let f = Fixture::new();
    let spawn = f.private("process/replaced");
    std::fs::write(spawn.join("cid"), &f.owned).expect("captured CID");
    std::fs::write(spawn.join("pid"), b"inventoried").expect("original custody");
    let stub = f.root().join("bin/podman");
    let script = std::fs::read_to_string(&stub).expect("runtime stub");
    let replacement = format!(
        "touch \"$root/removed-$5\"; mv '{0}' '{0}-inventoried'; mkdir -m 700 '{0}'; printf replacement > '{0}/pid'",
        spawn.display()
    );
    let swapped = script.replace("touch \"$root/removed-$5\"", &replacement);
    assert_ne!(
        swapped, script,
        "runtime replacement hook must be installed"
    );
    std::fs::write(&stub, swapped).expect("replacement hook");
    f.purge(true);
    assert_eq!(
        std::fs::read(spawn.join("pid")).expect("replacement custody"),
        b"replacement",
        "captured CID must not authorize the replacement directory"
    );
    assert_eq!(
        std::fs::read(spawn.with_file_name("replaced-inventoried").join("pid"))
            .expect("held original custody"),
        b"inventoried",
        "inode mismatch must refuse before clearing the held original"
    );
}
#[test]
fn ac6_runtime_helper_is_shared_and_receipt_convention_is_preserved() {
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut pending = vec![source_dir.clone()];
    let mut scanned = 0;
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("source directory") {
            let path = entry.expect("source entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative = path.strip_prefix(&source_dir).expect("source path");
            let purge_module = relative
                .components()
                .any(|component| component.as_os_str() == "purge")
                || path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("purge"));
            if path.extension().is_none_or(|extension| extension != "rs") || !purge_module {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("purge source");
            for spelling in [
                "Command::new",
                "std::process::Command",
                ".try_wait(",
                ".spawn(",
            ] {
                assert!(
                    !source.contains(spelling),
                    "ambient runtime runner {spelling} in {}",
                    path.display()
                );
            }
            scanned += 1;
        }
    }
    assert!(scanned >= 2, "every purge module must be scanned");
    let f = Fixture::new();
    f.purge(true);
    for call in f.calls().lines() {
        assert!(
            matches!(
                call.split_whitespace().next(),
                Some("ps" | "inspect" | "rm" | "container" | "unshare")
            ),
            "{call}"
        );
    }
    let text = std::fs::read_to_string(f.root().join("receipt.json")).expect("receipt");
    assert!(text.contains("\"complete\""), "{text}");
}
#[test]
fn ac7_adr_documents_home_scoped_runtime_removal() {
    let adr = include_str!("../../../docs/adr/ADR-067-maos-data-root-layout-and-purge.md");
    assert!(
        adr.contains("maos.spirit_id")
            && adr.contains("maos.boot_nonce")
            && adr.contains("podman://"),
        "ADR-067 still describes report-only runtime residue"
    );
}

#[test]
fn ac2_live_root_refuses_before_runtime_enumeration() {
    use std::io::Write;
    use std::process::Stdio;
    let f = Fixture::new();
    assert!(f
        .command()
        .args(["init", "--plain"])
        .output()
        .expect("init")
        .status
        .success());
    let mut shell = f
        .command()
        .args(["shell", "--plain"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("live root");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !f.root().join("maos/journal/lifecycle.ndjson").exists() {
        assert!(std::time::Instant::now() < deadline, "root did not start");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let refused = f
        .command()
        .args(["purge", "--yes", "--receipt"])
        .arg(f.root().join("receipt.json"))
        .output()
        .expect("contending purge");
    shell
        .stdin
        .take()
        .expect("stdin")
        .write_all(b"/quit\n")
        .expect("quit");
    assert!(shell.wait().expect("root exit").success());
    assert_eq!(
        refused.status.code(),
        Some(69),
        "{}",
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        f.calls().is_empty(),
        "refusal invoked runtime: {}",
        f.calls()
    );
    assert!(!f.root().join("receipt.json").exists());
}
#[test]
fn ac3_no_storage_means_no_runtime_call() {
    let f = Fixture::new();
    std::fs::remove_dir(f.root().join("data/containers/storage")).expect("remove storage fixture");
    f.purge(false);
    assert!(f.calls().is_empty(), "dry run initialized runtime storage");
}
#[test]
fn ac5_private_directory_validation_preserves_symlinks_and_wrong_modes() {
    let f = Fixture::new();
    let wrong = f.private("wrong");
    std::fs::set_permissions(&wrong, std::fs::Permissions::from_mode(0o755)).expect("wrong mode");
    let parent = f.private("process");
    let outside = f.root().join("outside");
    std::fs::create_dir(&outside).expect("outside");
    std::os::unix::fs::symlink(&outside, parent.join("linked")).expect("link");
    f.purge(true);
    assert!(wrong.exists() && outside.exists() && parent.join("linked").symlink_metadata().is_ok());
}

#[test]
fn review_f1_surviving_residue_has_an_explicit_summary_and_nonzero_exit() {
    let f = Fixture::new();
    let retained = f.private("legacy-process/unknown");
    let out = f
        .command()
        .args(["purge", "--yes", "--receipt"])
        .arg(f.root().join("receipt.json"))
        .output()
        .expect("purge");
    assert_eq!(
        out.status.code(),
        Some(1),
        "surviving custody must not be exit-zero success"
    );
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(
        report.contains("complete with residue:") && report.contains(" left"),
        "{report}"
    );
    let text = std::fs::read_to_string(f.root().join("receipt.json")).expect("receipt");
    let receipt: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(receipt["schema_version"], 1);
    assert_eq!(
        receipt["status"], "complete",
        "D-08 A status vocabulary is unchanged"
    );
    let rows = receipt["roots"]
        .as_array()
        .unwrap_or_else(|| panic!("receipt roots"));
    let left = rows
        .iter()
        .filter(|row| {
            row["written_by"] == "maos"
                && row["disposition"].as_str().is_some_and(|d| {
                    d.starts_with("left") || d == "partially-removed" || d == "backend-unavailable"
                })
        })
        .count();
    assert!(left > 0);
    let summary = format!("complete with residue: {left} left");
    assert_eq!(
        report.lines().last(),
        Some(format!("maos purge: {summary}").as_str())
    );
    assert!(
        receipt["next_step"]
            .as_str()
            .is_some_and(|s| s.starts_with(&summary)),
        "{text}"
    );
    assert!(retained.exists());
}
#[test]
fn review_f2_unset_relative_home_and_legacy_ancestor_links_are_refused() {
    for home in [None, Some("relative-home")] {
        let f = Fixture::new();
        let mut cmd = f.command();
        if let Some(home) = home {
            cmd.env("HOME", home);
        } else {
            cmd.env_remove("HOME");
        }
        let out = cmd.arg("purge").output().expect("dry refusal");
        assert_eq!(
            out.status.code(),
            Some(78),
            "HOME must be absolute and present: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(f.calls().is_empty(), "HOME refusal ran runtime");
    }
    let f = Fixture::new();
    let outside = f.root().join("outside");
    std::fs::create_dir_all(outside.join("share/maos")).expect("victim");
    std::fs::write(outside.join("share/maos/sentinel"), b"keep").expect("sentinel");
    std::os::unix::fs::symlink(&outside, f.root().join("home/.local"))
        .expect("legacy ancestor link");
    let out = f
        .command()
        .args(["purge", "--yes", "--receipt"])
        .arg(f.root().join("receipt.json"))
        .output()
        .expect("legacy refusal");
    assert_eq!(
        out.status.code(),
        Some(78),
        "legacy ancestor symlink was accepted"
    );
    assert!(outside.join("share/maos/sentinel").exists());
    assert!(f.calls().is_empty());
}
#[test]
fn review_f3_home_ancestor_and_parent_components_are_refused() {
    for target in ["home", ""] {
        let f = Fixture::new();
        let receipt = tempfile::tempdir().expect("outside receipt");
        let out = f
            .command()
            .env("MAOS_MEMORY_ROOT", f.root().join(target))
            .args(["purge", "--yes", "--receipt"])
            .arg(receipt.path().join("receipt.json"))
            .output()
            .expect("home boundary refusal");
        assert_eq!(
            out.status.code(),
            Some(78),
            "HOME/ancestor purge root accepted"
        );
        assert!(f.root().join("home").exists());
        assert!(f.calls().is_empty());
    }
    let f = Fixture::new();
    std::fs::create_dir(f.root().join("stores")).expect("audit parent");
    let out = f
        .command()
        .env_remove("MAOS_HOME")
        .env("MAOS_AUDIT_DB", f.root().join("stores/../audit.sqlite"))
        .args(["purge", "--keep-log"])
        .output()
        .expect("protected audit refusal");
    assert_eq!(
        out.status.code(),
        Some(78),
        "protected path with parent components accepted"
    );
    assert!(f.calls().is_empty());
}
