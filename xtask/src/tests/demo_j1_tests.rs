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
/// exists.
#[test]
fn a_delegation_to_the_wrong_host_fails() {
    let observation = obs(
        vec![
            serde_json::json!({
                "event": "delegation_routed",
                "to_host": "somewhere-else",
                "recipient": DELEGATION_RECIPIENT,
                "intent": "development-task:write-workspace",
                "goal": "do the thing",
            }),
            serde_json::json!({"event": "topology_worker_admit", "frame_borne": true}),
        ],
        "",
    );
    let beats = evaluate_beats(&observation);
    let beat = beats
        .iter()
        .find(|b| b.name == "delegation-frame-crosses-loopback")
        .expect("beat present");
    assert!(beat.failed());
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
    // (a) The oracle said completed. A missing evidence pointer does not overturn
    //     the verdict — and the ref is deliberately still emitted on failures, so
    //     absence here means the worker printed nothing, not that it failed.
    let observation = obs(
        vec![serde_json::json!({
            "event": "worker_completion",
            "worker_cli": "worker-cli-fixture",
            "completion": "completed",
            "completed": true,
            "last_stdout_tl_ref": serde_json::Value::Null,
        })],
        "",
    );
    let beats = evaluate_beats(&observation);
    let beat = beats
        .iter()
        .find(|b| b.name == "worker-completed-by-adapter-oracle")
        .expect("beat present");
    assert!(
        !beat.failed(),
        "the oracle's verdict is the beat; the TL ref is an evidence pointer"
    );

    // (b) The load-bearing direction: a NON-completion with a perfectly good TL
    //     ref must fail. Under the old conjunction this case was reachable only by
    //     accident; it is the whole point now.
    let observation = obs(
        vec![serde_json::json!({
            "event": "worker_completion",
            "worker_cli": "codex",
            "completion": "not_completed:no_effect_evidence",
            "completed": false,
            "last_stdout_tl_ref": "01a003bfb38d20e97fcb08c16274374c",
        })],
        "",
    );
    let beats = evaluate_beats(&observation);
    let beat = beats
        .iter()
        .find(|b| b.name == "worker-completed-by-adapter-oracle")
        .expect("beat present");
    assert!(
        beat.failed(),
        "a citable TL ref must never launder a non-completion into a pass"
    );
}

/// The verbatim post-1a stream from the 2026-08-14 isolated run: every executed
/// beat must hold against the real event shapes, not against invented ones.
#[test]
fn the_full_post_1a_stream_proves_every_executed_beat() {
    let observation = obs(
        vec![
            serde_json::json!({"event": "spirit_loaded", "spirit_id": "orchestrator"}),
            serde_json::json!({"event": "spirit_loaded", "spirit_id": "architect"}),
            serde_json::json!({"event": "spirit_loaded", "spirit_id": "reviewer"}),
            serde_json::json!({
                "event": "delegation_routed",
                "to_host": DELEGATION_HOST,
                "recipient": DELEGATION_RECIPIENT,
                "intent": "development-task:write-workspace",
                "goal": "founder-loop: execute the delegated assignment from founder-loop-host",
            }),
            serde_json::json!({"event": "topology_worker_admit", "frame_borne": true}),
            serde_json::json!({
                "event": "host_grant_disposition",
                "granted_tier": "SandboxTier(3)",
                "egress": "declared-not-enforced",
            }),
            serde_json::json!({
                "event": "cli_wrapper_loaded",
                "granted_tier": "SandboxTier(3)",
                "child_pid": 1308069,
            }),
            serde_json::json!({
                "event": "worker_completion",
                "worker_cli": "worker-cli-fixture",
                "completion": "completed",
                "completed": true,
                "last_stdout_tl_ref": "01a003bfb38d20e97fcb08c16274374c",
            }),
            serde_json::json!({
                "event": "cli_wrapper_exit",
                "child_pid": 1308069,
                "stdout_lines": 3,
                "stderr_lines": 0,
                "exit_cause": "Exited { code: 0 }",
                "is_crash": false,
            }),
            serde_json::json!({
                "event": "delegation_completed",
                "result": "completed",
                "orchestrator_frames_drained": 1,
                "orchestrator_safe_point": true,
            }),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "orchestrator"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "architect"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "reviewer"}),
            serde_json::json!({"event": "drain"}),
        ],
        "",
    );
    let beats = evaluate_beats(&observation);
    let failed: Vec<&str> = beats
        .iter()
        .filter(|b| b.failed())
        .map(|b| b.name)
        .collect();
    assert!(failed.is_empty(), "unexpected failures: {failed:?}");
    assert_eq!(beats.len(), 9, "all nine executed beats must be evaluated");
}

/// The claim table is titled "execution order". Before the 2026-08-15 review each
/// beat independently took the FIRST event with a matching name, so a stream whose
/// completion preceded its own delegation — or which repeated a stage — could
/// satisfy every beat and still be printed as an execution-order proof.
#[test]
fn a_completion_before_its_own_delegation_fails_the_order_beat() {
    let observation = obs(
        vec![
            serde_json::json!({"event": "worker_completion", "completed": true}),
            serde_json::json!({"event": "delegation_routed", "to_host": DELEGATION_HOST}),
            serde_json::json!({"event": "cli_wrapper_loaded", "child_pid": 7}),
            serde_json::json!({"event": "delegation_completed", "result": "completed"}),
            serde_json::json!({"event": "drain"}),
        ],
        "",
    );
    let beat = beat_named(&observation, "lifecycle-stages-in-order");
    assert!(
        beat.failed(),
        "a completion that precedes its delegation is not an execution order"
    );
}

#[test]
fn a_repeated_lifecycle_stage_fails_the_order_beat() {
    let observation = obs(
        vec![
            serde_json::json!({"event": "delegation_routed", "to_host": DELEGATION_HOST}),
            serde_json::json!({"event": "cli_wrapper_loaded", "child_pid": 7}),
            serde_json::json!({"event": "worker_completion", "completed": true}),
            serde_json::json!({"event": "worker_completion", "completed": false}),
            serde_json::json!({"event": "delegation_completed", "result": "completed"}),
            serde_json::json!({"event": "drain"}),
        ],
        "",
    );
    let beat = beat_named(&observation, "lifecycle-stages-in-order");
    assert!(
        beat.failed(),
        "two worker_completion rows means the run cannot claim one correlated worker run"
    );
    assert!(
        beat.detail.contains("not once"),
        "the narration must name the duplicate, got {:?}",
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
    let observation = obs(
        vec![
            serde_json::json!({
                "event": "cli_wrapper_exit",
                "exit_cause": "Signaled { signal: 9 }",
                "is_crash": true,
            }),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "orchestrator"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "architect"}),
            serde_json::json!({"event": "on_idle_fired", "spirit_id": "reviewer"}),
        ],
        "",
    );
    let beat = beat_named(&observation, "worker-exited-and-loop-went-idle");
    assert!(beat.failed(), "a crash is never a clean exit");
}

#[test]
fn a_spirit_that_never_idled_fails_the_idle_beat() {
    let observation = obs(
        vec![
            serde_json::json!({
                "event": "cli_wrapper_exit",
                "exit_cause": "Exited { code: 0 }",
                "is_crash": false,
            }),
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
fn command_metadata_is_derived_and_never_says_injected() {
    // Review 2a-P11 + AC3.5/F22 — the sealed `command_metadata` must name the
    // manifest, the topology, the adapter identity and the adapter's OWN
    // credential variable, and must say INHERITED, never "injected host-side"
    // (MAOS holds no credential and injects nothing).
    let manifest = std::path::Path::new("/tmp/manifest-codex.toml");
    let topology = std::path::Path::new("/tmp/topo.toml");
    let argv: Vec<String> = ["exec", "--json"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let meta = derive_command_metadata(manifest, topology, "codex", &argv, "CODEX_API_KEY");
    assert!(meta.contains("manifest-codex.toml"), "got: {meta}");
    assert!(meta.contains("topo.toml"), "got: {meta}");
    assert!(meta.contains("`codex`"), "got: {meta}");
    assert!(meta.contains("CODEX_API_KEY"), "got: {meta}");
    assert!(
        meta.contains("inherited from the operator's environment"),
        "got: {meta}"
    );
    assert!(!meta.contains("injected host-side"), "got: {meta}");
    // A claude run derives ANTHROPIC_API_KEY — the variable the scan targets.
    let meta = derive_command_metadata(manifest, topology, "claude", &argv, "ANTHROPIC_API_KEY");
    assert!(meta.contains("ANTHROPIC_API_KEY"), "got: {meta}");
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
