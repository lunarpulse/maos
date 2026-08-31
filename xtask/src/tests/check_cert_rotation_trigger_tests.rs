// Story 14-2a — unit tests for the rotation-trigger gate's own oracles.
//
// These live in `xtask/src/tests/` because that directory is UNCHARGED by
// `kloc-check` (measured: `xtask/src` = 43603 tokei lines, `xtask/src/tests/` =
// 2472, charged figure = 41131), so the gate's controls cost nothing against a
// ceiling that is at zero headroom. The file is `include!`d by its owning
// module, so it carries OUTER comments only.
use super::*;

fn workspace_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is directly under the workspace root")
}

/// The AST probe must see a METHOD call. This is the exact defect that made
/// `FunctionCallProbe` a null control for this story: it only visits
/// `ExprCall`, so a method call is invisible to it.
#[test]
fn method_call_probe_sees_a_method_call_that_the_free_function_probe_cannot() {
    let source = syn::parse_file(
        "fn root() { let state = load(); state.install_cert_rotation(a, b, c, d); }",
    )
    .expect("fixture parses");
    let mut probe = MethodCallProbe {
        name: "install_cert_rotation",
        found: false,
    };
    probe.visit_file(&source);
    assert!(probe.found, "a method call must be visible to this probe");
}

/// A free function of the same name is NOT the production shape this gate is
/// about, and the probe must not be satisfied by one.
#[test]
fn method_call_probe_is_not_satisfied_by_a_free_function_of_the_same_name() {
    let source = syn::parse_file("fn root() { install_cert_rotation(a, b, c, d); }")
        .expect("fixture parses");
    let mut probe = MethodCallProbe {
        name: "install_cert_rotation",
        found: false,
    };
    probe.visit_file(&source);
    assert!(
        !probe.found,
        "the production caller is a method on an Arc-held handle; a same-named free \
         function is a different claim"
    );
}

/// The narrowing that makes the spelling check worth keeping: an install that
/// exists only under `#[cfg(test)]` is NOT a production caller.
#[test]
fn method_call_probe_skips_cfg_test_modules() {
    let source = syn::parse_file(
        "#[cfg(test)] mod tests { fn t() { state.install_cert_rotation(a); } }",
    )
    .expect("fixture parses");
    let mut probe = MethodCallProbe {
        name: "install_cert_rotation",
        found: false,
    };
    probe.visit_file(&source);
    assert!(
        !probe.found,
        "a test-only install would let the gate pass while production never rotates"
    );
}

#[test]
fn method_call_probe_skips_cfg_test_functions() {
    let source =
        syn::parse_file("#[cfg(test)] fn t() { state.install_cert_rotation(a); }")
            .expect("fixture parses");
    let mut probe = MethodCallProbe {
        name: "install_cert_rotation",
        found: false,
    };
    probe.visit_file(&source);
    assert!(!probe.found);
}

/// The gate's verdict parser must read libtest's summary line, including the
/// multi-`test result:` case a multi-invocation leg produces.
#[test]
fn test_summary_parser_folds_every_result_line() {
    let output = "running 1 test\ntest a ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored\n\
                  running 2 tests\ntest result: FAILED. 1 passed; 1 failed; 0 ignored\n";
    assert_eq!(parse_test_summary(output), (2, 1));
}

#[test]
fn test_summary_parser_reports_zero_when_nothing_ran() {
    assert_eq!(parse_test_summary("error: could not compile\n"), (0, 0));
}

/// The derived enrollment is the control that stops the leg table going
/// decorative: every `#[test]`/`#[tokio::test]` function in the story's
/// rotation test files (five at HEAD)
/// must be named by a leg, and the derivation must be by ATTRIBUTE —
/// the old prefix rule silently ignored the cohort and a2a-core files. This
/// asserts the reconciliation the gate performs, against the REAL files
/// rather than a fixture.
#[test]
fn every_derived_rotation_test_is_named_by_a_leg() {
    let derived =
        derive_rotation_tests_in(workspace_root()).expect("the rotation test files are readable");
    assert!(
        !derived.is_empty(),
        "an empty derivation must be an error, never a pass"
    );
    let legs = leg_invocations();
    for (stem, name) in &derived {
        assert!(
            legs.iter().any(|(_, invocations)| invocations
                .iter()
                .any(|i| i.test_file == stem.as_str() && i.filter == name.as_str())),
            "derived test `{name}` ({stem}) is named by no leg"
        );
    }
}

/// The derivation is by ATTRIBUTE, not by name prefix: these cohort and
/// a2a-core tests carry no `t_14_2a_` prefix and were invisible to the old
/// prefix walk, which enrolled two files while silently ignoring the other
/// two. And helpers WITHOUT a test attribute are not tests, however
/// test-adjacent their names or their neighbors are.
#[test]
fn derivation_is_attribute_aware_not_prefix_based() {
    let derived =
        derive_rotation_tests_in(workspace_root()).expect("the rotation test files are readable");
    for name in [
        "a_signed_reissue_that_rotates_a_member_opens_a_window_moves_both_planes_and_promotes",
        "abort_discards_next_and_leaves_the_current_pin_serving",
        "open_then_move_keeps_the_invariant_at_every_observable_instant",
    ] {
        assert!(
            derived.iter().any(|(_, derived_name)| derived_name == name),
            "`{name}` carries a test attribute but the derivation missed it — the walk has \
             gone back to matching name prefixes"
        );
    }
    for name in ["block_on", "pinned_core", "plane_a_is_accepted_by_plane_b"] {
        assert!(
            !derived.iter().any(|(_, derived_name)| derived_name == name),
            "`{name}` has no test attribute and must never be enrolled"
        );
    }
}

/// Attribute matching is structural: `#[tokio::test]` with or without
/// arguments, on a sync or async fn, qualifies; a bare helper, a
/// `#[cfg(test)]` gate alone, and unattributed fns do not; nested modules are
/// walked.
#[test]
fn derivation_matches_test_attributes_structurally() {
    let mut names = derive_test_fns(
        "#[test]\n\
         fn sync_plain() {}\n\
         \n\
         #[tokio::test]\n\
         async fn async_plain() {}\n\
         \n\
         #[tokio::test(flavor = \"multi_thread\", worker_threads = 2)]\n\
         async fn async_with_args() {}\n\
         \n\
         fn helper() {}\n\
         async fn helper_async() {}\n\
         #[cfg(test)]\n\
         fn cfg_gate_only() {}\n\
         \n\
         mod inner {\n\
             #[test]\n\
             fn nested() {}\n\
         }\n",
    )
    .expect("fixture parses");
    names.sort();
    assert_eq!(
        names,
        vec!["async_plain", "async_with_args", "nested", "sync_plain"],
        "exactly the fns carrying a test attribute, however spelled or nested"
    );
}

/// The per-file zero rule: ONE file with no test-attributed function fails BY
/// NAME even though the other files derive plenty — that is what makes a
/// silently-ignored file impossible. A file that does not parse is a
/// different, equally loud error.
#[test]
fn a_file_that_derives_zero_tests_is_a_per_file_error() {
    let empty = per_file_derivation(
        std::path::Path::new("crates/example/tests/story_file.rs"),
        "fn block_on() {}\nfn fingerprint() {}\n",
    )
    .expect_err("zero test-attributed functions in one file must fail THAT file");
    assert!(
        empty.contains("ZERO") && empty.contains("story_file.rs"),
        "the error must name the offending file: {empty}"
    );
    let parse_error = per_file_derivation(
        std::path::Path::new("crates/example/tests/story_file.rs"),
        "fn broken(",
    )
    .expect_err("an unparseable test file cannot be enrolled");
    assert!(
        parse_error.contains("cannot parse"),
        "a parse failure is a distinct, loud error: {parse_error}"
    );
}

/// Every leg invocation must name a test that EXISTS. A filter naming a
/// deleted or renamed test would run zero tests, which the vacuous guard
/// catches at runtime — but catching it here costs nothing.
#[test]
fn every_leg_filter_names_a_test_that_exists() {
    for (label, invocations) in leg_invocations() {
        for invocation in invocations {
            let dir = TEST_FILES
                .iter()
                .find(|(_, stem, _)| *stem == invocation.test_file)
                .map(|(_, _, dir)| *dir)
                .unwrap_or_else(|| panic!("{label}: unknown test file {}", invocation.test_file));
            let path = workspace_root()
                .join(dir)
                .join(format!("{}.rs", invocation.test_file));
            let text = std::fs::read_to_string(&path).expect("test file is readable");
            assert!(
                text.contains(&format!("fn {}(", invocation.filter)),
                "{label}: leg filter `{}` names no test in {}",
                invocation.filter,
                path.display()
            );
        }
    }
}

/// Four legs, and each must carry at least one invocation. A leg with no
/// invocation would report green having asserted nothing.
#[test]
fn the_gate_has_four_non_empty_legs() {
    let legs = leg_invocations();
    assert_eq!(legs.len(), 4, "AC6.3 names four legs");
    for (label, invocations) in &legs {
        assert!(
            !invocations.is_empty(),
            "{label} leg has no invocation, so it could report green having run nothing"
        );
    }
}

/// The runtime control leg must require a derived witness marker on BOTH of
/// its live-daemon invocations. Without one, an early `return` in the test
/// would leave the leg green having observed nothing.
#[test]
fn the_runtime_control_leg_requires_a_derived_marker() {
    let legs = leg_invocations();
    let (_, control) = legs
        .iter()
        .find(|(label, _)| *label == "rotation-trigger-production-caller")
        .expect("the control leg exists");
    let observing = control
        .iter()
        .find(|i| i.filter.contains("rotates_a_live_daemons_peer_trust"))
        .expect("the leg names the live-daemon observation");
    assert_eq!(
        observing.marker,
        Some("ROTATION_WINDOWS_OBSERVED=1"),
        "the live-daemon observation must publish the count of windows it actually read \
         from the operator surface"
    );
    let closing = control
        .iter()
        .find(|i| i.filter.contains("installed_production_closer"))
        .expect("the leg names the installed production closer");
    assert_eq!(
        closing.marker,
        Some("ROTATION_WINDOW_CLOSED_OBSERVED=1"),
        "the installed-closer observation must publish its witness too: without it, an \
         early return after the window opens would leave the leg green having never \
         waited for the close"
    );
}
