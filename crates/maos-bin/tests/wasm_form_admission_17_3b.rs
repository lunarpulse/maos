#![cfg(feature = "network")]
#![forbid(unsafe_code)]

use maos_bin::admission::gate_manifest;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;

const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const BUTLER_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";
const READY_TIMEOUT: Duration = Duration::from_secs(45);

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

fn wasm_manifest(tier: Option<&str>) -> String {
    let sandbox_tier = tier
        .map(|tier| format!("tier = {tier:?}\n"))
        .unwrap_or_default();
    format!(
        r#"[class]
name = "wasm-form-17-3b"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 5
min_substrate_version = "0.1.0"
forms = ["wasm-component"]
artifact = "dist/spirit.wasm"
trust_tier = "local"
description = "WASM component admission fixture"

[resources]
cpu_max_pct = 10
memory_max_mb = 64
fd_max = 16

[output_shape]
required_fields = ["result"]

[posture]
default = "assistive"
allowed_max = "assistive"

[sandbox]
{sandbox_tier}"#
    )
}

fn expected_engine_refusal() -> &'static str {
    if cfg!(feature = "wasm-host") {
        "wasm_launch_not_built"
    } else {
        "wasm_engine_off"
    }
}

fn assert_gate_code(manifest: &str, expected: &str) {
    let refusal = gate_manifest(manifest, &|_| true).expect_err("manifest must be refused");
    assert_eq!(
        refusal.code(),
        expected,
        "manifest must reach the typed refusal rather than a generic failure: {refusal}"
    );
}

#[test]
fn gate_manifest_enforces_wasm_component_form_and_tier_before_engine_refusal() {
    for tier in ["T0", "T1", "T3"] {
        assert_gate_code(&wasm_manifest(Some(tier)), "wasm_component_requires_t2");
    }
    assert_gate_code(&wasm_manifest(Some("T4")), "manifest_section_invalid");
    assert_gate_code(&wasm_manifest(None), expected_engine_refusal());

    let missing_output_shape =
        wasm_manifest(Some("T2")).replace("[output_shape]\nrequired_fields = [\"result\"]\n\n", "");
    assert_gate_code(&missing_output_shape, "manifest_section_invalid");
    let bad_posture =
        wasm_manifest(Some("T2")).replace("default = \"assistive\"", "default = \"not-a-posture\"");
    assert_gate_code(&bad_posture, "manifest_section_invalid");

    let mixed = wasm_manifest(Some("T2")).replace(
        r#"["wasm-component"]"#,
        r#"["wasm-component", "rust-inproc"]"#,
    );
    assert_gate_code(&mixed, "class_section_invalid");

    let missing_artifact =
        wasm_manifest(Some("T2")).replace("artifact = \"dist/spirit.wasm\"\n", "");
    assert_gate_code(&missing_artifact, "class_section_invalid");

    let bad_artifact = wasm_manifest(Some("T2")).replace(
        "artifact = \"dist/spirit.wasm\"",
        "artifact = \"../spirit.wasm\"",
    );
    assert_gate_code(&bad_artifact, "class_section_invalid");

    let schema_v4 = wasm_manifest(Some("T2"))
        .replace("manifest_schema_version = 5", "manifest_schema_version = 4");
    assert_gate_code(&schema_v4, "class_section_invalid");

    let rust_with_artifact =
        wasm_manifest(Some("T0")).replace(r#"["wasm-component"]"#, r#"["rust-inproc"]"#);
    assert_gate_code(&rust_with_artifact, "class_section_invalid");

    let v4_rust = wasm_manifest(Some("T0"))
        .replace(r#"["wasm-component"]"#, r#"["rust-inproc"]"#)
        .replace("artifact = \"dist/spirit.wasm\"\n", "")
        .replace("manifest_schema_version = 5", "manifest_schema_version = 4");
    assert!(
        gate_manifest(&v4_rust, &|_| true).is_ok(),
        "a schema-v4 rust-inproc manifest remains admitted"
    );
}

fn init_root(scratch: &TempDir) -> PathBuf {
    let home = scratch.path().join("maos-home");
    let output = Command::new(maos())
        .arg("init")
        .env("HOME", scratch.path().join("home"))
        .env("MAOS_HOME", &home)
        .env("XDG_DATA_HOME", scratch.path().join("xdg"))
        .current_dir(workspace_root())
        .output()
        .expect("run maos init");
    assert!(
        output.status.success(),
        "maos init must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    home
}

fn run_component_manifest(manifest: &Path, label: &str) {
    let scratch = TempDir::new().expect("create isolated standalone root");
    let home = init_root(&scratch);
    let output = Command::new(maos())
        .args(["run", &manifest.to_string_lossy(), "--once"])
        .env("HOME", scratch.path().join("home"))
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", scratch.path().join("xdg"))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .current_dir(workspace_root())
        .output()
        .expect("run maos component manifest");
    assert!(!output.status.success(), "{label} must be refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected_engine_refusal()),
        "{label} must expose {} on stderr, got: {stderr}",
        expected_engine_refusal()
    );
    assert!(
        !stderr.contains("unknown_spirit_class")
            && !stderr.contains("unknown Spirit class")
            && !stderr.contains("class_section_invalid"),
        "{label} must pass the form gate before the engine refusal, got: {stderr}"
    );
}

#[test]
fn maos_run_refuses_wasm_component_manifests_by_named_engine_state() {
    let fixture = TempDir::new().expect("create component fixture directory");
    let manifest = fixture.path().join("component.toml");
    std::fs::write(&manifest, wasm_manifest(Some("T2"))).expect("write component manifest");
    run_component_manifest(&manifest, "test-local schema-v5 manifest");

    run_component_manifest(
        &workspace_root().join("examples/example-spirit-ts/manifest.toml"),
        "TypeScript example manifest",
    );
}

struct DaemonRoot {
    scratch: TempDir,
    home: PathBuf,
    child: Option<Child>,
}

impl DaemonRoot {
    fn butler() -> Self {
        let scratch = TempDir::new().expect("create isolated daemon root");
        let home = init_root(&scratch);
        let child = Command::new(maos())
            .args(["run", BUTLER_MANIFEST])
            .env("HOME", scratch.path().join("home"))
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.path().join("xdg"))
            .env("MAOS_INFERENCE_MODE", "replay")
            .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
            .env("MAOS_NOTIFY_DISABLE", "1")
            .current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos run root");
        let root = Self {
            scratch,
            home,
            child: Some(child),
        };
        root.await_running();
        root
    }

    fn ctl(&self, args: &[&str]) -> std::process::Output {
        Command::new(maosctl())
            .args(args)
            .env("HOME", self.scratch.path().join("home"))
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.scratch.path().join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maosctl")
    }

    fn await_running(&self) {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            let output = self.ctl(&["spirit", "inspect", "butler"]);
            if output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("lifecycle_state: Running")
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("the daemon never reported butler Running within {READY_TIMEOUT:?}");
    }

    fn roster(&self) -> Vec<String> {
        let control = maos_domain::operator_door::ControlFile::load(
            &maos_domain::operator_door::control_file_path(&self.home),
        )
        .expect("control.json loads");
        let endpoint = control
            .endpoint
            .rsplit("://")
            .next()
            .expect("control endpoint carries a host:port");
        let mut stream = std::net::TcpStream::connect(endpoint).expect("connect to operator door");
        let request = format!(
            "GET /v1/daemon HTTP/1.1\r\nHost: {endpoint}\r\nAuthorization: Bearer {}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            control.token
        );
        stream
            .write_all(request.as_bytes())
            .expect("write daemon status request");
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .expect("read daemon status response");
        let text = String::from_utf8_lossy(&response);
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("a well-formed HTTP response");
        serde_json::from_str::<serde_json::Value>(body).expect("daemon status JSON")["spirit_ids"]
            .as_array()
            .expect("spirit_ids is an array")
            .iter()
            .map(|id| id.as_str().expect("spirit id is a string").to_owned())
            .collect()
    }
}

impl Drop for DaemonRoot {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn maosctl_load_returns_wasm_engine_refusal_and_preserves_roster() {
    let root = DaemonRoot::butler();
    let baseline = root.roster();
    assert_eq!(baseline, ["butler"]);

    let manifest = root.scratch.path().join("component.toml");
    std::fs::write(&manifest, wasm_manifest(Some("T2"))).expect("write door component manifest");
    let output = root.ctl(&["load", &manifest.to_string_lossy()]);
    assert!(
        !output.status.success(),
        "the door must refuse a component manifest"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("HTTP 400") && stderr.contains(expected_engine_refusal()),
        "the door must return HTTP 400 with {}, got: {stderr}",
        expected_engine_refusal()
    );
    assert!(
        !stderr.contains("unknown_spirit_class") && !stderr.contains("class_section_invalid"),
        "the door must reach the engine refusal, got: {stderr}"
    );
    assert_eq!(
        root.roster(),
        baseline,
        "a refused component manifest must leave the daemon roster unchanged"
    );
}

#[test]
fn maosctl_upgrade_refuses_wasm_successor_before_replacing_the_loaded_spirit() {
    let root = DaemonRoot::butler();
    let baseline = root.roster();
    let successor = std::fs::read_to_string(workspace_root().join(BUTLER_MANIFEST))
        .expect("read first-party predecessor manifest")
        .replace("version = \"0.3.0\"", "version = \"0.4.0\"")
        .replace("manifest_schema_version = 2", "manifest_schema_version = 5")
        .replace(
            "forms = [\"rust-inproc\"]",
            "forms = [\"wasm-component\"]\nartifact = \"dist/spirit.wasm\"",
        )
        .replace("tier = \"T0\"", "tier = \"T2\"");
    let successor = format!(
        "{successor}\n[scheduling]\npriority_weight = 100\n[lifecycle]\nenabled_hooks = []\n"
    );
    let path = root.scratch.path().join("successor.toml");
    std::fs::write(&path, successor).expect("write wasm successor manifest");

    let output = root.ctl(&[
        "spirit",
        "upgrade",
        "butler",
        "--to",
        path.to_str().expect("UTF-8 successor path"),
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success() && stderr.contains("wasm-component"),
        "a WASM successor must not enter the in-process hot-swap: {stderr}"
    );
    assert_eq!(
        root.roster(),
        baseline,
        "refused upgrade must preserve the roster"
    );
}
