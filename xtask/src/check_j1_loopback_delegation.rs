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
//!    On the loopback profile `frame.from.host_id` is self-asserted: the shared
//!    intake body resolves the peer from it and only `handle_intake_verified` binds
//!    it to a TLS-verified identity. When rung 2 turns verification on, this leg
//!    flips from "documented gap" to "now enforced" and the change is visible in a
//!    CI diff instead of buried in a story nobody re-reads.
//!
//! Binding class: [`crate::gate_common::BindingClass::Blocking`] — hermetic (reads
//! committed sources), so a violation reds CI at HEAD regardless of `CURRENT_PHASE`.
//!
//! `j1-crosshost-1b` adds the consent refusal legs to this gate. It does not stand
//! one up in a green field.

use crate::gate_common::{dev_enforced_red_blocks, BindingClass};
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

/// The `cargo test` targets that MUST be enrolled in this gate's own CI job. A
/// test file that is not named here and in the workflow is a suggestion: 24 of 28
/// `crates/maos-bin/tests/` targets are invoked by no job at all.
const ENROLLED_TEST_TARGETS: &[&str] = &["worker_completion_2a", "worker_manifests_2a"];

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
    ]
}

#[derive(Debug)]
struct Finding {
    check: &'static str,
    detail: String,
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
fn leg_frame_borne_route_intact(root: &Path, findings: &mut Vec<Finding>) {
    const CHECK: &'static str = "frame-borne-route-intact";

    // (1) The topology declares the delegation target. Without `host` the worker is
    //     loaded as an ordinary local member and no frame is ever emitted — the
    //     single likeliest form of "route locally anyway".
    if let Some(src) = read(root, TOPOLOGY, findings, CHECK) {
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
/// Returns `true` while wire identity is unbound on the loopback intake path.
fn leg_loopback_from_host_unverified(root: &Path, findings: &mut Vec<Finding>) -> bool {
    const CHECK: &'static str = "loopback-from-host-unverified";
    let Some(src) = read(root, A2A_ROUTER_RS, findings, CHECK) else {
        return false;
    };
    // `handle_intake` resolves the peer straight from the frame's self-asserted
    // `from.host_id`; only `handle_intake_verified` binds it to the TLS-verified
    // peer. While BOTH facts hold, the loopback profile does not verify wire
    // identity — the documented rung-1 boundary.
    let self_asserted = contains_live(&src, "frame.from.host_id");
    let verified_entry = contains_live(&src, "pub async fn handle_intake_verified");
    let unverified = self_asserted && verified_entry;
    if !unverified {
        // The boundary CHANGED. That is not a violation, but it must not pass
        // silently: rung 2 turning verification on is exactly the event this leg
        // exists to surface in a CI diff.
        findings.push(Finding {
            check: CHECK,
            detail: format!(
                "the loopback wire-identity boundary MOVED in {A2A_ROUTER_RS} \
                 (self_asserted={self_asserted}, verified_entry={verified_entry}). If rung 2 \
                 enabled verification, update this leg and the j1-crosshost story record — do not \
                 delete the leg"
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
fn leg_completion_oracle_per_adapter(root: &Path, findings: &mut Vec<Finding>) {
    const CHECK: &'static str = "completion-oracle-per-adapter";
    let Some(src) = read(root, WORKER_CLI_RS, findings, CHECK) else {
        return;
    };
    let flat = structural(production_before_tests(&src));
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
fn leg_worker_cli_under_library(root: &Path, findings: &mut Vec<Finding>) {
    const CHECK: &'static str = "worker-cli-under-library";
    if let Some(src) = read(root, BIN_LIB_RS, findings, CHECK) {
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

/// LEG 5 — j1-crosshost-2a AC1.7(iii) / AC1.9. The ENROLLMENT leg.
///
/// Without eyes on the workflow, this gate could not see whether its own vectors
/// were ever RUN. Deleting a `--test` line left the gate green, which made the
/// AC1.9 falsifier vacuous — a falsifier standing in for a falsifier. The read is
/// static (a file, not a `cargo` invocation), so it stays safe inside the
/// proven-red tempdir.
fn leg_completion_vectors_enrolled(root: &Path, findings: &mut Vec<Finding>) {
    const CHECK: &'static str = "completion-vectors-enrolled";
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
    let flat = structural(&job_block);
    for target in ENROLLED_TEST_TARGETS {
        if !flat.contains(&format!("--test{target}")) {
            findings.push(Finding {
                check: CHECK,
                detail: format!(
                    "{WORKFLOW}'s `check-j1-loopback-delegation` JOB does not invoke `cargo \
                     test -p maos-bin --test {target}` (an occurrence in a DIFFERENT job does \
                     not count — only this job is Blocking, in gate-registry.toml, and a needs \
                     of the aggregate). An un-enrolled test target is a suggestion, not a \
                     control: no CI job runs `-p maos-bin` unscoped, so the vectors would \
                     never execute",
                ),
            });
        }
    }
}

pub fn run(json: bool) -> Result<(), String> {
    run_with_root(json, Path::new("."))
}

pub fn run_with_root(json: bool, root: &Path) -> Result<(), String> {
    let root: PathBuf = root.to_path_buf();
    let mut findings: Vec<Finding> = Vec::new();

    leg_frame_borne_route_intact(&root, &mut findings);
    let from_host_unverified = leg_loopback_from_host_unverified(&root, &mut findings);
    leg_completion_oracle_per_adapter(&root, &mut findings);
    leg_worker_cli_under_library(&root, &mut findings);
    leg_completion_vectors_enrolled(&root, &mut findings);

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
                // Recorded as a BOUNDARY, not a failure (AC4.4). `true` means the
                // loopback profile still does not bind wire identity.
                "loopback_from_host_unverified": from_host_unverified,
                "delegation_intent": DELEGATION_INTENT,
                "findings": findings.iter().map(|f| serde_json::json!({
                    "check": f.check, "detail": f.detail,
                })).collect::<Vec<_>>(),
            })
        );
    } else if oracle_green {
        eprintln!(
            "check-j1-loopback-delegation: PASS — frame-borne route intact; \
             boundary: loopback `frame.from.host_id` unverified = {from_host_unverified} \
             (rung-2 flips this)"
        );
    } else {
        eprintln!(
            "check-j1-loopback-delegation: BLOCKING — {} finding(s):",
            findings.len()
        );
        for f in &findings {
            eprintln!("  [FAIL] {} — {}", f.check, f.detail);
        }
    }

    if oracle_green || !dev_blocks {
        Ok(())
    } else {
        Err(format!(
            "check-j1-loopback-delegation: {} finding(s) — the J1 delegation is not provably \
             frame-borne",
            findings.len()
        ))
    }
}
