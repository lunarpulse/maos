#![forbid(unsafe_code)]

//! The Worker-CLI adapter seam's executable contracts (story `j1-crosshost-2a`,
//! AC1.1-1.3, AC2.1-2.3).
//!
//! These legs previously lived in `worker_cli.rs`'s in-`src` `#[cfg(test)] mod
//! tests`. That module is charged to `maos-bin`'s KLOC budget and is **never
//! executed by CI** — no CI job runs `-p maos-bin` unscoped, so all four
//! invocations use `--test <name>` and those assertions never ran. Moving them
//! here costs zero budget (`xtask/src/kloc_check.rs` excludes `tests/`), makes
//! them executable, and — the reason AC1.1 exists — lets the completion-oracle
//! vectors name `ClaudeCli`/`CodexCli`/`parse_completion` at all.

use maos_bin::worker_cli::*;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

// ── select_worker_cli ──────────────────────────────────────────────────

#[test]
fn select_resolves_fixture_codex_claude_by_basename() {
    assert_eq!(
        select_worker_cli("/tmp/target/debug/worker-cli-fixture")
            .unwrap()
            .name(),
        "worker-cli-fixture"
    );
    assert_eq!(select_worker_cli("/usr/bin/codex").unwrap().name(), "codex");
    assert_eq!(select_worker_cli("codex.exe").unwrap().name(), "codex");
    assert_eq!(select_worker_cli("claude").unwrap().name(), "claude");
}

#[test]
fn select_fails_closed_on_unsupported_wrapper() {
    // The fail-closed contract: an unknown wrapper yields no adapter, so the
    // caller refuses before spawn.
    assert!(select_worker_cli("/usr/bin/rm").is_none());
    assert!(select_worker_cli("bash").is_none());
}

#[test]
fn argv_appends_task_as_trailing_arg() {
    assert_eq!(FixtureCli.argv("do the thing"), s(&["do the thing"]));
    assert_eq!(CodexCli.argv("scaffold a CLI"), s(&["scaffold a CLI"]));
    assert_eq!(ClaudeCli.argv("scaffold a CLI"), s(&["scaffold a CLI"]));
}

#[test]
fn only_codex_declares_noninteractive_env_and_no_secret_leaks() {
    assert_eq!(
        CodexCli.nonsecret_env(),
        vec![("CODEX_NON_INTERACTIVE".to_string(), "1".to_string())]
    );
    assert!(FixtureCli.nonsecret_env().is_empty());
    assert!(ClaudeCli.nonsecret_env().is_empty());
    // No adapter's non-secret env may carry a credential-shaped key.
    for env in [
        CodexCli.nonsecret_env(),
        ClaudeCli.nonsecret_env(),
        FixtureCli.nonsecret_env(),
    ] {
        for (k, _) in env {
            assert!(!k.contains("KEY") && !k.contains("TOKEN") && !k.contains("SECRET"));
        }
    }
}

// ── T5 CI/local split gate ──────────────────────────────────────────────

#[test]
fn live_agent_gate_lets_the_fixture_run_without_the_flag() {
    assert!(live_agent_gate(FIXTURE_CLI_NAME, false).is_ok());
    assert!(live_agent_gate(FIXTURE_CLI_NAME, true).is_ok());
}

#[test]
fn live_agent_gate_refuses_real_cli_without_the_flag() {
    // The load-bearing negative: CI (no MAOS_LIVE_AGENT) cannot spawn a paid
    // agent — codex/claude are refused fail-closed.
    assert!(live_agent_gate("codex", false).is_err());
    assert!(live_agent_gate("claude", false).is_err());
}

#[test]
fn live_agent_gate_permits_real_cli_only_with_the_local_optin() {
    assert!(live_agent_gate("codex", true).is_ok());
    assert!(live_agent_gate("claude", true).is_ok());
}

#[test]
fn every_real_adapter_names_its_ambient_auth_footgun_but_the_fixture_is_immune() {
    use std::path::Path;
    // AC2.1 — line `assert_eq!(ClaudeCli.ambient_auth_path(home), None)` USED TO
    // LIVE HERE, under a comment reading "only codex names the footgun". It was a
    // false claim with a green test behind it, and it made `refuse_ambient_auth` a
    // no-op for claude. It is INVERTED, not accompanied by a second test.
    let home = Path::new("/home/demo");
    assert_eq!(
        CodexCli.ambient_auth_path(home),
        Some(home.join(".codex").join("auth.json"))
    );
    assert_eq!(
        ClaudeCli.ambient_auth_path(home),
        // Leading dot on the FILENAME, unlike codex's `auth.json`.
        Some(home.join(".claude").join(".credentials.json"))
    );
    // The hermetic fixture holds no credential of any kind — this immunity is
    // load-bearing: CI runs the fixture through the same live-path code.
    assert_eq!(FixtureCli.ambient_auth_path(home), None);

    // Refusal against a real temp home (std-only, no dep). Positive-before-
    // negative ordering is deliberate: absence PERMITS, then plant, then refuse.
    let tmp =
        std::env::temp_dir().join(format!("maos-authtest-{}-{}", std::process::id(), line!()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join(".codex")).unwrap();
    std::fs::create_dir_all(tmp.join(".claude")).unwrap();
    // Neither credential file exists yet → both permitted.
    assert!(refuse_ambient_auth(&CodexCli, &tmp).is_ok());
    assert!(refuse_ambient_auth(&ClaudeCli, &tmp).is_ok());
    // Plant each footgun → refused fail-closed, and the refusal returns the path.
    std::fs::write(tmp.join(".codex").join("auth.json"), b"{\"token\":\"x\"}").unwrap();
    assert_eq!(
        refuse_ambient_auth(&CodexCli, &tmp).unwrap_err(),
        tmp.join(".codex").join("auth.json")
    );
    std::fs::write(
        tmp.join(".claude").join(".credentials.json"),
        b"{\"claudeAiOauth\":{}}",
    )
    .unwrap();
    assert_eq!(
        refuse_ambient_auth(&ClaudeCli, &tmp).unwrap_err(),
        tmp.join(".claude").join(".credentials.json")
    );
    // The fixture is never shadowed by an ambient token from either provider.
    assert!(refuse_ambient_auth(&FixtureCli, &tmp).is_ok());
    let _ = std::fs::remove_dir_all(&tmp);
}

// ── admission probe strategy — fixture handshake vs real-CLI liveness ────

#[test]
fn only_the_fixture_speaks_the_bridge_handshake() {
    // The gap this closes: the kernel `--maos-bridge-probe` handshake is
    // fixture-only, so real CLIs must NOT be routed through it (they exit
    // non-zero and never admit). The fixture keeps it; codex/claude get a
    // `--version` liveness probe.
    assert_eq!(FixtureCli.probe_strategy(), ProbeStrategy::BridgeHandshake);
    assert_eq!(
        CodexCli.probe_strategy(),
        ProbeStrategy::Liveness {
            argv: s(&["--version"])
        }
    );
    assert_eq!(
        ClaudeCli.probe_strategy(),
        ProbeStrategy::Liveness {
            argv: s(&["--version"])
        }
    );
}

#[test]
fn liveness_probe_passes_on_clean_exit_and_fails_closed_otherwise() {
    use std::time::Duration;
    let t = Duration::from_secs(5);
    // A binary that exits 0 → admitted.
    assert!(run_liveness_probe("true", &[], t).is_ok());
    // A binary that exits non-zero → fail-closed (this is the codex-exit-2
    // admission failure the operator hit, now a clean typed refusal).
    let err = run_liveness_probe("false", &[], t).unwrap_err();
    assert!(err.contains("non-zero exit"), "got: {err}");
    // A missing binary → fail-closed on spawn.
    let err = run_liveness_probe("maos-no-such-binary-xyz", &[], t).unwrap_err();
    assert!(err.contains("spawn failed"), "got: {err}");
}

#[test]
fn liveness_probe_times_out_and_fails_closed_on_a_hang() {
    use std::time::Duration;
    // `sleep 5` under a 200ms budget → killed + refused (a hanging CLI must
    // not block admission), mirroring the kernel probe's 2s timeout guard.
    let err = run_liveness_probe("sleep", &s(&["5"]), Duration::from_millis(200)).unwrap_err();
    assert!(err.contains("timed out"), "got: {err}");
}

// ── the completion oracle — a raw exit is NEVER completion ──────────────

#[test]
fn fixture_completes_on_terminal_marker_line() {
    let stdout = s(&[
        "worker: received task assignment",
        "worker: executing fixture-replayed work",
        "worker: task complete",
    ]);
    let c = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
    assert!(c.is_completed());
    assert!(
        matches!(c, WorkerCompletion::Completed { final_message } if final_message.contains(FIXTURE_COMPLETION_MARKER))
    );
}

#[test]
fn fixture_exit0_without_marker_is_not_completion() {
    // The load-bearing negative: exit code 0 but no completion marker ⇒ NOT a
    // completion. A raw exit is never the oracle.
    let stdout = s(&["worker: started", "worker: still working"]);
    let c = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
    );
}

#[test]
fn fixture_crash_is_process_crash_even_with_marker_present() {
    // Even if the marker somehow appears, a non-clean exit is a crash — a
    // signaled death is never silently upgraded to completion.
    let stdout = s(&["worker: task complete"]);
    let c = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Crashed);
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { exit_code: None })
    );
    let c2 = FixtureCli.parse_completion(&stdout, &[], WorkerExit::Exited(137));
    assert_eq!(
        c2,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash {
            exit_code: Some(137)
        })
    );
}

// ── the codex oracle — `codex exec --json` JSONL, effect-based ───────────────

/// A `codex exec --json` line for an APPLIED patch (`codex-cli 0.144.4`
/// `ThreadEvent`/`ThreadItemDetails` shapes).
fn codex_file_change(status: &str) -> String {
    format!(
        r#"{{"type":"item.completed","item":{{"id":"i1","type":"file_change","changes":[{{"path":"/w/main.rs","kind":"add"}}],"status":"{status}"}}}}"#
    )
}

fn codex_agent_message(text: &str) -> String {
    format!(
        r#"{{"type":"item.completed","item":{{"id":"i2","type":"agent_message","text":"{text}"}}}}"#
    )
}

const CODEX_TURN_COMPLETED: &str = r#"{"type":"turn.completed","usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":2,"reasoning_output_tokens":0}}"#;
const CODEX_TURN_FAILED: &str = r#"{"type":"turn.failed","error":{"message":"model error"}}"#;
const CODEX_THREAD_STARTED: &str = r#"{"type":"thread.started","thread_id":"t1"}"#;

#[test]
fn codex_completes_only_with_turn_completed_and_applied_file_change() {
    let stdout = vec![
        CODEX_THREAD_STARTED.to_string(),
        codex_file_change("completed"),
        codex_agent_message("wrote main.rs and ran the test"),
        CODEX_TURN_COMPLETED.to_string(),
    ];
    let stderr = s(&["progress: thinking", "progress: writing files"]);
    let c = CodexCli.parse_completion(&stdout, &stderr, WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::Completed {
            final_message: "wrote main.rs and ran the test".to_string()
        }
    );
}

#[test]
fn codex_prose_final_line_is_no_longer_a_completion() {
    // THE SHIP-BLOCKER, as a vector. The old oracle was "clean exit + non-empty
    // final stdout line", so a worker that refused, explained itself fluently and
    // exited 0 scored `completed: true` over a file it never wrote. Prose is not
    // a machine-readable completion contract; a JSONL stream is.
    let stdout = s(&[
        "I'm not able to write to that path without approval.",
        "Let me know if you'd like me to try something else.",
    ]);
    let c = CodexCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
    );
}

#[test]
fn codex_turn_completed_without_a_file_change_is_no_effect_evidence() {
    // The airtight case: codex exits 0 on a COMPLETED turn that produced no file.
    // Sound because `required_argv_flags` refuses a codex manifest without
    // `--sandbox workspace-write`, so every admitted codex run is write-class.
    let stdout = vec![
        CODEX_THREAD_STARTED.to_string(),
        codex_agent_message("I would rather not modify that file."),
        CODEX_TURN_COMPLETED.to_string(),
    ];
    let c = CodexCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoEffectEvidence)
    );
}

#[test]
fn codex_failed_patch_is_not_effect_evidence() {
    // `PatchApplyStatus::Failed` means the patch did NOT land. An `item.completed`
    // is a lifecycle terminal, not a success — the `status` field is the verdict.
    let stdout = vec![
        codex_file_change("failed"),
        codex_agent_message("the patch did not apply"),
        CODEX_TURN_COMPLETED.to_string(),
    ];
    let c = CodexCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoEffectEvidence)
    );
}

#[test]
fn codex_turn_failed_is_a_typed_failure_not_a_missing_marker() {
    // The CLI said it failed — distinguishable from "said nothing legible".
    let stdout = vec![
        codex_file_change("completed"),
        CODEX_TURN_FAILED.to_string(),
    ];
    let c = CodexCli.parse_completion(&stdout, &[], WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::TurnFailed)
    );
}

#[test]
fn codex_exit0_but_empty_stdout_is_no_completion() {
    // Progress-only on stderr, nothing on stdout ⇒ codex emitted no event stream
    // ⇒ not a completion (Tier-2 stays open).
    let c = CodexCli.parse_completion(&[], &s(&["progress: ..."]), WorkerExit::Exited(0));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
    );
}

#[test]
fn codex_nonzero_exit_is_process_crash() {
    let c = CodexCli.parse_completion(&s(&["partial"]), &[], WorkerExit::Exited(1));
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::ProcessCrash { exit_code: Some(1) })
    );
}

// ── the claude oracle — one `--output-format json` result object ─────────────

fn claude_result(subtype: &str, is_error: bool, denials: &str, result: &str) -> String {
    format!(
        r#"{{"type":"result","subtype":"{subtype}","is_error":{is_error},"num_turns":3,"duration_ms":10,"result":"{result}","permission_denials":{denials},"total_cost_usd":0.01}}"#
    )
}

#[test]
fn claude_completes_on_a_clean_result_object_with_no_denials() {
    let c = ClaudeCli.parse_completion(
        &[claude_result("success", false, "[]", "patch applied")],
        &[],
        WorkerExit::Exited(0),
    );
    assert_eq!(
        c,
        WorkerCompletion::Completed {
            final_message: "patch applied".to_string()
        }
    );
}

#[test]
fn claude_permission_denial_is_refused_even_though_it_reports_success() {
    // The measured refusal shape on `claude 2.1.233`: `subtype: "success"`,
    // `is_error: false`, exit 0 — and `permission_denials` is the ONLY field that
    // distinguishes it. `--print` has no TTY to approve the tool call.
    let c = ClaudeCli.parse_completion(
        &[claude_result(
            "success",
            false,
            r#"[{"tool_name":"Write"}]"#,
            "I need permission to edit that file.",
        )],
        &[],
        WorkerExit::Exited(0),
    );
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::PermissionDenied)
    );
}

#[test]
fn claude_error_result_is_a_typed_failure() {
    let c = ClaudeCli.parse_completion(
        &[claude_result("error_during_execution", true, "[]", "boom")],
        &[],
        WorkerExit::Exited(0),
    );
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::TurnFailed)
    );
}

#[test]
fn claude_parses_a_pretty_printed_result_object_split_across_tl_rows() {
    // F24/Trap 23 — every stdout line becomes its own CliSubprocessOutput TL row.
    // A per-line parse (correct for codex JSONL) matches NO line of a
    // pretty-printed object and would turn a real success into a false negative.
    let c = ClaudeCli.parse_completion(
        &s(&[
            "{",
            "  \"type\": \"result\",",
            "  \"subtype\": \"success\",",
            "  \"is_error\": false,",
            "  \"result\": \"multi-line ok\",",
            "  \"permission_denials\": []",
            "}",
        ]),
        &[],
        WorkerExit::Exited(0),
    );
    assert_eq!(
        c,
        WorkerCompletion::Completed {
            final_message: "multi-line ok".to_string()
        }
    );
}

#[test]
fn claude_prose_only_output_is_not_a_completion() {
    // The live defect, on the claude side: a refusal in prose with exit 0.
    let c = ClaudeCli.parse_completion(
        &s(&["I wasn't able to write the file without permission."]),
        &[],
        WorkerExit::Exited(0),
    );
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
    );
}

#[test]
fn claude_absent_permission_denials_fails_closed() {
    // An absent field is NOT an empty array: absence means the run cannot PROVE
    // no tool permission was denied, and an unprovable claim fails closed.
    let c = ClaudeCli.parse_completion(
        &s(&[r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#]),
        &[],
        WorkerExit::Exited(0),
    );
    assert_eq!(
        c,
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoCompletionMarker)
    );
}

#[test]
fn the_two_oracles_are_not_equivalent_and_the_asymmetry_is_asserted() {
    // AC1.2's non-negotiable record, as an executable claim rather than prose:
    // codex proves EFFECT natively, claude does not. The SAME logical event — "the
    // model declined without attempting a tool call" — is caught on the codex side
    // (`NoEffectEvidence`) and is INDISTINGUISHABLE FROM SUCCESS on the claude
    // side. That residual is claude's and this story does not close it.
    let codex_declined = vec![
        codex_agent_message("I would rather not modify that file."),
        CODEX_TURN_COMPLETED.to_string(),
    ];
    assert_eq!(
        CodexCli.parse_completion(&codex_declined, &[], WorkerExit::Exited(0)),
        WorkerCompletion::NotCompleted(WorkerNonCompletion::NoEffectEvidence),
        "codex's stream carries native effect evidence, so a bare decline is caught"
    );
    let claude_declined = vec![claude_result(
        "success",
        false,
        "[]",
        "I would rather not modify that file.",
    )];
    assert!(
        ClaudeCli
            .parse_completion(&claude_declined, &[], WorkerExit::Exited(0))
            .is_completed(),
        "MEASURED RESIDUAL, not a bug in this test: a claude model that declines \
         WITHOUT attempting a tool call leaves permission_denials empty and is \
         indistinguishable from success. claude's JSON is a permission-denial \
         detector, not an effect oracle. Closing this needs an effect oracle \
         (deferred: it requires a kernel-core `cwd` on BridgeSpawnSpec)."
    );
}

// ── AC1.3 — an adapter can demand its argv flags, and the run refuses ────────
#[test]
fn adapters_declare_the_argv_flags_their_oracles_depend_on() {
    assert_eq!(
        CodexCli.required_argv_flags(),
        &[
            &["exec"][..],
            &["--json"][..],
            &["--sandbox", "workspace-write"][..]
        ]
    );
    assert_eq!(
        ClaudeCli.required_argv_flags(),
        &[
            &["--print"][..],
            &["--output-format", "json"][..],
            &["--bare"][..]
        ]
    );
    // The hermetic fixture's marker oracle needs no flags — the fixture path must
    // not acquire a new requirement (it would red the journey + drain suites).
    assert!(FixtureCli.required_argv_flags().is_empty());
}

#[test]
fn wrong_value_flag_combinations_are_refused_not_scatter_matched() {
    // Review 2a-P2 — token scatter must NOT satisfy a (flag, value) pair:
    // both manifests below carry every required TOKEN, and neither carries a
    // single required PAIR. Token-presence validation admitted both.
    let err = refuse_missing_argv_flags(
        &ClaudeCli,
        &s(&[
            "--print",
            "--output-format",
            "text",
            "--session-id",
            "json",
            "--bare",
        ]),
    )
    .unwrap_err();
    assert!(
        err.contains("--output-format json"),
        "must name the missing GROUP, not a token: {err}"
    );
    assert!(
        refuse_missing_argv_flags(
            &CodexCli,
            &s(&[
                "exec",
                "--sandbox",
                "read-only",
                "--cd",
                "workspace-write",
                "--json"
            ]),
        )
        .is_err(),
        "a `--sandbox read-only` + stray `workspace-write` token must not satisfy the pair"
    );
}

#[test]
fn bypass_flags_are_refused_at_the_production_seam() {
    // Review 2a-P2 — the bypass list used to live ONLY in the committed-manifest
    // reader; an operator-supplied topology could bypass on the live path while
    // the sealed capture kept asserting the declared posture.
    let err = refuse_unsafe_argv(
        &CodexCli,
        &s(&[
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "--dangerously-bypass-approvals-and-sandbox",
        ]),
    )
    .unwrap_err();
    assert!(err.contains("bypass"), "must name the bypass: {err}");
    let err = refuse_unsafe_argv(
        &ClaudeCli,
        &s(&[
            "--print",
            "--output-format",
            "json",
            "--bare",
            "--permission-mode",
            "bypassPermissions",
        ]),
    )
    .unwrap_err();
    assert!(
        err.contains("bypassPermissions"),
        "a bypass hiding as a flag VALUE must be caught (it also suppresses \
         permission_denials, the oracle's verdict): {err}"
    );
    // The fixture is exempt from posture controls by construction.
    assert!(refuse_unsafe_argv(&FixtureCli, &s(&["--maos-worker"])).is_ok());
}

#[test]
fn repeated_isolation_flags_are_refused() {
    // Review 2a-P2 — adapters re-parse repeated flags LAST-WINS, so a second
    // `--sandbox` makes the hashed posture and the effective one diverge.
    let err = refuse_unsafe_argv(
        &CodexCli,
        &s(&[
            "exec",
            "--json",
            "--sandbox",
            "workspace-write",
            "--sandbox",
            "read-only",
        ]),
    )
    .unwrap_err();
    assert!(err.contains("repeats"), "must name the repetition: {err}");
    assert!(refuse_unsafe_argv(
        &CodexCli,
        &s(&["exec", "--json", "--sandbox", "workspace-write"])
    )
    .is_ok());
}

#[test]
fn the_isolation_declaration_is_content_checked_not_token_checked() {
    // Review 2a-P3 — `{}` settings or a sandbox-less document must not count as
    // a jail, and codex without the long-form pair declares nothing.
    assert!(ClaudeCli
        .refuse_missing_isolation(&s(&[
            "--print",
            "--settings",
            "{\"sandbox\":{\"enabled\":true}}"
        ]))
        .is_ok());
    assert!(ClaudeCli
        .refuse_missing_isolation(&s(&["--print", "--settings", "{}"]))
        .is_err());
    assert!(ClaudeCli
        .refuse_missing_isolation(&s(&[
            "--print",
            "--settings",
            "{\"sandbox\":{\"enabled\":false}}"
        ]))
        .is_err());
    assert!(ClaudeCli
        .refuse_missing_isolation(&s(&["--print"]))
        .is_err());
    assert!(CodexCli
        .refuse_missing_isolation(&s(&["exec", "--sandbox", "workspace-write"]))
        .is_ok());
    assert!(CodexCli
        .refuse_missing_isolation(&s(&["exec", "--sandbox", "read-only"]))
        .is_err());
    // The fixture is exempt by construction (it writes no files).
    assert!(FixtureCli.refuse_missing_isolation(&[]).is_ok());
}

#[test]
fn a_manifest_that_omits_a_required_flag_is_refused() {
    // The negative AC1.3 mandates. Without this the inverse defect appears: an
    // adapter assuming JSON while the manifest ships prose turns a REAL success
    // into a false negative.
    let err = refuse_missing_argv_flags(&CodexCli, &s(&["exec", "--sandbox", "workspace-write"]))
        .unwrap_err();
    assert!(err.contains("--json"), "must name the missing token: {err}");
    assert!(err.contains("codex"), "must name the adapter: {err}");

    let err = refuse_missing_argv_flags(
        &ClaudeCli,
        &s(&[
            "--print",
            "--output-format",
            "json",
            "--permission-mode",
            "acceptEdits",
        ]),
    )
    .unwrap_err();
    assert!(
        err.contains("--bare"),
        "--bare is a reproducibility precondition, not optional: {err}"
    );
}

#[test]
fn a_manifest_carrying_every_required_flag_is_permitted() {
    assert!(refuse_missing_argv_flags(
        &CodexCli,
        &s(&["exec", "--sandbox", "workspace-write", "--json"])
    )
    .is_ok());
    assert!(refuse_missing_argv_flags(
        &ClaudeCli,
        &s(&[
            "--print",
            "--output-format",
            "json",
            "--bare",
            "--permission-mode",
            "acceptEdits"
        ])
    )
    .is_ok());
    // The fixture is permitted with the manifest it actually ships.
    assert!(refuse_missing_argv_flags(&FixtureCli, &s(&["--maos-worker"])).is_ok());
}

#[test]
fn label_is_nonsecret_and_never_leaks_message() {
    // Trap 10 — the intent is orthogonal to the oracle: the LABEL must carry no
    // message text. Repaired by changing the INPUT to something the stricter
    // oracle accepts, never by weakening the oracle.
    let c = CodexCli.parse_completion(
        &[
            codex_file_change("completed"),
            codex_agent_message("SECRET sk-abc123 leaked into the message"),
            CODEX_TURN_COMPLETED.to_string(),
        ],
        &[],
        WorkerExit::Exited(0),
    );
    // The label carries no message text (the message may echo redacted content).
    assert_eq!(c.label(), "completed");
    assert!(!c.label().contains("SECRET") && !c.label().contains("sk-"));

    // Every typed non-completion label is non-secret and stable.
    for nc in [
        WorkerNonCompletion::NoCompletionMarker,
        WorkerNonCompletion::TurnFailed,
        WorkerNonCompletion::NoEffectEvidence,
        WorkerNonCompletion::PermissionDenied,
        WorkerNonCompletion::ProcessCrash { exit_code: Some(1) },
    ] {
        let label = WorkerCompletion::NotCompleted(nc).label();
        assert!(label.starts_with("not_completed:"), "got: {label}");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AC1.4 — a planted refusal reds END-TO-END, hermetically, with NO live agent.
//
// The defect these vectors falsify was found by EXECUTION, not review: a live
// `claude -p` refused a write, exited 0, and the oracle scored it
// `completed: true`. That verdict is the admission condition for signing
// (`xtask/src/demo_j1.rs`: "the live worker did not complete — nothing to sign"),
// so a false completion on ONE host with no faults injected is a false signature.
//
// Three constraints make this a real control rather than a unit test in a costume:
//  * it runs the REAL `maos run` binary, so it exercises the composition root's
//    wiring (adapter selection, live gate, argv-flag refusal, TL read-back,
//    completion enforcement) and not just `parse_completion`;
//  * it goes through the TOPOLOGY path, because the standalone path used to
//    DISCARD the verdict — a vector run there would pass while the defect lived;
//  * `HOME` is isolated. `refuse_ambient_auth` reads the operator's REAL `$HOME`,
//    so a vector that leaks it passes or fails depending on whose laptop runs it.
//
// The adapter is `codex`/`claude` (basename dispatch) with `MAOS_LIVE_AGENT=1`,
// never `FixtureCli`: the fixture's marker oracle is structurally immune to this
// defect, so a fixture-emitted refusal would prove nothing.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod planted_refusal {
    use std::process::Command;

    fn workspace_root() -> &'static str {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
    }

    struct Tmp(std::path::PathBuf);

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn tmp(tag: &str) -> Tmp {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("maos-2a-{tag}-{nanos}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Tmp(path)
    }

    /// A fake worker binary on a prepended `PATH`. `resolve_cli_binary` searches
    /// exe-sibling → parent → `$PATH`, first hit wins, so a script named `codex`
    /// or `claude` is selected by `select_worker_cli`'s basename dispatch and
    /// admitted by the `--version` liveness probe. It always exits 0 — that is the
    /// whole point: the defect was "exit 0 is treated as completion".
    fn plant_fake_cli(bindir: &std::path::Path, name: &str, body: &str) {
        std::fs::create_dir_all(bindir).unwrap();
        let p = bindir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\nexit 0\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn write_grant(root: &std::path::Path, image: &str, author: &str) -> std::path::PathBuf {
        let grants = root.join(format!("host-grants-{image}.toml"));
        std::fs::write(
            &grants,
            format!(
                "[[grant]]\nattested_image = \"{image}\"\nsigning_key_id = \"{author}\"\n\
                 permitted_tier = \"T3\"\npermitted_egress_destinations = []\n"
            ),
        )
        .unwrap();
        grants
    }

    fn write_worker_manifest(
        root: &std::path::Path,
        command: &str,
        argv_prefix: &str,
        author: &str,
    ) -> std::path::PathBuf {
        let p = root.join(format!("{command}-worker.toml"));
        std::fs::write(
            &p,
            format!(
                "[cli_wrapper]\ncommand = \"{command}\"\nargv_prefix = {argv_prefix}\n\
                 output_shape_version = \"1.0.0\"\nrecovery_policy = \"respawn_fresh\"\n\
                 [cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\n\
                 control_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\
                 [sandbox]\ntier = \"T3\"\n[author]\nname = \"{author}\"\n"
            ),
        )
        .unwrap();
        p
    }

    /// A topology whose single member is the `[cli_wrapper]` worker, with `host`
    /// set so the delegation is FRAME-BORNE. Without `host` the delegated task is
    /// `None`, the completion enforcement is skipped entirely, and the vector
    /// would pass vacuously.
    fn write_topology(root: &std::path::Path, worker: &std::path::Path) -> std::path::PathBuf {
        let p = root.join("topology.toml");
        std::fs::write(
            &p,
            format!(
                "[topology]\nname = \"crosshost-2a-planted-refusal\"\n\n\
                 [[topology.spirits]]\nmanifest = \"{}\"\nhost = \"developer-remote-host\"\n",
                worker.display()
            ),
        )
        .unwrap();
        p
    }

    /// `manifest` is either a topology manifest or the worker manifest itself —
    /// AC1.5 requires the standalone path to enforce the verdict too, so both
    /// entry points are driven through the same isolation.
    fn run_live(
        root: &std::path::Path,
        grants: &std::path::Path,
        manifest: &std::path::Path,
    ) -> std::process::Output {
        let bindir = root.join("bin");
        let mut path = std::ffi::OsString::from(&bindir);
        path.push(":");
        path.push(std::env::var_os("PATH").unwrap_or_default());
        Command::new(env!("CARGO_BIN_EXE_maos"))
            .args(["run", manifest.to_str().unwrap(), "--once"])
            // F14 — `refuse_ambient_auth` reads the REAL `$HOME`. Isolate it or the
            // result depends on whose machine runs the suite.
            .env("HOME", root)
            .env("XDG_DATA_HOME", root)
            .env("MAOS_HOME", root)
            .env("MAOS_HOST_GRANTS", grants)
            .env("PATH", path)
            // The opt-in the CI/local split gate demands. It is safe here because
            // the binary on PATH is a shell script, never a paid agent.
            .env("MAOS_LIVE_AGENT", "1")
            .current_dir(workspace_root())
            .output()
            .expect("failed to execute maos-bin")
    }

    #[test]
    fn codex_turn_completed_with_no_file_change_fails_the_run() {
        // THE VECTOR. A `codex exec --json` stream that reaches `turn.completed`
        // with a fluent closing message and touches NOTHING. Under the previous
        // oracle ("clean exit + non-empty final stdout line") this was
        // `completed: true` and signable.
        let t = tmp("codex-noeffect");
        plant_fake_cli(
            &t.0.join("bin"),
            "codex",
            r#"echo '{"type":"thread.started","thread_id":"t1"}'
echo '{"type":"item.completed","item":{"id":"i1","type":"agent_message","text":"I am not able to write to that path without approval."}}'
echo '{"type":"turn.completed","usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0}}'"#,
        );
        let grants = write_grant(&t.0, "codex", "OpenAI");
        let worker = write_worker_manifest(
            &t.0,
            "codex",
            r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
            "OpenAI",
        );
        let topology = write_topology(&t.0, &worker);
        let out = run_live(&t.0, &grants, &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !out.status.success(),
            "a codex turn that completed without writing anything must FAIL the \
             run — exit 0 is not completion.\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        // Never exit code alone: the refusal must be legible.
        assert!(
            stderr.contains("did not complete") && stderr.contains("no_effect_evidence"),
            "stderr must NAME the completion failure and its typed reason.\nstderr:\n{stderr}"
        );
        assert!(
            stdout.contains(r#""completed":false"#),
            "the worker_completion event must journal the honest verdict.\nstdout:\n{stdout}"
        );
    }

    #[test]
    fn the_standalone_path_no_longer_discards_the_verdict() {
        // AC1.5 — the SECOND false-success surface. `maos run <worker manifest>
        // --once` used to drop the returned `WorkerCompletion` and exit 0 even when
        // the oracle said `completed: false`. That matters because the signed run's
        // runbook sends the operator down THIS path first, to confirm the worker
        // actually writes: a standalone path that exits 0 on a refusal is a
        // pre-flight check that certifies the exact defect the story exists to
        // catch. Note there is no topology and no `host` here — the enforcement
        // must not depend on a delegation being present.
        let t = tmp("standalone");
        plant_fake_cli(
            &t.0.join("bin"),
            "codex",
            "echo 'I am not going to touch that file.'",
        );
        let grants = write_grant(&t.0, "codex", "OpenAI");
        let worker = write_worker_manifest(
            &t.0,
            "codex",
            r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
            "OpenAI",
        );
        let out = run_live(&t.0, &grants, &worker);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "the standalone cli_wrapper path must fail on a non-completion.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("standalone cli_wrapper worker did not complete"),
            "the refusal must name the standalone path, so the operator knows which \
             pre-flight step failed.\nstderr:\n{stderr}"
        );
    }

    #[test]
    fn codex_prose_only_refusal_fails_the_run() {
        // The literal shape of the observed defect: no structured stream at all,
        // just a refusal sentence and exit 0.
        let t = tmp("codex-prose");
        plant_fake_cli(
            &t.0.join("bin"),
            "codex",
            "echo 'I will not modify that file.'",
        );
        let grants = write_grant(&t.0, "codex", "OpenAI");
        let worker = write_worker_manifest(
            &t.0,
            "codex",
            r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
            "OpenAI",
        );
        let topology = write_topology(&t.0, &worker);
        let out = run_live(&t.0, &grants, &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "a prose refusal on exit 0 must FAIL the run.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("did not complete") && stderr.contains("no_completion_marker"),
            "stderr must NAME the completion failure.\nstderr:\n{stderr}"
        );
    }

    #[test]
    fn claude_permission_denial_fails_the_run() {
        // The exact live transcript class that started this story: `subtype:
        // "success"`, `is_error: false`, exit 0, and a non-empty
        // `permission_denials` — because `--print` has no TTY to approve.
        let t = tmp("claude-denied");
        plant_fake_cli(
            &t.0.join("bin"),
            "claude",
            r#"echo '{"type":"result","subtype":"success","is_error":false,"num_turns":1,"result":"I need permission to edit that file.","permission_denials":[{"tool_name":"Write"}],"total_cost_usd":0.01}'"#,
        );
        let grants = write_grant(&t.0, "claude", "Anthropic");
        let worker = write_worker_manifest(
            &t.0,
            "claude",
            r#"["--print", "--output-format", "json", "--bare", "--permission-mode", "acceptEdits"]"#,
            "Anthropic",
        );
        let topology = write_topology(&t.0, &worker);
        let out = run_live(&t.0, &grants, &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "a claude permission denial reported as `success` must FAIL the \
             run.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("did not complete") && stderr.contains("permission_denied"),
            "stderr must NAME the completion failure.\nstderr:\n{stderr}"
        );
    }

    #[test]
    fn a_manifest_missing_the_oracles_argv_flag_refuses_before_the_spawn() {
        // AC1.3's negative, end-to-end: the manifest ships prose while the adapter
        // parses JSON. Refusing is the ONLY correct outcome — running would report
        // a real success as a non-completion, which is F4's inversion.
        let t = tmp("codex-noflag");
        plant_fake_cli(&t.0.join("bin"), "codex", "echo 'unreachable'");
        let grants = write_grant(&t.0, "codex", "OpenAI");
        let worker = write_worker_manifest(
            &t.0,
            "codex",
            r#"["exec", "--sandbox", "workspace-write"]"#,
            "OpenAI",
        );
        let topology = write_topology(&t.0, &worker);
        let out = run_live(&t.0, &grants, &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "a manifest omitting `--json` must refuse.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("--json") && stderr.contains("argv_prefix"),
            "the refusal must name the missing token and where it belongs.\nstderr:\n{stderr}"
        );
    }

    #[test]
    fn a_planted_claude_credentials_file_refuses_the_live_run() {
        // AC2.7 — the paired proven-red for AC2.1. Deleting the `refuse_ambient_auth`
        // block at the composition root must red THIS test. Before it existed,
        // deleting that block reddened exactly one unit test that called the
        // function directly and never exercised the production wiring — a control
        // with no coverage of its own call site.
        //
        // `HOME` is the tempdir, never the operator's. On the development box
        // `~/.claude/.credentials.json` really exists, so a test that read the real
        // `$HOME` would pass here and fail on a clean machine (or the reverse).
        let t = tmp("claude-cred");
        plant_fake_cli(&t.0.join("bin"), "claude", "echo 'unreachable'");
        std::fs::create_dir_all(t.0.join(".claude")).unwrap();
        std::fs::write(
            t.0.join(".claude").join(".credentials.json"),
            b"{\"claudeAiOauth\":{\"accessToken\":\"sk-ant-oat01-REDACTME\"}}",
        )
        .unwrap();
        let grants = write_grant(&t.0, "claude", "Anthropic");
        let worker = write_worker_manifest(
            &t.0,
            "claude",
            r#"["--print", "--output-format", "json", "--bare"]"#,
            "Anthropic",
        );
        let topology = write_topology(&t.0, &worker);
        let out = run_live(&t.0, &grants, &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "an ambient claude credential file must refuse the live run.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains(".credentials.json") && stderr.contains("ambient auth file"),
            "the refusal must name the PLANTED PATH so the operator can act on \
             it.\nstderr:\n{stderr}"
        );
        // The refusal happens BEFORE the spawn, so the worker never ran.
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("cli_wrapper_loaded"),
            "no child may be spawned after a credential refusal.\nstdout:\n{stdout}"
        );
    }

    #[test]
    fn an_unset_home_refuses_the_live_run_instead_of_skipping_the_check() {
        // AC2.4 — the fail-OPEN this closes: an unset `HOME` used to skip the
        // clean-home check entirely, so "we could not look" was treated as "there is
        // nothing there". An unverifiable credential control is not a satisfied one.
        let t = tmp("no-home");
        plant_fake_cli(&t.0.join("bin"), "codex", "echo 'unreachable'");
        let grants = write_grant(&t.0, "codex", "OpenAI");
        let worker = write_worker_manifest(
            &t.0,
            "codex",
            r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
            "OpenAI",
        );
        let topology = write_topology(&t.0, &worker);
        let bindir = t.0.join("bin");
        let mut path = std::ffi::OsString::from(&bindir);
        path.push(":");
        path.push(std::env::var_os("PATH").unwrap_or_default());
        let out = Command::new(env!("CARGO_BIN_EXE_maos"))
            .args(["run", topology.to_str().unwrap(), "--once"])
            .env_remove("HOME")
            .env("XDG_DATA_HOME", &t.0)
            .env("MAOS_HOME", &t.0)
            .env("MAOS_HOST_GRANTS", &grants)
            .env("PATH", path)
            .env("MAOS_LIVE_AGENT", "1")
            .current_dir(workspace_root())
            .output()
            .expect("failed to execute maos-bin");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "an unset HOME on the live path must refuse.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("HOME is unset"),
            "the refusal must say WHY it could not verify the invariant.\nstderr:\n{stderr}"
        );
    }

    #[test]
    fn the_hermetic_fixture_path_is_untouched_by_the_credential_controls() {
        // AC1.10 / the immunity that keeps CI green. `MAOS_LIVE_AGENT` is never set
        // in CI, so the whole clean-home block is skipped — but the fixture must ALSO
        // survive with the flag set and a credential file planted for BOTH real
        // providers, because `FixtureCli::ambient_auth_path` is `None` and its marker
        // oracle is unchanged. If a future change makes a credential control reach
        // the fixture, `journey_j1` and `drain_once_audit_writer` go red transitively;
        // this fails first and names why.
        let t = tmp("fixture-immune");
        std::fs::create_dir_all(t.0.join(".claude")).unwrap();
        std::fs::write(t.0.join(".claude").join(".credentials.json"), b"{}").unwrap();
        std::fs::create_dir_all(t.0.join(".codex")).unwrap();
        std::fs::write(t.0.join(".codex").join("auth.json"), b"{}").unwrap();
        let mut path = std::ffi::OsString::from(
            std::path::PathBuf::from(workspace_root()).join("target/debug"),
        );
        path.push(":");
        path.push(std::env::var_os("PATH").unwrap_or_default());
        let out = Command::new(env!("CARGO_BIN_EXE_maos"))
            .args(["run", "spirits/worker/manifest.toml", "--once"])
            .env("HOME", &t.0)
            .env("XDG_DATA_HOME", &t.0)
            .env("MAOS_HOME", &t.0)
            .env("PATH", path)
            .env("MAOS_LIVE_AGENT", "1")
            .current_dir(workspace_root())
            .output()
            .expect("failed to execute maos-bin");
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "the hermetic fixture must stay green with both providers' credential \
             files planted and MAOS_LIVE_AGENT set.\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            stdout.contains(r#""completed":true"#),
            "the fixture's marker oracle is unchanged.\nstdout:\n{stdout}"
        );
    }

    #[test]
    fn a_hostless_topology_refusal_also_fails_the_run() {
        // Review 2a-P4 — the third false-success surface. AC1.5 closed the
        // standalone path; a topology entry WITHOUT `host` gets
        // `delegated_task = None` and its verdict used to be discarded one
        // branch over, with the same pre-flight-certifies-the-defect shape the
        // runbook's "pin the invocation standalone first" step creates.
        let t = tmp("hostless");
        plant_fake_cli(
            &t.0.join("bin"),
            "codex",
            "echo 'I will not modify that file.'",
        );
        let grants = write_grant(&t.0, "codex", "OpenAI");
        let worker = write_worker_manifest(
            &t.0,
            "codex",
            r#"["exec", "--sandbox", "workspace-write", "--json"]"#,
            "OpenAI",
        );
        // Same topology writer, MINUS the `host` key: the entry is admitted as a
        // local member, `frame_borne: false`, no delegation — and the refusal
        // must still fail the run.
        let topology = t.0.join("topology-hostless.toml");
        std::fs::write(
            &topology,
            format!(
                "[topology]\nname = \"crosshost-2a-hostless-refusal\"\n\n\
                 [[topology.spirits]]\nmanifest = \"{}\"\n",
                worker.display()
            ),
        )
        .unwrap();
        let out = run_live(&t.0, &grants, &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !out.status.success(),
            "a hostless topology entry must not exit 0 over a refusal — the \
             absent task and the dropped verdict are different things.\nstdout:\n{stdout}\n\
             stderr:\n{stderr}"
        );
        assert!(
            stderr.contains("did not complete") && stderr.contains("no_completion_marker"),
            "stderr must NAME the completion failure on the hostless path too.\nstderr:\n{stderr}"
        );
        assert!(
            stdout.contains("\"frame_borne\":false"),
            "the vector must actually exercise the hostless branch.\nstdout:\n{stdout}"
        );
    }
}
