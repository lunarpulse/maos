#![forbid(unsafe_code)]

//! Proven-red vectors for the `j1-crosshost-2b` additions to
//! `check-j1-loopback-delegation`.
//!
//! Every vector runs the actual xtask binary against a fixture tree rooted at its
//! tempdir. The gate reads `root.join(rel)`, so the fixtures are complete enough for
//! every governed leg to pass before one fact is removed.

use std::io::Write;
use std::path::Path;

const TOPOLOGY: &str = "spirits/topologies/j1-founder-loop.toml";
const DELEGATION_RS: &str = "crates/maos-bin/src/delegation.rs";
const MAILBOX_RS: &str = "crates/maos-iac/src/adapter/mailbox.rs";
const MAIN_RS: &str = "crates/maos-bin/src/main.rs";
const ORCHESTRATOR_RS: &str = "spirits/orchestrator/src/lib.rs";
const A2A_ROUTER_RS: &str = "crates/maos-a2a-core/src/router.rs";
const WORKER_CLI_RS: &str = "crates/maos-bin/src/worker_cli.rs";
const BIN_LIB_RS: &str = "crates/maos-bin/src/lib.rs";
const WORKFLOW: &str = ".github/workflows/discipline.yml";
const CONSENT_REFUSAL_RS: &str = "crates/maos-bin/tests/consent_refusal_1b.rs";
const TWO_HOST_PROOF_RS: &str = "crates/maos-bin/tests/two_host_delegation_2b.rs";
const HOST_GRANTS_RS: &str = "crates/maos-bin/tests/host_grants_2b.rs";
const BOUNDED_POSTURES_RS: &str = "crates/maos-bin/tests/bounded_postures_2b.rs";
const WORKER_COMPLETION_TEST: &str = "crates/maos-bin/tests/worker_completion_2a.rs";
const WORKER_MANIFESTS_TEST: &str = "crates/maos-bin/tests/worker_manifests_2a.rs";
const DELEGATION_LEG_TEST: &str = "crates/maos-bin/tests/delegation_leg_1a.rs";
const TOPOLOGY_DELEGATION_TEST: &str = "crates/maos-bin/tests/topology_delegation_1a.rs";

const GOOD_TOPOLOGY: &str = "[topology]\nname = \"j1-founder-loop\"\n\n\
    [[topology.spirits]]\nmanifest = \"../orchestrator/manifest.toml\"\n\n\
    [[topology.spirits]]\nmanifest = \"../worker/manifest.toml\"\nhost = \"developer-remote-host\"\n";
const GOOD_DELEGATION: &str = "pub async fn install(mailbox: Arc<Mailbox>, intent: &A2AIntent) {\n\
    let (router, rx) = maos_a2a::pairing::paired_loopback_router(&[]).await?;\n}\n\
    pub async fn delegate(&mut self) {\n    for addr in routed.to.iter_mut() {\n        addr.host_id = None;\n    }\n}\n";
const FORK_DELEGATION: &str = "pub async fn install(mailbox: Arc<Mailbox>, intent: &A2AIntent) {\n\
    let (router, rx) = maos_a2a::pairing::paired_loopback_router(&[]).await?;\n}\n\
    fn router_for(kind: DelegationRouter) {\n        let _ = maos_a2a_tcp::TcpA2ATransport::connect(&peer);\n        let _ = DelegationRouter::CrossHostVerified(router);\n    }\n\
    pub async fn delegate(&mut self) {\n    for addr in routed.to.iter_mut() {\n        addr.host_id = None;\n    }\n}\n";
const GOOD_MAILBOX: &str = "fn phase_three() {\n    let router = self.a2a_router.get().ok_or_else(|| {\n        IacBusError::CrossHostNotConfigured { host_id: hosts }\n    })?;\n}\n";
const GOOD_MAIN: &str = "#[cfg(feature = \"network\")]\nuse maos_bin::worker_cli;\nfn topology_loop() {\n    let frame = delegation_emitter.assign_frame_remote(seq, recipient, role);\n    let goal = delegation_leg.delegate(&iac, frame).await?;\n}\n";
const GOOD_ORCHESTRATOR: &str =
    "pub const DELEGATION_CONSENT_INTENT: &str = \"development-task:write-workspace\";\n";
const GOOD_A2A_ROUTER: &str = "fn intake() {\n    let peer_host = match &frame.from.host_id { Some(h) => h.clone(), None => loopback() };\n}\npub async fn handle_intake_verified(&self, r: Req, p: &PeerId) {}\n";
const GOOD_WORKER_CLI: &str = "fn codex_jsonl_oracle(stdout: &[String], exit: WorkerExit) {}\nfn claude_result_object_oracle(stdout: &[String], exit: WorkerExit) {}\nfn parse_completion(&self, stdout: &[String], stderr: &[String], exit: WorkerExit) -> WorkerCompletion { codex_jsonl_oracle(stdout, exit) }\nfn parse_completion_claude(&self, stdout: &[String], stderr: &[String], exit: WorkerExit) -> WorkerCompletion { claude_result_object_oracle(stdout, exit) }\nfn required_argv_flags(&self) -> &'static [&'static [&'static str]] { &[] }\nfn ambient_auth_path(&self, home: &Path) -> Option<PathBuf> { Some(home.join(\".claude\").join(\".credentials.json\")) }\n";
const GOOD_BIN_LIB: &str = "#[cfg(feature = \"network\")]\npub mod worker_cli;\n";
const GOOD_WORKFLOW: &str = "jobs:\n  check-j1-loopback-delegation:\n    steps:\n      - run: cargo test -p maos-bin --test delegation_leg_1a --test topology_delegation_1a -- --test-threads=1\n      - run: cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1\n      - run: cargo test -p maos-bin --test worker_manifests_2a -- --test-threads=1\n      - run: cargo test -p maos-bin --test consent_refusal_1b -- --test-threads=1\n      - run: cargo test -p maos-bin --test two_host_delegation_2b -- --test-threads=1\n      - run: cargo test -p maos-bin --test host_grants_2b -- --test-threads=1\n      - run: cargo test -p maos-bin --test bounded_postures_2b -- --test-threads=1\n";
const GOOD_CONSENT_REFUSAL: &str = "#[tokio::test]\nasync fn allowlisted_delegation_intent_is_admitted() { assert_eq!(delivered.intent_class, Some(DELEGATION_CONSENT_INTENT.to_string())); }\n#[tokio::test]\nasync fn pin_both_peer_ids() { assert_eq!(TO_HOST, \"developer-remote-host\"); assert_eq!(FROM_HOST, \"founder-loop-host\"); }\n#[tokio::test]\nasync fn minus_32001_at_peer() { match error { A2AError::IntentDeniedAtPeer { peer, message } => { assert_eq!(peer, TO_HOST, \"destination\"); assert!(message.contains(FROM_HOST)); } } }\n#[tokio::test]\nasync fn minus_32009_send_seam() { match error { A2AError::ConsentUnclassified { direction: IntentDirection::Send, reason } => { assert_eq!(reason, expected); } } }\n#[tokio::test]\nasync fn minus_32009_accept_seam() { assert_eq!(nack.error.code, CODE_CONSENT_UNCLASSIFIED, \"accept seam\"); let reason = serde_json::from_value::<UnclassifiedReason>(v.clone()).unwrap(); assert_eq!(nack_reason(&nack.error), expected, \"accept seam reason\"); let reachable = [UnclassifiedReason::Absent, UnclassifiedReason::NonCanonical, UnclassifiedReason::Oversized]; }\n#[tokio::test]\nasync fn non_conflation_both_ways() { assert_ne!(nack.error.code, CODE_CONSENT_UNCLASSIFIED, \"-32001 direction\"); assert_ne!(nack.error.code, CODE_INTENT_DENIED, \"-32009 direction\"); }\n#[tokio::test]\nasync fn the_third_code() { assert_eq!(nack.error.code, CODE_CONSENT_EXPIRED, \"expiry stays distinct\"); }\n";
const GOOD_TWO_HOST_PROOF: &str = "#[test]\nfn crossing_uses_two_processes() { let _ = env!(\"CARGO_BIN_EXE_maos\"); mint_pems(); let worker_manifest = (); assert!(matches!(first, HostBOutcome::Ran { .. })); assert_eq!(host_a_frame_id, host_b_frame_id); let _ = MAOS_AUDIT_DB; }\n#[test]\nfn sink_uninstalled() {}\n#[test]\nfn duplicate_frame_is_typed() { assert!(matches!(second, HostBOutcome::Duplicate { .. })); }\nimpl Drop for RunningDaemon {}\n";
const GOOD_TEST_STUB: &str = "#[test]\nfn placeholder() {}\n";

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

fn lay_green(root: &Path) {
    write_file(root, TOPOLOGY, GOOD_TOPOLOGY);
    write_file(root, DELEGATION_RS, GOOD_DELEGATION);
    write_file(root, MAILBOX_RS, GOOD_MAILBOX);
    write_file(root, MAIN_RS, GOOD_MAIN);
    write_file(root, ORCHESTRATOR_RS, GOOD_ORCHESTRATOR);
    write_file(root, A2A_ROUTER_RS, GOOD_A2A_ROUTER);
    write_file(root, WORKER_CLI_RS, GOOD_WORKER_CLI);
    write_file(root, BIN_LIB_RS, GOOD_BIN_LIB);
    write_file(root, WORKFLOW, GOOD_WORKFLOW);
    write_file(root, CONSENT_REFUSAL_RS, GOOD_CONSENT_REFUSAL);
    write_file(root, TWO_HOST_PROOF_RS, GOOD_TWO_HOST_PROOF);
    for test in [
        HOST_GRANTS_RS,
        BOUNDED_POSTURES_RS,
        WORKER_COMPLETION_TEST,
        WORKER_MANIFESTS_TEST,
        DELEGATION_LEG_TEST,
        TOPOLOGY_DELEGATION_TEST,
    ] {
        write_file(root, test, GOOD_TEST_STUB);
    }
}

struct Verdict {
    passed: bool,
    stdout: String,
    success: bool,
}

fn run_gate(plant: impl FnOnce(&Path)) -> Verdict {
    let dir = tempfile::tempdir().unwrap();
    lay_green(dir.path());
    plant(dir.path());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["check-j1-loopback-delegation", "--json"])
        .current_dir(dir.path())
        .output()
        .expect("xtask must run");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Verdict {
        passed: stdout.contains("\"passed\":true") || stdout.contains("\"passed\": true"),
        stdout,
        success: output.status.success(),
    }
}

fn assert_red(verdict: &Verdict, expected_detail: &str, vector: &str) {
    assert!(
        !verdict.passed && !verdict.success,
        "the Blocking gate stayed green for `{vector}`\nstdout:\n{}",
        verdict.stdout
    );
    assert!(
        verdict.stdout.contains(expected_detail),
        "the finding must name `{expected_detail}` for `{vector}`\nstdout:\n{}",
        verdict.stdout
    );
}

#[test]
fn baseline_fixture_tree_is_green() {
    let verdict = run_gate(|_| {});
    assert!(
        verdict.passed && verdict.success,
        "the fixture baseline must be green\nstdout:\n{}",
        verdict.stdout
    );
}

#[test]
fn fork_shape_keeps_the_loopback_boundary_true_and_green() {
    // 1b's total-replacement vector removes the loopback call entirely. This is its
    // missing twin: the deployed fork retains that call beside `maos_a2a_tcp` and
    // `CrossHostVerified`, so the honest permanent-loopback value remains true.
    let verdict = run_gate(|root| write_file(root, DELEGATION_RS, FORK_DELEGATION));
    assert!(
        verdict.passed && verdict.success,
        "the fork must not report a moved static boundary\nstdout:\n{}",
        verdict.stdout
    );
    assert!(
        verdict
            .stdout
            .contains("\"loopback_from_host_unverified\":true")
            || verdict
                .stdout
                .contains("\"loopback_from_host_unverified\": true"),
        "the retained loopback arm must publish true\nstdout:\n{}",
        verdict.stdout
    );
}

#[test]
fn deleting_any_identity_proof_needle_reds_under_its_leg() {
    let needle = "HostBOutcome::Duplicate";
    let verdict = run_gate(|root| {
        let weakened = GOOD_TWO_HOST_PROOF.replace(needle, "DuplicateWasRemoved");
        assert_ne!(
            weakened, GOOD_TWO_HOST_PROOF,
            "the fixture mutation must apply"
        );
        write_file(root, TWO_HOST_PROOF_RS, &weakened);
    });
    assert_red(
        &verdict,
        "cross-host-identity-proof",
        "an identity-proof assertion skeleton was removed",
    );
}

#[test]
fn inert_outcome_bindings_red_identity_proof_leg() {
    // §A6 review P12 — the twin of the deletion vector: the assertion-SHAPED
    // needles must not be satisfiable by inert text. A proof whose outcome
    // checks degrade into `let` bindings (assertions deleted, registrations
    // kept) must red the leg — before the needles were assertion-shaped, this
    // exact degradation kept the gate green.
    let verdict = run_gate(|root| {
        let degraded = GOOD_TWO_HOST_PROOF
            .replace(
                "assert!(matches!(first, HostBOutcome::Ran { .. }))",
                "let outcome = HostBOutcome::Ran;",
            )
            .replace(
                "assert!(matches!(second, HostBOutcome::Duplicate { .. }))",
                "let outcome = HostBOutcome::Duplicate;",
            );
        assert_ne!(
            degraded, GOOD_TWO_HOST_PROOF,
            "the fixture mutation must apply"
        );
        write_file(root, TWO_HOST_PROOF_RS, &degraded);
    });
    assert_red(
        &verdict,
        "cross-host-identity-proof",
        "an inert-binding proof kept the type names but lost the assertions",
    );
}

#[test]
fn missing_identity_proof_is_a_finding_not_a_skip() {
    let verdict = run_gate(|root| std::fs::remove_file(root.join(TWO_HOST_PROOF_RS)).unwrap());
    assert_red(
        &verdict,
        "cross-host-identity-proof",
        "the identity-proof subject was removed",
    );
}

#[test]
fn fewer_than_three_registered_identity_tests_reds() {
    let verdict = run_gate(|root| {
        let reduced = GOOD_TWO_HOST_PROOF.replacen("#[test]", "#[cfg(test)]", 1);
        write_file(root, TWO_HOST_PROOF_RS, &reduced);
    });
    assert_red(
        &verdict,
        "carries 2 registered test functions",
        "the identity proof has fewer than three registered tests",
    );
}

#[test]
fn unenrolling_two_host_proof_reds_completion_vectors_leg() {
    let verdict = run_gate(|root| {
        let unenrolled = GOOD_WORKFLOW.replace(
            "      - run: cargo test -p maos-bin --test two_host_delegation_2b -- --test-threads=1\n",
            "",
        );
        assert_ne!(
            unenrolled, GOOD_WORKFLOW,
            "the workflow mutation must apply"
        );
        write_file(root, WORKFLOW, &unenrolled);
    });
    assert_red(
        &verdict,
        "completion-vectors-enrolled",
        "the two-host proof target is not enrolled",
    );
}
