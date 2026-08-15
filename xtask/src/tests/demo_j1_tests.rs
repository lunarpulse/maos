use super::*;

fn obs(events: Vec<serde_json::Value>, stderr: &str) -> SceneObservation {
    SceneObservation {
        events,
        stderr: stderr.to_string(),
        wall: std::time::Duration::from_millis(180),
        exit_ok: true,
    }
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

#[test]
fn the_two_host_rung_is_owned_by_crosshost_2() {
    let rung = unlanded_beats()
        .into_iter()
        .find(|b| b.name == "two-host-signed-run")
        .expect("declared");
    assert_eq!(rung.owner, Some("j1-crosshost-2"));
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

/// A raw exit code is never completion — only the adapter's oracle is, and it
/// must carry the worker TL ref a digest can cite.
#[test]
fn completion_requires_the_oracle_and_a_tl_ref() {
    let observation = obs(
        vec![serde_json::json!({
            "event": "worker_completion",
            "worker_cli": "worker-cli-fixture",
            "completion": "completed",
            "completed": true,
            "completion_tl_ref": serde_json::Value::Null,
        })],
        "",
    );
    let beats = evaluate_beats(&observation);
    let beat = beats
        .iter()
        .find(|b| b.name == "worker-completed-by-adapter-oracle")
        .expect("beat present");
    assert!(beat.failed(), "a completion with no TL ref is not evidence");
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
                "completion_tl_ref": "01a003bfb38d20e97fcb08c16274374c",
            }),
            serde_json::json!({
                "event": "delegation_completed",
                "result": "completed",
                "orchestrator_frames_drained": 1,
                "orchestrator_safe_point": true,
            }),
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
    assert_eq!(beats.len(), 7, "all seven executed beats must be evaluated");
}

#[test]
fn entry_count_reads_the_verify_line() {
    assert_eq!(
        entry_count("audit verify-bundle — OK (247 entries, seq 12)"),
        247
    );
    assert_eq!(entry_count("no count here"), 0);
}
