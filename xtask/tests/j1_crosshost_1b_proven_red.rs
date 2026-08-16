#![forbid(unsafe_code)]

//! Proven-red vectors for `check-j1-loopback-delegation`'s **j1-crosshost-1b legs**
//! (AC2.1 · AC2.2 · AC2.2a · AC2.4 · AC2.5).
//!
//! Four things landed on a gate that already blocks: the `consent-refusal-proofs`
//! leg, the repaired boundary leg, the derived CI-enrollment set, and the SHARED
//! vacuous-green guard in `gate_common`. Each must be FALSIFIABLE or the story
//! registered four green boxes and called it coverage.
//!
//! Per-story file, following `2a`'s precedent (`j1_crosshost_2a_proven_red.rs`): a
//! story's falsifiers should red for that story's reasons, and each file owns its own
//! CI step. Mirrors both existing files exactly — the REAL `xtask` binary against a
//! self-contained fixture tree in a tempdir, one invariant mutated per vector.
//!
//! ## Two vectors here exist because a guard can itself be vacuous
//!
//! `oracle_green = findings.is_empty()` is blind to a leg that read nothing. `1b`
//! landed the per-leg `LegAudit` record in `xtask/src/gate_common.rs` and made this
//! gate its first consumer across all six legs. A vacuity guard nobody can prove
//! fires is the defect it exists to prevent, so:
//!
//! * [`emptying_the_derived_enrollment_set_must_red_as_VACUOUS`] proves the guard
//!   FIRES and NAMES the leg — the derivation would otherwise pass green over an
//!   empty input set, which is exactly how a filesystem walk goes decorative; and
//! * [`every_leg_publishes_a_non_zero_check_count`] is its positive control: without
//!   it, a guard that always reported "fine" would look identical.
//!
//! ## Why no leg here invokes `cargo`
//!
//! `1a`'s F3, still true: this fixture tree is a tempdir with NO `Cargo.toml`, and
//! `check_vetting_attestation::invoke_leg` builds `Command::new("cargo")` without
//! setting `current_dir`. Copying that template would make the gate red no matter
//! what is planted — every vector below would "pass" for the wrong reason while CI
//! reported green. Every `1b` leg is source-STATIC and root-relative.

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

/// The gate's TENTH governed file — this story's subject.
const CONSENT_REFUSAL_RS: &str = "crates/maos-bin/tests/consent_refusal_1b.rs";
/// The directory the enrolled `cargo test` set is DERIVED from, and the other two
/// members of that set. They must exist as files or `2a`'s enrollment vectors have
/// nothing to un-enroll.
const MAOS_BIN_TESTS_DIR: &str = "crates/maos-bin/tests";
const WORKER_COMPLETION_TEST: &str = "crates/maos-bin/tests/worker_completion_2a.rs";
const WORKER_MANIFESTS_TEST: &str = "crates/maos-bin/tests/worker_manifests_2a.rs";

const GOOD_TOPOLOGY: &str = "[topology]\nname = \"j1-founder-loop\"\n\n\
     [[topology.spirits]]\nmanifest = \"../orchestrator/manifest.toml\"\n\n\
     [[topology.spirits]]\nmanifest = \"../worker/manifest.toml\"\n\
     host = \"developer-remote-host\"\n";

/// The COMPOSITION ROOT of the J1 delegation. AC2.2a's repaired boundary leg reads
/// this file, not `router.rs`: rung 2's flip is a change of which router the root
/// composes, which is not a text change in `router.rs` at all.
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

/// The peer RESOLUTION EXPRESSION, not the bare `frame.from.host_id` token: the old
/// boundary leg needled the token and was pinned green by
/// `handle_intake_verified`'s own TLS-mismatch message literal at `router.rs:1514`.
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
     \x20     - run: cargo test -p maos-bin --test consent_refusal_1b -- --test-threads=1\n";

/// The thirteen refusal-proof assertion SKELETONS the `consent-refusal-proofs` leg
/// needles, laid out one concern per function so each vector below can surgically
/// remove exactly one and observe the gate flip.
///
/// This is a fixture, not a copy: the leg matches structure, so carrying the shapes
/// is sufficient and cannot drift the way a copied 480-line file would.
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

const GOOD_TEST_STUB: &str = "#[test]\nfn placeholder() {}\n";

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// Lay down a fixture tree that the gate must find GREEN.
///
/// **This function exists three times** (`1a`, `2a`, here) and every governed file
/// must be laid in all three: `read()` pushes a Finding when a governed file is
/// missing, so a tree that misses one reds the gate on the TREE, at which point
/// every planted vector in THAT file passes for the wrong reason and CI still
/// reports green.
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
    write_file(root, WORKER_COMPLETION_TEST, GOOD_TEST_STUB);
    write_file(root, WORKER_MANIFESTS_TEST, GOOD_TEST_STUB);
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

/// Remove exactly one assertion skeleton from the refusal proofs.
fn weaken_consent_proof(root: &Path, from: &str, to: &str) {
    let weakened = GOOD_CONSENT_REFUSAL.replace(from, to);
    assert_ne!(
        weakened, GOOD_CONSENT_REFUSAL,
        "the mutation `{from}` must actually change the fixture, or the vector proves nothing"
    );
    write_file(root, CONSENT_REFUSAL_RS, &weakened);
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
// AC2.1 — the consent-refusal leg. Deleting or weakening any assertion must RED.
// ═══════════════════════════════════════════════════════════════════

/// THE headline regression: the proofs stop asserting the peer-side refusal at all.
/// Without the typed `IntentDeniedAtPeer` assertion a disallowed intent that was
/// silently ADMITTED would satisfy whatever remains.
#[test]
fn dropping_the_minus_32001_deny_assertion_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "A2AError::IntentDeniedAtPeer { peer, message } => {",
            "Ok(admitted) => {",
        );
    });
    assert_red(
        &v,
        "A2AError::IntentDeniedAtPeer{peer,message",
        "a disallowed intent is admitted instead of refused",
    );
}

/// The refusal must stay LEGIBLE as source-keyed. On loopback the
/// `accept_allowlist` consulted is the SOURCE host's (`router.rs:1087-1090`); a
/// proof that stops asserting the NACK names `founder-loop-host` would read
/// identically if the deny were keyed off the destination.
#[test]
fn dropping_the_source_host_assertion_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert!(message.contains(FROM_HOST));",
            "assert!(!message.is_empty());",
        );
    });
    assert_red(
        &v,
        "message.contains(FROM_HOST)",
        "the source-keying asymmetry is no longer observable",
    );
}

/// Both `peer_id` strings must appear literally, so a production rename reds the
/// proof instead of silently re-pointing which allowlist judges the frame.
#[test]
fn unpinning_the_source_peer_id_literal_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_eq!(FROM_HOST, \"founder-loop-host\");",
            "let _ = FROM_HOST;",
        );
    });
    assert_red(
        &v,
        "assert_eq!(FROM_HOST,\\\"founder-loop-host\\\"",
        "the source peer_id literal is unpinned",
    );
}

/// Fail-closed is required at BOTH seams (architecture §7.2). Dropping the send
/// seam means an unclassified frame could leave the host and be judged at the far
/// end instead of never being transmitted.
#[test]
fn dropping_the_minus_32009_send_seam_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "A2AError::ConsentUnclassified { direction: IntentDirection::Send, reason } => {",
            "Err(other) => {",
        );
    });
    assert_red(
        &v,
        "A2AError::ConsentUnclassified{direction:IntentDirection::Send,",
        "the -32009 send seam is no longer asserted",
    );
}

/// And the accept seam, the other half of the same invariant.
#[test]
fn dropping_the_minus_32009_accept_seam_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_eq!(nack.error.code, CODE_CONSENT_UNCLASSIFIED, \"accept seam\");",
            "assert!(nack.error.code < 0);",
        );
    });
    assert_red(
        &v,
        "assert_eq!(nack.error.code,CODE_CONSENT_UNCLASSIFIED",
        "the -32009 accept seam is no longer asserted",
    );
}

/// A numeric-only deny assertion makes the refusal illegible — the operator sees
/// "-32009" and cannot tell absent from oversized. `fail_closed_8_8.rs:128-135`
/// exists to prevent exactly this, so the typed read-back is mandatory.
#[test]
fn dropping_the_typed_reason_readback_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "let reason = serde_json::from_value::<UnclassifiedReason>(v.clone()).unwrap();",
            "let reason = v.to_string();",
        );
    });
    assert_red(
        &v,
        "serde_json::from_value::<UnclassifiedReason>(",
        "the deny reason is asserted numerically instead of typed",
    );
}

/// Every reachable `UnclassifiedReason` must be covered. Dropping `Oversized`
/// leaves the `> MAX_CANONICAL_INTENT_LEN` boundary unproven.
#[test]
fn dropping_a_reachable_unclassified_reason_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(root, "UnclassifiedReason::Oversized", "()");
    });
    assert_red(
        &v,
        "UnclassifiedReason::Oversized",
        "the oversized-intent reason is no longer covered",
    );
}

/// §A6 review P1 — the proofs must stay REGISTERED. Every needle is satisfied by
/// assertion-shaped text; delete one `#[tokio::test]` and `cargo test --test
/// consent_refusal_1b` runs six tests instead of seven while every needle (and
/// the gate) stays green — unless the leg COUNTS.
#[test]
fn deleting_a_test_attribute_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "#[tokio::test]\nasync fn the_third_code()",
            "async fn the_third_code()",
        );
    });
    assert_red(
        &v,
        "expected at least 7",
        "a refusal proof stopped being a registered test",
    );
}

/// §A6 review P3 — the AC1.1 positive control: without a working positive in
/// the same binary, every negative below can pass vacuously — a pairing that
/// admits nothing "refuses" everything.
#[test]
fn dropping_the_positive_control_assertion_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(root, "Some(DELEGATION_CONSENT_INTENT.to_string())", "None");
    });
    assert_red(
        &v,
        "Some(DELEGATION_CONSENT_INTENT.to_string())",
        "the local positive control was deleted",
    );
}

/// §A6 review P3 — AC1.2's peer binding: the typed deny must name the
/// DESTINATION peer, and the destructure needle alone could not tell.
#[test]
fn dropping_the_destination_peer_binding_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_eq!(peer, TO_HOST, \"destination\");",
            "let _ = peer;",
        );
    });
    assert_red(
        &v,
        "assert_eq!(peer,TO_HOST,",
        "the deny no longer names the destination peer",
    );
}

/// §A6 review P3 — the accept seam's TYPED reason must be BOUND to the
/// expectation, not merely parsed: the helper needle alone let the comparison
/// be deleted, leaving a numeric-only deny.
#[test]
fn dropping_the_accept_seam_typed_binding_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_eq!(nack_reason(&nack.error), expected, \"accept seam reason\");",
            "assert!(nack.error.code < 0);",
        );
    });
    assert_red(
        &v,
        "assert_eq!(nack_reason(&nack.error),expected,",
        "the typed reason is parsed but never bound to the expectation",
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC2.4 — non-conflation, BOTH directions. Each is its own vector, because
// asserting one direction and calling it "both ways" is the defect.
// ═══════════════════════════════════════════════════════════════════

/// Collapse `-32001` into `-32009`: a policy deny reported as unclassified sends
/// the operator hunting a malformed frame for a frame whose intent was fine.
#[test]
fn collapsing_minus_32001_into_minus_32009_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_ne!(nack.error.code, CODE_CONSENT_UNCLASSIFIED, \"-32001 direction\");",
            "// conflation tolerated",
        );
    });
    assert_red(
        &v,
        "assert_ne!(nack.error.code,CODE_CONSENT_UNCLASSIFIED",
        "-32001 collapsed into -32009",
    );
}

/// The inverse collapse, which a one-directional proof would miss entirely: a
/// malformed frame reported as a policy refusal sends the operator hunting an
/// allowlist for a frame that never carried an intent.
#[test]
fn collapsing_minus_32009_into_minus_32001_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_ne!(nack.error.code, CODE_INTENT_DENIED, \"-32009 direction\");",
            "// conflation tolerated",
        );
    });
    assert_red(
        &v,
        "assert_ne!(nack.error.code,CODE_INTENT_DENIED",
        "-32009 collapsed into -32001",
    );
}

/// The THIRD code. `prepare_outbound` stamps a TTL on every real frame and
/// `handle_intake_inner` enforces it, so `-32003` is live and must stay distinct
/// from both deny codes.
#[test]
fn dropping_the_expiry_code_assertion_must_red() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_eq!(nack.error.code, CODE_CONSENT_EXPIRED, \"expiry stays distinct\");",
            "let _ = nack;",
        );
    });
    assert_red(
        &v,
        "assert_eq!(nack.error.code,CODE_CONSENT_EXPIRED",
        "the -32003 expiry code is no longer kept distinct",
    );
}

/// The leg must not be satisfiable by PROSE. `structural()` strips comment lines
/// before matching, so commenting an assertion out reds the gate — an invariant
/// described is not an invariant asserted.
#[test]
fn commenting_out_an_assertion_does_not_satisfy_the_leg() {
    let v = run_gate(|root| {
        weaken_consent_proof(
            root,
            "assert_eq!(nack.error.code, CODE_CONSENT_EXPIRED, \"expiry stays distinct\");",
            "// assert_eq!(nack.error.code, CODE_CONSENT_EXPIRED, \"expiry stays distinct\");",
        );
    });
    assert_red(
        &v,
        "assert_eq!(nack.error.code,CODE_CONSENT_EXPIRED",
        "the expiry assertion moved into a comment",
    );
}

/// Reformatting must NOT flip the leg. A `Blocking` gate a formatter can red is a
/// false-alarm machine — the exact defect `246660f9` fixed in this gate, whose
/// commit message asked `1b` to reuse the same normalization for its refusal legs.
#[test]
fn reformatting_the_refusal_proofs_does_not_flip_the_leg() {
    let v = run_gate(|root| {
        // Split every assertion across lines the way `cargo fmt` would.
        let reformatted = GOOD_CONSENT_REFUSAL
            .replace("assert_eq!(nack.error.code, ", "assert_eq!(\n        nack.error.code,\n        ")
            .replace("assert_ne!(nack.error.code, ", "assert_ne!(\n        nack.error.code,\n        ")
            .replace(
                "A2AError::IntentDeniedAtPeer { peer, message } => {",
                "A2AError::IntentDeniedAtPeer {\n            peer,\n            message,\n        } => {",
            );
        write_file(root, CONSENT_REFUSAL_RS, &reformatted);
    });
    assert!(
        v.passed && v.success,
        "the refusal leg must depend on STRUCTURE, not on layout\nstdout:\n{}",
        v.stdout
    );
}

/// A missing subject is a FINDING, never a skip. This is the tenth governed file;
/// deleting it must red rather than quietly reduce the gate to five legs.
#[test]
fn a_missing_refusal_proof_file_is_a_finding_never_a_skip() {
    let v = run_gate(|root| {
        std::fs::remove_file(root.join(CONSENT_REFUSAL_RS)).unwrap();
    });
    assert_red(
        &v,
        "cannot read crates/maos-bin/tests/consent_refusal_1b.rs",
        "the refusal proofs were deleted",
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC2.5 — the DERIVED enrollment set. This line is the only thing connecting the
// static oracle to the behaviour it judges.
// ═══════════════════════════════════════════════════════════════════

/// Delete `--test consent_refusal_1b` from the workflow. The gate MUST red: it
/// would otherwise keep finding the right words in a file that never runs.
#[test]
fn deleting_the_consent_refusal_enrollment_line_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            &GOOD_WORKFLOW.replace("--test consent_refusal_1b", "--test something_else"),
        );
    });
    assert_red(
        &v,
        "--test consent_refusal_1b",
        "consent-refusal CI enrollment deleted",
    );
}

/// The falsifier the DERIVATION itself needs. A derivation that cannot be shown to
/// notice a NEW file is a const list wearing a walk: plant an un-enrolled
/// `*_1b.rs` and the gate must red by construction, with nobody having remembered
/// to add a const line.
#[test]
fn planting_an_unenrolled_j1_test_file_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            "crates/maos-bin/tests/xyz_1b.rs",
            "#[test]\nfn never_run_in_ci() {}\n",
        );
    });
    assert_red(
        &v,
        "--test xyz_1b",
        "a new J1 test file was added without enrolling it",
    );
}

/// Enrollment must live in THIS gate's Blocking job, unchanged from `2a`'s
/// scoping: the same `--test` lines under an unrelated job are enrollment in name
/// only.
#[test]
fn consent_enrollment_moved_to_another_job_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            "jobs:\n  some-other-job:\n    steps:\n\
             \x20     - run: cargo test -p maos-bin --test consent_refusal_1b -- --test-threads=1\n\
             \x20 check-j1-loopback-delegation:\n    steps:\n\
             \x20     - run: cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1\n\
             \x20     - run: cargo test -p maos-bin --test worker_manifests_2a -- --test-threads=1\n",
        );
    });
    assert_red(
        &v,
        "does not invoke `cargo test -p maos-bin --test consent_refusal_1b`",
        "consent enrollment moved out of the Blocking job",
    );
}

/// §A6 review P4 — a `--test` token in a step NAME is enrollment in prose: the
/// token survives in the job block while the behavioural test stops running.
/// The moved-to-another-job vector covers cross-JOB escape; this is the
/// same-job one.
#[test]
fn enrollment_in_prose_is_not_enrollment_must_red() {
    let v = run_gate(|root| {
        write_file(
            root,
            WORKFLOW,
            "jobs:\n  check-j1-loopback-delegation:\n    steps:\n\
             \x20     - name: cargo test -p maos-bin --test consent_refusal_1b\n\
             \x20       run: cargo test -p maos-bin -- --test-threads=1\n\
             \x20     - run: cargo test -p maos-bin --test delegation_leg_1a --test topology_delegation_1a -- --test-threads=1\n\
             \x20     - run: cargo test -p maos-bin --test worker_completion_2a -- --test-threads=1\n\
             \x20     - run: cargo test -p maos-bin --test worker_manifests_2a -- --test-threads=1\n",
        );
    });
    assert_red(
        &v,
        "does not invoke `cargo test -p maos-bin --test consent_refusal_1b`",
        "the enrollment token survived in a step name",
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC2.3 — the leg-omission null control. `ledger_leg_names()` hand-lists SIX names
// against six invoked legs, and until this story it was the ONLY
// `ledger_leg_names()` owner in the crate with no derivation test
// (`check_reza_production_path.rs:1164`, `check_cross_region_consensus.rs:315`,
// `check_multi_tenant_loom.rs:1933` and `check_multi_region_slo.rs:752` all have
// one). `2a` added three names by hand with nothing reconciling them, so a leg added
// and forgotten in the accessor reds nothing.
//
// The test lives HERE, in a file CI already invokes, rather than in a new
// `xtask/tests/` target nobody would have to remember to enroll — which is the
// failure this story exists to close. It is out of `xtask/src/` deliberately:
// an in-`src` `#[cfg(test)]` module is KLOC-charged (D11-E3, `xtask/src/tests/`
// measures 2367 charged lines) and executed by no CI job.
// ═══════════════════════════════════════════════════════════════════

/// The two halves must reconcile: `legs` is `ledger_leg_names()`, `leg_audits` is
/// one record per leg `judge()` actually invoked. A leg added to the runner but
/// forgotten in the accessor makes `judge()` panic outright; a name in the accessor
/// that no leg claims leaves the two collections different lengths. Either way the
/// omission is observable instead of silent.
#[test]
fn ledger_leg_names_reconciles_with_the_legs_actually_invoked() {
    let v = run_gate(|_| {});
    let json: serde_json::Value =
        serde_json::from_str(v.stdout.trim()).expect("the gate emits JSON, not a panic");

    let published: Vec<&str> = json["legs"]
        .as_array()
        .expect("the gate publishes ledger_leg_names()")
        .iter()
        .map(|name| name.as_str().expect("leg names are strings"))
        .collect();
    let audited: Vec<&str> = json["leg_audits"]
        .as_array()
        .expect("the gate publishes per-leg audits")
        .iter()
        .map(|audit| {
            audit["leg"]
                .as_str()
                .expect("audited leg names are strings")
        })
        .collect();

    assert_eq!(
        published.len(),
        audited.len(),
        "ledger_leg_names() published {published:?} but {audited:?} legs ran — a leg listed \
         and never invoked is a green box, and a leg invoked and never listed cannot be \
         reported under its own name"
    );
    for name in &published {
        assert!(
            audited.contains(name),
            "leg `{name}` is published in ledger_leg_names() but no leg reported an audit \
             under it\naudited: {audited:?}"
        );
    }

    // Trap 21 — the names are consumed by `demo_j1`'s beat matcher and by the
    // per-leg record, so a rename is a silent breakage, not a refactor.
    assert_eq!(
        published,
        vec![
            "frame-borne-route-intact",
            "loopback-from-host-unverified",
            "completion-oracle-per-adapter",
            "worker-cli-under-library",
            "completion-vectors-enrolled",
            "consent-refusal-proofs",
        ],
        "renaming or dropping a published leg name breaks demo_j1's hard-coded matcher"
    );
}

// ═══════════════════════════════════════════════════════════════════
// AC2.2 — the SHARED vacuous-green guard, and its own falsifier.
// ═══════════════════════════════════════════════════════════════════

/// The guard's falsifier. Empty the directory the enrollment set is DERIVED from:
/// the derivation then iterates nothing, evaluates NO check, and pushes no finding
/// — so under `oracle_green = findings.is_empty()` alone the leg would be
/// indistinguishable from a leg that passed. The `LegAudit` record is the only
/// thing that can see it, and the finding must NAME the leg.
///
/// A vacuity guard that cannot be shown to fire is the defect it exists to
/// prevent, which is why this vector asserts on the guard's own wording rather
/// than merely on redness.
#[test]
#[allow(non_snake_case)]
fn emptying_the_derived_enrollment_set_must_red_as_VACUOUS() {
    let v = run_gate(|root| {
        std::fs::remove_dir_all(root.join(MAOS_BIN_TESTS_DIR)).unwrap();
    });
    assert_red(
        &v,
        "reported no executed check",
        "the derived enrollment set was emptied",
    );
    assert!(
        v.stdout.contains("completion-vectors-enrolled"),
        "the vacuity finding must NAME the vacuous leg, or a red gate points at the wrong \
         thing\nstdout:\n{}",
        v.stdout
    );
}

/// The guard's positive control. Without this, a guard hard-wired to report
/// "everything fine" would be indistinguishable from a working one — and every
/// per-leg count published below is what AC2.10's per-leg demo beats consume.
#[test]
fn every_leg_publishes_a_non_zero_check_count() {
    let v = run_gate(|_| {});
    assert!(v.passed, "baseline must be green\nstdout:\n{}", v.stdout);
    let audits: serde_json::Value =
        serde_json::from_str(v.stdout.trim()).expect("the gate emits JSON");
    let audits = audits["leg_audits"]
        .as_array()
        .expect("the gate publishes per-leg audits");
    assert_eq!(audits.len(), 6, "all six legs must be audited: {audits:?}");
    for audit in audits {
        assert_eq!(
            audit["ran"].as_bool(),
            Some(true),
            "leg {} must report that it ran",
            audit["leg"]
        );
        assert!(
            audit["checks"].as_u64().unwrap_or(0) > 0,
            "leg {} must report at least one executed check",
            audit["leg"]
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// AC2.2a — the repaired boundary leg, at the door the flip actually arrives
// through.
// ═══════════════════════════════════════════════════════════════════

/// The flipped state, planted. When `j1-crosshost-2b` composes a verified
/// transport at the J1 composition root, this leg must NOT pass silently — the
/// boundary moving is the whole event it exists to surface.
///
/// The pre-`1b` leg could not observe this at all: it read only `router.rs`, and
/// "rung 2 turns verification on" is a change of which router the composition root
/// builds. That made it a leg that published `true` in every possible future.
#[test]
fn boundary_leg_reds_when_the_composition_root_gains_a_verified_transport() {
    let v = run_gate(|root| {
        write_file(
            root,
            DELEGATION_RS,
            &GOOD_DELEGATION.replace(
                "maos_a2a::pairing::paired_loopback_router(&[]).await?",
                "maos_a2a_tcp::TcpA2ATransport::connect(&peer).await?",
            ),
        );
    });
    assert_red(
        &v,
        "boundary MOVED",
        "the composition root now builds a verified transport",
    );
}

/// The boundary is RECORDED, not failed, and it is PUBLISHED — so rung 2 flipping
/// it shows up in a CI diff rather than in a story nobody re-reads.
#[test]
fn the_unverified_boundary_is_published_and_does_not_fail_the_gate() {
    let v = run_gate(|_| {});
    assert!(
        v.passed,
        "the boundary must not fail the gate\nstdout:\n{}",
        v.stdout
    );
    assert!(
        v.stdout.contains("\"loopback_from_host_unverified\":true")
            || v.stdout.contains("\"loopback_from_host_unverified\": true"),
        "the gate must PUBLISH the unverified-wire-identity boundary\nstdout:\n{}",
        v.stdout
    );
}
