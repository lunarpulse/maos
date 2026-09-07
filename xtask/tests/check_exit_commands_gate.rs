//! Story `15-3` AC5 — proven-red vectors for `check-exit-commands`.
//!
//! **Every vector here is about a DEFECT the gate exists to catch, and each is
//! measured against a GREEN control so the red cannot be satisfied by something
//! incidental.** The defects are not hypothetical: each one was found in this
//! repository's own exit blocks while the gate was being written.
//!
//! * `epic-15-*` matches THREE files, two of them preflights with no exit block
//!   — a glob-and-skip corpus silently passes the day a real epic loses its
//!   block, so the roster is positive and zero-or-two is a finding (F10).
//! * `for i in $(seq 1 50); do (exec 3<>/dev/tcp/…) …` glues a subshell paren
//!   to the head word, and `MAOS_SPIRIT_SIGNING_KEY=$(od … | tr …) maos-spirit
//!   publish …` hides a pipe inside `$( )` that re-heads the command to `tr`.
//! * `maosctl halt resolve <halt_id> --spirit …` is read by bash as TWO
//!   REDIRECTIONS: run verbatim it creates a file named `--spirit` and swallows
//!   the flag the command requires, while `bash -n` stays clean (F15).
//! * Stripping a `MAOS_ONE_SHOT=` prefix leaves a bare `maos`, which is a LEGAL
//!   verb-less invocation — so the gate would resolve green while the real
//!   command is an undeclared one-shot mode (§6).
//!
//! These drive `check_exit_commands::audit` — the SAME function
//! `xtask check-exit-commands` calls in CI. A vector that exercised a copy of
//! the tokeniser would prove nothing about the gate that actually runs.

use std::collections::{BTreeMap, BTreeSet};

use xtask::check_exit_commands::{audit, Corpus, EpicFile, FindingKind, Surfaces};

/// The surfaces a fixture corpus is judged against: `maos` has three verbs and
/// one one-shot mode, `maosctl` one, `xtask` one. Deliberately tiny so a
/// planted absent verb is unambiguous.
fn surfaces() -> Surfaces {
    Surfaces {
        maos: set(&["init", "run", "audit"]),
        maosctl: set(&["halt", "install", "pause", "spirit"]),
        xtask: set(&["kloc-check"]),
        subcommands: [
            ("maosctl halt".to_string(), set(&["list", "resolve"])),
            (
                "maosctl spirit".to_string(),
                set(&["hot-swap-precheck", "inspect", "upgrade"]),
            ),
            (
                "maos-spirit".to_string(),
                set(&["inspect", "publish", "validate"]),
            ),
        ]
        .into_iter()
        .collect(),
        one_shot_modes: set(&["registry-server"]),
        present_binaries: set(&["maos", "maosctl", "xtask"]),
        unavailable_binaries: BTreeMap::new(),
        surface_failures: Vec::new(),
    }
}

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|v| v.to_string()).collect()
}

fn statuses(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// An epic document carrying `block` as its hermetic exit block, plus whatever
/// `extra` prose (provenance bullets, AC bullets) the vector needs.
fn epic_doc(block: &str, extra: &str) -> String {
    format!(
        "# fixture epic\n\n**Hermetic exit command (CI, no secrets):**\n\n```\n{block}\n```\n\n\
         **Exit-block provenance:**\n{extra}\n"
    )
}

/// A one-epic corpus: epic 99, `in-progress`, one block-carrying file.
fn corpus(block: &str, extra: &str) -> Corpus {
    Corpus {
        epics: [(
            99,
            vec![EpicFile {
                name: "epic-99-fixture.md".to_string(),
                content: epic_doc(block, extra),
            }],
        )]
        .into_iter()
        .collect(),
        statuses: statuses(&[("epic-99", "in-progress"), ("99-1-owner", "backlog")]),
        story_files: BTreeMap::new(),
    }
}

/// Every verb in this block exists in [`surfaces`]; it is the control every red
/// below is measured against.
const GREEN_BLOCK: &str = "\
maos init && maos run spirits/butler/manifest.toml --once   # 1 — comment is stripped
maosctl halt list --spirit hello-spirit                     # 2
cargo run -p xtask -- kloc-check                            # 3 — the xtask verb resolves
MAOS_ONE_SHOT=registry-server maos &                        # 4 — mode resolved, not stripped";

fn kinds(audit: &xtask::check_exit_commands::Audit) -> Vec<FindingKind> {
    audit.findings.iter().map(|f| f.kind).collect()
}

// ───────────────────────────── the GREEN control ─────────────────────────────

#[test]
fn green_control_block_passes_and_is_not_vacuous() {
    let result = audit(
        &corpus(GREEN_BLOCK, "- Line 1 — everything exists."),
        &surfaces(),
    );
    assert!(
        result.passed(),
        "the GREEN control must pass; findings: {:#?}",
        result.findings
    );
    // A green that read nothing is the failure mode this gate exists to close.
    assert_eq!(result.blocks_read, 1, "the control block must be READ");
    assert_eq!(
        result.epics_in_roster, 1,
        "epic 99 is in-progress and must be in the roster"
    );
    assert!(
        result.tokens_resolved >= 5,
        "expected init/run/halt/kloc-check/registry-server to resolve, got {}",
        result.tokens_resolved
    );
}

// ───────────────────────────── vector 1: absent verb ─────────────────────────────

#[test]
fn planted_absent_verb_reds() {
    let block = GREEN_BLOCK.replace("maos init", "maos summon-dragon");
    let result = audit(
        &corpus(&block, "- Line 1 — everything exists."),
        &surfaces(),
    );
    assert!(!result.passed(), "an absent verb must red");
    assert!(
        kinds(&result).contains(&FindingKind::UnresolvedVerb),
        "expected UnresolvedVerb, got {:#?}",
        result.findings
    );
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.detail.contains("summon-dragon")),
        "the finding must NAME the offending verb: {:#?}",
        result.findings
    );
}

// ───────────────────────────── vector 2: unrecognised token ─────────────────────────────

#[test]
fn unrecognised_token_reds_instead_of_being_skipped() {
    let block = format!("{GREEN_BLOCK}\nkubectl apply -f whatever.yaml");
    let result = audit(
        &corpus(&block, "- Line 1 — everything exists."),
        &surfaces(),
    );
    assert!(
        !result.passed(),
        "an unrecognised head must be a FINDING, never a silent skip"
    );
    assert!(
        kinds(&result).contains(&FindingKind::UnrecognisedToken),
        "expected UnrecognisedToken, got {:#?}",
        result.findings
    );
}

// ───────────────────────────── vector 3: bogus one-shot mode ─────────────────────────────

#[test]
fn bogus_one_shot_mode_reds_even_though_bare_maos_is_legal() {
    // Stripping the prefix leaves a bare `maos`, which is a LEGAL verb-less
    // invocation (shell mode, exit 0). That is exactly why the mode must be
    // RESOLVED rather than stripped: otherwise the gate reports green while the
    // real command is an undeclared one-shot mode.
    let block = GREEN_BLOCK.replace("MAOS_ONE_SHOT=registry-server", "MAOS_ONE_SHOT=not-a-mode");
    let result = audit(
        &corpus(&block, "- Line 1 — everything exists."),
        &surfaces(),
    );
    assert!(!result.passed(), "an undeclared one-shot mode must red");
    assert!(
        kinds(&result).contains(&FindingKind::UnresolvedOneShotMode),
        "expected UnresolvedOneShotMode, got {:#?}",
        result.findings
    );
}

#[test]
fn bare_maos_with_a_declared_mode_is_green() {
    // The GREEN half of vector 3: the same shape with a mode that EXISTS must
    // pass, so the red above cannot be an artefact of the bare-`maos` shape.
    let result = audit(
        &corpus(
            "MAOS_ONE_SHOT=registry-server maos &",
            "- Line 1 — the mode exists.",
        ),
        &surfaces(),
    );
    assert!(
        result.passed(),
        "a declared one-shot mode on a bare `maos` must pass; findings: {:#?}",
        result.findings
    );
    assert_eq!(result.tokens_resolved, 1);
}

// ───────────────────────────── vector 4: owed by a DONE story ─────────────────────────────

#[test]
fn resolve_or_be_named_accepts_a_token_owed_by_an_open_story() {
    // The GREEN half of vector 4. `maos teleport` does not exist, but the
    // provenance names an OPEN story and that story's own file claims the token.
    let mut c = corpus(
        "maos teleport --now",
        "- Line 1 — `maos teleport` created by 99-1 AC2.",
    );
    c.story_files.insert(
        "99-1-owner".to_string(),
        "AC2 adds the `maos teleport` verb.".to_string(),
    );
    let result = audit(&c, &surfaces());
    assert!(
        result.passed(),
        "a token owed by a named OPEN story that claims it must pass; findings: {:#?}",
        result.findings
    );
    assert_eq!(result.owed.len(), 1, "the token must be recorded as OWED");
    assert_eq!(result.owed[0].token, "teleport");
    assert_eq!(result.owed[0].owner_story, "99-1-owner");
}

#[test]
fn resolve_or_be_named_reds_when_the_owning_story_is_done() {
    // An owed token whose owner already SHIPPED is not owed — it is missing.
    let mut c = corpus(
        "maos teleport --now",
        "- Line 1 — `maos teleport` created by 99-1 AC2.",
    );
    c.statuses
        .insert("99-1-owner".to_string(), "done".to_string());
    c.story_files.insert(
        "99-1-owner".to_string(),
        "AC2 adds the `maos teleport` verb.".to_string(),
    );
    let result = audit(&c, &surfaces());
    assert!(
        !result.passed(),
        "a token owed by a DONE story must red — the story shipped without it"
    );
    assert!(
        result.owed.is_empty(),
        "a done owner cannot create an owed token"
    );
    assert!(kinds(&result).contains(&FindingKind::UnresolvedVerb));
}

#[test]
fn resolve_or_be_named_reds_when_the_owner_never_claims_the_token() {
    // F14's strengthening. Without it this vector PASSES and the gate merely
    // proves that a story key exists in a YAML file.
    let mut c = corpus(
        "maos summon-dragon --now",
        "- Line 1 — `maos summon-dragon` created by 99-1 AC2.",
    );
    c.story_files.insert(
        "99-1-owner".to_string(),
        "AC2 adds something else entirely.".to_string(),
    );
    let result = audit(&c, &surfaces());
    assert!(
        !result.passed(),
        "a provenance receipt whose owner never names the token must red"
    );
    assert!(result.owed.is_empty());
    assert!(kinds(&result).contains(&FindingKind::UnresolvedVerb));
}

// ───────────────────────────── vector 5: the roster ─────────────────────────────

#[test]
fn two_exit_block_files_for_one_epic_reds() {
    let mut c = corpus(GREEN_BLOCK, "- Line 1 — everything exists.");
    c.epics.get_mut(&99).expect("epic 99").push(EpicFile {
        name: "epic-99-preflight.md".to_string(),
        content: epic_doc(GREEN_BLOCK, "- Line 1 — a SECOND block."),
    });
    let result = audit(&c, &surfaces());
    assert!(
        !result.passed(),
        "two exit-block files for one epic must red"
    );
    assert!(
        kinds(&result).contains(&FindingKind::RosterMultiple),
        "expected RosterMultiple, got {:#?}",
        result.findings
    );
}

#[test]
fn extra_files_without_a_block_are_not_a_finding() {
    // The GREEN half of vector 5, and the measured shape of `epic-15-*`: three
    // files match the glob, two are preflights with no exit block. Exactly one
    // carrier is correct, so the extra files must NOT red.
    let mut c = corpus(GREEN_BLOCK, "- Line 1 — everything exists.");
    c.epics.get_mut(&99).expect("epic 99").push(EpicFile {
        name: "epic-99-preflight.md".to_string(),
        content: "# a preflight with no exit block\n".to_string(),
    });
    let result = audit(&c, &surfaces());
    assert!(
        result.passed(),
        "extra non-carrier files must not red; findings: {:#?}",
        result.findings
    );
    assert_eq!(result.blocks_read, 1);
}

#[test]
fn a_non_done_epic_with_no_exit_block_reds() {
    // The defect a glob-and-skip corpus passes silently.
    let c = Corpus {
        epics: [(
            99,
            vec![EpicFile {
                name: "epic-99-fixture.md".to_string(),
                content: "# an epic that LOST its exit block\n".to_string(),
            }],
        )]
        .into_iter()
        .collect(),
        statuses: statuses(&[("epic-99", "in-progress")]),
        story_files: BTreeMap::new(),
    };
    let result = audit(&c, &surfaces());
    assert!(!result.passed(), "a non-done epic with no block must red");
    assert!(
        kinds(&result).contains(&FindingKind::RosterZero),
        "expected RosterZero, got {:#?}",
        result.findings
    );
}

// ───────────────────────────── vector 6: the placeholder hazard ─────────────────────────────

#[test]
fn unquoted_placeholder_reds_as_a_redirection_hazard() {
    let block = "maosctl halt resolve <halt_id> --spirit hello-spirit --kind provided-context";
    let extra = "- Line 1 — the harness substitutes <halt_id> from `halt list`.";
    let result = audit(&corpus(block, extra), &surfaces());
    assert!(
        !result.passed(),
        "an UNQUOTED <placeholder> is two bash redirections and must red"
    );
    assert!(
        kinds(&result).contains(&FindingKind::PlaceholderHazard),
        "expected PlaceholderHazard, got {:#?}",
        result.findings
    );
}

#[test]
fn quoted_placeholder_is_green() {
    // The GREEN half: quoting makes bash treat it as a literal word, so the
    // required flag is no longer swallowed by `> --spirit`.
    let block = "maosctl halt resolve \"<halt_id>\" --spirit hello-spirit --kind provided-context";
    let extra = "- Line 1 — the harness substitutes <halt_id> from `halt list`.";
    let result = audit(&corpus(block, extra), &surfaces());
    assert!(
        result.passed(),
        "a quoted placeholder must pass; findings: {:#?}",
        result.findings
    );
}

// ───────────────────────────── vector 7: the counters ─────────────────────────────

#[test]
fn an_empty_corpus_reds_rather_than_passing_vacuously() {
    // `findings.is_empty()` cannot tell "everything resolved" from "the glob
    // matched nothing", which is why the counters hard-fail at zero.
    let result = audit(&Corpus::default(), &surfaces());
    assert!(!result.passed(), "an empty corpus must NOT pass");
    assert_eq!(result.blocks_read, 0);
    assert_eq!(result.tokens_resolved, 0);
    assert_eq!(
        kinds(&result)
            .iter()
            .filter(|k| **k == FindingKind::VacuousCorpus)
            .count(),
        2,
        "both counters must report: zero blocks AND zero tokens"
    );
}

#[test]
fn done_epics_are_out_of_the_roster() {
    // Epics 0–14 predate the exit-block format and are all `done`; pulling them
    // into the roster would red the gate on history it cannot fix.
    let mut c = corpus(GREEN_BLOCK, "- Line 1 — everything exists.");
    c.statuses.insert("epic-99".to_string(), "done".to_string());
    let result = audit(&c, &surfaces());
    assert_eq!(
        result.epics_in_roster, 0,
        "a done epic is out of the roster"
    );
    // With nothing in the roster the vacuity guard fires — which is correct:
    // the gate refuses to report green over a corpus it read nothing from.
    assert!(!result.passed());
    assert!(kinds(&result).contains(&FindingKind::VacuousCorpus));
}

// ───────────────────────────── the tokeniser's measured shapes ─────────────────────────────

#[test]
fn command_substitution_is_matched_before_separator_splitting() {
    // epic-20 line 7, measured: the `$( )` contains a PIPE. Splitting on
    // separators first re-heads the command to `tr` (reproduced: a naive pass
    // reports head `tr`; the real head is `maos-spirit`).
    let commands = xtask::check_exit_commands::tokenise_line(
        7,
        "MAOS_SPIRIT_SIGNING_KEY=$(od -An -tx1 -N32 /dev/urandom | tr -d ' \\n') maos run --once",
    )
    .expect("the measured command is balanced");
    let heads: Vec<&str> = commands.iter().map(|c| c.head.as_str()).collect();
    assert!(
        heads.contains(&"maos"),
        "the outer head must survive the pipe inside `$( )`, got {heads:?}"
    );
    assert!(
        heads.contains(&"od") && heads.contains(&"tr"),
        "the substituted commands are commands too, got {heads:?}"
    );
}

#[test]
fn a_glued_subshell_paren_does_not_become_the_head() {
    // epic-20 line 6, measured: `do (exec 3<>/dev/tcp/…)` glues `(` to `exec`,
    // and `3<>` is a redirection rather than an assignment.
    let commands = xtask::check_exit_commands::tokenise_line(
        6,
        "for i in $(seq 1 50); do (exec 3<>/dev/tcp/127.0.0.1/6789) 2>/dev/null && break; sleep 0.1; done",
    )
    .expect("the measured command is balanced");
    let heads: Vec<&str> = commands.iter().map(|c| c.head.as_str()).collect();
    assert!(
        heads.contains(&"exec"),
        "the head must be `exec`, not `(exec`, got {heads:?}"
    );
    assert!(
        !heads.iter().any(|h| h.starts_with('(')),
        "no head may keep a glued paren, got {heads:?}"
    );
}

#[test]
fn whole_line_comments_carry_no_tokens() {
    // Five of epic-19's six block lines are WHOLE-LINE comments; AC3's original
    // clause covered only trailing ones.
    assert!(xtask::check_exit_commands::tokenise_line(
        2,
        "# hermetic is demo-j1's DEFAULT (no --live-codex; there is NO --replay flag)"
    )
    .expect("comments are balanced")
    .is_empty());
}

#[test]
fn a_hash_inside_quotes_is_not_a_comment() {
    let commands = xtask::check_exit_commands::tokenise_line(1, "maos run --text \"a # b\" --once")
        .expect("the quoted hash is balanced");
    assert_eq!(commands.len(), 1);
    assert!(
        commands[0].args.iter().any(|a| a.contains("a # b")),
        "the quoted hash must survive: {:?}",
        commands[0].args
    );
}

#[test]
fn cargo_run_p_xtask_resolves_the_verb_after_the_double_dash() {
    // The shape epics 15, 19 and 21 use for every xtask token.
    let result = audit(
        &corpus("cargo run -p xtask -- not-a-verb", "- Line 1 — nope."),
        &surfaces(),
    );
    assert!(!result.passed(), "an absent xtask verb must red");
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.detail.contains("not-a-verb")),
        "the finding must name the xtask verb: {:#?}",
        result.findings
    );
}

#[test]
fn a_plain_cargo_invocation_is_declared_not_resolved() {
    // `cargo test --workspace --no-fail-fast` (epic-15 line 1) and
    // `cargo build -p maos-bin … --bin maos` (epic-17 line 3) are cargo's own
    // surface, not ours; neither may red and neither may count as resolved.
    let result = audit(
        &corpus(
            "maos init && cargo test --workspace --no-fail-fast\ncargo build -p maos-bin --bin maos",
            "- Line 1 — cargo built-ins.",
        ),
        &surfaces(),
    );
    assert!(
        result.passed(),
        "plain cargo invocations must not red; findings: {:#?}",
        result.findings
    );
    assert_eq!(
        result.tokens_resolved, 1,
        "only `maos init` resolves; cargo's own verbs are declared non-commands"
    );
}

#[test]
fn sprint_status_epic_without_any_candidate_file_reds() {
    let mut c = corpus(GREEN_BLOCK, "- Line 1 — everything exists.");
    c.epics.clear();
    let result = audit(&c, &surfaces());
    assert!(!result.passed());
    assert!(kinds(&result).contains(&FindingKind::RosterZero));
}

#[test]
fn nested_and_secondary_cli_commands_are_resolved_at_every_level() {
    let nested = audit(
        &corpus(
            "maosctl spirit install example@1.0",
            "- Line 1 — no owner claims install.",
        ),
        &surfaces(),
    );
    assert!(!nested.passed());
    assert!(nested
        .findings
        .iter()
        .any(|finding| finding.detail.contains("maosctl spirit install")));

    let mut secondary_surfaces = surfaces();
    secondary_surfaces
        .present_binaries
        .insert("maos-spirit".to_string());
    let secondary = audit(
        &corpus("maos-spirit teleport", "- Line 1 — no such command."),
        &secondary_surfaces,
    );
    assert!(!secondary.passed());
    assert!(secondary
        .findings
        .iter()
        .any(|finding| finding.detail.contains("maos-spirit teleport")));
}

#[test]
fn owed_token_binds_to_the_exact_exit_line_and_full_command_path() {
    let mut c = corpus(
        "maosctl install package@1.0\nmaosctl spirit install example@1.0",
        "- Line 1 — `maosctl install` is added by 99-1 AC1.\n\
         - Line 2 — `maosctl spirit install` is added by 99-2 AC1.",
    );
    c.statuses
        .insert("99-1-package-installer".to_string(), "backlog".to_string());
    c.statuses
        .insert("99-2-spirit-installer".to_string(), "backlog".to_string());
    c.story_files.insert(
        "99-1-package-installer".to_string(),
        "AC1 creates the maosctl install command.".to_string(),
    );
    c.story_files.insert(
        "99-2-spirit-installer".to_string(),
        "AC1 creates the maosctl spirit install command.".to_string(),
    );
    let result = audit(&c, &surfaces());
    assert!(
        result.passed(),
        "the exact line and command owner should make only the nested command owed: {:#?}",
        result.findings
    );
    assert_eq!(result.owed.len(), 1);
    assert_eq!(result.owed[0].token, "install");
    assert_eq!(result.owed[0].owner_story, "99-2-spirit-installer");
}

#[test]
fn owner_claim_requires_token_boundaries() {
    let mut c = corpus("maos eval", "- Line 1 — `maos eval` created by 99-1 AC2.");
    c.story_files.insert(
        "99-1-owner".to_string(),
        "AC2 improves evaluation only.".to_string(),
    );
    let result = audit(&c, &surfaces());
    assert!(!result.passed());
    assert!(result.owed.is_empty());
    assert!(kinds(&result).contains(&FindingKind::UnresolvedVerb));
}

#[test]
fn quoted_placeholder_still_requires_a_declaration() {
    let result = audit(
        &corpus(
            "maosctl halt list \"<halt_id>\"",
            "- Line 1 — no substitution declaration.",
        ),
        &surfaces(),
    );
    assert!(!result.passed());
    assert!(kinds(&result).contains(&FindingKind::PlaceholderHazard));
}

#[test]
fn malformed_shell_boundaries_red_instead_of_resolving() {
    for command in ["maos init \"unterminated", "maos init $(printf broken"] {
        let result = audit(&corpus(command, "- Line 1 — malformed."), &surfaces());
        assert!(!result.passed(), "{command:?} must red");
        assert!(
            kinds(&result).contains(&FindingKind::MalformedCommand),
            "{command:?}: {:#?}",
            result.findings
        );
    }
}

#[test]
fn exit_block_without_a_closing_fence_reds() {
    let c = Corpus {
        epics: [(
            99,
            vec![EpicFile {
                name: "epic-99-fixture.md".to_string(),
                content: "# fixture\n\n**Hermetic exit command:**\n\n```\nmaos init\n".to_string(),
            }],
        )]
        .into_iter()
        .collect(),
        statuses: statuses(&[("epic-99", "in-progress")]),
        story_files: BTreeMap::new(),
    };
    let result = audit(&c, &surfaces());
    assert!(!result.passed());
    assert!(kinds(&result).contains(&FindingKind::ExitBlockMissing));
}

#[test]
fn env_wrapper_does_not_hide_its_child_command() {
    let bogus_mode = audit(
        &corpus(
            "env MAOS_ONE_SHOT=not-a-mode maos",
            "- Line 1 — no such mode.",
        ),
        &surfaces(),
    );
    assert!(!bogus_mode.passed());
    assert!(kinds(&bogus_mode).contains(&FindingKind::UnresolvedOneShotMode));

    let bogus_verb = audit(
        &corpus("env maos teleport", "- Line 1 — no such verb."),
        &surfaces(),
    );
    assert!(!bogus_verb.passed());
    assert!(kinds(&bogus_verb).contains(&FindingKind::UnresolvedVerb));
}

#[test]
fn unreadable_required_help_surface_is_a_finding() {
    let mut unavailable = surfaces();
    unavailable
        .surface_failures
        .push("maos-spirit --help timed out".to_string());
    let result = audit(
        &corpus(GREEN_BLOCK, "- Line 1 — everything exists."),
        &unavailable,
    );
    assert!(!result.passed());
    assert!(kinds(&result).contains(&FindingKind::SurfaceUnavailable));
}

#[test]
fn unreadable_planned_binary_is_owed_by_its_exact_exit_line() {
    let mut c = corpus(
        "maos init\nmaos-registry-server --bind 127.0.0.1:0 --root /tmp/maos",
        "- Line 1 — `maos init` exists.\n\
         - Line 2 — `maos-registry-server` is added by 20-1 AC1.",
    );
    c.statuses.insert(
        "20-1-local-registry-server".to_string(),
        "backlog".to_string(),
    );
    c.story_files.insert(
        "20-1-local-registry-server".to_string(),
        "AC1 creates the maos-registry-server command.".to_string(),
    );
    let mut unavailable = surfaces();
    unavailable.unavailable_binaries.insert(
        "maos-registry-server".to_string(),
        "exited with status 1".to_string(),
    );

    let result = audit(&c, &unavailable);
    assert!(
        result.passed(),
        "the present but unimplemented binary must remain explicitly owed: {:#?}",
        result.findings
    );
    assert_eq!(result.tokens_resolved, 1);
    assert_eq!(result.owed.len(), 1);
    assert_eq!(result.owed[0].token, "maos-registry-server");
    assert_eq!(result.owed[0].owner_story, "20-1-local-registry-server");
}

#[test]
fn unreadable_unowned_binary_is_a_surface_finding() {
    let mut unavailable = surfaces();
    unavailable.unavailable_binaries.insert(
        "maos-registry-server".to_string(),
        "exited with status 1".to_string(),
    );
    let result = audit(
        &corpus(
            "maos init\nmaos-registry-server --bind 127.0.0.1:0 --root /tmp/maos",
            "- Line 1 — `maos init` exists.\n\
             - Line 2 — no story owns `maos-registry-server`.",
        ),
        &unavailable,
    );
    assert!(!result.passed());
    assert!(kinds(&result).contains(&FindingKind::SurfaceUnavailable));
}
