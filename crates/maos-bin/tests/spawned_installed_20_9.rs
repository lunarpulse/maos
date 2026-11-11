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

#[test]
fn an_installed_component_is_upgraded_in_place_through_the_door_without_maos_run() {
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
        manifest(ID, "1.0.0", "dist/lifecycle_probe.wasm", &ALL_HOOKS, "").replace(
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
