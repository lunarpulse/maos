#![forbid(unsafe_code)]

//! Story 14-2a (AC6) — `check-cert-rotation-trigger` gate.
//!
//! # Why the obvious gate for this story is a null control
//!
//! The only "prove a production caller exists" primitive in the repo is
//! `FunctionCallProbe` (`xtask/src/check_reza_production_path.rs:40-56`), and it
//! implements **`visit_expr_call` only** — `syn::ExprCall` with an `Expr::Path`
//! head, i.e. FREE-FUNCTION calls. Measured at `a22f0c01`: there was not one
//! `visit_expr_method_call` implementation in any `xtask/src/check_*.rs` module.
//! Every call this story's deliverable makes is a METHOD call
//! (`state.install_cert_rotation(…)`, `transport.pins()`, `transport.core()`),
//! so that probe copied as-is would have been **green from birth and unable to
//! red** — which is worse than no gate, because it looks like proof.
//!
//! This gate therefore implements `visit_expr_method_call` (below), **and then
//! refuses to call that the proof**:
//!
//! * **THE CONTROL is the runtime leg** — a real `maos` child process in
//!   `MAOS_ONE_SHOT=cohort-a2a-daemon` mode receives a real signed
//!   `cohort:manifest-reissue` over real mTLS and the pin-set change is
//!   OBSERVED from outside the process through the production operator HTTP
//!   surface. Its proven-red vector is **unwiring the `OnceLock` install** in
//!   `crates/maos-bin/src/main.rs` — not deleting a line of text. Executed
//!   2026-08-30: with `rotation_state.install_cert_rotation(…)` removed, the
//!   daemon still boots, still binds, still ACKs the signed reissue, still
//!   advances its manifest version, and the leg FAILS on
//!   `{"open_windows":[]}`.
//! * **The AST probe is the cheap second opinion.** It catches the careless
//!   deletion in milliseconds. It stays green if the call is moved into a
//!   `#[cfg(test)]` block or into a function nothing reaches, so this module
//!   says out loud that it is a spelling check with a narrowed spelling: it
//!   skips `#[cfg(test)]` modules and requires the call to appear in the
//!   production composition root.
//!
//! # The legs (AC6.3, each independently red-able)
//!
//! 1. **rotation-trigger-production-caller** — the RUNTIME control: a live
//!    daemon's peer trust rotates from a signed manifest reissue
//!    (`ROTATION_WINDOWS_OBSERVED=<n>` is the derived witness — a silent skip
//!    or an early return emits no marker and the leg REDS), and the window
//!    that reissue opens is closed by the PRODUCTION grace install and
//!    journaled (`ROTATION_WINDOW_CLOSED_OBSERVED=1`) — so a no-op or
//!    early-firing timer at the composition root reds here, not only in the
//!    timer type's own isolated leg.
//! 2. **reload-atomicity** — the AC2.2 invariant
//!    (`peers[p].cert_fingerprint ∈ {pins[p].fingerprint, rotation_next[p]}`)
//!    under a real reissue, plus the two-plane primitives and their rollback
//!    ordering.
//! 3. **rotation-audit-round-trip** — the rotation timeline queried back out of
//!    a real daemon's own Transparency Log under the stable
//!    `a2a:cert-rotation` intent (`ROTATION_AUDIT_ROWS=<n>`).
//! 4. **post-grace-reject-journaled** — §7.2.1.a's `cert_post_grace_reject`,
//!    the one failure mode this whole mechanism exists to produce, with its own
//!    non-vacuous control (a healthy handshake journals nothing).
//!
//! # The controls at the gate boundary
//!
//! * **Exact test counts** — every invocation names one exact test and passes
//!   libtest's `--exact`, so a verdict is bound to that test rather than to
//!   substring-filter coincidence.
//! * **Derived enrollment** — every `#[test]`/`#[tokio::test]` function in the
//!   story's rotation test files (five at HEAD) is DERIVED from the filesystem
//!   by ATTRIBUTE — not by name prefix, which derived only the prefix-named
//!   files and silently ignored the cohort and a2a-core files — and every
//!   derived test must be named by a leg. Each file must independently derive
//!   at least one test; an empty per-file derivation is an ERROR, because
//!   that is how a filesystem-derived enrollment goes decorative.
//! * **Vacuous-leg guard** — a leg that was attempted but ran zero tests
//!   hard-fails at every phase.
//! * **`BindingClass::Blocking`** — a RED oracle hard-fails at HEAD regardless
//!   of `CURRENT_PHASE`.

use crate::gate_common::{
    dev_enforced_red_blocks, emit_command, is_blocking_at, read_disposition, BindingClass,
    CURRENT_PHASE,
};
use std::process::{Command, Stdio};
use syn::visit::Visit;

const GATE_NAME: &str = "check-cert-rotation-trigger";

/// The production composition root the runtime trigger must be installed from.
const COMPOSITION_ROOT: &str = "crates/maos-bin/src/main.rs";
/// The method call that installs the set-once rotation control.
const INSTALL_METHOD: &str = "install_cert_rotation";
/// The two live-mesh handles the install MUST pass, or it installs a copy.
const PLANE_METHODS: [&str; 2] = ["pins", "core"];
/// Derived-enrollment scope: the story's rotation test files (five at HEAD)
/// and their packages.
const TEST_FILES: [(&str, &str, &str); 5] = [
    (
        "maos-bin",
        "cert_rotation_trigger_14_2a",
        "crates/maos-bin/tests",
    ),
    (
        "maos-cohort",
        "cert_rotation_14_2a",
        "crates/maos-cohort/tests",
    ),
    (
        "maos-a2a-core",
        "t_14_2a_rotation_reload",
        "crates/maos-a2a-core/tests",
    ),
    (
        "maos-a2a-tcp",
        "t_14_2a_post_grace_journal",
        "crates/maos-a2a-tcp/tests",
    ),
    (
        "maos-a2a-tcp",
        "t_14_2a_connection_panic_supervision",
        "crates/maos-a2a-tcp/tests",
    ),
];

/// One oracle invocation: its package, test binary, exact test name, the number
/// of tests the filter must match, the feature set it needs, and the derived
/// measurement marker its test must print.
struct Invocation {
    package: &'static str,
    test_file: &'static str,
    filter: &'static str,
    features: Option<&'static str>,
    expected_tests: u32,
    marker: Option<&'static str>,
}

struct LegResult {
    label: &'static str,
    passed: u32,
    failed: u32,
    ran: bool,
    attempted: bool,
    green: bool,
}

/// A method-call probe. Unlike `FunctionCallProbe` this visits
/// `syn::ExprMethodCall`, which is the only shape a call on an `Arc`-held
/// handle can have — and it SKIPS `#[cfg(test)]` modules, so an install that
/// exists only under test does not satisfy it.
struct MethodCallProbe<'a> {
    name: &'a str,
    found: bool,
}

impl<'ast> Visit<'ast> for MethodCallProbe<'_> {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        self.found |= call.method == self.name;
        syn::visit::visit_expr_method_call(self, call);
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        if is_cfg_test(&module.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, module);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        if is_cfg_test(&function.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, function);
    }
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .to_token_stream_string()
                .replace(char::is_whitespace, "")
                .contains("test")
    })
}

/// `syn::Attribute` has no stable string form, so render it through `quote`
/// exactly as the other AST gates in this directory do.
trait TokenStreamString {
    fn to_token_stream_string(&self) -> String;
}

impl TokenStreamString for syn::Attribute {
    fn to_token_stream_string(&self) -> String {
        use quote::ToTokens;
        self.to_token_stream().to_string()
    }
}

/// The AST second opinion: the install must be a METHOD call in the production
/// composition root, outside any `#[cfg(test)]`, and the two live-mesh handles
/// must be passed to it.
fn probe_production_install() -> Result<(), String> {
    let source = std::fs::read_to_string(COMPOSITION_ROOT)
        .map_err(|error| format!("{GATE_NAME}: cannot read {COMPOSITION_ROOT}: {error}"))?;
    let file = syn::parse_file(&source)
        .map_err(|error| format!("{GATE_NAME}: cannot parse {COMPOSITION_ROOT}: {error}"))?;
    let mut probe = MethodCallProbe {
        name: INSTALL_METHOD,
        found: false,
    };
    probe.visit_file(&file);
    if !probe.found {
        return Err(format!(
            "{GATE_NAME}: FAIL — no non-test `{INSTALL_METHOD}` METHOD call in \
             {COMPOSITION_ROOT}: the rotation trigger has no production caller, so a signed \
             manifest reissue cannot reach the live trust planes (AC1.2)"
        ));
    }
    for method in PLANE_METHODS {
        let mut plane = MethodCallProbe {
            name: method,
            found: false,
        };
        plane.visit_file(&file);
        if !plane.found {
            return Err(format!(
                "{GATE_NAME}: FAIL — {COMPOSITION_ROOT} never calls `.{method}()`: the install \
                 must be handed the RUNNING mesh's handles, not freshly built ones, or the \
                 reload reloads a copy (the measured-out `MAOS_ONE_SHOT` failure shape)"
            ));
        }
    }
    Ok(())
}

fn leg_invocations() -> Vec<(&'static str, Vec<Invocation>)> {
    vec![
        (
            "rotation-trigger-production-caller",
            vec![
                Invocation {
                    package: "maos-bin",
                    test_file: "cert_rotation_trigger_14_2a",
                    filter: "t_14_2a_a_signed_reissue_rotates_a_live_daemons_peer_trust",
                    features: Some("network"),
                    expected_tests: 1,
                    marker: Some("ROTATION_WINDOWS_OBSERVED=1"),
                },
                // The PRODUCTION timer, not the manual fixture: a no-op
                // `TokioGraceTimer::schedule` would leave every other leg green
                // and every widened trust set open forever.
                Invocation {
                    package: "maos-bin",
                    test_file: "cert_rotation_trigger_14_2a",
                    filter: "t_14_2a_the_production_grace_timer_fires_its_terminal_action_exactly_once",
                    features: Some("network"),
                    expected_tests: 1,
                    marker: None,
                },
                // The INSTALLED closer: the leg above proves the timer type
                // fires in isolation; this one proves the composition root
                // actually arms it. A no-op or early timer at the
                // `install_cert_rotation` site reds here and nowhere else.
                Invocation {
                    package: "maos-bin",
                    test_file: "cert_rotation_trigger_14_2a",
                    filter: "t_14_2a_the_installed_production_closer_closes_the_window_and_journals_the_close",
                    features: Some("network"),
                    expected_tests: 1,
                    marker: Some("ROTATION_WINDOW_CLOSED_OBSERVED=1"),
                },
            ],
        ),
        (
            "reload-atomicity",
            vec![
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter:
                        "a_signed_reissue_that_rotates_a_member_opens_a_window_moves_both_planes_and_promotes",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_reload_that_cannot_write_its_evidence_rolls_both_planes_back_and_fails_closed",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_cross_peer_fingerprint_swap_is_refused_for_both_peers_and_journaled",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                // THE ordering controls: they observe the two-plane write order
                // and the rollback order IN FLIGHT, at the instants production
                // arms its deadline and writes its compensating row. End-state
                // assertions cannot see a swapped order; these can, and a
                // mutation that swaps either one reds them deterministically.
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "the_two_plane_write_order_is_observed_in_flight",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_multi_peer_reload_that_fails_on_the_second_peer_rolls_the_first_one_back",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_newer_manifest_that_re_declares_the_serving_generation_aborts_the_window",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_third_generation_replaces_an_in_flight_window_and_the_stale_closer_cannot_win",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_failed_third_generation_reload_restores_the_superseded_window",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                // §A6 close-pass controls. The first two are the ones that
                // matter most: the terminal action must refuse to promote a
                // generation the router never declared (an audit-sink PANIC is
                // the production failure mode and it unwinds), and a stale
                // fire-and-forget closer must touch nothing.
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_closer_whose_router_declaration_never_committed_discards_instead_of_promoting",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_stale_closer_from_an_aborted_window_cannot_end_a_later_one",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_reissue_that_changes_no_fingerprint_takes_no_action_at_all",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "re_applying_the_same_rotation_during_an_open_window_is_idempotent",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_member_the_live_transport_never_declared_is_refused_not_inserted",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_reissue_that_removes_this_host_is_applied_and_journaled_never_silently_skipped",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "an_unpinned_peer_gets_its_declaration_moved_with_no_window_to_close",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "the_rotation_control_is_set_once",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "t_grace_matches_the_shipped_formula_on_the_cold_deployment_branch",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "the_operator_read_surface_never_reports_a_window_the_store_does_not_hold",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "the_projection_is_the_only_fingerprint_source_and_it_normalizes_through_parse",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_signed_reissue_with_an_uppercase_fingerprint_normalizes_on_the_live_planes",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "a_refused_rotation_is_reported_as_a_divergence_on_the_read_seam",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "an_unpinned_member_is_reported_as_awaiting_first_contact_not_as_a_divergence",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-core",
                    test_file: "t_14_2a_rotation_reload",
                    filter: "a_conditional_close_promotes_only_the_generation_it_opened",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-core",
                    test_file: "t_14_2a_rotation_reload",
                    filter: "open_then_move_keeps_the_invariant_at_every_observable_instant",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-core",
                    test_file: "t_14_2a_rotation_reload",
                    filter: "rollback_restores_plane_a_before_aborting_the_window",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-core",
                    test_file: "t_14_2a_rotation_reload",
                    filter: "abort_discards_next_and_leaves_the_current_pin_serving",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-core",
                    test_file: "t_14_2a_rotation_reload",
                    filter: "plane_a_mutator_reports_the_fingerprint_it_replaced",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
            ],
        ),
        (
            "rotation-audit-round-trip",
            vec![
                Invocation {
                    package: "maos-bin",
                    test_file: "cert_rotation_trigger_14_2a",
                    filter: "t_14_2a_rotation_rows_round_trip_through_the_real_audit_surface",
                    features: Some("network"),
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "every_rotation_variant_round_trips_through_the_real_transparency_log",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "exactly_one_terminal_row_is_written_per_opened_window",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-cohort",
                    test_file: "cert_rotation_14_2a",
                    filter: "every_rotation_row_carries_one_stable_greppable_intent",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                // The audit-safety boundary over the real wire: the I2
                // fail-stop panic (a sink `append` that PANICS, never `Err`)
                // must reach the supervised connection boundary, and the
                // healthy-session control proves the boundary fires on
                // panics, not on connections.
                Invocation {
                    package: "maos-a2a-tcp",
                    test_file: "t_14_2a_connection_panic_supervision",
                    filter: "t_14_2a_a_panicking_audit_write_is_caught_by_the_supervised_boundary",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-tcp",
                    test_file: "t_14_2a_connection_panic_supervision",
                    filter: "t_14_2a_a_healthy_session_trips_no_supervised_boundary",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
            ],
        ),
        (
            "post-grace-reject-journaled",
            vec![
                Invocation {
                    package: "maos-a2a-tcp",
                    test_file: "t_14_2a_post_grace_journal",
                    filter: "t_14_2a_a_retired_leaf_presented_after_the_grace_is_journaled",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-tcp",
                    test_file: "t_14_2a_post_grace_journal",
                    filter: "t_14_2a_a_promoted_generation_makes_the_retired_leaf_a_queryable_refusal",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
                Invocation {
                    package: "maos-a2a-tcp",
                    test_file: "t_14_2a_post_grace_journal",
                    filter: "t_14_2a_a_current_generation_handshake_journals_no_post_grace_row",
                    features: None,
                    expected_tests: 1,
                    marker: None,
                },
            ],
        ),
    ]
}

/// Invoke ONE exact test and parse its summary. GREEN requires: exit success, a
/// `test result:` line, exactly the expected number of passes, zero failures,
/// the exact `running <n> test` line, and any required marker as an exact
/// trimmed line.
fn invoke_cargo_test(invocation: &Invocation) -> Result<(u32, u32, bool, bool), String> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "--locked",
        "-p",
        invocation.package,
        "--test",
        invocation.test_file,
    ]);
    if let Some(features) = invocation.features {
        cmd.args(["--features", features]);
    }
    cmd.args([
        "--",
        invocation.filter,
        "--exact",
        "--nocapture",
        "--test-threads=1",
    ]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let output = cmd.output().map_err(|error| {
        format!(
            "cannot invoke `cargo test` ({}/{}): {error}",
            invocation.package, invocation.test_file
        )
    })?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let (passed, failed) = parse_test_summary(&combined);
    let ran = combined
        .lines()
        .any(|line| line.trim().starts_with("test result:"));
    let running_exact = combined.contains(&format!("running {} test", invocation.expected_tests));
    let measured = invocation.marker.is_none_or(|marker| {
        combined
            .lines()
            .filter(|line| line.trim() == marker)
            .count() as u32
            >= invocation.expected_tests
    });
    let green = output.status.success()
        && ran
        && passed == invocation.expected_tests
        && failed == 0
        && running_exact
        && measured;
    if !green {
        eprintln!(
            "{GATE_NAME}: {} (filter={:?}) NOT green (passed={passed}, expected={}, \
             failed={failed}, ran={ran}, running-exact={running_exact}, measured={measured}, \
             exit={}) — see the cargo output above",
            invocation.test_file, invocation.filter, invocation.expected_tests, output.status
        );
        eprintln!("{combined}");
    }
    Ok((passed, failed, ran, green))
}

fn run_leg(label: &'static str, invocations: &[Invocation]) -> LegResult {
    let (mut passed, mut failed, mut ran, mut green) = (0u32, 0u32, false, true);
    for invocation in invocations {
        match invoke_cargo_test(invocation) {
            Ok((p, f, r, g)) => {
                (passed, failed, ran, green) = (passed + p, failed + f, ran | r, green & g);
            }
            Err(error) => {
                eprintln!(
                    "{GATE_NAME}: {label} leg error ({}): {error}",
                    invocation.filter
                );
                (failed, ran, green) = (failed + 1, true, false);
            }
        }
    }
    LegResult {
        label,
        passed,
        failed,
        ran,
        attempted: true,
        green,
    }
}

fn parse_test_summary(output: &str) -> (u32, u32) {
    output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("test result:"))
        .fold((0, 0), |(passed, failed), rest| {
            (
                passed + parse_count(rest, "passed"),
                failed + parse_count(rest, "failed"),
            )
        })
}

fn parse_count(summary: &str, key: &str) -> u32 {
    summary
        .match_indices(key)
        .filter_map(|(at, _)| {
            let head = summary[..at].trim_end();
            let digits = head.trim_end_matches(|c: char| c.is_ascii_digit());
            head[digits.len()..].parse::<u32>().ok()
        })
        .sum()
}

/// AC6.2 — derive every real `#[test]`/`#[tokio::test]` function in the story's
/// rotation test files (five at HEAD) and require a leg to name it. A
/// hand-listed filter set is a suggestion; this is a control. The derivation
/// is by ATTRIBUTE, not by name prefix: the prefix rule (`t_14_2a_*`) derived
/// only the prefix-named files and silently ignored the cohort and a2a-core
/// files, whose tests carry no prefix — the gate then reported a fraction of
/// the story's tests as enrolled while the rest would never run. Each file
/// must independently derive at least one test; an empty per-file derivation
/// is an ERROR.
fn derive_rotation_tests() -> Result<Vec<(String, String)>, String> {
    derive_rotation_tests_in(std::path::Path::new("."))
}

/// `root` is the workspace root: `.` under `cargo run -p xtask`, and the
/// resolved parent of `CARGO_MANIFEST_DIR` when this gate's own unit tests call
/// it (they run with the xtask package as CWD).
fn derive_rotation_tests_in(root: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    let mut derived = Vec::new();
    for (_, stem, dir) in TEST_FILES {
        let path = root.join(dir).join(format!("{stem}.rs"));
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{GATE_NAME}: cannot read {}: {error}", path.display()))?;
        for name in per_file_derivation(&path, &text)? {
            derived.push((stem.to_string(), name));
        }
    }
    derived.sort();
    derived.dedup();
    Ok(derived)
}

/// One file's contribution: every fn carrying a test attribute, parsed as Rust
/// so the attribute matches structurally however it is laid out across lines
/// and whatever arguments it carries. Zero derived tests in ONE file is an
/// error even when the other three derive plenty — that is exactly the shape
/// of a silently-ignored file.
fn per_file_derivation(path: &std::path::Path, text: &str) -> Result<Vec<String>, String> {
    let names = derive_test_fns(text)
        .map_err(|error| format!("{GATE_NAME}: cannot parse {}: {error}", path.display()))?;
    if names.is_empty() {
        return Err(format!(
            "{GATE_NAME}: FAIL — {} derives ZERO `#[test]`/`#[tokio::test]` functions (AC6.2: \
             an empty per-file derivation is how a filesystem-derived enrollment goes \
             decorative)",
            path.display()
        ));
    }
    Ok(names)
}

/// The real test functions of one test source: `#[test]` and `#[tokio::test]`
/// (with any argument list), including functions nested in inline modules.
fn derive_test_fns(text: &str) -> Result<Vec<String>, String> {
    let file = syn::parse_file(text).map_err(|error| error.to_string())?;
    let mut names = Vec::new();
    collect_test_fns(&file.items, &mut names);
    Ok(names)
}

fn collect_test_fns(items: &[syn::Item], names: &mut Vec<String>) {
    for item in items {
        match item {
            syn::Item::Fn(function) => {
                if function.attrs.iter().any(is_test_attribute) {
                    names.push(function.sig.ident.to_string());
                }
            }
            // A `mod tests { ... }` block inside an integration test file is
            // still part of that file's test surface.
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_test_fns(items, names);
                }
            }
            _ => {}
        }
    }
}

/// `#[test]`, and `#[tokio::test]` however annotated. A `#[cfg(test)]` gate
/// alone does NOT make a function a test, and neither does any other
/// attribute someone might reach for (`#[should_panic]` without `#[test]`
/// runs nothing).
fn is_test_attribute(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    if path.is_ident("test") {
        return true;
    }
    path.segments.len() == 2
        && path.segments[0].ident == "tokio"
        && path.segments[1].ident == "test"
}

fn legs_json(legs: &[LegResult]) -> serde_json::Value {
    serde_json::Value::Array(
        legs.iter()
            .map(|leg| {
                serde_json::json!({
                    "label": leg.label,
                    "passed": leg.passed,
                    "failed": leg.failed,
                    "ran": leg.ran,
                    "attempted": leg.attempted,
                    "green": leg.green,
                })
            })
            .collect(),
    )
}

pub fn run(json: bool) -> Result<(), String> {
    // 1. Read + VALIDATE the phase disposition: this gate lands blocking, and a
    //    silent downgrade to advisory on any rung is the floor-relaxation shape
    //    Story 10.5 AC6 named.
    let disposition = read_disposition(GATE_NAME)?;
    for rung in ["v1_5", "v2_0", "v2_2"] {
        if disposition.get(rung).map(String::as_str) != Some("blocking") {
            return Err(format!(
                "{GATE_NAME}: registry defect — {rung} disposition must be \"blocking\" (got \
                 {:?}); this gate proves the story's own deliverable and an advisory rung would \
                 let it ship unproven (AC6.3)",
                disposition.get(rung)
            ));
        }
    }
    let blocking_now = is_blocking_at(&disposition, CURRENT_PHASE);
    let dev_blocks = blocking_now || dev_enforced_red_blocks(BindingClass::Blocking, true);

    // 2. The cheap second opinion, BEFORE any cargo run: a deleted install is
    //    caught in milliseconds. It is NOT the proof — see the module doc.
    if let Err(error) = probe_production_install() {
        emit_command(json, "error", &error);
        return Err(error);
    }

    // 3. Derived enrollment, still before any cargo run: a rotation test no leg
    //    names would never run.
    let derived = derive_rotation_tests()?;
    let legs_data = leg_invocations();
    for (stem, name) in &derived {
        let covered = legs_data.iter().any(|(_, invocations)| {
            invocations
                .iter()
                .any(|i| i.test_file == stem.as_str() && i.filter == name.as_str())
        });
        if !covered {
            return Err(format!(
                "{GATE_NAME}: FAIL — `{name}` ({stem}) will never run: no leg names it (AC6.2 \
                 derived enrollment)"
            ));
        }
    }

    // 4. Per-leg oracles.
    let legs: Vec<LegResult> = legs_data
        .iter()
        .map(|(label, invocations)| run_leg(label, invocations))
        .collect();

    // 5. Vacuous-green guard: a leg attempted but with zero tests run cannot
    //    pass. A re-stubbed harness is not evidence.
    for leg in &legs {
        if leg.attempted && (!leg.ran || (leg.passed == 0 && leg.failed == 0)) {
            let msg = format!(
                "{GATE_NAME}: FAIL — {} leg is vacuous (ran={}, passed={}, failed={})",
                leg.label, leg.ran, leg.passed, leg.failed
            );
            emit_command(json, "error", &msg);
            return Err(msg);
        }
    }

    let oracle_green = legs.iter().all(|leg| leg.green);
    if oracle_green {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "gate": GATE_NAME,
                    "passed": true,
                    "oracle_green": true,
                    "blocking_now": blocking_now,
                    "dev_blocks": dev_blocks,
                    "ship_phase": CURRENT_PHASE,
                    "disposition": disposition,
                    "enrolled_rotation_tests": derived.len(),
                    "legs": legs_json(&legs),
                })
            );
        } else {
            eprintln!(
                "{GATE_NAME}: PASSED — oracle green ({} legs, {} enrolled rotation tests); {} at {}",
                legs.len(),
                derived.len(),
                if dev_blocks { "BLOCKING" } else { "advisory" },
                CURRENT_PHASE,
            );
        }
        return Ok(());
    }

    let detail: String = legs
        .iter()
        .map(|leg| {
            format!(
                "- {} leg: {} passed, {} failed (ran={}, attempted={}, green={})\n",
                leg.label, leg.passed, leg.failed, leg.ran, leg.attempted, leg.green
            )
        })
        .collect();
    let msg = format!("{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE} (binding):\n{detail}");
    emit_command(json, "error", &msg);
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": false,
                "oracle_green": false,
                "blocking_now": blocking_now,
                "dev_blocks": dev_blocks,
                "ship_phase": CURRENT_PHASE,
                "disposition": disposition,
                "enrolled_rotation_tests": derived.len(),
                "legs": legs_json(&legs),
                "error": &msg,
            })
        );
    } else {
        eprintln!("{msg}");
    }
    Err(format!(
        "{GATE_NAME}: BLOCKING — oracle RED at {CURRENT_PHASE}"
    ))
}

#[cfg(test)]
mod tests {
    include!("tests/check_cert_rotation_trigger_tests.rs");
}
