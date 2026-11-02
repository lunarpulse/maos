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
        let script = format!("#!/bin/sh\nroot='{}'\nprintf '%s\\n' \"$*\" >> \"$root/calls\"\n[ -z \"$AWS_SECRET_ACCESS_KEY\" ] || exit 99\ncase \"$1\" in\nps) cat \"$root/candidates\";;\ninspect) cat \"$root/$2.json\";;\nrm) [ -f \"$root/receipt.json\" ] || exit 98; grep -q '\"pending\"' \"$root/receipt.json\" || exit 97; touch \"$root/removed-$5\";;\ncontainer) [ -f \"$root/removed-$3\" ] && exit 1; [ \"$3\" = '{absent_cid}' ] && exit 1; exit 0;;\nunshare) exit 0;;\n*) exit 96;;\nesac\n",root.display());
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
            out.status.success(),
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
    let empty = f.private("empty");
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
    let empty = f.private("empty");
    let absent = f.private("process/absent");
    let present = f.private("process/present");
    let unknown = f.private("process/unknown");
    std::fs::write(absent.join("cid"), format!("{:064x}", 6)).expect("absent CID");
    std::fs::write(present.join("cid"), &f.other).expect("present CID");
    let out = f.purge(true);
    assert!(!empty.exists(), "empty process root leaked");
    assert!(!absent.exists(), "absent CID custody leaked");
    assert!(present.exists(), "live home's custody removed");
    assert!(unknown.exists(), "unknown custody removed");
    let report = String::from_utf8_lossy(&out.stdout);
    for forbidden in [
        "termination_unconfirmed",
        "kill_failed",
        "runtime_client_unconfirmed",
    ] {
        assert!(!report.contains(forbidden), "{report}");
    }
}
#[test]
fn ac6_runtime_helper_is_shared_and_receipt_convention_is_preserved() {
    let source = include_str!("../src/purge.rs");
    assert!(
        !source.contains("child.try_wait()"),
        "purge has a second bounded runtime implementation"
    );
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
