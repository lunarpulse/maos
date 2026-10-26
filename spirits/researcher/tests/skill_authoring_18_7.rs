#![forbid(unsafe_code)]
//! Story 18-7 D-09/CR-6: the native model-authored submission step.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use maos_domain::distillation::{DistillationError, DistillationReceipt, DistillationRequest};
use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::context_budget::{
    ContextBudgetPort, ContextLevel, ContextReportError, ContextStatus,
};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::InferencePort;
use maos_domain::self_telemetry::{
    DistillationOutcomeEntry, HaltTelemetryEntry, SelfTelemetryReport,
};
use maos_skill::{
    AuthorIdentity, ESkillQueue, ESkillStore, QueueEntry, SkillAdmissionQueue, SkillAdmissionState,
    SkillPayload, SubmissionContext,
};
use maos_spirit_sdk::spirit_test::SpiritTest;
use researcher::{
    __maos_spirit_vtable_Researcher, seed_claims, RecalledFrame, Researcher, ResearcherError,
    ResearcherInferenceBinding, SkillAuthorOutcome, SkillAuthorPort, SkillPortError,
};

const PID: u32 = 187;
const DOCUMENT: &str = "---\nid = \"researcher.survey-method\"\nversion = \"0.1.0\"\nname = \"Survey method\"\ndescription = \"Preserve conflicting evidence in research surveys\"\nrequired_capabilities = []\n---\n# Survey method\nCompare methodology before combining contradictory findings.\n";
const DIFF: &str = "--- a/researcher.survey-method.md\n+++ b/researcher.survey-method.md\n@@ -9 +9,2 @@\n Compare methodology before combining contradictory findings.\n+Retain uncertainty when the evidence remains contradictory.\n";
const TELEMETRY_SUMMARY: &str = "successes=0 failures=1 halts=1 distillations=1";

type Events = Arc<Mutex<Vec<&'static str>>>;

struct Model {
    seen: Mutex<Vec<InferenceRequest>>,
    generation_error: Option<fn() -> InferenceError>,
    malformed: bool,
    events: Events,
}
impl InferencePort for Model {
    fn complete(&self, request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        let generation = request.options.max_tokens == 2048;
        let proposal = request.prompt.starts_with("Propose one improvement");
        self.events
            .lock()
            .unwrap()
            .push(if generation { "generate" } else { "survey" });
        self.seen.lock().unwrap().push(request);
        if generation {
            if let Some(error) = self.generation_error {
                return Err(error());
            }
        }
        Ok(InferenceResponse {
            text: if !generation {
                "a model finding"
            } else if self.malformed {
                "not a skill"
            } else if proposal {
                DIFF
            } else {
                DOCUMENT
            }
            .into(),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 7,
                output_tokens: 11,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "replay".into(),
                endpoint_url: "test://authoring".into(),
                model_id: None,
            },
        })
    }
}

struct Host {
    queue: Mutex<SkillAdmissionQueue>,
    telemetry: Mutex<SelfTelemetryReport>,
    denied: bool,
    reads_fail: bool,
    events: Events,
}
impl Host {
    fn context<'a>(
        author: &'a AuthorIdentity,
        actions: &'a BTreeSet<String>,
    ) -> SubmissionContext<'a> {
        SubmissionContext {
            author,
            declared_actions: actions,
            principal_ids: &[],
        }
    }
    fn identity() -> AuthorIdentity {
        AuthorIdentity {
            class: "researcher".into(),
            pid: PID,
            boot_nonce: "test-boot".into(),
        }
    }
    fn admit_document(&self) -> QueueEntry {
        let mut stored = self.queue.lock().unwrap().to_stored();
        let entry = stored
            .iter_mut()
            .find(|entry| matches!(&entry.payload, Some(SkillPayload::AuthorSelf(_))))
            .unwrap();
        entry.state = SkillAdmissionState::Admitted;
        let entry = entry.clone();
        *self.queue.lock().unwrap() = SkillAdmissionQueue::from_stored(stored).unwrap();
        entry
    }
}
impl SkillAuthorPort for Host {
    fn preflight(&self, revision: bool) -> Result<String, SkillPortError> {
        self.events.lock().unwrap().push("preflight");
        if self.denied {
            return Err(SkillPortError::CapabilityDenied);
        }
        if !revision {
            return Ok(String::new());
        }
        let report = self.telemetry.lock().unwrap();
        Ok(format!(
            "successes={} failures={} halts={} distillations={}",
            report.success_count,
            report.failure_count,
            report.halt_events.len(),
            report.distillation_outcomes.len()
        ))
    }
    fn own_entries(&self) -> Result<Vec<QueueEntry>, SkillPortError> {
        self.events.lock().unwrap().push("own_entries");
        if self.reads_fail {
            return Err(SkillPortError::HostShutDown);
        }
        Ok(self.queue.lock().unwrap().to_stored())
    }
    fn author(&self, document: String) -> Result<QueueEntry, SkillPortError> {
        self.events.lock().unwrap().push("author");
        if self.denied {
            return Err(SkillPortError::CapabilityDenied);
        }
        let mut queue = self.queue.lock().unwrap();
        let (entry, _) = queue.prepare_author(
            document,
            &Self::context(&Self::identity(), &BTreeSet::new()),
            &[],
        )?;
        queue.commit_prepared(entry.clone())?;
        Ok(entry)
    }
    fn propose(
        &self,
        target_id: String,
        target_version: String,
        diff: String,
    ) -> Result<QueueEntry, SkillPortError> {
        self.events.lock().unwrap().push("propose");
        if self.denied {
            return Err(SkillPortError::CapabilityDenied);
        }
        let evidence = self.telemetry.lock().unwrap().clone();
        let proposal = maos_skill::build_proposal(
            target_id.as_str().into(),
            target_version.as_str().into(),
            diff,
            evidence,
        )
        .map_err(|error| ESkillStore::InvalidEntry(ESkillQueue::Proposal(error)))?;
        let mut queue = self.queue.lock().unwrap();
        let (entry, _) = queue.prepare_proposal(
            proposal,
            &Self::context(&Self::identity(), &BTreeSet::new()),
        )?;
        queue.commit_prepared(entry.clone())?;
        Ok(entry)
    }
}

#[derive(Default)]
struct Context(Mutex<Vec<u32>>);
impl ContextBudgetPort for Context {
    fn report_context_used(&self, used: u32) -> Result<ContextStatus, ContextReportError> {
        self.0.lock().unwrap().push(used);
        Ok(ContextStatus {
            used,
            window: 200_000,
            level: ContextLevel::Within,
            crossed: None,
        })
    }
}

struct Writer {
    events: Events,
    fail: bool,
}
impl maos_domain::ports::DistillationPort for Writer {
    fn write_distillate(
        &self,
        pid: u32,
        request: DistillationRequest,
    ) -> Result<DistillationReceipt, DistillationError> {
        assert_eq!(pid, PID);
        self.events.lock().unwrap().push("distill");
        if self.fail {
            return Err(DistillationError::AuditChainMissing {
                reason: "writer refused".into(),
            });
        }
        Ok(DistillationReceipt::new(
            [0x18; 16],
            Default::default(),
            request.source_log_ref,
            1,
            0,
        ))
    }
    fn admit_for_consumer(
        &self,
        _: [u8; 16],
        _: &maos_domain::invariants::i13::AllowedPromotionSet,
    ) -> Result<(), DistillationError> {
        panic!("authoring must not perform consumer admission")
    }
}

fn frames() -> Vec<RecalledFrame> {
    seed_claims()
        .into_iter()
        .enumerate()
        .map(|(index, claim)| RecalledFrame {
            frame_id: [index as u8 + 1; 16],
            intent: "researcher:seed_claim".into(),
            payload: serde_json::to_vec(&claim).unwrap(),
        })
        .collect()
}
fn idle(researcher: &Researcher) {
    let vtable = __maos_spirit_vtable_Researcher();
    let mut harness = SpiritTest::new(researcher, &vtable);
    harness.fixture_mut().invoke_on_idle = true;
    assert_eq!(harness.run().base.hooks_fired.get("on_idle"), Some(&1));
}
fn setup(
    denied: bool,
    reads_fail: bool,
    generation_error: Option<fn() -> InferenceError>,
    malformed: bool,
) -> (
    Researcher,
    Arc<Model>,
    Arc<Host>,
    Arc<AtomicUsize>,
    Arc<Context>,
) {
    let events = Events::default();
    let model = Arc::new(Model {
        seen: Mutex::default(),
        generation_error,
        malformed,
        events: events.clone(),
    });
    let host = Arc::new(Host {
        queue: Mutex::default(),
        telemetry: Mutex::new(
            SelfTelemetryReport::new(
                PID,
                10,
                20,
                0,
                1,
                0,
                0,
                0,
                vec![HaltTelemetryEntry::new(
                    "survey-conflict",
                    "methodology_conflict",
                    "threshold",
                    1.0,
                    Some(0.5),
                    15,
                    None,
                )
                .unwrap()],
                vec![DistillationOutcomeEntry::new([0x18; 16], 3, 1, 16)],
                20,
            )
            .unwrap(),
        ),
        denied,
        reads_fail,
        events,
    });
    let mints = Arc::new(AtomicUsize::new(0));
    let counted = mints.clone();
    let binding = ResearcherInferenceBinding::per_pass(
        "replay".into(),
        Arc::new(move |pid, _| {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(CapabilityToken::new(
                TokenId([18; 16]),
                pid,
                u64::MAX,
                [0; 64],
            ))
        }),
        Arc::new(|| Some(PID)),
    );
    let context = Arc::new(Context::default());
    let researcher = Researcher::with_frames(frames())
        .with_deferred_inference_port(model.clone(), Arc::new(Mutex::new(Some(binding))))
        .with_context_port(context.clone())
        .with_distillation_port(Arc::new(Writer {
            events: host.events.clone(),
            fail: false,
        }))
        .with_skill_author_port(host.clone());
    (researcher, model, host, mints, context)
}

#[test]
fn bound_pass_authors_model_bytes_once_with_the_pass_binding_then_waits_for_admission() {
    let (researcher, model, host, mints, context) = setup(false, false, None, false);
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    let Some(SkillAuthorOutcome::Authored(entry)) = researcher.take_skill_author_outcome() else {
        panic!("expected authored pending document")
    };
    assert_eq!(entry.state, SkillAdmissionState::Pending);
    assert_eq!(
        entry.payload,
        Some(SkillPayload::AuthorSelf(DOCUMENT.into()))
    );
    assert_eq!(
        *host.events.lock().unwrap(),
        [
            "survey",
            "survey",
            "survey",
            "distill",
            "own_entries",
            "preflight",
            "generate",
            "author"
        ]
    );
    assert_eq!(mints.load(Ordering::SeqCst), 1);
    assert_eq!(
        researcher.completions_consumed(),
        3,
        "survey-only metric stays compatible"
    );
    let seen = model.seen.lock().unwrap();
    assert_eq!(seen.len(), 4);
    for request in seen.iter() {
        assert_eq!(request.spirit_pid, PID);
        assert_eq!(request.provider_id.as_deref(), Some("replay"));
        assert_eq!(
            request.capability_token, seen[0].capability_token,
            "same minted token across the pass"
        );
    }
    assert_eq!(
        seen[3].prompt,
        researcher::skill_author::authoring_prompt(&researcher.last_output().unwrap(), None, None)
    );
    drop(seen);
    assert_eq!(
        *context.0.lock().unwrap(),
        [18; 4],
        "generation usage is reported too"
    );
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    assert!(matches!(
        researcher.take_skill_author_outcome(),
        Some(SkillAuthorOutcome::PendingExisting(_))
    ));
    assert_eq!(
        model.seen.lock().unwrap().len(),
        7,
        "pending draft needs no new generation"
    );
    assert_eq!(host.queue.lock().unwrap().to_stored().len(), 1);
    assert_eq!(mints.load(Ordering::SeqCst), 2);
}

#[test]
fn admitted_owned_document_gets_model_diff_with_exact_target_then_pending_proposal_is_inert() {
    let (researcher, model, host, _, _) = setup(false, false, None, false);
    idle(&researcher);
    let base = host.admit_document();
    let reloaded = researcher.clone();
    idle(&reloaded);
    assert!(reloaded.take_pass_error().is_none());
    let Some(SkillAuthorOutcome::Proposed(entry)) = reloaded.take_skill_author_outcome() else {
        panic!("expected proposal")
    };
    assert_eq!(entry.state, SkillAdmissionState::Pending);
    let Some(SkillPayload::RevisionProposal(proposal)) = entry.payload else {
        panic!("missing proposal payload")
    };
    assert_eq!(proposal.target_skill_id, base.id);
    assert_eq!(proposal.target_version, base.version);
    assert_eq!(proposal.proposed_diff, DIFF);
    assert_eq!(proposal.telemetry_evidence.spirit_pid, PID);
    let seen = model.seen.lock().unwrap();
    assert_eq!(
        seen[7].prompt,
        researcher::skill_author::authoring_prompt(
            &reloaded.last_output().unwrap(),
            Some(&base),
            Some(TELEMETRY_SUMMARY),
        )
    );
    assert!(seen[7].prompt.contains(DOCUMENT));
    assert!(seen[7].prompt.contains(TELEMETRY_SUMMARY));
    drop(seen);
    idle(&reloaded);
    assert!(reloaded.take_pass_error().is_none());
    assert!(matches!(
        reloaded.take_skill_author_outcome(),
        Some(SkillAuthorOutcome::PendingExisting(_))
    ));
    assert_eq!(model.seen.lock().unwrap().len(), 11);
    assert_eq!(host.queue.lock().unwrap().to_stored().len(), 2);
}

#[test]
fn revision_generation_consumes_host_counts_and_changes_hash_when_counts_change() {
    use sha2::{Digest, Sha256};
    let mut hashes = Vec::new();
    for (successes, failures, halts, distillations) in [
        (0, 1, 1, 1),
        (7, 1, 1, 1),
        (0, 2, 1, 1),
        (0, 1, 0, 1),
        (0, 1, 1, 0),
    ] {
        let (researcher, model, host, _, _) = setup(false, false, None, false);
        host.author(DOCUMENT.into()).unwrap();
        let base = host.admit_document();
        {
            let mut report = host.telemetry.lock().unwrap();
            report.success_count = successes;
            report.failure_count = failures;
            report.halt_events.truncate(halts);
            report.distillation_outcomes.truncate(distillations);
        }
        idle(&researcher);
        assert!(researcher.take_pass_error().is_none());
        let Some(SkillAuthorOutcome::Proposed(entry)) = researcher.take_skill_author_outcome()
        else {
            panic!("expected revision generated using host telemetry")
        };
        let Some(SkillPayload::RevisionProposal(proposal)) = entry.payload else {
            panic!("missing proposal payload")
        };
        assert_eq!(proposal.telemetry_evidence.success_count, successes);
        assert_eq!(proposal.telemetry_evidence.failure_count, failures);
        assert_eq!(proposal.telemetry_evidence.halt_events.len(), halts);
        assert_eq!(
            proposal.telemetry_evidence.distillation_outcomes.len(),
            distillations
        );
        let summary = format!(
            "successes={successes} failures={failures} halts={halts} distillations={distillations}"
        );
        let output = researcher.last_output().unwrap();
        let seen = model.seen.lock().unwrap();
        assert_eq!(seen.len(), 4);
        assert_eq!(
            seen[3].prompt,
            researcher::skill_author::authoring_prompt(&output, Some(&base), Some(&summary))
        );
        assert!(seen[3].prompt.contains(summary.as_str()));
        hashes.push(Sha256::digest(seen[3].prompt.as_bytes()));
        assert_eq!(
            researcher::skill_author::authoring_prompt(&output, None, Some(&summary)),
            researcher::skill_author::authoring_prompt(&output, None, None),
            "author-only prompt remains independent of revision telemetry"
        );
    }
    for hash in &hashes[1..] {
        assert_ne!(
            &hashes[0], hash,
            "each host count affects the revision prompt"
        );
    }
}

#[test]
fn undeclared_author_refuses_before_generation_and_retains_the_successful_survey() {
    let (researcher, model, host, _, context) = setup(true, false, None, false);
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    assert!(matches!(
        researcher.take_skill_author_error(),
        Some(ResearcherError::SkillAuthor(
            SkillPortError::CapabilityDenied
        ))
    ));
    assert_eq!(model.seen.lock().unwrap().len(), 3);
    assert_eq!(
        *host.events.lock().unwrap(),
        [
            "survey",
            "survey",
            "survey",
            "distill",
            "own_entries",
            "preflight"
        ]
    );
    assert_eq!(*context.0.lock().unwrap(), [18; 3]);
    assert_eq!(researcher.completions_consumed(), 3);
    assert!(host.queue.lock().unwrap().to_stored().is_empty());
    assert_eq!(researcher.last_output().unwrap().findings.len(), 3);
    assert!(researcher.take_skill_author_outcome().is_none());
}

#[test]
fn unbound_deterministic_pass_never_reads_or_submits_even_with_author_port_installed() {
    let (_, model, host, _, _) = setup(true, true, None, false);
    let researcher = Researcher::with_frames(frames()).with_skill_author_port(host.clone());
    idle(&researcher);
    assert!(researcher.last_output().is_some());
    assert!(researcher.take_pass_error().is_none());
    assert!(host.events.lock().unwrap().is_empty());
    assert!(model.seen.lock().unwrap().is_empty());
    assert!(researcher.take_skill_author_outcome().is_none());
}

#[test]
fn unavailable_reader_and_generation_drift_are_typed_with_no_submission_or_fallback() {
    let (researcher, model, host, _, _) = setup(false, true, None, false);
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    assert!(matches!(
        researcher.take_skill_author_error(),
        Some(ResearcherError::SkillAuthor(SkillPortError::HostShutDown))
    ));
    assert_eq!(model.seen.lock().unwrap().len(), 3);
    assert!(host.queue.lock().unwrap().to_stored().is_empty());
    assert_eq!(researcher.last_output().unwrap().findings.len(), 3);
    let (researcher, model, host, _, _) = setup(
        false,
        false,
        Some(|| InferenceError::MalformedResponse("strict authoring prompt drift".into())),
        false,
    );
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    assert!(matches!(
        researcher.take_skill_author_error(),
        Some(ResearcherError::Inference(
            InferenceError::MalformedResponse(_)
        ))
    ));
    assert_eq!(model.seen.lock().unwrap().len(), 4);
    assert_eq!(host.events.lock().unwrap().last(), Some(&"generate"));
    assert!(host.queue.lock().unwrap().to_stored().is_empty());
    assert_eq!(researcher.last_output().unwrap().findings.len(), 3);
}

#[test]
fn malformed_model_document_is_nonfatal_to_survey_and_not_replaced_with_template() {
    let (researcher, model, host, _, context) = setup(false, false, None, true);
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    assert!(matches!(
        researcher.take_skill_author_error(),
        Some(ResearcherError::SkillAuthor(SkillPortError::Submission(
            ESkillStore::InvalidEntry(ESkillQueue::Schema(_))
        )))
    ));
    assert!(host.queue.lock().unwrap().to_stored().is_empty());
    assert_eq!(model.seen.lock().unwrap().len(), 4);
    assert_eq!(host.events.lock().unwrap().last(), Some(&"author"));
    assert_eq!(*context.0.lock().unwrap(), [18; 4]);
    let output = researcher
        .last_output()
        .expect("successful survey retained");
    assert_eq!(output.findings.len(), 3);
    assert!(output
        .findings
        .iter()
        .all(|finding| finding.statement == "a model finding"));
    assert!(researcher.take_skill_author_outcome().is_none());
}

#[test]
fn distillation_refusal_prevents_generation_and_host_calls() {
    let (researcher, model, host, _, _) = setup(false, false, None, false);
    let researcher = researcher.with_distillation_port(Arc::new(Writer {
        events: host.events.clone(),
        fail: true,
    }));
    idle(&researcher);
    assert!(matches!(
        researcher.take_pass_error(),
        Some(ResearcherError::Distillation(_))
    ));
    assert_eq!(model.seen.lock().unwrap().len(), 3);
    assert_eq!(host.events.lock().unwrap().last(), Some(&"distill"));
    assert!(host.queue.lock().unwrap().to_stored().is_empty());
    assert!(researcher.last_output().is_none());
}

#[test]
fn strict_seed_fixture_pins_real_prompts_for_author_then_admitted_base_proposal() {
    use sha2::{Digest, Sha256};
    struct StrictSeed {
        entries: Vec<serde_json::Value>,
        next: AtomicUsize,
    }
    impl InferencePort for StrictSeed {
        fn complete(&self, request: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
            let index = self.next.fetch_add(1, Ordering::SeqCst);
            let entry = self.entries.get(index).expect("strict fixture exhausted");
            assert_eq!(entry["sequence"], index);
            assert_eq!(
                entry["prompt_sha256"],
                format!("{:x}", Sha256::digest(request.prompt.as_bytes()))
            );
            assert_eq!(entry["prompt_len"], request.prompt.len());
            Ok(InferenceResponse {
                text: entry["response"]["text"].as_str().unwrap().into(),
                stop_reason: StopReason::StopSequence,
                usage: TokenUsage {
                    input_tokens: entry["response"]["usage"]["input_tokens"].as_u64().unwrap()
                        as u32,
                    output_tokens: entry["response"]["usage"]["output_tokens"]
                        .as_u64()
                        .unwrap() as u32,
                },
                provider_attribution: ProviderAttribution {
                    provider_id: "replay".into(),
                    endpoint_url: "cassette://j-researcher/skill-authoring-18-7".into(),
                    model_id: Some("hand-authored-seed".into()),
                },
            })
        }
    }
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../crates/maos-journey-test/cassettes/j-researcher/skill-authoring-18-7.json"
    ))
    .unwrap();
    assert_eq!(fixture["provenance"], "seed");
    let entries = fixture["entries"].as_array().unwrap().clone();
    assert_eq!(entries.len(), 8);
    let model = Arc::new(StrictSeed {
        entries,
        next: AtomicUsize::new(0),
    });
    let (_, _, host, _, _) = setup(false, false, None, false);
    let researcher = Researcher::with_frames(frames())
        .with_inference_port(
            model.clone(),
            "replay",
            CapabilityToken::new(TokenId([18; 16]), PID, u64::MAX, [0; 64]),
            PID,
        )
        .with_skill_author_port(host.clone());
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    let Some(SkillAuthorOutcome::Authored(entry)) = researcher.take_skill_author_outcome() else {
        panic!("expected fixture author submission")
    };
    assert_eq!(
        entry.payload,
        Some(SkillPayload::AuthorSelf(DOCUMENT.into()))
    );
    host.admit_document();
    idle(&researcher);
    assert!(researcher.take_pass_error().is_none());
    let Some(SkillAuthorOutcome::Proposed(entry)) = researcher.take_skill_author_outcome() else {
        panic!("expected fixture revision submission")
    };
    let Some(SkillPayload::RevisionProposal(proposal)) = entry.payload else {
        panic!("missing revision payload")
    };
    assert_eq!(proposal.proposed_diff, DIFF);
    assert_eq!(model.next.load(Ordering::SeqCst), 8);
}
