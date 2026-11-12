//! Story 20-9 review correction (GPT #2) — an installed component, booted by the shell root
//! (`InstalledSpirits::boot`, no `maos run`), is upgraded in place through the real door. The
//! swap handles used to be armed only by `maos run`, so this root's component was taken for an
//! in-process Spirit and its WASM successor was refused.
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "support/spawned_world_20_9.rs"]
mod world;

use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::time::{Duration, Instant};
use support::{publish_artifact, root, sibling, InProcessServer, Scratch};
use world::{manifest, ALL_HOOKS, ID};

struct Shell {
    scratch: Scratch,
    child: Option<Child>,
    /// The shell reads its REPL from here and leaves on EOF: held open for the vector's life.
    stdin: Option<ChildStdin>,
}

impl Shell {
    fn ctl(&self, args: &[&str]) -> std::process::Output {
        self.scratch
            .command(&sibling("maosctl"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .args(args)
            .output()
            .expect("maosctl")
    }

    fn field(&self, key: &str) -> Option<String> {
        let output = self.ctl(&["spirit", "inspect", ID]);
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::to_owned)
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.scratch.0.join("shell.stderr")).unwrap_or_default()
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.stdin.take();
        if let Some(mut child) = self.child.take() {
            let until = Instant::now() + Duration::from_secs(10);
            while Instant::now() < until && matches!(child.try_wait(), Ok(None)) {
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// An installed component (`maos spirit install`, then the shell root's boot) at `1.0.0`
/// whose manifest appends `extra` (further sections); returns the running shell and the
/// package directory a successor manifest can sit in.
fn boot_installed(extra: &str) -> (Shell, std::path::PathBuf) {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let package = scratch.0.join("package");
    std::fs::create_dir_all(&package).unwrap();
    let probe = world::fixtures::fixture("lifecycle_probe.wasm");
    std::fs::copy(&probe, package.join("lifecycle_probe.wasm")).unwrap();
    // The registry package: the published manifest names the component under `dist/`.
    let published = package.join("manifest.toml");
    std::fs::write(
        &published,
        manifest(ID, "1.0.0", "dist/lifecycle_probe.wasm", &ALL_HOOKS, extra).replace(
            "trust_tier = \"local\"",
            "trust_tier = \"public-untrusted\"",
        ),
    )
    .unwrap();
    let key = scratch.0.join("publisher.key");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&key, "53".repeat(32)).unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let output = publish_artifact(
        &scratch,
        &sibling("maos-spirit"),
        &server.addr,
        &published,
        &probe,
        &key,
    );
    assert!(output.status.success(), "publish: {}", text(&output));
    let install = scratch
        .command(&sibling("maosctl"))
        .args([
            "spirit",
            "install",
            &format!("{ID}@1.0.0"),
            "--registry-uri",
            &format!("http://{}/mcp", server.addr),
        ])
        .output()
        .unwrap();
    assert!(install.status.success(), "install: {}", text(&install));
    let maos = Path::new(env!("CARGO_BIN_EXE_maos"));
    let _ = world::fixtures::fixture("maos-wasm-runner");
    let init = scratch
        .command(maos)
        .args(["init", "--plain"])
        .output()
        .unwrap();
    assert!(init.status.success(), "init: {}", text(&init));

    let mut child = scratch
        .command(maos)
        .args(["shell", "--plain"])
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REGISTRY_URI", "")
        .env("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "300")
        .env("MAOS_YANK_POLICY", "warn")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json"),
        )
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(scratch.0.join("shell.stderr")).unwrap())
        .spawn()
        .expect("maos shell");
    let stdin = child.stdin.take();
    let shell = Shell {
        scratch,
        child: Some(child),
        stdin,
    };
    let until = Instant::now() + Duration::from_secs(90);
    while shell.field("lifecycle_state").as_deref() != Some("Running") {
        assert!(
            Instant::now() < until,
            "the installed component never reached Running:\n{}",
            shell.stderr()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    (shell, package)
}

#[test]
fn an_installed_component_is_upgraded_in_place_through_the_door_without_maos_run() {
    let (shell, package) = boot_installed("");
    let pid = shell.field("pid").expect("the door reports its pid");

    // The successor's manifest sits beside its artifact; the installed predecessor is a
    // component the root booted, not one `maos run` started.
    let v2 = package.join("v2.toml");
    std::fs::write(
        &v2,
        manifest(ID, "1.1.0", "lifecycle_probe.wasm", &ALL_HOOKS, ""),
    )
    .unwrap();
    let output = shell.ctl(&[
        "spirit",
        "upgrade",
        ID,
        "--to",
        v2.to_str().unwrap(),
        "--policy",
        "hot-swap",
    ]);
    assert!(
        output.status.success(),
        "the door upgrades the installed component: {}\nshell: {}",
        text(&output),
        shell.stderr()
    );
    assert_eq!(shell.field("pid"), Some(pid), "the same pid");
    assert_eq!(shell.field("lifecycle_state").as_deref(), Some("Running"));
}

/// Story 20-8 review correction (GPT P2-1) — the dry run compares a successor against the
/// predecessor's LIVE swap-invariant declaration, as the real swap does. An installed
/// component's boot records no retained manifest text (a cold swap refuses it as
/// `cold_swap_predecessor_unknown`), so a retained-text comparison had nothing to compare
/// and called a dropping successor safe that the coordinator then refuses.
#[test]
fn an_installed_components_precheck_agrees_with_its_swap_on_a_dropped_swap_invariant() {
    let (shell, package) =
        boot_installed("[swap_invariants]\npreserve = [\"open_pr_state\", \"review_queue\"]\n");
    let pid = shell.field("pid").expect("the door reports its pid");
    let successor = |name: &str, preserve: &str| {
        let path = package.join(name);
        let section = format!("[swap_invariants]\npreserve = [{preserve}]\n");
        let text = manifest(ID, "1.1.0", "lifecycle_probe.wasm", &ALL_HOOKS, &section);
        std::fs::write(&path, text).unwrap();
        path
    };
    let swap = |path: &Path, policy: &str| {
        shell.ctl(&[
            "spirit",
            "upgrade",
            ID,
            "--to",
            path.to_str().unwrap(),
            "--policy",
            policy,
        ])
    };
    let precheck = |path: &Path| {
        let output = shell.ctl(&[
            "spirit",
            "hot-swap-precheck",
            ID,
            "--to",
            path.to_str().unwrap(),
        ]);
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("a precheck body ({error}): {}", text(&output)));
        (output.status.code(), report)
    };

    let dropping = successor("v2-drops.toml", "\"open_pr_state\"");
    // The premise: this root retained no manifest text for its installed component.
    let cold = swap(&dropping, "cold-swap");
    assert!(
        text(&cold).contains("cold_swap_predecessor_unknown"),
        "no retained text for an installed component: {}",
        text(&cold)
    );
    let (exit, report) = precheck(&dropping);
    assert_eq!(exit, Some(2), "an unsafe verdict: {report}");
    assert_eq!(report["safe"], false, "{report}");
    assert_eq!(
        report["verdict"]["verdict"], "SwapInvariantDropped",
        "{report}"
    );
    assert_eq!(report["verdict"]["cause"], "review_queue", "{report}");
    let refused = swap(&dropping, "hot-swap");
    assert!(
        !refused.status.success()
            && text(&refused).contains("swap_invariant_dropped")
            && text(&refused).contains("review_queue"),
        "the swap refuses what the precheck reported: {}\nshell: {}",
        text(&refused),
        shell.stderr()
    );
    assert_eq!(
        shell.field("pid"),
        Some(pid.clone()),
        "the predecessor is untouched"
    );

    let superset = successor("v2-superset.toml", "\"review_queue\", \"open_pr_state\"");
    let (exit, report) = precheck(&superset);
    assert_eq!(exit, Some(0), "a superset prechecks safe: {report}");
    assert_eq!(report["safe"], true, "{report}");
    let swapped = swap(&superset, "hot-swap");
    assert!(
        swapped.status.success(),
        "the swap takes what the precheck passed: {}\nshell: {}",
        text(&swapped),
        shell.stderr()
    );
    assert_eq!(shell.field("pid"), Some(pid), "the same pid");
}
