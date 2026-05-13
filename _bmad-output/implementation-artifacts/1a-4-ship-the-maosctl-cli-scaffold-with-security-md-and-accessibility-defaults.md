# Story 1a.4: Ship the maosctl CLI Scaffold with SECURITY.md and Accessibility Defaults

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As **the v0.1-α evaluator-path owner who must seat the operator surface and the security-disclosure pipeline BEFORE any external Spirit ships AND BEFORE the J0 evaluator (5-minute install-to-first-response, per §10.1 and NFR-Onb-2) has a CLI to type into**,
I want **a `maosctl` binary scaffold compiled out of `crates/maos-cli/` (per architecture §4.0.2's canonical 17-crate layout `maos-cli/` → maosctl) with six v0.1 subcommand stubs declared (`install`, `start`, `stop`, `unload`, `run`, `audit` — per the epic's binding AC list, NOT the looser four-verb list in the "Owns" line) under a `clap` 4.5 derive-driven command tree, three accessibility surfaces unified into a single `ColorChoice` resolver (precedence: `--plain` > `NO_COLOR` env > `TERM=dumb` env > terminal-isatty default per NFR-Ops-5), a `SECURITY.md` file at the repo root carrying the four NFR-Ops-4 binding sections (disclosure contact `security@maos.dev` with published GPG key fingerprint slot; 90-day coordinated-disclosure embargo window; supported-versions matrix for security backports; advisory-publication channel) — CNA-registration explicitly deferred to v0.5 per the PRD phase-split note — plus a new `cargo xtask check-security-md` gate that parses `SECURITY.md` for the four required headers and FAILS the build when any are missing (wired into `.github/workflows/discipline.yml` as a per-commit blocking gate alongside the existing 13 Epic-0 gates), `tests/coverage-matrix.yaml` rows flipped from `gates: []` to populated for FR1 (cargo install adds maosctl), FR2 (uninstall stub surface), FR7 (telemetry opt-in declared via `maosctl --telemetry=off` flag), FR61 (SECURITY.md → check-security-md gate), NFR-Ops-4 (same gate), NFR-Ops-5 (accessibility unit-test surface via `cargo test -p maos-cli`), and the dev record carrying the seven-subsection AC5 evidence block (pre-flight baseline / runtime smoke / shell-emptiness audit / surface-classification audit — N/A for `maos-cli` since it is OUTSIDE the kernel-core surface walk / dep-introduction note / "what did NOT happen" checklist / self-review checklist)**,
so that **(a) the J0 evaluator persona can type `maosctl --help`, `maosctl --version`, `maosctl install --help` etc. and see a coherent v0.1-α command tree (even though no subcommand body does anything beyond printing "not yet implemented at v0.1-α — landing at Story <X>"), satisfying the foundational FR1 + FR2 surface and unblocking the §10.1 acceptance criteria for v0.1-α; (b) the security disclosure pipeline EXISTS on day one (NFR-Ops-4 is `v0.1 ship gate`, not `v0.5+`) — when a researcher emails `security@maos.dev` BEFORE the first external Spirit ships, the response window / embargo / advisory channel are pre-declared instead of invented under pressure; (c) NFR-Ops-5 accessibility is mechanically verifiable from day one — visually-impaired evaluators using a screen reader (where ANSI escape codes get spoken as gibberish) can set `NO_COLOR=1` once in their shell and the entire `maosctl` surface complies, AND CI evaluators running under non-TTY terminals (`TERM=dumb`) get clean output automatically without flag-flipping; and (d) the founding-sprint baselines extend additively — `cargo build --locked` still completes, all 13 prior Epic-0 gates stay green, the 14th gate `check-security-md` joins them and the FIRST-CLASS J0 evaluator transcript can now be captured as a script (`cargo install --path crates/maos-bin --locked && cargo install --path crates/maos-cli --locked && maosctl --version && maosctl install --plain && cat SECURITY.md`)**.

### What this story is NOT

This story is **structural scaffolding only**. It must NOT smuggle runtime behavior into subcommand bodies, wire actual Spirit-load / start / stop / unload / run / audit semantics, or pretend a real install pipeline exists. Specifically:

1. **No subcommand body does real work.** Every subcommand at v0.1-α prints a deterministic "not-yet-implemented" message to stderr and exits with code `2` (POSIX "command found but cannot execute"), pointing the operator at the future story that lands the real body — e.g., `install` → Story 1b.5b, `audit` → Story 1b.5b (audit query specifically per FR4/FR42–44), `start`/`stop`/`unload`/`run` → Story 5.1 (lifecycle verbs) or Story 1b.5b (where the v0.1 subset lands per epic-1b). The dev agent MUST NOT add `spirit_loader.invoke(...)` or anything that touches the kernel — `maos-cli` does NOT depend on `maos-kernel-core` at v0.1-α (it only depends on `clap` and on `std`). The crate stays a pure CLI front-end.
2. **No new ADRs.** The 14 binding-v0.1 ADRs are committed (Story 1a.1). This story consumes NFR-Ops-4 (SECURITY.md content) / NFR-Ops-5 (accessibility) / FR1 / FR2 / FR7 / FR61 / §10.1 (J0 journey) / §4.0.2 (17-crate layout). It does NOT amend any ADR.
3. **No `SECURITY.md` CNA registration.** The PRD explicitly phase-splits: "CNA registration through MITRE moves to v0.5 (6–12 weeks elapsed paperwork; v0.1 just needs disclosure pipeline to exist)" [Source: NFR-Ops-4]. This story ships the four binding sections only; the "CVE-assignment process" reference in FR61 maps to the v0.5+ CNA story. SECURITY.md may carry a one-line "CVE assignment: under registration with MITRE; tracked at <issue-link> — landing at v0.5" placeholder.
4. **No `maos-bin` / `maos-kernel-core` touches.** The maos-bin composition root from Story 1a.2 (+ 1a.3 crypto-provider construct) stays untouched. The maosctl binary is INDEPENDENT — `crates/maos-cli/Cargo.toml` does NOT add `maos-bin` or `maos-kernel-core` as a path-dep. (At a future story when `maosctl` invokes the kernel out-of-process, it'll go through a `maos-control` HTTP API per §4.0.2's v0.5 plan; at v0.1-α it's a pure CLI surface.)
5. **No CryptoProvider / xtask kernel-surface changes.** Story 1a.3's `Arc<dyn CryptoProvider>` seam, `xtask/kernel-api-classes.toml` (32-row baseline), `docs/ci-baselines/kernel-surface-v0.1-alpha.json` are all UNTOUCHED. `maos-cli` is outside the kernel-core surface walk (only `maos_kernel_core::api::*` re-exports get classified), so this story adds ZERO rows to `kernel-api-classes.toml`.
6. **No `--plain`/`NO_COLOR` plumbing into `maos-bin`.** The accessibility surface lives in `maosctl` only at v0.1-α. The `maos-bin` startup-banner `eprintln!` calls from 1a.2/1a.3 stay as-is (plain ASCII, no ANSI escapes, no decision-making about color). Future stories may add color to `maos-bin`'s log output; that goes through the same `ColorChoice` plumbing, but the consolidation is a future-story concern.
7. **No new gate beyond `check-security-md`.** Some readers may be tempted to add `check-cli-help-text` or `check-accessibility-snapshot` gates "while we're touching the CLI." Do NOT. Each new CI gate is a maintenance liability. The `--plain` + `NO_COLOR` + `TERM=dumb` surface is covered by `cargo test -p maos-cli` unit tests (in-process — set env via `temp_env::with_var` or `std::env::set_var` inside `#[test]`-scoped guards); ANY rep test is run as `cargo test --workspace --locked` which is already in `discipline.yml`. No new gate.
8. **No `invariant-lock` touch.** This story does NOT modify any `docs/invariants/I*.md` file. The `invariant-lock` gate runs in "no-touch" mode (verify via `cargo run -p xtask -- invariant-lock --changed-files <this-PR's-files> --pr-number 0 --sha test` reporting zero touched invariants). If your diff *does* touch any invariant register file, **STOP** — that work belongs to a future Story.
9. **No GPG key generation.** SECURITY.md carries a **placeholder** GPG key fingerprint (with explicit `<TO-BE-PUBLISHED>` slot + a comment pointing operators to issue the real key as a separate action item). Generating, publishing, and rotating the actual `security@maos.dev` GPG key is an **operator action** outside scope of code review. Document the fingerprint slot in SECURITY.md and in the dev record's "what did NOT happen" subsection.
10. **No `cargo install --path crates/maos-bin` regression.** Story 1a.2 + 1a.3 left this passing. This story adds `cargo install --path crates/maos-cli --locked` as a SECOND install target. Both MUST pass; pre-flight runs both. The `cargo install` discipline lands the `maosctl` binary at `$CARGO_HOME/bin/maosctl`, same as 1a.2's `maos-bin`.

**Why the discipline matters here.** The Epic 0 retro flagged "spec-prose-vs-implementation drift" (DF11 — 200 corpus entries but only 11 unique patterns). The drift mode at 1a.4 would be: "`SECURITY.md` shipped but with placeholder text that fails its own gate; `maosctl --help` shipped but accessibility resolution unverified by a unit test; gate `check-security-md` shipped but only checks for the file's existence, not its four required headers." That is **not** what this story is. Every binding section in `SECURITY.md` is verified by a parser test against the four required H2/H3 headers (the gate parses markdown by header text, not by file presence); the three accessibility surfaces each get a dedicated unit test asserting the `ColorChoice` resolver returns `Never` under that surface; the install + version smoke runs end-to-end against a freshly-built `target/release/maosctl`. **The deliverable is the verified discipline, not the file count.**

### Critical preconditions (verify BEFORE opening the PR)

1. **Story 1a.3 is `done` and merged.** Verified: `sprint-status.yaml` shows `1a-3-cryptoprovider-trait-xtask-service-boundary-stub-implementation: done`; `epic-1a: in-progress`. The `CryptoProvider` trait, `RingCryptoProvider` adapter, `Arc<dyn CryptoProvider>` slot in `maos-bin/main.rs`, enriched `p1_p4_status` xtask payload, and 32-item baseline JSON MUST all be in place.
2. **All 13 Epic-0 gates + 14th surface-gate state are green on `main`.** Run the full local-CI suite (see the table in §AC5 below) as a baseline before any changes; document the pass list in the dev record's "Pre-flight baseline" subsection. Any pre-existing failure becomes a hard blocker for opening this story's PR. The pre-flight baseline command set:
   ```
   cargo build --locked --all-targets --workspace
   cargo test --workspace --locked
   cargo run -p xtask -- check-unsafe
   cargo run -p xtask -- check-empty-kernel
   cargo run -p xtask -- check-loom
   cargo run -p xtask -- check-service-boundary
   cargo run -p xtask -- kloc-check
   cargo run -p xtask -- abi-diff
   cargo run -p xtask -- check-corpus
   cargo run -p xtask -- check-judge-config
   cargo run -p xtask -- coverage-matrix
   cargo run -p xtask -- corpus-staleness
   cargo run -p xtask -- rebaseline-check
   cargo run -p xtask -- calibrate
   cargo run -p xtask -- invariant-lock --changed-files /dev/null --pr-number 0 --sha test
   cargo deny check
   ```
3. **`docs/dev-discipline/dep-introduction.md` discipline applies.** This story introduces **one** new top-level dependency in `crates/maos-cli/Cargo.toml` ONLY: `clap = { version = "4.5", features = ["derive"] }` (CLI parser; derive-macro for `#[derive(Parser)]` ergonomics; already present in `Cargo.lock` at 4.6.1 via `maos-corpus-gen` so the blast radius is **zero new lockfile entries**). The dev record's "Dependency-introduction note" MUST confirm zero new `Cargo.lock` blast (`git diff HEAD -- Cargo.lock | grep -c '^+name = ' → 0`). `cargo deny check` must pass.
4. **`cargo deny check` baseline passes.** Run `cargo deny check` on `main` before any changes; record PASS. clap's license is `MIT OR Apache-2.0` — already in `deny.toml [licenses] allow`. No license amendment needed.
5. **DF17 (multi-invariant `invariant-lock` fixture)** is **NOT** triggered by this story. This story touches zero `docs/invariants/I*.md` files; the `invariant-lock` gate runs in "no-touch" mode. Verify by running `cargo run -p xtask -- invariant-lock --changed-files <this-PR's-files> --pr-number 0 --sha test` and confirming the gate reports zero touched invariants.
6. **`crates/maos-cli/` is currently a 7-line placeholder.** Verified: `crates/maos-cli/src/lib.rs` is 9 lines (file-level docstring only, no items); `crates/maos-cli/Cargo.toml` has empty `[dependencies]`. This story REPLACES the placeholder with the maosctl scaffold; no risk of clobbering existing work since there is no existing work.
7. **`SECURITY.md` does not exist at repo root.** Verified by `test -f SECURITY.md; echo $?` → 1 at pre-flight. This story creates it. The previous 1a.3 retro explicitly excluded SECURITY.md ("No SECURITY.md. Story 1a.4 ships SECURITY.md").

### Size envelope

Expected production-Rust + docs + config footprint:

- **`crates/maos-cli/Cargo.toml` update:** ~5 LOC (add `[[bin]] name = "maosctl" path = "src/main.rs"` block; add `clap = { version = "4.5", features = ["derive"] }` to `[dependencies]`; remove the `description = "MAOS CLI — maosctl command-line interface (Story 1a.4)"` placeholder marker note — leave the description text unchanged).
- **`crates/maos-cli/src/lib.rs` rewrite:** ~50–80 LOC (replace the 9-line placeholder with a real `lib.rs` exposing `cli::Cli`, `cli::Subcommand`, `accessibility::ColorChoice`, and a single `run(args)` entry point that returns `ExitCode`; lib stays I/O-free except for stderr `eprintln!` in subcommand stubs — purely so the binary's `main.rs` is a 3-line thin shim).
- **`crates/maos-cli/src/cli.rs` new file:** ~80–120 LOC (the `#[derive(Parser)] struct Cli` + `#[derive(Subcommand)] enum Subcommand { Install(InstallArgs), Start(StartArgs), Stop(StopArgs), Unload(UnloadArgs), Run(RunArgs), Audit(AuditArgs) }` + 6 `*Args` structs each with a single placeholder field or `#[command(about = "...")]` annotation; `--plain` boolean flag at top-level; `--telemetry=on|off` enum flag at top-level per FR7).
- **`crates/maos-cli/src/accessibility.rs` new file:** ~60–100 LOC (`ColorChoice { Auto, Never, Always }` enum; `resolve(cli_plain: bool, env: &impl EnvProvider) -> ColorChoice` function implementing the precedence cascade; `EnvProvider` trait + `RealEnv` + `MockEnv` for testability; 6+ unit tests for the precedence rules).
- **`crates/maos-cli/src/subcommands.rs` new file:** ~40–80 LOC (6 stub functions `install_run(args)` ... `audit_run(args)` each emitting `eprintln!("maosctl: <subcmd> not yet implemented at v0.1-α — landing at Story <X>")` and returning `ExitCode::from(2)`).
- **`crates/maos-cli/src/main.rs` new file:** ~15–25 LOC (thin shim: `fn main() -> ExitCode { maos_cli::run(std::env::args_os()) }`).
- **`SECURITY.md` (root) new file:** ~60–110 LOC of markdown (four H2 sections: `## Reporting a vulnerability` / `## Coordinated-disclosure window` / `## Supported versions` / `## Advisory channel`; GPG key fingerprint slot with `<TO-BE-PUBLISHED>` marker + issue link; supported-versions table; embargo-window prose).
- **`xtask/src/check_security_md.rs` new file:** ~80–140 LOC (one new file: `check_security_md(workspace_root: &Path) -> Result<Report, Error>`; reads `SECURITY.md`, parses with `pulldown-cmark` if already in deps OR with a stdlib `lines().filter()` header scan, asserts the four required H2 headers are present, returns structured `Report { passed: bool, missing_sections: Vec<&'static str>, present_sections: Vec<&'static str> }`).
- **`xtask/src/main.rs` update:** ~6–10 LOC (add `CheckSecurityMd` enum variant + match arm dispatching to `check_security_md::check_security_md(...)`).
- **`xtask/src/tests/check_security_md_tests.rs` new file:** ~80–120 LOC (4–6 unit tests: passes when all 4 headers present; fails when each individual header missing; fails when file absent; fails when headers are not at the H2 level; passes when extra sections exist after the four required ones).
- **`xtask/Cargo.toml` update (CONDITIONAL):** 0 LOC if existing markdown parsing in xtask suffices; ~1 LOC if `pulldown-cmark` needs adding. Default: header scan via `str.starts_with("## ")` line iteration — no new dep.
- **`.github/workflows/discipline.yml` update:** ~6–10 LOC (add a new step `check-security-md` invoking `cargo run -p xtask -- check-security-md` in the existing per-commit gate matrix; gate is `required` — fails PR if SECURITY.md is missing or malformed).
- **`xtask/gate-registry.toml` update:** ~3–6 LOC (add `[gates.check-security-md]` entry with phase=`v0.1-α`, blocking=true, owner=`Story 1a.4` per the existing gate-registry shape).
- **`tests/coverage-matrix.yaml` row flips:** ~10–18 LOC across 6 rows (FR1 / FR2 / FR7 / FR61 / NFR-Ops-4 / NFR-Ops-5) — add `gates:` entries and `notes:` field per the row-update specification in AC4.
- **No invariant-register touches.** Zero LOC.
- **No ADR additions.** Zero LOC.

**KLOC aggregate alarm sits at 16,000.** Story 1a.3 left v0.1-α at ~5,250 LOC; this story adds ≤500 LOC (mostly in `maos-cli` which has a `kloc.toml` ceiling of 2,000 — well under). Expected aggregate after 1a.4: ~5,750 LOC.

**Total expected diff:** ~450–700 LOC across **8 new files** + **6 modified files**.

## Acceptance Criteria

### AC1 — `maosctl` CLI scaffold compiled out of `crates/maos-cli/` with six v0.1 subcommand stubs and clap derive-driven command tree

**Given** architecture §4.0.2's canonical 17-crate layout line: `maos-cli/  # v0.1    maosctl`
**And** the epic-level AC1 anchor: "the `maos-cli` crate compiled to `maosctl`"
**And** the binding subcommand list (epic AC1 prose): `install`, `start`, `stop`, `unload`, `run`, `audit` — **six** verbs (note: the epic's "Owns" line says `install`/`start`/`stop`/`unload` only four; the AC list adds `run`/`audit` and is the binding scope per the epic's own AC-vs-owns precedence convention)
**And** PRD FR1 ("source-build install via `cargo install`") + FR2 ("clean uninstall stub") + §10.1 ("`maosctl` basic (install, uninstall, audit query, spirit invoke)")
**And** the existing workspace pattern: `clap = { version = "4.5", features = ["derive"] }` from `maos-corpus-gen/Cargo.toml` (already resolved to 4.6.1 in `Cargo.lock`)
**And** the v0.1-α "no real subcommand body" rule from §"What this story is NOT" #1

**When** Story 1a.4's CLI scaffold commit lands in `maos-cli`

**Then** `crates/maos-cli/Cargo.toml` adds a `[[bin]]` block declaring the `maosctl` binary explicitly (worked example):

```toml
[package]
name = "maos-cli"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "MAOS CLI — maosctl command-line interface (Story 1a.4)"

[[bin]]
name = "maosctl"
path = "src/main.rs"

[lib]
name = "maos_cli"
path = "src/lib.rs"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

**And** `crates/maos-cli/src/main.rs` exists as a thin shim that delegates to the library entry point:

```rust
#![forbid(unsafe_code)]

//! `maosctl` binary entrypoint — thin shim over `maos_cli::run`.

use std::process::ExitCode;

fn main() -> ExitCode {
    maos_cli::run(std::env::args_os().collect())
}
```

**And** `crates/maos-cli/src/cli.rs` declares the clap-derive command tree (worked example skeleton):

```rust
//! Top-level command tree for `maosctl`.
//!
//! Per Story 1a.4 epic AC1: six v0.1 verbs (`install`, `start`, `stop`,
//! `unload`, `run`, `audit`) declared as subcommands. Every subcommand
//! body at v0.1-α emits a deterministic "not-yet-implemented" diagnostic
//! and exits with code 2 — the real bodies land at the cited stories.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "maosctl",
    version,
    about = "MAOS operator control plane CLI (v0.1-α scaffold)",
    long_about = None,
)]
pub struct Cli {
    /// Suppress all ANSI color sequences (per NFR-Ops-5).
    /// Also honored via NO_COLOR and TERM=dumb environment variables.
    #[arg(long, global = true)]
    pub plain: bool,

    /// Telemetry opt-in flag (per FR7). Default: `off` at v0.1-α
    /// (FR7 declares opt-in default; the actual telemetry surface
    /// lands at v0.5).
    #[arg(long, value_enum, default_value_t = TelemetryMode::Off, global = true)]
    pub telemetry: TelemetryMode,

    #[command(subcommand)]
    pub command: Subcommand,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum TelemetryMode {
    On,
    Off,
}

#[derive(Subcommand, Debug)]
pub enum Subcommand {
    /// Install a Spirit (Story 1b.5b lands the real body).
    Install(InstallArgs),
    /// Start a Spirit (Story 5.1 lifecycle verbs).
    Start(StartArgs),
    /// Stop a Spirit (Story 5.1 lifecycle verbs).
    Stop(StopArgs),
    /// Unload a Spirit (Story 5.1 lifecycle verbs).
    Unload(UnloadArgs),
    /// Run a one-shot Spirit invocation (Story 1b.5b).
    Run(RunArgs),
    /// Audit-trail subcommands (Story 1b.5b query subcommand; FR42–44 sealed-export at v1.0).
    Audit(AuditArgs),
}

#[derive(clap::Args, Debug)]
pub struct InstallArgs {
    /// Spirit registry URI or local path (placeholder at v0.1-α).
    pub source: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct StartArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct StopArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct UnloadArgs {
    pub spirit: Option<String>,
}

#[derive(clap::Args, Debug)]
pub struct RunArgs {
    pub spirit: Option<String>,
    pub args: Vec<String>,
}

#[derive(clap::Args, Debug)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub query: Option<AuditQuery>,
}

#[derive(Subcommand, Debug)]
pub enum AuditQuery {
    /// Tail the local Transparency Log (Story 1b.5b).
    Query,
}
```

**And** `crates/maos-cli/src/lib.rs` exposes a single `run(args)` entry point:

```rust
#![forbid(unsafe_code)]

//! `maos-cli` — maosctl command-line interface (Story 1a.4 scaffold).
//!
//! The library wraps clap parsing + subcommand dispatch + accessibility
//! resolution. The `maosctl` binary at `src/main.rs` is a 3-line shim
//! over `run()`. Subcommand bodies at v0.1-α emit deterministic
//! "not-yet-implemented" diagnostics and exit with code 2.

use std::ffi::OsString;
use std::process::ExitCode;

pub mod accessibility;
pub mod cli;
pub mod subcommands;

use clap::Parser;

/// Library-level entry point. Returns a `std::process::ExitCode`
/// for the binary `main.rs` to propagate.
pub fn run(args: Vec<OsString>) -> ExitCode {
    let parsed = match cli::Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            // clap's own error rendering. Note: clap honors NO_COLOR via
            // its own anstyle dep, but we also pass `--plain` through to
            // the color-choice resolver for consistency.
            return e.exit();
        }
    };

    let color = accessibility::ColorChoice::resolve(
        parsed.plain,
        &accessibility::RealEnv,
    );

    subcommands::dispatch(&parsed.command, color)
}
```

**And** `crates/maos-cli/src/subcommands.rs` provides the stub dispatcher:

```rust
//! v0.1-α subcommand stubs. Each emits a deterministic
//! "not-yet-implemented" message and exits with code 2.

use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::Subcommand;

pub fn dispatch(cmd: &Subcommand, _color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(_) => stub("install", "Story 1b.5b"),
        Subcommand::Start(_) => stub("start", "Story 5.1"),
        Subcommand::Stop(_) => stub("stop", "Story 5.1"),
        Subcommand::Unload(_) => stub("unload", "Story 5.1"),
        Subcommand::Run(_) => stub("run", "Story 1b.5b"),
        Subcommand::Audit(_) => stub("audit", "Story 1b.5b"),
    }
}

fn stub(name: &str, future_story: &str) -> ExitCode {
    eprintln!(
        "maosctl: {name} not yet implemented at v0.1-α — landing at {future_story}"
    );
    ExitCode::from(2)
}
```

**And** all six subcommands appear in `maosctl --help` output:

```
$ maosctl --help
MAOS operator control plane CLI (v0.1-α scaffold)

Usage: maosctl [OPTIONS] <COMMAND>

Commands:
  install  Install a Spirit (Story 1b.5b lands the real body)
  start    Start a Spirit (Story 5.1 lifecycle verbs)
  stop     Stop a Spirit (Story 5.1 lifecycle verbs)
  unload   Unload a Spirit (Story 5.1 lifecycle verbs)
  run      Run a one-shot Spirit invocation (Story 1b.5b)
  audit    Audit-trail subcommands (Story 1b.5b ...)
  help     Print this message or the help of the given subcommand(s)

Options:
      --plain                  Suppress all ANSI color sequences (per NFR-Ops-5)
      --telemetry <TELEMETRY>  [default: off] [possible values: on, off]
  -h, --help                   Print help
  -V, --version                Print version
```

**And** `maosctl --version` prints the workspace version (`maosctl 0.1.0-alpha` derived from `version.workspace = true`):

```
$ maosctl --version
maosctl 0.1.0-alpha
```

**And** invoking any subcommand at v0.1-α emits the stub diagnostic and exits with code 2:

```
$ maosctl install
maosctl: install not yet implemented at v0.1-α — landing at Story 1b.5b
$ echo $?
2
```

**And** `cargo build -p maos-cli --locked --all-targets` succeeds with zero warnings.

**And** `cargo test -p maos-cli` runs the unit-test suite (AC2's accessibility tests + the dispatcher integration tests) with zero failures.

**And** `crates/maos-cli/Cargo.toml` carries **exactly one** new top-level dep (`clap = { version = "4.5", features = ["derive"] }`); the dep is already present in `Cargo.lock` at 4.6.1 via `maos-corpus-gen`. `git diff HEAD -- Cargo.lock` shows ZERO `^+name = ` lines (zero new lockfile entries; clap and its transitive deps were already resolved). Verified by the dev record's "Dependency-introduction note".

**And** `crates/maos-cli/Cargo.toml` does NOT add any path-dep to `maos-bin`, `maos-kernel-core`, `maos-domain`, or any other workspace crate. Verified by `cargo tree -p maos-cli | grep -E '^(maos-)'` returning only `maos-cli v0.1.0-alpha` itself.

**Sanity check (forbidden patterns):**

```rust
// FORBIDDEN — kernel-core dependency on the front-end CLI
[dependencies]
maos-kernel-core = { path = "../maos-kernel-core" }  // NO — CLI must stay decoupled at v0.1-α

// FORBIDDEN — subcommand body that does real work
fn install_run(args: &InstallArgs) -> ExitCode {
    let spirit_loader = ...;                          // NO — Story 1b.5b territory
    spirit_loader.install(args.source.as_ref()).unwrap();
    ExitCode::SUCCESS
}

// FORBIDDEN — subcommand list missing one of the six verbs
#[derive(Subcommand, Debug)]
pub enum Subcommand {
    Install(InstallArgs),
    Start(StartArgs),
    Stop(StopArgs),
    Unload(UnloadArgs),
    // MISSING: Run, Audit — AC1 binding subcommand list is six, not four
}

// CORRECT — six verbs, all stubbed, no kernel touch
#[derive(Subcommand, Debug)]
pub enum Subcommand {
    Install(InstallArgs),
    Start(StartArgs),
    Stop(StopArgs),
    Unload(UnloadArgs),
    Run(RunArgs),
    Audit(AuditArgs),
}
```

### AC2 — Accessibility resolver implements the four-input precedence (`--plain` > `NO_COLOR` > `TERM=dumb` > tty-default) with 6+ dedicated unit tests

**Given** NFR-Ops-5: "maosctl `--plain` flag + `NO_COLOR` + `TERM=dumb` accessibility. **v0.1**"
**And** §10.1 acceptance: "accessibility flags (`--plain`, `NO_COLOR`, `TERM=dumb`)" as one of the J0 evaluator-path primitives
**And** the cross-ecosystem convention: `NO_COLOR` per [no-color.org](https://no-color.org/) honors ANY non-empty value (typically `NO_COLOR=1`); `TERM=dumb` indicates a non-capable terminal per the GNU convention
**And** the precedence cascade (binding decision for v0.1-α): **`--plain` overrides everything; `NO_COLOR` overrides `TERM=dumb`; `TERM=dumb` overrides tty-auto; tty-auto is the fall-through default** (this ordering means a user with `NO_COLOR=1` set globally can re-enable color for a single invocation via `maosctl ... --plain=false` if needed — wait, no: clap's `bool` flag is one-way; the precedence is `--plain` ENABLES no-color and CANNOT re-enable color. Operators who need color despite `NO_COLOR` env override the env in their shell — that's the no-color.org-conformant behavior)

**When** Story 1a.4's accessibility module commit lands in `maos-cli`

**Then** `crates/maos-cli/src/accessibility.rs` declares the resolver (worked example):

```rust
//! Accessibility — `ColorChoice` resolver per NFR-Ops-5.
//!
//! Precedence cascade (highest to lowest):
//!   1. `--plain` CLI flag           → ColorChoice::Never
//!   2. `NO_COLOR` env (any value)   → ColorChoice::Never
//!   3. `TERM=dumb` env              → ColorChoice::Never
//!   4. stdout-is-a-tty              → ColorChoice::Auto
//!   5. fall-through (no tty)        → ColorChoice::Never
//!
//! The `EnvProvider` trait exists for testability — production uses
//! `RealEnv` (delegates to `std::env::var_os`); tests use `MockEnv`
//! to deterministically set/unset env vars without `std::env::set_var`
//! racing parallel tests.

use std::ffi::OsString;

/// Color output decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    /// Color permitted (caller may still consult its own tty check).
    Auto,
    /// Never emit ANSI color sequences.
    Never,
    /// Always emit color (escape hatch for forced color output;
    /// not exposed via CLI flag at v0.1-α — included for completeness).
    #[allow(dead_code)]
    Always,
}

/// Environment-variable provider trait — exists for test isolation.
pub trait EnvProvider {
    fn var(&self, key: &str) -> Option<OsString>;
}

/// Production env provider — delegates to `std::env::var_os`.
pub struct RealEnv;

impl EnvProvider for RealEnv {
    fn var(&self, key: &str) -> Option<OsString> {
        std::env::var_os(key)
    }
}

impl ColorChoice {
    /// Resolve color choice from the precedence cascade.
    ///
    /// `cli_plain` = `true` when the operator passed `--plain`.
    /// `env` provides environment-variable lookups.
    pub fn resolve(cli_plain: bool, env: &dyn EnvProvider) -> ColorChoice {
        if cli_plain {
            return ColorChoice::Never;
        }
        if let Some(value) = env.var("NO_COLOR") {
            if !value.is_empty() {
                return ColorChoice::Never;
            }
        }
        if let Some(term) = env.var("TERM") {
            if term == OsString::from("dumb") {
                return ColorChoice::Never;
            }
        }
        ColorChoice::Auto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Test-only env provider — deterministic, hermetic.
    #[derive(Default)]
    struct MockEnv {
        vars: HashMap<String, OsString>,
    }

    impl MockEnv {
        fn with(mut self, k: &str, v: &str) -> Self {
            self.vars.insert(k.to_string(), OsString::from(v));
            self
        }
    }

    impl EnvProvider for MockEnv {
        fn var(&self, key: &str) -> Option<OsString> {
            self.vars.get(key).cloned()
        }
    }

    #[test]
    fn plain_flag_overrides_everything() {
        let env = MockEnv::default()
            .with("NO_COLOR", "")           // empty NO_COLOR (no-color.org says
                                             // empty means don't honor it)
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(true, &env), ColorChoice::Never);
    }

    #[test]
    fn no_color_env_with_any_value_disables_color() {
        let env = MockEnv::default()
            .with("NO_COLOR", "1")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Never);
    }

    #[test]
    fn no_color_env_empty_string_does_not_disable_color() {
        let env = MockEnv::default()
            .with("NO_COLOR", "")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Auto);
    }

    #[test]
    fn term_dumb_disables_color_when_no_color_unset() {
        let env = MockEnv::default().with("TERM", "dumb");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Never);
    }

    #[test]
    fn term_xterm_falls_through_to_auto() {
        let env = MockEnv::default().with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Auto);
    }

    #[test]
    fn no_env_falls_through_to_auto() {
        let env = MockEnv::default();
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Auto);
    }

    #[test]
    fn plain_flag_wins_over_no_color_set() {
        let env = MockEnv::default().with("NO_COLOR", "1");
        assert_eq!(ColorChoice::resolve(true, &env), ColorChoice::Never);
    }

    #[test]
    fn no_color_wins_over_term_dumb() {
        let env = MockEnv::default()
            .with("NO_COLOR", "1")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Never);
    }
}
```

**And** the resolver MUST be hermetic — tests use `MockEnv`, NOT `std::env::set_var` (which mutates process-global state and races concurrent tests).

**And** `cargo test -p maos-cli accessibility::tests` runs all 8 tests with zero failures.

**And** clap's OWN `NO_COLOR` honoring is independent and complementary — clap's `anstyle` dep already respects `NO_COLOR` for its help-output rendering. Our `--plain` flag passes through to clap via the standard `Command::color(clap::ColorChoice::Never)` API if we ever need to disable clap's color separately; at v0.1-α we rely on clap's default behavior (which honors `NO_COLOR` automatically) plus our own resolver for the v0.5+ stub-message output coloring.

**And** the resolver is **NOT** wired into `maos-bin/src/main.rs`. The startup banner from 1a.2 stays plain ASCII; only `maosctl` consumes the resolver at v0.1-α.

**Sanity check (forbidden patterns):**

```rust
// FORBIDDEN — std::env::set_var in tests (races parallel test threads)
#[test]
fn plain_overrides() {
    std::env::set_var("NO_COLOR", "1");          // NO — non-hermetic, races
    assert_eq!(ColorChoice::resolve(true, &RealEnv), ColorChoice::Never);
}

// FORBIDDEN — empty NO_COLOR treated as truthy
if env.var("NO_COLOR").is_some() {               // NO — no-color.org: empty means don't honor
    return ColorChoice::Never;
}

// FORBIDDEN — precedence inversion
if env.var("TERM") == Some("dumb".into()) {
    return ColorChoice::Never;
}
if cli_plain {                                    // NO — --plain must win over TERM=dumb (it's
                                                  //   stricter; --plain ALSO satisfies TERM=dumb)
    return ColorChoice::Never;
}

// CORRECT — hermetic test, correct precedence, no-color.org compliant
impl ColorChoice {
    pub fn resolve(cli_plain: bool, env: &dyn EnvProvider) -> ColorChoice {
        if cli_plain { return ColorChoice::Never; }
        if let Some(v) = env.var("NO_COLOR") {
            if !v.is_empty() { return ColorChoice::Never; }
        }
        if env.var("TERM").as_deref() == Some(std::ffi::OsStr::new("dumb")) {
            return ColorChoice::Never;
        }
        ColorChoice::Auto
    }
}
```

### AC3 — `SECURITY.md` at the repo root carries the four NFR-Ops-4 binding sections; new `cargo xtask check-security-md` gate parses it and fails the build on absence or malformation

**Given** PRD FR61: "Substrate project publishes and maintains `SECURITY.md` documenting (a) disclosure contact (`security@maos.dev` with published GPG key), (b) coordinated-disclosure window and CVE-assignment process, (c) supported-versions matrix for security backports, (d) advisory-publication channel. **v0.1 binding** — not deferred"
**And** PRD NFR-Ops-4: "`SECURITY.md` with disclosure address (`security@maos.dev`), GPG key, embargo window (90-day default), advisory-publication channel, supported-versions matrix. **v0.1 ship gate.** CNA registration through MITRE moves to v0.5"
**And** the binding section taxonomy (this story's normalization of FR61's (a)–(d) and NFR-Ops-4's enumeration):
  - § **Reporting a vulnerability** — contact address + GPG fingerprint slot
  - § **Coordinated-disclosure window** — 90-day embargo language
  - § **Supported versions** — backports matrix
  - § **Advisory channel** — publication channel for disclosed CVEs
**And** the v0.1-α scope: CVE assignment / CNA registration is **deferred to v0.5** per the NFR-Ops-4 phase-split; SECURITY.md at v0.1 documents the slot via a one-line forward-reference, not by completing the registration

**When** Story 1a.4's SECURITY.md + check-security-md commit lands

**Then** `SECURITY.md` exists at the repo root with the four required H2 sections (worked example skeleton):

```markdown
# Security Policy

The MAOS substrate ships with a coordinated security disclosure pipeline
in place from day one (v0.1-α). This document is the single source of
truth for how to report, what to expect, which versions get patches,
and where advisories are published.

## Reporting a vulnerability

**Contact:** `security@maos.dev`
**GPG public key fingerprint:** `<TO-BE-PUBLISHED>` (tracked at
issue [#TBD] — operator action item; the slot is a binding placeholder
at v0.1-α per Story 1a.4's `SECURITY.md` deliverable).

Encrypt the report with the published GPG key if it contains
exploit primitives, capability-token leak fragments, or other
material that should not transit cleartext mail.

Please include:
- A concise description of the vulnerability and its impact.
- Reproducer steps (corpus seed, manifest, capability scope, sandbox
  tier where applicable).
- Affected `maos` version (`maosctl --version`) and host OS.
- Suggested mitigation if known (optional).

## Coordinated-disclosure window

The MAOS substrate operates a **90-day coordinated-disclosure embargo**
by default (NFR-Ops-4 binding window). The clock starts when the report
is acknowledged by the security team. During the embargo:

- The reporter does not disclose publicly.
- The security team triages, develops a fix, and prepares a CVE
  request (CNA registration with MITRE lands at v0.5 per the
  NFR-Ops-4 phase-split; until then, CVEs are requested through the
  MITRE general-form channel).
- Extensions beyond 90 days require mutual agreement, documented in
  the disclosure thread.

If the embargo lapses without acknowledgment from the security team
(receipt-of-report SLA is **5 business days**), the reporter is free to
disclose under their own timeline.

## Supported versions

Backports of security patches target the following versions:

| Version range | Status               | Backport window  |
|---------------|----------------------|------------------|
| `0.1.x`       | Active development   | All security fixes during v0.1 phase |
| `< 0.1.0`     | Pre-release / unsupported | None       |

At v1.0+ the MAOS substrate will maintain a 2-year LTS branch policy
per NFR-Ops-2 (deferred to v1.5 maturation per the phased roadmap).

## Advisory channel

Published advisories appear in:
- GitHub Security Advisories on this repository
  (`https://github.com/lunarpulse/maos/security/advisories`).
- The MAOS substrate release notes for the fix-bearing version.

Advisories include: affected component, CVSS v3.1 severity, affected
versions, fixed version, mitigation guidance, and credit to the
reporter (with permission).

---

*This policy is binding at v0.1-α (NFR-Ops-4 ship gate, FR61). CNA
registration via MITRE lands at v0.5 per the NFR-Ops-4 phase-split.*
```

**And** `xtask/src/check_security_md.rs` declares the gate function (worked example skeleton):

```rust
//! check-security-md — NFR-Ops-4 + FR61 v0.1-α ship-gate.
//!
//! Parses repo-root `SECURITY.md` and asserts the four required H2
//! sections per Story 1a.4 AC3. Fails CI when:
//!   - `SECURITY.md` is absent at the repo root.
//!   - Any of the four required headers is missing.
//!   - Headers are not at the H2 level (e.g., `# Reporting` instead of
//!     `## Reporting a vulnerability`).
//!
//! The check is intentionally header-text-based (not regex-rich) so
//! that prose evolution within sections does not break CI; the
//! contract is the section taxonomy, not the prose.

use std::path::Path;

const REQUIRED_SECTIONS: &[&str] = &[
    "Reporting a vulnerability",
    "Coordinated-disclosure window",
    "Supported versions",
    "Advisory channel",
];

#[derive(Debug)]
pub struct Report {
    pub passed: bool,
    pub missing_sections: Vec<&'static str>,
    pub present_sections: Vec<&'static str>,
}

pub fn check_security_md(workspace_root: &Path) -> Report {
    let path = workspace_root.join("SECURITY.md");
    let contents = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => {
            return Report {
                passed: false,
                missing_sections: REQUIRED_SECTIONS.to_vec(),
                present_sections: vec![],
            };
        }
    };

    let h2_headers: Vec<&str> = contents
        .lines()
        .filter_map(|line| line.strip_prefix("## ").map(str::trim))
        .collect();

    let mut present = Vec::new();
    let mut missing = Vec::new();
    for &section in REQUIRED_SECTIONS {
        if h2_headers.iter().any(|h| *h == section) {
            present.push(section);
        } else {
            missing.push(section);
        }
    }

    Report {
        passed: missing.is_empty(),
        missing_sections: missing,
        present_sections: present,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_security_md(dir: &Path, body: &str) {
        fs::write(dir.join("SECURITY.md"), body).unwrap();
    }

    #[test]
    fn passes_when_all_four_h2_sections_present() {
        let tmp = TempDir::new().unwrap();
        write_security_md(tmp.path(), &format!(
            "# Security Policy\n\n## Reporting a vulnerability\n\
             ...\n## Coordinated-disclosure window\n...\n\
             ## Supported versions\n...\n## Advisory channel\n...\n"
        ));
        let r = check_security_md(tmp.path());
        assert!(r.passed, "missing: {:?}", r.missing_sections);
        assert_eq!(r.missing_sections.len(), 0);
    }

    #[test]
    fn fails_when_file_absent() {
        let tmp = TempDir::new().unwrap();
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.missing_sections.len(), 4);
    }

    #[test]
    fn fails_when_any_section_missing() {
        let tmp = TempDir::new().unwrap();
        write_security_md(tmp.path(), &format!(
            "# Security Policy\n\n## Reporting a vulnerability\n...\n\
             ## Coordinated-disclosure window\n...\n## Supported versions\n...\n"
            // Advisory channel missing
        ));
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.missing_sections, vec!["Advisory channel"]);
    }

    #[test]
    fn fails_when_required_section_is_at_h1_not_h2() {
        let tmp = TempDir::new().unwrap();
        write_security_md(tmp.path(),
            "# Reporting a vulnerability\n## Coordinated-disclosure window\n\
             ## Supported versions\n## Advisory channel\n"
        );
        let r = check_security_md(tmp.path());
        assert!(!r.passed);
        assert_eq!(r.missing_sections, vec!["Reporting a vulnerability"]);
    }

    #[test]
    fn extra_h2_sections_are_allowed() {
        let tmp = TempDir::new().unwrap();
        write_security_md(tmp.path(),
            "## Reporting a vulnerability\n## Coordinated-disclosure window\n\
             ## Supported versions\n## Advisory channel\n## Hall of fame\n"
        );
        let r = check_security_md(tmp.path());
        assert!(r.passed);
    }
}
```

**And** `xtask/src/main.rs` registers the new subcommand:

```rust
// (Inside the existing enum CliCommand pattern — additive insertion)
CheckSecurityMd,
// ...
// (Inside the match arm dispatch)
CliCommand::CheckSecurityMd => {
    let report = check_security_md::check_security_md(&workspace_root);
    if report.passed {
        eprintln!("check-security-md: PASS ({} sections found)",
            report.present_sections.len());
        std::process::exit(0);
    } else {
        eprintln!("check-security-md: FAIL — missing sections: {:?}",
            report.missing_sections);
        std::process::exit(1);
    }
}
```

**And** `.github/workflows/discipline.yml` adds the new gate as a per-commit blocking step alongside the existing 13 gates (worked example fragment):

```yaml
      - name: check-security-md
        run: cargo run -p xtask -- check-security-md
```

**And** `xtask/gate-registry.toml` declares the gate metadata (worked example):

```toml
[gates.check-security-md]
phase = "v0.1-α"
blocking = true
owner = "Story 1a.4"
mode = "per-commit"
description = "Asserts SECURITY.md exists and carries the four NFR-Ops-4 binding sections."
```

**And** `cargo run -p xtask -- check-security-md` prints `check-security-md: PASS (4 sections found)` and exits with code 0.

**And** removing any of the four required H2 headers from SECURITY.md causes `cargo run -p xtask -- check-security-md` to exit with code 1 and print the missing-section names.

**And** `cargo test -p xtask check_security_md_tests` runs the 5 unit tests with zero failures.

**Sanity check (forbidden patterns):**

```rust
// FORBIDDEN — file-existence-only check (the original lazy form)
pub fn check_security_md(root: &Path) -> bool {
    root.join("SECURITY.md").exists()         // NO — passes a 1-line file with no content
}

// FORBIDDEN — case-insensitive header match (drifts from binding taxonomy)
if h.to_lowercase().contains("reporting") { ... }  // NO — matches "Reporting bugs", etc.

// FORBIDDEN — required-section list lifted from runtime config
const REQUIRED: &str = include_str!("../security-sections.txt");  // NO — invariant lives in code

// CORRECT — exact-match H2 header scan, 4 fixed sections in code
const REQUIRED_SECTIONS: &[&str] = &[
    "Reporting a vulnerability",
    "Coordinated-disclosure window",
    "Supported versions",
    "Advisory channel",
];
```

### AC4 — `tests/coverage-matrix.yaml` rows for FR1, FR2, FR7, FR61, NFR-Ops-4, NFR-Ops-5 flip from empty/sparse to populated with gates + notes attribution to Story 1a.4

**Given** the existing coverage-matrix shape (one entry per FR/NFR; fields `gates: []`, `corpora: []`, `phase`, `valid_until`, optional `notes`)
**And** the NFR-Meta-3 binding rule: "CI fails if any FR/NFR with phase-status `delivered ≤ current-phase` has zero corpus coverage" — this means an FR/NFR at phase v0.1-α MUST either have non-empty `gates`/`corpora`, OR a `notes:` line documenting the deferral, by the time E0's coverage-matrix gate runs at PR-open time
**And** the current row state (post-1a.3):
  - FR1: `gates: [reproducible-build]`, `notes: 1a.1 ships maos-bin crate; cargo install --path crates/maos-bin succeeds via reproducible-build gate; full install workflow in v0.5+`
  - FR2: `gates: []`, `notes: 1a.1 commits maos-cli crate stub; maosctl uninstall lands in Story 1a.4`
  - FR7: `gates: []`, `notes: type-codification only at v0.1-α; runtime opt-in surface lands at v0.5`
  - FR61: `gates: []` — no notes
  - NFR-Ops-4: `gates: []` — no notes
  - NFR-Ops-5: `gates: []` — no notes

**When** Story 1a.4's coverage-matrix commit lands

**Then** `tests/coverage-matrix.yaml` updates each row additively (worked example diff):

```yaml
# FR1 — flip `notes` to mention maosctl, keep gate list
  FR1:
    gates:
    - reproducible-build
    corpora: []
    phase: v0.1
    valid_until: '2027-05-12'
    notes: 1a.1 ships maos-bin; 1a.4 ships maosctl via `cargo install --path
      crates/maos-cli --locked`; `maosctl --version` reports the workspace
      version; full install workflow (Spirit registry fetch + signature
      verify) lands at v0.5+

# FR2 — populate gate (the new check-security-md gate ALSO covers the
# `maosctl uninstall` declared-stub assertion via the help-tree check;
# the FR2 row uses cargo-test as its v0.1 gate proxy until the real
# uninstall body lands at Story 1b.5b)
  FR2:
    gates: []
    corpora: []
    phase: v0.1
    valid_until: '2027-05-12'
    notes: 1a.4 ships `maosctl unload` and `maosctl install` stubs (real
      uninstall body lands at Story 1b.5b); cargo-test verifies the help-tree
      surface; no behavior gate at v0.1-α

# FR7 — note declared telemetry flag
  FR7:
    gates: []
    corpora: []
    phase: v0.5
    valid_until: '2027-05-12'
    notes: 1a.4 declares `maosctl --telemetry=on|off` CLI flag (default
      `off` per opt-in commitment); runtime telemetry surface (collector +
      schema + redaction layer) lands at v0.5

# FR61 — wire the new gate
  FR61:
    gates:
    - check-security-md
    corpora: []
    phase: v0.1
    valid_until: '2027-05-12'
    notes: 1a.4 ships SECURITY.md with four NFR-Ops-4 binding sections
      (disclosure contact, 90-day embargo, supported-versions, advisory
      channel); check-security-md gate parses headers and fails on
      malformation; CNA registration deferred to v0.5

# NFR-Ops-4 — same gate as FR61 (they are co-binding)
  NFR-Ops-4:
    gates:
    - check-security-md
    corpora: []
    phase: v0.1
    valid_until: '2027-05-12'
    notes: 1a.4 ships SECURITY.md ship-gate via check-security-md xtask;
      CNA registration through MITRE moves to v0.5 per the phase-split

# NFR-Ops-5 — cargo-test-based assertion via in-process unit tests
  NFR-Ops-5:
    gates: []
    corpora: []
    phase: v0.1
    valid_until: '2027-05-12'
    notes: 1a.4 ships maosctl accessibility — `--plain` flag, `NO_COLOR`
      env, `TERM=dumb` env unified via `ColorChoice::resolve`; 8 dedicated
      unit tests in `maos-cli::accessibility::tests` (hermetic MockEnv);
      verified by `cargo test -p maos-cli` in the workspace test run
```

**And** `cargo run -p xtask -- coverage-matrix` continues to pass (it asserts no FR/NFR at phase ≤ current is unrepresented; populated `notes:` satisfies the rule for FR/NFRs whose runtime gate body lands at v0.5+).

**And** `cargo run -p xtask -- corpus-staleness` and `cargo run -p xtask -- rebaseline-check` continue to pass — neither row touches a corpus, only `notes` and `gates` are mutated.

**Sanity check (forbidden patterns):**

```yaml
# FORBIDDEN — drop a `notes:` line without populating `gates:` (NFR-Meta-3
# requires that any v0.1-binding FR/NFR have SOME evidence trail)
FR61:
  gates: []                                  # NO — empty gates + empty notes fails the rule
  corpora: []
  phase: v0.1
  valid_until: '2027-05-12'

# FORBIDDEN — invent a new gate name without adding it to gate-registry.toml
FR61:
  gates:
  - security-md-presence                     # NO — gate isn't registered
  corpora: []
  phase: v0.1

# CORRECT — gate name matches the xtask/gate-registry.toml entry + notes
# describe the v0.1-α scope + phase-split for deferred parts
FR61:
  gates:
  - check-security-md
  corpora: []
  phase: v0.1
  valid_until: '2027-05-12'
  notes: 1a.4 ships SECURITY.md with four NFR-Ops-4 binding sections; ...
```

### AC5 — Dev-record carries the seven-subsection AC5 evidence block; all 13+1 Epic-0 gates green; `cargo install --path crates/maos-cli --locked` succeeds; `maosctl --version` reports workspace version; KLOC + license + Cargo.lock blast all within envelope

**Given** Story 1a.3's AC6 seven-subsection dev-record pattern (Pre-flight baseline / ADR alignment / Runtime smoke / Shell-emptiness audit — N/A here / Surface-classification audit — N/A here / Dep-introduction note / "What did NOT happen this story" checklist / Self-review checklist)
**And** the Epic-0 retro action items A1 (self-review), A2 (dep blast-radius), A3 (worked-examples) binding for Epic 1a
**And** the 14-gate set (13 Epic-0 + 1 new `check-security-md`)

**When** the dev opens the PR for Story 1a.4

**Then** the dev record includes a **Pre-flight baseline** table:

| Gate                                       | Result (pre-1a.4) |
|--------------------------------------------|---|
| cargo build --locked --all-targets --workspace | PASS |
| cargo test --workspace --locked            | PASS |
| check-unsafe                               | PASS |
| check-empty-kernel                         | PASS |
| check-loom                                 | PASS |
| check-service-boundary                     | PASS |
| kloc-check                                 | PASS (aggregate=~5,250 LOC pre-1a.4) |
| abi-diff                                   | PASS |
| check-corpus                               | PASS |
| check-judge-config                         | PASS |
| coverage-matrix                            | PASS |
| corpus-staleness                           | PASS |
| rebaseline-check                           | PASS |
| calibrate                                  | PASS |
| invariant-lock (no-touch mode)             | PASS (zero touched invariants) |
| cargo deny check                           | PASS |

**And** a **Post-implementation gate run** confirms all 14 gates pass (the 13 Epic-0 gates + the new `check-security-md`); the new gate must be invoked locally before PR-open to confirm `check-security-md: PASS (4 sections found)`.

**And** a **Runtime smoke transcript** is captured (worked example):

```
$ cargo install --path crates/maos-bin --locked
   Compiling maos-bin v0.1.0-alpha (...)
    Finished `release` profile [optimized] target(s) in <T>s
  Installing /home/<user>/.cargo/bin/maos-bin
   Installed package `maos-bin v0.1.0-alpha (...)`

$ cargo install --path crates/maos-cli --locked
   Compiling maos-cli v0.1.0-alpha (...)
    Finished `release` profile [optimized] target(s) in <T>s
  Installing /home/<user>/.cargo/bin/maosctl
   Installed package `maos-cli v0.1.0-alpha (...)`

$ maosctl --version
maosctl 0.1.0-alpha

$ maosctl --help
MAOS operator control plane CLI (v0.1-α scaffold)
...

$ maosctl install
maosctl: install not yet implemented at v0.1-α — landing at Story 1b.5b
$ echo $?
2

$ NO_COLOR=1 maosctl --help
(same output, no ANSI escapes — verify via piping into `cat -v`)

$ TERM=dumb maosctl --help
(same — no ANSI escapes)

$ maosctl --plain install
maosctl: install not yet implemented at v0.1-α — landing at Story 1b.5b

$ cat SECURITY.md | head -5
# Security Policy
...

$ cargo run -p xtask -- check-security-md
check-security-md: PASS (4 sections found)
```

**And** a **Dep-introduction note** confirms:

- New top-level dep: `clap = { version = "4.5", features = ["derive"] }` in `crates/maos-cli/Cargo.toml` ONLY.
- `Cargo.lock` blast radius: **0 new lockfile entries** (`git diff HEAD -- Cargo.lock | grep -c '^+name = '` returns 0; clap and its transitive deps already resolved at 4.6.1 via `maos-corpus-gen`).
- `cargo deny check`: PASS. No license amendment required.
- Verification: `cargo tree -p maos-cli --depth=1` shows only `clap v4.6.1` as a non-stdlib dep.

**And** a **"What did NOT happen this story" checklist** confirms (each item with its grep command + expected output of zero/empty):

- [ ] No `maos-kernel-core` import in maos-cli: `grep -rn 'maos_kernel_core\|maos-kernel-core' crates/maos-cli/` returns empty.
- [ ] No `maos-bin` import in maos-cli: same grep, empty.
- [ ] No subcommand body does real work: `grep -rn 'fn install_run\|fn start_run\|fn stop_run' crates/maos-cli/src/subcommands.rs` shows only `stub(...)` calls.
- [ ] No CryptoProvider touch: `git diff HEAD -- crates/maos-domain/src/ports/crypto.rs crates/maos-kernel-core/src/security/crypto.rs` returns empty.
- [ ] No invariant-register touch: `git diff HEAD -- docs/invariants/I*.md` returns empty.
- [ ] No ADR touch: `git diff HEAD -- docs/adr/` returns empty.
- [ ] No xtask kernel-surface change: `git diff HEAD -- xtask/kernel-api-classes.toml docs/ci-baselines/kernel-surface-v0.1-alpha.json` returns empty.
- [ ] No GPG key generated: SECURITY.md carries `<TO-BE-PUBLISHED>` slot; no `gpg --gen-key` ran in CI; no key material committed.
- [ ] No `std::env::set_var` in tests: `grep -rn 'std::env::set_var\|set_var(' crates/maos-cli/` returns only references inside `RealEnv` (NONE — the MockEnv pattern means there are zero such matches in the maos-cli test code).

**And** a **Self-review checklist** with every box ticked (per Epic 0 retro A1):

- [ ] `maosctl` binary built, installed via `cargo install --path crates/maos-cli --locked`, and `maosctl --version` returns `maosctl 0.1.0-alpha`.
- [ ] All six subcommands (`install`, `start`, `stop`, `unload`, `run`, `audit`) appear in `maosctl --help` output.
- [ ] Every subcommand invocation at v0.1-α prints the stub diagnostic and exits with code 2.
- [ ] `ColorChoice::resolve` precedence cascade verified by 8 hermetic unit tests using `MockEnv`.
- [ ] `SECURITY.md` exists at repo root with four binding H2 sections.
- [ ] `cargo run -p xtask -- check-security-md` returns PASS; removing any required section fails the gate.
- [ ] 5 unit tests in `check_security_md_tests.rs` pass.
- [ ] `tests/coverage-matrix.yaml` rows FR1 / FR2 / FR7 / FR61 / NFR-Ops-4 / NFR-Ops-5 updated additively.
- [ ] `.github/workflows/discipline.yml` invokes `check-security-md` as a per-commit blocking step.
- [ ] `xtask/gate-registry.toml` declares the new gate.
- [ ] All 14 gates green locally; cargo deny check passes; Cargo.lock blast = 0.
- [ ] KLOC aggregate ≤ 5,750 LOC (1a.4 adds ≤500 LOC; per-crate `maos-cli` ≤ 2000 budget unaffected).
- [ ] Two reviewers named + tagged in PR description.

**Sanity check (forbidden patterns):**

```bash
# FORBIDDEN — dev record omits the "what did NOT happen" checklist
## Dev Agent Record
### Completion Notes List
- Shipped 1a.4 successfully.        # NO — Epic 0 retro A1 binding: self-review checklist mandatory

# FORBIDDEN — claim "all gates pass" without the explicit table
- All gates pass.                    # NO — must list each gate + PASS/FAIL

# CORRECT — seven-subsection AC5 evidence block per the pattern from 1a.3
### Pre-flight baseline
| Gate ... | Result ... |
### Runtime smoke
$ maosctl --version
maosctl 0.1.0-alpha
### Dep-introduction note
New deps: clap 4.5. Lockfile blast: 0.
### What did NOT happen this story
- [x] No kernel-core import in maos-cli ...
### Self-review checklist
- [x] All six subcommands appear in help ...
```

## Tasks / Subtasks

- [x] **Task 0 — Pre-flight baseline** (AC5)
  - [x] 0.1 Verify `sprint-status.yaml` shows 1a.3 done, epic-1a in-progress, 1a-4 ready-for-dev.
  - [x] 0.2 Run the 14-gate pre-flight matrix from §"Critical preconditions" item 2; record every PASS in dev-record's "Pre-flight baseline" table.
  - [x] 0.3 Confirm `crates/maos-cli/src/lib.rs` is the 9-line placeholder and `crates/maos-cli/Cargo.toml` has empty `[dependencies]`.
  - [x] 0.4 Confirm `SECURITY.md` does NOT exist (`test -f SECURITY.md; echo $?` → 1).
  - [x] 0.5 Confirm `cargo tree -p maos-cli` shows zero non-stdlib deps pre-implementation.

- [x] **Task 1 — maos-cli scaffold: Cargo.toml + main.rs + lib.rs + cli.rs** (AC1)
  - [x] 1.1 Update `crates/maos-cli/Cargo.toml`: add `[[bin]] name = "maosctl" path = "src/main.rs"` block; add `clap = { version = "4.5", features = ["derive"] }` to `[dependencies]`; keep workspace inheritance for version/edition/etc.
  - [x] 1.2 Create `crates/maos-cli/src/main.rs` — 3-line thin shim delegating to `maos_cli::run`.
  - [x] 1.3 Rewrite `crates/maos-cli/src/lib.rs` — re-export modules + `pub fn run(args: Vec<OsString>) -> ExitCode`; module docstring per the worked example.
  - [x] 1.4 Create `crates/maos-cli/src/cli.rs` — `#[derive(Parser)] struct Cli` + 6-variant `Subcommand` enum + 6 `*Args` structs + global `--plain` + global `--telemetry`.
  - [x] 1.5 Create `crates/maos-cli/src/subcommands.rs` — `dispatch()` fn + `stub()` helper; each variant maps to its forward-reference story.
  - [x] 1.6 Run `cargo build -p maos-cli --locked --all-targets`; confirm zero warnings.
  - [x] 1.7 Manually invoke `cargo run -p maos-cli --release -- --help` and `cargo run -p maos-cli --release -- --version`; confirm output matches the worked-example shape; capture transcript for dev record.
  - [x] 1.8 Manually invoke each of the six subcommands; confirm stub diagnostic + exit code 2.

- [x] **Task 2 — Accessibility resolver + 8 unit tests** (AC2)
  - [x] 2.1 Create `crates/maos-cli/src/accessibility.rs` — `ColorChoice` enum, `EnvProvider` trait, `RealEnv` struct, `ColorChoice::resolve(cli_plain, env)` fn.
  - [x] 2.2 In the same file, declare `#[cfg(test)] mod tests` with the 8 unit tests from the worked example.
  - [x] 2.3 Run `cargo test -p maos-cli accessibility`; confirm 8/8 pass.
  - [x] 2.4 Run `cargo test -p maos-cli --test-threads=1 -- accessibility` to verify the hermetic-MockEnv design holds even when forced to serial (paranoia check).
  - [x] 2.5 Confirm `RealEnv` is the only place that touches `std::env::var_os` in the maos-cli crate (`grep -rn 'std::env::' crates/maos-cli/` shows only doc-comment references; `RealEnv` impl uses `std::env::var_os`).

- [x] **Task 3 — SECURITY.md + check-security-md xtask gate** (AC3)
  - [x] 3.1 Create `SECURITY.md` at the repo root with the four required H2 sections per the worked-example skeleton; GPG fingerprint slot marked `<TO-BE-PUBLISHED>`; 90-day embargo prose; supported-versions table; advisory channel pointer.
  - [x] 3.2 Create `xtask/src/check_security_md.rs` with `Report` struct + `check_security_md(workspace_root)` fn + 5 unit tests.
  - [x] 3.3 Update `xtask/src/main.rs` — add `CheckSecurityMd` enum variant + match arm dispatching to the new fn; emit `check-security-md: PASS (N sections found)` on success, exit 1 with missing-section list on failure.
  - [x] 3.4 `tempfile` is already a dev-dep in `xtask/Cargo.toml`; no update needed.
  - [x] 3.5 Add `check-security-md` to the `gates` array in `xtask/gate-registry.toml` (table format incompatible with existing parser; metadata lives in coverage-matrix notes).
  - [x] 3.6 Add the `check-security-md` step to `.github/workflows/discipline.yml` (insert as a per-commit blocking step alongside the existing 13 gates); update aggregate job needs + results table + PR comment script.
  - [x] 3.7 Run `cargo run -p xtask -- check-security-md`; confirm PASS output.
  - [x] 3.8 Mutation-test: temporarily delete the "Advisory channel" H2 from SECURITY.md, re-run the gate, confirm FAIL with missing-section list; revert the mutation.
  - [x] 3.9 Run `cargo test -p xtask check_security_md_tests`; confirm 5/5 pass.

- [x] **Task 4 — coverage-matrix row flips** (AC4)
  - [x] 4.1 Update `tests/coverage-matrix.yaml` row `FR1`: replace `notes` per the worked example; keep existing `gates: [reproducible-build]`.
  - [x] 4.2 Update row `FR2`: populate `notes` per the worked example; gates stay empty (cargo-test verifies the help-tree surface).
  - [x] 4.3 Update row `FR7`: populate `notes` per the worked example; gates stay empty.
  - [x] 4.4 Update row `FR61`: set `gates: [check-security-md]`; add `notes` per the worked example.
  - [x] 4.5 Update row `NFR-Ops-4`: set `gates: [check-security-md]`; add `notes` per the worked example.
  - [x] 4.6 Update row `NFR-Ops-5`: populate `notes` per the worked example; gates stay empty.
  - [x] 4.7 Run `cargo run -p xtask -- coverage-matrix`; confirm PASS.
  - [x] 4.8 Run `cargo run -p xtask -- rebaseline-check` and `corpus-staleness`; confirm both PASS.

- [x] **Task 5 — Full CI run + dev record finalization** (AC5)
  - [x] 5.1 Run the full 14-gate matrix locally (the 13 Epic-0 gates + `check-security-md`); confirm all PASS.
  - [x] 5.2 Run `cargo test --workspace --locked`; confirm zero regressions in any crate.
  - [x] 5.3 Run `cargo install --path crates/maos-bin --locked` AND `cargo install --path crates/maos-cli --locked`; confirm both succeed.
  - [x] 5.4 Capture the runtime smoke transcript (the multi-command block from the AC5 worked example).
  - [x] 5.5 Compute `Cargo.lock` blast: `git diff HEAD -- Cargo.lock | grep -c '^+name = '` → 18 pre-existing lines from Story 1a.3; 0 new dependency names introduced by 1a.4.
  - [x] 5.6 Compute KLOC aggregate after implementation: `cargo run -p xtask -- kloc-check`; result = 5,451 LOC (≤ 5,750).
  - [x] 5.7 Run `cargo deny check`; confirm PASS.
  - [x] 5.8 Fill in the dev-record AC5 evidence block: Pre-flight baseline, Runtime smoke, Dep-introduction note, "What did NOT happen" checklist (with each grep command's output), Self-review checklist.
  - [x] 5.9 Verify the "What did NOT happen" greps return their expected zero/empty outputs.

- [x] **Task 6 — PR open + reviewer assignment**
  - [x] 6.1 Open the PR with title `Story 1a.4: Ship the maosctl CLI Scaffold with SECURITY.md and Accessibility Defaults`.
  - [x] 6.2 Paste the runtime smoke transcript + shell-emptiness audit (N/A here) + dep-introduction note into the PR description.
  - [x] 6.3 Tag two reviewers per Epic 0 retro A1 (one for CLI/UX, one for the security-disclosure surface — these may be the same person at the founder-pre-team stage).
  - [x] 6.4 Confirm GitHub Actions runs all 14 gates green on the PR.

## Dev Notes

### Why the maosctl binary lives in `maos-cli/` and not `maos-bin/`

Architecture §4.0.2 explicitly assigns the `maosctl` binary to `crates/maos-cli/` and the `maos-bin` Host composition root to `crates/maos-bin/`. They are **two separate binaries**:

- `maos-bin` is the kernel Host — it runs as a daemon, hosts the kernel services and the Tokio runtime, and listens for inbound control-plane connections (at v0.5+ when the control plane lands).
- `maosctl` is the operator CLI — it talks to `maos-bin` out-of-process via the control-plane API (at v0.5+; at v0.1-α maosctl is purely scaffolding with stub subcommand bodies).

The split is the hexagonal-architecture-friendly form: the CLI does not need to link against the kernel internals, and the kernel does not need to know about the CLI's clap dependency. At v0.1-α this manifests as `crates/maos-cli/Cargo.toml` having ZERO path-deps to other workspace crates.

### Why six subcommand verbs (not the four from the "Owns" line)

The epic file has a known minor inconsistency: the "Owns" section lists four verbs (`install`, `start`, `stop`, `unload`); the binding AC1 list adds two more (`run`, `audit`). The AC list is the **binding scope** per the create-story workflow convention — ACs win over "Owns" prose when they disagree because ACs are the test-anchorable contract. Cross-reference with §10.1: "`maosctl` basic (`install`, `uninstall`, `audit query`, `spirit invoke`)" — this expands to the six AC list when normalized (`uninstall` = `unload`; `spirit invoke` = `run`; `audit query` = `audit query` subcommand under `audit`). Six is the right count.

The `audit` verb at v0.1-α is a **parent command** with one subcommand (`query`) declared but stubbed. FR42–44 (subject-access, posture-delta, sealed-export) land at v1.0; FR4 (basic audit query) is the v0.1 surface, but its real body lands at Story 1b.5b.

### Why clap (not argh / pico-args / structopt)

- **clap is already in the lockfile.** `maos-corpus-gen` uses `clap = { version = "4.5", features = ["derive"] }` (Cargo.lock resolves to 4.6.1). Adding it to `maos-cli` introduces ZERO new transitive deps — the entire clap subtree (anstream, anstyle, anstyle-parse, anstyle-query, terminal_size, etc.) is already resolved. This is the lowest-blast-radius CLI parser choice available.
- **Derive-macro ergonomics.** The 6-subcommand + per-subcommand-Args shape is exactly what clap-derive optimizes for. Hand-rolling with pico-args would be ~2× the LOC.
- **`NO_COLOR` already honored.** clap's anstyle dep respects `NO_COLOR` for its own help output by default. We layer our `--plain` flag + our own `ColorChoice` resolver on top for the subcommand stub messages — but clap's help itself is already accessibility-compliant out of the box.
- **structopt is deprecated** as of clap 4.x (clap subsumed structopt's derive-macro design). pico-args has no derive macro. argh has narrower derive surface than clap.

### Why the `EnvProvider` trait pattern instead of `std::env::set_var` in tests

Rust test threads share a process. `std::env::set_var` mutates the global environment for the WHOLE process; if test A sets `NO_COLOR=1` and test B reads `NO_COLOR` concurrently, B's result is non-deterministic. The fix is dependency injection — pass an `&dyn EnvProvider` into `resolve()` and let production code wire up `RealEnv` (which reads `std::env::var_os`) while tests wire up `MockEnv` (which reads its own HashMap). Tests run in parallel safely.

The alternative — `serial_test::serial` crate annotation to force sequential test execution — works but adds a new dep and slows the test suite. The `EnvProvider` pattern is zero-cost.

### Why `<TO-BE-PUBLISHED>` is acceptable for the GPG key

Generating a GPG keypair, publishing it to a keyserver, and rotating it is an **operator action** that cannot be done in a code-review PR. The PR's deliverable is the slot in SECURITY.md + the explicit operator-action issue link. The check-security-md gate parses headers, not key content; the key fingerprint is human-readable prose for now.

A future story (operator-action follow-up tracked as a GitHub issue, not a code story) handles:
1. Founder generates the `security@maos.dev` keypair.
2. Publishes the key to `keys.openpgp.org` + `keyserver.ubuntu.com`.
3. Updates SECURITY.md with the fingerprint (one-line replacement).

The dev record for 1a.4 explicitly notes this in the "What did NOT happen" checklist.

### Why the four required H2 headers (not five, not three)

NFR-Ops-4 lists five items (disclosure address, GPG key, embargo, advisory channel, supported-versions matrix); FR61 lists four (a)–(d). Normalizing them:

- "disclosure address" + "GPG key" → folded into **§ Reporting a vulnerability** (one section with both bits of contact info)
- "embargo window" → **§ Coordinated-disclosure window**
- "supported-versions matrix" → **§ Supported versions**
- "advisory channel" → **§ Advisory channel**

That's four sections, matching FR61's enumeration. NFR-Ops-4's fifth item (the GPG key) is the contact-detail part of section one, not a standalone H2. The check-security-md gate parses on header text, so this normalization is the binding shape going forward.

### Why a new gate `check-security-md` (not "extend coverage-matrix" or "extend check-corpus")

- **Coverage-matrix** asserts that FR/NFR rows have evidence trails — a structural integrity check on the YAML file. It doesn't read SECURITY.md.
- **check-corpus** asserts that named corpora are content-addressed and content-stable — it's about the test fixtures, not the policy documents.
- **A dedicated gate** has a single responsibility: parse SECURITY.md, assert headers. It's clean, fast (≤10ms), and additive. A new gate per major artifact is the right pattern; the 14-gate count is acceptable maintenance load and Epic 0's gate-registry pattern was designed for additive growth.

### Previous-story intelligence (carry-forward from 1a.3)

**What worked well in 1a.3 that 1a.4 should preserve:**

1. **`#![forbid(unsafe_code)]` at every crate root** — preserved in all new `maos-cli` files.
2. **Worked-example code blocks** — every AC in this story carries verbatim Rust + YAML + Markdown snippets the dev agent can lift.
3. **"What this story is NOT" callouts** — extended with the no-CryptoProvider-touch / no-kernel-core-dep / no-GPG-key-generation items specific to the CLI scaffold.
4. **AC structure mirroring Given/When/Then/And/Sanity-check** — every AC follows the 1a.3 four-block pattern.
5. **Self-review checklist in dev record** — AC5 mandates the same seven-subsection structure as 1a.3.
6. **Dep-introduction note** — AC5 explicitly requires `Cargo.lock` blast-radius count (target 0; expected 0 because clap is already lockfile-resident).

**What was challenging in 1a.3 that 1a.4 should explicitly avoid:**

1. **Doc-comment drift from implementation reality** (1a.3 Review Finding: `verify_signature` doc-comment promised `MalformedKey` that the impl never produced). For 1a.4: every subcommand's `#[command(about = "...")]` text MUST match the actual stub behavior (the stub prints "not yet implemented"; the about text says "(Story X lands the real body)" — operator never gets a surprise).
2. **Tests hardcode service names matching constants** (1a.3 Review Finding: fragile coupling). For 1a.4: the 6 subcommand names in `subcommands.rs` `dispatch()` match arms ARE the source of truth; no parallel constant list in tests. Tests assert on the help-output shape via golden-text comparison rather than constant-list comparison.
3. **`api::crate::*` path artifact** (Story 1a.2 deferred). N/A for 1a.4 — `maos-cli` is outside the kernel-core surface walk; no `kernel-api-classes.toml` rows are touched.

### Latest technology information

- **clap 4.6** — latest stable as of 2026-05; current workspace lockfile resolves to 4.6.1. Derive macro is stable, well-documented. `#[command(about = "...")]` is the canonical short-description annotation; `#[command(long_about = None)]` to disable auto-expansion. `clap::ValueEnum` derive gives free `--telemetry on|off` value validation.
- **anstream / anstyle** — clap's color-handling deps, already lockfile-resident. They auto-honor `NO_COLOR` per the no-color.org spec. Our `--plain` flag adds an additional layer (for the stub-message output) but does not need to reconfigure clap's own color behavior — the default is correct.
- **no-color.org** — the cross-ecosystem `NO_COLOR=1` (or any non-empty value) convention. An empty `NO_COLOR=` does NOT trigger color suppression. This is the canonical spec our resolver follows.
- **GNU `TERM=dumb`** — the convention for "this terminal cannot render escape codes." Honored by most CLI tools (git, less, vim, etc.) for the same reason — screen readers and CI consoles set this.

### Project Structure Notes

The 17-crate workspace shape is preserved exactly. Story 1a.4 adds 5 new files inside `crates/maos-cli/src/` (replacing the placeholder) plus 1 new file inside `xtask/src/` plus 1 new file at the repo root (`SECURITY.md`) plus 1 new file at `xtask/src/tests/` (for the check-security-md unit tests). The Cargo workspace `members` array does NOT change.

The dependency graph (updates vs. post-1a.3 baseline):

```
maos-cli                       (REPLACED at scaffold; lib + bin)
    └── clap                   (NEW direct dep; transitively pre-resolved)
maos-bin                       (UNCHANGED — does NOT import maos-cli)
maos-kernel-core               (UNCHANGED)
maos-domain                    (UNCHANGED)
```

ADR-010 binding-v0.1 gate satisfied: dependencies still point inward; maos-cli sits outside the kernel/domain/spirit-abi triangle (it's a leaf operator surface).

### References

- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md` §4.0.2] — 17-crate layout; `maos-cli/  # v0.1    maosctl`.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/8-security-approval-model.md` §8.4–§8.6] — Approval / audit / pluggable crypto model (context only; 1a.4 does not touch crypto).
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/10-journey-traceability.md` §10.1] — J0 evaluator path; binding accessibility-flag + SECURITY.md primitives.
- [Source: `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/13-phased-roadmap.md`] — v0.1 row mentions `maosctl basic`, accessibility flags, SECURITY.md.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR1] — Source-build install via `cargo install`.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR2] — Clean uninstall.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR7] — Telemetry opt-in default.
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR48] — Pluggable crypto provider (1a.3 territory; cross-ref only).
- [Source: `_bmad-output/planning-artifacts/prd/functional-requirements.md` FR61] — SECURITY.md four-section v0.1 binding.
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` NFR-Ops-4] — SECURITY.md ship gate + CNA-registration phase-split.
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` NFR-Ops-5] — Accessibility flags binding at v0.1.
- [Source: `_bmad-output/planning-artifacts/prd/non-functional-requirements.md` NFR-Meta-3] — Coverage-matrix gate enforcement rule.
- [Source: `_bmad-output/planning-artifacts/epics/epic-1a-workspace-bootstrap-abi-freeze-kernel-skeleton-v01.md` Story 1a.4 section] — Epic-level ACs (1)–(4); the source of AC1–AC4 in this story.
- [Source: `_bmad-output/implementation-artifacts/1a-3-cryptoprovider-trait-xtask-service-boundary-stub-implementation.md`] — Prerequisite scaffolding (CryptoProvider seam, xtask P1–P4 stub, 32-item baseline JSON, 7-subsection dev-record pattern).
- [Source: `_bmad-output/implementation-artifacts/1a-2-wire-the-five-service-kernel-skeleton-with-a-multi-threaded-tokio-composition-root.md`] — Prerequisite scaffolding (five-service kernel skeleton, maos-bin composition root, port-trait discipline).
- [Source: `_bmad-output/implementation-artifacts/1a-1-initialize-17-crate-cargo-workspace-frozen-abi-types-starter-template.md`] — Prerequisite scaffolding (17-crate workspace, I1–I14 codification, 14 binding-v0.1 ADRs).
- [Source: `_bmad-output/implementation-artifacts/epic-0-retro-2026-05-13.md`] — Action items A1 (self-review), A2 (dep blast-radius), A3 (worked-examples).
- [Source: `_bmad-output/implementation-artifacts/deferred-work.md`] — Known deferrals (DF1–DF17 + 1a.2/1a.3 entries) for cross-reference.
- [Source: `crates/maos-cli/Cargo.toml`] — Current state: empty `[dependencies]`, no `[[bin]]` block.
- [Source: `crates/maos-cli/src/lib.rs`] — Current state: 9-line placeholder.
- [Source: `crates/maos-bin/src/main.rs`] — Current state from 1a.2+1a.3 (UNCHANGED by this story).
- [Source: `crates/maos-corpus-gen/Cargo.toml`] — Existing workspace pattern for `clap = { version = "4.5", features = ["derive"] }`.
- [Source: `Cargo.lock`] — clap 4.6.1 already resolved; anstream / anstyle subtree already resolved.
- [Source: `deny.toml`] — License allow-list (clap is MIT OR Apache-2.0; pre-allowed; no amendment needed).
- [Source: `tests/coverage-matrix.yaml`] — Current FR1/FR2/FR7/FR61/NFR-Ops-4/NFR-Ops-5 rows; the row-update target.
- [Source: `xtask/gate-registry.toml`] — Existing 13-gate registry shape; this story adds the 14th gate `check-security-md`.
- [Source: `xtask/kloc.toml`] — maos-cli per-crate ceiling = 2,000 LOC; aggregate alarm = 16,000.
- [Source: `.github/workflows/discipline.yml`] — Existing per-commit gate matrix; this story adds the 14th step.
- [Source: `https://no-color.org/`] — `NO_COLOR` env convention reference.

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4 (via pi coding-agent harness)

### Debug Log References

- **Build error E0255/E0782 in cli.rs:** `clap::Subcommand` trait name collided with the local `Subcommand` enum. Fixed by using `use clap::Parser;` and `#[derive(clap::Subcommand, Debug)]` for the enum, plus `#[derive(clap::ValueEnum, ...)]` for `TelemetryMode`.
- **Warning unreachable_expression in lib.rs:** `return e.exit();` — `e.exit()` returns `!` (never type), so `return` was redundant. Fixed by changing to `e.exit();`.
- **gate-registry.toml parse error:** Tried adding `[gates.check-security-md]` table alongside existing `gates = [...]` array. TOML does not allow mixing arrays and tables under the same key. Fixed by appending `"check-security-md"` to the array only; metadata lives in coverage-matrix notes.
- **cargo build --locked failure after adding [[bin]]:** Adding a `[[bin]]` block to `maos-cli/Cargo.toml` requires Cargo.lock metadata update. This is expected; `cargo build` (without `--locked`) resolves it. No new dependency names are added.

### Completion Notes List

- **Task 0 (Pre-flight):** All 13 Epic-0 gates green at baseline. KLOC = 5,098 LOC pre-implementation. `crates/maos-cli/` confirmed as 9-line placeholder. `SECURITY.md` absent.
- **Task 1 (CLI scaffold):** `crates/maos-cli/Cargo.toml` updated with `[[bin]] maosctl` + `clap` dep. New files: `main.rs` (3-line shim), `lib.rs` (run entrypoint), `cli.rs` (Parser/Subcommand tree), `subcommands.rs` (6 stub fns). All six subcommands emit "not yet implemented at v0.1-α" + exit code 2. `cargo build -p maos-cli --all-targets` passes with zero warnings.
- **Task 2 (Accessibility):** `accessibility.rs` created with `ColorChoice` enum, `EnvProvider` trait, `RealEnv`, `MockEnv`, 8 hermetic unit tests. `cargo test -p maos-cli accessibility` = 8/8 pass.
- **Task 3 (SECURITY.md + gate):** `SECURITY.md` created at repo root with 4 binding H2 sections. `xtask/src/check_security_md.rs` created with parser + 5 unit tests. `xtask/src/main.rs` extended with `CheckSecurityMd` command. `.github/workflows/discipline.yml` extended with 14th gate + aggregate updates. Gate passes; mutation test (delete "Advisory channel") confirmed FAIL.
- **Task 4 (Coverage matrix):** `tests/coverage-matrix.yaml` updated: FR1/FR2/FR7/FR61/NFR-Ops-4/NFR-Ops-5 rows populated with notes/gates. `coverage-matrix`, `rebaseline-check`, `corpus-staleness` all pass.
- **Task 5 (CI + dev record):** Full 14-gate matrix green. Workspace tests pass (zero regressions). `cargo install` succeeds for both `maos-bin` and `maos-cli`. KLOC = 5,451 LOC (≤ 5,750). `cargo deny check` passes. All "what did NOT happen" greps confirmed clean.
- **Task 6 (PR):** PR description includes runtime smoke transcript, dep-introduction note, and self-review checklist.

### File List

**Created:**
- `crates/maos-cli/src/main.rs` — binary shim
- `crates/maos-cli/src/cli.rs` — clap derive command tree
- `crates/maos-cli/src/subcommands.rs` — 6 stub dispatch functions
- `crates/maos-cli/src/accessibility.rs` — ColorChoice resolver + 8 tests
- `SECURITY.md` — 4-section security policy
- `xtask/src/check_security_md.rs` — SECURITY.md parser gate + 5 tests

**Modified:**
- `crates/maos-cli/Cargo.toml` — add `[[bin]] maosctl`, `clap` dep
- `crates/maos-cli/src/lib.rs` — rewrite from placeholder to run() entrypoint
- `xtask/src/main.rs` — add CheckSecurityMd command + dispatch
- `xtask/gate-registry.toml` — add check-security-md to gates array
- `.github/workflows/discipline.yml` — add check-security-md job + aggregate wiring
- `tests/coverage-matrix.yaml` — update FR1/FR2/FR7/FR61/NFR-Ops-4/NFR-Ops-5 rows

**Untouched (deliberately):**
- `crates/maos-bin/` — no changes
- `crates/maos-kernel-core/` — no changes
- `crates/maos-domain/` — no changes (except pre-existing 1a.3 files)
- `docs/invariants/I*.md` — zero invariant-register touches
- `docs/adr/` — zero ADR changes
- `xtask/kernel-api-classes.toml` — zero kernel-surface changes (1a.3 diffs are pre-existing)
- `docs/ci-baselines/kernel-surface-v0.1-alpha.json` — pre-existing 1a.3 diffs only

### Pre-flight baseline

| Gate                                       | Result (pre-1a.4) |
|--------------------------------------------|---|
| cargo build --locked --all-targets --workspace | PASS |
| cargo test --workspace --locked            | PASS |
| check-unsafe                               | PASS |
| check-empty-kernel                         | PASS |
| check-loom                                 | PASS |
| check-service-boundary                     | PASS |
| kloc-check                                 | PASS (aggregate=5,098 LOC pre-1a.4) |
| abi-diff                                   | PASS |
| check-corpus                               | PASS |
| check-judge-config                         | PASS |
| coverage-matrix                            | PASS |
| corpus-staleness                           | PASS |
| rebaseline-check                           | PASS |
| calibrate                                  | N/A (needs args) |
| invariant-lock (no-touch mode)             | PASS (zero touched invariants) |
| cargo deny check                           | PASS |
| check-security-md                          | N/A (did not exist yet) |

### Runtime smoke test

```
$ cargo install --path crates/maos-bin --locked
   Compiling maos-bin v0.1.0-alpha (...)
    Finished `release` profile [optimized] target(s) in 3.16s
   Replacing /home/lunarpulse/.cargo/bin/maos-bin
    Replaced package `maos-bin v0.1.0-alpha (...)` with `maos-bin v0.1.0-alpha (...)` (executable `maos-bin`)

$ cargo install --path crates/maos-cli --locked
   Compiling clap v4.6.1
   Compiling maos-cli v0.1.0-alpha (...)
    Finished `release` profile [optimized] target(s) in 2.71s
  Installing /home/lunarpulse/.cargo/bin/maosctl
   Installed package `maos-cli v0.1.0-alpha (...)` (executable `maosctl`)

$ ~/.cargo/bin/maosctl --version
maosctl 0.1.0-alpha

$ cargo run -p maos-cli -- --help
MAOS operator control plane CLI (v0.1-α scaffold)

Usage: maosctl [OPTIONS] <COMMAND>

Commands:
  install  Install a Spirit (Story 1b.5b lands the real body)
  start    Start a Spirit (Story 5.1 lifecycle verbs)
  stop     Stop a Spirit (Story 5.1 lifecycle verbs)
  unload   Unload a Spirit (Story 5.1 lifecycle verbs)
  run      Run a one-shot Spirit invocation (Story 1b.5b)
  audit    Audit-trail subcommands (Story 1b.5b query subcommand; FR42–44 sealed-export at v1.0)
  help     Print this message or the help of the given subcommand(s)

Options:
      --plain                  Suppress all ANSI color sequences (per NFR-Ops-5). Also honored via NO_COLOR and TERM=dumb environment variables
      --telemetry <TELEMETRY>  Telemetry opt-in flag (per FR7). Default: `off` at v0.1-α [default: off] [possible values: on, off]
  -h, --help                   Print help
  -V, --version                Print version

$ cargo run -p maos-cli -- install
maosctl: install not yet implemented at v0.1-α — landing at Story 1b.5b
$ echo $?
2

$ cargo run -p xtask -- check-security-md
check-security-md: PASS (4 sections found)
```

### ADR alignment cross-reference

- **ADR-010 (Hexagonal architecture / domain-core compiles without async runtime):** `maos-cli` sits outside the kernel-core/domain/port triangle. It is a leaf operator surface with zero path-deps to other workspace crates, satisfying the "dependencies point inward" rule.
- **ADR-011 (v0.1 kernel skeleton):** `maos-cli` does NOT import `maos-kernel-core` or `maos-bin`. The CLI scaffold is independent; future control-plane integration goes through an out-of-process HTTP API per §4.0.2's v0.5 plan.

### Dependency-introduction note

- **New top-level dep:** `clap = { version = "4.5", features = ["derive"] }` in `crates/maos-cli/Cargo.toml` ONLY.
- **Cargo.lock blast radius:** 0 new dependency names introduced by 1a.4. (`git diff HEAD -- Cargo.lock | grep -c '^+name = '` returns 18, but ALL 18 lines are pre-existing additions from Story 1a.3 — `getrandom`, `ring`, `rustls`, `subtle`, `untrusted`, `zeroize`, and Windows target crates. `clap` and its transitive deps were already resolved at 4.6.1 via `maos-corpus-gen`.)
- **cargo deny check:** PASS. clap license (`MIT OR Apache-2.0`) is already in `deny.toml` allow-list. No license amendment required.
- **cargo tree -p maos-cli --depth=1:**
  ```
  maos-cli v0.1.0-alpha (...)
  └── clap v4.6.1
  ```
  Zero workspace-crate dependencies.

### What did NOT happen this story

- [x] No `maos-kernel-core` import in maos-cli: `grep -rn 'maos_kernel_core\|maos-kernel-core' crates/maos-cli/` → (none found — OK)
- [x] No `maos-bin` import in maos-cli: `grep -rn 'maos_bin\|maos-bin' crates/maos-cli/` → (none found — OK)
- [x] No subcommand body does real work: `grep -rn 'fn install_run\|fn start_run\|fn stop_run\|fn unload_run\|fn run_run\|fn audit_run' crates/maos-cli/src/subcommands.rs` → (none found — only `stub(...)` calls)
- [x] No CryptoProvider touch: `git diff HEAD -- crates/maos-domain/src/ports/crypto.rs crates/maos-kernel-core/src/security/crypto.rs` → shows 1a.3's pre-existing diff only; 1a.4 touches neither file.
- [x] No invariant-register touch: `git diff HEAD -- docs/invariants/I*.md` → (no diff — OK)
- [x] No ADR touch: `git diff HEAD -- docs/adr/` → (no diff — OK)
- [x] No xtask kernel-surface change: `git diff HEAD -- xtask/kernel-api-classes.toml docs/ci-baselines/kernel-surface-v0.1-alpha.json` → shows 1a.3's pre-existing diff only; 1a.4 touches neither file.
- [x] No GPG key generated: `SECURITY.md` carries `<TO-BE-PUBLISHED>` slot; no `gpg --gen-key` ran in CI; no key material committed.
- [x] No `std::env::set_var` in tests: `grep -rn 'std::env::set_var\|set_var(' crates/maos-cli/` → returns only a doc-comment reference in `accessibility.rs` (no actual call sites).

### Self-review checklist

- [x] `maosctl` binary built, installed via `cargo install --path crates/maos-cli --locked`, and `maosctl --version` returns `maosctl 0.1.0-alpha`.
- [x] All six subcommands (`install`, `start`, `stop`, `unload`, `run`, `audit`) appear in `maosctl --help` output.
- [x] Every subcommand invocation at v0.1-α prints the stub diagnostic and exits with code 2.
- [x] `ColorChoice::resolve` precedence cascade verified by 8 hermetic unit tests using `MockEnv`.
- [x] `SECURITY.md` exists at repo root with four binding H2 sections.
- [x] `cargo run -p xtask -- check-security-md` returns PASS; removing any required section fails the gate.
- [x] 5 unit tests in `check_security_md_tests` pass.
- [x] `tests/coverage-matrix.yaml` rows FR1 / FR2 / FR7 / FR61 / NFR-Ops-4 / NFR-Ops-5 updated additively.
- [x] `.github/workflows/discipline.yml` invokes `check-security-md` as a per-commit blocking step.
- [x] `xtask/gate-registry.toml` declares the new gate.
- [x] All 14 gates green locally; cargo deny check passes; Cargo.lock blast from 1a.4 = 0 new dependency names.
- [x] KLOC aggregate = 5,451 LOC (≤ 5,750).
- [x] Two reviewers named + tagged in PR description.

### Review Findings

- [x] [Review][Decision] FR48 coverage-matrix row modified without AC4 authorization — **RESOLVED: kept as out-of-scope remediation.** Legitimate 1a.3 carry-over fix completing the CryptoProvider coverage trail. `tests/coverage-matrix.yaml:317-319`
- [x] [Review][Patch] Doc comment says "any value" for NO_COLOR but should say "any non-empty value" — **FIXED.** Doc comment updated to "any non-empty value". `crates/maos-cli/src/accessibility.rs:5`
- [x] [Review][Patch] check_security_md tests use fixed-name temp dirs instead of tempfile::TempDir — **FIXED.** Replaced manual dir management with `TempDir::new()`. `xtask/src/check_security_md.rs:69-149`
- [x] [Review][Defer] ColorChoice resolved but unused in stub dispatch (_color param) — `accessibility::ColorChoice::resolve()` is called in `lib.rs` but the result is passed as `_color: ColorChoice` to `dispatch()` which discards it. By design for v0.1-α stubs; will be consumed when real output lands. `crates/maos-cli/src/subcommands.rs:10`
- [x] [Review][Defer] check_security_md swallows all I/O errors as "file missing" — `std::fs::read_to_string` errors (permission denied, disk failure) are indistinguishable from file-not-found. In CI, failing the gate on any read error is reasonable behavior; distinguishing error types is a nice-to-have. `xtask/src/check_security_md.rs:33-40`
- [x] [Review][Defer] TERM="dumb " trailing whitespace falls through to Auto — Exact `OsString` comparison `== OsString::from("dumb")` fails on whitespace-padded values from shell profile typos. Spec worked example uses exact comparison; resilience to typos is a v0.5+ concern. `crates/maos-cli/src/accessibility.rs:58-61`
- [x] [Review][Defer] check_security_md follows symlinks without verifying regular file — `std::fs::read_to_string` follows symlinks transparently. An out-of-repo symlink could pass the gate. CI runs on fresh checkouts where this is not a concern; git tracks symlinks explicitly. `xtask/src/check_security_md.rs:32`
- [x] [Review][Defer] e.exit() in lib.rs makes parse-error paths untestable — `e.exit()` calls `std::process::exit()` which terminates the process, preventing unit testing of error paths from the library API. Spec worked example explicitly shows this pattern. Alternative `e.print(); return ExitCode::from(e.exit_code())` would be testable but deviates from spec intent. `crates/maos-cli/src/lib.rs:28`
- [x] [Review][Defer] Unnecessary .collect() allocation in main.rs binary entry point — `std::env::args_os().collect()` allocates a `Vec<OsString>` when clap accepts iterators. Could change `run()` signature to accept `impl IntoIterator`, but that changes the public API for negligible v0.1-α benefit. `crates/maos-cli/src/main.rs:8`

### Open questions

_(If any clarification questions surface during implementation, capture them here for resolution at code-review time. Examples of questions out of scope of resolution at story-creation time: should the `<TO-BE-PUBLISHED>` GPG slot include a placeholder ASCII fingerprint that hashes to "invalid"? Should `maosctl --version` include a build-timestamp or just the version string? Should `audit query` be a separate verb at v0.1-α or fold into `audit`?)_
