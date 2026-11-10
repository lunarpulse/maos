#![cfg(feature = "network")]
#![forbid(unsafe_code)]
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn real_shell_shares_provider_across_package_crl_and_token() {
    use maos_compliance::builder::{build_signed_envelope, encode_claim_bytes};
    use maos_domain::ports::registry::{SignedPackage, SpiritId, TrustTier};
    use maos_domain::revocation::{
        canonical_entries_bytes, CrlId, RevocationEntry, RevocationOrigin, SignedRevocationList,
    };
    use maos_spirit_abi::compliance::{
        CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use sha2::{Digest, Sha256};
    struct Reap(std::process::Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let data = dir.path().join("data");
    let installed = dir
        .path()
        .join(".local/share/maos/registry/installed/crypto-integrated/1.0.0");
    std::fs::create_dir_all(&installed).unwrap();
    let manifest = b"[spirit]\nname = \"crypto-integrated\"\nversion = \"1.0.0\"\ntrust_tier = \"public_untrusted\"\nsandbox_tier = \"t3\"\n".to_vec();
    let artifact = b"installed component".to_vec();
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let public: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
    let mut hash = Sha256::new();
    hash.update((manifest.len() as u64).to_le_bytes());
    hash.update(&manifest);
    hash.update((artifact.len() as u64).to_le_bytes());
    hash.update(&artifact);
    let signature = key.sign(&hash.finalize()).as_ref().try_into().unwrap();
    let fingerprint = ExecutionContextFingerprint {
        manifest_hash: Sha256::digest(&manifest).into(),
        spirit_version: "1.0.0".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: Default::default(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: String::new(),
            endpoint_url: String::new(),
            model_id: None,
        },
        crypto_provider: CryptoProviderId(String::new()),
    };
    let claim = build_signed_envelope(encode_claim_bytes(&fingerprint, None), &key, public);
    let package = SignedPackage::new(
        SpiritId::from("crypto-integrated"),
        "1.0.0".into(),
        manifest.clone(),
        artifact.clone(),
        signature,
        public,
        claim,
    );
    std::fs::write(installed.join("manifest.toml"), manifest).unwrap();
    std::fs::write(installed.join("artifact.bin"), artifact).unwrap();
    std::fs::write(
        installed.join("signed_package.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
    std::fs::write(
        installed
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("installed-index.json"),
        r#"{"crypto-integrated":"crypto-integrated/1.0.0"}"#,
    )
    .unwrap();
    let crl_key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
    let crl_public: [u8; 32] = crl_key.public_key().as_ref().try_into().unwrap();
    let entries =
        vec![RevocationEntry::new("never-matches", ">=0.1.0,<0.2.0", "compromised", None).unwrap()];
    let crl = SignedRevocationList::new(
        CrlId::from_entries(&entries).unwrap(),
        1,
        0,
        RevocationOrigin::Operator,
        entries.clone(),
        crl_key
            .sign(&canonical_entries_bytes(&entries).unwrap())
            .as_ref()
            .try_into()
            .unwrap(),
        crl_public,
    )
    .unwrap();
    let crl_path = dir.path().join("input.crl");
    std::fs::write(&crl_path, serde_json::to_vec(&crl).unwrap()).unwrap();
    let binary = fixtures::fixture("maos-crypto-recording");
    let ctl = std::path::Path::new(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    let env = |cmd: &mut Command| {
        cmd.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", dir.path())
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", &data)
            .env("XDG_CONFIG_HOME", dir.path().join("config"))
            .env("MAOS_MEMORY_ROOT", dir.path().join("memory"))
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env("MAOS_OLLAMA_URL", "skip")
            .env("MAOS_CRL_TRUST_ANCHOR_PUB_HEX", hex::encode(crl_public));
    };
    let mut init = Command::new(&binary);
    env(&mut init);
    let output = init
        .args(["init", "--crypto-provider", "ring"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut reselect = Command::new(&binary);
    env(&mut reselect);
    let output = reselect
        .args(["init", "--crypto-provider", "recording"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ring -> recording"));
    let mut command = Command::new(&binary);
    env(&mut command);
    command
        .args(["shell", "--plain"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut root = Reap(command.spawn().unwrap());
    let log_path = home.join("crypto-recording-identity.log");
    let deadline = Instant::now() + Duration::from_secs(25);
    let before_crl = loop {
        let log = std::fs::read_to_string(&log_path).unwrap_or_default();
        let mut list = Command::new(&ctl);
        env(&mut list);
        if list
            .args(["revocations", "list"])
            .output()
            .unwrap()
            .status
            .success()
            && log
                .lines()
                .filter(|line| line.ends_with(" verify_signature"))
                .count()
                >= 2
        {
            break log;
        }
        assert!(
            root.0.try_wait().unwrap().is_none(),
            "shell exited before boot verification"
        );
        assert!(
            Instant::now() < deadline,
            "shell did not verify installed package before serving: {log}"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    let boot_verifies = before_crl
        .lines()
        .filter(|line| line.ends_with(" verify_signature"))
        .count();
    let mut import = Command::new(&ctl);
    env(&mut import);
    let output = import
        .args(["revocations", "import"])
        .arg(&crl_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let log = std::fs::read_to_string(&log_path).unwrap();
        if log
            .lines()
            .filter(|line| line.ends_with(" verify_signature"))
            .count()
            > boot_verifies
        {
            break;
        }
        assert!(Instant::now() < deadline, "CRL not verified: {log}");
        std::thread::sleep(Duration::from_millis(20));
    }
    root.0
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"@hello-spirit say hi\n")
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let operations = loop {
        let log = std::fs::read_to_string(&log_path).unwrap();
        if log
            .lines()
            .any(|line| line.ends_with(" sign_capability_token"))
        {
            break log;
        }
        assert!(
            Instant::now() < deadline,
            "shell did not issue token: {log}"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let rows: Vec<_> = operations
        .lines()
        .map(|line| line.split_once(' ').unwrap())
        .collect();
    assert!(rows.iter().all(|(id, _)| *id == rows[0].0), "{operations}");
}
