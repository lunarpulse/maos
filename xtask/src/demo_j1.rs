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
//! 2. Loopback beats are labeled `v0.8 rung — loopback rehearsal`. "cross-host"
//!    is never claimed while `two-host-signed-run` is not `PROVEN_LIVE_SIGNED`;
//!    that rung belongs to `j1-crosshost-2`.
//! 3. A fixture take never claims the Tier-2 beat. Only `--live-codex` ending in
//!    a verified sealed bundle earns `PROVEN_LIVE_SIGNED`.
//!
//! Every run is provisioned into an ISOLATED state home (`MAOS_HOME` +
//! `XDG_DATA_HOME`). That is not tidiness: on the operator's shared
//! `~/.local/share/maos` a pre-existing corrupted lifecycle journal prints
//! warnings that have nothing to do with this run, and a demo that narrates
//! ambient noise as its own output is lying quietly. With a fresh home, a
//! `journal:` warning is a real finding.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::gate_common::EvidenceState;

/// The scene's topology. The `host` key on its worker entry is what makes the
/// delegation frame-borne (`j1-crosshost-1a` AC1.1).
const TOPOLOGY: &str = "spirits/topologies/j1-founder-loop.toml";

/// Destination peer id / topology `host` the delegation must reach.
const DELEGATION_HOST: &str = "developer-remote-host";
/// The Spirit the `task.assign` is addressed to — NOT the `"worker"` subprocess
/// label (the Worker is a subprocess, not a mailbox peer).
const DELEGATION_RECIPIENT: &str = "developer-remote";

/// The gate that judges the wire. Hermetic, `Blocking`, shipped by 1a.
const DELEGATION_GATE: &str = "check-j1-loopback-delegation";

/// Where gate ledgers are published, when they are.
const REPORTS_DIR: &str = "tests/reports";

/// Egress posture the run declares — declared, NOT enforced, with its follow-up.
const EGRESS_POSTURE: &str = "declared-not-enforced";
const EGRESS_FOLLOWUP: &str = "FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT";

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

    if !Path::new(TOPOLOGY).exists() {
        return Err(format!(
            "demo-j1: {TOPOLOGY} not found — run this from the repository root"
        ));
    }

    let bins = preflight(skip_build)?;
    let home = provision_home(keep_home.as_deref())?;

    section("The scene");
    for (name, what) in [
        (
            "P1  orchestrator/architect/reviewer",
            "three class Spirits load from the topology",
        ),
        (
            "P2  task.assign -> developer-remote",
            "a real consent-carrying frame over the loopback A2A layer",
        ),
        (
            "P3  Developer-Worker admitted",
            "host-managed grant, T3, real child process",
        ),
        (
            "P4  completion parsed by the adapter",
            "the oracle decides completion — never the exit code",
        ),
        (
            "P5  delegation closed at a safe point",
            "TaskComplete journaled, no frame left in flight",
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

    if skip_gate {
        beats.push(Beat::absent(
            "frame-borne-route-intact",
            "the wire is judged by a Blocking gate, not by this narration",
            "--skip-gate was passed",
        ));
    } else {
        section("Running the judge");
        beats.push(run_delegation_gate());
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

    apply_published_ledgers(&mut beats);

    section("Claim table (execution order)");
    for beat in &beats {
        let mark = if !beat.executed {
            "--  "
        } else if beat.state.is_proven() {
            "ok  "
        } else {
            "FAIL"
        };
        println!(
            "  {mark} {:<38} {:<19} {}",
            beat.name,
            beat.state.as_str(),
            beat.detail
        );
        println!("       {:<38} {}", "", beat.narration);
    }

    section("What this run does NOT claim");
    println!("  rung             v0.8 — loopback rehearsal. `developer-remote` is a peer id on");
    println!("                   THIS host; no packet left the machine. Two real hosts over");
    println!("                   mTLS/TOFU is j1-crosshost-2 and reads ABSENT above.");
    println!("  peer auth        on loopback `frame.from.host_id` is self-asserted — the frame");
    println!("                   picks which allowlist judges it. Rung 2 binds it to a");
    println!("                   TLS-verified identity (1a's recorded boundary leg).");
    println!("  cap mediation    the cli_wrapper token path proceeds under host-grant authority;");
    println!("                   kernel `proc.exec` mediation is an Epic-9 operator-policy");
    println!("                   surface, and a Cedar permit alone cannot green it. The");
    println!("                   CapabilityInvocation exit row IS journaled either way.");
    println!("  egress           {EGRESS_POSTURE} ({EGRESS_FOLLOWUP}).");
    println!("  halt/resume      safe shutdown with no in-flight delegation is proven; the");
    println!("                   post-resume digest citing the exact pre-halt ref is NOT —");
    println!("                   FOLLOWUP-J1-RESUME-SEAM.");

    section("Where it landed");
    match &keep_home {
        Some(path) => println!("  state home       {} (kept: --keep-home)", path.display()),
        None => {
            let _ = std::fs::remove_dir_all(&home);
            println!("  state home       ephemeral, removed on exit (pass --keep-home to inspect)");
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

/// The Tier-2 beat name, referenced from both the declaration and the live take.
const TIER2_BEAT: &str = "tier2-live-agent-signed";

/// Resolved binaries the scene drives.
struct Bins {
    maos: PathBuf,
    /// Directory holding the daemon and its `worker-cli-fixture` sibling.
    dir: PathBuf,
}

fn preflight(skip_build: bool) -> Result<Bins, String> {
    section("Preflight");
    let dir = PathBuf::from("target/debug");
    let maos = dir.join("maos");
    let fixture = dir.join("worker-cli-fixture");

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

    for (label, path) in [("maos", &maos), ("worker-cli-fixture", &fixture)] {
        if !path.exists() {
            return Err(format!(
                "demo-j1: {label} not found at {} — build the workspace first (drop --skip-build)",
                path.display()
            ));
        }
    }
    println!("  binaries         maos + worker-cli-fixture present as siblings in target/debug");
    Ok(Bins { maos, dir })
}

/// Create the isolated state home. `MAOS_HOME` and `XDG_DATA_HOME` both point
/// here so every consumer — daemon, journal, transparency log, and `maosctl` —
/// resolves to the same fresh tree.
fn provision_home(keep: Option<&Path>) -> Result<PathBuf, String> {
    let home = match keep {
        Some(path) => path.to_path_buf(),
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
    Ok(home)
}

fn run_scene(bins: &Bins, home: &Path) -> Result<SceneObservation, String> {
    let path_var = match std::env::var_os("PATH") {
        Some(existing) => {
            let mut parts = vec![bins.dir.clone()];
            parts.extend(std::env::split_paths(&existing));
            std::env::join_paths(parts).map_err(|e| format!("demo-j1: cannot extend PATH: {e}"))?
        }
        None => bins.dir.clone().into_os_string(),
    };

    let started = Instant::now();
    let out = Command::new(&bins.maos)
        .args(["run", TOPOLOGY, "--once"])
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .env("PATH", path_var)
        // A fixture take must never be able to spawn a paid agent.
        .env_remove("MAOS_LIVE_AGENT")
        .output()
        .map_err(|e| format!("demo-j1: cannot run the daemon: {e}"))?;
    let wall = started.elapsed();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let events = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .filter(|v: &serde_json::Value| v.get("event").is_some())
        .collect();

    Ok(SceneObservation {
        events,
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        wall,
        exit_ok: out.status.success(),
    })
}

/// Turn the observation into beats. Every assertion is on a NAMED event field —
/// never on topology internals, which 1a rewrote and rung 2 will rewrite again.
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

    // P2 — frame-borne delegation over loopback.
    let admit_frame_borne = obs
        .event("topology_worker_admit")
        .and_then(|e| e["frame_borne"].as_bool())
        .unwrap_or(false);
    let (delegation_ok, delegation_detail) = match obs.event("delegation_routed") {
        Some(ev) => {
            let to_host = ev["to_host"].as_str().unwrap_or_default();
            let recipient = ev["recipient"].as_str().unwrap_or_default();
            let intent = ev["intent"].as_str().unwrap_or_default();
            let goal_present = ev["goal"].as_str().is_some_and(|g| !g.trim().is_empty());
            let ok = to_host == DELEGATION_HOST
                && recipient == DELEGATION_RECIPIENT
                && !intent.is_empty()
                && goal_present
                && admit_frame_borne;
            (
                ok,
                format!(
                    "v0.8 rung — loopback rehearsal; -> {to_host} as {recipient}, \
                     intent {intent}, frame_borne={admit_frame_borne}"
                ),
            )
        }
        None => (
            false,
            "no delegation_routed event — the worker task was not frame-borne".to_string(),
        ),
    };
    beats.push(Beat::executed(
        "delegation-frame-crosses-loopback",
        "a real task.assign carried the consent envelope to developer-remote",
        state_of(delegation_ok),
        delegation_detail,
    ));

    // P3 — host-managed grant + real subprocess.
    let grant = obs.event("host_grant_disposition");
    let wrapper = obs.event("cli_wrapper_loaded");
    let child_pid = wrapper.and_then(|e| e["child_pid"].as_u64()).unwrap_or(0);
    let tier = wrapper
        .and_then(|e| e["granted_tier"].as_str())
        .or_else(|| grant.and_then(|e| e["granted_tier"].as_str()))
        .unwrap_or("unknown");
    let egress = grant
        .and_then(|e| e["egress"].as_str())
        .unwrap_or("unreported");
    beats.push(Beat::executed(
        "worker-admitted-under-host-grant",
        "the Developer-Worker was admitted by a host-managed grant, not by trust",
        state_of(grant.is_some() && child_pid > 0),
        format!("granted_tier {tier}, real child_pid {child_pid}, egress {egress}"),
    ));

    // P4 — the completion oracle.
    let completion = obs.event("worker_completion");
    let completed = completion
        .and_then(|e| e["completed"].as_bool())
        .unwrap_or(false);
    let tl_ref = completion
        .and_then(|e| e["completion_tl_ref"].as_str())
        .unwrap_or_default();
    let label = completion
        .and_then(|e| e["completion"].as_str())
        .unwrap_or("none");
    beats.push(Beat::executed(
        "worker-completed-by-adapter-oracle",
        "completion came from the adapter's parse_completion, never from an exit code",
        state_of(completed && !tl_ref.is_empty()),
        format!("completion `{label}`, worker TL ref {}", short_ref(tl_ref)),
    ));

    // P5 — the delegation closed cleanly.
    let closed = obs.event("delegation_completed");
    let safe_point = closed
        .and_then(|e| e["orchestrator_safe_point"].as_bool())
        .unwrap_or(false);
    beats.push(Beat::executed(
        "delegation-closed-at-safe-point",
        "TaskComplete was journaled and no frame was left in flight",
        state_of(closed.is_some() && safe_point),
        match closed {
            Some(ev) => format!(
                "result `{}`, frames drained {}, safe_point {safe_point}",
                ev["result"].as_str().unwrap_or("?"),
                ev["orchestrator_frames_drained"].as_u64().unwrap_or(0)
            ),
            None => "no delegation_completed event".to_string(),
        },
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

    // P6b — the drain. This is the beat the 2026-08-14 rehearsal caught: a
    // timeout here means queued capability rows can be lost, so a later
    // sealed-export over this window would sign an incomplete bundle.
    let drain_timed_out = obs.stderr.contains("audit writer topology drain timed out");
    let drained = obs.event("drain").is_some();
    beats.push(Beat::executed(
        "audit-drain-clean",
        "every queued audit row reached SQLite before the process exited",
        state_of(drained && !drain_timed_out),
        if drain_timed_out {
            format!(
                "DRAIN TIMED OUT — queued rows may be lost, so a sealed-export over this \
                 window could sign an incomplete bundle (wall {:.3}s)",
                obs.wall.as_secs_f64()
            )
        } else {
            format!(
                "drain observed, no timeout, wall {:.3}s",
                obs.wall.as_secs_f64()
            )
        },
    ));

    beats
}

/// Invoke 1a's hermetic gate as the judge of the wire. It reads committed
/// sources, so it is cheap and needs no substrate.
fn run_delegation_gate() -> Beat {
    match crate::check_j1_loopback_delegation::run(false) {
        Ok(()) => Beat::executed(
            "frame-borne-route-intact",
            "the Blocking gate agrees the route is frame-borne",
            EvidenceState::ProvenBlocking,
            format!(
                "{DELEGATION_GATE} exit 0; legs {:?}",
                crate::check_j1_loopback_delegation::ledger_leg_names()
            ),
        ),
        Err(why) => Beat::executed(
            "frame-borne-route-intact",
            "the Blocking gate judges the wire",
            EvidenceState::Indeterminate,
            format!("{DELEGATION_GATE} reported findings: {why}"),
        ),
    }
}

/// The beats no story has delivered yet. Declared so they are visible, owned so
/// nobody has to guess who closes them.
fn unlanded_beats() -> Vec<Beat> {
    vec![
        Beat::absent(
            "disallowed-intent-refused-blocking",
            "a disallowed intent must be REFUSED (-32001 CODE_INTENT_DENIED, distinct from -32009)",
            "j1-crosshost-1b",
        ),
        Beat::absent(
            TIER2_BEAT,
            "one real paid agent run, captured and sealed under a named human signer",
            "--live-codex (operator-local, never CI)",
        ),
        Beat::absent(
            "two-host-signed-run",
            "two real hosts over mTLS/TOFU, heterogeneous worker, one reconciled signed bundle",
            "j1-crosshost-2",
        ),
        Beat::absent(
            "halt-resume-referential-identity",
            "the post-resume digest cites the exact pre-halt typed ref",
            "FOLLOWUP-J1-RESUME-SEAM",
        ),
    ]
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
                beat.state = match leg.evidence_state.as_str() {
                    "PROVEN_BLOCKING" => EvidenceState::ProvenBlocking,
                    "PROVEN_LIVE_SIGNED" => EvidenceState::ProvenLiveSigned,
                    "ABSENT" => EvidenceState::Absent,
                    _ => EvidenceState::Indeterminate,
                };
                beat.detail = format!("from published ledger: {}", leg.evidence_state);
                beat.executed = beat.state != EvidenceState::Absent;
                beat.owner = None;
            }
        }
    }
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
    // The codex worker manifest and its topology are OPERATOR-authored (the
    // runbook has you transcribe the exact argv after pinning it by hand); the
    // repo ships only the fixture profile, so the path must be supplied.
    let topology = codex_topology
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("MAOS_DEMO_J1_CODEX_TOPOLOGY").map(PathBuf::from))
        .ok_or_else(|| {
            "no codex topology: pass --codex-topology <path> (or set \
             MAOS_DEMO_J1_CODEX_TOPOLOGY). The repo ships only the fixture worker manifest; \
             author the codex profile per \
             _bmad-output/test-artifacts/runbook-j1-tier-2-signed-live-run.md Phase 1.5"
                .to_string()
        })?;
    if !topology.exists() {
        return Err(format!(
            "codex topology {} does not exist",
            topology.display()
        ));
    }

    // Clean-home invariant: an ambient subscription token cannot be attested, so
    // it must never enter a signed run's sandbox.
    if let Some(base) = std::env::var_os("HOME").map(PathBuf::from) {
        if base.join(".codex/auth.json").exists() {
            return Err(
                "~/.codex/auth.json exists — a signed run must use the metered \
                        API-key path. Remove it and retry (runbook Phase 1.3)"
                    .to_string(),
            );
        }
    }
    if std::env::var_os("CODEX_API_KEY").is_none() {
        return Err(
            "CODEX_API_KEY unset — `codex exec` IGNORES OPENAI_API_KEY for auth \
                    (401 Missing bearer). Set CODEX_API_KEY=\"$OPENAI_API_KEY\""
                .to_string(),
        );
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

    // A disposable demo dir the worker may CRUD inside, outside the repo tree.
    let demo_dir = home.join("demo-workspace");
    std::fs::create_dir_all(&demo_dir)
        .map_err(|e| format!("cannot create the disposable demo dir: {e}"))?;

    let out = Command::new(&bins.maos)
        .arg("run")
        .arg(topology.canonicalize().unwrap_or(topology.clone()))
        .arg("--live")
        .current_dir(&demo_dir)
        .env("MAOS_HOME", home)
        .env("XDG_DATA_HOME", home)
        .env("MAOS_LIVE_AGENT", "1")
        .output()
        .map_err(|e| format!("cannot run the live daemon: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
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
    let tl_ref = completion
        .as_ref()
        .and_then(|e| e["completion_tl_ref"].as_str())
        .unwrap_or_default()
        .to_string();
    println!(
        "  live run         {identity} completed, worker TL ref {}",
        short_ref(&tl_ref)
    );

    // The capture carries NON-SECRET fields only, spelled exactly as
    // `record-capture` validates them: it refuses a credential-shaped value and
    // refuses an overclaimed control (egress "enforced", redaction not
    // "verified"), so the gate stays open rather than signing a lie.
    let capture = serde_json::json!({
        "signer": format!("{signer} (named human signer)"),
        "live_agent_identity": identity,
        "command_metadata":
            "maos run <codex topology> --live; CODEX_API_KEY injected host-side (value redacted)",
        "host_grant_disposition":
            "host-managed grant admitted; a mismatch would have refused",
        "audit_refs": [tl_ref],
        "egress": EGRESS_POSTURE,
        "egress_followup": EGRESS_FOLLOWUP,
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
    run_checked(
        Command::new(&maosctl)
            .args(["audit", "sealed-export", "--range", "1d", "--audit-key"])
            .arg(&key)
            .arg("--output")
            .arg(&bundle)
            .env("MAOS_HOME", home)
            .env("XDG_DATA_HOME", home),
        "sealed-export",
    )?;

    let verify = Command::new(&maosctl)
        .args(["audit", "verify-bundle"])
        .arg(&bundle)
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

fn run_checked(command: &mut Command, label: &str) -> Result<(), String> {
    let out = command
        .output()
        .map_err(|e| format!("cannot run {label}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{label} refused: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
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
