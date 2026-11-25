//! Story 22-7 AC1 (witness b) — the SHIPPED domain Spirit answers `maosctl ask` in a real `maos serve`
//! Host. The test publishes `examples/incident-triage-ts/manifest.toml` and the component built from
//! that source (`MAOS_DOMAIN_COMPONENT`), installs it, boots the Host from the kit's own
//! `host.toml.example`, and asks four texts.
//!
//! `#[ignore]`d: it needs the built component and a writable delegated cgroup v2, because the shipped
//! manifest keeps the template's caps (`cpu_max_pct`, `memory_max_mb`). CI runs it in the node job under
//! `systemd-run --user --scope -p Delegate=yes` (`discipline.yml`, `domain-spirit-22-7`).
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use support::{publish_artifact, root, sibling, InProcessServer, Scratch};

const ID: &str = "incident-triage-ts";
const WAIT: Duration = Duration::from_secs(240);
/// The first ask of a componentized-JS guest took over 120 s on a debug-profile runner (Story 22-7 local run:
/// `--wait` at the door's default 120 s returned `running`), so the default wait is not enough here.
const ASK_WAIT_SECS: u64 = 300;

fn text(output: &Output) -> String {
    format!(
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

struct Host {
    scratch: Scratch,
    _server: InProcessServer,
    child: Option<Child>,
}

impl Host {
    fn command(&self, program: &Path) -> Command {
        let mut command = self.scratch.command(program);
        command
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("MAOS_REGISTRY_URI", "")
            .env("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "300")
            .env_remove("MAOS_INFERENCE_MODE")
            .env_remove("MAOS_REPLAY_CASSETTE")
            .current_dir(root());
        command
    }

    fn ctl(&self, args: &[&str]) -> Output {
        self.command(&sibling("maosctl"))
            .args(args)
            .output()
            .unwrap()
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(self.scratch.0.join("serve.stderr")).unwrap_or_default()
    }

    /// Publish and install the shipped package, then boot a Host whose cast is the kit's example file.
    fn boot(component: &Path) -> Self {
        let scratch = Scratch::new();
        let server = InProcessServer::start(&scratch);
        let key: PathBuf = scratch.0.join("publisher.key");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&key, "53".repeat(32)).unwrap();
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        let manifest = root().join("examples/incident-triage-ts/manifest.toml");
        let published = publish_artifact(
            &scratch,
            &sibling("maos-spirit"),
            &server.addr,
            &manifest,
            component,
            &key,
        );
        assert!(published.status.success(), "publish: {}", text(&published));
        let install = scratch
            .command(&sibling("maosctl"))
            .args([
                "spirit",
                "install",
                &format!("{ID}@0.1.0"),
                "--registry-uri",
            ])
            .arg(format!("http://{}/mcp", server.addr))
            .output()
            .unwrap();
        assert!(install.status.success(), "install: {}", text(&install));
        let _ = sibling("maos-wasm-runner");
        let mut host = Self {
            scratch,
            _server: server,
            child: None,
        };
        let init = host
            .command(Path::new(env!("CARGO_BIN_EXE_maos")))
            .args(["init", "--plain"])
            .output()
            .unwrap();
        assert!(init.status.success(), "init: {}", text(&init));
        let config = host.scratch.0.join("host.toml");
        let example =
            std::fs::read_to_string(root().join("kit/spirit-author/host.toml.example")).unwrap();
        assert!(
            example.contains("NAME"),
            "the kit's host file names its Spirit NAME"
        );
        std::fs::write(&config, example.replace("NAME", ID)).unwrap();
        let log = std::fs::File::create(host.scratch.0.join("serve.stderr")).unwrap();
        let child = host
            .command(Path::new(env!("CARGO_BIN_EXE_maos")))
            .args(["serve", "--config"])
            .arg(&config)
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .expect("spawn maos serve");
        host.child = Some(child);
        host.await_running();
        host
    }

    fn await_running(&mut self) {
        let until = Instant::now() + WAIT;
        loop {
            let out = self.ctl(&["agents"]);
            // Story 22-5: `id pid state ...`, tab-separated; read id and state only.
            let running = String::from_utf8_lossy(&out.stdout).lines().any(|line| {
                let mut fields = line.split('\t');
                fields.next() == Some(ID) && fields.nth(1) == Some("Running")
            });
            if out.status.success() && running {
                return;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
            {
                panic!("maos serve exited early: {status}\n{}", self.stderr());
            }
            assert!(
                Instant::now() < until,
                "{ID} never reached Running (a capped component needs a delegated cgroup)\n{}",
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// `maosctl ask --wait`: exit 0 and the completed task object.
    fn ask(&self, request: &str) -> serde_json::Value {
        let wait = format!("--wait={ASK_WAIT_SECS}");
        let out = self.ctl(&["ask", ID, request, &wait]);
        assert!(
            out.status.success(),
            "ask {request:?}: {}\n{}",
            text(&out),
            self.stderr()
        );
        let task: serde_json::Value = serde_json::from_slice(&out.stdout)
            .unwrap_or_else(|e| panic!("ask printed no JSON task: {e}\n{}", text(&out)));
        assert_eq!(task["state"], "completed", "{task}");
        assert_eq!(task["spirit_id"], ID);
        task
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("--- maos serve stderr ---\n{}", self.stderr());
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
#[ignore = "needs MAOS_DOMAIN_COMPONENT=<built dist/spirit.wasm> and a delegated cgroup v2"]
fn the_shipped_domain_spirit_answers_maosctl_ask() {
    let component = PathBuf::from(std::env::var_os("MAOS_DOMAIN_COMPONENT").expect(
        "MAOS_DOMAIN_COMPONENT must name the built examples/incident-triage-ts/dist/spirit.wasm",
    ));
    assert!(component.is_file(), "{} is not a file", component.display());
    let host = Host::boot(&component);
    for (request, expected) in [
        (
            "Customers report data loss after the migration",
            "sev1 | rule: data-loss | first step: Freeze writes and page the data on-call.",
        ),
        (
            "Checkout is slow, p99 latency doubled",
            "sev2 | rule: degraded-latency | first step: Check the saturation dashboard of the affected service.",
        ),
        (
            "There is a typo on the pricing page",
            "sev3 | rule: minor-defect | first step: File a ticket in the owning team's backlog.",
        ),
        (
            "The cafeteria ran out of oat milk",
            "sev4 | rule: none | first step: no rule matched",
        ),
    ] {
        let task = host.ask(request);
        assert_eq!(task["result"], expected, "{request:?}: {task}");
        assert!(!task["result"].as_str().unwrap().starts_with("echo:"));
    }
}
