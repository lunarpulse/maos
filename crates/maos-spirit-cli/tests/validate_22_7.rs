//! Story 22-7 AC2 — `maos-spirit validate` reports what the Host would refuse, offline, with an
//! ephemeral signing key. The in-process cases run `validate_bytes`; the exit-code and JSON cases
//! run the real binary under a throwaway `HOME`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use maos_spirit_cli::validate::validate_bytes;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(repo().join("tests/fixtures/wasm").join(name)).unwrap()
}

/// `templates/spirit-ts/manifest.toml` with its cargo-generate placeholder filled in.
fn template_manifest() -> String {
    std::fs::read_to_string(repo().join("templates/spirit-ts/manifest.toml"))
        .unwrap()
        .replace("{{project-name}}", "domain-spirit")
}

fn component() -> Vec<u8> {
    fixture("echo_spirit_component.wasm")
}

fn codes(manifest: &str, artifact: Vec<u8>) -> Vec<(String, String)> {
    let report = validate_bytes(manifest.as_bytes().to_vec(), artifact, None).unwrap();
    report
        .findings
        .into_iter()
        .map(|f| (f.check.to_owned(), f.code.to_owned()))
        .collect()
}

fn has(findings: &[(String, String)], check: &str, code: &str) -> bool {
    findings.iter().any(|(c, k)| c == check && k == code)
}

#[test]
fn validate_accepts_the_template_manifest_with_a_component() {
    let report = validate_bytes(template_manifest().into_bytes(), component(), None).unwrap();
    assert!(report.admissible(), "{:?}", report.findings);
    assert_eq!(report.spirit_id, "domain-spirit");
    assert_eq!(report.version, "0.1.0");
    assert_eq!(report.form, "wasm-component");
    assert_eq!(report.effective_tier, Some("public_untrusted"));
    assert_eq!(report.sandbox_tier_floor, Some("T2"));
}

#[test]
fn validate_refuses_rust_inproc_from_a_registry() {
    let manifest = std::fs::read_to_string(repo().join("templates/spirit-rust/manifest.toml"))
        .unwrap()
        .replace("{{crate_name}}", "my-rust-spirit");
    let found = codes(&manifest, component());
    assert!(has(&found, "manifest", "unknown_spirit_class"), "{found:?}");
    assert!(
        has(&found, "admission", "first_party_form_from_registry"),
        "{found:?}"
    );
}

#[test]
fn validate_refuses_a_component_below_t2() {
    let manifest = template_manifest().replace("tier = \"T2\"", "tier = \"T1\"");
    let found = codes(&manifest, component());
    assert!(
        has(&found, "manifest", "wasm_component_requires_t2"),
        "{found:?}"
    );
}

#[test]
fn validate_refuses_a_core_module_as_artifact() {
    let found = codes(&template_manifest(), fixture("benign.wasm"));
    assert_eq!(
        found,
        vec![(
            "component".to_owned(),
            "component_not_a_component".to_owned()
        )]
    );
}

#[test]
fn validate_refuses_a_missing_required_section() {
    let without_posture = template_manifest().replace(
        "[posture]\ndefault = \"assistive\"\nallowed_max = \"assistive\"\n",
        "",
    );
    assert_ne!(
        without_posture,
        template_manifest(),
        "the fixture still carries [posture]"
    );
    let found = codes(&without_posture, component());
    assert!(
        has(&found, "manifest", "manifest_section_invalid"),
        "{found:?}"
    );
}

#[test]
fn validate_refuses_a_component_that_lacks_the_world_exports() {
    // A minimal empty component: valid encoding, none of the three exports.
    let empty = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];
    let found = codes(&template_manifest(), empty.to_vec());
    assert_eq!(
        found,
        vec![(
            "component".to_owned(),
            "component_missing_export".to_owned()
        )]
    );
}

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("validate-22-7-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("home")).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(scratch: &Scratch, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_maos-spirit"))
        .arg("validate")
        .args(args)
        .env("HOME", scratch.0.join("home"))
        .env_remove("MAOS_REGISTRY_TIER_FLOOR")
        .env_remove("MAOS_REGISTRY_ORG_SIGNING_PUBKEY")
        .env_remove("MAOS_REGISTRY_REQUIRE_SERVER_TIER_SIGNATURE")
        .output()
        .unwrap()
}

fn write_inputs(scratch: &Scratch, manifest: &str, artifact: &[u8]) -> (PathBuf, PathBuf) {
    let (m, a) = (
        scratch.0.join("manifest.toml"),
        scratch.0.join("spirit.wasm"),
    );
    std::fs::write(&m, manifest).unwrap();
    std::fs::write(&a, artifact).unwrap();
    (m, a)
}

#[test]
fn validate_exit_2_on_a_missing_file() {
    let scratch = Scratch::new("missing");
    let out = run(
        &scratch,
        &[
            "--manifest",
            "/nonexistent/manifest.toml",
            "--artifact",
            "/nonexistent/spirit.wasm",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn validate_exit_codes_follow_the_verdict() {
    let scratch = Scratch::new("verdict");
    let (m, a) = write_inputs(&scratch, &template_manifest(), &component());
    let ok = run(
        &scratch,
        &[
            "--manifest",
            m.to_str().unwrap(),
            "--artifact",
            a.to_str().unwrap(),
        ],
    );
    assert_eq!(
        ok.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&ok.stdout)
    );
    assert!(String::from_utf8_lossy(&ok.stdout).contains("admissible"));

    let (m, a) = write_inputs(&scratch, &template_manifest(), &fixture("benign.wasm"));
    let bad = run(
        &scratch,
        &[
            "--manifest",
            m.to_str().unwrap(),
            "--artifact",
            a.to_str().unwrap(),
        ],
    );
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stdout).contains("component_not_a_component"));
}

#[test]
fn validate_json_report_is_stable() {
    let scratch = Scratch::new("json");
    let (m, a) = write_inputs(&scratch, &template_manifest(), &fixture("benign.wasm"));
    let out = run(
        &scratch,
        &[
            "--manifest",
            m.to_str().unwrap(),
            "--artifact",
            a.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    // Parsed value, never key order: a workspace build enables serde_json `preserve_order`.
    let got: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let want = serde_json::json!({
        "spirit_id": "domain-spirit",
        "version": "0.1.0",
        "form": "wasm-component",
        "effective_tier": "public_untrusted",
        "sandbox_tier_floor": "T2",
        "findings": [{
            "check": "component",
            "code": "component_not_a_component",
            "detail": got["findings"][0]["detail"].clone(),
        }],
    });
    assert_eq!(got, want);
    assert!(got["findings"][0]["detail"]
        .as_str()
        .unwrap()
        .contains("core WebAssembly module"));
}
