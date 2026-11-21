//! A real `maos shell` root over registry-installed component Spirits (Story 20-9, shared with
//! Story 21-7): publish, `maosctl spirit install`, boot, wait for `Running`, drive through the
//! door. Include with `#[path = "support/installed_shell_20_9.rs"] mod shell;` next to the
//! registry harness mounted as `mod support`.
#![allow(dead_code)]

use crate::support::{publish_artifact, root, sibling, InProcessServer, Scratch};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Stdio};
use std::time::{Duration, Instant};

pub struct Shell {
    pub scratch: Scratch,
    child: Option<Child>,
    /// The shell reads its REPL from here and leaves on EOF: held open for the vector's life.
    pub stdin: Option<ChildStdin>,
}

impl Shell {
    pub fn ctl(&self, args: &[&str]) -> std::process::Output {
        self.scratch
            .command(&sibling("maosctl"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .args(args)
            .output()
            .expect("maosctl")
    }

    /// One `key: value` line of `maosctl spirit inspect <id>`.
    pub fn field(&self, id: &str, key: &str) -> Option<String> {
        let output = self.ctl(&["spirit", "inspect", id]);
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::to_owned)
    }

    pub fn stderr(&self) -> String {
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

pub fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Each `(id, manifest)` installed at `1.0.0` from the prebuilt `fixture` component (the
/// manifest names it `dist/<fixture>`), then the shell root booted over them and waited on until
/// every one is `Running`. Returns the shell and each package directory (manifest + artifact
/// copy, where a successor manifest can sit).
pub fn boot_installed(components: &[(&str, String)], fixture: &str) -> (Shell, Vec<PathBuf>) {
    let scratch = Scratch::new();
    let server = InProcessServer::start(&scratch);
    let artifact = sibling(fixture);
    let key = scratch.0.join("publisher.key");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&key, "53".repeat(32)).unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let mut packages = Vec::new();
    for (id, manifest) in components {
        // The registry package: the published manifest names the component under `dist/`.
        let package = scratch.0.join("package").join(id);
        std::fs::create_dir_all(&package).unwrap();
        std::fs::copy(&artifact, package.join(fixture)).unwrap();
        let published = package.join("manifest.toml");
        std::fs::write(
            &published,
            manifest.replace(
                "trust_tier = \"local\"",
                "trust_tier = \"public-untrusted\"",
            ),
        )
        .unwrap();
        let output = publish_artifact(
            &scratch,
            &sibling("maos-spirit"),
            &server.addr,
            &published,
            &artifact,
            &key,
        );
        assert!(output.status.success(), "publish {id}: {}", text(&output));
        let install = scratch
            .command(&sibling("maosctl"))
            .args([
                "spirit",
                "install",
                &format!("{id}@1.0.0"),
                "--registry-uri",
                &format!("http://{}/mcp", server.addr),
            ])
            .output()
            .unwrap();
        assert!(install.status.success(), "install {id}: {}", text(&install));
        packages.push(package);
    }
    let maos = Path::new(env!("CARGO_BIN_EXE_maos"));
    let _ = sibling("maos-wasm-runner");
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
        .env_remove("MAOS_LOOM_POSTGRES")
        .env_remove("MAOS_LOOM_HOME_TEAM")
        .env_remove("MAOS_CROSS_TEAM_SHARE_PEER")
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
    for (id, _) in components {
        while shell.field(id, "lifecycle_state").as_deref() != Some("Running") {
            assert!(
                Instant::now() < until,
                "the installed component {id} never reached Running:\n{}",
                shell.stderr()
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }
    (shell, packages)
}
