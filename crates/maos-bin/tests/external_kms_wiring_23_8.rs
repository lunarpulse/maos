//! Story 23-8 — composition-root wiring of the external KMS adapter (AC1 d–e,
//! AC2 witness, AC3 boot refusals, AC6, AC7, AC8). The endpoint is the
//! in-process `maos.kms.v1` fixture of `maos-secrets`; no container, no network,
//! no sleeps. Every daemon-booting test runs the `maos` binary with a scrubbed
//! environment: a stray `MAOS_KMS_*`, `MAOS_SSO_*`, `MAOS_SIEM_*` or
//! `MAOS_AUDIT_KEY` in the runner would flip the posture
//! (`any_env_with_prefix`), so nothing is inherited.

#![cfg(feature = "network")]

#[path = "../../maos-secrets/tests/support/kms_fixture_23_8.rs"]
mod kms_fixture;

use maos_bin::enterprise_identity::{
    configure_kms, ConfiguredKms, EnterpriseConfig, EnterpriseFailure, EnterpriseRuntime,
    KmsBootRefusal, KmsEnv,
};
use maos_domain::outbound::{ResolveFailed, ResolveHost};
use maos_kernel_core::security::RingCryptoProvider;
use maos_loom_lite::seal::AtRestSealer;
use maos_loom_lite::store::StoreError;
use std::net::IpAddr;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

const PLAINTEXT: &[u8] = b"{\"collective\":\"row\",\"canary\":\"collective-canary-5521\"}";
const KEY_CANARY: &str = "zz-not-a-hex-master-key-canary";

/// Name resolution scripted per test; `None` fails like a dead resolver.
struct Names(Option<Vec<IpAddr>>);

impl ResolveHost for Names {
    fn resolve(&self, _host: &str, _port: u16) -> Result<Vec<IpAddr>, ResolveFailed> {
        self.0.clone().ok_or(ResolveFailed)
    }
}

fn select(
    endpoint: Option<&str>,
    master_key: Option<&str>,
    allow_private: bool,
    names: Option<Vec<IpAddr>>,
) -> Result<Option<ConfiguredKms>, KmsBootRefusal> {
    configure_kms(
        &KmsEnv {
            endpoint,
            master_key,
            allow_private,
            any_kms_variable: endpoint.is_some() || master_key.is_some(),
        },
        &Names(names),
    )
}

// ── AC6 — selection and refusals (in-process, scripted environment) ────────

#[test]
fn both_endpoint_and_master_key_is_a_conflict_naming_both_variables_and_no_value() {
    let refusal = select(Some("http://127.0.0.1:9"), Some(KEY_CANARY), false, None).unwrap_err();
    assert_eq!(refusal.code(), "kms_config_conflict");
    let text = refusal.to_string();
    assert!(text.contains("MAOS_KMS_ENDPOINT") && text.contains("MAOS_KMS_MASTER_KEY"));
    assert!(!text.contains(KEY_CANARY) && !text.contains("127.0.0.1"));
}

#[test]
fn an_unusable_master_key_is_a_refusal_not_a_silent_downgrade() {
    for bad in [
        KEY_CANARY.to_owned(),
        "ab".repeat(31),
        "ab".repeat(33),
        "00".repeat(32),
        String::new(),
    ] {
        let refusal = select(None, Some(&bad), false, None).unwrap_err();
        assert_eq!(refusal, KmsBootRefusal::MasterKeyInvalid, "{bad:?}");
        assert_eq!(refusal.code(), "kms_master_key_invalid");
        assert!(!refusal.to_string().contains(KEY_CANARY));
    }
    let kms = select(None, Some(&"42".repeat(32)), false, None)
        .unwrap()
        .expect("a valid key selects the local KMS");
    assert_eq!(kms.kind(), "local master key");
    assert!(kms.probe().is_none());
    assert!(kms.port().is_healthy());
}

#[test]
fn a_kms_variable_without_a_key_source_is_refused_and_no_variable_means_plaintext_default() {
    let refusal = configure_kms(
        &KmsEnv {
            any_kms_variable: true,
            ..KmsEnv::default()
        },
        &Names(None),
    )
    .unwrap_err();
    assert_eq!(refusal.code(), "kms_config_incomplete");
    assert!(configure_kms(&KmsEnv::default(), &Names(None))
        .unwrap()
        .is_none());
}

#[cfg(feature = "online-adapters")]
mod online {
    use super::*;
    use kms_fixture::KmsFixture;
    use maos_domain::ports::CryptoProvider;

    fn public() -> Option<Vec<IpAddr>> {
        Some(vec!["93.184.216.34".parse().unwrap()])
    }

    fn refused(endpoint: &str, allow_private: bool, names: Option<Vec<IpAddr>>) -> &'static str {
        match select(Some(endpoint), None, allow_private, names) {
            Err(KmsBootRefusal::EndpointRefused(detail)) => detail,
            other => panic!("{endpoint}: expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn http_is_admitted_only_to_a_literal_loopback_host() {
        for ok in [
            "http://127.0.0.1:8200",
            "http://[::1]:8200",
            "http://127.0.0.1:8200/v1/maos/",
        ] {
            let kms = select(Some(ok), None, false, None)
                .unwrap_or_else(|r| panic!("{ok}: {r}"))
                .expect("selected");
            assert_eq!(kms.kind(), "external endpoint");
        }
        // Any other host over http is refused, whatever it resolves to.
        for http in [
            "http://10.0.0.5:8200",
            "http://192.168.1.9",
            "http://8.8.8.8",
            "http://localhost:8200",
            "http://kms.example.org",
        ] {
            assert_eq!(
                refused(http, false, public()),
                "outbound_http_refused",
                "{http}"
            );
            assert_eq!(
                refused(http, true, public()),
                "outbound_http_refused",
                "{http}"
            );
        }
    }

    #[test]
    fn https_passes_the_outbound_policy_and_private_ranges_need_the_opt_in() {
        assert!(select(Some("https://kms.example.org"), None, false, public()).is_ok());
        assert!(select(Some("https://8.8.8.8:8443"), None, false, None).is_ok());
        // Private and link-local destinations.
        assert_eq!(
            refused("https://10.0.0.5", false, None),
            "outbound_private_refused"
        );
        assert!(select(Some("https://10.0.0.5"), None, true, None).is_ok());
        assert_eq!(
            refused("https://169.254.169.254", false, None),
            "outbound_link_local_refused"
        );
        assert_eq!(
            refused("https://169.254.169.254", true, None),
            "outbound_link_local_refused",
            "link-local is never admitted, opt-in or not"
        );
        let private_name = Some(vec!["10.1.2.3".parse().unwrap()]);
        assert_eq!(
            refused("https://kms.internal", false, private_name.clone()),
            "outbound_private_refused"
        );
        assert!(select(Some("https://kms.internal"), None, true, private_name).is_ok());
        // A name that resolves to loopback is not a literal loopback.
        let loopback_name = Some(vec!["127.0.0.1".parse().unwrap()]);
        assert_eq!(
            refused("https://kms.internal", false, loopback_name),
            "outbound_loopback_refused"
        );
        // A dead resolver and a malformed URL.
        assert_eq!(
            refused("https://kms.example.org", false, None),
            "outbound_unresolved"
        );
        for bad in ["", "   ", "not a url", "ftp://127.0.0.1:1"] {
            assert_eq!(
                refused(bad, false, public()),
                "outbound_url_invalid",
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_url_that_could_carry_a_credential_is_refused() {
        for bad in [
            "https://user:pass@kms.example.org",
            "https://kms.example.org/?token=x",
            "https://kms.example.org/#frag",
        ] {
            refused(bad, false, public());
        }
    }

    /// `MAOS_KMS_ALLOW_PRIVATE` is not "private only": the shared policy admits
    /// loopback whenever `allow_private` is set (`OutboundPolicy::admit`), by
    /// any route, so a name that resolves to loopback, to a private address or
    /// to both boots. Without the opt-in a name that resolves to loopback is
    /// refused. Link-local and reserved stay refused either way, and plain
    /// `http` stays loopback-literal only.
    #[test]
    fn with_the_opt_in_private_and_loopback_addresses_are_admitted_including_names() {
        let loopback_name = || Some(vec!["127.0.0.1".parse().unwrap()]);
        let private_name = || Some(vec!["10.1.2.3".parse().unwrap()]);
        let mixed = || {
            Some(vec![
                "10.1.2.3".parse().unwrap(),
                "127.0.0.1".parse().unwrap(),
            ])
        };
        assert_eq!(
            refused("https://localhost:8443", false, loopback_name()),
            "outbound_loopback_refused"
        );
        for names in [loopback_name(), private_name(), mixed()] {
            assert!(
                select(Some("https://localhost:8443"), None, true, names.clone()).is_ok(),
                "the opt-in admits {names:?}"
            );
        }
        assert!(select(Some("https://127.0.0.1:8443"), None, true, None).is_ok());
        assert_eq!(
            refused(
                "https://localhost:8443",
                true,
                Some(vec!["169.254.169.254".parse().unwrap()])
            ),
            "outbound_link_local_refused",
            "link-local is never admitted"
        );
        assert_eq!(
            refused("http://localhost:8443", true, loopback_name()),
            "outbound_http_refused",
            "plain http is a literal loopback only, opt-in or not"
        );
    }

    /// P3-1: no probe line claims that sealed writes resume while the bearer is
    /// pending or unreadable, or while calls fail; the probe outcome follows the
    /// bearer state and an unreadable bearer sends nothing.
    #[test]
    fn the_probe_outcome_follows_the_bearer_and_no_line_claims_writes_resume_until_calls_work() {
        use maos_bin::enterprise_identity::KmsProbeOutcome as Outcome;

        let fixture = KmsFixture::start();
        let kms = external(&fixture.url());
        let probe = kms.probe().expect("external endpoint");
        assert_eq!(probe(), Outcome::ReachableBearerPending);
        kms.bearer_unreadable();
        assert_eq!(probe(), Outcome::ReachableBearerUnreadable);
        let posts_before = fixture.count("POST", "/wrap");
        assert!(kms.port().wrap_data_key(&[1u8; 32]).is_err());
        assert!(
            !kms.port().is_healthy(),
            "an unreadable bearer is never healthy"
        );
        assert_eq!(
            fixture.count("POST", "/wrap"),
            posts_before,
            "nothing is sent"
        );
        kms.provision_bearer(None);
        assert_eq!(probe(), Outcome::Ready);

        for outcome in [
            Outcome::Unreachable,
            Outcome::ReachableBearerPending,
            Outcome::ReachableBearerUnreadable,
            Outcome::ReachableCallsFailing,
        ] {
            for boot in [true, false] {
                let line = outcome.line(boot);
                assert!(
                    !line.contains("resume"),
                    "{outcome:?} must not claim writes resume: {line}"
                );
            }
        }
        assert!(Outcome::Ready.line(false).contains("resume"));
        assert!(Outcome::ReachableBearerPending
            .line(true)
            .contains("stay refused"));
        assert!(!Outcome::Unreachable.reachable() && Outcome::Ready.reachable());
    }

    /// KMS-3: the outcome is derived from what the probe did to the applied
    /// health state. A healthy answer that was discarded because a call failed
    /// while it was in flight (or that left the adapter unhealthy) is never
    /// `Ready`, so the log never says that writes resume while health stayed down.
    #[test]
    fn a_discarded_or_unhealthy_probe_is_never_reported_as_ready() {
        use maos_bin::enterprise_identity::{probe_outcome, KmsProbeOutcome as Outcome};
        use maos_secrets::http_kms::{BearerStatus as Bearer, ProbeResult as Probe};

        assert_eq!(
            probe_outcome(Probe::Unreachable, Bearer::Settled, false, false),
            Outcome::Unreachable
        );
        let discarded = probe_outcome(Probe::Discarded, Bearer::Settled, false, true);
        assert_eq!(discarded, Outcome::ReachableCallsFailing);
        assert!(!discarded.line(false).contains("resume"));
        // Applied, but the adapter is not healthy afterwards.
        assert_eq!(
            probe_outcome(Probe::Applied, Bearer::Settled, false, false),
            Outcome::ReachableCallsFailing
        );
        assert_eq!(
            probe_outcome(Probe::Applied, Bearer::Settled, true, true),
            Outcome::ReachableCallsFailing
        );
        assert_eq!(
            probe_outcome(Probe::Applied, Bearer::Pending, false, false),
            Outcome::ReachableBearerPending
        );
        assert_eq!(
            probe_outcome(Probe::Applied, Bearer::Unreadable, false, false),
            Outcome::ReachableBearerUnreadable
        );
        assert_eq!(
            probe_outcome(Probe::Applied, Bearer::Settled, true, false),
            Outcome::Ready
        );
    }

    fn runtime_over(kms: &ConfiguredKms) -> EnterpriseRuntime {
        kms.provision_bearer(None);
        let runtime = EnterpriseRuntime::from_ports(
            EnterpriseConfig::empty().with_kms_available(),
            None,
            Some(kms.port()),
            None,
            Some(Arc::new(RingCryptoProvider) as Arc<dyn CryptoProvider>),
            None,
            None,
            0,
        );
        runtime.with_kms_probe(kms.probe().expect("an external endpoint has a probe"))
    }

    fn external(url: &str) -> ConfiguredKms {
        select(Some(url), None, false, None)
            .expect("endpoint accepted")
            .expect("selected")
    }

    // ── AC1 (d) (e) ─────────────────────────────────────────────────────────

    #[test]
    fn with_the_endpoint_down_sealed_writes_are_refused_and_resume_when_it_returns() {
        let mut fixture = KmsFixture::start();
        let kms = external(&fixture.url());
        let runtime = runtime_over(&kms);
        let probe = runtime.kms_probe().unwrap();
        assert!(probe().reachable(), "the endpoint is up");

        let sealed = runtime
            .seal_row_at_rest(PLAINTEXT)
            .expect("sealed while up");
        assert_ne!(sealed, PLAINTEXT);
        assert!(!sealed.windows(PLAINTEXT.len()).any(|w| w == PLAINTEXT));

        // (d) the fixture stopped and one probe run.
        fixture.stop();
        assert!(
            !probe().reachable(),
            "the probe of a stopped endpoint fails"
        );
        assert!(matches!(
            runtime.seal_row_at_rest(PLAINTEXT),
            Err(EnterpriseFailure::KmsSealedWriteRefused { .. })
        ));
        let sealer = AtRestSealer::new(runtime.at_rest_seal_hook());
        assert!(sealer.is_configured());
        assert!(matches!(
            sealer.seal(PLAINTEXT),
            Err(StoreError::AtRestSeal(_))
        ));

        // (e) the fixture restarted and one probe run.
        let _restarted = KmsFixture::restart(&fixture);
        // The endpoint answers again but no probe has seen it: the cached
        // health still says down, so the write is refused without any I/O. A
        // seal that skips the health check would succeed here.
        assert!(matches!(
            runtime.seal_row_at_rest(PLAINTEXT),
            Err(EnterpriseFailure::KmsSealedWriteRefused { .. })
        ));
        assert!(matches!(
            sealer.seal(PLAINTEXT),
            Err(StoreError::AtRestSeal(_))
        ));
        assert!(probe().reachable());
        let again = runtime
            .seal_row_at_rest(PLAINTEXT)
            .expect("sealing resumes");
        assert_ne!(again, PLAINTEXT);
        assert_ne!(
            sealer.seal(PLAINTEXT).expect("the hook resumes too"),
            PLAINTEXT
        );
    }

    // ── AC7 ─────────────────────────────────────────────────────────────────

    /// The endpoint is unreachable when the hook is built and no probe has ever
    /// succeeded. The old `at_rest_seal_hook` returned `None` here, so loom-lite
    /// kept the Option-A plaintext default and wrote the row. Now the hook
    /// exists and refuses the write; nothing is persisted.
    #[test]
    fn a_kms_down_at_boot_never_yields_a_plaintext_collective_row() {
        let mut dead = KmsFixture::start();
        dead.stop();
        let kms = external(&dead.url());
        let runtime = runtime_over(&kms);

        let hook = runtime.at_rest_seal_hook();
        assert!(
            hook.is_some(),
            "a configured KMS always yields a hook; None would mean plaintext"
        );
        let sealer = AtRestSealer::new(hook);
        assert!(sealer.is_configured());
        match sealer.seal(PLAINTEXT) {
            Err(StoreError::AtRestSeal(_)) => {}
            Ok(bytes) => panic!(
                "a Collective row was produced with the KMS down (plaintext: {})",
                bytes == PLAINTEXT
            ),
            Err(other) => panic!("unexpected error: {other:?}"),
        }
        assert!(runtime.seal_row_at_rest(PLAINTEXT).is_err());
        assert!(
            !runtime.kms_unbuilt(),
            "an unhealthy endpoint is not unbuilt"
        );

        // "...and succeeds once a probe sees the endpoint."
        let _up = KmsFixture::restart(&dead);
        assert!(runtime.kms_probe().unwrap()().reachable());
        let sealed = sealer.seal(PLAINTEXT).expect("sealed after the probe");
        assert_ne!(sealed, PLAINTEXT);
    }
}

/// Regression of the old fail-open: a configured KMS that is latched down (an
/// unusable local key used to land here) returned `None` and loom-lite wrote
/// plaintext. The hook now exists and always refuses.
#[test]
fn a_configured_down_runtime_has_a_refusing_hook_never_none() {
    let runtime = EnterpriseRuntime::from_config(&EnterpriseConfig::empty().with_kms_down())
        .expect("configured-down runtime");
    let hook = runtime.at_rest_seal_hook().expect("a hook, not None");
    assert!(hook(PLAINTEXT).is_err());
    assert!(runtime.seal_row_at_rest(PLAINTEXT).is_err());
    assert!(
        runtime.kms_unbuilt(),
        "the cohort daemon still refuses to start"
    );

    // Available with no port or no crypto provider behind it is the same.
    let hollow = EnterpriseRuntime::from_config(&EnterpriseConfig::empty().with_kms_available())
        .expect("hollow runtime");
    assert!(hollow.at_rest_seal_hook().expect("a hook")(PLAINTEXT).is_err());
    assert!(hollow.kms_unbuilt());

    // No KMS configured: Option-A plaintext, byte-identical, no hook.
    let disabled = EnterpriseRuntime::from_config(&EnterpriseConfig::empty()).unwrap();
    assert!(disabled.at_rest_seal_hook().is_none());
    assert_eq!(disabled.seal_row_at_rest(PLAINTEXT).unwrap(), PLAINTEXT);
    assert!(!disabled.kms_unbuilt());
}

// ── AC2 — vendor strings stay out of the release binary's source ────────────

fn rust_sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_vendor_string_is_in_the_release_sources() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_sources(&manifest.join("src"), &mut files);
    rust_sources(&manifest.join("../maos-secrets/src"), &mut files);
    assert!(
        files.len() > 10,
        "the source walk found {} files",
        files.len()
    );
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        for vendor in ["transit/", "X-Vault-Token"] {
            assert!(
                !text.contains(vendor),
                "{} contains the vendor string {vendor:?}: vendor strings stay in the sidecar",
                file.display()
            );
        }
    }
}

/// The reference sidecar's self-test against a stub Transit (AC2). It needs
/// `python3`; a host without it fails rather than skipping.
#[test]
fn the_reference_sidecar_passes_its_own_self_test() {
    let directory =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kms-sidecar-openbao");
    let output = Command::new("python3")
        .arg("selftest.py")
        .current_dir(&directory)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/local/bin")
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .output()
        .expect("python3 is required to run the sidecar self-test");
    assert!(
        output.status.success(),
        "sidecar self-test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── AC5 — the probe task ────────────────────────────────────────────────────

/// The periodic probe runs on a blocking thread until the shutdown token
/// fires. No timing is asserted: the test waits for three probes under an
/// outer deadline, cancels, and waits for the task to stop.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_probe_task_runs_until_the_shutdown_token_fires() {
    use maos_bin::enterprise_identity::{spawn_kms_probe, KmsProbeFn, KmsProbeOutcome};
    use std::time::Duration;

    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let probe: KmsProbeFn = Arc::new(move || {
        let _ = sender.send(());
        KmsProbeOutcome::Ready
    });
    let cancel = tokio_util::sync::CancellationToken::new();
    let handle = spawn_kms_probe(probe, Duration::from_millis(5), cancel.clone());
    for _ in 0..3 {
        tokio::time::timeout(Duration::from_secs(30), receiver.recv())
            .await
            .expect("the probe task must keep probing")
            .expect("probe channel open");
    }
    cancel.cancel();
    tokio::time::timeout(Duration::from_secs(30), handle)
        .await
        .expect("the probe task must stop when the token fires")
        .expect("the probe task did not panic");
}

// ── Subprocess boots (AC3, AC6, AC7, AC8) ───────────────────────────────────

fn boot(root: &Path, name: &str, vars: &[(&str, &str)]) -> Output {
    use std::io::Write;

    let home = root.join(format!("home-{name}"));
    let maos_home = root.join(format!("maos-home-{name}"));
    std::fs::create_dir_all(&maos_home).expect("create MAOS_HOME");
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .env_clear()
        .current_dir(root)
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .env("MAOS_HOME", &maos_home)
        .env("XDG_DATA_HOME", root.join(format!("xdg-data-{name}")))
        .env("XDG_CONFIG_HOME", root.join(format!("xdg-config-{name}")))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .env("MAOS_OLLAMA_URL", "skip")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in vars {
        command.env(key, value);
    }
    let mut child = command.spawn().expect("start maos");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"/quit\n")
        .ok();
    child.wait_with_output().expect("wait for maos")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_refused(output: &Output, code: &str) -> String {
    let text = stderr(output);
    assert_eq!(output.status.code(), Some(78), "{text}");
    assert!(text.contains(code), "expected {code} in: {text}");
    text
}

fn dead_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.local_addr().unwrap().port()
}

#[test]
fn boot_refuses_a_conflict_a_remote_http_endpoint_and_a_stray_variable() {
    let temp = tempfile::tempdir().expect("fixture");
    let root = temp.path();

    let conflict = boot(
        root,
        "conflict",
        &[
            ("MAOS_KMS_ENDPOINT", "http://127.0.0.1:9"),
            ("MAOS_KMS_MASTER_KEY", KEY_CANARY),
        ],
    );
    let text = assert_refused(&conflict, "kms_config_conflict");
    assert!(text.contains("MAOS_KMS_ENDPOINT") && text.contains("MAOS_KMS_MASTER_KEY"));
    assert!(!text.contains(KEY_CANARY));

    // A key variable alone, with no key source (and no ambient MAOS_KMS_*).
    let stray = boot(root, "stray", &[("MAOS_KMS_BEARER", "stray-bearer-canary")]);
    let text = assert_refused(&stray, "kms_config_incomplete");
    assert!(!text.contains("stray-bearer-canary"));

    #[cfg(feature = "online-adapters")]
    {
        let remote = boot(
            root,
            "remote",
            &[("MAOS_KMS_ENDPOINT", "http://10.9.9.9:8200/secret-path")],
        );
        let text = assert_refused(&remote, "kms_endpoint_refused");
        assert!(text.contains("outbound_http_refused"));
        assert!(!text.contains("secret-path"));
    }
    #[cfg(not(feature = "online-adapters"))]
    {
        let unsupported = boot(
            root,
            "unsupported",
            &[("MAOS_KMS_ENDPOINT", "http://127.0.0.1:9")],
        );
        assert_refused(&unsupported, "kms_endpoint_unsupported_build");
    }
}

/// The serve-path regression: a malformed local key used to latch
/// `ConfiguredDown`, boot with `kms=true` and write plaintext Collective rows.
/// It is now a boot refusal and the Host never reaches the runtime line.
#[test]
fn a_malformed_master_key_refuses_the_boot_instead_of_booting_plaintext() {
    let temp = tempfile::tempdir().expect("fixture");
    let output = boot(
        temp.path(),
        "badkey",
        &[("MAOS_KMS_MASTER_KEY", KEY_CANARY)],
    );
    let text = assert_refused(&output, "kms_master_key_invalid");
    assert!(!text.contains(KEY_CANARY));
    assert!(
        !text.contains("enterprise identity/at-rest/SIEM runtime configured"),
        "the Host must not boot with a half-configured KMS: {text}"
    );

    let zero = boot(
        temp.path(),
        "zerokey",
        &[("MAOS_KMS_MASTER_KEY", &"00".repeat(32))],
    );
    assert_refused(&zero, "kms_master_key_invalid");

    // A valid key still boots with the KMS on.
    let good = boot(
        temp.path(),
        "goodkey",
        &[("MAOS_KMS_MASTER_KEY", &"42".repeat(32))],
    );
    let text = stderr(&good);
    assert!(good.status.success(), "{text}");
    assert!(text.contains("kms=true"), "{text}");
}

#[test]
fn without_any_kms_variable_the_default_is_unchanged() {
    let temp = tempfile::tempdir().expect("fixture");
    let output = boot(temp.path(), "plain", &[]);
    let text = stderr(&output);
    assert!(output.status.success(), "{text}");
    assert!(
        text.contains("enterprise identity/at-rest/SIEM NOT configured"),
        "{text}"
    );
}

#[cfg(feature = "online-adapters")]
#[test]
fn an_unreachable_endpoint_is_not_a_boot_refusal() {
    let temp = tempfile::tempdir().expect("fixture");
    let url = format!("http://127.0.0.1:{}", dead_port());
    let output = boot(temp.path(), "down", &[("MAOS_KMS_ENDPOINT", &url)]);
    let text = stderr(&output);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("kms=true"), "{text}");
    assert!(
        text.contains("external KMS endpoint UNREACHABLE at boot"),
        "{text}"
    );
}

// ── AC8 — encrypted-file over the endpoint ──────────────────────────────────

#[cfg(all(feature = "online-adapters", unix))]
#[test]
fn encrypted_file_works_over_the_endpoint_and_a_down_endpoint_never_falls_back_to_the_environment()
{
    use kms_fixture::KmsFixture;
    use maos_domain::ports::{SecretKey, SecretStore};
    use maos_secrets::http_kms::{HttpKms, MonotonicClock, TimedHttp};

    const BEARER: &str = "kms-boot-bearer-canary-31d7";
    const CREDENTIAL: &str = "sk-ant-sealed-through-the-kms";

    let temp = tempfile::tempdir().expect("fixture");
    let root = temp.path();
    let mut fixture = KmsFixture::start();

    // Seal one credential through the endpoint, into the directory the booted
    // Host reads (`$MAOS_HOME/secrets`).
    let seal_home = root.join("maos-home-sealed");
    let adapter = Arc::new(
        HttpKms::new(
            Arc::new(TimedHttp::default()),
            &fixture.url(),
            Some(BEARER.to_owned()),
            Arc::new(MonotonicClock::new()),
        )
        .expect("adapter"),
    );
    assert!(adapter.probe());
    maos_secrets::EncryptedFileSecretStore::new(
        seal_home.join("secrets"),
        adapter,
        Arc::new(RingCryptoProvider),
    )
    .put(SecretKey::AnthropicApiKey, CREDENTIAL)
    .expect("seal the credential");

    let vars = [
        ("MAOS_SECRETS_BACKEND", "encrypted-file"),
        ("MAOS_KMS_BEARER", BEARER),
    ];
    let url = fixture.url();
    let mut up_vars = vars.to_vec();
    up_vars.push(("MAOS_KMS_ENDPOINT", &url));
    // The boot helper makes `maos-home-<name>`: name the sealed home.
    let up = boot(root, "sealed", &up_vars);
    let text = stderr(&up);
    assert!(up.status.success(), "{text}");
    assert!(
        text.contains("external KMS bearer read"),
        "the boot says the bearer was settled: {text}"
    );
    assert!(
        text.contains("maos: Anthropic provider registered"),
        "the credential must open through /unwrap: {text}"
    );
    assert!(
        !text.contains(BEARER) && !text.contains(CREDENTIAL),
        "{text}"
    );
    let unwraps: Vec<_> = fixture
        .requests()
        .into_iter()
        .filter(|r| r.path == "/unwrap")
        .collect();
    assert!(!unwraps.is_empty(), "the read went through POST /unwrap");
    assert!(unwraps
        .iter()
        .all(|r| r.authorization.as_deref() == Some(format!("Bearer {BEARER}").as_str())));

    // The endpoint down: the read fails; the environment is never consulted.
    fixture.stop();
    let mut down_vars = vars.to_vec();
    down_vars.push(("MAOS_KMS_ENDPOINT", &url));
    down_vars.push(("MAOS_ANTHROPIC_API_KEY", "sk-ant-environment-fallback"));
    let down = boot(root, "sealed", &down_vars);
    let text = stderr(&down);
    assert!(
        !text.contains("maos: Anthropic provider registered"),
        "no provider may register from the environment fallback: {text}"
    );
    // The absence above must not pass on an unrelated early failure: this
    // boot got through the KMS boot probe (which found the endpoint down),
    // the secret store and provider resolution, to the inference port.
    assert!(
        text.contains("external KMS endpoint UNREACHABLE at boot"),
        "the second boot must have probed the dead endpoint: {text}"
    );
    assert!(
        text.contains("maos: ProviderRateLimiter initialized"),
        "the second boot must have passed provider resolution: {text}"
    );
    assert!(!text.contains("sk-ant-environment-fallback") && !text.contains(CREDENTIAL));
}
