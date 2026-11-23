//! Story 22-7 AC2 — `maos-spirit validate` must never call admissible what the Host refuses.
//!
//! `maos_registry::validate` re-states the boot gate's manifest checks (`gate_manifest` lives in
//! `maos-bin`, which depends on `maos-spirit-cli`, so the validator cannot call it). This test is
//! the guard against drift: every corpus manifest goes through `validate` and through the real
//! `gate_manifest` (with the class predicate `installed_spirits.rs` passes: no compiled-in class)
//! and `admit_spirit`, and the verdict and each refusal code must agree.
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use maos_bin::admission::gate_manifest;
use maos_domain::ports::registry::TrustTier;
use maos_registry::admission::{admit_spirit, AdmissionConfig};
use maos_spirit_cli::publish::{build_signed_package, PublishArgs};
use maos_spirit_cli::validate::validate_bytes;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn template() -> String {
    std::fs::read_to_string(root().join("templates/spirit-ts/manifest.toml"))
        .unwrap()
        .replace("{{project-name}}", "parity-spirit")
}

/// The 22-4 witness manifest (`directed_task_22_4.rs`), the smallest launchable component.
fn witness() -> String {
    r#"[class]
name = "probe-d"
description = "directed task witness"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 7
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "dist/recall_probe.wasm"
trust_tier = "public-untrusted"
[posture]
default = "assistive"
allowed_max = "assistive"
[output_shape]
required_fields = ["introduction"]
[budget]
context_window_size = 4096
time_cap_seconds = 30
[resources]
fd_max = 64
[sandbox]
tier = "T2"
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.log]
recall = true
fetch = true
"#
    .to_owned()
}

fn corpus() -> Vec<(&'static str, String)> {
    let t = template();
    let with = |extra: &str| format!("{t}\n{extra}\n");
    vec![
        ("template", t.clone()),
        ("22-4 witness", witness()),
        (
            "rust-inproc",
            std::fs::read_to_string(root().join("templates/spirit-rust/manifest.toml"))
                .unwrap()
                .replace("{{crate_name}}", "parity-rust"),
        ),
        ("wasm at T0", t.replace("tier = \"T2\"", "tier = \"T0\"")),
        ("wasm at T1", t.replace("tier = \"T2\"", "tier = \"T1\"")),
        ("no [sandbox]", t.replace("[sandbox]\ntier = \"T2\"\n", "")),
        (
            "no [posture]",
            t.replace("[posture]\ndefault = \"assistive\"\nallowed_max = \"assistive\"\n", ""),
        ),
        ("no [resources]", {
            let start = t.find("[resources]").unwrap();
            let end = t.find("[sandbox]").unwrap();
            format!("{}{}", &t[..start], &t[end..])
        }),
        ("no [output_shape]", t.replace("[output_shape]\nrequired_fields = [\"introduction\"]\n", "")),
        ("resource cap out of range", t.replace("cpu_max_pct = 100", "cpu_max_pct = 100000")),
        (
            "telemetry hook without its table (schema 7)",
            with("[lifecycle]\nenabled_hooks = [\"on_telemetry_event\"]"),
        ),
        (
            "telemetry table without its hook",
            with("[lifecycle]\nenabled_hooks = []\n[lifecycle.telemetry]\nscope = \"x\"\ntime_cap_seconds = 1"),
        ),
        ("unknown lifecycle hook", with("[lifecycle]\nenabled_hooks = [\"on_nothing\"]")),
        (
            "provider pin not declared in provider.complete",
            with("[providers.primary]\nid = \"anthropic\""),
        ),
        (
            "provider fallback",
            with(
                "[capabilities.required.provider]\ncomplete = [\"anthropic\"]\n[providers.primary]\nid = \"anthropic\"\n[[providers.fallback]]\nid = \"anthropic\"",
            ),
        ),
        ("provider id unsupported", with("[providers.primary]\nid = \"no-such-provider\"")),
        ("malformed [swap_invariants]", with("[swap_invariants]\npreserve = 3")),
        ("malformed [consent]", with("[consent]\nrecall = 3")),
        ("fkcs off-surface reference", with("[fkcs]\ninternal_references = [\"maos_kernel_core::scheduler::pick_next_spirit_from_slice\"]")),
        ("cli_wrapper beside class", with("[cli_wrapper]\ncommand = \"x\"")),
        ("not TOML", "this = = is not toml".to_owned()),
        ("no [class]", t.replace("[class]", "[klass]")),
        ("local tier wasm", t.replace("public-untrusted", "local")),
        ("public-vetted aspiration", t.replace("public-untrusted", "public-vetted")),
        ("schema 6", t.replace("manifest_schema_version = 7", "manifest_schema_version = 6")),
    ]
}

fn component() -> Vec<u8> {
    std::fs::read(root().join("tests/fixtures/wasm/echo_spirit_component.wasm")).unwrap()
}

/// What `maosctl spirit install` would hand `admit_spirit` for this manifest, signed for real.
fn install_gate(dir: &Path, manifest: &str) -> Option<&'static str> {
    let (m, a, key) = (dir.join("m.toml"), dir.join("a.wasm"), dir.join("k.key"));
    std::fs::write(&m, manifest).unwrap();
    std::fs::write(&a, component()).unwrap();
    std::fs::write(&key, "07".repeat(32)).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    // `--tier` is only checked against the manifest, so read it back from the manifest itself.
    let tier = maos_registry::admission::extract_manifest_tier(manifest.as_bytes()).ok();
    let tier = match tier {
        Some(TrustTier::Local) => "local",
        Some(TrustTier::OrgInternal) => "org_internal",
        Some(TrustTier::PublicVetted) => "public_vetted",
        _ => "public_untrusted",
    };
    let pkg = build_signed_package(&PublishArgs {
        tier: tier.into(),
        manifest: m,
        artifact: a,
        signing_key: Some(key),
        signing_key_env: None,
        registry_uri: None,
        compliance_claim: None,
        dry_run: true,
    });
    let pkg = match pkg {
        Ok(pkg) => pkg,
        // No identity to sign (not TOML, no [class]): `validate` signs an empty-id package and
        // the gate is the one that refuses it; ask the gate the same way.
        Err(_) => {
            maos_spirit_cli::validate::ephemeral_package(manifest.as_bytes().to_vec(), component())
                .unwrap()
        }
    };
    let cfg = AdmissionConfig {
        tier_floor: TrustTier::Local,
        registry_origin_tier: TrustTier::Local,
        t3_for_public_untrusted: false,
        allow_unsigned_local: true,
        org_signing_pubkey: None,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };
    admit_spirit(&pkg, &cfg).err().map(|e| e.code())
}

#[test]
fn validate_and_the_boot_gate_agree_on_every_corpus_manifest() {
    let scratch = tempfile::tempdir().unwrap();
    let mut refused = 0;
    let mut admitted = 0;
    for (name, manifest) in corpus() {
        let boot = gate_manifest(&manifest, &|_| false).err().map(|e| e.code());
        let install = install_gate(scratch.path(), &manifest);
        let report = validate_bytes(manifest.clone().into_bytes(), component(), None).unwrap();
        let reported = |check: &str| {
            report
                .findings
                .iter()
                .find(|f| f.check == check)
                .map(|f| f.code)
        };
        assert_eq!(
            reported("manifest"),
            boot,
            "{name}: boot-gate verdict\n{report:?}"
        );
        assert_eq!(
            reported("admission"),
            install,
            "{name}: install-gate verdict\n{report:?}"
        );
        assert_eq!(
            report.admissible(),
            boot.is_none() && install.is_none(),
            "{name}: validate must be admissible exactly when both gates admit\n{report:?}"
        );
        if report.admissible() {
            admitted += 1;
        } else {
            refused += 1;
        }
    }
    // The corpus must exercise both verdicts, or the equality above proves nothing.
    assert!(admitted >= 2, "admitted {admitted}");
    assert!(refused >= 12, "refused {refused}");
}

#[test]
fn validate_is_stricter_than_the_gates_only_by_the_component_check() {
    // A core module passes both manifest gates and fails only the component check.
    let module = std::fs::read(root().join("tests/fixtures/wasm/benign.wasm")).unwrap();
    let report = validate_bytes(template().into_bytes(), module, None).unwrap();
    let checks: Vec<_> = report.findings.iter().map(|f| f.check).collect();
    assert_eq!(checks, ["component"]);
}
