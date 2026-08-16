#![forbid(unsafe_code)]

//! Gate — `check-j1-loopback-delegation` (story `j1-crosshost-1a`, AC4).
//!
//! The J1 `developer-remote` delegation is **frame-borne**: the Orchestrator emits
//! a real `task.assign` carrying an ADR-012 consent envelope, it routes through the
//! loopback A2A layer, and an in-process consumer drains it and hands the payload's
//! `goal` to the Worker spawn. Every part of that is new, and the failure mode that
//! matters is silent: **route locally anyway**. Drop the topology's `host` key, or
//! let the mailbox fall through to local delivery when no router is installed, and
//! the founder loop still runs, still exits 0, and still journals a Worker — while
//! proving nothing about the wire the v1.0 cross-host rung depends on.
//!
//! ## Why this is a gate and not a test
//!
//! A test that is not behind a gate is a suggestion: it rots, someone "fixes" a red
//! assertion by relaxing it, and the routing regression walks through. That is
//! `smoke_cli_wrapper_8_12` with extra steps — a Tier-1 control with no CI
//! invocation. This gate exists so the regression has to get past a `Blocking`
//! oracle, not past a reviewer's memory.
//!
//! ## Legs
//!
//! 1. **`frame-borne-route-intact`** — the proven-red. Five conditions that are each
//!    individually necessary for the delegation to be frame-borne. Breaking any one
//!    of them is a "route locally anyway" regression, and `xtask/tests/` plants each
//!    one against a fixture tree and asserts this gate goes RED.
//! 2. **`loopback-from-host-unverified`** — a recorded **boundary**, not a failure.
//!    The J1 delegation path composes `LoopbackA2ARouter`, which calls
//!    `handle_intake` directly, so `frame.from.host_id` is self-asserted and the
//!    frame chooses which `accept_allowlist` judges it. The leg reads the
//!    COMPOSITION ROOT — the file where rung 2's flip actually happens — so when a
//!    verified transport replaces the loopback router the leg flips and the change
//!    is visible in a CI diff instead of buried in a story nobody re-reads.
//! 3. **`completion-oracle-per-adapter`** (`2a`) — neither real adapter may decide
//!    completion from "clean exit + a non-empty final stdout line".
//! 4. **`worker-cli-under-library`** (`2a`) — `worker_cli` stays under the library,
//!    or every relocated vector ceases to exist rather than failing.
//! 5. **`completion-vectors-enrolled`** (`2a`, derivation by `1b`) — every J1
//!    `crates/maos-bin/tests/` target is named in THIS gate's Blocking job. The
//!    enrolled set is DERIVED from the directory, never hand-listed: a const list is
//!    one forgotten line away from a dead test behind a green gate.
//! 6. **`consent-refusal-proofs`** (`1b`) — the ADR-012 refusal assertions exist and
//!    are correctly shaped in `crates/maos-bin/tests/consent_refusal_1b.rs`: the
//!    `-32001` typed deny, both `-32009` seams with a typed reason, the both-ways
//!    non-conflation, and `-32003` kept distinct. Seventeen structural needles
//!    **plus a `#[tokio::test]` registration count** — the shapes must live in
//!    executing tests, not dead functions (§A6 review P1/P3).
//!
//! ## Vacuity
//!
//! `oracle_green = findings.is_empty()` cannot distinguish a leg that PASSED from a
//! leg that read nothing. Every leg therefore records a
//! [`crate::gate_common::LegAudit`], and a leg reporting `!ran || checks == 0`
//! hard-FAILs — including the derived enrollment set coming back empty, which is
//! how a filesystem derivation goes quietly decorative.
//!
//! Binding class: [`crate::gate_common::BindingClass::Blocking`] — hermetic (reads
//! committed sources), so a violation reds CI at HEAD regardless of `CURRENT_PHASE`.
//!
//! Every leg is source-STATIC and root-relative, honouring `run_with_root`. A
//! `cargo`-invoking leg would inherit the proven-red tempdir (which has no
//! `Cargo.toml`), vacuum every planted vector, and report green.

use crate::gate_common::{dev_enforced_red_blocks, vacuous_legs, BindingClass, LegAudit};
use std::fs;
use std::path::{Path, PathBuf};

/// The consent intent the delegation must carry. ADR-012 names **effect
/// authority**: what is granted is a T3 worker running
/// `codex exec --sandbox workspace-write` — arbitrary code execution and
/// filesystem mutation on the receiver. A job-category name would not say that.
const DELEGATION_INTENT: &str = "development-task:write-workspace";
/// The topology `host` on the worker entry, which is also the destination peer id.
const DELEGATION_HOST: &str = "developer-remote-host";

/// Files the oracle reads, all relative to the gate's root so the proven-red suite
/// can point it at a fixture tree.
const TOPOLOGY: &str = "spirits/topologies/j1-founder-loop.toml";
const DELEGATION_RS: &str = "crates/maos-bin/src/delegation.rs";
const MAILBOX_RS: &str = "crates/maos-iac/src/adapter/mailbox.rs";
const MAIN_RS: &str = "crates/maos-bin/src/main.rs";
const ORCHESTRATOR_RS: &str = "spirits/orchestrator/src/lib.rs";
const A2A_ROUTER_RS: &str = "crates/maos-a2a-core/src/router.rs";
/// j1-crosshost-2a AC1.7 — the completion-oracle seam and the library boundary
/// that makes its tests executable at all.
const WORKER_CLI_RS: &str = "crates/maos-bin/src/worker_cli.rs";
const BIN_LIB_RS: &str = "crates/maos-bin/src/lib.rs";
/// j1-crosshost-2a AC1.7(iii) — the workflow itself is GOVERNED now.
///
/// Without this the gate had no eyes on the file its own enrollment lives in, so
/// deleting a `--test` line left the gate green and AC1.9's falsifier was a
/// falsifier standing in for a falsifier. Two in-repo idioms already read this
/// file from a gate (`check_loom_substrate_drift`'s `WORKFLOW` const and
/// `check_epic_6_bridge`'s `discipline_yml_has_step`); this is the third.
const WORKFLOW: &str = ".github/workflows/discipline.yml";

/// The DIRECTORY the enrolled `cargo test` set is DERIVED from, and the filename
/// suffixes that mark a target as J1-lane.
///
/// `2a` shipped this leg against a hand-maintained `ENROLLED_TEST_TARGETS` const.
/// `1b` replaced the const with this derivation, because a hand-maintained list is
/// `smoke_cli_wrapper_8_12`'s failure re-created inside the gate built to prevent
/// it: add a J1 test file, forget the const line, and the file is dead in CI while
/// the gate stays green. Derived from the filesystem, an un-enrolled J1 test reds
/// the gate BY CONSTRUCTION rather than by remembering. No CI job runs
/// `-p maos-bin` unscoped — every invocation names explicit `--test` targets — so
/// 24 of the 30 files in this directory are invoked by no job at all.
const MAOS_BIN_TESTS_DIR: &str = "crates/maos-bin/tests";
const J1_TEST_SUFFIXES: &[&str] = &["_1a.rs", "_1b.rs", "_2a.rs"];

/// j1-crosshost-1b AC1 — the refusal proofs themselves, the gate's TENTH governed
/// file. The gate can only check that the assertions are WRITTEN; only the CI
/// enrollment above proves they RUN.
const CONSENT_REFUSAL_RS: &str = "crates/maos-bin/tests/consent_refusal_1b.rs";

/// The shared oracle both real adapters used to delegate to. Its whole contract
/// was "clean exit + a non-empty final stdout line", which scored a live refusal
/// that exited 0 as `completed: true`. It is DELETED; this gate exists so it
/// cannot come back, in that name or by that shape.
const RETIRED_SHARED_ORACLE: &str = "final_stdout_message_oracle";
/// The per-adapter oracles that replaced it, each derived from its own CLI's
/// machine-readable contract.
const CODEX_ORACLE: &str = "codex_jsonl_oracle";
const CLAUDE_ORACLE: &str = "claude_result_object_oracle";

/// The leg names this gate publishes, so enrollment surfaces and `1b`'s additions
/// can be reconciled against one list rather than against prose.
pub fn ledger_leg_names() -> Vec<&'static str> {
    vec![
        "frame-borne-route-intact",
        "loopback-from-host-unverified",
        // j1-crosshost-2a AC1.7
        "completion-oracle-per-adapter",
        "worker-cli-under-library",
        "completion-vectors-enrolled",
        // j1-crosshost-1b AC2.1
        "consent-refusal-proofs",
    ]
}

/// One thing the oracle found wrong, attributed to the leg that found it.
#[derive(Debug)]
pub struct Finding {
    pub check: &'static str,
    pub detail: String,
}

/// Read a governed source file. A missing file is a FINDING, never a skip: a gate
/// that silently passes when its subject is absent is the null control this story
/// exists to stop.
fn read(
    root: &Path,
    rel: &str,
    findings: &mut Vec<Finding>,
    check: &'static str,
) -> Option<String> {
    match fs::read_to_string(root.join(rel)) {
        Ok(s) => Some(s),
        Err(e) => {
            findings.push(Finding {
                check,
                detail: format!("cannot read {rel}: {e}"),
            });
            None
        }
    }
}

/// Lines of `src` that are not comments — so an invariant cannot be satisfied by
/// prose describing it.
fn live_lines(src: &str) -> impl Iterator<Item = &str> {
    src.lines().filter(|l| {
        let t = l.trim_start();
        !(t.starts_with("//") || t.starts_with('#') || t.starts_with('*') || t.is_empty())
    })
}

fn contains_live(src: &str, needle: &str) -> bool {
    live_lines(src).any(|l| l.contains(needle))
}

fn production_before_tests(src: &str) -> &str {
    src.split_once("\n#[cfg(test)]")
        .map_or(src, |(production, _)| production)
}

/// Strip comment-only lines, then ALL whitespace, leaving a structural skeleton.
///
/// A `Blocking` gate a formatter can flip is a false-alarm machine. This one was:
/// its needle hard-coded the single-line spelling
/// `self.a2a_router.get().ok_or_else`, so when `cargo fmt` split that chain onto
/// separate lines (`mailbox.rs` was unformatted at commit `6827dc87`, and the
/// blocking `cargo fmt --all -- --check` gate at `discipline.yml:151` demands it
/// be formatted) the needle stopped matching a fail-closed branch that was still
/// there. Found 2026-08-14 by story `j1-demo-one-command-scene`, whose scene
/// calls this gate as its judge. Matching the skeleton makes the oracle depend on
/// code STRUCTURE, never on layout — `j1-crosshost-1b`'s refusal legs should use
/// the same normalization.
fn structural(src: &str) -> String {
    live_lines(src)
        .flat_map(str::chars)
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// Match the actual Phase-3 absent-router expression, not an error-name
/// occurrence elsewhere in the file. In particular, the `#[cfg(test)]` module
/// contains `CrossHostNotConfigured` assertions that must not keep this gate
/// green after the production branch is removed.
fn absent_router_branch_fails_closed(src: &str) -> bool {
    let flat = structural(production_before_tests(src));
    flat.contains(
        "letrouter=self.a2a_router.get().ok_or_else(||IacBusError::CrossHostNotConfigured{",
    ) || flat.contains(
        "letrouter=self.a2a_router.get().ok_or_else(||{IacBusError::CrossHostNotConfigured{",
    )
}

/// LEG 1 — the proven-red. Each condition is necessary for the delegation to be
/// frame-borne; breaking any one routes the task locally instead.
fn leg_frame_borne_route_intact(root: &Path, findings: &mut Vec<Finding>, audit: &mut LegAudit) {
    const CHECK: &'static str = "frame-borne-route-intact";
    audit.entered();

    // (1) The topology declares the delegation target. Without `host` the worker is
    //     loaded as an ordinary local member and no frame is ever emitted — the
    //     single likeliest form of "route locally anyway".
    if let Some(src) = read(root, TOPOLOGY, findings, CHECK) {
        audit.checked();
        let hosts = live_lines(&src)
            .filter(|l| l.trim_start().starts_with("host"))
            .count();
        if hosts != 1 || !contains_live(&src, DELEGATION_HOST) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{TOPOLOGY} must declare exactly ONE `host = \"{DELEGATION_HOST}\"` entry \
                     (found {hosts} `host` keys); without it the Worker is loaded locally and no \
                     task.assign frame is ever emitted"
                ),
            });
        }
    }

    // (2) The emit uses the REMOTE builder. `assign_frame` hardcodes
    //     `host_id: None` + `consent_envelope: None`, so emitting through it routes
    //     the frame same-host with no consent — locally, silently.
    if let Some(src) = read(root, MAIN_RS, findings, CHECK) {
        audit.checked();
        if !contains_live(&src, "assign_frame_remote") {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{MAIN_RS} must emit the delegation via `assign_frame_remote`; `assign_frame` \
                     hardcodes host_id: None and consent_envelope: None and would deliver locally"
                ),
            });
        }
        // (3) The env shortcut stays deleted. A reintroduced read makes the frame
        //     decorative: the Worker would take its task from the environment again
        //     while the frame still routed, so every other leg would stay green.
        audit.checked();
        if contains_live(&src, "MAOS_WORKER_TASK") {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{MAIN_RS} reads MAOS_WORKER_TASK again — the delegation frame becomes \
                     decorative while every routing assertion stays green"
                ),
            });
        }
    }

    // (4) The pump strips `to[..].host_id` before local re-delivery. Without the
    //     strip the frame re-enters the cross-host branch forever; with a strip that
    //     is applied BEFORE routing instead, it never leaves the host.
    if let Some(src) = read(root, DELEGATION_RS, findings, CHECK) {
        audit.checked();
        if !contains_live(&src, "host_id = None") {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{DELEGATION_RS} must strip `to[..].host_id` in the pump before local \
                     re-delivery, or the routed frame loops through the cross-host branch"
                ),
            });
        }
    }

    // (5) The absent-router path fails CLOSED. If Phase 3 ever falls through to
    //     local delivery, a host-bearing frame is delivered same-host with no peer
    //     consent check at all — "route locally anyway" in its purest form.
    if let Some(src) = read(root, MAILBOX_RS, findings, CHECK) {
        audit.checked();
        if !absent_router_branch_fails_closed(&src) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{MAILBOX_RS} must fail closed with CrossHostNotConfigured in the production \
                     A2A-router `ok_or_else` branch; a local-delivery fallback routes cross-host \
                     frames same-host"
                ),
            });
        }
    }

    // (6) The consent intent names effect authority and `task.assign` is never used
    //     as one (its `.` is non-canonical and fails closed at the sender, so the
    //     frame would never route at all).
    if let Some(src) = read(root, ORCHESTRATOR_RS, findings, CHECK) {
        audit.checked();
        if !contains_live(&src, DELEGATION_INTENT) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{ORCHESTRATOR_RS} must define the delegation consent intent as \
                     `{DELEGATION_INTENT}` (ADR-012 names effect authority, not a job category)"
                ),
            });
        }
        // A line that BUILDS the intent and CHECKS canonicality in the same breath
        // is the pin-test that keeps the trap pinned (`task.assign` must be
        // non-canonical), not a use of it. The regression this catches is a
        // production consent intent built from a dotted name, which fails closed at
        // `prepare_outbound` and never routes — a wire that looks broken rather than
        // refused.
        audit.checked();
        let dotted_consent_use = live_lines(production_before_tests(&src)).any(|l| {
            l.contains("\"task.assign\"")
                && l.contains("A2AIntent::new")
                && !l.contains("is_canonical")
        });
        if dotted_consent_use {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{ORCHESTRATOR_RS} builds a consent A2AIntent from `task.assign` — the `.` is \
                     non-canonical, so the frame fails closed at prepare_outbound and never routes"
                ),
            });
        }
    }
}

/// LEG 2 — Vex's boundary leg. NOT a failure: a recorded gap with a defined flip.
///
/// Returns `true` while the J1 delegation path does not bind wire identity.
///
/// ## REPAIRED by `j1-crosshost-1b` AC2.2a — it was a null control
///
/// Until `1b` this leg computed
/// `contains("frame.from.host_id") && contains("pub async fn handle_intake_verified")`
/// over `crates/maos-a2a-core/src/router.rs`, and published `true` **forever, in
/// every possible future**. Two independent reasons, both measured:
///
/// * **Both needles are permanent features of that file.** The loopback path will
///   always need the self-asserted resolution, and the verified entry point already
///   exists for the TCP path. Worse, `contains_live` filters comment-prefixed lines
///   but NOT string literals, and `router.rs:1514` is the `format!` literal
///   `"frame.from.host_id {} does not match TLS-verified peer {}"` — inside
///   `handle_intake_verified`'s OWN TLS-mismatch NACK. Deleting the loopback
///   resolution entirely would have left the needle green, pinned by the code that
///   proves verification IS enforced.
/// * **The leg could not observe its own declared trigger.** "Rung 2 turns
///   verification on" is a change in WHICH router entry the composition root calls
///   — in `maos-bin` and `maos-a2a-tcp` — not a text change in `router.rs`, the
///   only file the leg read.
///
/// So the leg now reads the **composition root of the J1 delegation**. While it
/// builds its router through `maos_a2a::pairing::paired_loopback_router`, the
/// transport is [`LoopbackA2ARouter`], which calls `handle_intake` **directly**
/// (`crates/maos-a2a/src/adapter.rs:82`, `:97`) — so there is no TLS-verified peer
/// to bind `frame.from.host_id` to, and the frame chooses which `accept_allowlist`
/// judges it. When `j1-crosshost-2b` composes a verified transport here instead,
/// this leg genuinely flips, and `xtask/tests/j1_crosshost_1b_proven_red.rs` plants
/// that flipped state as a vector.
///
/// `router.rs` stays governed as the SECOND door the flip can arrive through: if
/// the shared intake body ever binds identity itself, the J1 path becomes verified
/// without the composition root changing a line. That term needles the peer
/// RESOLUTION EXPRESSION, never the bare `frame.from.host_id` token — which is what
/// the old leg did, and why `handle_intake_verified`'s own NACK message pinned it
/// green.
fn leg_loopback_from_host_unverified(
    root: &Path,
    findings: &mut Vec<Finding>,
    audit: &mut LegAudit,
) -> bool {
    const CHECK: &'static str = "loopback-from-host-unverified";
    audit.entered();
    let Some(src) = read(root, DELEGATION_RS, findings, CHECK) else {
        return false;
    };
    audit.checked();
    let flat = structural(production_before_tests(&src));
    let loopback_composed = flat.contains("paired_loopback_router(");
    // Rung 2's flip, named by both of the shapes it can arrive in: a verified
    // intake entry point, or the live TCP transport crate at the composition
    // root. §A6 review P6: gated on loopback NOT being composed — a
    // preparatory `use maos_a2a_tcp` beside a still-loopback router must not
    // flip the boundary; only replacing the loopback construction does.
    let verified_composed = !loopback_composed
        && (flat.contains("handle_intake_verified") || flat.contains("maos_a2a_tcp"));

    // Door two: the shared intake body resolves the peer from the frame's OWN
    // `from.host_id`. `contains_live` would match `handle_intake_verified`'s
    // TLS-mismatch message literal, so this needles the `match` expression.
    let Some(router_src) = read(root, A2A_ROUTER_RS, findings, CHECK) else {
        return false;
    };
    audit.checked();
    // §A6 review P2: PRODUCTION half only — `router.rs` carries a `#[cfg(test)]`
    // module (at `:1786`), and a copy of the resolution relocated there must not
    // keep this leg publishing the old boundary.
    let self_asserted_resolution = structural(production_before_tests(&router_src))
        .contains("letpeer_host=match&frame.from.host_id{");

    let unverified = loopback_composed && self_asserted_resolution && !verified_composed;
    if !unverified {
        // The boundary CHANGED. That is not a violation, but it must not pass
        // silently: rung 2 turning verification on is exactly the event this leg
        // exists to surface in a CI diff.
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "the J1 wire-identity boundary MOVED (loopback_composed={loopback_composed}, \
                 self_asserted_resolution={self_asserted_resolution}, \
                 verified_composed={verified_composed}) across {DELEGATION_RS} + \
                 {A2A_ROUTER_RS}. If rung 2 composed a verified transport, update this leg, the \
                 AC1.5(a) non-coverage statement in j1-crosshost-2b, and the story records — do \
                 not delete the leg"
            ),
        });
    }
    unverified
}

/// LEG 3 — j1-crosshost-2a AC1.7(i). Neither real adapter may decide completion
/// from "clean exit + a non-empty final stdout line".
///
/// That shared oracle is the whole defect: a live `claude -p` refused a write,
/// printed a fluent explanation, exited 0, and was scored `completed: true` — and
/// that verdict is the admission condition for signing. Each adapter must instead
/// consume its OWN machine-readable contract (codex's `--json` `ThreadEvent`
/// stream, claude's `--output-format json` result object), so the leg asserts both
/// the absence of the retired oracle and the presence of the two that replaced it.
///
/// Source-STRUCTURAL and root-relative, over the production half only: a
/// `#[cfg(test)]` vector that mentions the retired name must not red the gate, and
/// a `cargo`-invoking leg would inherit the proven-red tempdir (which has no
/// `Cargo.toml`) and vacuum every planted vector while CI reported green.
fn leg_completion_oracle_per_adapter(
    root: &Path,
    findings: &mut Vec<Finding>,
    audit: &mut LegAudit,
) {
    const CHECK: &'static str = "completion-oracle-per-adapter";
    audit.entered();
    let Some(src) = read(root, WORKER_CLI_RS, findings, CHECK) else {
        return;
    };
    let flat = structural(production_before_tests(&src));
    audit.checked();
    if flat.contains(RETIRED_SHARED_ORACLE) {
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "{WORKER_CLI_RS} still carries `{RETIRED_SHARED_ORACLE}` in production code. \
                 That oracle is \"clean exit + non-empty final stdout line\": it certified a \
                 live refusal that exited 0 as a completion. Each real adapter must consume \
                 its own structured output instead"
            ),
        });
    }
    // Review 2a-P8 — needle on the CALL FORM, not the function name: the green
    // fixture of the proven-red suite passes with EMPTY oracle stubs, so a
    // name-presence check stayed green while `parse_completion` delegated to
    // anything. The call form is the wiring; a named-but-unwired helper no
    // longer satisfies the leg.
    for (adapter, oracle_call) in [
        ("CodexCli", "codex_jsonl_oracle(stdout,exit)"),
        ("ClaudeCli", "claude_result_object_oracle(stdout,exit)"),
    ] {
        audit.checked();
        if !flat.contains(oracle_call) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{WORKER_CLI_RS} does not CALL `{oracle_call}` — {adapter}'s \
                     parse_completion must be WIRED to its own structured-output oracle. A \
                     named-but-unwired helper keeps the name while the verdict comes from \
                     somewhere else"
                ),
            });
        }
    }
    // The seam that makes the structured oracles honest: an adapter whose oracle
    // assumes a flag must be able to DEMAND it, or a manifest shipping prose turns
    // a real success into a false negative — F4's inversion.
    audit.checked();
    if !flat.contains("fnrequired_argv_flags") {
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "{WORKER_CLI_RS} has no `required_argv_flags` — without it an adapter cannot \
                 demand the structured-output flag its oracle parses, and a manifest that \
                 omits it converts a REAL success into a non-completion"
            ),
        });
    }
    // The clean-home invariant must not be codex-only again: `ClaudeCli` asserting
    // `None` here was a false claim with a green test behind it, and it made
    // `refuse_ambient_auth` a no-op for claude.
    audit.checked();
    if !flat.contains(".claude\").join(\".credentials.json") {
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "{WORKER_CLI_RS} does not name claude's ambient credential file \
                 (`~/.claude/.credentials.json`) — `refuse_ambient_auth` is then a NO-OP for \
                 claude and a signed claude run would stamp an unattestable redaction claim"
            ),
        });
    }
}

/// LEG 4 — j1-crosshost-2a AC1.7(ii). `worker_cli` must stay under the library.
///
/// This is the regression that would silently re-orphan every relocated test: the
/// module was `mod worker_cli;` in `main.rs`, so nothing under
/// `crates/maos-bin/tests/` could name `ClaudeCli`, `CodexCli` or
/// `parse_completion`, and its 204-line in-`src` test module was both charged to
/// the crate's KLOC budget and executed by no CI job. Move it back and the vectors
/// do not fail — they cease to exist.
fn leg_worker_cli_under_library(root: &Path, findings: &mut Vec<Finding>, audit: &mut LegAudit) {
    const CHECK: &'static str = "worker-cli-under-library";
    audit.entered();
    if let Some(src) = read(root, BIN_LIB_RS, findings, CHECK) {
        audit.checked();
        if !contains_live(&src, "pub mod worker_cli") {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{BIN_LIB_RS} does not export `pub mod worker_cli` — every completion-oracle \
                     vector under crates/maos-bin/tests/ is then unable to NAME the adapters, so \
                     the controls do not fail, they vanish"
                ),
            });
        }
    }
    if let Some(src) = read(root, MAIN_RS, findings, CHECK) {
        audit.checked();
        // `main.rs` must CONSUME the library module, never re-declare it: a
        // `mod worker_cli;` here compiles a SECOND copy that the tests cannot see.
        if contains_live(&src, "mod worker_cli;")
            && !contains_live(&src, "use maos_bin::worker_cli")
        {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{MAIN_RS} re-declares `mod worker_cli;` instead of consuming \
                     `maos_bin::worker_cli` — that compiles a second, test-invisible copy of the \
                     adapter seam"
                ),
            });
        }
    }
    // The in-`src` test module must stay gone: it is budget-charged and CI-invisible.
    if let Some(src) = read(root, WORKER_CLI_RS, findings, CHECK) {
        audit.checked();
        if src.contains("\n#[cfg(test)]") {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{WORKER_CLI_RS} has an in-`src` `#[cfg(test)]` module again. It is charged \
                     to maos-bin's KLOC ceiling and executed by NO CI job (every invocation is \
                     `--test <name>`); the vectors belong in crates/maos-bin/tests/"
                ),
            });
        }
    }
}

/// The J1 `cargo test` targets DERIVED from `crates/maos-bin/tests/`, sorted.
///
/// A missing directory is a FINDING, never a skip — same rule as [`read`]: a gate
/// that silently passes when its subject is absent is the null control this lane
/// keeps catching. An EMPTY derived set is not a finding here; it is caught by the
/// vacuity guard, because a leg whose input set is empty checks nothing.
fn derive_enrolled_targets(
    root: &Path,
    findings: &mut Vec<Finding>,
    check: &'static str,
) -> Vec<String> {
    let dir = root.join(MAOS_BIN_TESTS_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            findings.push(Finding {
                check,
                detail: format!(
                    "cannot read {MAOS_BIN_TESTS_DIR}: {e} — the enrolled `cargo test` set is \
                     DERIVED from this directory, so an unreadable one means the gate cannot \
                     tell whether any J1 vector runs in CI"
                ),
            });
            return Vec::new();
        }
    };
    let mut targets: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| J1_TEST_SUFFIXES.iter().any(|suffix| name.ends_with(suffix)))
        .filter_map(|name| name.strip_suffix(".rs").map(str::to_string))
        .collect();
    targets.sort();
    targets
}

/// LEG 5 — j1-crosshost-2a AC1.7(iii) / AC1.9, with `1b` AC2.5(b)'s derivation.
///
/// Without eyes on the workflow, this gate could not see whether its own vectors
/// were ever RUN. Deleting a `--test` line left the gate green, which made the
/// AC1.9 falsifier vacuous — a falsifier standing in for a falsifier. The read is
/// static (a file, not a `cargo` invocation), so it stays safe inside the
/// proven-red tempdir.
///
/// `1b` replaced `2a`'s hand-maintained `ENROLLED_TEST_TARGETS` const with a
/// derivation over `crates/maos-bin/tests/`. The const was one forgotten line away
/// from a dead test behind a green gate — `smoke_cli_wrapper_8_12`'s exact failure
/// re-created inside the gate built to prevent it. Derived, an un-enrolled J1 test
/// file reds this gate BY CONSTRUCTION. `2a`'s job-scoping is unchanged; only the
/// source of the list moved.
fn leg_completion_vectors_enrolled(root: &Path, findings: &mut Vec<Finding>, audit: &mut LegAudit) {
    const CHECK: &'static str = "completion-vectors-enrolled";
    audit.entered();
    let Some(src) = read(root, WORKFLOW, findings, CHECK) else {
        return;
    };
    // Review 2a-P8 — SCOPE to this gate's own job block. AC1.7(iii)/AC1.8
    // require the `check-j1-loopback-delegation` JOB (BindingClass::Blocking,
    // gate-registry-enrolled, a `needs` of the aggregate) to carry the vectors;
    // a workflow-wide scan stayed green when the lines merely existed SOMEWHERE,
    // including a non-blocking job. The job block ends at the next top-level
    // `  <name>:` key (2-space indent, GitHub Actions job syntax).
    let Some((_, job_rest)) = src.split_once("check-j1-loopback-delegation:") else {
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "{WORKFLOW} declares no `check-j1-loopback-delegation` job — the gate's own \
                 enrollment home is gone, so its vectors run nowhere"
            ),
        });
        return;
    };
    let job_block: String = job_rest
        .lines()
        // Stop at the next top-level job key: exactly 2-space indented
        // (`runs-on:`/`steps:` and deeper lines carry 4+).
        .take_while(|l| !(l.starts_with("  ") && !l.starts_with("   ")))
        .collect::<Vec<_>>()
        .join("\n");
    // §A6 review P4 — only EXECUTABLE cargo commands count. A `--test` token in
    // a step `name:` or an `echo` inside the job block is enrollment in prose:
    // the token survives while the behavioural test stops running. Collect the
    // `run:` lines (single-line and `run: |` block bodies — a continuation is
    // any line indented deeper than its opening `run:`), then match each target
    // at TOKEN boundaries so `--test foo_1b_extra` cannot satisfy `foo_1b`.
    let mut run_lines: Vec<String> = Vec::new();
    let mut open_run: Option<usize> = None;
    for line in job_block.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(run_indent) = open_run {
            if indent > run_indent {
                run_lines.push(structural(line));
                continue;
            }
            open_run = None;
        }
        if trimmed.starts_with("run:") || trimmed.starts_with("- run:") {
            open_run = Some(indent);
            run_lines.push(structural(line));
        }
    }
    let token_bounded = |flat: &str, needle: &str| {
        flat.find(needle)
            .map(|at| {
                let after = &flat[at + needle.len()..];
                after.is_empty()
                    || !(after.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
            })
            .unwrap_or(false)
    };
    for target in derive_enrolled_targets(root, findings, CHECK) {
        audit.checked();
        let needle = format!("--test{target}");
        if !run_lines.iter().any(|flat| token_bounded(flat, &needle)) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{WORKFLOW}'s `check-j1-loopback-delegation` JOB does not invoke `cargo \
                     test -p maos-bin --test {target}` (an occurrence in a DIFFERENT job does \
                     not count — only this job is Blocking, in gate-registry.toml, and a needs \
                     of the aggregate). An un-enrolled test target is a suggestion, not a \
                     control: no CI job runs `-p maos-bin` unscoped, so the vectors would \
                     never execute. The set is DERIVED from {MAOS_BIN_TESTS_DIR} — adding a J1 \
                     test file without enrolling it reds this gate by construction",
                ),
            });
        }
    }
}

/// LEG 6 — j1-crosshost-1b AC2.1. The ADR-012 refusal proofs exist and are
/// correctly SHAPED.
///
/// This gate is a static source-text oracle: it has never observed a frame being
/// refused and cannot. What it CAN do is refuse to let the assertions be deleted or
/// weakened while the gate stays green — deleting any needle below reds a
/// `Blocking` oracle. Whether the file RUNS is a different mechanism in a different
/// file, which is [`leg_completion_vectors_enrolled`]'s job; the two together are
/// the control.
///
/// Uses the composed idiom `structural(production_before_tests(src))` for every
/// multi-token needle, never bare `contains_live` — layout-sensitive needles are
/// what made `cargo fmt` and this gate mutually exclusive at `246660f9`. One honest
/// caveat: `production_before_tests` splits at `"\n#[cfg(test)]"`, which an
/// integration test under `crates/*/tests/` never contains, so on THIS file the
/// call is a no-op. It is used anyway for uniformity — but what protects this file
/// from a needle satisfied inside a test module is that the whole file IS the test,
/// and `structural` still strips the doc comments, so the module docs cannot
/// satisfy a single needle below.
///
/// ## Known limit (accepted by §A6 round-table 2026-08-16, 8/8 — per spec +
/// long-term correctness)
///
/// `structural()` strips comment-PREFIXED lines; it does not strip string
/// literals or inline `/* … */` blocks, and it cannot: needles like
/// `assert_eq!(TO_HOST,"developer-remote-host"` embed literals themselves, so
/// quote-stripping the skeleton would stop the gate matching its own mandated
/// assertions, and a heuristic filter is the `246660f9` false-alarm class. A
/// bait `const BAIT: &str = "assert_eq!(…)"` therefore satisfies a needle
/// while the assertion no longer executes. Every ACCIDENTAL path — deletion,
/// weakening, `//`-commenting, reformatting, relocation into a test module,
/// de-registration — reds this gate; deliberate decoys in committed,
/// review-visible code do not. The durable fix (token-aware needles) is
/// **14-6's** instrument work; this gate is its first customer.
fn leg_consent_refusal_proofs(root: &Path, findings: &mut Vec<Finding>, audit: &mut LegAudit) {
    const CHECK: &'static str = "consent-refusal-proofs";
    audit.entered();
    let Some(src) = read(root, CONSENT_REFUSAL_RS, findings, CHECK) else {
        return;
    };
    let flat = structural(production_before_tests(&src));

    // Each needle IS an assertion skeleton — which is why no paraphrase column is
    // carried: the needle names the assertion more precisely than a description of
    // it could, and what each one proves is documented above, in comments, which
    // cost no budget. Order follows AC1.1 → AC1.4.
    //
    // Whitespace- and comment-insensitive by construction, so a needle that appears
    // only in prose does not count — and NO needle carries a closing delimiter,
    // because `cargo fmt` adds a trailing comma when it breaks a pattern or a macro
    // call across lines: `{peer,message}` would stop matching a `{peer,message,}`
    // that is still there. That is the `246660f9` false-alarm class, and this gate's
    // own reformat vector caught it a second time while this leg was being written.
    const REQUIRED: &[&str] = &[
        // AC1.1 — the LOCAL positive control still exists and still asserts the
        // delivered intent (§A6 review P3: without needles here the whole
        // positive test was deletable behind a green gate, leaving negatives
        // that can pass vacuously). NOTE: `live_lines` strips `#`-prefixed
        // lines, so an attribute cannot appear in a needle — REGISTRATION is
        // the count check below, existence is the fn identity.
        "asyncfnallowlisted_delegation_intent_is_admitted",
        "Some(DELEGATION_CONSENT_INTENT.to_string())",
        // AC1.2 — the -32001 typed peer deny, the NACK naming the SOURCE host,
        // both peer_id strings pinned literally, and the deny naming the
        // DESTINATION peer (P3: the destructure needle alone accepted a
        // `-32001` that identified the wrong host).
        "A2AError::IntentDeniedAtPeer{peer,message",
        "message.contains(FROM_HOST)",
        "assert_eq!(peer,TO_HOST,",
        "assert_eq!(FROM_HOST,\"founder-loop-host\"",
        "assert_eq!(TO_HOST,\"developer-remote-host\"",
        // AC1.3 — -32009 at BOTH seams, with the reason read back TYPED and
        // BOUND to the expectation (P3: the helper needle alone let the
        // comparison be deleted, leaving a numeric-only deny), over every
        // reachable `UnclassifiedReason`.
        "A2AError::ConsentUnclassified{direction:IntentDirection::Send,",
        "assert_eq!(nack.error.code,CODE_CONSENT_UNCLASSIFIED",
        "assert_eq!(nack_reason(&nack.error),expected,",
        "serde_json::from_value::<UnclassifiedReason>(",
        "UnclassifiedReason::Absent",
        "UnclassifiedReason::NonCanonical",
        "UnclassifiedReason::Oversized",
        // AC1.4 — non-conflation in BOTH directions, and -32003 kept distinct.
        "assert_ne!(nack.error.code,CODE_CONSENT_UNCLASSIFIED",
        "assert_ne!(nack.error.code,CODE_INTENT_DENIED",
        "assert_eq!(nack.error.code,CODE_CONSENT_EXPIRED",
    ];
    /// §A6 review P1 — the assertions above are satisfied by text in
    /// UNANNOTATED functions too (the proven-red fixture's own shapes prove
    /// it). Only a registration count makes "the file still contains seven
    /// executing tests" observable to a static oracle.
    const REQUIRED_TESTS: usize = 7;

    for needle in REQUIRED {
        audit.checked();
        if !flat.contains(needle) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{CONSENT_REFUSAL_RS} no longer asserts `{needle}`. Deleting or weakening an \
                     ADR-012 refusal assertion while this gate stays green is the vacuity it \
                     exists to close"
                ),
            });
        }
    }

    // §A6 review P1 — the proofs must remain REGISTERED tests. Needles alone
    // cannot tell an executing test from an assertion-shaped dead function:
    // delete the seven `#[tokio::test]` attributes and `cargo test --test
    // consent_refusal_1b` runs ZERO tests, exits 0, and the gate + CI stay
    // green — `smoke_cli_wrapper_8_12`'s death at a new address.
    let registered = src.lines().filter(|l| l.trim() == "#[tokio::test]").count();
    audit.checked();
    if registered < REQUIRED_TESTS {
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "{CONSENT_REFUSAL_RS} carries {registered} `#[tokio::test]` functions, \
                 expected at least {REQUIRED_TESTS} — the assertion shapes exist but \
                 nothing executes them"
            ),
        });
    }
}

/// What one run of the oracle OBSERVED, per leg. Exposed so a caller can report a
/// red consent leg under its OWN name instead of collapsing every leg into one
/// boolean — which is what `demo_j1` did until `1b` AC2.10, so after `2a` a red
/// completion-oracle leg printed `FAIL frame-borne-route-intact`.
pub struct Judgement {
    pub findings: Vec<Finding>,
    pub audits: Vec<LegAudit>,
    pub loopback_from_host_unverified: bool,
}

impl Judgement {
    /// Whether one named leg ran real checks and pushed no finding. A leg with no
    /// audit is `None` — UNKNOWN, never green.
    pub fn leg_green(&self, leg: &str) -> Option<bool> {
        let audit = self.audits.iter().find(|a| a.leg() == leg)?;
        Some(!audit.is_vacuous() && self.findings.iter().all(|f| f.check != leg))
    }

    /// The leg's observed particulars, for a narrated claim table.
    pub fn leg_detail(&self, leg: &str) -> Option<String> {
        let audit = self.audits.iter().find(|a| a.leg() == leg)?;
        let found: Vec<&str> = self
            .findings
            .iter()
            .filter(|f| f.check == leg)
            .map(|f| f.detail.as_str())
            .collect();
        Some(match found.is_empty() {
            true => format!("{} check(s) evaluated, no findings", audit.checks()),
            false => format!(
                "{} check(s); {} finding(s): {}",
                audit.checks(),
                found.len(),
                found.join(" · ")
            ),
        })
    }
}

/// Run every leg against `root` and return the raw observation.
pub fn judge(root: &Path) -> Judgement {
    let root: PathBuf = root.to_path_buf();
    let mut findings: Vec<Finding> = Vec::new();
    // One audit per PUBLISHED leg name, so a leg listed and never invoked reports
    // `ran: false` instead of vanishing. The slice pattern is the other half of the
    // reconciliation: add a leg to `ledger_leg_names()` without wiring it here (or
    // vice versa) and this stops compiling.
    let mut audits: Vec<LegAudit> = ledger_leg_names().into_iter().map(LegAudit::new).collect();
    let [leg1, leg2, leg3, leg4, leg5, leg6] = &mut audits[..] else {
        panic!("ledger_leg_names() must publish exactly the six legs judge() invokes");
    };

    leg_frame_borne_route_intact(&root, &mut findings, leg1);
    let loopback_from_host_unverified =
        leg_loopback_from_host_unverified(&root, &mut findings, leg2);
    leg_completion_oracle_per_adapter(&root, &mut findings, leg3);
    leg_worker_cli_under_library(&root, &mut findings, leg4);
    leg_completion_vectors_enrolled(&root, &mut findings, leg5);
    leg_consent_refusal_proofs(&root, &mut findings, leg6);

    // The vacuous-green guard (AC2.2), consuming the SHARED primitive. A leg that
    // read nothing and pushed nothing is indistinguishable from a leg that passed
    // under `oracle_green = findings.is_empty()` — the one condition that
    // aggregation is blind to — so it becomes a finding of its own.
    for leg in vacuous_legs(&audits) {
        findings.push(Finding {
            check: "leg-vacuity",
            detail: format!(
                "leg `{leg}` reported no executed check. A leg that reads nothing is \
                 indistinguishable from a leg that passed, so this gate treats it as RED — fix \
                 the leg or its input, never the guard"
            ),
        });
    }

    Judgement {
        findings,
        audits,
        loopback_from_host_unverified,
    }
}

pub fn run(json: bool) -> Result<(), String> {
    run_with_root(json, Path::new("."))
}

pub fn run_with_root(json: bool, root: &Path) -> Result<(), String> {
    let judgement = judge(root);
    let findings = &judgement.findings;
    let from_host_unverified = judgement.loopback_from_host_unverified;

    let oracle_green = findings.is_empty();
    // Hermetic: a RED oracle hard-fails at HEAD regardless of CURRENT_PHASE.
    let dev_blocks = dev_enforced_red_blocks(BindingClass::Blocking, true);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": "check-j1-loopback-delegation",
                "passed": oracle_green,
                "oracle_green": oracle_green,
                "binding": "Blocking",
                "legs": ledger_leg_names(),
                // Per-leg outcomes (AC2.10): a red consent leg must be reportable
                // under its OWN name, not under leg 1's.
                "leg_audits": judgement.audits,
                // §A6 review P11 — AC2.1's seventh site: the consent leg's
                // named boolean, derived (never annotated), for JSON callers.
                "consent_refusal_proofs": judgement
                    .leg_green("consent-refusal-proofs")
                    .unwrap_or(false),
                // Recorded as a BOUNDARY, not a failure (AC4.4). `true` means the
                // J1 delegation path still does not bind wire identity.
                "loopback_from_host_unverified": from_host_unverified,
                "delegation_intent": DELEGATION_INTENT,
                "findings": findings.iter().map(|f| serde_json::json!({
                    "check": f.check, "detail": f.detail,
                })).collect::<Vec<_>>(),
            })
        );
    } else if oracle_green {
        eprintln!(
            "check-j1-loopback-delegation: PASS — frame-borne route intact, consent refusals \
             proven and enrolled; boundary: J1 `frame.from.host_id` unverified = \
             {from_host_unverified} (rung-2 flips this)"
        );
    } else {
        eprintln!(
            "check-j1-loopback-delegation: BLOCKING — {} finding(s):",
            findings.len()
        );
        for f in findings {
            eprintln!("  [FAIL] {} — {}", f.check, f.detail);
        }
    }

    if oracle_green || !dev_blocks {
        Ok(())
    } else {
        Err(format!(
            "check-j1-loopback-delegation: {} finding(s) — the J1 delegation is not provably \
             frame-borne and refusing",
            findings.len()
        ))
    }
}
