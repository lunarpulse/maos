#![forbid(unsafe_code)]

//! Proven-red vectors for `check-j1-loopback-delegation` (story `j1-crosshost-1a`,
//! AC4.3-4.4).
//!
//! AC4.3 is explicit that the skeleton's leg must be the proven-red: *not* a smoke
//! test, *not* a compile check. A planted "route locally anyway" regression must RED
//! the gate. Without these vectors we would have registered an empty box in seven
//! places and called it enrollment — the exact failure shape the story exists to
//! stop.
//!
//! Mirrors `story_10_5_proven_red.rs`: every vector runs the REAL `xtask` binary
//! against a self-contained fixture tree in a tempdir. The gate resolves its
//! governed paths relative to the current directory, so each test lays down its own
//! copy of that tree, mutates exactly one invariant, and asserts the verdict flips.
//!
//! The fixtures are deliberately minimal — the gate reads named invariants out of
//! source text, so a fixture needs only the lines that carry them. A fixture that
//! copied the real files would drift; one that carries the invariants cannot.

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
/// j1-crosshost-2a added three files to the gate's governed list, so this fixture
/// tree must carry them or every 1a vector below reds for the WRONG reason (a
/// missing governed file is a finding, never a skip). 2a's own vectors live in
/// `j1_crosshost_2a_proven_red.rs`; these are green stand-ins only.
const WORKER_CLI_RS: &str = "crates/maos-bin/src/worker_cli.rs";
const BIN_LIB_RS: &str = "crates/maos-bin/src/lib.rs";
const WORKFLOW: &str = ".github/workflows/discipline.yml";
/// j1-crosshost-1b reaches into THIS fixture tree twice, and both would fail
/// SILENTLY if skipped. (1) `crates/maos-bin/tests/consent_refusal_1b.rs` is the
/// gate's TENTH governed file, and `read()` pushes a Finding when a governed file
/// is missing — so a tree that does not lay it reds the gate on the tree ITSELF, at
/// which point every planted vector below "passes" for the wrong reason while CI
/// reports green. (2) the enrolled `cargo test` set is now DERIVED from
/// `crates/maos-bin/tests/`, so the tree must lay the targets whose enrollment it
/// asserts — including `2a`'s two, or `2a`'s own enrollment vectors go vacuous.
/// `lay_green` exists in THREE files now (`1a`, `2a`, `1b`); all three carry these.
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

const GOOD_MAIN: &str = "fn topology_loop() {\n\
     \x20   let frame = delegation_emitter.assign_frame_remote(seq, recipient, role);\n\
     \x20   let goal = delegation_leg.delegate(&iac, frame).await?;\n}\n";

const GOOD_ORCHESTRATOR: &str =
    "pub const DELEGATION_CONSENT_INTENT: &str = \"development-task:write-workspace\";\n\
     #[cfg(test)]\nmod tests {\n\
     \x20   fn validation_paths() {\n\
     \x20       let _ = A2AIntent::new(\"task.assign\");\n\
     \x20       assert!(!A2AIntent::new(\"task.assign\").is_canonical());\n\
     \x20   }\n}\n";

const GOOD_A2A_ROUTER: &str = "fn intake() {\n\
     \x20   let peer_host = match &frame.from.host_id { Some(h) => h.clone(), None => loopback() };\n}\n\
     pub async fn handle_intake_verified(&self, r: Req, p: &PeerId) {}\n";

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

const GOOD_WORKFLOW: &str = "jobs:\n  check-j1-loopback-delegation:\n    steps:\n\
     \x20     - run: cargo test -p maos-bin --test delegation_leg_1a --test topology_delegation_1a -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test worker_manifests_2a -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test consent_refusal_1b -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test two_host_delegation_2b -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test host_grants_2b -- --test-threads=1\n\
     \x20     - run: cargo test -p maos-bin --test bounded_postures_2b -- --test-threads=1\n";

/// The refusal-proof assertion skeletons the `consent-refusal-proofs` leg needles.
/// Structural (whitespace- and comment-insensitive), so this fixture carries the
/// SHAPES rather than a copy of the real 480-line test file.
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

/// The derived enrollment set's other two members. Content is irrelevant — the
/// derivation keys on the FILENAME; what matters is that they exist, so `2a`'s
/// enrollment vectors still have something to be un-enrolled.
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
    write_file(root, WORKFLOW, GOOD_WORKFLOW);
    // j1-crosshost-1b: the tenth governed file, plus the two other members of the
    // DERIVED enrollment set. Miss any of them and this tree's own
    // `baseline_fixture_tree_is_green` reds, which makes every vector below pass
    // for the wrong reason while CI still reports green.
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
// AC4.3 — "route locally anyway", planted five ways
// ═══════════════════════════════════════════════════════════════════

/// THE canonical regression: drop the topology's `host` key. The founder loop still
/// runs, still exits 0, still journals a Worker — and the delegation never happens
/// because the worker is loaded as an ordinary local member.
#[test]
fn route_locally_anyway_by_dropping_the_topology_host_key() {
    let v = run_gate(|root| {
        write_file(
            root,
            TOPOLOGY,
            &GOOD_TOPOLOGY.replace("host = \"developer-remote-host\"\n", ""),
        );
    });
    assert_red(&v, "must declare exactly ONE", "topology host key removed");
}

/// Emit through `assign_frame` instead of `assign_frame_remote`: `host_id: None` and
/// `consent_envelope: None` mean the frame is delivered same-host, unconsented.
#[test]
fn route_locally_anyway_by_emitting_the_same_host_frame() {
    let v = run_gate(|root| {
        write_file(
            root,
            MAIN_RS,
            &GOOD_MAIN.replace("assign_frame_remote", "assign_frame"),
        );
    });
    assert_red(&v, "assign_frame_remote", "same-host emit");
}

/// Let Phase 3 fall through to local delivery instead of failing closed. A
/// host-bearing frame would then be delivered same-host with no peer consent check
/// at all — "route locally anyway" in its purest form. The planted test-only
/// occurrence pins the capture surface: a file-wide keyword search would
/// incorrectly stay green after the production branch was broken.
#[test]
fn route_locally_anyway_by_removing_the_production_fail_closed_error() {
    let v = run_gate(|root| {
        let broken_production = GOOD_MAILBOX.replace(
            "IacBusError::CrossHostNotConfigured { host_id: hosts }",
            "deliver_locally(frame)",
        );
        let test_only_decoy = "\n#[cfg(test)]\nmod tests {\n    fn still_mentions_error() {\n        \
                               let _ = IacBusError::CrossHostNotConfigured { host_id: hosts };\n    }\n}\n";
        write_file(
            root,
            MAILBOX_RS,
            &format!("{broken_production}{test_only_decoy}"),
        );
    });
    assert_red(
        &v,
        "must fail closed",
        "production fail-closed error removed while test-only marker remains",
    );
}

/// Replacing the production closure with local delivery must RED even if a test
/// module preserves the exact fail-closed expression. The gate must not let
/// `#[cfg(test)]` source satisfy the production invariant.
#[test]
fn route_locally_anyway_but_keep_a_matching_expression_in_a_test_module() {
    let v = run_gate(|root| {
        let production_fallback = "fn phase_three() {\n    deliver_locally(frame)?;\n}\n";
        let test_only_match = "\n#[cfg(test)]\nmod tests {\n    fn preserves_fail_closed_shape() {\n        \
                               let router = self.a2a_router.get().ok_or_else(|| {\n            \
                               IacBusError::CrossHostNotConfigured { host_id: hosts }\n        })?;\n    }\n}\n";
        write_file(
            root,
            MAILBOX_RS,
            &format!("{production_fallback}{test_only_match}"),
        );
    });
    assert_red(
        &v,
        "must fail closed",
        "production fallback with matching test-only fail-closed expression",
    );
}

/// Drop the pump's `host_id` strip. The re-delivered frame re-enters the cross-host
/// branch and loops instead of reaching the consumer.
#[test]
fn delegation_breaks_when_the_pump_stops_stripping_host_id() {
    let v = run_gate(|root| {
        write_file(
            root,
            DELEGATION_RS,
            &GOOD_DELEGATION.replace("addr.host_id = None;", "// stripped"),
        );
    });
    assert_red(&v, "must strip", "host_id strip removed");
}

/// Reintroduce the env shortcut. This is the subtlest vector: the frame still routes,
/// so every routing assertion stays green while the Worker silently takes its task
/// from the environment again — the delegation becomes decorative.
#[test]
fn delegation_becomes_decorative_when_the_env_shortcut_returns() {
    let v = run_gate(|root| {
        write_file(
            root,
            MAIN_RS,
            &format!("{GOOD_MAIN}\nlet t = std::env::var(\"MAOS_WORKER_TASK\").unwrap();\n"),
        );
    });
    assert_red(&v, "MAOS_WORKER_TASK", "env shortcut reintroduced");
}

/// Rename the consent intent to a job category. ADR-012 names effect authority; a
/// job title does not state what the receiving host is authorizing.
#[test]
fn consent_intent_must_name_effect_authority_not_a_job_category() {
    let v = run_gate(|root| {
        write_file(
            root,
            ORCHESTRATOR_RS,
            &GOOD_ORCHESTRATOR.replace("development-task:write-workspace", "development-task"),
        );
    });
    assert_red(&v, "effect authority", "intent narrowed to a job category");
}

/// A dotted consent intent fails closed at the sender, so the wire looks broken
/// rather than refused. The pin-test in the green fixture proves the gate does NOT
/// confuse the two.
#[test]
fn dotted_consent_intent_is_caught_but_the_pin_test_is_not() {
    let v = run_gate(|root| {
        let production_regression = GOOD_ORCHESTRATOR.replacen(
            "#[cfg(test)]",
            "fn production() { let i = A2AIntent::new(\"task.assign\"); }\n#[cfg(test)]",
            1,
        );
        write_file(root, ORCHESTRATOR_RS, &production_regression);
    });
    assert_red(&v, "non-canonical", "dotted consent intent");

    // And the inverse: the pin-test alone (present in every green run above) must
    // NOT red the gate. `baseline_fixture_tree_is_green` covers it; this assertion
    // states the intent so a future tightening of the predicate cannot silently
    // start failing the pin.
    assert!(
        GOOD_ORCHESTRATOR.contains("is_canonical"),
        "the green fixture must retain the task.assign pin-test, which is what makes \
         the previous assertion a discrimination rather than a keyword match"
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC4.4 — Vex's boundary leg
// ═══════════════════════════════════════════════════════════════════

/// The boundary is RECORDED, not failed: the J1 delegation composes
/// `paired_loopback_router`, whose transport calls `handle_intake` directly, so
/// `frame.from.host_id` is self-asserted and the frame chooses its own judge.
#[test]
fn loopback_from_host_is_recorded_as_unverified_not_as_a_failure() {
    let v = run_gate(|_| {});
    assert!(
        v.passed,
        "the boundary must not fail the gate\nstdout:\n{}",
        v.stdout
    );
    assert!(
        v.stdout.contains("\"loopback_from_host_unverified\":true")
            || v.stdout.contains("\"loopback_from_host_unverified\": true"),
        "the gate must PUBLISH the unverified-wire-identity boundary so rung 2 flipping it \
         shows up in a CI diff\nstdout:\n{}",
        v.stdout
    );
}

/// When rung 2 turns verification on, the leg must not pass silently — the boundary
/// moving is the event this leg exists to surface.
///
/// Retargeted by j1-crosshost-1b AC2.2a. The old vector deleted
/// `pub async fn handle_intake_verified` from `router.rs`, which was a mutation of a
/// needle that could never actually flip: the token was ALSO pinned by
/// `handle_intake_verified`'s own TLS-mismatch message literal, and rung 2's real
/// trigger is a change of composition root, not a text change in `router.rs`. Both
/// doors are now plantable — this one is the shared-intake door; the
/// composition-root door is `j1_crosshost_1b_proven_red.rs`.
#[test]
fn boundary_leg_reds_when_the_shared_intake_stops_self_asserting() {
    let v = run_gate(|root| {
        write_file(
            root,
            A2A_ROUTER_RS,
            &GOOD_A2A_ROUTER.replace(
                "let peer_host = match &frame.from.host_id {",
                "let peer_host = self.tls_verified_peer(&conn); if false {",
            ),
        );
    });
    assert_red(
        &v,
        "STATIC loopback boundary shape changed",
        "shared intake no longer resolves the peer from the frame's own host_id",
    );
}
