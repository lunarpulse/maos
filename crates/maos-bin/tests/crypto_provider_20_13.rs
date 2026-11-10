#![cfg(feature = "network")]

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

use std::process::Command;

#[test]
fn init_persists_selected_provider() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["init", "--crypto-provider", "ring"])
        .env_clear()
        .env("HOME", home.path())
        .env("MAOS_HOME", home.path().join("maos"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let config = std::fs::read_to_string(home.path().join("maos/config.toml")).unwrap();
    let value: toml::Value = toml::from_str(&config).unwrap();
    assert_eq!(
        value
            .get("crypto")
            .and_then(|v| v.get("provider"))
            .and_then(toml::Value::as_str),
        Some("ring")
    );
}

#[test]
fn unknown_provider_refused_without_changing_config() {
    let home = tempfile::tempdir().unwrap();
    let command = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_maos"))
            .args(args)
            .env_clear()
            .env("HOME", home.path())
            .env("MAOS_HOME", home.path().join("maos"))
            .env("XDG_DATA_HOME", home.path().join("data"))
            .output()
            .unwrap()
    };
    assert!(command(&["init"]).status.success());
    let config = home.path().join("maos/config.toml");
    let original = std::fs::read(&config).unwrap();
    let rejected = command(&["init", "--crypto-provider", "bogus"]);
    assert_eq!(rejected.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("ring"));
    assert_eq!(std::fs::read(&config).unwrap(), original);
    for args in [
        vec!["init", "--crypto-provider=bogus"],
        vec![
            "init",
            "--crypto-provider",
            "ring",
            "--crypto-provider",
            "bogus",
        ],
    ] {
        let rejected = command(&args);
        assert_eq!(rejected.status.code(), Some(78), "{args:?}");
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("ring"));
        assert_eq!(std::fs::read(&config).unwrap(), original);
    }
    std::fs::write(&config, "[crypto]\nprovider = \"bogus\"\n").unwrap();
    let rejected = command(&["run", "spirits/butler/manifest.toml", "--once"]);
    assert_eq!(rejected.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("ring"));
    let rejected_without_manifest = command(&["run"]);
    assert_eq!(rejected_without_manifest.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&rejected_without_manifest.stderr).contains("ring"));
    std::fs::write(&config, "[crypto]\nprovider = 4\n").unwrap();
    let malformed = command(&["run", "spirits/butler/manifest.toml", "--once"]);
    assert_eq!(malformed.status.code(), Some(78));
    assert!(
        String::from_utf8_lossy(&malformed.stderr).contains("[crypto] provider must be a string")
    );
}

#[test]
fn malformed_provider_reports_the_exact_type_error() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join("config.toml"), "[crypto]\nprovider = 4\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["run"])
        .env_clear()
        .env("HOME", home.path())
        .env("MAOS_HOME", home.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&output.stderr).contains("[crypto] provider must be a string"));
}

#[test]
fn absent_home_never_reads_working_directory_config() {
    let cwd = tempfile::tempdir().unwrap();
    std::fs::write(
        cwd.path().join("config.toml"),
        "[crypto]\nprovider = \"bogus\"\n",
    )
    .unwrap();
    let data = tempfile::tempdir().unwrap();
    let audit = data.path().join("audit/transparency.sqlite");
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .current_dir(cwd.path())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("XDG_DATA_HOME", data.path().join("xdg"))
        .env("MAOS_MEMORY_ROOT", data.path().join("memory"))
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_NOTIFY_DISABLE", "1")
        .env(
            "MAOS_JOURNAL_PATH",
            data.path().join("journal/lifecycle.ndjson"),
        )
        .env("MAOS_ARCHIVE_DIR", data.path().join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", data.path().join("proofs"))
        .env("MAOS_AUDIT_DB", &audit)
        .args(["shell", "--plain"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let db = rusqlite::Connection::open(audit).unwrap();
    let payload: Vec<u8> = db.query_row(
        "SELECT payload_redacted FROM transparency_log WHERE intent = 'crypto.provider.selected'",
        [],
        |row| row.get(0),
    ).unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(payload["provider"], "ring");
    assert_eq!(payload["source"], "default");

    let configured = Command::new(env!("CARGO_BIN_EXE_maos"))
        .current_dir(cwd.path())
        .env_clear()
        .env("MAOS_HOME", cwd.path())
        .args(["run"])
        .output()
        .unwrap();
    assert_eq!(configured.status.code(), Some(78));
    assert!(String::from_utf8_lossy(&configured.stderr).contains("bogus"));
}

#[test]
fn ring_version_matches_lockfile() {
    let lock: toml::Value = toml::from_str(include_str!("../../../Cargo.lock")).unwrap();
    let packages = lock["package"].as_array().unwrap();
    let versions: Vec<_> = packages
        .iter()
        .filter(|p| p["name"].as_str() == Some("ring"))
        .filter_map(|p| p["version"].as_str())
        .collect();
    assert!(versions.contains(&maos_bin::crypto_provider::validate("ring").unwrap().version));
}

#[test]
fn reselection_preserves_unrelated_configuration() {
    let home = tempfile::tempdir().unwrap();
    let config_dir = home.path().join("maos");
    let init = || {
        Command::new(env!("CARGO_BIN_EXE_maos"))
            .env_clear()
            .env("HOME", home.path())
            .env("MAOS_HOME", &config_dir)
            .env("XDG_DATA_HOME", home.path().join("data"))
            .args(["init", "--crypto-provider", "ring"])
            .output()
            .unwrap()
    };
    assert!(init().status.success());
    let config = config_dir.join("config.toml");
    let before = std::fs::read_to_string(&config).unwrap();
    let result = init();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("ring -> ring"));
    assert_eq!(std::fs::read_to_string(config).unwrap(), before);
}

#[test]
fn init_repairs_and_reselects_inline_or_missing_provider() {
    let home = tempfile::tempdir().unwrap();
    let dir = home.path().join("maos");
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config.toml");
    let binary = fixtures::fixture("maos-crypto-recording");
    for previous in [
        "crypto = { provider = \"ring\", keep = true }\n",
        "[crypto]\nkeep = true\n",
    ] {
        std::fs::write(
            &config,
            format!("# retained\n{previous}\n[slots]\n[retention]\n"),
        )
        .unwrap();
        let output = Command::new(&binary)
            .args(["init", "--crypto-provider", "recording"])
            .env_clear()
            .env("HOME", home.path())
            .env("MAOS_HOME", &dir)
            .env("XDG_DATA_HOME", home.path().join("data"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = std::fs::read_to_string(&config).unwrap();
        assert!(
            text.contains("# retained") && text.contains("keep = true"),
            "{text}"
        );
        let parsed: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(parsed["crypto"]["provider"].as_str(), Some("recording"));
        if previous.starts_with("crypto") {
            assert!(String::from_utf8_lossy(&output.stdout).contains("ring -> recording"));
        }
    }
}

#[test]
fn recording_provider_observes_boot_token_signing() {
    let home = tempfile::tempdir().unwrap();
    let binary = fixtures::fixture("maos-crypto-recording");
    let command = |args: &[&str]| {
        Command::new(&binary)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", home.path())
            .env("MAOS_HOME", home.path().join("maos"))
            .env("XDG_DATA_HOME", home.path().join("data"))
            .env("XDG_CONFIG_HOME", home.path().join("config"))
            .env("MAOS_MEMORY_ROOT", home.path().join("memory"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    let init = command(&["init", "--crypto-provider", "recording"]);
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let boot = Command::new(&binary)
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home.path())
        .env("MAOS_HOME", home.path().join("maos"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("MAOS_MEMORY_ROOT", home.path().join("memory"))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .env("MAOS_ONE_SHOT", "hello-spirit")
        .env("MAOS_OLLAMA_URL", "skip")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    let log = std::fs::read_to_string(home.path().join("maos/crypto-recording.log")).unwrap();
    assert!(
        log.lines().any(|line| line == "sign_capability_token"),
        "{log}"
    );
}

#[test]
fn installed_claims_compare_actual_crypto_and_endpoint() {
    use maos_compliance::builder::{build_signed_envelope, encode_claim_bytes};
    use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
    use maos_spirit_abi::compliance::{
        CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use sha2::Digest;
    use std::collections::BTreeSet;

    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let public: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
    let home = tempfile::tempdir().unwrap();
    let manifest_path = home.path().join("manifest.toml");
    std::fs::write(home.path().join("artifact.bin"), b"binary").unwrap();
    let provider = maos_kernel_core::security::crypto::RingCryptoProvider;
    let verify = |crypto: &str, endpoint: &str| {
        let manifest = format!(
            "[spirit]\nname = \"drift-spirit\"\nversion = \"0.1.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\nprovider_id = \"anthropic\"\nendpoint_url = \"{endpoint}\"\ncrypto_provider = \"{crypto}\"\n"
        );
        std::fs::write(&manifest_path, &manifest).unwrap();
        let fingerprint = ExecutionContextFingerprint {
            manifest_hash: sha2::Sha256::digest(manifest.as_bytes()).into(),
            spirit_version: "0.1.0".into(),
            trust_tier: TrustTier::PublicUntrusted,
            sandbox_tier: SandboxTier::T3,
            capability_scope: BTreeSet::new(),
            provider_endpoint: ProviderEndpointPin {
                provider_id: "anthropic".into(),
                endpoint_url: endpoint.into(),
                model_id: None,
            },
            crypto_provider: CryptoProviderId(crypto.into()),
        };
        let claim = build_signed_envelope(encode_claim_bytes(&fingerprint, None), &key, public);
        let mut hash = sha2::Sha256::new();
        hash.update((manifest.len() as u64).to_le_bytes());
        hash.update(manifest.as_bytes());
        hash.update((6u64).to_le_bytes());
        hash.update(b"binary");
        let signature: [u8; 64] = key.sign(&hash.finalize()).as_ref().try_into().unwrap();
        let pkg = SignedPackage::new(
            SpiritId::from("drift-spirit"),
            "0.1.0".into(),
            manifest.into_bytes(),
            b"binary".to_vec(),
            signature,
            public,
            claim,
        );
        std::fs::write(
            home.path().join("signed_package.json"),
            serde_json::to_vec(&pkg).unwrap(),
        )
        .unwrap();
        maos_registry::install::verify_installed(
            "drift-spirit",
            &manifest_path,
            &provider,
            "ring",
            &|id| (id == "anthropic").then_some("https://api.anthropic.com"),
        )
    };
    let crypto = verify("fips-module", "https://api.anthropic.com")
        .unwrap_err()
        .to_string();
    assert!(crypto.contains("CryptoProvider"), "{crypto}");
    // Boot the installed package through the real composition root, not a
    // second evaluator: both publisher and claim must reach the recording port.
    let registry = home.path().join(".local/share/maos/registry");
    let installed = registry.join("installed/drift-spirit/0.1.0");
    std::fs::create_dir_all(&installed).unwrap();
    for name in ["manifest.toml", "artifact.bin", "signed_package.json"] {
        std::fs::copy(home.path().join(name), installed.join(name)).unwrap();
    }
    std::fs::write(
        registry.join("installed-index.json"),
        r#"{"drift-spirit":"drift-spirit/0.1.0"}"#,
    )
    .unwrap();
    let binary = fixtures::fixture("maos-crypto-recording");
    let command = |args: &[&str]| {
        Command::new(&binary)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", home.path())
            .env("MAOS_HOME", home.path().join("maos"))
            .env("XDG_DATA_HOME", home.path().join("data"))
            .env("XDG_CONFIG_HOME", home.path().join("config"))
            .env("MAOS_MEMORY_ROOT", home.path().join("memory"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    assert!(command(&["init", "--crypto-provider", "recording"])
        .status
        .success());
    let refused_boot = command(&["shell", "--plain"]);
    assert_eq!(
        refused_boot.status.code(),
        Some(0),
        "daemon continues after per-spirit refusal"
    );
    let refusal = String::from_utf8_lossy(&refused_boot.stderr);
    assert!(
        refusal.contains("installed spirit 'drift-spirit' refused at boot: admission_refused")
            && refusal.contains("CryptoProvider"),
        "{refusal}"
    );
    std::fs::remove_file(home.path().join("maos/crypto-recording.log")).unwrap();
    let endpoint = verify("ring", "https://evil.example")
        .unwrap_err()
        .to_string();
    assert!(endpoint.contains("ProviderEndpoint"), "{endpoint}");
    assert!(verify("", "").is_ok());
    for name in ["manifest.toml", "artifact.bin", "signed_package.json"] {
        std::fs::copy(home.path().join(name), installed.join(name)).unwrap();
    }
    let boot = command(&["shell", "--plain"]);
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    let log = std::fs::read_to_string(home.path().join("maos/crypto-recording.log")).unwrap();
    assert!(
        log.lines()
            .filter(|line| *line == "verify_signature")
            .count()
            >= 2,
        "{log}"
    );
}

#[test]
fn selected_provider_is_journaled_once_per_boot() {
    let home = tempfile::tempdir().unwrap();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_maos"));
    let ctl = binary.with_file_name("maosctl");
    assert!(ctl.is_file(), "build maos-cli first");
    let run = |program: &std::path::Path, args: &[&str]| {
        Command::new(program)
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", home.path())
            .env("MAOS_HOME", home.path().join("maos"))
            .env("XDG_DATA_HOME", home.path().join("data"))
            .env("XDG_CONFIG_HOME", home.path().join("config"))
            .env("MAOS_MEMORY_ROOT", home.path().join("memory"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap()
    };
    assert!(run(binary, &["init"]).status.success());
    let boot = run(binary, &["shell", "--plain"]);
    assert!(
        boot.status.success(),
        "{}",
        String::from_utf8_lossy(&boot.stderr)
    );
    let result = run(
        &ctl,
        &[
            "audit",
            "query",
            "--frame-kind",
            "TelemetryEvent",
            "--intent-contains",
            "crypto.provider",
            "--format",
            "ndjson",
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&result.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let payload: serde_json::Value =
        serde_json::from_str(rows[0]["payload"].as_str().unwrap()).unwrap();
    assert_eq!(payload["provider"], "ring");
    assert_eq!(
        payload["version"],
        maos_bin::crypto_provider::validate("ring").unwrap().version
    );
    assert_eq!(payload["source"], "config");
    // An upgraded home without a crypto table records the default exactly once.
    std::fs::write(home.path().join("maos/config.toml"), "[slots]\n").unwrap();
    let second = run(binary, &["shell", "--plain"]);
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let result = run(
        &ctl,
        &[
            "audit",
            "query",
            "--frame-kind",
            "TelemetryEvent",
            "--intent-contains",
            "crypto.provider",
            "--format",
            "ndjson",
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&result.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(rows.len(), 2, "{rows:?}");
    let payloads: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| serde_json::from_str(row["payload"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(
        payloads
            .iter()
            .filter(|payload| payload["source"] == "default")
            .count(),
        1
    );
    assert_eq!(
        payloads
            .iter()
            .filter(|payload| payload["source"] == "config")
            .count(),
        1
    );
}

#[test]
fn selection_journal_failure_stops_real_boot() {
    use maos_kernel_core::iac::TransparencyLogAdapter;
    let home = tempfile::tempdir().unwrap();
    let db = home.path().join("maos/audit/transparency.sqlite");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    drop(TransparencyLogAdapter::open_with_global_legal_holds(&db, &db, 1).unwrap());
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fail_crypto_selection BEFORE INSERT ON transparency_log \
         WHEN NEW.intent = 'crypto.provider.selected' \
         BEGIN SELECT RAISE(ABORT, 'selection storage failure'); END;",
        )
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_maos"))
        .args(["shell", "--plain"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", home.path())
        .env("MAOS_HOME", home.path().join("maos"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("MAOS_MEMORY_ROOT", home.path().join("memory"))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("selection storage failure"));
    assert!(!home.path().join("maos/control.json").exists());
}

#[test]
fn recording_provider_observes_crl_import_over_door() {
    use maos_domain::revocation::{
        canonical_entries_bytes, CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let scratch = tempfile::tempdir().unwrap();
    let bin = fixtures::fixture("maos-crypto-recording");
    let ctl = std::path::Path::new(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    let key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    let public: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
    let entries =
        vec![RevocationEntry::new("hello-spirit", ">=0.1.0,<0.2.0", "compromised", None).unwrap()];
    let signature: [u8; 64] = key
        .sign(&canonical_entries_bytes(&entries).unwrap())
        .as_ref()
        .try_into()
        .unwrap();
    let crl = SignedRevocationList::new(
        CrlId::from_entries(&entries).unwrap(),
        1,
        0,
        RevocationOrigin::Operator,
        entries,
        signature,
        public,
    )
    .unwrap();
    let crl_path = scratch.path().join("input.crl");
    std::fs::write(&crl_path, serde_json::to_vec(&crl).unwrap()).unwrap();
    let command = |program: &std::path::Path| {
        let mut cmd = Command::new(program);
        cmd.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.path())
            .env("MAOS_HOME", scratch.path().join("maos"))
            .env("XDG_DATA_HOME", scratch.path().join("data"))
            .env("XDG_CONFIG_HOME", scratch.path().join("config"))
            .env("MAOS_MEMORY_ROOT", scratch.path().join("memory"))
            .env("MAOS_CRL_TRUST_ANCHOR_PUB_HEX", hex::encode(public))
            .env("MAOS_NOTIFY_DISABLE", "1");
        cmd
    };
    assert!(command(&bin)
        .args(["init", "--crypto-provider", "recording"])
        .output()
        .unwrap()
        .status
        .success());
    let mut root = ChildGuard(
        command(&bin)
            .args(["shell", "--plain"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        let list = command(&ctl)
            .args(["revocations", "list"])
            .output()
            .unwrap();
        if list.status.success() {
            break;
        }
        assert!(
            root.0.try_wait().unwrap().is_none(),
            "daemon exited before door readiness"
        );
        assert!(
            Instant::now() < deadline,
            "operator door did not become ready"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let imported = command(&ctl)
        .args(["revocations", "import"])
        .arg(&crl_path)
        .output()
        .unwrap();
    assert!(
        imported.status.success(),
        "{}",
        String::from_utf8_lossy(&imported.stderr)
    );
    let log = std::fs::read_to_string(scratch.path().join("maos/crypto-recording.log")).unwrap();
    assert!(log.lines().any(|line| line == "verify_signature"), "{log}");
}

#[cfg(feature = "crypto-recording")]
#[test]
fn recording_provider_routes_real_at_rest_seal() {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(
        home.path().join("config.toml"),
        "[crypto]\nprovider = \"recording\"\n",
    )
    .unwrap();
    let selected = maos_bin::crypto_provider::select(Some(home.path())).unwrap();
    let kms = maos_secrets::LocalMasterKeyKms::from_master_key(&[0x42; 32]).unwrap();
    let sealed =
        maos_secrets::seal_at_rest(&kms, selected.provider.as_ref(), b"sensitive row").unwrap();
    assert_ne!(sealed, b"sensitive row");
    let log = std::fs::read_to_string(home.path().join("crypto-recording.log")).unwrap();
    assert!(log.lines().any(|line| line == "seal_for_export"));
}
