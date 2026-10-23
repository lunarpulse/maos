#![allow(dead_code)]

//! Shared test-harness capability declaration (the Story 17-6 T2 convention).

/// `CI`/`GITHUB_ACTIONS` set to anything but `""`, `0` or `false`
/// (case-insensitive) declares a capable runner.
pub fn declared_ci() -> bool {
    let declared = |key| {
        std::env::var(key).is_ok_and(|value| {
            let value = value.trim();
            !(value.is_empty() || value == "0" || value.eq_ignore_ascii_case("false"))
        })
    };
    declared("CI") || declared("GITHUB_ACTIONS")
}

/// Story 20-12 AC3 — the local SKIP profile, one rule for every substrate test.
///
/// Returns `true` when the caller must return early: `prereq` is absent AND the
/// host has not declared CI capability. It then prints exactly one
/// `SKIP: <prereq>` line. On a declared-CI host it returns `false` even when the
/// prerequisite is absent, so the caller runs on into its own fail-closed
/// assertion — the CI path is byte-identical to the code before this helper.
pub fn skip_unless(prereq: &str, present: bool) -> bool {
    if present || declared_ci() {
        return false;
    }
    eprintln!("SKIP: {prereq}");
    true
}
