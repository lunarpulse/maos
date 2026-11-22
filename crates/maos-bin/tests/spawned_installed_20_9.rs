//! Story 20-9 review correction (GPT #2) — an installed component, booted by the shell root
//! (`InstalledSpirits::boot`, no `maos run`), is upgraded in place through the real door. The
//! swap handles used to be armed only by `maos run`, so this root's component was taken for an
//! in-process Spirit and its WASM successor was refused.
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "support/installed_shell_20_9.rs"]
mod shell;
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "support/spawned_world_20_9.rs"]
mod world;

use shell::{text, Shell};
use std::path::Path;
use world::{manifest, ALL_HOOKS, ID};

/// An installed component (`maos spirit install`, then the shell root's boot) at `1.0.0`
/// whose manifest appends `extra` (further sections); returns the running shell.
fn boot_installed(extra: &str) -> Shell {
    let component = manifest(ID, "1.0.0", "dist/lifecycle_probe.wasm", &ALL_HOOKS, extra);
    shell::boot_installed(&[(ID, component)], "lifecycle_probe.wasm").0
}

/// A successor of the installed component: a signed package of the SAME publisher (a same-pid
/// swap keeps the Spirit's publisher-bound storage identity), installed beside the running
/// version. Returns the installed successor manifest.
fn signed_successor(shell: &Shell, version: &str, extra: &str) -> std::path::PathBuf {
    let text = manifest(ID, version, "dist/lifecycle_probe.wasm", &ALL_HOOKS, extra);
    shell.signed_successor(ID, version, &text, "lifecycle_probe.wasm")
}

#[test]
fn an_installed_component_is_upgraded_in_place_through_the_door_without_maos_run() {
    let shell = boot_installed("");
    let pid = shell.field(ID, "pid").expect("the door reports its pid");

    // The successor is a signed package of the same publisher, installed beside the running
    // version; the installed predecessor is a component the root booted, not one `maos run`
    // started.
    let v2 = signed_successor(&shell, "1.1.0", "");
    let output = shell.ctl(&[
        "spirit",
        "upgrade",
        ID,
        "--to",
        v2.to_str().unwrap(),
        "--policy",
        "hot-swap",
    ]);
    assert!(
        output.status.success(),
        "the door upgrades the installed component: {}\nshell: {}",
        text(&output),
        shell.stderr()
    );
    assert_eq!(shell.field(ID, "pid"), Some(pid), "the same pid");
    assert_eq!(
        shell.field(ID, "lifecycle_state").as_deref(),
        Some("Running")
    );
}

/// Story 20-8 review correction (GPT P2-1) — the dry run compares a successor against the
/// predecessor's LIVE swap-invariant declaration, as the real swap does. An installed
/// component's boot records no retained manifest text (a cold swap refuses it as
/// `cold_swap_predecessor_unknown`), so a retained-text comparison had nothing to compare
/// and called a dropping successor safe that the coordinator then refuses.
#[test]
fn an_installed_components_precheck_agrees_with_its_swap_on_a_dropped_swap_invariant() {
    let shell =
        boot_installed("[swap_invariants]\npreserve = [\"open_pr_state\", \"review_queue\"]\n");
    let pid = shell.field(ID, "pid").expect("the door reports its pid");
    let successor = |version: &str, preserve: &str| {
        signed_successor(
            &shell,
            version,
            &format!("[swap_invariants]\npreserve = [{preserve}]\n"),
        )
    };
    let swap = |path: &Path, policy: &str| {
        shell.ctl(&[
            "spirit",
            "upgrade",
            ID,
            "--to",
            path.to_str().unwrap(),
            "--policy",
            policy,
        ])
    };
    let precheck = |path: &Path| {
        let output = shell.ctl(&[
            "spirit",
            "hot-swap-precheck",
            ID,
            "--to",
            path.to_str().unwrap(),
        ]);
        let report: serde_json::Value = serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| panic!("a precheck body ({error}): {}", text(&output)));
        (output.status.code(), report)
    };

    let dropping = successor("1.1.0", "\"open_pr_state\"");
    // The premise: this root retained no manifest text for its installed component.
    let cold = swap(&dropping, "cold-swap");
    assert!(
        text(&cold).contains("cold_swap_predecessor_unknown"),
        "no retained text for an installed component: {}",
        text(&cold)
    );
    let (exit, report) = precheck(&dropping);
    assert_eq!(exit, Some(2), "an unsafe verdict: {report}");
    assert_eq!(report["safe"], false, "{report}");
    assert_eq!(
        report["verdict"]["verdict"], "SwapInvariantDropped",
        "{report}"
    );
    assert_eq!(report["verdict"]["cause"], "review_queue", "{report}");
    let refused = swap(&dropping, "hot-swap");
    assert!(
        !refused.status.success()
            && text(&refused).contains("swap_invariant_dropped")
            && text(&refused).contains("review_queue"),
        "the swap refuses what the precheck reported: {}\nshell: {}",
        text(&refused),
        shell.stderr()
    );
    assert_eq!(
        shell.field(ID, "pid"),
        Some(pid.clone()),
        "the predecessor is untouched"
    );

    let superset = successor("1.1.1", "\"review_queue\", \"open_pr_state\"");
    let (exit, report) = precheck(&superset);
    assert_eq!(exit, Some(0), "a superset prechecks safe: {report}");
    assert_eq!(report["safe"], true, "{report}");
    let swapped = swap(&superset, "hot-swap");
    assert!(
        swapped.status.success(),
        "the swap takes what the precheck passed: {}\nshell: {}",
        text(&swapped),
        shell.stderr()
    );
    assert_eq!(shell.field(ID, "pid"), Some(pid), "the same pid");
}
