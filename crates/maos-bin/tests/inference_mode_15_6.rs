#![cfg(feature = "network")]

use std::path::PathBuf;

use maos_bin::inference_mode::{InferenceModeError, ResolvedInferenceMode};
use sha2::{Digest, Sha256};

fn cassette() -> PathBuf {
    PathBuf::from("fixtures/inference.json")
}

fn cassette_fingerprint(path: &std::path::Path) -> u64 {
    let digest = Sha256::digest(path.as_os_str().as_encoded_bytes());
    u64::from_be_bytes(digest[..8].try_into().unwrap())
}

#[test]
fn unset_mode_preserves_existing_precedence() {
    assert_eq!(
        ResolvedInferenceMode::resolve(None, Some(cassette()), true, true, false).unwrap(),
        ResolvedInferenceMode::Live { explicit: false }
    );
    assert_eq!(
        ResolvedInferenceMode::resolve(None, Some(cassette()), true, false, false).unwrap(),
        ResolvedInferenceMode::Replay {
            cassette: cassette(),
            strict: true,
            explicit: false,
        }
    );
    assert_eq!(
        ResolvedInferenceMode::resolve(None, None, false, false, false).unwrap(),
        ResolvedInferenceMode::Deterministic
    );
}

#[test]
fn explicit_mode_is_authoritative() {
    assert_eq!(
        ResolvedInferenceMode::resolve(Some("live"), Some(cassette()), true, false, true).unwrap(),
        ResolvedInferenceMode::Live { explicit: true }
    );
    assert_eq!(
        ResolvedInferenceMode::resolve(Some("record"), Some(cassette()), true, false, false)
            .unwrap(),
        ResolvedInferenceMode::Record {
            cassette: cassette(),
        }
    );
    assert_eq!(
        ResolvedInferenceMode::resolve(Some("replay"), Some(cassette()), true, false, false)
            .unwrap(),
        ResolvedInferenceMode::Replay {
            cassette: cassette(),
            strict: true,
            explicit: true,
        }
    );
}

#[test]
fn invalid_combinations_fail_with_typed_configuration_errors() {
    assert!(matches!(
        ResolvedInferenceMode::resolve(Some("bogus"), None, false, false, false),
        Err(InferenceModeError::InvalidMode { value }) if value == "bogus"
    ));
    assert!(matches!(
        ResolvedInferenceMode::resolve(Some("replay"), None, false, false, false),
        Err(InferenceModeError::CassetteRequired { mode: "replay" })
    ));
    assert!(matches!(
        ResolvedInferenceMode::resolve(Some("record"), None, false, false, false),
        Err(InferenceModeError::CassetteRequired { mode: "record" })
    ));
    assert!(matches!(
        ResolvedInferenceMode::resolve(Some("replay"), Some(cassette()), false, true, false),
        Err(InferenceModeError::ReplayLiveConflict)
    ));
    assert!(matches!(
        ResolvedInferenceMode::resolve(Some("live"), None, false, false, false),
        Err(InferenceModeError::LiveProviderUnavailable)
    ));
}

#[test]
fn replay_llm_flag_is_retired() {
    let error = maos_bin::worker_spawn::parse_run_args(
        ["run", "researcher", "--replay-llm"]
            .into_iter()
            .map(str::to_owned),
    )
    .unwrap_err();
    assert!(error.contains("unknown argument '--replay-llm'"));
}

#[test]
fn explicit_replay_is_observable_before_an_empty_shell_exits() {
    use std::io::Write as _;
    use std::process::Stdio;

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let home = tempfile::tempdir().unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_maos"))
        .arg("shell")
        .current_dir(&root)
        .env_clear()
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("xdg"))
        .env("MAOS_INFERENCE_MODE", "replay")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root.join("crates/maos-journey-test/cassettes/j0/shell-intro.json"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"@hello-spirit hi\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "replay shell should exit 0; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("maos shell: MAOS_INFERENCE_MODE=replay"),
        "empty-input shell must expose the selected replay mode"
    );
    // Review P8: the banner alone proves nothing — the replayed turn must
    // print the cassette's own text, never the transport-error fallback.
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Hello! I am the MAOS hello-spirit reference implementation"),
        "replayed shell must print the cassette text; output:\n{combined}"
    );
    assert!(
        !combined.contains("Inference transport error"),
        "replayed shell must not fall back to the transport-error text; output:\n{combined}"
    );
}

use std::sync::Arc;

use maos_bin::cassette_replay::{
    CassetteRecordProvider, CassetteRecorder, CassetteReplayProvider, FlushOutcome,
};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope, TokenId};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::TokenIssuer;
use maos_domain::ports::inference::{
    InferenceOptions, InferencePort, InferenceRequest, InferenceResponse, ProviderAttribution,
    StopReason, TokenUsage,
};
use maos_domain::ports::CapabilityRegistryPort;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_quota::CapQuotaTracker;
use maos_kernel_core::capability::cap_tokens::Ed25519SigningKey;
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::security::crypto::RingCryptoProvider;
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_providers::provider::{Provider, ProviderError};

fn request(prompt: &str) -> InferenceRequest {
    InferenceRequest::new(
        7,
        CapabilityToken::new(TokenId([0; 16]), 0, u64::MAX, [0; 64]),
        prompt.to_owned(),
        InferenceOptions::default(),
        None,
        Vec::new(),
    )
}

fn response(text: &str) -> InferenceResponse {
    InferenceResponse {
        text: text.to_owned(),
        stop_reason: StopReason::ProviderStop("complete".into()),
        usage: TokenUsage {
            input_tokens: 11,
            output_tokens: 13,
        },
        provider_attribution: ProviderAttribution {
            provider_id: "anthropic".into(),
            endpoint_url: "https://provider.test".into(),
            model_id: Some("fixture-model".into()),
        },
    }
}

struct FixedProvider;

impl Provider for FixedProvider {
    fn complete(&self, _req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        Ok(response("recorded response"))
    }

    fn credential_fingerprint(&self) -> u64 {
        42
    }
}

#[test]
fn replay_provider_preserves_response_and_exhausts_strictly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("replay.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "entries": [{
                "sequence": 0,
                "prompt_sha256": "cf07194ee232eb531e15f690000d19846dea69cf05504782658afcfacb9228a2",
                "prompt_len": 6,
                "response": {
                    "text": "replayed response",
                    "stop_reason": "complete",
                    "usage": {"input_tokens": 11, "output_tokens": 13},
                    "provider_attribution": {
                        "provider_id": "anthropic",
                        "endpoint_url": "https://provider.test",
                        "model_id": "fixture-model"
                    }
                }
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let provider = CassetteReplayProvider::from_file(&path, true).unwrap();
    assert_eq!(
        provider.credential_fingerprint(),
        cassette_fingerprint(&path)
    );
    let replayed = provider.complete(&request("prompt")).unwrap();
    assert_eq!(replayed, response("replayed response"));
    assert!(matches!(
        provider.complete(&request("prompt")),
        // Review D2: exhaustion surfaces as Serde → MalformedResponse, which
        // maos-spirit-hello propagates instead of converting to a success.
        Err(ProviderError::Serde(message)) if message.contains("exhausted")
    ));
}

#[test]
fn record_provider_flushes_complete_v1_entry_to_requested_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recorded.json");
    let recorder = Arc::new(CassetteRecorder::new(
        path.clone(),
        "researcher".into(),
        "test-session".into(),
    ));
    let provider = CassetteRecordProvider::new(Arc::new(FixedProvider), Arc::clone(&recorder));
    assert_eq!(
        provider.credential_fingerprint(),
        cassette_fingerprint(&path)
    );

    assert_eq!(
        provider.complete(&request("prompt")).unwrap(),
        response("recorded response")
    );
    assert_eq!(
        recorder.flush().unwrap(),
        FlushOutcome::Written { entries: 1 }
    );
    assert_eq!(
        recorder.flush().unwrap(),
        FlushOutcome::AlreadyFlushed { entries: 1 }
    );

    let cassette: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(cassette["schema_version"], "maos.journey.cassette/v1");
    assert_eq!(cassette["provenance"], "live-record");
    assert_eq!(cassette["entries"][0]["sequence"], 0);
    assert_eq!(cassette["entries"][0]["prompt_len"], 6);
    assert_eq!(
        cassette["entries"][0]["prompt_sha256"],
        "cf07194ee232eb531e15f690000d19846dea69cf05504782658afcfacb9228a2"
    );
    assert_eq!(
        cassette["entries"][0]["response"]["text"],
        "recorded response"
    );
    assert_eq!(
        cassette["entries"][0]["response"]["provider_attribution"]["provider_id"],
        "anthropic"
    );
    let replay = CassetteReplayProvider::from_file(&path, true).unwrap();
    assert_eq!(
        replay.complete(&request("prompt")).unwrap(),
        response("recorded response")
    );
}

#[test]
fn replay_reader_is_lenient_only_for_absent_provenance() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("provenance.json");
    let write_cassette = |provenance: Option<serde_json::Value>| {
        let mut cassette = serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "entries": [],
        });
        if let Some(provenance) = provenance {
            cassette["provenance"] = provenance;
        }
        std::fs::write(&path, serde_json::to_vec(&cassette).unwrap()).unwrap();
    };

    write_cassette(None);
    assert!(CassetteReplayProvider::from_file(&path, true).is_ok());
    for legal in ["seed", "live-record"] {
        write_cassette(Some(legal.into()));
        assert!(CassetteReplayProvider::from_file(&path, true).is_ok());
    }
    for illegal in [
        serde_json::Value::String(String::new()),
        serde_json::Value::String("imported".into()),
        serde_json::Value::Null,
    ] {
        write_cassette(Some(illegal));
        let error = CassetteReplayProvider::from_file(&path, true)
            .err()
            .expect("illegal provenance must be rejected");
        assert!(error.contains("provenance"), "unexpected error: {error}");
    }
}

#[test]
fn replay_provider_keeps_router_keys_and_kernel_mediation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mediated-replay.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "maos.journey.cassette/v1",
            "entries": [{
                "sequence": 0,
                "prompt_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                "prompt_len": 0,
                "response": {
                    "text": "mediated replay",
                    "stop_reason": "stop_sequence",
                    "usage": {"input_tokens": 3, "output_tokens": 5},
                    "provider_attribution": {
                        "provider_id": "openai",
                        "endpoint_url": "cassette://mediated",
                        "model_id": "fixture"
                    }
                }
            }]
        }))
        .unwrap(),
    )
    .unwrap();

    let replay: Arc<dyn Provider> =
        Arc::new(CassetteReplayProvider::from_file(&path, true).unwrap());
    let mut providers = std::collections::BTreeMap::new();
    for key in ["anthropic", "ollama", "openai"] {
        providers.insert(key.to_owned(), Arc::clone(&replay));
    }
    let router = Arc::new(MultiProviderRouter::new(
        providers,
        Some("anthropic".into()),
    ));
    assert_eq!(
        router.registered_ids(),
        vec!["anthropic", "ollama", "openai"]
    );
    assert_eq!(router.default_id(), Some("anthropic"));

    let policy = Arc::new(PolicyTable::new());
    let mut policy_inner = PolicyTableInner::default();
    policy_inner.manifest_scopes.insert(
        7,
        ManifestCapabilityScope {
            scopes: vec![Scope::ProviderInfer {
                provider: "openai".into(),
            }],
            declared_tier: SandboxTier(0),
            trust_tier: maos_kernel_core::capability::cap_policy::decision::TrustTier::Verified,
        },
    );
    policy.update(policy_inner);
    let (audit_tx, _audit_rx) = maos_kernel_core::capability::cap_audit::channel();
    let capability = Arc::new(CapabilityRegistryAdapter::new(
        Arc::new(RingCryptoProvider),
        Ed25519SigningKey::new([0x56; 32]),
        0x15_06,
        policy,
        audit_tx,
        CapQuotaTracker::new(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(TelemetryStreamAdapter::default()),
    ));
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let token = capability
        .issue_with_mediation(
            7,
            Scope::ProviderInfer {
                provider: "openai".into(),
            },
            60,
            [0; 32],
            IntentClass::Standard,
        )
        .unwrap();
    capability.verify(&token, [0; 32], SandboxTier(0)).unwrap();

    let transparency_log = Arc::new(TransparencyLogAdapter::open_in_memory(0x15_06));
    let adapter = InferencePortAdapter::new(
        router,
        capability,
        Arc::clone(&transparency_log),
        Arc::new(IacRtMetrics::new()),
    );
    let replayed = adapter
        .complete(InferenceRequest::new(
            7,
            token,
            "mediated prompt".into(),
            InferenceOptions::default(),
            Some("openai".into()),
            Vec::new(),
        ))
        .unwrap();
    assert_eq!(replayed.text, "mediated replay");
    let rows = transparency_log
        .query_frames(FrameFilter {
            kind: Some(FrameKind::InferenceCall),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn empty_recording_flush_is_a_loud_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.json");
    let recorder = CassetteRecorder::new(path.clone(), "researcher".into(), "empty".into());

    let error = recorder.flush().unwrap_err();
    assert!(error.contains("zero successful inference responses"));
    assert!(!path.exists());
}

#[test]
fn record_with_zero_completions_exits_non_zero() {
    // Review P6 (D-15-6-J falsifier): pins the composition-root flush()
    // call itself. `Drop` cannot set the exit code, so deleting the
    // drain-point call makes an empty record run exit 0 — this test reds.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let home = tempfile::tempdir().unwrap();
    let cassette = home.path().join("empty-record.json");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_maos"))
        .arg("shell")
        .current_dir(&root)
        .env_clear()
        .env("HOME", home.path())
        .env("MAOS_HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("xdg"))
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_INFERENCE_MODE", "record")
        .env("MAOS_REPLAY_CASSETTE", &cassette)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "record with zero completions must exit non-zero (D-15-6-K); stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("zero successful inference responses"),
        "the refusal must name its cause; stderr:\n{stderr}"
    );
    assert!(
        !cassette.exists(),
        "a refused record run must not write a cassette"
    );
}

#[test]
fn rate_limiter_polarity_is_mode_scoped() {
    // AC3 / review P9: replay is the only limiter-exempt mode — record and
    // live make real provider calls on the paid path and stay throttled,
    // and the unset compatibility modes keep HEAD's throttled behavior.
    let explicit_replay =
        ResolvedInferenceMode::resolve(Some("replay"), Some(cassette()), false, false, false)
            .unwrap();
    assert!(!explicit_replay.uses_rate_limiter());

    let record =
        ResolvedInferenceMode::resolve(Some("record"), Some(cassette()), false, false, false)
            .unwrap();
    assert!(record.uses_rate_limiter());

    let live =
        ResolvedInferenceMode::resolve(Some("live"), Some(cassette()), false, false, true)
            .unwrap();
    assert!(live.uses_rate_limiter());

    let unset_live = ResolvedInferenceMode::resolve(None, Some(cassette()), false, true, false)
        .unwrap();
    assert!(unset_live.uses_rate_limiter());
    let unset_replay = ResolvedInferenceMode::resolve(None, Some(cassette()), true, false, false)
        .unwrap();
    assert!(unset_replay.uses_rate_limiter());
    let deterministic = ResolvedInferenceMode::resolve(None, None, false, false, false).unwrap();
    assert!(deterministic.uses_rate_limiter());
}
#[test]
fn invalid_mode_inputs_are_refused_before_any_inference_consumer_is_wired() {
    let cases: &[(&str, &[&str], &[(&str, &str)], &str)] = &[
        (
            "unsupported",
            &["run", "researcher", "--once"],
            &[("MAOS_INFERENCE_MODE", "bogus")],
            "MAOS_INFERENCE_MODE has unsupported value 'bogus'",
        ),
        (
            "missing-cassette",
            &["run", "researcher", "--once"],
            &[("MAOS_INFERENCE_MODE", "replay")],
            "MAOS_INFERENCE_MODE=replay requires MAOS_REPLAY_CASSETTE",
        ),
        (
            "replay-live-conflict",
            &["run", "researcher", "--live", "--once"],
            &[
                ("MAOS_INFERENCE_MODE", "replay"),
                ("MAOS_REPLAY_CASSETTE", "unused.json"),
            ],
            "MAOS_INFERENCE_MODE=replay conflicts with --live",
        ),
        (
            "live-unconfigured",
            &["run", "researcher", "--once"],
            &[("MAOS_INFERENCE_MODE", "live")],
            "MAOS_INFERENCE_MODE=live requires an explicitly configured inference provider",
        ),
    ];

    for (name, args, envs, expected) in cases {
        let home = tempfile::Builder::new()
            .prefix(&format!("maos-15-6-{name}-"))
            .tempdir()
            .unwrap();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_maos"))
            .args(*args)
            .current_dir(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap(),
            )
            .env_clear()
            .env("HOME", home.path())
            .env("MAOS_HOME", home.path())
            .env("XDG_DATA_HOME", home.path().join("xdg"))
            .env("MAOS_OLLAMA_URL", "skip")
            .envs(envs.iter().copied())
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{name} unexpectedly succeeded");
        assert!(
            stderr.contains(expected),
            "{name} did not name its configuration error; stderr={stderr}"
        );
        for forbidden in [
            "Inference Port initialized",
            "researcher live-inference seam wired",
            "researcher cassette-replay inference wired",
            "butler live MCP port wired",
        ] {
            assert!(
                !stderr.contains(forbidden),
                "{name} reached inference consumer '{forbidden}' before refusal"
            );
        }
    }
}
