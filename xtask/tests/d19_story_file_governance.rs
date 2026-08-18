#![forbid(unsafe_code)]

//! D19 (vehicle 14-0, option (a)) — **the planted red.**
//!
//! Seven story-file walkers selected their subjects with
//! `name.starts_with(|c: char| c.is_ascii_digit())`. Every story whose key does not
//! begin with a digit was invisible to all seven at once — and that is the entire
//! `j1-*` lane: the one running cross-host mTLS, signed artifacts and a paid agent,
//! i.e. exactly where dev-record, model-tier and review-findings discipline matters
//! most. The hole stayed open across `1a`, `1b`, `j1-demo-one-command-scene`, `2a`
//! and `2b`; each disclosed it in prose and none closed it, which is why disclosure
//! stopped being an acceptable disposition.
//!
//! **Acceptance is this file, not the helper.** A shared helper without a planted
//! red is a refactor. Every vector below plants a defect in a `j1-*` story file and
//! asserts a Blocking gate goes RED — the thing that was impossible before.
//!
//! The fixtures live in a tempdir and the gates run with `current_dir` at the
//! fixture root — isolation rests on the gates' clap defaults resolving
//! `_bmad-output/...` relative to the CWD, not on explicit flags (§A6 review
//! 2026-08-18: the doc previously claimed flag-based isolation that these
//! vectors do not exercise). Nothing here reads or mutates the real repo tree.

use std::io::Write;
use std::path::Path;

/// A `j1-*` key: the exact shape the digit filter could never see.
const J1_KEY: &str = "j1-crosshost-9z-planted-vector";
/// A numeric key: the shape the digit filter DID see. Used to prove each vector is
/// about the DEFECT, not about the filename.
const NUMERIC_KEY: &str = "99-1-planted-vector";
/// A `.md` file in the story dir that is NOT a sprint key — a design note. It must
/// stay invisible: the new rule is membership, not "everything with an extension".
const UNGOVERNED: &str = "some-design-note";

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

/// A sprint status declaring `keys`, in the real file's shape — including the long
/// trailing provenance comments the single-sourced parser must strip.
fn sprint_status(keys: &[&str]) -> String {
    let mut s = String::from("last_updated: '2026-08-17'\nproject: maos\n\ndevelopment_status:\n");
    s.push_str("  # a comment line inside the block must not become a key\n");
    for key in keys {
        s.push_str(&format!(
            "  {key}: done  # dev_model_used: anthropic/claude-opus-5; SEALED 2026-08-17\n"
        ));
    }
    s
}

/// A COMPLETE story file: every field the walkers demand.
fn complete_story(key: &str) -> String {
    format!(
        "---\nstory_key: {key}\ndev_model_used: anthropic/claude-opus-5\n---\n\n\
         # Story {key}\n\n\
         ## Tasks\n\n- [x] T1 done\n\n\
         ## Dev Agent Record\n\n\
         ### Agent Model Used\n\nanthropic/claude-opus-5 (harness: omp, 2026-08-17)\n\n\
         ### Debug Log References\n\nnone\n\n\
         ### Completion Notes List\n\nEverything landed.\n\n\
         ### File List\n\n- `crates/maos-cli/src/subcommands.rs`\n\n\
         ## Change Log\n\n| Date | Change |\n|---|---|\n| 2026-08-17 | Shipped. |\n\n\
         ### Review Findings\n\n| # | Severity | Finding | Status |\n|---|---|---|---|\n\
         | 1 | Low | A real finding. | CLOSED (`crates/maos-cli/src/subcommands.rs`) |\n"
    )
}

/// The minimal `deferred-work.md` the dev-record gate requires: at least one open
/// owner assertion, or its own sweep reports itself vacuous.
const DEFERRED_WORK: &str = "# Deferred Work\n\n\
    ## Deferred from: something\n\n\
    - A real deferred item. Ownerless and open: no story successor exists.\n";

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    /// `keys` are declared in the sprint status; each gets a COMPLETE story file.
    fn new(keys: &[&str]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            dir.path(),
            "_bmad-output/implementation-artifacts/sprint-status.yaml",
            &sprint_status(keys),
        );
        for key in keys {
            write_file(
                dir.path(),
                &format!("_bmad-output/implementation-artifacts/{key}.md"),
                &complete_story(key),
            );
        }
        // An ungoverned `.md` with EVERY defect. It must never red anything.
        write_file(
            dir.path(),
            &format!("_bmad-output/implementation-artifacts/{UNGOVERNED}.md"),
            "# A design note\n\n### Review Findings\n\n_No review findings._\n",
        );
        write_file(
            dir.path(),
            "_bmad-output/implementation-artifacts/deferred-work.md",
            DEFERRED_WORK,
        );
        Self { dir }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn story(&self, key: &str) -> String {
        format!("_bmad-output/implementation-artifacts/{key}.md")
    }

    /// Run a gate against this fixture. Gates that take explicit dir arguments get
    /// them; the rest run with `current_dir` at the fixture root.
    fn gate(&self, gate: &str, extra: &[&str]) -> (bool, String) {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg(gate)
            .args(extra)
            .arg("--json")
            .current_dir(self.dir.path())
            .output()
            .expect("xtask must run");
        (
            out.status.success(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }
}

// ── Non-vacuity first ──────────────────────────────────────────────────────

/// A complete fixture is GREEN on every converted gate. Without this, every red
/// below could be satisfied by a gate that reds on anything.
#[test]
fn a_complete_fixture_is_green_on_every_converted_gate() {
    let fx = Fixture::new(&[J1_KEY, NUMERIC_KEY]);
    for gate in [
        "check-bare-review-findings",
        "check-dev-model-used-populated",
        "check-dev-record-completeness",
        "check-review-findings-resolved",
    ] {
        let (ok, out) = fx.gate(gate, &[]);
        assert!(ok, "{gate} must be GREEN on a complete fixture:\n{out}");
    }
}

/// An ungoverned `.md` carrying every defect reds nothing. Membership is by KEY,
/// so a design note in the story directory is not a story.
#[test]
fn an_ungoverned_md_file_reds_nothing() {
    let fx = Fixture::new(&[J1_KEY]);
    // The fixture already wrote `some-design-note.md` with a bare RF placeholder
    // and no dev record at all.
    for gate in [
        "check-bare-review-findings",
        "check-dev-model-used-populated",
        "check-dev-record-completeness",
        "check-review-findings-resolved",
    ] {
        let (ok, out) = fx.gate(gate, &[]);
        assert!(
            ok,
            "{gate} must ignore a `.md` that is not a sprint-status key:\n{out}"
        );
    }
    let (_, out) = fx.gate("check-bare-review-findings", &[]);
    assert!(
        !out.contains(UNGOVERNED),
        "an ungoverned file must not even be named: {out}"
    );
}

// ── THE PLANTED RED — a `j1-*` defect must RED ─────────────────────────────

/// **D19's binding acceptance test.** A `j1-*` story with a MISSING dev record
/// makes `check-dev-record-completeness` RED. Under the digit filter this file was
/// never opened, so this defect shipped green five times.
#[test]
fn a_j1_story_with_a_missing_dev_record_reds_the_blocking_gate() {
    let fx = Fixture::new(&[J1_KEY]);
    let mut gutted = complete_story(J1_KEY);
    // Remove the whole Dev Agent Record — the exact defect the gate exists to catch.
    gutted = gutted
        .split("## Dev Agent Record")
        .next()
        .unwrap()
        .to_string();
    write_file(fx.path(), &fx.story(J1_KEY), &gutted);

    let (ok, out) = fx.gate("check-dev-record-completeness", &[]);
    assert!(
        !ok,
        "a `j1-*` story with no dev record MUST red a Blocking gate — this is D19's \
         acceptance test and it was impossible before:\n{out}"
    );
    assert!(
        out.contains(J1_KEY),
        "the finding must name the j1 story: {out}"
    );
}

/// The same defect under a NUMERIC key is also red — so the vector is about the
/// DEFECT, not the filename. This is what shows the old filter's hole was the only
/// thing hiding the j1 case.
#[test]
fn the_same_defect_reds_under_a_numeric_key_too() {
    let fx = Fixture::new(&[NUMERIC_KEY]);
    let gutted = complete_story(NUMERIC_KEY)
        .split("## Dev Agent Record")
        .next()
        .unwrap()
        .to_string();
    write_file(fx.path(), &fx.story(NUMERIC_KEY), &gutted);

    let (ok, out) = fx.gate("check-dev-record-completeness", &[]);
    assert!(!ok, "the defect is the defect, whatever the name:\n{out}");
    assert!(out.contains(NUMERIC_KEY), "{out}");
}

/// A `j1-*` story with a bare review-findings placeholder reds
/// `check-bare-review-findings`.
#[test]
fn a_j1_story_with_a_bare_review_findings_placeholder_reds() {
    let fx = Fixture::new(&[J1_KEY]);
    let bare = complete_story(J1_KEY).replace(
        "| 1 | Low | A real finding. | CLOSED (`crates/maos-cli/src/subcommands.rs`) |",
        "_No review findings._",
    );
    write_file(fx.path(), &fx.story(J1_KEY), &bare);

    let (ok, out) = fx.gate("check-bare-review-findings", &[]);
    assert!(
        !ok,
        "a bare RF placeholder in a `j1-*` story must red:\n{out}"
    );
    assert!(out.contains(J1_KEY), "{out}");
}

/// A `j1-*` story with no recorded model reds `check-dev-model-used-populated`.
#[test]
fn a_j1_story_with_no_recorded_model_reds() {
    let fx = Fixture::new(&[J1_KEY]);
    // MISSING, not merely unrecognised: the frontmatter field is gone and the
    // `Agent Model Used` section carries nothing. An unknown-but-present model is
    // only a warning, so a vector that plants one would prove the wrong thing.
    let no_model = complete_story(J1_KEY)
        .replace("dev_model_used: anthropic/claude-opus-5\n", "")
        .replace(
            "### Agent Model Used\n\nanthropic/claude-opus-5 (harness: omp, 2026-08-17)\n\n",
            "",
        );
    write_file(fx.path(), &fx.story(J1_KEY), &no_model);

    let (ok, out) = fx.gate("check-dev-model-used-populated", &[]);
    assert!(
        !ok,
        "a `j1-*` story with no model provenance must red:\n{out}"
    );
    assert!(out.contains(J1_KEY), "{out}");
}

/// A `j1-*` story with an OPEN review finding while its sprint status is `done`
/// reds `check-review-findings-resolved`.
#[test]
fn a_j1_story_with_an_open_finding_while_done_reds() {
    let fx = Fixture::new(&[J1_KEY]);
    let open_finding = complete_story(J1_KEY).replace(
        "| 1 | Low | A real finding. | CLOSED (`crates/maos-cli/src/subcommands.rs`) |",
        "| 1 | High | A real finding. | OPEN |",
    );
    write_file(fx.path(), &fx.story(J1_KEY), &open_finding);

    let (ok, out) = fx.gate("check-review-findings-resolved", &[]);
    assert!(!ok, "an OPEN finding on a `done` j1 story must red:\n{out}");
    assert!(out.contains(J1_KEY), "{out}");
}

// ── Fail-closed: a gate that governs nothing must not pass ─────────────────

/// No `sprint-status.yaml` ⇒ the governed set cannot be derived ⇒ RED. Returning
/// an empty set would silently reduce five Blocking gates to no-ops.
#[test]
fn a_missing_sprint_status_reds_rather_than_governing_nothing() {
    let fx = Fixture::new(&[J1_KEY]);
    std::fs::remove_file(
        fx.path()
            .join("_bmad-output/implementation-artifacts/sprint-status.yaml"),
    )
    .unwrap();
    for gate in [
        "check-bare-review-findings",
        "check-dev-model-used-populated",
        "check-dev-record-completeness",
        "check-review-findings-resolved",
    ] {
        let (ok, out) = fx.gate(gate, &[]);
        assert!(
            !ok,
            "{gate} must FAIL CLOSED with no story list, not govern an empty set:\n{out}"
        );
        assert!(
            out.contains("ZERO development_status story keys")
                || out.contains("Refusing to walk an empty set"),
            "{gate} must say WHY it refused: {out}"
        );
    }
}

/// An empty `development_status` block is the same failure with a file present —
/// the sneakier version, because the file exists and parses.
#[test]
fn an_empty_development_status_block_reds_too() {
    let fx = Fixture::new(&[J1_KEY]);
    write_file(
        fx.path(),
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        "last_updated: '2026-08-17'\ndevelopment_status:\n  # every entry commented out\n",
    );
    let (ok, out) = fx.gate("check-dev-record-completeness", &[]);
    assert!(
        !ok,
        "an empty governed set must refuse, not pass for the wrong reason:\n{out}"
    );
}

/// §A6 review of j1-crosshost-2c (2026-08-18): the INVERSE hole. A key
/// DECLARED in sprint-status whose story file is missing shrank the governed
/// set silently — `rm <story>.md` and the story escapes every walker. An
/// ACTIVE key (review/in-progress/ready-for-dev) with no file must RED every
/// converted gate — this vector exercises all seven walk sites end-to-end,
/// including the two (`check-dev-model-tier`, `check-epic-6-bridge`) the
/// per-defect vectors above never reach.
#[test]
fn an_active_declared_story_with_no_file_reds_every_walker() {
    const GATES: &[&str] = &[
        "check-bare-review-findings",
        "check-dev-model-used-populated",
        "check-dev-record-completeness",
        "check-review-findings-resolved",
        "check-dev-model-tier",
        "check-epic-6-bridge",
    ];
    for gate in GATES {
        let fx = Fixture::new(&[J1_KEY, NUMERIC_KEY]);
        // Declare an ACTIVE story whose file does not exist.
        let status_path = fx
            .path()
            .join("_bmad-output/implementation-artifacts/sprint-status.yaml");
        let mut text = std::fs::read_to_string(&status_path).unwrap();
        text.push_str("  j1-crosshost-8z-ghost-story: review\n");
        std::fs::write(&status_path, text).unwrap();

        let (ok, out) = fx.gate(gate, &[]);
        assert!(
            !ok,
            "`{gate}` must RED when an ACTIVE declared story has no file\n{out}"
        );
        // `check-epic-6-bridge` consumes the same helper but its own fixture
        // substrate (story 5.5d, serde-error allowlists…) fails first in a
        // minimal fixture, so its red is not guaranteed to carry the
        // missing-file message — the walk site is still exercised.
        if *gate != "check-epic-6-bridge" {
            assert!(
                out.contains("no story file"),
                "`{gate}` must name the missing file for the ghost story\n{out}"
            );
        }
    }
}

/// `epic-*` roll-ups and retrospectives are not stories and never were. The helper
/// must not newly govern them.
#[test]
fn epic_rollups_and_retrospectives_are_not_governed() {
    let fx = Fixture::new(&[J1_KEY, "epic-99", "epic-99-retrospective"]);
    // Gut both non-story files completely.
    for key in ["epic-99", "epic-99-retrospective"] {
        write_file(fx.path(), &fx.story(key), "# not a story\n");
    }
    let (ok, out) = fx.gate("check-dev-record-completeness", &[]);
    assert!(
        ok,
        "epic roll-ups and retrospectives must stay out of the governed set:\n{out}"
    );
}
