#![forbid(unsafe_code)]

//! `xtask demo-j1` — the one-command J1 founder-loop scene.
//!
//! Story `j1-tier2-live-agent-signed-bridge` closed J1's LOCAL leg with a signed
//! bundle, and `j1-crosshost-1a` made the delegation frame-borne. Neither left a
//! way to *watch* it: the entry points were a raw `maos run … --once` whose
//! evidence is an NDJSON stream, and a six-phase operator runbook. This runner is
//! the missing observable surface.
//!
//! Like [`crate::demo_reza`], it is orchestration ONLY. It re-implements no
//! oracle and owns no evidence: the adapter's `parse_completion` remains the
//! completion authority, `check-j1-loopback-delegation` remains the judge of the
//! wire, and a published ledger — when one exists — outranks anything narrated
//! here.
//!
//! ## The honest-labeling contract
//!
//! A demo that only shows what works is an advertisement. This one declares the
//! FULL J1 beat set, including beats whose substrate has not landed, and renders
//! those `ABSENT` against the story that owns them. Three rules hold:
//!
//! 1. `ABSENT` never becomes green, and an unlanded beat never fails the run — it
//!    is a visible placeholder, not a silent skip (the Family-B discipline from
//!    Story 13.6e).
//! 2. Loopback beats are labeled `v0.8 rung — loopback rehearsal`.
//!    `two-host-delegation` is judged separately; the later
//!    `two-host-signed-run` rung belongs to `j1-crosshost-2d-paid-two-host-run`
//!    (RF-0, 2026-08-18: `2c` owns the judge, `2d` owns the run).
//! 3. A fixture take never claims the Tier-2 beat. Only `--live-codex` ending in
//!    a verified sealed bundle earns `PROVEN_LIVE_SIGNED`; the flag selects LIVE
//!    and the `--codex-topology` Worker selects the adapter (codex or claude —
//!    e.g. `spirits/topologies/j1-founder-loop-claude.toml`, Story 19-3 D-08 A).
//!
//! Every run is provisioned into an ISOLATED state home (`MAOS_HOME` +
//! `XDG_DATA_HOME`). That is not tidiness: on the operator's shared
//! `~/.local/share/maos` a pre-existing corrupted lifecycle journal prints
//! warnings that have nothing to do with this run, and a demo that narrates
//! ambient noise as its own output is lying quietly. With a fresh home, a
//! `journal:` warning is a real finding.
//!
//! ## The epic scene (Story 19-4)
//!
//! The hermetic default replays the whole founder loop on
//! `j1-founder-loop-epic.toml` with `--epic tests/fixtures/j1/founder-loop-epic.md`
//! (D-19-4-09 A): the Orchestrator splits three `### Story` sections, judges each
//! before its frame (the replayed cassette is the env pair `MAOS_INFERENCE_MODE` /
//! `MAOS_REPLAY_CASSETTE` — no flag, ADR-064), the KERNEL halts the ambiguous
//! section, this runner answers the halt over the operator door while the child's
//! stdout streams, the section resumes with the answer, and the run ends with an
//! FR17 digest whose every cited ref must resolve to an audit row. The halt is
//! resolved in the same boot (D-19-4-05 A): a halt still pending at exit is not
//! resumed by a later boot, and this runner never claims it is.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::gate_common::EvidenceState;

/// The scene's topology: the Orchestrator, two class Spirits and one
/// frame-borne Developer-Worker per epic section (Story 19-4 D-19-4-09 A). The
/// `host` key on each worker entry is what makes the delegation frame-borne
/// (`j1-crosshost-1a` AC1.1).
const TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";

/// Story 19-4 D-19-4-04 A — the epic the scene decomposes: three `### Story`
/// sections, section 2 under-specified on purpose (its acceptance names no
/// measurable bound).
const EPIC: &str = "tests/fixtures/j1/founder-loop-epic.md";
/// One Worker per section: the topology's three host-bearing entries.
const EPIC_SECTIONS: usize = 3;
/// The section whose replayed judgement exceeds the manifest's 0.5 rule.
const AMBIGUOUS_SECTION: u64 = 2;
/// The scalar tag the Orchestrator's halt is journaled under (D-19-4-08 A).
const AMBIGUITY_TAG: &str = "dispatch_ambiguity";
/// D-19-4-05 A — how long the child holds a halted section for the door's
/// answer. Generous on purpose: this runner resolves the halt within one read
/// of the stream, and the bound only decides how long a lost answer takes to
/// surface as the 19-1 stop.
const HALT_HOLD_ENV: &str = "MAOS_EPIC_HALT_HOLD_SECS";
const HALT_HOLD_SECS: &str = "120";
/// The operator's answer to the ambiguity halt, sent over the door as
/// `provided-context`; the resumed section's goal must carry it verbatim.
const CLARIFICATION: &str = "The nightly import of the 10,000-row fixture must finish in \
                             under 60 seconds; acceptance is one timed run under that bound.";

/// Destination peer id / topology `host` the delegation must reach.
const DELEGATION_HOST: &str = "developer-remote-host";
/// The Spirit the `task.assign` is addressed to — NOT the `"worker"` subprocess
/// label (the Worker is a subprocess, not a mailbox peer).
const DELEGATION_RECIPIENT: &str = "developer-remote";

/// The gate that judges the wire. Hermetic, `Blocking`, shipped by 1a.
const DELEGATION_GATE: &str = "check-j1-loopback-delegation";

/// Where gate ledgers are published, when they are.
const REPORTS_DIR: &str = "tests/reports";

/// Current Worker controls; emitted only for the verified T3 route.
const EGRESS_POSTURE: &str = "enforced";
const FS_JAIL_POSTURE: &str = "maos-enforced";

/// One narrated beat of the journey.
///
/// `executed` separates "this run exercised it and it held" from "declared so you
/// can see it is missing". Only an executed beat can fail the scene.
struct Beat {
    name: &'static str,
    /// What the beat means in plain language, for the narration.
    narration: &'static str,
    state: EvidenceState,
    /// The observed particulars — what was actually seen, or why not.
    detail: String,
    executed: bool,
    /// For a non-executed beat: the story or follow-up that owns it.
    owner: Option<&'static str>,
}

impl Beat {
    fn executed(
        name: &'static str,
        narration: &'static str,
        state: EvidenceState,
        detail: String,
    ) -> Self {
        Self {
            name,
            narration,
            state,
            detail,
            executed: true,
            owner: None,
        }
    }

    fn absent(name: &'static str, narration: &'static str, owner: &'static str) -> Self {
        Self {
            name,
            narration,
            state: EvidenceState::Absent,
            detail: format!("not exercised — owned by {owner}"),
            executed: false,
            owner: Some(owner),
        }
    }

    /// An executed beat that did not hold. An ABSENT placeholder is never this.
    fn failed(&self) -> bool {
        self.executed && !self.state.is_proven()
    }
}

/// Everything the scene observed in one `maos run … --once`.
struct SceneObservation {
    events: Vec<serde_json::Value>,
    stderr: String,
    wall: std::time::Duration,
    exit_ok: bool,
}

impl SceneObservation {
    fn event(&self, name: &str) -> Option<&serde_json::Value> {
        self.events
            .iter()
            .find(|e| e["event"].as_str() == Some(name))
    }

    fn events_named(&self, name: &str) -> Vec<&serde_json::Value> {
        self.events
            .iter()
            .filter(|e| e["event"].as_str() == Some(name))
            .collect()
    }
}

pub fn run(
    live_codex: bool,
    codex_topology: Option<PathBuf>,
    keep_home: Option<PathBuf>,
    skip_build: bool,
    skip_gate: bool,
) -> Result<(), String> {
    banner("J1 founder loop — an orchestrator, a reviewer, and a remote that is still loopback");

    for required in [TOPOLOGY, EPIC] {
        if !Path::new(required).exists() {
            return Err(format!(
                "demo-j1: {required} not found — run this from the repository root"
            ));
        }
    }

    let bins = preflight(skip_build)?;
    let home = provision_home(keep_home.as_deref())?;
    // Every `?` below returns before the "Where it landed" block, so without this
    // guard a failed run leaves its state home behind. The guard is the backstop;
    // the normal path still removes it explicitly so the result can be reported.
    let _home_guard = EphemeralHome {
        path: home.clone(),
        keep: keep_home.is_some(),
    };

    section("The scene");
    for (name, what) in [
        (
            "P1  orchestrator/architect/reviewer",
            "three class Spirits load from the topology",
        ),
        (
            "E   epic -> three judged sections",
            "the Orchestrator splits the epic and judges each section before its frame",
        ),
        (
            "P2  task.assign -> developer-remote",
            "a real consent-carrying frame per section over the loopback A2A layer",
        ),
        (
            "P3  Developer-Workers admitted",
            "host-managed grant, T3, a real child process per section",
        ),
        (
            "P4  completion parsed by the adapter",
            "the oracle decides completion — never the exit code",
        ),
        (
            "P5  delegations closed at a safe point",
            "TaskComplete journaled, no frame left in flight",
        ),
        (
            "H   the kernel halts section 2",
            "answered over the door, section 2 resumes; the FR17 digest cites real rows",
        ),
        (
            "P6  audit drain + clean exit",
            "every queued row reaches SQLite before the process leaves",
        ),
    ] {
        println!("  {name:<38} {what}");
    }
    println!();
    println!("  The state home is isolated for this run, so any `journal:` warning below is");
    println!("  this run's own finding and not the operator's ambient state.");

    section("Running the founder loop");
    let obs = run_scene(&bins, &home)?;
    println!(
        "  {} events, exit {}, {:.3}s wall",
        obs.events.len(),
        if obs.exit_ok { "0" } else { "nonzero" },
        obs.wall.as_secs_f64()
    );

    let mut beats = evaluate_beats(&obs);

    // Story 19-4 AC3 — the digest's citations, joined to an independent read of
    // the same window. An unreadable window cites nothing, so the beat fails.
    let audit_rows = audit_query(&bins, &home).unwrap_or_else(|why| {
        println!("  audit query      FAILED: {why}");
        String::new()
    });
    beats.push(digest_cites_source_refs(&obs, &audit_rows));

    // T5 — the sealed-export parity control. The drain fix is only meaningful if
    // the rows it flushed are actually COVERED by a signature, so cross-check an
    // independent reader (`audit query`) against what the signer sealed. Without
    // this the scene could render `audit-drain-clean` green while an export over
    // the same window silently omitted rows.
    beats.push(match sealed_export_parity(&bins, &home) {
        Ok((queried, exported)) => Beat::executed(
            "sealed-export-covers-the-run",
            "what an independent reader sees in the window is what the signer sealed",
            state_of(queried > 0 && exported >= queried),
            format!("audit query saw {queried} row(s); the signed bundle covers {exported}"),
        ),
        Err(why) => Beat::executed(
            "sealed-export-covers-the-run",
            "what an independent reader sees in the window is what the signer sealed",
            EvidenceState::Indeterminate,
            format!("parity NOT established: {why}"),
        ),
    });

    if skip_gate {
        // One ABSENT beat per leg, not one for the gate: a claim table that says
        // "the gate did not run" under a single leg's name is a claim about the
        // wrong thing.
        for leg in crate::check_j1_loopback_delegation::ledger_leg_names() {
            beats.push(Beat::absent(
                leg,
                "the wire is judged by a Blocking gate, not by this narration",
                "--skip-gate was passed",
            ));
        }
        // §A6 review P5 — the conjunction beat LEFT `unlanded_beats()` and is
        // emitted only by `run_delegation_gate()`, which this branch skips:
        // omit it here and a `--skip-gate` claim table silently stops claiming
        // the refusal work exists at all. Honest labeling means ABSENT, not
        // absent-from-the-table.
        beats.push(Beat::absent(
            "disallowed-intent-refused-blocking",
            "a disallowed intent must be REFUSED (-32001 CODE_INTENT_DENIED, distinct from -32009)",
            "--skip-gate was passed",
        ));
        beats.push(absent_two_host_delegation());
    } else {
        section("Running the judge");
        beats.extend(run_delegation_gate());
    }

    beats.extend(unlanded_beats());

    if live_codex {
        section("Tier-2 live signed take");
        let (state, detail) = match live_codex_take(&bins, codex_topology.as_deref(), &home) {
            Ok(entries) => (
                EvidenceState::ProvenLiveSigned,
                format!("sealed bundle verified OK ({entries} entries)"),
            ),
            Err(why) => (
                EvidenceState::Indeterminate,
                format!("attempted and NOT signed: {why}"),
            ),
        };
        if let Some(beat) = beats.iter_mut().find(|b| b.name == TIER2_BEAT) {
            beat.state = state;
            beat.detail = detail;
            beat.executed = true;
            beat.owner = None;
        }
    }

    // j1-crosshost-2c AC5.3 — the executed-leg flip, before published ledgers so a
    // real ledger still outranks it.
    apply_two_host_signed_run(&mut beats);
    apply_published_ledgers(&mut beats);

    section("Claim table (execution order)");
    for beat in &beats {
        println!("{}", render_beat_row(beat));
        println!("       {:<38} {}", "", beat.narration);
    }
    section("What this run does NOT claim");
    println!("  rung             v0.8 — loopback rehearsal. `developer-remote` is a peer id on");
    println!("                   THIS host; no packet left the machine. This demo does not");
    println!("                   exercise the cross-host path: the crossing is proven in two logs");
    println!(
        "                   by `two-host-delegation`; Rung 2b binds it to a TLS-verified identity."
    );
    println!("  peer auth        on loopback `frame.from.host_id` is self-asserted — the frame");
    println!("                   picks which allowlist judges it. The gate's");
    println!("                   loopback-from-host-unverified leg remains the permanent");
    println!("                   loopback-arm boundary.");
    println!("  cap mediation    the cli_wrapper token path proceeds under host-grant authority;");
    println!("                   kernel `proc.exec` mediation is an Epic-9 operator-policy");
    println!("                   surface, and a Cedar permit alone cannot green it. The");
    println!("                   CapabilityInvocation exit row IS journaled either way.");
    println!("  egress           {EGRESS_POSTURE} — exact manifest/host-grant intersection");
    println!("                   through the scoped proxy; direct container networking disabled.");
    println!("  fs jail          {FS_JAIL_POSTURE} — verified T3 container, read-only root,");
    println!("                   non-root payload and bounded tmpfs; adapter restrictions are");
    println!("                   additional controls, not the MAOS boundary.");
    println!("  halt/resume      the ambiguity halt is resolved over the door IN THE SAME BOOT");
    println!("                   (D-19-4-05 A); a halt still pending when the process exits");
    println!("                   is not resumed by a later boot, and nothing here claims it.");

    section("Where it landed");
    match &keep_home {
        // AC5 forbids absolute workstation paths in scene output. The operator
        // supplied this path, so naming its leaf is enough to find it again.
        Some(path) => println!(
            "  state home       kept at your --keep-home path (leaf `{}`)",
            path.file_name().unwrap_or_default().to_string_lossy()
        ),
        None => {
            remove_worker_workspaces(&home);
            match std::fs::remove_dir_all(&home) {
                Ok(()) => {
                    println!("  state home       ephemeral, removed on exit (pass --keep-home to inspect)")
                }
                Err(e) => println!("  state home       ephemeral, but removal FAILED: {e}"),
            }
        }
    }
    println!("  runbook          _bmad-output/test-artifacts/runbook-j1-demo.md");

    let failures: Vec<&str> = beats
        .iter()
        .filter(|b| b.failed())
        .map(|b| b.name)
        .collect();
    if !failures.is_empty() {
        return Err(format!(
            "demo-j1: {} executed beat(s) did not hold: {}",
            failures.len(),
            failures.join(", ")
        ));
    }
    if !obs.exit_ok {
        return Err("demo-j1: the founder loop exited nonzero — see the output above".to_string());
    }
    println!();
    println!("  Every EXECUTED beat held. Read the table, never the exit code: this run exits");
    println!("  0 with ABSENT beats outstanding, and that is the honest state of J1 today.");
    Ok(())
}

/// Removes the isolated state home on EVERY return path, including the early `?`
/// ones. `remove_dir_all` on an already-removed home is ignored, so the normal
/// path can still delete it explicitly and report the outcome.
struct EphemeralHome {
    path: PathBuf,
    keep: bool,
}

impl Drop for EphemeralHome {
    fn drop(&mut self) {
        if !self.keep {
            remove_worker_workspaces(&self.path);
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// Story 19-3 — a live Worker's files under `<home>/workspaces` belong to its
/// mapped subuid (podman's `:U`), so `remove_dir_all` hits EACCES on them and
/// stops partway, leaving the rest of the home behind. Remove them inside
/// podman's user namespace first. Best-effort and bounded: a failure surfaces
/// as the home's own removal failure.
fn remove_worker_workspaces(home: &Path) {
    let workspaces = home.join("workspaces");
    if !workspaces.exists() {
        return;
    }
    let _ = Command::new("timeout")
        .args(["-k", "5s", "120s", "podman", "unshare", "rm", "-rf"])
        .arg(&workspaces)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// The Tier-2 beat name, referenced from both the declaration and the live take.
const TIER2_BEAT: &str = "tier2-live-agent-signed";

/// The founder loop has exactly three class Spirits, so `on_idle_fired` must
/// arrive three times. Fewer means a Spirit never went idle.
const EXPECTED_IDLE_FIRES: usize = 3;

/// The `worker_cli.name()` values a Tier-2 take may sign for — an ALLOWLIST, not
/// a single provider (`maos-bin/src/worker_cli.rs` `SUPPORTED_WORKER_CLIS` =
/// fixture | codex | claude).
///
/// j1-crosshost-2a AC3.4 — this WIDENED from the literal `"codex"`; it was NOT
/// deleted, and it must never be. The topology is OPERATOR-authored, so nothing
/// else stops the hermetic fixture from reaching the signing path and earning a
/// Tier-2 label. The check is an anti-overclaim control, not a hardcode: adding
/// `claude` is a widening; removing the check is a regression to
/// "worker-cli-fixture earns PROVEN_LIVE_SIGNED".
const SIGNABLE_WORKER_CLIS: &[&str] = &["codex", "claude"];

/// Resolved binaries the scene drives.
struct Bins {
    maos: PathBuf,
    /// Directory holding the daemon and its `worker-cli-fixture` sibling.
    dir: PathBuf,
}

fn preflight(skip_build: bool) -> Result<Bins, String> {
    section("Preflight");
    for name in [
        "MAOS_T3_RUNTIME",
        "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
    ] {
        if std::env::var_os(name).is_none() {
            return Err(format!(
                "demo-j1: T3 Worker substrate missing {name}; run scripts/provision_t3_tests.py and source its t3.env"
            ));
        }
    }
    let dir = PathBuf::from("target/debug");
    let maos = dir.join("maos");
    let fixture = dir.join("worker-cli-fixture");
    let maosctl = dir.join("maosctl");

    if skip_build {
        println!("  build            skipped (--skip-build)");
    } else {
        println!("  build            cargo build --workspace");
        let status = Command::new("cargo")
            .args(["build", "--workspace"])
            .status()
            .map_err(|e| format!("demo-j1: cannot run cargo: {e}"))?;
        if !status.success() {
            return Err("demo-j1: workspace build failed".to_string());
        }
    }

    for (label, path) in [
        ("maos", &maos),
        ("worker-cli-fixture", &fixture),
        ("maosctl", &maosctl),
    ] {
        if !path.exists() {
            return Err(format!(
                "demo-j1: {label} not found at {} — build the workspace first (drop --skip-build)",
                path.display()
            ));
        }
    }
    // ABSOLUTE, never repo-relative: the PATH prepend and the `maosctl` sibling
    // lookup must not depend on the cwd the daemon child happens to inherit.
    let dir = dir
        .canonicalize()
        .map_err(|e| format!("demo-j1: cannot resolve target/debug: {e}"))?;
    let maos = dir.join("maos");
    println!("  binaries         maos + worker-cli-fixture present as siblings in target/debug");
    Ok(Bins { maos, dir })
}

/// Create the isolated state home. `MAOS_HOME` and `XDG_DATA_HOME` both point
/// here so every consumer — daemon, journal, transparency log, and `maosctl` —
/// resolves to the same fresh tree.
fn provision_home(keep: Option<&Path>) -> Result<PathBuf, String> {
    let home = match keep {
        Some(path) => {
            // A retained home is only isolated if it starts empty. Leftover
            // journal/transparency-log files would be read by this run, and on the
            // signed path `sealed-export --range 1d` would sign the PREVIOUS run's
            // rows alongside this capture — attribution by leftover file.
            if let Ok(mut entries) = std::fs::read_dir(path) {
                if entries.next().is_some() {
                    return Err(format!(
                        "demo-j1: --keep-home {} is not empty. A retained home must start \
                         empty or this run reads the previous run's journal and a signed \
                         export could cover its rows",
                        path.display()
                    ));
                }
            }
            path.to_path_buf()
        }
        None => {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default();
            std::env::temp_dir().join(format!("maos-demo-j1-{nanos}"))
        }
    };
    std::fs::create_dir_all(&home)
        .map_err(|e| format!("demo-j1: cannot create state home {}: {e}", home.display()))?;
    // Absolute so no consumer's cwd can reinterpret it.
    home.canonicalize()
        .map_err(|e| format!("demo-j1: cannot resolve state home {}: {e}", home.display()))
}

/// T5 — cross-check the signed window against an independent read of it.
///
/// `audit query` walks the Transparency Log; `sealed-export` signs the rows it
/// covers. If the signer's entry count falls short of what a plain reader can
/// see, the bundle is incomplete — exactly the failure the `--once` drain fix
/// exists to prevent, and a failure a row-count comparison can actually catch.
/// The key is generated INSIDE the disposable home, so this costs nothing and
/// leaves no operator key behind.
fn sealed_export_parity(bins: &Bins, home: &Path) -> Result<(usize, usize), String> {
    let maosctl = bins.dir.join("maosctl");
    let key = home.join("demo-parity.key");
    let bundle = home.join("demo-parity-bundle.json");

    run_checked(
        Command::new(&maosctl)
            .args(["audit", "keygen", "--output"])
            .arg(&key)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "audit keygen",
    )?;

    let queried = audit_query(bins, home)?;
    let rows = queried
        .lines()
        .filter(|l| l.trim_start().starts_with('{'))
        .count();

    let exported = run_checked(
        Command::new(&maosctl)
            .args(["audit", "sealed-export", "--range", "1d", "--audit-key"])
            .arg(&key)
            .arg("--output")
            .arg(&bundle)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "sealed-export",
    )?;
    Ok((rows, entry_count(&exported)))
}

/// One `maosctl audit query --range 1d --format ndjson` over the isolated home —
/// the independent reader both the sealed-export parity and the digest's
/// citations are judged against.
fn audit_query(bins: &Bins, home: &Path) -> Result<String, String> {
    run_checked(
        Command::new(bins.dir.join("maosctl"))
            .args(["audit", "query", "--range", "1d", "--format", "ndjson"])
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "audit query",
    )
}

/// Story 19-4 (D-19-4-09 A, D-19-4-05 A) — run the epic scene with its stdout
/// STREAMED (read line by line while stderr drains on its own thread). The
/// moment the child prints `dispatch_halted` naming pending halts, this runner
/// answers each over the operator door (`maosctl halt resolve … --kind
/// provided-context`) while the child holds the section. The door exists
/// because the home is `maos init`ed first (the J0 `init_world` shape).
fn run_scene(bins: &Bins, home: &Path) -> Result<SceneObservation, String> {
    let path_var = match std::env::var_os("PATH") {
        Some(existing) => {
            let mut parts = vec![bins.dir.clone()];
            parts.extend(std::env::split_paths(&existing));
            std::env::join_paths(parts).map_err(|e| format!("demo-j1: cannot extend PATH: {e}"))?
        }
        None => bins.dir.clone().into_os_string(),
    };
    run_checked(
        Command::new(&bins.maos)
            .arg("init")
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home)
            .env("PATH", &path_var),
        "demo-j1: the operator door (maos init)",
    )?;

    let started = Instant::now();
    let mut child = Command::new(&bins.maos)
        .args(["run", TOPOLOGY, "--epic", EPIC, "--once"])
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .env("PATH", &path_var)
        .env(HALT_HOLD_ENV, HALT_HOLD_SECS)
        // A fixture take must never be able to spawn a paid agent.
        .env_remove("MAOS_LIVE_AGENT")
        // Keep the fixture Worker's workspace inside the temporary home, where
        // `remove_worker_workspaces(home)` reaches it.
        .env_remove("MAOS_WORKER_WORKSPACES_ROOT")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("demo-j1: cannot run the daemon: {e}"))?;
    let (Some(stdout), Some(mut stderr)) = (child.stdout.take(), child.stderr.take()) else {
        return Err("demo-j1: the daemon's output was not captured".to_string());
    };
    let stderr_drain = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut text);
        text
    });
    let mut events: Vec<serde_json::Value> = Vec::new();
    for line in std::io::BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
    {
        // A non-JSON line parses to `null` and carries no event name.
        let event: serde_json::Value = serde_json::from_str(line.trim()).unwrap_or_default();
        let Some(name) = event["event"].as_str() else {
            continue;
        };
        // Streamed as it arrives: the judgement, halt and resume beats.
        if name.starts_with("dispatch_") || matches!(name, "morning_digest" | "drain") {
            let at = event.get("section").or(event.get("halt_ids"));
            println!(
                "  > {name} {}",
                at.map(ToString::to_string).unwrap_or_default()
            );
        }
        let halted = (name == "dispatch_halted").then(|| event["halt_ids"].clone());
        events.push(event);
        let pending = halted.iter().filter_map(|ids| ids.as_array()).flatten();
        for halt_id in pending.filter_map(serde_json::Value::as_str) {
            match resolve_halt(bins, home, halt_id) {
                Ok(_) => println!("  > resolved {halt_id} over the door (provided-context)"),
                Err(why) => println!("  > resolve FAILED for {halt_id}: {why}"),
            }
        }
    }
    let status = child
        .wait()
        .map_err(|e| format!("demo-j1: cannot wait for the daemon: {e}"))?;
    Ok(SceneObservation {
        events,
        stderr: stderr_drain.join().unwrap_or_default(),
        wall: started.elapsed(),
        exit_ok: status.success(),
    })
}

/// Answer one pending halt the way an operator does: `maosctl halt resolve` over
/// the door of the running root, with [`CLARIFICATION`] as the provided context.
fn resolve_halt(bins: &Bins, home: &Path, halt_id: &str) -> Result<String, String> {
    run_checked(
        Command::new(bins.dir.join("maosctl"))
            .args(["halt", "resolve", halt_id, "--spirit"])
            .arg(maos_bin::delegation::FROM_SPIRIT)
            .args(["--kind", "provided-context", "--text", CLARIFICATION])
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "maosctl halt resolve",
    )
}

/// The `### Story <n>` number a delegated goal opens with — the section it was
/// split from, since the goal is the section's text (plus any appended answer).
fn story_number(goal: &str) -> Option<usize> {
    let rest = goal.trim_start().strip_prefix("### Story")?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Story 19-4 (D-19-4-09 A) — `(arrived, held)`: how many `name` events the
/// scene printed and how many of them satisfy `held`. A per-section beat holds
/// only when both equal [`EPIC_SECTIONS`], so one section that held can never
/// stand in for two that did not.
fn per_section(
    obs: &SceneObservation,
    name: &str,
    held: impl Fn(&serde_json::Value) -> bool,
) -> (usize, usize) {
    let events = obs.events_named(name);
    (
        events.len(),
        events.into_iter().filter(|&e| held(e)).count(),
    )
}

fn every_section((arrived, held): (usize, usize)) -> bool {
    arrived == EPIC_SECTIONS && held == EPIC_SECTIONS
}

/// Turn the observation into beats. Every assertion is on a NAMED event field —
/// never on topology internals, which 1a rewrote and rung 2 will rewrite again.
/// P2–P6 are judged PER SECTION (Story 19-4 D-19-4-09 A); details render
/// `(arrived, held)` pairs against [`EPIC_SECTIONS`].
fn evaluate_beats(obs: &SceneObservation) -> Vec<Beat> {
    let mut beats = Vec::new();

    // P1 — the three class Spirits.
    let loaded: Vec<String> = obs
        .events_named("spirit_loaded")
        .iter()
        .filter_map(|e| e["spirit_id"].as_str().map(str::to_string))
        .collect();
    let expected = ["orchestrator", "architect", "reviewer"];
    let exact =
        loaded.len() == expected.len() && expected.iter().all(|id| loaded.iter().any(|l| l == id));
    beats.push(Beat::executed(
        "topology-spirits-loaded",
        "the founder's three class Spirits came up from the topology",
        state_of(exact),
        format!("loaded {loaded:?}"),
    ));

    // P2 — every section's delegation is frame-borne over loopback.
    let routed = per_section(obs, "delegation_routed", |e| {
        e["to_host"] == DELEGATION_HOST
            && e["recipient"] == DELEGATION_RECIPIENT
            && e["intent"]
                .as_str()
                .is_some_and(|intent| !intent.is_empty())
            && e["goal"]
                .as_str()
                .is_some_and(|goal| !goal.trim().is_empty())
    });
    let frame_borne = per_section(obs, "topology_worker_admit", |e| e["frame_borne"] == true);
    beats.push(Beat::executed(
        "delegation-frame-crosses-loopback",
        "every section's task.assign carried the consent envelope to developer-remote",
        state_of(every_section(routed) && every_section(frame_borne)),
        format!(
            "v0.8 rung — loopback rehearsal; frames to {DELEGATION_HOST} as \
             {DELEGATION_RECIPIENT} {routed:?}, frame-borne admits {frame_borne:?}"
        ),
    ));

    // P3 — a host-managed grant and a real, DISTINCT subprocess per section.
    let grants = per_section(obs, "host_grant_disposition", |_| true);
    let child_pids: std::collections::BTreeSet<u64> = obs
        .events_named("cli_wrapper_loaded")
        .iter()
        .filter_map(|e| e["child_pid"].as_u64().filter(|pid| *pid > 0))
        .collect();
    beats.push(Beat::executed(
        "worker-admitted-under-host-grant",
        "each Developer-Worker was admitted by a host-managed grant, not by trust",
        state_of(every_section(grants) && child_pids.len() == EPIC_SECTIONS),
        format!("grants {grants:?}, distinct real child_pids {child_pids:?}"),
    ));

    // P4 — the completion oracle, per section. j1-crosshost-2a AC1.6: the beat
    // asserts the oracle's verdict alone, never `last_stdout_tl_ref` (assigned on
    // every stdout row, independently of the oracle).
    let completed = per_section(obs, "worker_completion", |e| e["completed"] == true);
    beats.push(Beat::executed(
        "worker-completed-by-adapter-oracle",
        "completion came from the adapter's structured-output oracle, never from an exit code",
        state_of(every_section(completed)),
        format!("completed by the oracle {completed:?}"),
    ));

    // P5 — every delegation closed cleanly.
    let closed = per_section(obs, "delegation_completed", |e| {
        e["orchestrator_safe_point"] == true
    });
    beats.push(Beat::executed(
        "delegation-closed-at-safe-point",
        "TaskComplete was journaled for each section and no frame was left in flight",
        state_of(every_section(closed)),
        format!("closed at a safe point {closed:?}"),
    ));

    // P5b — every wrapped CLI reported a non-crash exit and every class Spirit
    // went idle; without them the scene could narrate a clean loop over a
    // subprocess that never exited.
    let exited = per_section(obs, "cli_wrapper_exit", |e| e["is_crash"] == false);
    let idle_fired = obs.events_named("on_idle_fired").len();
    beats.push(Beat::executed(
        "worker-exited-and-loop-went-idle",
        "every wrapped CLI reported a non-crash exit and every class Spirit fired on_idle",
        state_of(every_section(exited) && idle_fired == EXPECTED_IDLE_FIRES),
        format!("non-crash exits {exited:?}, on_idle_fired {idle_fired}/{EXPECTED_IDLE_FIRES}"),
    ));

    // P6a — the isolated home stayed clean.
    let journal_warnings = obs
        .stderr
        .lines()
        .filter(|l| l.trim_start().starts_with("journal: WARNING"))
        .count();
    beats.push(Beat::executed(
        "state-home-clean",
        "a fresh state home means a journal warning would be this run's own finding",
        state_of(journal_warnings == 0),
        format!("{journal_warnings} journal warning(s) in an isolated home"),
    ));

    // P6a-order — the claim table is titled "execution order", so verify it:
    // each section's stages EXACTLY once and in sequence, sections in order, the
    // drain once and last. A duplicate, a completion before its own delegation or
    // overlapping sections could otherwise satisfy every beat independently.
    const SECTION_STAGES: [&str; 4] = [
        "delegation_routed",
        "cli_wrapper_loaded",
        "worker_completion",
        "delegation_completed",
    ];
    let observed: Vec<&str> = obs
        .events
        .iter()
        .filter_map(|e| e["event"].as_str())
        .filter(|name| SECTION_STAGES.contains(name) || *name == "drain")
        .collect();
    let mut ordered = SECTION_STAGES.repeat(EPIC_SECTIONS);
    ordered.push("drain");
    beats.push(Beat::executed(
        "lifecycle-stages-in-order",
        "per section: one delegation, one worker run, one completion — in order, drain last",
        state_of(observed == ordered),
        format!(
            "{}expected {} stages (per section then drain), observed {observed:?}",
            if observed == ordered {
                ""
            } else {
                "stream order NOT proven: "
            },
            ordered.len()
        ),
    ));

    // P6b — the drain. This is the beat the 2026-08-14 rehearsal caught: a
    // timeout means queued capability rows can be lost, so a sealed-export over
    // this window would sign an incomplete bundle. The writer-failure diagnostic
    // is NON-FATAL (the process still exits 0), so it must not read green either.
    let drain_timed_out = obs.stderr.contains("audit writer topology drain timed out");
    let writer_failed = obs.stderr.contains("audit writer task failed");
    let drained = obs.event("drain").is_some();
    beats.push(Beat::executed(
        "audit-drain-clean",
        "every queued audit row reached SQLite before the process exited",
        state_of(drained && !drain_timed_out && !writer_failed),
        if drain_timed_out {
            format!(
                "DRAIN TIMED OUT — queued rows may be lost, so a sealed-export over this \
                 window could sign an incomplete bundle (wall {:.3}s)",
                obs.wall.as_secs_f64()
            )
        } else if writer_failed {
            "AUDIT WRITER TASK FAILED during drain — the daemon reports this and still \
             exits 0, so queued rows are NOT proven persisted"
                .to_string()
        } else {
            format!(
                "drain observed, no timeout, no writer failure, wall {:.3}s",
                obs.wall.as_secs_f64()
            )
        },
    ));

    // Story 19-4 AC1.2 — one frame per section, opened in section order, closed.
    let stories: Vec<Option<usize>> = obs
        .events_named("delegation_routed")
        .iter()
        .map(|e| e["goal"].as_str().and_then(story_number))
        .collect();
    let in_order = stories.iter().copied().eq((1..=EPIC_SECTIONS).map(Some));
    beats.push(Beat::executed(
        "epic-decomposed-n-frames",
        "the Orchestrator split the epic into one frame per `### Story` section, in order",
        state_of(in_order && closed.0 == EPIC_SECTIONS),
        format!(
            "{EPIC}: frames open stories {stories:?}, {} closed",
            closed.0
        ),
    ));

    // Story 19-4 AC2 — the ambiguous section was judged and the KERNEL halted it
    // within two frames (one under D-19-4-02 A: the judgement precedes its frame).
    let judged = obs
        .events_named("dispatch_judged")
        .into_iter()
        .find(|e| e["section"].as_u64() == Some(AMBIGUOUS_SECTION));
    let halt_id = judged.and_then(|e| e["halt_id"].as_str());
    let halt_frame = judged.and_then(|e| e["halt_frame_id"].as_str());
    let names_halt = |e: &serde_json::Value, kind: &str| {
        e["event"] == kind
            && e["halt_ids"]
                .as_array()
                .is_some_and(|ids| halt_id.is_some() && ids.iter().any(|x| x.as_str() == halt_id))
    };
    let halted_at = obs
        .events
        .iter()
        .position(|e| names_halt(e, "dispatch_halted"));
    let frames_before = halted_at.map(|at| {
        obs.events[..at]
            .iter()
            .filter(|e| e["event"] == "delegation_routed")
            .count()
    });
    let why = match (judged, halt_id) {
        (None, _) => "no dispatch_judged (an unset MAOS_INFERENCE_MODE judges nothing)",
        (Some(_), None) => "the kernel raised no halt",
        (Some(_), Some(_)) => "the kernel halted it",
    };
    beats.push(Beat::executed(
        "ambiguity-halt-within-2-frames",
        "the Orchestrator judged the ambiguous section and the kernel halted it within two frames",
        state_of(
            judged.is_some_and(|e| e["tag"] == AMBIGUITY_TAG)
                && frames_before.is_some_and(|frames| frames <= 2),
        ),
        format!(
            "section {AMBIGUOUS_SECTION} judged {:?}: {why}; halt {halt_id:?}, frames before \
             dispatch_halted {frames_before:?}",
            judged.map(|e| (&e["tag"], &e["value"]))
        ),
    ));

    // Story 19-4 AC5 — answered over the door, resumed WITH the answer, and the
    // post-resume digest cites the exact pre-halt frame as RESOLVED, never open.
    let resumed_at = obs.events.iter().position(|e| {
        names_halt(e, "dispatch_resumed")
            && e["section"].as_u64() == Some(AMBIGUOUS_SECTION)
            && e["provided_context"] == true
    });
    let clarified = obs.events_named("delegation_routed").iter().any(|e| {
        e["goal"].as_str().is_some_and(|goal| {
            story_number(goal) == usize::try_from(AMBIGUOUS_SECTION).ok()
                && goal.contains(CLARIFICATION)
        })
    });
    let digest_at = obs
        .events
        .iter()
        .rposition(|e| e["event"] == "morning_digest");
    let digest = digest_at.map(|at| &obs.events[at]["digest"]);
    // The digest's `field` entries citing the halt's own frame.
    let listing = |field: &'static str| {
        let halts = digest
            .and_then(|d| d[field].as_array())
            .into_iter()
            .flatten();
        halts.filter(move |h| halt_frame.is_some() && h["source_log_ref"].as_str() == halt_frame)
    };
    let resolved = listing("resolved_halts")
        .any(|h| h["halt_id"].as_str() == halt_id && h["resolution"] == "provided_context");
    let still_open = listing("open_halts").next().is_some();
    let post_resume = matches!((resumed_at, digest_at), (Some(r), Some(d)) if d > r);
    beats.push(Beat::executed(
        "halt-resume-referential-identity",
        "the post-resume digest cites the exact pre-halt typed ref",
        state_of(post_resume && clarified && resolved && !still_open),
        format!(
            "halt frame {halt_frame:?}: resumed {}, answer in section {AMBIGUOUS_SECTION}'s \
             goal {clarified}, digest after resume {post_resume}, cited as resolved {resolved}, \
             still listed open {still_open}",
            resumed_at.is_some()
        ),
    ));

    beats
}

/// Story 19-4 AC3 — the digest beat's name.
const DIGEST_BEAT: &str = "digest-cites-source-refs";

/// Story 19-4 AC3.3 (D-19-4-06 A) — join the run's ONE `morning_digest` to an
/// independent `audit query` over the same window (`audit_ndjson`, one row per
/// line). Every `completed[].source_log_ref` must name a `task.complete` or
/// `cli.subprocess.output` row, every `open_halts[]`/`resolved_halts[]` ref an
/// `epistemic.halt` row, and the digest must cite at least one completion per
/// section. Pure over its inputs so a vector can plant a hallucinated ref.
fn digest_cites_source_refs(obs: &SceneObservation, audit_ndjson: &str) -> Beat {
    let rows: Vec<serde_json::Value> = audit_ndjson
        .lines()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .collect();
    let digests = obs.events_named("morning_digest");
    let digest = match digests.as_slice() {
        [one] => one["digest"].as_object(),
        _ => None,
    };
    // `(listed, uncited)`: how many refs the digest's `field` lists, and those
    // that name no audit row of one of `kinds`.
    let check = |field: &str, kinds: &[&str]| {
        let listed = digest.and_then(|d| d.get(field)).and_then(|v| v.as_array());
        let refs: Vec<&str> = listed.map_or_else(Vec::new, |items| {
            items
                .iter()
                .map(|item| item["source_log_ref"].as_str().unwrap_or_default())
                .collect()
        });
        let uncited: Vec<String> = refs
            .iter()
            .filter(|r| {
                !rows.iter().any(|row| {
                    row["frame_id"] == **r
                        && row["kind"].as_str().is_some_and(|k| kinds.contains(&k))
                })
            })
            .map(|r| short_ref(r))
            .collect();
        (refs.len(), uncited)
    };
    let (completed, mut uncited) = check("completed", &["task.complete", "cli.subprocess.output"]);
    for field in ["open_halts", "resolved_halts"] {
        uncited.extend(check(field, &["epistemic.halt"]).1);
    }
    Beat::executed(
        DIGEST_BEAT,
        "every completion and halt the FR17 digest cites resolves to a Transparency-Log row",
        state_of(digests.len() == 1 && completed >= EPIC_SECTIONS && uncited.is_empty()),
        format!(
            "{} morning_digest(s), {completed} completion ref(s) (>= {EPIC_SECTIONS}); refs that \
             name no matching audit row of their kind among {} row(s): {uncited:?}",
            digests.len(),
            rows.len()
        ),
    )
}

/// What each published gate leg MEANS, for the claim table. Hard-matched on the
/// names `ledger_leg_names()` publishes; a rename falls through to the catch-all,
/// which is deliberately honest rather than silent.
fn leg_narration(leg: &str) -> &'static str {
    match leg {
        "frame-borne-route-intact" => "the route is frame-borne, not local",
        "loopback-from-host-unverified" => "the wire-identity boundary is where rung 1 says",
        "completion-oracle-per-adapter" => "each adapter reads its OWN structured output",
        "worker-cli-under-library" => "the adapter seam stays nameable by its vectors",
        "completion-vectors-enrolled" => "every J1 test target is actually invoked by CI",
        "consent-refusal-proofs" => "-32001 / -32009 / -32003 stay distinct and asserted",
        "cross-host-identity-proof" => {
            "the crossing is proven in two logs under a verified wire identity"
        }
        _ => "a gate leg this narration has no description for",
    }
}

/// Invoke the hermetic gate as the judge of the wire and narrate it **one beat per
/// leg**. It reads committed sources, so it is cheap and needs no substrate.
///
/// j1-crosshost-1b AC2.10 — until this story the whole gate collapsed into a single
/// boolean emitted under the name `frame-borne-route-intact`, so after `2a` grew the
/// gate to five legs a red *completion-oracle* or *enrollment* leg printed
/// `FAIL frame-borne-route-intact`: the narrated artifact named the wrong failure.
/// With `1b`'s consent leg that would have been six legs behind one name.
///
/// The trailing `disallowed-intent-refused-blocking` beat is AC2.11's, flipped out
/// of `unlanded_beats()` in code. It is a CONJUNCTION on purpose: the refusal is
/// PROVEN-BLOCKING only when the assertions exist (`consent-refusal-proofs`, a
/// static source oracle) AND CI executes them (`completion-vectors-enrolled`). This
/// gate has never observed a frame being refused and cannot, so claiming the first
/// half alone would print something the artifact cannot back.
fn run_delegation_gate() -> Vec<Beat> {
    use crate::check_j1_loopback_delegation as gate;
    let judged = gate::judge(Path::new("."));
    // A leg with no audit is UNKNOWN, never green — `leg_green` returns `None` and
    // this maps it to a non-proven state, not to a silent pass.
    let green = |leg: &str| judged.leg_green(leg).unwrap_or(false);
    let mut beats: Vec<Beat> = gate::ledger_leg_names()
        .into_iter()
        .map(|leg| {
            let detail = judged
                .leg_detail(leg)
                .unwrap_or_else(|| format!("{DELEGATION_GATE} published no audit for `{leg}`"));
            Beat::executed(leg, leg_narration(leg), state_of(green(leg)), detail)
        })
        .collect();

    let written = green("consent-refusal-proofs");
    let run = green("completion-vectors-enrolled");
    beats.push(Beat::executed(
        "disallowed-intent-refused-blocking",
        "a disallowed intent must be REFUSED (-32001 CODE_INTENT_DENIED, distinct from -32009)",
        state_of(written && run),
        format!(
            "refusal assertions asserted = {written}, enrolled in CI = {run} \
             (crates/maos-bin/tests/consent_refusal_1b.rs, judged by {DELEGATION_GATE})"
        ),
    ));
    // This derives only from the seventh leg: Leg 2 permanently describes the
    // loopback rehearsal arm and must never become this crossing's proxy.
    beats.push(Beat::executed(
        "two-host-delegation",
        "two real hosts over mTLS/TOFU, a frame crossed, a worker ran on the far side, both logs carry the same sixteen bytes",
        state_of(green("cross-host-identity-proof")),
        "judged by cross-host-identity-proof".to_string(),
    ));
    beats
}

/// The explicit `--skip-gate` counterpart for the gate-derived delegation beat.
fn absent_two_host_delegation() -> Beat {
    Beat::absent(
        "two-host-delegation",
        "two real hosts over mTLS/TOFU, a frame crossed, a worker ran on the far side, both logs carry the same sixteen bytes",
        "j1-crosshost-2b",
    )
}

/// The beats no story has delivered yet. Declared so they are visible, owned so
/// nobody has to guess who closes them.
///
/// `disallowed-intent-refused-blocking` LEFT this list in `j1-crosshost-1b`: the
/// refusal proofs landed, so leaving it here would have made the narrated artifact
/// state that this work was never done. `halt-resume-referential-identity` LEFT it
/// in Story 19-4 (AC5): it is an executed beat of the epic scene now.
fn unlanded_beats() -> Vec<Beat> {
    vec![
        Beat::absent(
            TIER2_BEAT,
            "one real paid agent run (codex OR claude), captured and sealed under a named human signer",
            "--live-codex (operator-local, never CI; the adapter comes from the topology, not the flag name)",
        ),
        Beat::absent(
            "two-host-signed-run",
            "two real hosts over mTLS/TOFU, heterogeneous worker, one reconciled signed bundle",
            "j1-crosshost-2d-paid-two-host-run",
        ),
    ]
}

/// `j1-crosshost-2c` AC5.3 — flip `two-host-signed-run` by an **EXECUTED LEG**.
///
/// The published-ledger route is structurally dead twice: `apply_published_ledgers`
/// filters `l.gate == DELEGATION_GATE` and that gate writes no ledger file, and
/// `ledger_gates()` is the four Postgres substrate gates. So this mirrors the
/// in-process Tier-2 flip: run the judge, read what it observed.
///
/// The owner string was re-pointed to `j1-crosshost-2d-paid-two-host-run` by
/// RF-0 (§A6 round-table, 2026-08-18): `2c` owns the judge, `2d` owns the run.
/// `unlanded_beats` above carries it; leg 9 of the judge enforces it.
/// Three outcomes, and **none of them claims a signed run** — that is the R1
/// re-scope, ratified at the 2026-08-21 round-table and implemented by
/// `j1-crosshost-2e` (F2/F3):
///   * no capture → the beat stays ABSENT. `Beat::absent` sets `executed: false`,
///     so an unlanded beat can never fail a run — which is the honest model for a
///     claim whose substrate is an operator, two hosts and a funded key.
///   * capture present, judge RED → `INDETERMINATE`, and the claim is refused.
///   * capture present, judge GREEN → `INDETERMINATE` still. The gate has no way
///     to verify the operator's evidence and does not pretend to: `PROVEN_LIVE_SIGNED`
///     was structurally unreachable here (F2 — the `MAOS-EVIDENCE-V1` nonce is
///     recomputed at gate-run time, so no pre-written transcript can carry it, and
///     nothing in the workspace produced the file). The evidence of a two-host run
///     is the TWO BUNDLE SIGNATURES verified by the third-party `verify.py` plus a
///     `reconcile-hosts` that executes — all operator-performed, none of it
///     checkable from inside a hermetic gate.
///
/// **`executed` stays FALSE on both present-capture branches, and that is the F3
/// fix.** `Beat::failed()` is `executed && !state.is_proven()`, and `is_proven()`
/// admits only `ProvenBlocking | ProvenLiveSigned`, so marking an `Indeterminate`
/// beat `executed` made `demo-j1` exit NONZERO and render `FAIL` the moment a
/// capture landed — contradicting this very doc comment, which has always said
/// `INDETERMINATE` "is not a failure". The beat now renders as a visible `--`
/// non-claim carrying its owner, which is what an un-earned claim should look like.
///
/// ⚠ Do NOT "fix" this by widening `is_proven()`: it is consumed for
/// `product_claim` and artifact refs in `evidence_ledger.rs`, so widening it would
/// make OTHER gates over-claim in order to correct a demo exit code.
fn apply_two_host_signed_run(beats: &mut [Beat]) {
    let judgement = crate::check_j1_two_host_signed_run::judge(Path::new("."));
    apply_capture_judgement(beats, &judgement);
}

/// §A6 review P9 (AC3.5) — the PURE state-application seam. It takes the judge's
/// verdict instead of a root so a vector can INJECT a present-unsigned capture:
/// at HEAD the capture is absent, so a test that only calls `apply` against `.`
/// cannot fail — "a vector that cannot fail is the thing this lane keeps
/// finding" (Trap 17). Every rule of the doc comment above is enforced here and
/// pinned by `demo_j1_tests::two_host_beat_with_present_capture_renders_*`.
fn apply_capture_judgement(
    beats: &mut [Beat],
    judgement: &crate::check_j1_two_host_signed_run::Judgement,
) {
    if !judgement.capture_present {
        return;
    }
    let detail = if judgement.findings.is_empty() {
        format!(
            "capture present and validated; the run is NOT claimed — {} (the two bundle \
             signatures and `reconcile-hosts` are operator-verified, not gate-verified)",
            crate::check_j1_two_host_signed_run::CLAIM_SCOPE
        )
    } else {
        format!(
            "capture present but the judge found {} finding(s) — the claim is refused",
            judgement.findings.len()
        )
    };
    if let Some(beat) = beats.iter_mut().find(|b| b.name == "two-host-signed-run") {
        beat.state = EvidenceState::Indeterminate;
        beat.detail = detail;
        // `executed` deliberately left FALSE and the owner deliberately retained:
        // this beat is not a thing that ran and failed, it is a thing nobody has
        // earned yet. See the doc comment above (F3).
    }
}
/// §A6 review P9 (AC3.5) — the ONE place a claim-table row is rendered, extracted
/// so the tests assert the ACTUAL rendered line instead of the static declaration
/// (previously `demo_j1_tests` read `unlanded_beats()` while claiming to test the
/// render). The mark vocabulary is the demo's honesty contract: `--` is a visible
/// non-claim, `ok` is proven, `FAIL` is an executed beat that did not hold.
fn render_beat_row(beat: &Beat) -> String {
    let mark = if !beat.executed {
        "--  "
    } else if beat.state.is_proven() {
        "ok  "
    } else {
        "FAIL"
    };
    format!(
        "  {mark} {:<38} {:<19} {}",
        beat.name,
        beat.state.as_str(),
        beat.detail
    )
}

/// A published ledger outranks anything this runner narrates. Read them through
/// the validating loader — never a raw `serde_json::Value` peek, which is how a
/// stale or unbound report sneaks into a claim table.
fn apply_published_ledgers(beats: &mut [Beat]) {
    let ledgers = match crate::evidence_ledger::load_published_ledgers(Path::new(REPORTS_DIR)) {
        Ok(ledgers) => ledgers,
        Err(problems) => {
            section("Published ledgers");

            println!(
                "  {} report(s) did not validate; every beat keeps its observed state:",
                problems.len()
            );
            for problem in problems.iter().take(4) {
                println!("    - {problem}");
            }
            return;
        }
    };
    let j1: Vec<_> = ledgers
        .iter()
        .filter(|l| l.gate == DELEGATION_GATE)
        .collect();
    if j1.is_empty() {
        return;
    }
    section("Published ledgers");
    for ledger in j1 {
        println!(
            "  {:<38} {} @ {}",
            ledger.gate,
            ledger.product_claim,
            ledger.commit.chars().take(8).collect::<String>()
        );
        for leg in &ledger.legs {
            if let Some(beat) = beats.iter_mut().find(|b| b.name == leg.name) {
                let published = match leg.evidence_state.as_str() {
                    "PROVEN_BLOCKING" => EvidenceState::ProvenBlocking,
                    "PROVEN_LIVE_SIGNED" => EvidenceState::ProvenLiveSigned,
                    "ABSENT" => EvidenceState::Absent,
                    _ => EvidenceState::Indeterminate,
                };
                // A ledger may LIGHT a beat this run never executed. It must never
                // PROMOTE one this run executed and watched fail: outside GitHub
                // Actions `load_published_ledgers` has no build binding to compare,
                // so a stale green report left in `tests/reports/` by an older
                // checkout would otherwise bury a live failure and exit 0.
                if beat.executed && !beat.state.is_proven() && published.is_proven() {
                    println!(
                        "    {:<36} ledger claims {} — IGNORED, this run observed {}",
                        beat.name,
                        leg.evidence_state,
                        beat.state.as_str()
                    );
                    continue;
                }
                beat.state = published;
                beat.detail = format!("from published ledger: {}", leg.evidence_state);
                beat.executed = beat.state != EvidenceState::Absent;
                beat.owner = None;
            }
        }
    }
}

/// The worker adapter + manifest a topology will ACTUALLY drive, resolved by
/// reading the topology the operator supplied.
///
/// j1-crosshost-2a AC2.6/AC3.5 — the Tier-2 leg used to hardcode codex's clean-home
/// path and codex's credential variable, and it wrote a literal
/// `"maos run <codex topology> …"` into the SIGNED capture's `command_metadata`.
/// Once the identity allowlist widened, that literal would have made a claude run
/// sign a bundle asserting it was codex — the same defect class this story exists
/// to fix, one layer deeper. Everything provider-specific is now ASKED OF THE
/// ADAPTER, and the manifest it came from is carried into the capture.
struct TopologyWorker {
    cli: Box<dyn maos_bin::worker_cli::WorkerCli>,
    manifest: PathBuf,
    argv_prefix: Vec<String>,
}

fn resolve_topology_worker(topology: &Path) -> Result<TopologyWorker, String> {
    let text = std::fs::read_to_string(topology)
        .map_err(|e| format!("cannot read the topology {}: {e}", topology.display()))?;
    let root: toml::Value = toml::from_str(&text)
        .map_err(|e| format!("topology {} is not valid TOML: {e}", topology.display()))?;
    let base = topology.parent().unwrap_or(Path::new("."));
    let spirits = root
        .get("topology")
        .and_then(|t| t.get("spirits"))
        .and_then(|s| s.as_array())
        .ok_or_else(|| {
            format!(
                "topology {} declares no [[topology.spirits]]",
                topology.display()
            )
        })?;
    // Review 2a-P5 — production runs EVERY `[cli_wrapper]` member, but this
    // preflight and the post-run scan attest exactly ONE worker. Rather than
    // seal a capture that describes worker #1 while workers #2..N run with
    // their credential variable unscanned and their isolation unasserted, a
    // multi-worker topology is REFUSED outright: the signing machinery attests
    // one worker by construction.
    let mut found: Vec<TopologyWorker> = Vec::new();
    for entry in spirits {
        let Some(rel) = entry
            .get("manifest")
            .or_else(|| entry.get("path"))
            .and_then(|m| m.as_str())
        else {
            continue;
        };
        let p = {
            let raw = PathBuf::from(rel);
            if raw.is_absolute() {
                raw
            } else {
                base.join(raw)
            }
        };
        let Ok(child) = std::fs::read_to_string(&p) else {
            continue;
        };
        let Ok(child_root) = toml::from_str::<toml::Value>(&child) else {
            continue;
        };
        let Some(cw) = child_root.get("cli_wrapper") else {
            continue;
        };
        let command = cw
            .get("command")
            .and_then(|c| c.as_str())
            .ok_or_else(|| format!("{} has no [cli_wrapper] command", p.display()))?;
        let cli = maos_bin::worker_cli::select_worker_cli(command).ok_or_else(|| {
            format!(
                "{} names worker CLI '{command}', which no adapter supports (supported: {:?})",
                p.display(),
                maos_bin::worker_cli::SUPPORTED_WORKER_CLIS
            )
        })?;
        let argv_prefix = cw
            .get("argv_prefix")
            .and_then(|a| a.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        found.push(TopologyWorker {
            cli,
            manifest: p,
            argv_prefix,
        });
    }
    if found.len() > 1 {
        let names: Vec<&str> = found.iter().map(|w| w.cli.name()).collect();
        return Err(format!(
            "topology {} declares {} [cli_wrapper] members ({names:?}) — the Tier-2 \
             signing machinery verifies and attests exactly ONE worker, so a multi-worker \
             topology would be signed with worker #1's credential scan and isolation posture \
             while the others run unverified. Split the topology.",
            topology.display(),
            found.len()
        ));
    }
    found.into_iter().next().ok_or_else(|| {
        format!(
            "topology {} has no [cli_wrapper] member — there is no worker to sign for",
            topology.display()
        )
    })
}

/// Capture metadata names the actual adapter, manifest and daemon credential
/// channel without serializing either the raw key or the scoped bearer.
fn derive_command_metadata(
    manifest: &Path,
    topology: &Path,
    identity: &str,
    argv_prefix: &[String],
    secret_var: &str,
) -> String {
    format!(
        "maos run {} --live --once (topology {}); worker adapter `{identity}` with \
         argv_prefix {argv_prefix:?}; raw {secret_var} held by the daemon — \
         Worker receives a scoped bearer and proxy route (values redacted, scanned)",
        manifest.display(),
        topology.display()
    )
}

/// The Tier-2 leg: one real paid agent run, captured, journaled, sealed, verified.
///
/// Every precondition is checked BEFORE anything is spawned, and each failure is
/// a refusal rather than a downgrade — a Tier-2 beat that quietly becomes a
/// fixture take is exactly the overclaim the gate exists to stop.
fn live_codex_take(
    bins: &Bins,
    codex_topology: Option<&Path>,
    home: &Path,
) -> Result<usize, String> {
    // The live topology names the Worker manifest, and that manifest selects the
    // adapter; the repo ships `j1-founder-loop-codex.toml` and
    // `j1-founder-loop-claude.toml`, but the path is always supplied explicitly.
    let topology = codex_topology
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("MAOS_DEMO_J1_CODEX_TOPOLOGY").map(PathBuf::from))
        .ok_or_else(|| {
            "no live topology: pass --codex-topology <path> (or set \
             MAOS_DEMO_J1_CODEX_TOPOLOGY), e.g. spirits/topologies/j1-founder-loop-codex.toml \
             or spirits/topologies/j1-founder-loop-claude.toml"
                .to_string()
        })?;
    if !topology.exists() {
        return Err(format!(
            "codex topology {} does not exist",
            topology.display()
        ));
    }

    // Resolve WHICH adapter this topology drives before anything else — every
    // provider-specific precondition below is derived from it.
    let worker = resolve_topology_worker(&topology)?;
    // AC3.4 — refuse an unsignable adapter BEFORE the money is spent, not only
    // after. The post-run identity check below still stands: this one refuses the
    // declared intent, that one refuses what actually ran.
    if !SIGNABLE_WORKER_CLIS.contains(&worker.cli.name()) {
        return Err(format!(
            "topology worker {} names adapter `{}`, which may never earn {TIER2_BEAT} \
             (signable: {SIGNABLE_WORKER_CLIS:?}) — a fixture or unknown CLI must not reach the \
             signing path",
            worker.manifest.display(),
            worker.cli.name()
        ));
    }
    // Clean-home invariant, ASKED OF THE ADAPTER (AC2.6). Hardcoding
    // `~/.codex/auth.json` here made the demo's own preflight blind to claude for
    // exactly the reason the production path was. EMPTY is as unverifiable as
    // UNSET (review 2a-P7): an empty HOME makes the credential paths RELATIVE
    // while the child may apply its own fallback.
    let home_base = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            "HOME is unset or empty, so the clean-home invariant cannot be checked — \
             refusing. An unverifiable credential control is not a satisfied one."
                .to_string()
        })?;
    if let Err(p) = maos_bin::worker_cli::refuse_ambient_auth(worker.cli.as_ref(), &home_base) {
        return Err(format!(
            "{} exists — a signed run must use the metered API-key path. Remove it and retry \
             (runbook Phase 1.3). Note this is a FILENAME control: renaming the file satisfies \
             it without removing the credential.",
            p.display()
        ));
    }
    // AC1.3 — the adapter's oracle depends on argv flags. `maos run` refuses a
    // manifest that omits them, but that refusal would land AFTER this leg has
    // already announced it is about to spend money; refuse here too.
    maos_bin::worker_cli::refuse_missing_argv_flags(worker.cli.as_ref(), &worker.argv_prefix)
        .map_err(|e| format!("{} is not signable: {e}", worker.manifest.display()))?;
    // Adapter restrictions supplement the verified T3 boundary; reject an argv
    // bypass before announcing a paid run.
    maos_bin::worker_cli::refuse_unsafe_argv(worker.cli.as_ref(), &worker.argv_prefix)
        .map_err(|e| format!("{} is not signable: {e}", worker.manifest.display()))?;
    worker
        .cli
        .refuse_missing_isolation(&worker.argv_prefix)
        .map_err(|e| {
            format!(
                "{} cannot earn an fs_jail claim: {e}",
                worker.manifest.display()
            )
        })?;
    // AC2.5(a) — the credential variable is the ADAPTER's, not codex's. Scanning
    // for `CODEX_API_KEY` on a claude run is a silent no-op.
    let secret_var = worker.cli.credential_env_var().ok_or_else(|| {
        format!(
            "adapter `{}` declares no credential variable, so a signed run cannot \
             prove it scanned for one",
            worker.cli.name()
        )
    })?;
    // AC2.5(b) — an UNEXECUTED scan must be structurally unable to emit
    // `"verified"`. `CaptureDoc::validate` accepts `redaction_result` on string
    // equality, so "the operator forgot to export the variable" and "the scan ran
    // and passed" used to produce BYTE-IDENTICAL signed evidence. Refuse the
    // signing rather than teaching the validator a second accepted string: a
    // `"not-scanned"` value would re-create the defect one identifier over.
    let secret = std::env::var(secret_var).unwrap_or_default();
    if secret.is_empty() {
        return Err(format!(
            "{secret_var} is unset or empty, so the redaction scan cannot execute — refusing to \
             sign a capture whose `redaction_result` would read \"verified\" without a scan \
             having run. Set {secret_var} to the metered key the `{}` adapter reads.",
            worker.cli.name()
        ));
    }
    let grants = std::env::var_os("MAOS_HOST_GRANTS")
        .map(PathBuf::from)
        .ok_or_else(|| "MAOS_HOST_GRANTS unset — without it a real CLI fails closed".to_string())?;
    if !grants.exists() {
        return Err(format!(
            "MAOS_HOST_GRANTS points at {} which does not exist",
            grants.display()
        ));
    }
    let signer = std::env::var("MAOS_DEMO_J1_SIGNER").map_err(|_| {
        "MAOS_DEMO_J1_SIGNER unset — a Tier-2 capture requires a NAMED human signer".to_string()
    })?;
    let key = std::env::var_os("MAOS_DEMO_J1_SIGNER_KEY")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "MAOS_DEMO_J1_SIGNER_KEY unset — point it at the Ed25519 audit key from \
             `maosctl audit keygen`"
                .to_string()
        })?;
    let maosctl = bins.dir.join("maosctl");
    if !maosctl.exists() {
        return Err(format!("maosctl not found at {}", maosctl.display()));
    }
    println!("  preflight        clean home, metered key, host grants, named signer, audit key");

    // Story 19-3 (AC1.2) — no demo dir and no daemon `current_dir`: every Worker
    // runs under T3 in its own per-run workspace (`/workspace`), which the daemon
    // creates, diffs and reports.
    //
    // `--once` is REQUIRED. Without it `maos run` falls through to the continuous
    // serving loop (`maos-bin/src/main.rs:5232`) while `.output()` waits for the
    // child to exit — so a SUCCESSFUL paid take would hang forever and never
    // reach capture, sealing, or verification.
    let out = Command::new(&bins.maos)
        .arg("run")
        .arg(topology.canonicalize().unwrap_or(topology.clone()))
        .arg("--live")
        .arg("--once")
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        // Keep the workspace inside the ephemeral home, where cleanup and the
        // home-relative report below expect it.
        .env_remove("MAOS_WORKER_WORKSPACES_ROOT")
        .env("MAOS_LIVE_AGENT", "1")
        .output()
        .map_err(|e| format!("cannot run the live daemon: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Refuse, never downgrade. `worker_completion` is emitted mid-run, so the
    // event alone does not mean the daemon finished cleanly.
    if !out.status.success() {
        return Err(format!(
            "the live daemon exited nonzero ({:?}) — nothing to sign: {}",
            out.status.code(),
            stderr.trim()
        ));
    }
    if stderr.contains("audit writer task failed")
        || stderr.contains("audit writer topology drain timed out")
    {
        return Err(
            "the audit writer did not drain cleanly, so a sealed export over this window \
             could sign an INCOMPLETE bundle — refusing to sign"
                .to_string(),
        );
    }

    let completion = stdout
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l.trim()).ok())
        .find(|v: &serde_json::Value| v["event"].as_str() == Some("worker_completion"));
    let identity = completion
        .as_ref()
        .and_then(|e| e["worker_cli"].as_str())
        .unwrap_or("unknown")
        .to_string();
    if !completion
        .as_ref()
        .and_then(|e| e["completed"].as_bool())
        .unwrap_or(false)
    {
        return Err("the live worker did not complete — nothing to sign".to_string());
    }
    // The topology is OPERATOR-authored, so nothing else stops the hermetic
    // fixture from reaching the signing path and earning a Tier-2 label. This
    // check WIDENED to an allowlist (AC3.4); it must never be deleted.
    if !SIGNABLE_WORKER_CLIS.contains(&identity.as_str()) {
        return Err(format!(
            "the live worker identity is `{identity}`, which is not signable \
             (signable: {SIGNABLE_WORKER_CLIS:?}) — a fixture or unknown CLI must never earn \
             {TIER2_BEAT}"
        ));
    }
    // ...and it must be the adapter the topology declared. A mismatch means the
    // capture would attest a different provider than the preflight verified —
    // including the credential variable that was scanned for.
    if identity != worker.cli.name() {
        return Err(format!(
            "the live worker identity is `{identity}` but the topology declared `{}` — \
             refusing to sign a capture whose provider claim does not match the run",
            worker.cli.name()
        ));
    }
    let tl_ref = completion
        .as_ref()
        .and_then(|e| e["last_stdout_tl_ref"].as_str())
        .unwrap_or_default()
        .to_string();
    println!(
        "  live run         {identity} completed, worker TL ref {}",
        short_ref(&tl_ref)
    );
    // Story 19-3 (AC1.2) — where the Worker's effect lives; the diff that made
    // this run `completed` is over this directory. AC5 forbids absolute
    // workstation paths in scene output: name it relative to the state home.
    let workspace = completion
        .as_ref()
        .and_then(|e| e["workspace"].as_str())
        .map(Path::new)
        .and_then(|path| path.strip_prefix(home).ok())
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("  worker workspace {workspace}");

    // AC3 records `redaction_result: "verified"`. VERIFY it rather than assert
    // it: `record-capture` only checks that the field reads "verified", so an
    // unchecked literal would place an unearned control claim inside a SIGNED
    // bundle. Compare against the real secret and never print it.
    //
    // `secret` is guaranteed non-empty by the preflight (AC2.5b) — the
    // `if !secret.is_empty()` guard that used to sit here is GONE, because it made
    // "the operator forgot to export the variable" and "the scan ran and passed"
    // emit byte-identical signed evidence. The scan now always executes.
    if stdout.contains(&secret) || stderr.contains(&secret) {
        return Err(format!(
            "the live run echoed the {secret_var} value into its own output — refusing to \
             sign a capture that claims redaction was verified"
        ));
    }

    // The capture carries NON-SECRET fields only, spelled exactly as
    // `record-capture` validates them: it refuses a credential-shaped value and
    // refuses stale control claims or unverified redaction, so the gate stays open
    // rather than signing a lie.
    //
    // AC3.5 — `command_metadata` is DERIVED from the adapter and manifest actually
    // used. It used to be the literal `"maos run <codex topology> --live;
    // CODEX_API_KEY injected host-side (value redacted)"`, sealed into the signed
    // bundle: the moment the identity allowlist widened, a claude run would have
    // signed a bundle asserting it was codex.
    //
    // The daemon holds the raw provider credential. The Worker receives only
    // its scoped bearer and proxy route under a cleared inherited environment.
    let command_metadata = derive_command_metadata(
        &worker.manifest,
        &topology,
        &identity,
        &worker.argv_prefix,
        secret_var,
    );
    let capture = serde_json::json!({
        "signer": format!("{signer} (named human signer)"),
        "live_agent_identity": identity,
        "command_metadata": command_metadata,
        "host_grant_disposition":
            "host-managed grant admitted; a mismatch would have refused",
        "audit_refs": [tl_ref],
        "egress": EGRESS_POSTURE,
        "fs_jail": FS_JAIL_POSTURE,
        "redaction_result": "verified",
        "outcome":
            "worker completed via adapter oracle; no secret persisted; demo-j1 scene take",
    });
    let capture_path = home.join("j1-demo-capture.json");
    std::fs::write(
        &capture_path,
        serde_json::to_string_pretty(&capture).map_err(|e| format!("capture serialize: {e}"))?,
    )
    .map_err(|e| format!("cannot write the capture: {e}"))?;

    // Journal the capture as a `run.capture` row FIRST — sealed-export signs
    // audit ROWS, so an unjournaled capture would not be covered. Host-level
    // (no --spirit: v0.1 `resolve_spirit_name` accepts only `hello-spirit`).
    run_checked(
        Command::new(&maosctl)
            .args(["audit", "record-capture", "--capture"])
            .arg(&capture_path)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "record-capture",
    )?;

    // `--range`, never `--spirit`: the window must cover the worker rows AND the
    // fresh capture row.
    let bundle = home.join("j1-demo-bundle.json");
    let export_out = run_checked(
        Command::new(&maosctl)
            .args(["audit", "sealed-export", "--range", "1d", "--audit-key"])
            .arg(&key)
            .arg("--output")
            .arg(&bundle)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "sealed-export",
    )?;

    // The signed bytes are what leaves this machine, so scan THEM too — not only
    // the console stream. Unconditional (AC2.5b): the `if !secret.is_empty()` guard
    // that used to wrap this made the scan a silent no-op whenever the provider
    // variable was unset, while the capture still claimed `"verified"`.
    let signed = std::fs::read_to_string(&bundle)
        .map_err(|e| format!("cannot re-read the sealed bundle to check redaction: {e}"))?;
    if signed.contains(&secret) {
        // Review 2a-P6 — the bundle already exists on disk at this point, and
        // under `--keep-home` it would OUTLIVE this refusal: delete the signed
        // artifact itself, so "refusing to sign" does not leave a signed lie
        // behind for the operator to find later.
        let _ = std::fs::remove_file(&bundle);
        return Err(format!(
            "the sealed bundle contains the {secret_var} value — refusing to sign; the \
             dirty bundle has been deleted"
        ));
    }
    // `verify-bundle` REQUIRES `--pubkey` (`maos-cli/src/cli.rs`: `pubkey: String`
    // with no default), so omitting it exits at argument parsing AFTER the paid
    // run has already happened — the defect this leg shipped with. `sealed-export`
    // prints the pubkey it signed with; carry that hex into the verifier, and
    // refuse when it is absent instead of guessing.
    let pubkey = pubkey_hex(&export_out).ok_or_else(|| {
        format!(
            "sealed-export reported no pubkey, so the bundle cannot be verified: {}",
            export_out.trim()
        )
    })?;
    let verify = Command::new(&maosctl)
        .args(["audit", "verify-bundle"])
        .arg(&bundle)
        .arg("--pubkey")
        .arg(pubkey)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .output()
        .map_err(|e| format!("cannot run verify-bundle: {e}"))?;
    let verify_out = format!(
        "{}{}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );
    if !verify.status.success() {
        return Err(format!("verify-bundle failed: {}", verify_out.trim()));
    }
    println!("  signed bundle    verify OK — {}", verify_out.trim());
    Ok(entry_count(&verify_out))
}

/// `sealed-export` prints `… (<N> entries, pubkey <64-hex>)`. Pull that hex out:
/// `verify-bundle` requires it and has no default.
fn pubkey_hex(output: &str) -> Option<&str> {
    output
        .split_whitespace()
        .map(|token| token.trim_end_matches([')', ',', '.']))
        .find(|t| t.len() == 64 && t.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Pull the entry count out of `verify-bundle`'s "OK (<N> entries, seq <n>)".
fn entry_count(output: &str) -> usize {
    let tokens: Vec<&str> = output.split_whitespace().collect();
    tokens
        .windows(2)
        .find(|pair| pair[1].starts_with("entr"))
        .and_then(|pair| {
            pair[0]
                .trim_start_matches('(')
                .trim_end_matches(',')
                .parse::<usize>()
                .ok()
        })
        .unwrap_or(0)
}

fn run_checked(command: &mut Command, label: &str) -> Result<String, String> {
    let out = command
        .output()
        .map_err(|e| format!("cannot run {label}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{label} refused: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// A hermetic, reproducible observation that held is `PROVEN_BLOCKING`: CI can
/// re-run it from source alone, so it needs no signature. Anything else observed
/// is `INDETERMINATE` — never `ABSENT`, which is reserved for legs that never ran.
fn state_of(held: bool) -> EvidenceState {
    if held {
        EvidenceState::ProvenBlocking
    } else {
        EvidenceState::Indeterminate
    }
}

/// TL refs are long hex; a prefix identifies a row in narration without turning
/// the table into a wall.
fn short_ref(reference: &str) -> String {
    if reference.is_empty() {
        return "none".to_string();
    }
    format!("{}…", reference.chars().take(16).collect::<String>())
}

fn banner(title: &str) {
    println!("\n=== {title} ===");
}

fn section(title: &str) {
    println!("\n-- {title}");
}

// The test body lives in `src/tests/` — the kloc-excluded path (`xtask/kloc.toml:2`),
// the same `include!` shape as `rebaseline_check.rs:176-179`. `include!` keeps it
// textually inside this module, so the tests still reach private items.
#[cfg(test)]
mod tests {
    include!("tests/demo_j1_tests.rs");
}
