#![forbid(unsafe_code)]

//! Story 18-1 AC1 (CR-1; D-18-1-02/03/04 A) — exit line 1 at process level.
//!
//! `maos run spirits/butler/manifest.toml --once` on a keyless runner (fresh
//! `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, no provider key, `MAOS_OLLAMA_URL`
//! unset), `MAOS_INFERENCE_MODE=replay`, `MAOS_REPLAY_STRICT=1`:
//!
//! - the re-authored seed cassette → exit 0; the emitted notification's
//!   `pattern`/`confidence`/`evidence`/`options` equal the completion's and
//!   the `belief_variance` halt fires as today;
//! - a completion missing `options` → non-zero, naming the field (the AC1
//!   plant: the log-only shape check turns this case green → red);
//! - an empty `entries` cassette (exhaustion) → non-zero;
//! - a drifted prompt hash under strict → non-zero;
//! - unset mode (deterministic) → exit 0 and `options` emitted;
//! - issuance refuses a provider Butler does not declare (17-11).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::security::manifest::CapabilitiesRequired;
use maos_kernel_core::security::RingCryptoProvider;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;

const SEED_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn seed_cassette() -> serde_json::Value {
    let text = std::fs::read_to_string(workspace_root().join(SEED_CASSETTE)).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The completion the seed cassette replays, as a JSON object.
fn seed_completion() -> serde_json::Value {
    let text = seed_cassette()["entries"][0]["response"]["text"]
        .as_str()
        .expect("response.text is a string")
        .to_owned();
    serde_json::from_str(&text).expect("the seed completion is the D-18-1-03 JSON contract")
}

/// Write a variant of the seed cassette into `dir`.
fn cassette_variant(dir: &Path, name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> PathBuf {
    let mut cassette = seed_cassette();
    edit(&mut cassette);
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec_pretty(&cassette).unwrap()).unwrap();
    path
}

/// `maos run spirits/butler/manifest.toml --once` on a keyless runner.
fn run_butler_once(home: &Path, replay: Option<&Path>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.args(["run", "spirits/butler/manifest.toml", "--once"])
        .current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home.join("xdg"));
    if let Some(cassette) = replay {
        cmd.env("MAOS_INFERENCE_MODE", "replay")
            .env("MAOS_REPLAY_STRICT", "1")
            .env("MAOS_REPLAY_CASSETTE", cassette);
    }
    cmd.output().expect("spawn maos run")
}

fn events(output: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn event<'a>(events: &'a [serde_json::Value], name: &str) -> Option<&'a serde_json::Value> {
    events.iter().find(|e| e["event"] == name)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn exit_line_1_replays_the_proposal_through_the_inference_port() {
    let home = tempfile::tempdir().unwrap();
    let cassette = workspace_root().join(SEED_CASSETTE);
    let output = run_butler_once(home.path(), Some(&cassette));
    assert!(
        output.status.success(),
        "exit line 1 must exit 0; stderr:\n{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).contains("butler inference seam wired"),
        "the Butler arm must bind the Inference Port under explicit replay; stderr:\n{}",
        stderr(&output)
    );

    let events = events(&output);
    let notification = &event(&events, "butler_notification").expect("a butler_notification event")
        ["notification"];
    let completion = seed_completion();
    for field in ["pattern", "evidence", "options"] {
        assert_eq!(
            notification[field], completion[field],
            "`{field}` must be the completion's"
        );
    }
    let confidence = notification["confidence"].as_f64().unwrap();
    assert!(
        (confidence - completion["confidence"].as_f64().unwrap()).abs() < 1e-6,
        "`confidence` must be the completion's"
    );
    let halt = event(&events, "halt").expect("the belief_variance halt fires as today");
    assert_eq!(
        halt["render"],
        butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE)
    );
}

#[test]
fn a_completion_missing_options_exits_non_zero_naming_the_field() {
    let home = tempfile::tempdir().unwrap();
    let cassette = cassette_variant(home.path(), "missing-options.json", |c| {
        let mut completion = seed_completion();
        completion.as_object_mut().unwrap().remove("options");
        c["entries"][0]["response"]["text"] = completion.to_string().into();
    });
    let output = run_butler_once(home.path(), Some(&cassette));
    assert!(
        !output.status.success(),
        "a completion missing `options` must exit non-zero; stderr:\n{}",
        stderr(&output)
    );
    let err = stderr(&output);
    assert!(
        err.contains("pass refused") && err.contains("options"),
        "the refusal must be typed and name the field; stderr:\n{err}"
    );
    assert!(event(&events(&output), "butler_notification").is_none());
}

#[test]
fn cassette_exhaustion_exits_non_zero() {
    let home = tempfile::tempdir().unwrap();
    let cassette = cassette_variant(home.path(), "empty.json", |c| {
        c["entries"] = serde_json::json!([]);
    });
    let output = run_butler_once(home.path(), Some(&cassette));
    assert!(
        !output.status.success(),
        "exhaustion must exit non-zero; stderr:\n{}",
        stderr(&output)
    );
    let err = stderr(&output);
    assert!(
        err.contains("pass refused") && err.contains("exhausted"),
        "stderr:\n{err}"
    );
}

#[test]
fn strict_prompt_drift_exits_non_zero() {
    let home = tempfile::tempdir().unwrap();
    let cassette = cassette_variant(home.path(), "drifted.json", |c| {
        c["entries"][0]["prompt_sha256"] = "ab".repeat(32).into();
        c["entries"][0]["prompt_len"] = 7.into();
    });
    let output = run_butler_once(home.path(), Some(&cassette));
    assert!(
        !output.status.success(),
        "strict drift must exit non-zero; stderr:\n{}",
        stderr(&output)
    );
    let err = stderr(&output);
    assert!(
        err.contains("pass refused") && err.contains("drift"),
        "stderr:\n{err}"
    );
}

#[test]
fn unset_mode_stays_deterministic_and_emits_options() {
    let home = tempfile::tempdir().unwrap();
    let output = run_butler_once(home.path(), None);
    assert!(
        output.status.success(),
        "the deterministic path must stay green; stderr:\n{}",
        stderr(&output)
    );
    assert!(
        !stderr(&output).contains("butler inference seam wired"),
        "unset mode never binds the Inference Port (ADR-064)"
    );
    let events = events(&output);
    let notification = &event(&events, "butler_notification").expect("a butler_notification event")
        ["notification"];
    assert_eq!(
        notification["options"],
        serde_json::json!(butler::DETERMINISTIC_OPTIONS)
    );
}

/// A 17-11 regression guard (review AA-8/TIA-11): issuance refuses a provider
/// Butler does not declare with the typed policy denial, not just any error.
#[test]
fn issuance_refuses_a_provider_butler_does_not_declare() {
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(workspace_root().join("spirits/butler/manifest.toml")).unwrap(),
    )
    .unwrap();
    let required = toml::to_string(&manifest["capabilities"]["required"]).unwrap();
    let scopes = maos_kernel_core::security::capabilities_required_to_scopes(
        &CapabilitiesRequired::from_toml_str(&required).unwrap(),
    );
    assert!(
        scopes.contains(&Scope::ProviderInfer {
            provider: "anthropic".into()
        }),
        "Butler declares `anthropic` (manifest.toml:21)"
    );

    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        9,
        ManifestCapabilityScope {
            scopes,
            declared_tier: SandboxTier(0),
            trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(inner);
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x18; 32]),
        0x18_01,
        policy,
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    );
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let mint = |provider: &str| {
        capability.issue_with_mediation(
            9,
            Scope::ProviderInfer {
                provider: provider.into(),
            },
            60,
            [0; 32],
            IntentClass::Standard,
        )
    };
    assert!(mint("anthropic").is_ok(), "the declared provider mints");
    for undeclared in ["openai", "ollama"] {
        assert!(
            matches!(
                mint(undeclared),
                Err(maos_domain::ports::capability::CapError::PolicyDenied)
            ),
            "`{undeclared}` is not declared and must be refused at issuance, typed"
        );
    }
}

/// A bound Butler (a no-op port: the verdict never calls it).
fn bound_butler() -> butler::Butler {
    struct Unused;
    impl maos_domain::ports::InferencePort for Unused {
        fn complete(
            &self,
            _: maos_domain::ports::inference::InferenceRequest,
        ) -> Result<
            maos_domain::ports::inference::InferenceResponse,
            maos_domain::ports::inference::InferenceError,
        > {
            unreachable!("the verdict never infers")
        }
    }
    butler::Butler::new()
        .with_deferred_inference_port(Arc::new(Unused), Arc::new(std::sync::Mutex::new(None)))
}

/// Review BH-2/EC-2: under `--once` a BOUND pass the dispatcher did not run to
/// completion (budget exceeded, panicked, warned-but-not-fired) is a refusal,
/// even though the pass recorded no error and wrote no output (its blocking
/// thread may still be running). The deterministic path is unchanged.
#[test]
fn an_incomplete_bound_pass_is_a_refusal() {
    use maos_kernel_core::scheduler::HookOutcome;
    let predicate = maos_kernel_core::security::OutputShapePredicate::from(
        &maos_kernel_core::security::manifest::OutputShape::from_toml_str(
            "required_fields = [\"pattern\"]",
        )
        .unwrap(),
    );
    let verdict = |butler: &butler::Butler, outcome: HookOutcome| {
        maos_bin::butler_root::pass_verdict(Some(butler), &outcome, None, &predicate)
    };
    let bound = bound_butler();
    for outcome in [
        HookOutcome::BudgetExceeded {
            wall_ns: 31_000_000_000,
            cap_seconds: 30,
        },
        HookOutcome::Panicked {
            panic_payload_preview: "boom".into(),
        },
        HookOutcome::BudgetWarning80 {
            wall_ns: 25_000_000_000,
            cap_seconds: 30,
            fired: false,
        },
    ] {
        let err = verdict(&bound, outcome.clone()).expect_err("an incomplete bound pass refuses");
        assert!(err.contains("did not complete"), "{err}");
    }
    for outcome in [
        HookOutcome::Fired { wall_ns: 1 },
        HookOutcome::BudgetWarning80 {
            wall_ns: 25_000_000_000,
            cap_seconds: 30,
            fired: true,
        },
    ] {
        assert_eq!(verdict(&bound, outcome), Ok(()));
    }
    let unbound = butler::Butler::new();
    assert_eq!(
        verdict(
            &unbound,
            HookOutcome::BudgetExceeded {
                wall_ns: 31_000_000_000,
                cap_seconds: 30
            }
        ),
        Ok(()),
        "the deterministic path keeps its verdict"
    );
}
