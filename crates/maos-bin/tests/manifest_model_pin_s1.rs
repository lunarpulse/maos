#![forbid(unsafe_code)]
//! S1 (operator ruling Q6, 2026-11-13) — "manifest pin overrides the code base
//! or profile file": the model a Spirit's manifest pins reaches the wire.
//!
//! Real path, local fake only: the repo's own manifests → `admission` →
//! `researcher_root` / `orchestrator_root` / `ButlerSeam::attach` /
//! `BoundInference` → the kernel `InferencePortAdapter` → the admitted-model
//! gate → the real `AnthropicProvider` over the real `IoSubsystemAdapter` → a
//! loopback server that records every request body. No paid API call.
//!
//! The configured driver default is `claude-haiku-4-5`, so a pin is told apart
//! from "the provider default" on the wire.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use maos_bin::admission::{
    admitted_models, check_declared_pins, check_provider_pins, declared_pins, first_declared_pin,
    gate_manifest, pin_for, AdmissionRefusal, ConfiguredProvider, GatedManifest, ProviderCatalog,
    KNOWN_CLASS_NAMES,
};
use maos_bin::shell_host::BoundInference;
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::{EpistemicScalarPort, InferencePort, ScalarPortError};
use maos_eval::distill_replay::{judge_request, JudgeConfig};
use maos_kernel_core::api::IoSubsystemAdapter;
use maos_kernel_core::capability::cap_policy::decision::TrustTier;
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens::{init_monotonic_base, Ed25519SigningKey};
use maos_kernel_core::capability::{CapabilityRegistryAdapter, WorkingMemoryStore};
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::log_recall::LogRecallAdapter;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::inference::router::MultiProviderRouter;
use maos_kernel_core::inference::InferencePortAdapter;
use maos_kernel_core::security::{capabilities_required_to_scopes, RingCryptoProvider};
use maos_kernel_core::telemetry::iac_rt::IacRtMetrics;
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use maos_providers::{AdmittedModels, AnthropicProvider, Provider};
use maos_spirit_abi::ctx::{CapabilityHandle, Ctx, MailboxHandle};
use maos_spirit_abi::lifecycle::Spirit;
use researcher::{RecalledFrame, Researcher};
use serde_json::{json, Value};

const PID: u32 = 21;
const DEFAULT: &str = "claude-haiku-4-5";
const KEY: &str = "sk-ant-api03-s1-fixture";
const RESEARCHER: &str = include_str!("../../../spirits/researcher/manifest.toml");
const BUTLER: &str = include_str!("../../../spirits/butler/manifest.toml");
const ORCHESTRATOR: &str = include_str!("../../../spirits/orchestrator/manifest.toml");

fn gate(manifest: &str) -> GatedManifest {
    let known = |name: &str| KNOWN_CLASS_NAMES.contains(&name) || name == "hello-spirit";
    gate_manifest(manifest, &known).expect("a gated manifest")
}

/// A loopback Anthropic that answers every request with `text` and keeps the
/// JSON bodies it was sent.
fn fake_anthropic(text: &'static str) -> (String, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let bodies = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&bodies);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let seen = Arc::clone(&seen);
            thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim_end().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        length = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0u8; length];
                reader.read_exact(&mut body).unwrap();
                seen.lock()
                    .unwrap()
                    .push(serde_json::from_slice(&body).expect("a JSON request body"));
                let reply = json!({
                    "content": [{ "type": "text", "text": text }],
                    "stop_reason": "end_turn",
                    "usage": { "input_tokens": 3, "output_tokens": 2 }
                })
                .to_string();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    reply.len()
                );
                let mut stream = stream;
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(reply.as_bytes());
            });
        }
    });
    (url, bodies)
}

/// The kernel world and the production driver stack in front of a fake.
struct Rig {
    port: Arc<InferencePortAdapter>,
    capability: Arc<CapabilityRegistryAdapter>,
    tl: Arc<TransparencyLogAdapter>,
    bodies: Arc<Mutex<Vec<Value>>>,
}

impl Rig {
    fn new(gated: &GatedManifest, text: &'static str) -> Self {
        init_monotonic_base();
        let (url, bodies) = fake_anthropic(text);
        let policy = Arc::new(PolicyTable::new());
        let mut admitted = PolicyTableInner::default();
        admitted.manifest_scopes.insert(
            PID,
            ManifestCapabilityScope {
                scopes: capabilities_required_to_scopes(&gated.caps_required),
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
        policy.update(admitted);
        let (audit_tx, audit_rx) = maos_kernel_core::capability::cap_audit::channel();
        std::mem::forget(audit_rx);
        let capability = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(RingCryptoProvider),
            Ed25519SigningKey::new([0x21; 32]),
            0x21,
            policy,
            audit_tx,
            maos_kernel_core::capability::cap_quota::CapQuotaTracker::new(),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x21));
        // The production stack: the driver behind the admitted-model gate.
        let driver = AnthropicProvider::with_api_key(
            Arc::new(IoSubsystemAdapter::new()),
            url,
            DEFAULT.into(),
            KEY.into(),
        )
        .unwrap();
        let admitted = admitted_models("anthropic", DEFAULT);
        let gate: Arc<dyn Provider> = Arc::new(AdmittedModels::new(driver, admitted));
        let router = Arc::new(MultiProviderRouter::new(
            BTreeMap::from([("anthropic".to_owned(), gate)]),
            Some("anthropic".into()),
        ));
        let port = Arc::new(InferencePortAdapter::new(
            router,
            Arc::clone(&capability),
            Arc::clone(&tl),
            Arc::new(IacRtMetrics::new()),
        ));
        Self {
            port,
            capability,
            tl,
            bodies,
        }
    }

    fn seam(&self) -> butler::ButlerSeam {
        let (port, capability) = (Arc::clone(&self.port), Arc::clone(&self.capability));
        butler::ButlerSeam {
            inference: Some((
                Arc::new(move || Some(Arc::clone(&port) as Arc<dyn InferencePort + Send + Sync>)),
                vec!["anthropic".into()],
            )),
            mint: Arc::new(move |pid, provider: &str| {
                let scope = Scope::ProviderInfer {
                    provider: provider.into(),
                };
                capability
                    .issue_with_mediation(pid, scope, 60, [0; 32], IntentClass::Standard)
                    .map_err(|e| format!("token issue failed: {e}"))
            }),
            digest: Arc::new(butler::MorningDigestRenderer::new(
                std::path::PathBuf::new(),
                std::path::PathBuf::new(),
                Arc::new(|_, _| {}),
                Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
            )),
            rollover: false,
            clock: Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
            acceptance: None,
            scene_butler: Arc::default(),
            context: None,
        }
    }

    fn token(&self) -> CapabilityToken {
        let scope = Scope::ProviderInfer {
            provider: "anthropic".into(),
        };
        self.capability
            .issue_with_mediation(PID, scope, 60, [0; 32], IntentClass::Standard)
            .expect("a token")
    }

    /// The `model` of every request body the fake saw.
    fn wire_models(&self) -> Vec<String> {
        let bodies = self.bodies.lock().unwrap();
        bodies
            .iter()
            .map(|body| body["model"].as_str().expect("a model").to_owned())
            .collect()
    }

    /// The model suffix of every `InferenceCall` Transparency Log intent.
    fn tl_models(&self) -> Vec<String> {
        let frames = self
            .tl
            .query_frames(FrameFilter {
                kind: Some(FrameKind::InferenceCall),
                ..Default::default()
            })
            .unwrap();
        frames
            .iter()
            .map(|f| f.intent.rsplit(':').next().unwrap().to_owned())
            .collect()
    }
}

fn pid() -> Arc<dyn Fn() -> Option<u32> + Send + Sync> {
    Arc::new(|| Some(PID))
}

fn survey_frames() -> Vec<RecalledFrame> {
    researcher::seed_claims()
        .iter()
        .enumerate()
        .map(|(i, claim)| RecalledFrame {
            frame_id: [i as u8 + 1; 16],
            intent: "researcher:corpus_claim".into(),
            payload: serde_json::to_vec(claim).unwrap(),
        })
        .collect()
}

fn research(gated: &GatedManifest) -> (Rig, Result<(), String>) {
    let rig = Rig::new(gated, "A digest.");
    let tl = Arc::clone(&rig.tl);
    let host = maos_bin::researcher_root::ResearcherHost::new(
        Arc::clone(&tl),
        Arc::new(DistillateWriter::new(Arc::clone(&tl), Arc::new(0u8))),
        false,
        Arc::new(LogRecallAdapter::new(Arc::clone(&tl))),
    );
    let bound = maos_bin::researcher_root::attach(
        Researcher::new(),
        &rig.seam(),
        &declared_pins(gated),
        pid(),
        &host,
    )
    .expect("the Researcher binds its declared provider");
    let result = bound
        .infer_survey(&survey_frames())
        .map(drop)
        .map_err(|e| e.to_string());
    (rig, result)
}

#[test]
fn the_researchers_sonnet_pin_reaches_the_wire_not_the_provider_default() {
    let gated = gate(RESEARCHER);
    assert_eq!(
        declared_pins(&gated),
        ["anthropic.claude-sonnet-5-5", "openai.gpt-4o-mini"]
    );
    let (rig, result) = research(&gated);
    result.expect("the survey completes");
    let wire = rig.wire_models();
    assert_eq!(wire.len(), researcher::seed_claims().len());
    assert!(wire.iter().all(|m| m == "claude-sonnet-5-5"), "{wire:?}");
    // The Transparency Log records the model the vendor call was served by.
    let tl = rig.tl_models();
    assert_eq!(tl.len(), wire.len());
    assert!(tl.iter().all(|m| m == "claude-sonnet-5-5"), "{tl:?}");
}

#[test]
fn a_haiku_pinned_butler_carries_haiku_and_a_provider_only_orchestrator_the_default() {
    // Butler: `anthropic.claude-haiku-5-5`, which is NOT the driver default.
    let gated = gate(BUTLER);
    let rig = Rig::new(&gated, "{}");
    let butler = rig
        .seam()
        .attach(
            butler::Butler::with_scenario(butler::ScenarioInput {
                calendar: vec![
                    butler::CalendarEvent {
                        id: "a".into(),
                        title: "a".into(),
                        start_min: 540,
                        end_min: 600,
                        status: butler::EventStatus::Confirmed,
                    },
                    butler::CalendarEvent {
                        id: "b".into(),
                        title: "b".into(),
                        start_min: 570,
                        end_min: 630,
                        status: butler::EventStatus::Confirmed,
                    },
                ],
                ..Default::default()
            }),
            || first_declared_pin(&gated),
            pid(),
        )
        .expect("the Butler binds");
    butler.on_idle(&mut Ctx::for_rust_inproc_hook(
        CapabilityHandle(0),
        MailboxHandle(0),
    ));
    assert_eq!(rig.wire_models(), ["claude-haiku-5-5"]);
    assert_eq!(rig.tl_models(), ["claude-haiku-5-5"]);

    // Orchestrator declared provider-only: served the provider default.
    let provider_only = ORCHESTRATOR.replace(
        r#"provider.complete = ["anthropic.claude-haiku-5-5", "openai.gpt-4o-mini"]"#,
        r#"provider.complete = ["anthropic"]"#,
    );
    assert_ne!(provider_only, ORCHESTRATOR, "the manifest line moved");
    let gated = gate(&provider_only);
    assert_eq!(declared_pins(&gated), ["anthropic"]);
    let rig = Rig::new(&gated, r#"{"dispatch_ambiguity": 0.3}"#);
    ambiguity(&rig, &gated);
    assert_eq!(rig.wire_models(), [DEFAULT]);
    assert_eq!(rig.tl_models(), [DEFAULT]);

    // The same Orchestrator with its repo pin: Haiku 5.5, not the default.
    let gated = gate(ORCHESTRATOR);
    let rig = Rig::new(&gated, r#"{"dispatch_ambiguity": 0.3}"#);
    ambiguity(&rig, &gated);
    assert_eq!(rig.wire_models(), ["claude-haiku-5-5"]);
}

struct NoScalar;

impl EpistemicScalarPort for NoScalar {
    fn write_scalar(
        &self,
        _: u32,
        _: &str,
        _: &str,
        _: f64,
        _: &str,
    ) -> Result<Option<maos_domain::halt::HaltReceipt>, ScalarPortError> {
        Ok(None)
    }
}

fn ambiguity(rig: &Rig, gated: &GatedManifest) {
    let orchestrator = maos_bin::orchestrator_root::construct(
        orchestrator::Orchestrator::new("orchestrator"),
        &rig.seam(),
        &declared_pins(gated),
        pid(),
        &|_| Ok(Arc::new(NoScalar)),
    )
    .expect("the Orchestrator binds");
    let section = orchestrator::split_epic("### Story 1: parse\nAccept a CSV.\n", 1)
        .unwrap()
        .remove(0);
    orchestrator
        .judge_ambiguity(&section)
        .expect("the judgement completes");
}

#[test]
fn hello_carries_the_model_its_manifest_pins_for_the_bound_provider() {
    let hello = maos_spirit_hello::MANIFEST_TOML;
    for pinned in ["claude-haiku-5-5", "claude-sonnet-5-5"] {
        let manifest = hello.replace("anthropic.claude-haiku-5-5", &format!("anthropic.{pinned}"));
        let gated = gate(&manifest);
        let rig = Rig::new(&gated, "Hello.");
        let model = pin_for(
            gated.providers.as_ref(),
            &gated.caps_required.provider.complete,
            "anthropic",
        );
        let bound = BoundInference {
            inner: Arc::clone(&rig.port),
            provider: "anthropic".into(),
            model,
        };
        maos_spirit_hello::run(&bound, rig.token(), "tl").expect("hello answers");
        assert_eq!(rig.wire_models(), [pinned]);
        assert_eq!(rig.tl_models(), [pinned]);
    }
}

#[test]
fn a_haiku_workflow_and_the_sonnet_judge_share_one_configured_provider() {
    let gated = gate(BUTLER);
    let rig = Rig::new(&gated, "faithful");
    // The workflow's request is the Butler's pin (Haiku 5.5)…
    let bound = BoundInference {
        inner: Arc::clone(&rig.port),
        provider: "anthropic".into(),
        model: pin_for(None, &gated.caps_required.provider.complete, "anthropic"),
    };
    maos_spirit_hello::run(&bound, rig.token(), "tl").expect("the workflow call");
    // …and the judge keeps its own pinned Sonnet, from `tests/judge-config.toml`.
    let config = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/judge-config.toml"
    ))
    .unwrap();
    let judge_model = toml::from_str::<JudgeConfig>(&config)
        .unwrap()
        .wire_model()
        .unwrap();
    assert_eq!(judge_model, "claude-sonnet-5-5");
    let request = judge_request(
        PID,
        rig.token(),
        ("source", "finding"),
        "anthropic",
        &judge_model,
    );
    let reply = rig.port.complete(request).expect("the judge call");
    assert_eq!(
        reply.provider_attribution.model_id.as_deref(),
        Some("claude-sonnet-5-5")
    );
    assert_eq!(rig.wire_models(), ["claude-haiku-5-5", "claude-sonnet-5-5"]);
}

#[test]
fn a_pin_outside_the_allowlist_is_refused_before_any_transport_call() {
    // Runtime gate: the Researcher's pin swapped for a retired model.
    let retired = RESEARCHER.replace(
        "anthropic.claude-sonnet-5-5",
        "anthropic.claude-3-haiku-20240307",
    );
    assert_ne!(retired, RESEARCHER);
    let gated = gate(&retired);
    let (rig, result) = research(&gated);
    let error = result.expect_err("a retired model is not admitted");
    assert!(error.contains("claude-3-haiku-20240307"), "{error}");
    assert!(rig.wire_models().is_empty(), "no request reached the wire");

    // Admission: the same pin is refused typed, before a token is minted.
    let catalog: ProviderCatalog = BTreeMap::from([(
        "anthropic".to_owned(),
        ConfiguredProvider {
            endpoint: "https://api.anthropic.com".into(),
            model: "claude-haiku-5-5",
            registered: true,
        },
    )]);
    let refusal = check_declared_pins(&declared_pins(&gated), Some(&catalog))
        .expect_err("admission refuses the retired pin");
    assert!(matches!(
        &refusal,
        AdmissionRefusal::ModelNotAdmitted { provider, model }
            if provider == "anthropic" && model == "claude-3-haiku-20240307"
    ));
    assert_eq!(refusal.code(), "model_not_admitted");
    // Every repo pin is admitted for the configured providers it names.
    for manifest in [RESEARCHER, BUTLER, ORCHESTRATOR] {
        check_declared_pins(&declared_pins(&gate(manifest)), Some(&catalog)).unwrap();
    }
}

#[test]
fn admission_admits_several_models_per_provider_and_still_refuses_unlisted_ones() {
    let catalog: ProviderCatalog = ["anthropic", "gemini"]
        .into_iter()
        .map(|id| {
            let model = if id == "anthropic" {
                "claude-haiku-5-5"
            } else {
                "gemini-3.5-flash-lite"
            };
            let endpoint = format!("https://{id}.test");
            (
                id.to_owned(),
                ConfiguredProvider {
                    endpoint,
                    model,
                    registered: true,
                },
            )
        })
        .collect();
    let section = |id: &str, model: &str| {
        maos_kernel_core::security::ProvidersSection::from_toml_str(&format!(
            "[primary]\nid = \"{id}\"\nmodel_id = \"{model}\"\n"
        ))
        .unwrap()
    };
    // The configured Haiku default, the pinned Researcher Sonnet and the judge
    // Sonnet all coexist on the one configured Anthropic provider.
    for model in ["claude-haiku-5-5", "claude-sonnet-5-5"] {
        check_provider_pins(&section("anthropic", model), Some(&catalog)).unwrap();
    }
    // A model off the currency allowlist, a retired one and a typo are refused.
    for model in ["claude-3-haiku-20240307", "claude-sonnet-9", "gpt-4o-mini"] {
        let refusal = check_provider_pins(&section("anthropic", model), Some(&catalog))
            .expect_err("not on the allowlist");
        assert_eq!(refusal.code(), "provider_unavailable", "{model}");
    }
    // Gemini keeps its single-model rule (20-4 review F1): the pin must equal
    // the configured model.
    check_provider_pins(&section("gemini", "gemini-3.5-flash-lite"), Some(&catalog)).unwrap();
    assert!(check_provider_pins(&section("gemini", "gemini-3.8-flash"), Some(&catalog)).is_err());
    assert_eq!(
        admitted_models("gemini", "gemini-3.5-flash-lite"),
        ["gemini-3.5-flash-lite"]
    );
}

/// Review P2 — the one validation every admission/staging path runs: the shell's
/// embedded hello manifest (`check_gated_pins`) and a hot-swap successor's staged
/// text (`check_staged_pins`, which need not carry the admission-only sections).
/// The real binary is exercised in `one_shot_hello.rs` (one-shot, replay) and
/// `one_daemon_one_door_16_1.rs` (hot swap, cold swap, `maos run`, `maosctl load`).
#[test]
fn shared_pin_validation_covers_the_shell_manifest_and_a_minimal_staged_successor() {
    use maos_bin::admission::{check_gated_pins, check_staged_pins};
    let catalog: ProviderCatalog = BTreeMap::from([(
        "anthropic".to_owned(),
        ConfiguredProvider {
            endpoint: "https://api.anthropic.com".into(),
            model: "claude-haiku-5-5",
            registered: true,
        },
    )]);
    let hello = maos_spirit_hello::MANIFEST_TOML;
    check_gated_pins(&gate(hello), Some(&catalog)).unwrap();
    let retired = hello.replace(
        "anthropic.claude-haiku-5-5",
        "anthropic.claude-3-haiku-20240307",
    );
    assert_ne!(retired, hello);
    let refusal = check_gated_pins(&gate(&retired), Some(&catalog)).expect_err("retired");
    assert_eq!(refusal.code(), "model_not_admitted");

    // A minimal successor (no [sandbox]/[resources]/…) still has its pin checked.
    let minimal = |pin: &str| format!("[capabilities.required]\nprovider.complete = [\"{pin}\"]\n");
    check_staged_pins(&minimal("anthropic.claude-sonnet-5-5"), Some(&catalog)).unwrap();
    let refusal = check_staged_pins(
        &minimal("anthropic.claude-3-haiku-20240307"),
        Some(&catalog),
    )
    .expect_err("retired");
    assert_eq!(refusal.code(), "model_not_admitted");
    // `[providers]` assertions are validated on the staged text as well.
    let asserted = format!(
        "{}\n[providers.primary]\nid = \"anthropic\"\nmodel_id = \"claude-3-haiku-20240307\"\n",
        minimal("anthropic")
    );
    assert_eq!(
        check_staged_pins(&asserted, Some(&catalog))
            .expect_err("retired")
            .code(),
        "provider_unavailable"
    );
    // Unparseable text is refused, not waved through.
    assert!(check_staged_pins("not [toml", Some(&catalog)).is_err());
}
