#![forbid(unsafe_code)]

//! Story 19-1 (AC1.2) — `maos run … --epic <file>` parse vectors, driven
//! through the production `parse_run_args` (in-`src` tests are kloc-charged).
//! The run-time refusals (a manifest with no host-bearing entry, the
//! cross-host arm) are driven through the production `load_epic` here and
//! end to end in `epic_dispatch_19_1.rs`.

use maos_bin::orchestrator_root::load_epic;
use maos_bin::worker_spawn::{parse_run_args, RunArgs};

fn parse(args: &[&str]) -> Result<Option<RunArgs>, String> {
    parse_run_args(args.iter().map(|arg| (*arg).to_owned()))
}

fn epic_fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/j1/epic.md")
}

#[test]
fn epic_flag_carries_its_file_beside_once() {
    let run = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
        "tests/fixtures/j1/epic.md",
        "--once",
    ])
    .expect("a valid --epic run parses")
    .expect("a `run` invocation");
    assert_eq!(
        run,
        RunArgs {
            manifest_path: "spirits/topologies/j1-founder-loop-epic.toml".into(),
            live: false,
            once: true,
            scene: None,
            epic: Some("tests/fixtures/j1/epic.md".into()),
        }
    );
}

#[test]
fn no_epic_flag_leaves_epic_unset() {
    let run = parse(&["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .expect("parses")
        .expect("a `run` invocation");
    assert_eq!(run.epic, None, "without --epic the run is today's run");
}

#[test]
fn epic_without_a_value_is_refused() {
    let error = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
    ])
    .expect_err("--epic with no value must refuse");
    assert!(
        error.contains("--epic needs a file"),
        "the refusal names the missing value: {error}"
    );
}

#[test]
fn epic_followed_by_a_flag_is_refused_not_swallowed() {
    let error = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
        "--once",
    ])
    .expect_err("--epic must not take the next flag as its file");
    assert!(error.contains("--epic needs a file"), "{error}");
}

#[test]
fn epic_given_twice_is_refused() {
    let error = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
        "a.md",
        "--epic",
        "b.md",
    ])
    .expect_err("two epics must refuse");
    assert!(error.contains("--epic is given twice"), "{error}");
}

#[test]
fn epic_with_a_butler_scene_is_refused() {
    for scene in [
        &["--clock", "compressed:3d", "--acceptance", "fixture.toml"][..],
        &["--clock", "compressed:3d"][..],
        &["--acceptance", "fixture.toml"][..],
    ] {
        let mut args = vec!["run", "spirits/butler/manifest.toml", "--epic", "e.md"];
        args.extend_from_slice(scene);
        let error = parse(&args).expect_err("--epic with --clock/--acceptance must refuse");
        assert!(
            error.contains("--epic") && error.contains("--clock/--acceptance"),
            "the refusal names both flags for {scene:?}: {error}"
        );
    }
}

#[test]
fn usage_lists_the_epic_flag() {
    let error = parse(&["run", "m.toml", "--bogus"]).expect_err("unknown flag refuses");
    assert!(
        error.contains("[--epic <file>]"),
        "the usage line names --epic: {error}"
    );
}

#[test]
fn epic_on_a_topology_with_no_host_bearing_entry_is_refused() {
    let error = load_epic(&epic_fixture(), 0, false)
        .expect_err("no host-bearing entry has no Worker to bind a section to");
    assert!(error.contains("host-bearing"), "{error}");
}

#[test]
fn epic_on_the_cross_host_arm_is_refused_before_reading_the_file() {
    let error = load_epic(std::path::Path::new("/nonexistent/epic.md"), 3, true)
        .expect_err("--epic on the cross-host arm must refuse");
    assert!(
        error.contains("loopback arm only") && error.contains("MAOS_DELEGATED_GOAL"),
        "{error}"
    );
}

#[test]
fn the_fixture_epic_splits_into_three_sections_for_three_workers() {
    let sections = load_epic(&epic_fixture(), 3, false).expect("three sections, three entries");
    assert_eq!(
        sections.iter().map(|s| s.ordinal).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(sections.iter().all(|s| s.text.starts_with("### Story")));
}

#[test]
fn a_count_mismatch_is_refused_naming_both_counts() {
    let error = load_epic(&epic_fixture(), 2, false).expect_err("3 sections vs 2 entries");
    assert!(
        error.contains("3 `### Story` sections vs 2 host-bearing entries"),
        "{error}"
    );
}
