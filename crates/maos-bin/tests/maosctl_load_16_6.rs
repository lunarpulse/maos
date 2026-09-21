//! Story 16-6 — `maosctl load` against a REAL `maos run` root.
//!
//! # What this file exists to prove
//!
//! At HEAD an operator could not add a Spirit to a running daemon at all, and
//! `maosctl start` had no subject it could deliberately create: every root
//! loads, admits and starts in one unbroken run BEFORE the serving loop the
//! door answers from, so by the time any `maosctl` command can reach the
//! daemon no Spirit is ever in `Loaded`. 16-1 shipped `start` with nothing to
//! start.
//!
//! So the headline here is not one verb. It is the SEQUENCE:
//!
//! ```text
//! load → Loaded → start → Running → pause → Paused → resume → Running → unload → gone
//! ```
//!
//! FR9's five verbs against one Spirit, for the first time.
//!
//! # Why the state is always read back from the daemon
//!
//! Every assertion reads `lifecycle_state` through `GET /v1/spirits/{id}`,
//! which the door serves from `SpiritControlBlock::current_state`. A handler
//! that journalled a transition it never performed — the exact defect 16-1
//! was written to remove — cannot pass.
//!
//! # Isolation
//!
//! Each root gets its own `HOME`, `MAOS_HOME` and `XDG_DATA_HOME`
//! (D-16-1-Q; CI plants a decoy `control.json` on `HOME`). Roots are not
//! shared across state-changing concerns: `unload` is terminal for the
//! Spirit it touches, so sharing would make outcomes order-dependent.

#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The root Spirit. Butler runs from a cassette, so the root is hermetic.
const BUTLER_MANIFEST: &str = "spirits/butler/manifest.toml";
const BUTLER_CASSETTE: &str = "crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json";

/// The Spirit the operator ADDS. `reviewer` is chosen for measured reasons:
/// it is one of the eight compiled-in classes, `classify_spirit` maps it,
/// `build_spirit_obj` can construct it, it is `trust_tier = "local"` with
/// `[posture] default/allowed_max = assistive`, and neither the butler
/// boot-loud guard nor the mira scalar-port guard applies to it — so loading
/// it exercises the admission path without dragging in a second Spirit's
/// boot-loud wiring.
const REVIEWER_MANIFEST: &str = "spirits/reviewer/manifest.toml";

const READY_TIMEOUT: Duration = Duration::from_secs(45);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

fn maos() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_maos"))
}

/// `maosctl` is a `[[bin]]` of a SIBLING package, so `CARGO_BIN_EXE_maosctl`
/// is not defined for this target. Resolve it beside the `maos` binary cargo
/// built for us — same profile directory.
///
/// ⚠ `maos-cli` is a plain `[dependencies]` entry of `maos-bin`, so cargo
/// builds its LIB and never its `[[bin]] maosctl`. The CI job for this test
/// carries an explicit `cargo build --locked -p maos-cli --bin maosctl` step
/// for that reason; without it a cold runner reaches this assertion.
fn maosctl() -> PathBuf {
    let path = maos().with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli --bin maosctl` first",
        path.display()
    );
    path
}

struct Root {
    home: PathBuf,
    child: Option<Child>,
    scratch: PathBuf,
}

impl Root {
    fn butler(label: &str) -> Self {
        let scratch = std::env::temp_dir().join(format!(
            "maos-load-16-6-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let home = scratch.join("home");
        std::fs::create_dir_all(&home).expect("create scratch home");

        let init = Command::new(maos())
            .arg("init")
            .env("HOME", &scratch)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maos init");
        assert!(
            init.status.success(),
            "maos init must exit 0 on a fresh home: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        let child = Command::new(maos())
            .args(["run", BUTLER_MANIFEST])
            .env("HOME", &scratch)
            .env("MAOS_HOME", &home)
            .env("XDG_DATA_HOME", scratch.join("xdg"))
            .env("MAOS_INFERENCE_MODE", "replay")
            .env("MAOS_REPLAY_CASSETTE", BUTLER_CASSETTE)
            .env("MAOS_NOTIFY_DISABLE", "1")
            .current_dir(workspace_root())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn maos run");

        let root = Self {
            home,
            child: Some(child),
            scratch,
        };
        root.await_running("butler");
        root
    }

    /// Readiness is the SCB state, NOT a successful connect: the door binds
    /// before `scheduler.start`, so a connect-ready door can still answer 404.
    fn await_running(&self, spirit_id: &str) {
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if self.lifecycle_state(spirit_id).as_deref() == Some("Running") {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("the door never reported {spirit_id} Running within {READY_TIMEOUT:?}");
    }

    fn ctl(&self, args: &[&str]) -> std::process::Output {
        Command::new(maosctl())
            .args(args)
            .env("HOME", &self.scratch)
            .env("MAOS_HOME", &self.home)
            .env("XDG_DATA_HOME", self.scratch.join("xdg"))
            .current_dir(workspace_root())
            .output()
            .expect("run maosctl")
    }

    fn field(&self, spirit_id: &str, key: &str) -> Option<String> {
        let out = self.ctl(&["spirit", "inspect", spirit_id]);
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .map(str::to_owned)
    }

    fn lifecycle_state(&self, spirit_id: &str) -> Option<String> {
        self.field(spirit_id, "lifecycle_state")
    }

    /// Every Transparency-Log row this root wrote, as `(spirit_pid, intent)`.
    ///
    /// Read-only sqlite rather than `maosctl audit query`: that reader runs
    /// the FR4 schema validator and butler mints no capability token in
    /// replay mode, so every row would fail validation before an assertion
    /// could see it. Read-only so it cannot hold the lock the live daemon is
    /// racing.
    fn tl_rows(&self) -> Vec<(i64, String)> {
        let db = self.home.join("audit").join("transparency.sqlite");
        if !db.exists() {
            return Vec::new();
        }
        let conn = rusqlite::Connection::open_with_flags(
            &db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("open the Transparency Log read-only");
        let mut statement = conn
            .prepare("SELECT spirit_pid, intent FROM transparency_log ORDER BY rowid")
            .expect("prepare TL read");
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query the TL");
        rows.map(|row| row.expect("TL row")).collect()
    }

    fn intents(&self) -> Vec<String> {
        self.tl_rows()
            .into_iter()
            .map(|(_, intent)| intent)
            .collect()
    }

    fn terminate(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &child.id().to_string()])
                .status();
        }
        #[cfg(not(unix))]
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        self.terminate();
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

fn manifest_path() -> String {
    workspace_root()
        .join(REVIEWER_MANIFEST)
        .to_string_lossy()
        .into_owned()
}

/// AC1 + AC3 — the whole operator sequence, against one live root.
///
/// This is FR9's five verbs against one Spirit for the first time in the
/// tree's history, and every intermediate state is read back from the daemon
/// that owns it.
#[test]
fn load_start_pause_resume_unload_against_a_live_root() {
    let root = Root::butler("sequence");
    let manifest = manifest_path();

    // Nothing named `reviewer` exists in this root yet.
    assert_eq!(
        root.lifecycle_state("reviewer"),
        None,
        "the root boots with butler only"
    );

    // ── load ── the verb this story ships.
    let out = root.ctl(&["load", &manifest]);
    assert!(
        out.status.success(),
        "maosctl load must exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        root.lifecycle_state("reviewer").as_deref(),
        Some("Loaded"),
        "load leaves the Spirit LOADED — it does not start it. Collapsing the two \
         would leave `maosctl start` with no operator-creatable subject, which is \
         the state it has been in since 16-1 shipped it"
    );
    let out = root.ctl(&["load", &manifest]);
    assert!(
        !out.status.success(),
        "a duplicate load must be refused rather than silently re-admitted"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("HTTP 409") && stderr.contains("already_loaded"),
        "a duplicate load is specifically the typed 409 already_loaded conflict; got: {stderr}"
    );

    // ── start ── its first operator-created subject.
    let out = root.ctl(&["start", "reviewer"]);
    assert!(
        out.status.success(),
        "maosctl start must exit 0 from Loaded: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(root.lifecycle_state("reviewer").as_deref(), Some("Running"));

    // ── pause / resume ──
    assert!(root.ctl(&["pause", "reviewer"]).status.success());
    assert_eq!(root.lifecycle_state("reviewer").as_deref(), Some("Paused"));
    assert!(root.ctl(&["resume", "reviewer"]).status.success());
    assert_eq!(root.lifecycle_state("reviewer").as_deref(), Some("Running"));

    // ── unload ── gone, not merely reported gone.
    let out = root.ctl(&["unload", "reviewer"]);
    assert!(
        out.status.success(),
        "maosctl unload must exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        root.lifecycle_state("reviewer"),
        None,
        "unload removes the SCB; `spirit inspect` must 404, not report Unloaded"
    );

    // AC4 — the audit record must say what the operator actually did. The
    // compiler proves `command_verb` has a `Load` arm; nothing proves the
    // LABEL is right, and that label is the audit record.
    let intents = root.intents();
    assert!(
        intents
            .iter()
            .any(|intent| intent.starts_with("operator.load.") && intent.ends_with(":completed")),
        "the completion row must read `operator.load.<operation_id>:completed`; got {intents:?}"
    );
}

/// AC3's discriminating half — `start` is `Loaded → Running` and NOTHING
/// else, and at HEAD there was no `Loaded` Spirit an operator could create.
///
/// The claim must be worded `operator-creatable`, never `existent`: a failed
/// Worker `start` strands a durable `Loaded` SCB inside a serving root, so a
/// falsifier asserting "no `Loaded` SCB exists at HEAD" is disprovable. What
/// is true is that no operator could deliberately produce one — `maosctl
/// start` could only ever answer 409 — and `load` is what gives it a subject.
#[test]
fn start_on_a_running_spirit_is_refused_with_the_d16_1_j_conflict() {
    let root = Root::butler("start-409");

    let out = root.ctl(&["start", "butler"]);
    assert!(
        !out.status.success(),
        "start only moves a LOADED Spirit to Running"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("invalid_state_transition") && stderr.contains("HTTP 409"),
        "the refusal must be the typed D-16-1-J conflict; got: {stderr}"
    );
    assert_eq!(
        root.lifecycle_state("butler").as_deref(),
        Some("Running"),
        "a refused start must not have moved anything"
    );
}

/// AC2(a)(c) — the kernel arm, end to end over the door.
///
/// A Spirit loaded and never started must unload, and its name must be free
/// afterwards. At HEAD the first `unload` returned 409
/// `invalid_state_transition`, the SCB stayed in the map forever, and
/// `resolve_pid`'s linear scan kept matching it — so every later `load` of
/// that id answered `AlreadyLoaded`. The name was burned by using it once.
#[test]
fn a_loaded_but_never_started_spirit_unloads_and_frees_its_name() {
    let root = Root::butler("unload-from-loaded");
    let manifest = manifest_path();

    assert!(root.ctl(&["load", &manifest]).status.success());
    assert_eq!(
        root.lifecycle_state("reviewer").as_deref(),
        Some("Loaded"),
        "the Spirit must be in the state this story made durable"
    );

    let out = root.ctl(&["unload", "reviewer"]);
    assert!(
        out.status.success(),
        "Loaded → Unloaded is the FLAG-Winston kernel arm; at HEAD this was 409 \
         invalid_state_transition: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(root.lifecycle_state("reviewer"), None);

    // The name is not burned.
    let out = root.ctl(&["load", &manifest]);
    assert!(
        out.status.success(),
        "re-loading the same id after a clean unload must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(root.lifecycle_state("reviewer").as_deref(), Some("Loaded"));
}

/// AC2(e) + D-16-6-D — the critical section, driven CONCURRENTLY.
///
/// The kernel's own duplicate guard is not atomic: `resolve_pid` and
/// `spirits.insert` are separate lock acquisitions with the whole of
/// `admit_spirit` between them, and the CRL `admission_guard` that would
/// otherwise serialize them is `None` by default. Two concurrent loads of the
/// same id would both observe `None` and both insert, under different pids.
///
/// Exactly one must win.
///
/// ⚠ STATED, NOT ASSUMED — the limit of this falsifier. Deleting the door's
/// pre-CAS lock key and re-running it still PASSES: two `maosctl` process
/// spawns do not interleave tightly enough to land inside the kernel's
/// window. So this test asserts the CONTRACT but does not by itself
/// discriminate the lock. The lock's necessity rests on the measured shape
/// of `SpiritSchedulerAdapter::load` — two `RwLock` acquisitions separated
/// by `allocate_pid()` and the whole of `admit_spirit` — not on this test
/// going red. A discriminating falsifier needs two in-process submissions
/// against one port, which is the shell-host surface, not the CLI's.
#[test]
fn concurrent_loads_of_the_same_id_admit_exactly_one() {
    use std::sync::{Arc, Barrier};

    let root = Arc::new(Root::butler("concurrent"));
    let manifest = manifest_path();
    let barrier = Arc::new(Barrier::new(2));

    let handles: Vec<_> = (0..2)
        .map(|_| {
            let root = Arc::clone(&root);
            let barrier = Arc::clone(&barrier);
            let manifest = manifest.clone();
            std::thread::spawn(move || {
                barrier.wait();
                root.ctl(&["load", &manifest])
            })
        })
        .collect();

    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("load thread"))
        .collect();

    let winners = outcomes.iter().filter(|out| out.status.success()).count();
    assert_eq!(
        winners,
        1,
        "exactly one concurrent load may win; got {winners}. stderr: {:?}",
        outcomes
            .iter()
            .map(|out| String::from_utf8_lossy(&out.stderr).into_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        root.lifecycle_state("reviewer").as_deref(),
        Some("Loaded"),
        "the winner's Spirit is loaded exactly once"
    );
    let loser = outcomes
        .iter()
        .find(|out| !out.status.success())
        .expect("exactly one concurrent load loses");
    let loser_stderr = String::from_utf8_lossy(&loser.stderr);
    // §17 R12's prohibition is `handler_still_running` — contention resolved
    // AFTER the point of no return. The pre-CAS lock guarantees the loser
    // never starts; which typed refusal it gets depends on a race the test
    // cannot control: a winner whose handler outlasts the loser's `lock_wait`
    // leaves the lock held at expiry → `spirit_busy`; a fast winner frees the
    // lock, the loser's bounded wait acquires it, and the kernel answers the
    // honest typed 409 `already_loaded`. Both prove the loser never started.
    assert!(
        (loser_stderr.contains("spirit_busy") || loser_stderr.contains("already_loaded"))
            && !loser_stderr.contains("handler_still_running"),
        "the lock loser must never start; got: {loser_stderr}"
    );
}

/// AC1's refusal vectors — each a DISTINCT typed outcome, and each leaving
/// NOTHING behind.
///
/// A gate that refuses but leaks is the §1 defect wearing a different hat, so
/// every vector asserts the daemon's Spirit set is unchanged afterwards.
#[test]
fn every_refusal_vector_is_typed_and_leaves_nothing_behind() {
    let root = Root::butler("refusals");
    let scratch = root.scratch.join("manifests");
    std::fs::create_dir_all(&scratch).expect("scratch manifests");

    let baseline = daemon_roster(&root);
    assert_eq!(baseline, vec!["butler"], "butler is the only Spirit");

    let write = |name: &str, body: &str| -> String {
        let path = scratch.join(name);
        std::fs::write(&path, body).expect("write fixture manifest");
        path.to_string_lossy().into_owned()
    };

    // Gate 2 — unparseable TOML.
    let bad_toml = write(
        "bad.toml",
        "[class]\nname = \"bad-toml\"\nmanifest_schema_version = = 2\n",
    );
    // Gate 3 — `[cli_wrapper]` ⊕ `[class]`.
    let conflict = write(
        "conflict.toml",
        "[cli_wrapper]\ncommand = \"true\"\n[class]\nname = \"conflict-spirit\"\n",
    );
    // Gate 4 — no `[class]`.
    let no_class = write("no-class.toml", "[sandbox]\ntier = \"T2\"\n");
    // Gate 5 — a class this binary cannot construct. `rust-inproc` admits
    // first-party compiled-in classes only (ADR-060); the third-party form is
    // Epic 17's `wasm-component`.
    let unknown_class = write(
        "unknown.toml",
        "[class]\nname = \"totally-not-a-spirit\"\nversion = \"0.1.0\"\nabi = \"1.0\"\n\
         manifest_schema_version = 3\nmin_substrate_version = \"0.0.1\"\n\
         forms = [\"rust-inproc\"]\ntrust_tier = \"local\"\ndescription = \"x\"\n",
    );

    for (label, path, code, refused_id) in [
        (
            "unparseable TOML",
            bad_toml,
            "manifest_parse_failed",
            "bad-toml",
        ),
        (
            "cli_wrapper ⊕ class",
            conflict,
            "manifest_schema_conflict",
            "conflict-spirit",
        ),
        (
            "missing [class]",
            no_class,
            "class_section_invalid",
            "missing-class",
        ),
        (
            "unknown class",
            unknown_class,
            "unknown_spirit_class",
            "totally-not-a-spirit",
        ),
    ] {
        let out = root.ctl(&["load", &path]);
        assert!(
            !out.status.success(),
            "'{label}' must be refused, not admitted"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("HTTP 400") && stderr.contains(code),
            "'{label}' must be its exact typed 400 {code}; got: {stderr}"
        );
        assert_eq!(
            root.lifecycle_state(refused_id),
            None,
            "'{label}' must not leave its refused Spirit behind"
        );
        assert_eq!(
            daemon_roster(&root),
            baseline,
            "'{label}' must leave the daemon roster unchanged"
        );
    }

    // Gate 1 — an unreadable manifest is refused by the CLIENT before any
    // round trip (its own exit 1), which is the honest place for it.
    let out = root.ctl(&["load", &scratch.join("absent.toml").to_string_lossy()]);
    assert_eq!(out.status.code(), Some(1));

    // Gate 1 (daemon side) — a NON-REGULAR file. The client canonicalises a
    // directory happily; only the daemon can refuse it.
    let out = root.ctl(&["load", &scratch.to_string_lossy()]);
    assert!(
        !out.status.success(),
        "a directory is not a manifest and must be refused by the daemon"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("HTTP 400") && stderr.contains("manifest_unreadable"),
        "directory refusal must retain its typed code; got: {stderr}"
    );
    assert_eq!(daemon_roster(&root), baseline);
}

/// Admission runs after pid allocation, so a post-load substrate refusal must
/// unload its SCB and free its id for the next operator request.
#[test]
fn post_load_admission_refusal_rolls_back_and_frees_the_id() {
    let root = Root::butler("post-load-rollback");
    let baseline = daemon_roster(&root);
    let invalid_manifest = root.scratch.join("reviewer-needs-newer-substrate.toml");
    let reviewer = std::fs::read_to_string(workspace_root().join(REVIEWER_MANIFEST))
        .expect("read reviewer manifest fixture");
    let invalid = reviewer.replace(
        "min_substrate_version = \"0.1.0-alpha\"",
        "min_substrate_version = \"99.0.0\"",
    );
    assert_ne!(
        reviewer, invalid,
        "fixture must exercise the security admission substrate gate"
    );
    std::fs::write(&invalid_manifest, invalid).expect("write invalid substrate fixture");
    let invalid_manifest = invalid_manifest.to_string_lossy().into_owned();

    let out = root.ctl(&["load", &invalid_manifest]);
    assert!(
        !out.status.success(),
        "newer substrate requirement must refuse"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("HTTP 400") && stderr.contains("admission_refused"),
        "post-load security gate must preserve its typed refusal; got: {stderr}"
    );
    assert_eq!(root.lifecycle_state("reviewer"), None);
    assert_eq!(
        daemon_roster(&root),
        baseline,
        "post-load refusal must roll back the allocated pid and SCB"
    );

    let out = root.ctl(&["load", &manifest_path()]);
    assert!(
        out.status.success(),
        "the rolled-back id must be loadable afterwards: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(root.lifecycle_state("reviewer").as_deref(), Some("Loaded"));
}

/// Mira's diagnostic-confidence halt is synchronous and has no port on the
/// door construction path, so admission fails before class construction.
#[test]
fn mira_load_requires_an_epistemic_halt_port() {
    let root = Root::butler("mira-epistemic-port");
    let baseline = daemon_roster(&root);
    let manifest = workspace_root()
        .join("spirits/mira/manifest.toml")
        .to_string_lossy()
        .into_owned();

    let out = root.ctl(&["load", &manifest]);
    assert!(
        !out.status.success(),
        "Mira without its halt port must refuse"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("HTTP 400") && stderr.contains("epistemic_port_required"),
        "Mira's typed halt-port refusal must reach the door; got: {stderr}"
    );
    assert_eq!(root.lifecycle_state("mira"), None);
    assert_eq!(daemon_roster(&root), baseline);
}

/// AC1's R2 clause — a loaded Spirit must be VISIBLE.
///
/// `GET /v1/daemon` returned `spirit_ids` only, so a Spirit that was loaded
/// and then forgotten was indistinguishable from a working one on the only
/// surface that enumerates the daemon. Nothing supervises a `Loaded` Spirit —
/// the DRR picker selects only `Running` — so observability IS the contract:
/// it is the operator's cue to start it or unload it.
#[test]
fn a_loaded_spirit_is_distinguishable_from_a_running_one_on_the_daemon_surface() {
    let root = Root::butler("visibility");
    assert!(root.ctl(&["load", &manifest_path()]).status.success());

    // `spirit inspect` is the per-Spirit surface; the daemon surface is read
    // through the same door by the CLI's own discovery path. Read it directly
    // so the assertion is about the ROUTE, not about a renderer.
    let control = maos_domain::operator_door::ControlFile::load(
        &maos_domain::operator_door::control_file_path(&root.home),
    )
    .expect("control.json loads");
    let body = daemon_status(&control);

    let ids = body["spirit_ids"]
        .as_array()
        .expect("spirit_ids is an array of strings");
    let states = body["lifecycle_states"]
        .as_array()
        .expect("lifecycle_states is the new sibling key");
    assert_eq!(
        ids.len(),
        states.len(),
        "the two arrays are positionally aligned"
    );
    // ⚠ `spirit_ids` stays an array of STRINGS — reshaping it into objects
    // breaks `post_surface_16_1.rs`'s `daemon[\"spirit_ids\"][0] == \"butler\"`.
    assert!(ids.iter().all(|id| id.is_string()));

    let reviewer = ids
        .iter()
        .position(|id| id == "reviewer")
        .expect("the loaded Spirit appears in the daemon's roster");
    let butler = ids
        .iter()
        .position(|id| id == "butler")
        .expect("the running Spirit appears too");
    assert_eq!(states[reviewer], "Loaded");
    assert_eq!(states[butler], "Running");
    assert_ne!(
        states[reviewer], states[butler],
        "a loaded-and-forgotten Spirit must NOT look like a working one"
    );
}

/// One blocking `GET /v1/daemon` over the door, without a CLI in the way.
fn daemon_status(control: &maos_domain::operator_door::ControlFile) -> serde_json::Value {
    use std::io::Write;
    use std::net::TcpStream;

    let endpoint = control
        .endpoint
        .rsplit("://")
        .next()
        .expect("endpoint carries a host:port");
    let mut stream = TcpStream::connect(endpoint).expect("connect to the door");
    let request = format!(
        "GET /v1/daemon HTTP/1.1\r\nHost: {endpoint}\r\nAuthorization: Bearer {}\r\n\
         Connection: close\r\nContent-Length: 0\r\n\r\n",
        control.token
    );
    stream
        .write_all(request.as_bytes())
        .expect("write the request");
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("read the response");
    let text = String::from_utf8_lossy(&raw);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .expect("a well-formed HTTP response");
    serde_json::from_str(body).expect("the daemon route answers JSON")
}

/// The daemon's authoritative, sorted Spirit roster.
fn daemon_roster(root: &Root) -> Vec<String> {
    let control = maos_domain::operator_door::ControlFile::load(
        &maos_domain::operator_door::control_file_path(&root.home),
    )
    .expect("control.json loads");
    daemon_status(&control)["spirit_ids"]
        .as_array()
        .expect("spirit_ids is an array of strings")
        .iter()
        .map(|id| id.as_str().expect("spirit id is a string").to_owned())
        .collect()
}
