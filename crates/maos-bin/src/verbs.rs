//! Story 15-3 (AC4; rulings F5, F6, F13) — the single verb table of the
//! `maos` binary.
//!
//! F6 (lookup-then-dispatch): BOTH `main()`s of `main.rs` resolve argv through
//! [`dispatch`], which indexes [`VERBS`], and then dispatch on the resolved
//! [`VerbName`]. A verb known to a dispatcher but absent from the table is
//! therefore *unrepresentable* rather than merely tested — the dispatcher arms
//! are keyed on table rows, never on raw string literals. This module is
//! deliberately self-contained (std only, no `crate::` paths) so that
//! `crates/maos-bin/tests/verb_table_15_3.rs` can compile the very same file
//! by `#[path]` and hold the table equal to the dispatchable surface in both
//! directions.
//!
//! F5: `--help`/`-h`/`help` and `--version`/`-V` are answered from this table
//! in BOTH builds. At the story's baseline the network build hung (EXIT 124,
//! daemon boot) on all four.
//!
//! The stdout shape of `maos --help` is FROZEN by the `check-exit-commands`
//! contract: a `VERBS:` section header, one entry per verb (exactly two
//! leading spaces; the first whitespace-delimited token is the verb; build
//! annotation in brackets), then a `MAOS_ONE_SHOT MODES:` section header, one
//! entry per one-shot mode. Every verb of BOTH builds is listed in BOTH
//! builds' output, annotated — both builds ship (Epic 20 runs an air-gap
//! matrix leg), so the gate resolves a token if the table lists it at all.

/// A build flavor of the `maos` binary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Build {
    /// `--no-default-features` — the network surface is compiled out (R-AG2).
    AirGap,
    /// The default build — full network surface.
    Network,
}

impl Build {
    /// The annotation token used in the frozen `maos --help` table.
    pub fn label(self) -> &'static str {
        match self {
            Build::AirGap => "air-gap",
            Build::Network => "network",
        }
    }
}

/// The build this binary (or test harness) was compiled as. Feature-gated
/// dispatch keys off this; the table itself is feature-independent.
pub const CURRENT_BUILD: Build = if cfg!(feature = "network") {
    Build::Network
} else {
    Build::AirGap
};

/// Identity of a dispatchable verb. The set is closed on purpose: adding a
/// variant forces a compile error in every exhaustive `match` over it — both
/// `main()`s of `main.rs` and `tests/verb_table_15_3.rs` match without a `_`
/// arm, so "a dispatcher arm without a table row" cannot compile silently
/// (the direction-(a) tripwire of T10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerbName {
    Init,
    Run,
    Shell,
    Audit,
    Traceback,
    Backup,
    Install,
    Purge,
}

/// One row of the verb table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verb {
    /// Identity the dispatcher arms in `main.rs` match on.
    pub name: VerbName,
    /// The argv token that selects this verb.
    pub argv: &'static str,
    /// The builds whose binary dispatches this verb (F6). A build not listed
    /// here rejects the token as unknown: `backup`/`install` are air-gap-only,
    /// `shell`/`traceback` are network-only, and `purge` ships in both.
    pub builds: &'static [Build],
    /// One-line summary rendered in the help table.
    pub summary: &'static str,
}

/// THE verb table (F6). `--help` renders from it and both dispatchers index
/// it; `tests/verb_table_15_3.rs` asserts it equals the dispatchable surface
/// in both directions and in both feature sets.
pub const VERBS: &[Verb] = &[
    Verb {
        name: VerbName::Init,
        argv: "init",
        builds: &[Build::AirGap, Build::Network],
        summary: "Initialise a MAOS home",
    },
    Verb {
        name: VerbName::Run,
        argv: "run",
        builds: &[Build::AirGap, Build::Network],
        summary: "Run a worker",
    },
    Verb {
        name: VerbName::Shell,
        argv: "shell",
        builds: &[Build::Network],
        summary: "Interactive shell",
    },
    Verb {
        name: VerbName::Audit,
        argv: "audit",
        builds: &[Build::AirGap, Build::Network],
        summary: "Query the audit Transparency Log",
    },
    Verb {
        name: VerbName::Traceback,
        argv: "traceback",
        builds: &[Build::Network],
        summary: "Cross-wall log recall",
    },
    Verb {
        name: VerbName::Backup,
        argv: "backup",
        builds: &[Build::AirGap],
        summary: "Create, verify, or restore a TL backup",
    },
    Verb {
        name: VerbName::Install,
        argv: "install",
        builds: &[Build::AirGap],
        summary: "Install a verified release artifact",
    },
    Verb {
        name: VerbName::Purge,
        argv: "purge",
        builds: &[Build::AirGap, Build::Network],
        summary: "Remove MAOS-owned local state",
    },
];

/// Tokens that render the help table. Meta-flags, not verb rows: the frozen
/// contract's verb table lists verbs only, and `grep -c 'Some("'` inside each
/// `main()` must stay 0 (AC4), so these live here next to the lookup.
pub const HELP_FLAGS: &[&str] = &["--help", "-h", "help"];

/// Tokens that render the version line (F5: answered in BOTH builds).
pub const VERSION_FLAGS: &[&str] = &["--version", "-V"];

/// Every `MAOS_ONE_SHOT=<mode>` mode the composition root dispatches (the
/// one-shot block of `main.rs`'s network `main`). Story 16-1 (D-16-1-A)
/// removed the 16 door/read modes (pause, resume, start, stop, unload,
/// posture-shift, halt-list, halt-resolve, orchestrator-queue,
/// orchestrator-status, revoke-token, revocations-import, revocations-list,
/// hot-swap-precheck, spirit-upgrade, legal-hold-list): they reach the
/// running daemon through the operator door or run as in-process maosctl
/// readers, so `MAOS_ONE_SHOT` dispatches **32** distinct modes — re-measured
/// against the one-shot block. This is the list the `check-exit-commands`
/// gate resolves `MAOS_ONE_SHOT=<mode> maos` against.
pub const MAOS_ONE_SHOT_MODES: &[&str] = &[
    "acp-server",
    "bench-section-13-1",
    "cohort-a2a-daemon",
    "collective-erase",
    "forget",
    "governance-admit",
    "hello-spirit",
    "legal-hold-release",
    "registry-server",
    "smoke-a2a-consent-vocab-8-7",
    "smoke-a2a-fail-closed-8-8",
    "smoke-a2a-loopback-6-3",
    "smoke-a2a-tcp-8-6",
    "smoke-abi-7-5a",
    "smoke-bench-5e",
    "smoke-compliance-7-3",
    "smoke-discipline-7-1-5",
    "smoke-epic-4",
    "smoke-import-7-2",
    "smoke-mcp-acp-5",
    "smoke-multi-provider-5",
    "smoke-orchestrator-fanout-6-2",
    "smoke-registry-5d",
    "smoke-registry-7-2",
    "smoke-schedule-6-4",
    "smoke-skill-7-4",
    "smoke-spirit-5",
    "smoke-spirit-author-7-1",
    "smoke-supervision-5",
    "smoke-t3-sandbox-5",
    "smoke-upgrade-revoke-5",
    "uninstall",
];

/// A mode resolved through [`MAOS_ONE_SHOT_MODES`].
///
/// The network composition root accepts this type, not an unchecked env
/// string. A dispatch branch added without a table row is therefore
/// unreachable; a table row without a branch reaches the composition root's
/// fail-closed "registered but undispatched" diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OneShotMode(&'static str);

impl OneShotMode {
    /// ⚠ No longer called by the composition root. Story 16-1 deleted the
    /// twelve fake one-shot arms whose `match mode.as_str()` blocks were its
    /// only production callers; every SURVIVING arm compares the mode
    /// directly through [`PartialEq<&str>`] below.
    ///
    /// Kept because `crates/maos-bin/tests/verb_table_15_3.rs` includes this
    /// file with `#[path]` and reads the resolved mode back out as a string —
    /// the accessor is live there, and `dead_code` cannot see across the
    /// include.
    #[allow(dead_code)]
    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl PartialEq<&str> for OneShotMode {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl std::fmt::Display for OneShotMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

/// Resolve an environment-provided one-shot discriminator through the single
/// help/gate/runtime table before the composition root may dispatch it.
pub fn resolve_one_shot_mode(requested: &str) -> Option<OneShotMode> {
    MAOS_ONE_SHOT_MODES
        .iter()
        .copied()
        .find(|known| *known == requested)
        .map(OneShotMode)
}

/// Outcome of resolving argv against the table (F6 lookup-then-dispatch).
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A help flag: the caller renders via [`print_help`] and exits 0.
    Help,
    /// A version flag: the caller renders via [`print_version`] and exits 0.
    Version,
    /// No arguments: the caller applies its own no-argument behaviour.
    NoArgs,
    /// A first token that is no verb of this build. The caller diagnoses on
    /// stderr, prints the table, and exits non-zero instead of hanging.
    Unknown(String),
    /// A verb of this build: the caller dispatches on `Verb::name`.
    Verb(&'static Verb),
}

/// Resolve `argv` against [`VERBS`] for the compile-time build. BOTH
/// `main()`s call this before producing any output, so `maos --help` emits
/// nothing else (no scaffold banner) and an unknown verb exits instead of
/// booting the daemon.
pub fn dispatch(argv: &[String]) -> Outcome {
    let Some(first) = argv.first() else {
        return Outcome::NoArgs;
    };
    if HELP_FLAGS.contains(&first.as_str()) {
        return Outcome::Help;
    }
    if VERSION_FLAGS.contains(&first.as_str()) {
        return Outcome::Version;
    }
    match lookup(first, CURRENT_BUILD) {
        Some(verb) => Outcome::Verb(verb),
        None => Outcome::Unknown(first.clone()),
    }
}

/// Table lookup narrowed to one build. A token is a verb of `build` iff a row
/// lists it AND that row's `builds` contains `build`.
fn lookup(token: &str, build: Build) -> Option<&'static Verb> {
    VERBS
        .iter()
        .find(|verb| verb.argv == token && verb.builds.contains(&build))
}

/// The bracketed build annotation for one row, e.g. `[air-gap,network]`.
fn builds_annotation(builds: &[Build]) -> String {
    let labels: Vec<&str> = builds.iter().map(|build| build.label()).collect();
    format!("[{}]", labels.join(","))
}

/// Write one two-space-indented verb row per table entry — no section
/// headers. `print_air_gap_usage` in `main.rs` composes its human usage page
/// from this, so the rows render from the table and never from a second copy
/// of it (AC4).
pub fn print_verb_rows(out: &mut dyn std::io::Write) {
    for verb in VERBS {
        let _ = writeln!(
            out,
            "  {:<14}{:<17}  {}",
            verb.argv,
            builds_annotation(verb.builds),
            verb.summary
        );
    }
}

/// Render the FROZEN `maos --help` stdout contract: the `VERBS:` section,
/// then the `MAOS_ONE_SHOT MODES:` section. Every verb of both builds is
/// listed, annotated; nothing precedes the table.
pub fn print_help(out: &mut dyn std::io::Write) {
    let _ = writeln!(out, "VERBS:");
    print_verb_rows(out);
    let _ = writeln!(out, "MAOS_ONE_SHOT MODES:");
    for mode in MAOS_ONE_SHOT_MODES {
        let _ = writeln!(out, "  {mode}");
    }
}

/// Render the version line (F5): `maos <version>` on stdout.
pub fn print_version(out: &mut dyn std::io::Write) {
    let _ = writeln!(out, "maos {}", env!("CARGO_PKG_VERSION"));
}
