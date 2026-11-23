//! Story 22-1 — the Host configuration file and the standing cast.
//!
//! AC1 boots a real `maos serve` root and reads the cast back through `maosctl agents`
//! after a SIGTERM drain and after a SIGKILL. AC2 holds the key table to the env
//! registry. AC3 refuses every invalid file with a stable kind and a non-zero exit, with
//! no cast serving.
#![cfg(feature = "network")]
#![forbid(unsafe_code)]

use maos_bin::host_config::{
    parse_serve_args, CastSpirit, HostConfig, HostConfigKind, HOST_SETTINGS,
};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const REVIEWER_MANIFEST: &str = "spirits/reviewer/manifest.toml";
/// Topology members are deterministic class Spirits (nash, observer), not butler.
const NASH_MANIFEST: &str = "spirits/nash/manifest.toml";
const OBSERVER_MANIFEST: &str = "spirits/observer/manifest.toml";
const BUTLER_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
const READY_TIMEOUT: Duration = Duration::from_secs(60);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

fn maosctl() -> PathBuf {
    let path = maos().with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli --bin maosctl` first",
        path.display()
    );
    path
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "maos-host-22-1-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create scratch");
    dir
}

fn manifest(rel: &str) -> String {
    workspace_root().join(rel).to_string_lossy().into_owned()
}

fn write_config(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("host.toml");
    std::fs::write(&path, body).expect("write host.toml");
    path
}

fn kind_of(body: &str) -> HostConfigKind {
    let dir = scratch("kind");
    let path = write_config(&dir, body);
    let error = HostConfig::load(&path).expect_err("the file must be refused");
    let _ = std::fs::remove_dir_all(&dir);
    error.kind
}

// ── AC3: invalid files are typed refusals ───────────────────────────────────

#[test]
fn every_invalid_file_is_a_typed_refusal() {
    let butler = manifest(BUTLER_MANIFEST);
    let dir = scratch("ac3");
    let topology = dir.join("topology.toml");
    std::fs::write(
        &topology,
        "[topology]\n[[topology.spirits]]\nmanifest = \"x\"\n",
    )
    .unwrap();
    let no_section = dir.join("empty.toml");
    std::fs::write(&no_section, "name = \"x\"\n").unwrap();

    let missing = HostConfig::load(&dir.join("absent.toml")).expect_err("missing file");
    assert_eq!(missing.kind, HostConfigKind::Missing);
    assert_eq!(missing.kind.as_str(), "host_config_missing");

    let cases: Vec<(&str, String, HostConfigKind)> = vec![
        ("toml parse error", "[host\n".into(), HostConfigKind::Parse),
        (
            "unknown top-level key",
            "[host]\n[other]\n".into(),
            HostConfigKind::Parse,
        ),
        (
            "unknown host key",
            "[host]\nport = 1\n".into(),
            HostConfigKind::Parse,
        ),
        (
            "unknown cast key",
            format!("[[host.spirits]]\nmanifest = '{butler}'\nweight = 3\n"),
            HostConfigKind::Parse,
        ),
        (
            "a host key in a cast entry",
            format!("[[host.spirits]]\nmanifest = '{butler}'\nhost = 'b'\n"),
            HostConfigKind::CastHostKey,
        ),
        (
            "manifest and installed together",
            format!("[[host.spirits]]\nmanifest = '{butler}'\ninstalled = 'x'\n"),
            HostConfigKind::CastEntry,
        ),
        (
            "an empty cast entry",
            "[[host.spirits]]\n".into(),
            HostConfigKind::CastEntry,
        ),
        (
            "an unreadable manifest",
            "[[host.spirits]]\nmanifest = '/nonexistent/manifest.toml'\n".into(),
            HostConfigKind::CastEntry,
        ),
        (
            "a topology manifest as a member",
            format!("[[host.spirits]]\nmanifest = '{}'\n", topology.display()),
            HostConfigKind::CastEntry,
        ),
        (
            "a manifest with no class",
            format!("[[host.spirits]]\nmanifest = '{}'\n", no_section.display()),
            HostConfigKind::CastEntry,
        ),
        (
            "a relative home",
            "[host]\nhome = 'relative/home'\n".into(),
            HostConfigKind::Setting,
        ),
        (
            "a non-loopback door",
            "[host]\ndoor_bind = '0.0.0.0:7600'\n".into(),
            HostConfigKind::Setting,
        ),
        (
            "an invalid region",
            "[host]\nregion = 'EU WEST!'\n".into(),
            HostConfigKind::Setting,
        ),
        (
            "an unknown crypto provider",
            "[host]\ncrypto_provider = 'rot13'\n".into(),
            HostConfigKind::Setting,
        ),
        (
            "an unknown inference mode",
            "[host]\ninference_mode = 'guess'\n".into(),
            HostConfigKind::Setting,
        ),
    ];
    for (label, body, expected) in cases {
        let body = if body.starts_with("[[host.spirits]]") {
            // Entries live under `[host]`; the bare array form keeps each case one line.
            format!("[host]\n{body}")
        } else {
            body
        };
        assert_eq!(kind_of(&body), expected, "{label}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_valid_file_loads_its_cast_in_order() {
    let dir = scratch("valid");
    let body = format!(
        "[host]\nregion = 'eu-west'\ninference_mode = 'replay'\n\n\
         [[host.spirits]]\nmanifest = '{}'\n\n[[host.spirits]]\npath = '{}'\n\n\
         [[host.spirits]]\ninstalled = 'researcher'\n",
        manifest(BUTLER_MANIFEST),
        manifest(REVIEWER_MANIFEST)
    );
    let config = HostConfig::load(&write_config(&dir, &body)).expect("a valid file loads");
    let cast = config.cast();
    assert_eq!(cast.len(), 3);
    assert!(matches!(&cast[0], CastSpirit::Manifest(p) if p.ends_with("butler/manifest.toml")));
    assert!(matches!(&cast[1], CastSpirit::Manifest(p) if p.ends_with("reviewer/manifest.toml")));
    assert_eq!(cast[2], CastSpirit::Installed("researcher".into()));
    assert_eq!(
        config.manifest_entries().len(),
        2,
        "only manifest members are topology entries"
    );
    assert!(config.manifest_entries().iter().all(|e| e.host.is_none()));
    let topology = config.topology_toml();
    assert!(topology.contains("[[topology.spirits]]") && topology.contains("butler"));

    // Installed members come from the registry inventory; a missing name is a typed refusal.
    let inventory = vec![(
        "researcher".to_string(),
        PathBuf::from("/inventory/researcher"),
    )];
    assert_eq!(config.select_installed(inventory).unwrap().len(), 1);
    let error = config
        .select_installed(Vec::new())
        .expect_err("not installed");
    assert_eq!(error.kind, HostConfigKind::InstalledMissing);
    // A cast member boot refused refuses the whole Host.
    let refused = config.installed_refusal(&|name| (name == "researcher").then_some("signature"));
    assert_eq!(refused.unwrap_err().kind, HostConfigKind::InstalledRefused);
    assert!(config.installed_refusal(&|_| None).is_ok());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn serve_args_take_exactly_one_config() {
    let args = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        parse_serve_args(&args(&["--config", "h.toml"])).unwrap(),
        PathBuf::from("h.toml")
    );
    for bad in [
        &[][..],
        &["--config"],
        &["h.toml"],
        &["--config", "--x"],
        &["--config", "a", "b"],
    ] {
        assert!(parse_serve_args(&args(bad)).is_err(), "{bad:?}");
    }
}

// ── AC2: one documented key, one registered name ─────────────────────────────

#[test]
fn every_key_maps_to_one_registered_env_name_and_one_documented_key() {
    let registry =
        std::fs::read_to_string(workspace_root().join("crates/maos-bin/src/env_contract.rs"))
            .expect("env_contract.rs");
    let docs = std::fs::read_to_string(workspace_root().join("docs/maos.dev/run-maos.md"))
        .expect("run-maos.md");
    let mut seen_keys = std::collections::BTreeSet::new();
    let mut seen_envs = std::collections::BTreeSet::new();
    for (key, env) in HOST_SETTINGS {
        assert!(
            seen_keys.insert(*key) && seen_envs.insert(*env),
            "one key per name: {key}"
        );
        assert!(
            registry.contains(&format!("name: \"{env}\"")),
            "{key} maps to {env}, which env_contract.rs does not register"
        );
        assert!(
            docs.contains(&format!("`{key}`")) && docs.contains(&format!("`{env}`")),
            "run-maos.md must document the key `{key}` beside `{env}`"
        );
    }
}

#[test]
fn the_environment_overrides_a_key_and_never_stands_in_for_the_file() {
    let dir = scratch("env");
    let path = write_config(
        &dir,
        "[host]\nhome = '/srv/maos'\nregion = 'eu-west'\ninference_mode = 'replay'\n",
    );
    let config = HostConfig::load(&path).unwrap();
    let overlay = config.env_overlay(&|name| name == "MAOS_REGION_HOME");
    let names: Vec<&str> = overlay.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names,
        ["MAOS_HOME", "MAOS_INFERENCE_MODE"],
        "a set name is never overwritten"
    );
    assert!(config.env_overlay(&|_| true).is_empty());

    // The environment cannot rescue a missing or invalid file.
    let bad = write_config(&dir, "[host]\nregion = 'EU WEST!'\n");
    let out = Command::new(maos())
        .args(["serve", "--config"])
        .arg(&bad)
        .env("HOME", &dir)
        .env("MAOS_HOME", dir.join("home"))
        .env("MAOS_REGION_HOME", "eu-west")
        .output()
        .expect("run maos serve");
    assert_eq!(out.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&out.stderr).contains("host_config_setting"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_older_file_that_disagrees_refuses_the_boot() {
    let dir = scratch("older");
    let path = write_config(
        &dir,
        "[host]\nregion = 'eu-west'\ncrypto_provider = 'ring'\n",
    );
    let config = HostConfig::load(&path).unwrap();
    let env = |_: &str| None;
    let operator = |tag: &str| format!("[region]\nhome_region = \"{tag}\"\n");
    assert!(config
        .reconcile_older_files(&env, Some(&operator("EU-West")), None)
        .is_ok());
    assert!(config.reconcile_older_files(&env, None, None).is_ok());
    let error = config
        .reconcile_older_files(&env, Some(&operator("us-east")), None)
        .expect_err("operator.toml disagrees");
    assert_eq!(error.kind, HostConfigKind::OlderFile);
    let recorded = "[crypto]\nprovider = \"other\"\n";
    let error = config
        .reconcile_older_files(&env, None, Some(recorded))
        .expect_err("config.toml records another provider");
    assert_eq!(error.kind, HostConfigKind::OlderFile);

    // The same through the binary: the boot is refused before anything serves.
    let home = dir.join("home");
    std::fs::create_dir_all(dir.join(".config/maos")).unwrap();
    std::fs::write(dir.join(".config/maos/operator.toml"), operator("us-east")).unwrap();
    let out = Command::new(maos())
        .args(["serve", "--config"])
        .arg(&path)
        .env("HOME", &dir)
        .env("MAOS_HOME", &home)
        .env_remove("MAOS_REGION_HOME")
        .output()
        .expect("run maos serve");
    assert_eq!(out.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&out.stderr).contains("host_config_older_file_disagrees"));
    let _ = std::fs::remove_dir_all(&dir);
}

// ── AC3 through the binary: exit code, stable kind, nothing serving ───────────

#[test]
fn the_binary_refuses_bad_files_with_a_kind_and_serves_nothing() {
    let dir = scratch("bin");
    let butler = manifest(BUTLER_MANIFEST);
    let cases = [
        ("host_config_missing", None),
        ("host_config_parse", Some("[host\n".to_string())),
        (
            "host_config_cast_host_key",
            Some(format!("[host]\n[[host.spirits]]\nmanifest = '{butler}'\nhost = 'b'\n")),
        ),
        // A valid first member followed by a failing one: no partial cast is left serving.
        (
            "host_config_cast_entry",
            Some(format!(
                "[host]\n[[host.spirits]]\nmanifest = '{butler}'\n[[host.spirits]]\nmanifest = '/nonexistent.toml'\n"
            )),
        ),
        (
            "host_config_installed_missing",
            Some("[host]\n[[host.spirits]]\ninstalled = 'not-installed'\n".to_string()),
        ),
    ];
    for (kind, body) in cases {
        let home = dir.join(format!("home-{kind}"));
        std::fs::create_dir_all(&home).unwrap();
        let path = match body {
            Some(body) => write_config(&dir, &body),
            None => dir.join("absent.toml"),
        };
        let mut child = Command::new(maos())
            .args(["serve", "--config"])
            .arg(&path)
            .env("HOME", &dir)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", dir.join("xdg"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos serve");
        let deadline = Instant::now() + Duration::from_secs(30);
        let status = loop {
            if let Some(status) = child.try_wait().expect("poll") {
                break status;
            }
            assert!(
                Instant::now() < deadline,
                "{kind}: the refusal must be prompt, not a serve"
            );
            std::thread::sleep(Duration::from_millis(20));
        };
        let mut stderr = String::new();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert_eq!(status.code(), Some(78), "{kind}: {stderr}");
        assert!(
            stderr.contains(&format!("maos serve: {kind}")),
            "{kind}: {stderr}"
        );
        assert!(
            !stderr.contains("continuous serving loop"),
            "{kind}: a cast must not serve"
        );
        assert!(
            !home.join("control.json").exists(),
            "{kind}: no door may be published for a refused file"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ── AC1: the same cast after SIGTERM and after SIGKILL ──────────────────────

struct Host {
    dir: PathBuf,
    home: PathBuf,
    config: PathBuf,
    child: Option<Child>,
}

impl Host {
    fn new() -> Self {
        let dir = scratch("serve");
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let init = Command::new(maos())
            .arg("init")
            .env("HOME", &dir)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", dir.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maos init");
        assert!(
            init.status.success(),
            "{}",
            String::from_utf8_lossy(&init.stderr)
        );
        let config = write_config(
            &dir,
            &format!(
                "[host]\ninference_mode = 'replay'\n\n[[host.spirits]]\nmanifest = '{}'\n\n\
                 [[host.spirits]]\nmanifest = '{}'\n",
                manifest(NASH_MANIFEST),
                manifest(OBSERVER_MANIFEST)
            ),
        );
        Self {
            dir,
            home,
            config,
            child: None,
        }
    }

    fn command(&self, program: PathBuf) -> Command {
        let mut command = Command::new(program);
        command
            .env("HOME", &self.dir)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.dir.join("xdg"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_INFERENCE_MODE")
            .current_dir(workspace_root());
        command
    }

    fn boot(&mut self) {
        let child = self
            .command(maos())
            .args(["serve", "--config"])
            .arg(&self.config)
            .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn maos serve");
        self.child = Some(child);
    }

    /// `maosctl agents` lines, sorted: the operator's read-back of the cast.
    fn agents(&self) -> Vec<String> {
        let out = self
            .command(maosctl())
            .arg("agents")
            .output()
            .expect("run maosctl");
        if !out.status.success() {
            return Vec::new();
        }
        // Story 22-5 widened the line to `id pid state form originator task age spend`; the
        // 22-1 read-back is the id and the state (the pid and age change across a restart).
        let mut lines: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|line| {
                let mut fields = line.split('\t');
                format!(
                    "{}\t{}",
                    fields.next().unwrap_or_default(),
                    fields.nth(1).unwrap_or_default()
                )
            })
            .collect();
        lines.sort();
        lines
    }

    fn await_cast(&mut self) -> Vec<String> {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            let agents = self.agents();
            if agents.len() == 2 && agents.iter().all(|line| line.ends_with("\tRunning")) {
                return agents;
            }
            if let Some(status) = self
                .child
                .as_mut()
                .and_then(|c| c.try_wait().ok().flatten())
            {
                panic!("maos serve exited early: {status}");
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        panic!(
            "the cast never came up Running; last read-back: {:?}",
            self.agents()
        );
    }

    fn stop(&mut self, signal: &str) {
        let mut child = self.child.take().expect("a running host");
        Command::new("kill")
            .args([signal, &child.id().to_string()])
            .status()
            .unwrap();
        let _ = child.wait();
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn serve_brings_the_declared_cast_back_after_sigterm_and_after_sigkill() {
    let mut host = Host::new();
    host.boot();
    let first = host.await_cast();
    assert_eq!(
        first
            .iter()
            .map(|l| l.split('\t').next().unwrap())
            .collect::<Vec<_>>(),
        ["nash", "observer"],
        "exactly the file's cast: {first:?}"
    );

    host.stop("-TERM");
    assert!(host.agents().is_empty(), "a drained Host answers no agents");
    host.boot();
    assert_eq!(
        host.await_cast(),
        first,
        "the same cast after a SIGTERM drain"
    );

    host.stop("-KILL");
    host.boot();
    assert_eq!(host.await_cast(), first, "the same cast after a SIGKILL");
    host.stop("-TERM");
}
