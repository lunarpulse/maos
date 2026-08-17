#![forbid(unsafe_code)]

//! Proven-red vectors for `check-j1-loopback-delegation`'s **j1-crosshost-2a legs**
//! (AC1.7 · AC1.9).
//!
//! Three legs were added to a gate that already blocks: the per-adapter completion
//! oracle, the library boundary that makes the oracle's tests executable at all, and
//! the CI-enrollment leg. Each of them must be FALSIFIABLE, or the story registered
//! three green boxes and called it coverage — the exact failure shape `1a` was
//! corrected for and the reason `2a` was split out of a single 15-AC story.
//!
//! Mirrors `j1_crosshost_1a_proven_red.rs` exactly: every vector runs the REAL
//! `xtask` binary against a self-contained fixture tree in a tempdir, mutates one
//! invariant, and asserts the verdict flips. Fixtures carry only the lines that
//! carry the invariants — a fixture that copied the real files would drift; one that
//! carries the invariants cannot.
//!
//! ## The enrollment vector is the reason this file exists
//!
//! AC1.9 requires that deleting `--test worker_completion_2a` from the workflow REDS
//! the gate. At HEAD that vector would have passed VACUOUSLY: the gate governed six
//! source files and `.github/workflows/discipline.yml` was not among them, so the
//! gate had no eyes on the file its own enrollment lives in. Deleting the line left
//! it green. AC1.7(iii) added the workflow to the governed list precisely so this
//! vector can fire, and the cost — `lay_green` must lay a workflow into the fixture
//! tree — is paid below. That is the cost of the vector being real.
//!
//! Every leg here is source-STATIC and root-relative. A `cargo`-invoking leg would
//! inherit this tempdir (which has no `Cargo.toml`), vacuum every planted vector, and
//! report green — `1a`'s F3, still true. Reading a workflow FILE is static and safe;
//! invoking `cargo` is not.

use std::io::Write;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════
// Fixture tree — a GREEN baseline, one invariant per file
// ═══════════════════════════════════════════════════════════════════

const TOPOLOGY: &str = "spirits/topologies/j1-founder-loop.toml";
const DELEGATION_RS: &str = "crates/maos-bin/src/delegation.rs";
const MAILBOX_RS: &str = "crates/maos-iac/src/adapter/mailbox.rs";
const MAIN_RS: &str = "crates/maos-bin/src/main.rs";
const ORCHESTRATOR_RS: &str = "spirits/orchestrator/src/lib.rs";
const A2A_ROUTER_RS: &str = "crates/maos-a2a-core/src/router.rs";
const WORKER_CLI_RS: &str = "crates/maos-bin/src/worker_cli.rs";
const BIN_LIB_RS: &str = "crates/maos-bin/src/lib.rs";
const WORKFLOW: &str = ".github/workflows/discipline.yml";
/// j1-crosshost-1b: `lay_green` exists TWICE (here and in
/// `j1_crosshost_1a_proven_red.rs`) and a governed file this tree does not lay reds
/// the gate on the TREE, at which point every vector below passes for the wrong
/// reason while CI reports green. `1b` added `consent_refusal_1b.rs` as the tenth
/// governed file AND turned the enrolled `cargo test` set into a derivation over
/// `crates/maos-bin/tests/` — so the two targets THIS file's enrollment vectors
/// un-enroll must exist as files, or those vectors go vacuous.
const CONSENT_REFUSAL_RS: &str = "crates/maos-bin/tests/consent_refusal_1b.rs";
const WORKER_COMPLETION_TEST: &str = "crates/maos-bin/tests/worker_completion_2a.rs";
const WORKER_MANIFESTS_TEST: &str = "crates/maos-bin/tests/worker_manifests_2a.rs";
const TWO_HOST_PROOF_TEST: &str = "crates/maos-bin/tests/two_host_delegation_2b.rs";
const HOST_GRANTS_TEST: &str = "crates/maos-bin/tests/host_grants_2b.rs";
const BOUNDED_POSTURES_TEST: &str = "crates/maos-bin/tests/bounded_postures_2b.rs";

const GOOD_TOPOLOGY: &str = "[topology]\nname = \"j1-founder-loop\"\n\n\
     [[topology.spirits]]\nmanifest = \"../orchestrator/manifest.toml\"\n\n\
     [[topology.spirits]]\nmanifest = \"../worker/manifest.toml\"\n\
     host = \"developer-remote-host\"\n";

/// j1-crosshost-1b AC2.2a — the repaired boundary leg reads the COMPOSITION ROOT,
/// so this fixture must carry the loopback pairing call it keys on.
const GOOD_DELEGATION: &str = "pub async fn install(mailbox: Arc<Mailbox>, intent: &A2AIntent) {\n\
     \x20   let (router, rx) = maos_a2a::pairing::paired_loopback_router(&[]).await?;\n}\n\
     pub async fn delegate(&mut self) {\n    for addr in routed.to.iter_mut() {\n\
     \x20       addr.host_id = None;\n    }\n}\n";

const GOOD_MAILBOX: &str = "fn phase_three() {\n\
     \x20   let router = self.a2a_router.get().ok_or_else(|| {\n\
     \x20       IacBusError::CrossHostNotConfigured { host_id: hosts }\n    })?;\n}\n";

const GOOD_MAIN: &str = "#[cfg(feature = \"network\")]\nuse maos_bin::worker_cli;\n\
     fn topology_loop() {\n\
     \x20   let frame = delegation_emitter.assign_frame_remote(seq, recipient, role);\n\
     \x20   let goal = delegation_leg.delegate(&iac, frame).await?;\n}\n";

const GOOD_ORCHESTRATOR: &str =
    "pub const DELEGATION_CONSENT_INTENT: &str = \"development-task:write-workspace\";\n";

const GOOD_A2A_ROUTER: &str = "fn intake() {\n\
     \x20   let peer_host = match &frame.from.host_id { Some(h) => h.clone(), None => loopback() };\n}\n\
     pub async fn handle_intake_verified(&self, r: Req, p: &PeerId) {}\n";

/// The four production facts the completion-oracle leg reads: the two per-adapter
/// oracles that replaced the shared final-stdout-line oracle, the `required_argv_flags`
/// seam that lets an adapter demand the flag its oracle parses, and claude's ambient
/// credential path (whose absence made `refuse_ambient_auth` a no-op for claude).
const GOOD_WORKER_CLI: &str = "fn codex_jsonl_oracle(stdout: &[String], exit: WorkerExit) {}\n\
     fn claude_result_object_oracle(stdout: &[String], exit: WorkerExit) {}\n\
     fn parse_completion(&self, stdout: &[String], stderr: &[String], exit: WorkerExit) -> WorkerCompletion {\n\
     \x20   codex_jsonl_oracle(stdout, exit)\n}\n\
     fn parse_completion_claude(&self, stdout: &[String], stderr: &[String], exit: WorkerExit) -> WorkerCompletion {\n\
     \x20   claude_result_object_oracle(stdout, exit)\n}\n\
     fn required_argv_flags(&self) -> &'static [&'static [&'static str]] { &[] }\n\
     fn ambient_auth_path(&self, home: &Path) -> Option<PathBuf> {\n\
     \x20   Some(home.join(\".claude\").join(\".credentials.json\"))\n}\n";

const GOOD_BIN_LIB: &str = "#[cfg(feature = \"network\")]\npub mod worker_cli;\n";

/// The enrollment surface. Both `--test` targets must be here or the vectors that
/// prove the oracle never execute in CI.
const GOOD_WORKFLOW: &str = "jobs:\n  check-j1-loopback-delegation:\n    steps:\n\
     \x20     - run: cargo test -p maos-bin --test delegation_leg_1a --test topology_delegation_1a -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test worker_manifests_2a -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test consent_refusal_1b -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test two_host_delegation_2b -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test host_grants_2b -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test bounded_postures_2b -- --test-threads=1\n";

/// The refusal-proof assertion skeletons `1b`'s `consent-refusal-proofs` leg
/// needles. Structural, so this carries the SHAPES, not a copy of the real file.
const GOOD_CONSENT_REFUSAL: &str =
    "#[tokio::test]\nasync fn allowlisted_delegation_intent_is_admitted() {\n\
     \x20   assert_eq!(delivered.intent_class, Some(DELEGATION_CONSENT_INTENT.to_string()));\n}\n\
     #[tokio::test]\nasync fn pin_both_peer_ids() {\n\
     \x20   assert_eq!(TO_HOST, \"developer-remote-host\");\n\
     \x20   assert_eq!(FROM_HOST, \"founder-loop-host\");\n}\n\
     #[tokio::test]\nasync fn minus_32001_at_peer() {\n\
     \x20   match error {\n\
     \x20       A2AError::IntentDeniedAtPeer { peer, message } => {\n\
     \x20           assert_eq!(peer, TO_HOST, \"destination\");\n\
     \x20           assert!(message.contains(FROM_HOST));\n\
     \x20       }\n    }\n}\n\
     #[tokio::test]\nasync fn minus_32009_send_seam() {\n\
     \x20   match error {\n\
     \x20       A2AError::ConsentUnclassified { direction: IntentDirection::Send, reason } => {\n\
     \x20           assert_eq!(reason, expected);\n\
     \x20       }\n    }\n}\n\
     #[tokio::test]\nasync fn minus_32009_accept_seam() {\n\
     \x20   assert_eq!(nack.error.code, CODE_CONSENT_UNCLASSIFIED, \"accept seam\");\n\
     \x20   let reason = serde_json::from_value::<UnclassifiedReason>(v.clone()).unwrap();\n\
     \x20   assert_eq!(nack_reason(&nack.error), expected, \"accept seam reason\");\n\
     \x20   let reachable = [UnclassifiedReason::Absent, UnclassifiedReason::NonCanonical, \
     UnclassifiedReason::Oversized];\n}\n\
     #[tokio::test]\nasync fn non_conflation_both_ways() {\n\
     \x20   assert_ne!(nack.error.code, CODE_CONSENT_UNCLASSIFIED, \"-32001 direction\");\n\
     \x20   assert_ne!(nack.error.code, CODE_INTENT_DENIED, \"-32009 direction\");\n}\n\
     #[tokio::test]\nasync fn the_third_code() {\n\
     \x20   assert_eq!(nack.error.code, CODE_CONSENT_EXPIRED, \"expiry stays distinct\");\n}\n";

/// The derived enrollment set keys on FILENAMES, so content is irrelevant — what
/// matters is that the targets this file un-enrolls actually exist.
const GOOD_TEST_STUB: &str = "#[test]\nfn placeholder() {}\n";

const GOOD_TWO_HOST_PROOF: &str = "#[test]\nfn crossing() { let _ = env!(\"CARGO_BIN_EXE_maos\"); mint_pems(); let worker_manifest = (); assert!(matches!(first, HostBOutcome::Ran { .. })); assert_eq!(host_a_frame_id, host_b_frame_id); let _ = MAOS_AUDIT_DB; }\n#[test]\nfn sink_uninstalled() {}\n#[test]\nfn duplicate() { assert!(matches!(second, HostBOutcome::Duplicate { .. })); }\nimpl Drop for RunningDaemon {}\n";

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// Lay down a fixture tree that the gate must find GREEN.
fn lay_green(root: &Path) {
    write_file(root, TOPOLOGY, GOOD_TOPOLOGY);
    write_file(root, DELEGATION_RS, GOOD_DELEGATION);
    write_file(root, MAILBOX_RS, GOOD_MAILBOX);
    write_file(root, MAIN_RS, GOOD_MAIN);
    write_file(root, ORCHESTRATOR_RS, GOOD_ORCHESTRATOR);
    write_file(root, A2A_ROUTER_RS, GOOD_A2A_ROUTER);
    write_file(root, WORKER_CLI_RS, GOOD_WORKER_CLI);
    write_file(root, BIN_LIB_RS, GOOD_BIN_LIB);
    // AC1.9's cost, paid deliberately: without the workflow in the fixture tree the
    // enrollment leg has nothing to read and its vector cannot fire.
    write_file(root, WORKFLOW, GOOD_WORKFLOW);
    // j1-crosshost-1b: the tenth governed file, plus the two derived enrollment
    // targets THIS file's vectors un-enroll. Miss any and `baseline_fixture_tree_is_green`
    // reds, at which point every vector below passes for the wrong reason.
    write_file(root, CONSENT_REFUSAL_RS, GOOD_CONSENT_REFUSAL);
    write_file(root, WORKER_COMPLETION_TEST, GOOD_TEST_STUB);
    write_file(root, WORKER_MANIFESTS_TEST, GOOD_TEST_STUB);
    write_file(root, TWO_HOST_PROOF_TEST, GOOD_TWO_HOST_PROOF);
    write_file(root, HOST_GRANTS_TEST, GOOD_TEST_STUB);
    write_file(root, BOUNDED_POSTURES_TEST, GOOD_TEST_STUB);
    // §A6 review P8 — the derivation walks suffixes `_1a|_1b|_2a`; a tree that
    // lays no `_1a` target cannot notice `"_1a.rs"` being deleted from
    // `J1_TEST_SUFFIXES`, and the real gate would silently stop enforcing the
    // 1a enrollment behind every green vector.
    write_file(
        root,
        "crates/maos-bin/tests/delegation_leg_1a.rs",
        GOOD_TEST_STUB,
    );
    write_file(
        root,
        "crates/maos-bin/tests/topology_delegation_1a.rs",
        GOOD_TEST_STUB,
    );
}

struct Verdict {
    passed: bool,
    stdout: String,
    success: bool,
}

/// Run the real gate against a fixture tree built by `plant`.
fn run_gate(plant: impl FnOnce(&Path)) -> Verdict {
    let dir = tempfile::tempdir().unwrap();
    lay_green(dir.path());
    plant(dir.path());
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-j1-loopback-delegation", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("xtask must run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let passed = stdout.contains("\"passed\":true") || stdout.contains("\"passed\": true");
    Verdict {
        passed,
        stdout,
        success: out.status.success(),
    }
}

fn assert_red(v: &Verdict, expected_detail: &str, vector: &str) {
    assert!(
        !v.passed,
        "PROVEN-RED FAILURE — the gate stayed GREEN with `{vector}` planted. \
         A gate that cannot see this regression is an empty box.\nstdout:\n{}",
        v.stdout
    );
    assert!(
        !v.success,
        "a Blocking gate with a RED oracle must exit non-zero (`{vector}`)\nstdout:\n{}",
        v.stdout
    );
    assert!(
        v.stdout.contains(expected_detail),
        "the finding must NAME the regression (expected substring `{expected_detail}` for \
         vector `{vector}`)\nstdout:\n{}",
        v.stdout
    );
}

// ═══════════════════════════════════════════════════════════════════
// Baseline — the fixture tree must be GREEN, or every red below is vacuous
// ═══════════════════════════════════════════════════════════════════

#[test]
fn baseline_fixture_tree_is_green() {
    let v = run_gate(|_| {});
    assert!(
        v.passed && v.success,
        "the unmutated fixture tree must PASS, otherwise every proven-red vector \
         below passes for the wrong reason\nstdout:\n{}",
        v.stdout
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC1.7(i) — the completion oracle must stay per-adapter and structured
// ═══════════════════════════════════════════════════════════════════

/// THE regression: bring back the shared "clean exit + non-empty final stdout line"
/// oracle. That is the defect verbatim — a live `claude -p` refused a write, printed
/// a fluent explanation, exited 0, and was scored `completed: true`, which is the
/// admission condition for signing.
#[test]
fn the_shared_final_stdout_line_oracle_may_not_come_back() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKER_CLI_RS,
            &format!("{GOOD_WORKER_CLI}fn final_stdout_message_oracle(stdout: &[String]) {{}}\n"),
        );
    });
    assert_red(
        &v,
        "final_stdout_message_oracle",
        "shared final-stdout-line oracle reintroduced",
    );
}

/// A mention inside a `#[cfg(test)]` module must NOT red the gate: the vectors
/// legitimately name the retired oracle when documenting what they falsify, and a
/// file-wide keyword search would turn every such comment into a false alarm.
#[test]
fn a_test_module_mention_of_the_retired_oracle_is_not_a_regression() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKER_CLI_RS,
            &format!(
                "{GOOD_WORKER_CLI}\n#[cfg(test)]\nmod tests {{\n\
                 \x20   fn documents_what_it_replaced() {{ let _ = \"final_stdout_message_oracle\"; }}\n}}\n"
            ),
        );
    });
    // The `#[cfg(test)]` module itself is a separate violation (budget-charged and
    // CI-invisible), so the gate must red for THAT reason and NOT for the oracle.
    assert!(
        !v.passed,
        "an in-`src` test module is itself a finding\nstdout:\n{}",
        v.stdout
    );
    assert!(
        v.stdout.contains("in-`src` `#[cfg(test)]` module"),
        "the finding must be about the in-`src` test module\nstdout:\n{}",
        v.stdout
    );
    assert!(
        !v.stdout
            .contains("still carries `final_stdout_message_oracle` in production"),
        "a `#[cfg(test)]` mention must not be read as a production delegation — the \
         oracle leg reads the production half only\nstdout:\n{}",
        v.stdout
    );
}

/// Delete one adapter's structured oracle. Sharing an oracle between two CLIs whose
/// machine-readable contracts are NOT equivalent is how the asymmetry gets papered
/// over: codex proves effect natively, claude proves only that no tool permission was denied.
#[test]
fn each_real_adapter_must_keep_its_own_structured_oracle() {
    for (oracle, vector) in [
        ("codex_jsonl_oracle", "codex JSONL oracle deleted"),
        (
            "claude_result_object_oracle",
            "claude result-object oracle deleted",
        ),
    ] {
        let v = run_gate(|root| {
            write_file(
                root,
                WORKER_CLI_RS,
                &GOOD_WORKER_CLI.replace(oracle, "gone"),
            );
        });
        // Review 2a-P8 — the leg needles the CALL form now, so the expected
        // finding names it too.
        assert_red(
            &v,
            &format!("does not CALL `{oracle}(stdout,exit)`"),
            vector,
        );
    }
}

/// Review 2a-P8 — THE wiring falsifier. Keeping the oracle FUNCTIONS while
/// unwiring them from `parse_completion` is the exact shape the name-presence
/// check was blind to (the old green fixture passed with EMPTY stubs): the
/// names exist, the verdict comes from somewhere else.
#[test]
fn an_unwired_oracle_keeps_the_name_but_not_the_call_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKER_CLI_RS,
            &GOOD_WORKER_CLI.replace(
                "codex_jsonl_oracle(stdout, exit)",
                "final_line_oracle(stdout, exit)",
            ),
        );
    });
    assert_red(
        &v,
        "does not CALL `codex_jsonl_oracle(stdout,exit)`",
        "codex oracle named but unwired from parse_completion",
    );
}

/// Remove the `required_argv_flags` seam. Without it an adapter cannot DEMAND the
/// structured-output flag its oracle parses, so a manifest shipping prose converts a
/// REAL success into a non-completion — the inverse of the defect being fixed, and
/// just as wrong.
#[test]
fn an_adapter_must_be_able_to_demand_the_flag_its_oracle_parses() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKER_CLI_RS,
            &GOOD_WORKER_CLI.replace("fn required_argv_flags", "fn unrelated_helper"),
        );
    });
    assert_red(
        &v,
        "required_argv_flags",
        "argv-flag requirement seam removed",
    );
}

/// Revert claude's ambient credential path to `None`. That was a green in-repo claim
/// — `assert_eq!(ClaudeCli.ambient_auth_path(home), None)` under a comment reading
/// "only codex names the footgun" — and it made `refuse_ambient_auth` a NO-OP for
/// claude, so a signed claude run would have stamped an unattestable redaction claim.
#[test]
fn claude_may_not_go_back_to_having_no_ambient_credential_path() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKER_CLI_RS,
            &GOOD_WORKER_CLI.replace(
                ".claude\").join(\".credentials.json",
                ".nothing\").join(\"x",
            ),
        );
    });
    assert_red(
        &v,
        "refuse_ambient_auth` is then a NO-OP for claude",
        "claude ambient credential path removed",
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC1.7(ii) — the library boundary that makes the vectors exist at all
// ═══════════════════════════════════════════════════════════════════

/// Drop `pub mod worker_cli` from the library. This is the subtlest regression in the
/// story: the vectors do not FAIL, they cease to exist. Nothing under
/// `crates/maos-bin/tests/` can name `ClaudeCli`, `CodexCli` or `parse_completion`,
/// so the whole oracle suite silently stops being compiled.
#[test]
fn re_orphaning_the_adapter_seam_from_the_tests_must_red() {
    let v = run_gate(|root| {
        write_file(root, BIN_LIB_RS, "pub mod topology;\n");
    });
    assert_red(
        &v,
        "does not export `pub mod worker_cli`",
        "worker_cli removed from the library",
    );
}

/// Re-declare `mod worker_cli;` in `main.rs` without consuming the library module.
/// That compiles a SECOND, test-invisible copy of the adapter seam: the binary's
/// behaviour changes while the vectors keep asserting against the library copy.
#[test]
fn a_second_test_invisible_copy_of_the_adapter_seam_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            MAIN_RS,
            &GOOD_MAIN.replace("use maos_bin::worker_cli;", "mod worker_cli;"),
        );
    });
    assert_red(
        &v,
        "re-declares `mod worker_cli;`",
        "main.rs re-declares the module",
    );
}

/// Move the tests back into `worker_cli.rs`. That module is charged to `maos-bin`'s
/// KLOC ceiling AND executed by no CI job — every invocation is `--test <name>` — so
/// it is budget-charged code with no execution path, twice over.
#[test]
fn returning_the_tests_to_an_in_src_module_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKER_CLI_RS,
            &format!("{GOOD_WORKER_CLI}\n#[cfg(test)]\nmod tests {{}}\n"),
        );
    });
    assert_red(
        &v,
        "in-`src` `#[cfg(test)]` module",
        "tests moved back into src",
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC1.9 — the ENROLLMENT vector (1b's vector-#12 shape)
//
// This is the vector that could not fire at HEAD. The gate governed six source
// files and the workflow was not among them, so deleting a `--test` line left it
// green: a falsifier standing in for a falsifier.
// ═══════════════════════════════════════════════════════════════════

/// Delete the `--test worker_completion_2a` enrollment line. The gate MUST red: an
/// un-run test target is a suggestion, and 24 of 28 `crates/maos-bin/tests/` targets
/// are invoked by no CI job at all, so "it exists" is not evidence that it runs.
#[test]
fn deleting_the_completion_vector_enrollment_line_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            &GOOD_WORKFLOW.replace("--test worker_completion_2a", "--test something_else"),
        );
    });
    assert_red(
        &v,
        "--test worker_completion_2a",
        "completion-vector CI enrollment deleted",
    );
}

/// The same for the worker-manifest reader. Without it the shipped
/// `manifest-codex.toml` / `manifest-claude.toml` have no validator except an
/// operator-local paid run, which is the definition of decoration.
#[test]
fn deleting_the_manifest_reader_enrollment_line_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            &GOOD_WORKFLOW.replace("--test worker_manifests_2a", "--test something_else"),
        );
    });
    assert_red(
        &v,
        "--test worker_manifests_2a",
        "manifest-reader CI enrollment deleted",
    );
}

/// Deleting the WORKFLOW entirely must red too, not skip. A gate that silently
/// passes when its subject is absent is the null control this lane keeps catching.
#[test]
fn a_missing_workflow_is_a_finding_never_a_skip() {
    let v = run_gate(|root| {
        std::fs::remove_file(root.join(WORKFLOW)).unwrap();
    });
    assert_red(
        &v,
        "cannot read .github/workflows/discipline.yml",
        "workflow file deleted",
    );
}

/// Whitespace and line-continuation changes must NOT flip the enrollment leg. A
/// `Blocking` gate a formatter can red is a false-alarm machine — the exact defect
/// found in this gate's own `mailbox.rs` needle at `6827dc87`, when `cargo fmt` split
/// a chain across lines and a still-present fail-closed branch stopped matching.

/// Review 2a-P8 — enrollment must live in THIS gate's own Blocking job. The same
/// `--test` lines under an unrelated job kept the old workflow-wide scan green,
/// which is enrollment-in-name-only: only the `check-j1-loopback-delegation` job
/// is BindingClass::Blocking, in gate-registry.toml, and a `needs` of the
/// aggregate.
#[test]
fn enrollment_lines_moved_to_another_job_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            "jobs:\n  some-other-job:\n    steps:\n\
             \x20     - run: cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1\n\
             \x20     - run: cargo test -p maos-bin --test worker_manifests_2a -- --test-threads=1\n\
             \x20 check-j1-loopback-delegation:\n    steps:\n\
             \x20     - run: cargo run -p xtask -- check-j1-loopback-delegation --json\n",
        );
    });
    assert_red(
        &v,
        "does not invoke `cargo test -p maos-bin --test worker_completion_2a`",
        "enrollment moved out of the Blocking job",
    );
}

#[test]
fn reformatting_the_enrollment_step_does_not_flip_the_leg() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            "jobs:\n  check-j1-loopback-delegation:\n    steps:\n      - run: |\n\
             \x20         cargo test -p maos-bin \\\n            --test worker_completion_2a \\\n\
             \x20         -- --test-threads=1\n          cargo test -p maos-bin \\\n\
             \x20         --test worker_manifests_2a -- --test-threads=1\n\
             \x20         cargo test -p maos-bin \\\n            --test consent_refusal_1b \\\n\
     \x20         -- --test-threads=1\n          cargo test -p maos-bin --test delegation_leg_1a --test topology_delegation_1a -- --test-threads=1\n\
     \x20         cargo test -p maos-bin --test two_host_delegation_2b -- --test-threads=1\n\
     \x20         cargo test -p maos-bin --test host_grants_2b -- --test-threads=1\n\
     \x20         cargo test -p maos-bin --test bounded_postures_2b -- --test-threads=1\n",
        );
    });
    assert!(
        v.passed && v.success,
        "the enrollment leg must depend on STRUCTURE, not on layout\nstdout:\n{}",
        v.stdout
    );
}
