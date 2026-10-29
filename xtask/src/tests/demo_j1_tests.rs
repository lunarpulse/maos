use super::*;

fn obs(events: Vec<serde_json::Value>, stderr: &str) -> SceneObservation {
    SceneObservation {
        events,
        stderr: stderr.to_string(),
        wall: std::time::Duration::from_millis(180),
        exit_ok: true,
    }
}

/// Evaluate the observation and pull out one named beat.
fn beat_named(observation: &SceneObservation, name: &str) -> Beat {
    evaluate_beats(observation)
        .into_iter()
        .find(|b| b.name == name)
        .unwrap_or_else(|| panic!("beat `{name}` must be evaluated"))
}

/// The whole honest-labeling contract in one assertion: a beat nobody has built
/// yet is visible, owned, and does NOT fail the scene.
#[test]
fn absent_beats_are_visible_and_never_fail_the_scene() {
    for beat in unlanded_beats() {
        assert_eq!(
            beat.state.as_str(),
            "ABSENT",
            "{} must read ABSENT",
            beat.name
        );
        assert!(!beat.failed(), "{} must not fail the run", beat.name);
        assert!(beat.owner.is_some(), "{} must name an owner", beat.name);
    }
}

#[test]
fn a_fixture_take_never_claims_tier2() {
    let tier2 = unlanded_beats()
        .into_iter()
        .find(|b| b.name == TIER2_BEAT)
        .expect("declared");
    assert_eq!(tier2.state.as_str(), "ABSENT");
}

/// The split assigns its gate-derived crossing to `2b`; its explicit skip
/// counterpart remains owned by that story, while the later signed rung is `2c`.
#[test]
fn two_host_beats_are_owned_by_their_crosshost_stories() {
    let delegation = absent_two_host_delegation();
    let signed_run = unlanded_beats()
        .into_iter()
        .find(|beat| beat.name == "two-host-signed-run")
        .expect("declared");
    assert_eq!(delegation.owner, Some("j1-crosshost-2b"));
    assert_eq!(signed_run.owner, Some("j1-crosshost-2d-paid-two-host-run"));
    // §A6 review P10 (AC4.4) — the 2c-owned beat must render ABSENT, not merely
    // be declared unlanded: a state change short of execution (e.g. a future
    // "planned") must still red this pin so the narrated artifact cannot hint
    // at a signed rung nobody built.
    assert_eq!(
        signed_run.state.as_str(), "ABSENT",
        "two-host-signed-run stays ABSENT until j1-crosshost-2c delivers it"
    );

    assert!(
        run_delegation_gate()
            .iter()
            .any(|beat| beat.name == "two-host-delegation" && beat.executed),
        "the gate-derived beat must be present; its state depends on the real-tree judge"
    );
}

/// j1-crosshost-1b AC2.11 — the beat this story owns must NOT still render ABSENT.
/// If the refusal proofs land and the demo keeps declaring them unlanded, the
/// narrated artifact prints a false claim about its own work.
#[test]
fn the_refusal_beat_is_no_longer_declared_unlanded() {
    assert!(
        !unlanded_beats()
            .iter()
            .any(|b| b.name == "disallowed-intent-refused-blocking"),
        "j1-crosshost-1b landed the refusal proofs; the beat is emitted from the \
         gate-judging path now, never declared ABSENT"
    );
}

/// §A6 review P5 — the flipped beat must be EMITTED, not merely absent from the
/// unlanded list. Deleting the `beats.push` in `run_delegation_gate()` left
/// `the_refusal_beat_is_no_longer_declared_unlanded` green while the beat
/// vanished from every claim table. (State is deliberately NOT asserted here:
/// the judge runs against the real tree, which a unit-test CWD cannot promise.)
#[test]
fn the_refusal_beat_is_emitted_by_the_gate_judging_path() {
    let beats = run_delegation_gate();
    let beat = beats
        .iter()
        .find(|b| b.name == "disallowed-intent-refused-blocking")
        .expect("run_delegation_gate must emit the refusal beat");
    assert!(beat.executed, "the refusal beat is an executed judgement");
    assert_eq!(
        beats.len(),
        crate::check_j1_loopback_delegation::ledger_leg_names().len() + 2,
        "one beat per published gate leg, plus the refusal conjunction and the cross-host beat"
    );
}

#[test]
fn an_executed_beat_that_did_not_hold_fails() {
    let beat = Beat::executed("x", "y", state_of(false), String::new());
    assert!(beat.failed());
    assert_eq!(beat.state.as_str(), "INDETERMINATE");
}

/// The regression the 2026-08-14 rehearsal caught: a drain timeout means queued
/// capability rows can be lost, so it must FAIL the scene rather than be
/// narrated as fine.
#[test]
fn a_drain_timeout_fails_the_scene() {
    let observation = obs(
        vec![serde_json::json!({"event": "drain"})],
        "maos run: audit writer topology drain timed out after 5s",
    );
    let beats = evaluate_beats(&observation);
    let drain = beats
        .iter()
        .find(|b| b.name == "audit-drain-clean")
        .expect("beat present");
    assert!(drain.failed());
    assert!(drain.detail.contains("incomplete bundle"));
}

#[test]
fn an_ambient_journal_warning_fails_the_clean_home_beat() {
    let observation = obs(
        vec![serde_json::json!({"event": "drain"})],
        "journal: WARNING — skipping corrupted line 115: EOF while parsing a string",
    );
    let beats = evaluate_beats(&observation);
    let clean = beats
        .iter()
        .find(|b| b.name == "state-home-clean")
        .expect("beat present");
    assert!(clean.failed());
}

/// "Route locally anyway" is the silent regression 1a's gate exists to stop. The
/// scene must not read a missing delegation as proven either.
#[test]
fn a_missing_delegation_event_fails_the_frame_borne_beat() {
    let observation = obs(
        vec![serde_json::json!({"event": "topology_worker_admit", "frame_borne": false})],
        "",
    );
    let beats = evaluate_beats(&observation);
    let beat = beats
        .iter()
        .find(|b| b.name == "delegation-frame-crosses-loopback")
        .expect("beat present");
    assert!(beat.failed());
}

/// A delegation routed to the wrong host must not pass just because the event
/// exists — even when it is ONE of the three sections and the other two held.
#[test]
fn a_delegation_to_the_wrong_host_fails() {
    let mut events = epic_stream();
    let second = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e["event"] == "delegation_routed")
        .map(|(at, _)| at)
        .nth(1)
        .expect("three routed frames");
    events[second]["to_host"] = serde_json::json!("somewhere-else");
    let beat = beat_named(&obs(events, ""), "delegation-frame-crosses-loopback");
    assert!(beat.failed(), "{}", beat.detail);
}

/// A raw exit code is never completion — only the adapter's oracle is.
///
/// j1-crosshost-2a AC1.6 INVERTED the old premise of this test. It used to assert
/// that `completed: true` with a null TL ref FAILS the beat, because the beat
/// scored `completed && !tl_ref.is_empty()`. That conjunction was a null control:
/// `last_stdout_tl_ref` (then `completion_tl_ref`) is assigned on EVERY stdout
/// row, independently of the oracle, so its non-emptiness only ever proved "the
/// worker printed something". The beat now rests on the oracle's verdict alone,
/// and this test proves BOTH directions of that.
#[test]
fn completion_comes_from_the_oracle_not_from_a_tl_ref() {
    let completion = |completed: bool, tl_ref: serde_json::Value| {
        serde_json::json!({
            "event": "worker_completion",
            "worker_cli": "worker-cli-fixture",
            "completion": if completed { "completed" } else { "not_completed:no_effect_evidence" },
            "completed": completed,
            "last_stdout_tl_ref": tl_ref,
        })
    };
    // (a) The oracle said completed for every section. A missing evidence
    //     pointer does not overturn the verdict — and the ref is deliberately
    //     still emitted on failures, so absence here means the worker printed
    //     nothing, not that it failed.
    let observation = obs(
        (0..EPIC_SECTIONS)
            .map(|_| completion(true, serde_json::Value::Null))
            .collect(),
        "",
    );
    let beat = beat_named(&observation, "worker-completed-by-adapter-oracle");
    assert!(
        !beat.failed(),
        "the oracle's verdict is the beat; the TL ref is an evidence pointer"
    );

    // (b) The load-bearing direction: ONE section's NON-completion with a
    //     perfectly good TL ref must fail the beat, even behind two that held.
    let observation = obs(
        vec![
            completion(true, serde_json::json!("01a003bfb38d20e97fcb08c16274374c")),
            completion(false, serde_json::json!("01a003bfb38d20e97fcb08c16274374d")),
            completion(true, serde_json::json!("01a003bfb38d20e97fcb08c16274374e")),
        ],
        "",
    );
    let beat = beat_named(&observation, "worker-completed-by-adapter-oracle");
    assert!(
        beat.failed(),
        "a citable TL ref must never launder a non-completion into a pass"
    );
}

// ── Story 19-4 — the recorded epic scene (D-19-4-09 A) ──────────────────────

/// The judged halt's kernel id and its Transparency-Log frame id.
const HALT_ID: &str = "01J19D4AMBIGU0USSECT10N200";
const HALT_FRAME: &str = "03190000000000000000000000000002";
/// The three Workers' TaskComplete frame ids (the digest's cites).
const COMPLETION_REFS: [&str; 3] = [
    "01190000000000000000000000000001",
    "01190000000000000000000000000002",
    "01190000000000000000000000000003",
];

/// Section `k`'s goal as the root routes it: the section text verbatim, the
/// ambiguous one with the operator's answer appended.
fn section_goal(k: usize) -> String {
    let text = format!("### Story {k}: section {k}\n\nDo part {k}.");
    if k as u64 == AMBIGUOUS_SECTION {
        format!("{text}\n\nOperator clarification (halt resolved): {CLARIFICATION}")
    } else {
        text
    }
}

fn judged(k: usize) -> serde_json::Value {
    let ambiguous = k as u64 == AMBIGUOUS_SECTION;
    serde_json::json!({
        "event": "dispatch_judged",
        "section": k,
        "tag": AMBIGUITY_TAG,
        "value": if ambiguous { 0.86 } else { 0.12 },
        "halt_id": if ambiguous { serde_json::json!(HALT_ID) } else { serde_json::Value::Null },
        "halt_frame_id": if ambiguous { serde_json::json!(HALT_FRAME) } else { serde_json::Value::Null },
    })
}

fn dispatch_halted() -> serde_json::Value {
    serde_json::json!({
        "event": "dispatch_halted",
        "halt_ids": [HALT_ID],
        "dispatched_sections": 1,
        "undispatched_sections": [2, 3],
        "orchestrator_unloaded": false,
    })
}

fn dispatch_resumed() -> serde_json::Value {
    serde_json::json!({
        "event": "dispatch_resumed",
        "section": AMBIGUOUS_SECTION,
        "halt_ids": [HALT_ID],
        "provided_context": true,
    })
}

/// One section's frame, Worker run and completion, in the root's order.
fn section_events(k: usize) -> Vec<serde_json::Value> {
    let child_pid = 1_308_068 + k as u64;
    vec![
        serde_json::json!({
            "event": "delegation_routed",
            "to_host": DELEGATION_HOST,
            "recipient": DELEGATION_RECIPIENT,
            "intent": "development-task:write-workspace",
            "goal": section_goal(k),
        }),
        serde_json::json!({"event": "topology_worker_admit", "frame_borne": true}),
        serde_json::json!({
            "event": "host_grant_disposition",
            "granted_tier": "SandboxTier(3)",
            "egress": "enforced",
        }),
        serde_json::json!({
            "event": "cli_wrapper_loaded",
            "granted_tier": "SandboxTier(3)",
            "child_pid": child_pid,
        }),
        serde_json::json!({
            "event": "worker_completion",
            "worker_cli": "worker-cli-fixture",
            "completion": "completed",
            "completed": true,
            "last_stdout_tl_ref": COMPLETION_REFS[k - 1],
        }),
        serde_json::json!({
            "event": "cli_wrapper_exit",
            "child_pid": child_pid,
            "exit_cause": "Exited { code: 0 }",
            "is_crash": false,
        }),
        serde_json::json!({
            "event": "delegation_completed",
            "result": "completed",
            "orchestrator_frames_drained": 1,
            "orchestrator_safe_point": true,
        }),
    ]
}

fn morning_digest(
    completed: &[&str],
    open_halts: &[&str],
    resolved_halts: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "event": "morning_digest",
        "now_ns": 1_761_000_000_000_000_000u64,
        "digest": {
            "completed": completed
                .iter()
                .map(|r| serde_json::json!({"intent": "task.complete", "outcome": "succeeded", "source_log_ref": r}))
                .collect::<Vec<_>>(),
            "open_halts": open_halts
                .iter()
                .map(|r| serde_json::json!({"intent": AMBIGUITY_TAG, "source_log_ref": r}))
                .collect::<Vec<_>>(),
            "resolved_halts": resolved_halts
                .iter()
                .map(|r| serde_json::json!({
                    "intent": AMBIGUITY_TAG,
                    "halt_id": HALT_ID,
                    "resolution": "provided_context",
                    "source_log_ref": r,
                }))
                .collect::<Vec<_>>(),
            "anomalies": [],
            "trust_bar": 1.0,
        },
    })
}

fn idle_and_drain(digest: serde_json::Value) -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"event": "on_idle_fired", "spirit_id": "orchestrator"}),
        serde_json::json!({"event": "on_idle_fired", "spirit_id": "architect"}),
        serde_json::json!({"event": "on_idle_fired", "spirit_id": "reviewer"}),
        digest,
        serde_json::json!({"event": "drain", "topology": true}),
    ]
}

/// The recorded shape of the whole hermetic epic scene: three class Spirits,
/// section 1, the section-2 judgement and kernel halt, the door's resolution
/// releasing section 2 with the answer, section 3, the idle passes, the FR17
/// digest citing every completion and the RESOLVED halt, the drain.
fn epic_stream() -> Vec<serde_json::Value> {
    let mut events = vec![
        serde_json::json!({"event": "spirit_loaded", "spirit_id": "orchestrator"}),
        serde_json::json!({"event": "spirit_loaded", "spirit_id": "architect"}),
        serde_json::json!({"event": "spirit_loaded", "spirit_id": "reviewer"}),
        judged(1),
    ];
    events.extend(section_events(1));
    events.extend([judged(2), dispatch_halted(), dispatch_resumed()]);
    events.extend(section_events(2));
    events.push(judged(3));
    events.extend(section_events(3));
    events.extend(idle_and_drain(morning_digest(
        &COMPLETION_REFS,
        &[],
        &[HALT_FRAME],
    )));
    events
}

/// The `maosctl audit query --format ndjson` rows the digest beat joins on.
fn audit_rows() -> String {
    COMPLETION_REFS
        .iter()
        .map(|r| (*r, "task.complete"))
        .chain([(HALT_FRAME, "epistemic.halt")])
        .map(|(frame_id, kind)| {
            serde_json::json!({"frame_id": frame_id, "kind": kind, "intent": "x"}).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn replace_event(
    events: &mut [serde_json::Value],
    pred: impl Fn(&serde_json::Value) -> bool,
    with: serde_json::Value,
) {
    let at = events
        .iter()
        .position(pred)
        .expect("the event to replace is in the stream");
    events[at] = with;
}

fn is_event(name: &'static str) -> impl Fn(&serde_json::Value) -> bool {
    move |e| e["event"].as_str() == Some(name)
}

/// AC1.2 — every executed beat holds against the recorded epic stream: the
/// nine per-section beats (P1–P6) and the three new epic beats, and the digest
/// beat against its audit rows.
#[test]
fn the_full_epic_stream_proves_every_executed_beat() {
    let observation = obs(epic_stream(), "");
    let beats = evaluate_beats(&observation);
    let failed: Vec<(&str, &str)> = beats
        .iter()
        .filter(|b| b.failed())
        .map(|b| (b.name, b.detail.as_str()))
        .collect();
    assert!(failed.is_empty(), "unexpected failures: {failed:?}");
    let names: Vec<&str> = beats.iter().map(|b| b.name).collect();
    assert_eq!(
        names,
        vec![
            "topology-spirits-loaded",
            "delegation-frame-crosses-loopback",
            "worker-admitted-under-host-grant",
            "worker-completed-by-adapter-oracle",
            "delegation-closed-at-safe-point",
            "worker-exited-and-loop-went-idle",
            "state-home-clean",
            "lifecycle-stages-in-order",
            "audit-drain-clean",
            "epic-decomposed-n-frames",
            "ambiguity-halt-within-2-frames",
            "halt-resume-referential-identity",
        ],
        "all twelve executed scene beats, in claim-table order"
    );
    assert!(beats.iter().all(|b| b.executed));
    let digest = digest_cites_source_refs(&observation, &audit_rows());
    assert_eq!(digest.name, DIGEST_BEAT);
    assert!(!digest.failed(), "{}", digest.detail);
}

/// AC1.2 — `unlanded_beats()` keeps exactly the two beats only an operator or
/// two paid hosts can earn; the resume beat is executed now, never declared.
#[test]
fn only_the_tier2_and_two_host_beats_stay_unlanded() {
    let names: Vec<&str> = unlanded_beats().iter().map(|b| b.name).collect();
    assert_eq!(names, vec![TIER2_BEAT, "two-host-signed-run"]);
}

/// AC1.2 — a section dispatched out of order (frames 2 and 3 swapped) is not a
/// decomposition in section order.
#[test]
fn sections_dispatched_out_of_order_fail_the_decomposition_beat() {
    let mut events = epic_stream();
    let goals: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e["event"] == "delegation_routed")
        .map(|(at, _)| at)
        .collect();
    let (second, third) = (events[goals[1]].clone(), events[goals[2]].clone());
    events[goals[1]] = third;
    events[goals[2]] = second;
    let beat = beat_named(&obs(events, ""), "epic-decomposed-n-frames");
    assert!(beat.failed(), "{}", beat.detail);
}

/// AC2.6 — the proven-red twin, in-tree: the clear-judgement cassette's stream
/// (section 2 judged 0.31, no halt id, no `dispatch_halted`, no resume) must
/// FAIL `ambiguity-halt-within-2-frames` — the beat is the kernel's halt, not
/// the presence of a judgement.
#[test]
fn a_clear_judgement_stream_fails_the_ambiguity_halt_beat() {
    let mut events = epic_stream();
    events.retain(|e| !matches!(e["event"].as_str(), Some("dispatch_halted" | "dispatch_resumed")));
    replace_event(
        &mut events,
        |e| e["event"] == "dispatch_judged" && e["section"] == 2,
        serde_json::json!({
            "event": "dispatch_judged",
            "section": 2,
            "tag": AMBIGUITY_TAG,
            "value": 0.31,
            "halt_id": null,
            "halt_frame_id": null,
        }),
    );
    let beat = beat_named(&obs(events, ""), "ambiguity-halt-within-2-frames");
    assert!(beat.failed(), "a clear judgement raises no halt: {}", beat.detail);
    assert!(beat.detail.contains("the kernel raised no halt"), "{}", beat.detail);
}

/// ADR-064 — an unset inference mode judges nothing: no `dispatch_judged` at
/// all is never read as a halt.
#[test]
fn a_stream_without_judgements_fails_the_ambiguity_halt_beat() {
    let mut events = epic_stream();
    events.retain(|e| e["event"] != "dispatch_judged");
    let beat = beat_named(&obs(events, ""), "ambiguity-halt-within-2-frames");
    assert!(beat.failed());
    assert!(beat.detail.contains("no dispatch_judged"), "{}", beat.detail);
}

/// AC1.2 — a halt that lands only after three frames is not "within 2 frames".
#[test]
fn a_halt_after_three_frames_fails_the_ambiguity_halt_beat() {
    let mut events = epic_stream();
    events.retain(|e| e["event"] != "dispatch_halted");
    let drain_at = events
        .iter()
        .position(is_event("on_idle_fired"))
        .expect("idle passes");
    events.insert(drain_at, dispatch_halted());
    let beat = beat_named(&obs(events, ""), "ambiguity-halt-within-2-frames");
    assert!(beat.failed(), "{}", beat.detail);
}

/// AC5.4 — the proven-red twin, over the recorded stream of the plant (the
/// demo's resolve step removed): the hold expires, the 19-1 stop drains after
/// ONE frame, the digest lists the halt as OPEN — the halt beat still holds,
/// but `halt-resume-referential-identity` FAILS.
#[test]
fn the_resolve_step_removed_fails_the_resume_beat() {
    let mut events = vec![
        serde_json::json!({"event": "spirit_loaded", "spirit_id": "orchestrator"}),
        serde_json::json!({"event": "spirit_loaded", "spirit_id": "architect"}),
        serde_json::json!({"event": "spirit_loaded", "spirit_id": "reviewer"}),
        judged(1),
    ];
    events.extend(section_events(1));
    events.extend([judged(2), dispatch_halted()]);
    events.extend(idle_and_drain(morning_digest(
        &COMPLETION_REFS[..1],
        &[HALT_FRAME],
        &[],
    )));
    let observation = obs(events, "");
    let halt = beat_named(&observation, "ambiguity-halt-within-2-frames");
    assert!(!halt.failed(), "the halt itself held: {}", halt.detail);
    let resume = beat_named(&observation, "halt-resume-referential-identity");
    assert!(resume.failed(), "no resolution, no resume: {}", resume.detail);
    assert!(resume.detail.contains("resumed false"), "{}", resume.detail);
    assert!(
        beat_named(&observation, "epic-decomposed-n-frames").failed(),
        "one frame of three is not the decomposed epic"
    );
}

/// AC3.2 / AC5.3 — a digest that still lists the resolved halt as OPEN (the
/// mislabel D-19-4-07 B would have read) fails the resume beat even though the
/// resume itself happened.
#[test]
fn a_resolved_halt_listed_open_fails_the_resume_beat() {
    let mut events = epic_stream();
    replace_event(
        &mut events,
        is_event("morning_digest"),
        morning_digest(&COMPLETION_REFS, &[HALT_FRAME], &[]),
    );
    let beat = beat_named(&obs(events, ""), "halt-resume-referential-identity");
    assert!(beat.failed(), "{}", beat.detail);
}

/// AC5.3 — the digest must cite the EXACT pre-halt frame: a resolved halt with
/// another frame id is a different halt.
#[test]
fn a_resolved_halt_citing_another_frame_fails_the_resume_beat() {
    let mut events = epic_stream();
    replace_event(
        &mut events,
        is_event("morning_digest"),
        morning_digest(&COMPLETION_REFS, &[], &["03190000000000000000000000000099"]),
    );
    let beat = beat_named(&obs(events, ""), "halt-resume-referential-identity");
    assert!(beat.failed(), "{}", beat.detail);
}

/// AC5.1 — a resume whose section-2 goal does not carry the operator's answer
/// did not resume WITH it.
#[test]
fn a_resumed_section_without_the_clarification_fails_the_resume_beat() {
    let mut events = epic_stream();
    for event in events.iter_mut() {
        if event["event"] == "delegation_routed" {
            let goal = event["goal"].as_str().unwrap_or_default().replace(CLARIFICATION, "");
            event["goal"] = serde_json::json!(goal);
        }
    }
    let beat = beat_named(&obs(events, ""), "halt-resume-referential-identity");
    assert!(beat.failed(), "{}", beat.detail);
}

/// AC3.3 — the proven-red plant: ONE extra completion ref the audit log does
/// not hold (a hallucinated completion) fails `digest-cites-source-refs` and is
/// named in the finding.
#[test]
fn a_digest_citing_an_extra_completion_fails_the_digest_beat() {
    let hallucinated = "01190000000000000000000000000004";
    let mut events = epic_stream();
    let mut cited = COMPLETION_REFS.to_vec();
    cited.push(hallucinated);
    replace_event(
        &mut events,
        is_event("morning_digest"),
        morning_digest(&cited, &[], &[HALT_FRAME]),
    );
    let beat = digest_cites_source_refs(&obs(events, ""), &audit_rows());
    assert!(beat.failed(), "{}", beat.detail);
    assert!(
        beat.detail.contains("name no matching audit row of their kind")
            && beat.detail.contains(&format!("{:?}", short_ref(hallucinated))),
        "the finding names the hallucinated ref: {}",
        beat.detail
    );
}

/// AC3.3 — a ref of the WRONG kind (a halt ref naming a completion row) is not
/// a citation of a halt.
#[test]
fn a_halt_ref_naming_a_completion_row_fails_the_digest_beat() {
    let mut events = epic_stream();
    replace_event(
        &mut events,
        is_event("morning_digest"),
        morning_digest(&COMPLETION_REFS, &[], &[COMPLETION_REFS[0]]),
    );
    let beat = digest_cites_source_refs(&obs(events, ""), &audit_rows());
    assert!(beat.failed(), "{}", beat.detail);
}

/// AC3.3 — fewer completions than sections, no digest, or two digests are
/// never "the digest cites the run".
#[test]
fn a_missing_short_or_duplicated_digest_fails_the_digest_beat() {
    let mut short = epic_stream();
    replace_event(
        &mut short,
        is_event("morning_digest"),
        morning_digest(&COMPLETION_REFS[..2], &[], &[HALT_FRAME]),
    );
    let mut missing = epic_stream();
    missing.retain(|e| e["event"] != "morning_digest");
    let mut doubled = epic_stream();
    doubled.push(morning_digest(&COMPLETION_REFS, &[], &[HALT_FRAME]));
    for (label, events) in [("short", short), ("missing", missing), ("doubled", doubled)] {
        let beat = digest_cites_source_refs(&obs(events, ""), &audit_rows());
        assert!(beat.failed(), "{label}: {}", beat.detail);
    }
}

/// The claim table is titled "execution order". Before the 2026-08-15 review each
/// beat independently took the FIRST event with a matching name, so a stream whose
/// completion preceded its own delegation — or which repeated a stage — could
/// satisfy every beat and still be printed as an execution-order proof. Both
/// plants below go into the otherwise-green epic stream (Story 19-4).
#[test]
fn a_completion_before_its_own_delegation_fails_the_order_beat() {
    let mut events = epic_stream();
    let routed = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e["event"] == "delegation_routed")
        .map(|(at, _)| at)
        .nth(1)
        .expect("section 2's frame");
    let completion = events
        .iter()
        .skip(routed)
        .position(is_event("worker_completion"))
        .map(|offset| routed + offset)
        .expect("section 2's completion");
    let moved = events.remove(completion);
    events.insert(routed, moved);
    let beat = beat_named(&obs(events, ""), "lifecycle-stages-in-order");
    assert!(
        beat.failed(),
        "a completion that precedes its delegation is not an execution order"
    );
}

#[test]
fn a_repeated_lifecycle_stage_fails_the_order_beat() {
    let mut events = epic_stream();
    let completion = events
        .iter()
        .position(is_event("worker_completion"))
        .expect("section 1's completion");
    let duplicate = events[completion].clone();
    events.insert(completion + 1, duplicate);
    let beat = beat_named(&obs(events, ""), "lifecycle-stages-in-order");
    assert!(
        beat.failed(),
        "two worker_completion rows means the run cannot claim one correlated worker run"
    );
    assert!(
        beat.detail.contains("stream order NOT proven"),
        "the narration must name the broken order, got {:?}",
        beat.detail
    );
}

/// AC1 names `cli_wrapper_exit` and `on_idle_fired` ×3 as evidence. Before the
/// 2026-08-15 review the scene parsed neither, so a run whose worker never
/// reported an exit — or whose Spirits never went idle — still narrated success.
#[test]
fn a_missing_worker_exit_row_fails_the_idle_beat() {
    let observation = obs(
        vec![
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "orchestrator"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "architect"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "reviewer"}),
        ],
        "",
    );
    let beat = beat_named(&observation, "worker-exited-and-loop-went-idle");
    assert!(beat.failed(), "no cli_wrapper_exit row is not a clean loop");
}

#[test]
fn a_crashed_worker_exit_fails_the_idle_beat() {
    let mut events = epic_stream();
    let exit = events
        .iter()
        .rposition(is_event("cli_wrapper_exit"))
        .expect("section 3's exit");
    events[exit]["exit_cause"] = serde_json::json!("Signaled { signal: 9 }");
    events[exit]["is_crash"] = serde_json::json!(true);
    let beat = beat_named(&obs(events, ""), "worker-exited-and-loop-went-idle");
    assert!(beat.failed(), "a crash is never a clean exit");
}

#[test]
fn a_spirit_that_never_idled_fails_the_idle_beat() {
    let exit = serde_json::json!({
        "event": "cli_wrapper_exit",
        "exit_cause": "Exited { code: 0 }",
        "is_crash": false,
    });
    let observation = obs(
        vec![
            exit.clone(),
            exit.clone(),
            exit,
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "orchestrator"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "architect"}),
        ],
        "",
    );
    let beat = beat_named(&observation, "worker-exited-and-loop-went-idle");
    assert!(
        beat.failed(),
        "two idle callbacks for three class Spirits is not proof the loop went idle"
    );
    assert!(beat.detail.contains("on_idle_fired 2/3"), "{}", beat.detail);
}

/// The daemon prints `audit writer task failed during topology drain` and STILL
/// exits 0 (`maos-bin/src/main.rs:4314`). Queued rows are not proven persisted,
/// so the drain beat must not read green on that stderr.
#[test]
fn a_writer_task_failure_fails_the_drain_beat() {
    let observation = obs(
        vec![serde_json::json!({"event": "drain"})],
        "maos run: audit writer task failed during topology drain: channel closed",
    );
    let beat = beat_named(&observation, "audit-drain-clean");
    assert!(
        beat.failed(),
        "a failed writer join means the queue was never proven flushed"
    );
    assert!(
        beat.detail.contains("AUDIT WRITER TASK FAILED"),
        "the narration must name the writer failure, got {:?}",
        beat.detail
    );
}

/// `verify-bundle` requires `--pubkey` and has no default, so the hex must be
/// carried from what `sealed-export` printed. Guessing is not an option.
#[test]
fn the_sealed_export_pubkey_is_carried_into_verification() {
    let hex = "61f4f495dba703e74aff7d42b4286a1a914a89b592a98bf76ed3656c81107766";
    assert_eq!(
        pubkey_hex(&format!(
            "maosctl: sealed export written to /tmp/b.json (247 entries, pubkey {hex})"
        )),
        Some(hex)
    );
    // Too short, too long, and non-hex must all be refused rather than passed on
    // as a bogus key that would fail verification after the paid run.
    assert_eq!(pubkey_hex("(247 entries, pubkey deadbeef)"), None);
    assert_eq!(pubkey_hex(&format!("pubkey {}", "z".repeat(64))), None);
    assert_eq!(pubkey_hex("no pubkey at all"), None);
}

#[test]
fn entry_count_reads_the_verify_line() {
    assert_eq!(
        entry_count("audit verify-bundle — OK (247 entries, seq 12)"),
        247
    );
    assert_eq!(entry_count("no count here"), 0);
}

// ── Review 2a-P11 — the signing controls had ZERO regression coverage ───────

/// A topology+worker fixture pair laid into a tempdir, so `resolve_topology_worker`
/// is tested against REAL files the way the preflight reads them.
fn lay_topology(worker_manifests: &[(&str, &str)]) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let mut entries = String::new();
    for (i, (command, argv)) in worker_manifests.iter().enumerate() {
        let m = dir.path().join(format!("worker-{i}.toml"));
        std::fs::write(
            &m,
            format!(
                "[cli_wrapper]\ncommand = \"{command}\"\nargv_prefix = {argv}\n\
                 output_shape_version = \"1.0.0\"\n[sandbox]\ntier = \"T3\"\n"
            ),
        )
        .unwrap();
        entries.push_str(&format!(
            "[[topology.spirits]]\nmanifest = \"{}\"\nhost = \"developer-remote-host\"\n",
            m.display()
        ));
    }
    let topo = dir.path().join("topology.toml");
    std::fs::write(&topo, format!("[topology]\nname = \"t\"\n\n{entries}")).unwrap();
    (dir, topo)
}

#[test]
fn resolve_topology_worker_resolves_the_single_wrapper() {
    let (_dir, topo) = lay_topology(&[(
        "codex",
        r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
    )]);
    let w = resolve_topology_worker(&topo).unwrap();
    assert_eq!(w.cli.name(), "codex");
    assert_eq!(w.argv_prefix.len(), 4);
}

#[test]
fn resolve_topology_worker_refuses_a_topology_with_no_wrapper() {
    let dir = tempfile::tempdir().unwrap();
    let topo = dir.path().join("topology.toml");
    std::fs::write(
        &topo,
        "[topology]\nname = \"t\"\n\n[[topology.spirits]]\nmanifest = \"../orchestrator/manifest.toml\"\n",
    )
    .unwrap();
    let err = resolve_topology_worker(&topo)
        .map(|_| ())
        .unwrap_err();
    assert!(err.contains("no [cli_wrapper] member"), "got: {err}");
}

#[test]
fn resolve_topology_worker_refuses_multiple_wrappers() {
    // Review 2a-P5 — production runs EVERY wrapper; the signing preflight
    // attests ONE. Two wrappers must be refused, not silently first-picked.
    let (_dir, topo) = lay_topology(&[
        (
            "codex",
            r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
        ),
        (
            "claude",
            r#"["--print", "--output-format", "json", "--bare"]"#,
        ),
    ]);
    let err = resolve_topology_worker(&topo).map(|_| ()).unwrap_err();
    assert!(
        err.contains("2 [cli_wrapper] members"),
        "must name the count: {err}"
    );
    assert!(
        err.contains("unverified"),
        "must say WHY multi-worker topologies are refused: {err}"
    );
}

#[test]
fn resolve_topology_worker_refuses_an_unsupported_cli() {
    let (_dir, topo) = lay_topology(&[("rm", "[]")]);
    let err = resolve_topology_worker(&topo).map(|_| ()).unwrap_err();
    assert!(err.contains("no adapter supports"), "got: {err}");
}


#[test]
fn the_signable_allowlist_admits_both_real_adapters_and_refuses_the_fixture() {
    // AC3.4/F8 — WIDENED to an allowlist, never deleted: the fixture must never
    // earn PROVEN_LIVE_SIGNED, and an unknown identity must never appear in it.
    assert!(SIGNABLE_WORKER_CLIS.contains(&"codex"));
    assert!(SIGNABLE_WORKER_CLIS.contains(&"claude"));
    assert!(!SIGNABLE_WORKER_CLIS.contains(&"worker-cli-fixture"));
    assert_eq!(SIGNABLE_WORKER_CLIS.len(), 2);
}

// ── §A6 review P9 (AC3.5) — the F3 fix gets committed regressions ──────────

/// The F3 contract, pinned on the pure seam with an INJECTED present capture:
/// at HEAD the capture is absent, so only injection can make this vector able
/// to fail (Trap 17). A present-but-unproven capture must render as a visible
/// non-claim — `--` mark, INDETERMINATE, owner retained — and must not fail.
#[test]
fn two_host_beat_with_present_capture_renders_indeterminate_not_fail() {
    let mut beats = unlanded_beats();
    let judgement = crate::check_j1_two_host_signed_run::Judgement {
        findings: vec![],
        audits: vec![],
        capture_present: true,
        enrolled: vec![],
    };
    apply_capture_judgement(&mut beats, &judgement);
    let beat = beats
        .iter()
        .find(|b| b.name == "two-host-signed-run")
        .expect("the beat is declared by `unlanded_beats`");
    assert_eq!(beat.state.as_str(), "INDETERMINATE");
    assert!(
        !beat.executed,
        "F3: a present-but-unproven capture must NOT mark the beat executed"
    );
    assert!(!beat.failed(), "the beat must not fail the scene");
    assert_eq!(
        beat.owner,
        Some("j1-crosshost-2d-paid-two-host-run"),
        "the owner is retained — the rung still belongs to 2d"
    );
    let row = render_beat_row(beat);
    assert!(
        row.contains("two-host-signed-run"),
        "the rendered row names the beat: {row}"
    );
    assert!(
        row.contains("INDETERMINATE"),
        "the rendered row carries the honest state: {row}"
    );
    assert!(
        !row.contains("FAIL"),
        "a present-but-unproven capture is not a failure: {row}"
    );
    assert!(
        row.starts_with("  --"),
        "the rendered row uses the non-claim mark: {row}"
    );
}

/// The judge-refused direction of the same seam: findings present changes the
/// detail text, never the shape — refused is still not failed.
#[test]
fn two_host_beat_with_refused_capture_still_renders_indeterminate_not_fail() {
    let mut beats = unlanded_beats();
    let judgement = crate::check_j1_two_host_signed_run::Judgement {
        findings: vec![crate::check_j1_two_host_signed_run::Finding {
            check: "paid-run-capture",
            detail: "synthetic refusal for the §A6 P9 vector".to_string(),
        }],
        audits: vec![],
        capture_present: true,
        enrolled: vec![],
    };
    apply_capture_judgement(&mut beats, &judgement);
    let beat = beats
        .iter()
        .find(|b| b.name == "two-host-signed-run")
        .expect("the beat is declared");
    assert_eq!(beat.state.as_str(), "INDETERMINATE");
    assert!(!beat.executed && !beat.failed());
    let row = render_beat_row(beat);
    assert!(row.contains("INDETERMINATE") && !row.contains("FAIL"), "row: {row}");
    assert!(
        row.contains("the claim is refused"),
        "the refused direction names the refusal: {row}"
    );
}
