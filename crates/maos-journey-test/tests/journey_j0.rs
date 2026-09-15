#![forbid(unsafe_code)]

//! J0 — Evaluator surface journey (Grade A: production entry surface).
//!
//! Story 16-2 — the FULL J0 scene: `maos init` → `maos shell` → the
//! ambiguity halt → a resolution over EITHER surface (the door via
//! `maosctl halt resolve`, or a clarification typed in the REPL) → the REPL
//! renders the resolution and the Spirit PROCEEDS with the operator's
//! context → EOF unloads the Spirit with receipts. The Transparency Log and
//! approval log are asserted directly, test-side (kloc-free).
//!
//! Every bounded wait lives in the harness (`wait_for_screen`,
//! `wait_until`, `run_bounded`) per the JB-7 / H4 no-wallclock guard.

use maos_journey_test::{wait_until, AuditDb, JourneyWorld, Pty};

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn maos_bin() -> String {
    if let Some(bin) = option_env!("CARGO_BIN_EXE_maos") {
        return bin.to_string();
    }
    let debug_bin = workspace_root().join("target/debug/maos");
    if debug_bin.exists() {
        return debug_bin.to_string_lossy().into_owned();
    }
    panic!("maos binary not found — run `cargo build -p maos-bin` first");
}

/// `target/debug/maosctl` — built by `journey-hermetic-tier-1` and the
/// `j0-scene` job before this suite runs.
fn maosctl_bin() -> String {
    workspace_root()
        .join("target/debug/maosctl")
        .to_string_lossy()
        .into_owned()
}

fn j0_cassette_source() -> std::path::PathBuf {
    workspace_root().join("crates/maos-journey-test/cassettes/j0/shell-intro.json")
}

/// The J0 scene's shared setup: an isolated home with `maos init` done and
/// a COPY of the seed cassette (never the repo file — a record-mode leg must
/// not be able to rewrite the seed).
fn j0_world() -> (tempfile::TempDir, JourneyWorld) {
    let audit = AuditDb::temp();
    let cassette_copy = tempfile::NamedTempFile::new().expect("cassette copy");
    std::fs::copy(j0_cassette_source(), cassette_copy.path()).expect("copy cassette");
    let world = JourneyWorld::builder()
        .audit(audit)
        .cassette(cassette_copy.path().to_str().unwrap())
        .env("MAOS_INFERENCE_MODE", "replay")
        .build();
    let guard = tempfile::TempDir::new().expect("cassette guard");
    (guard, world)
}

/// `maos init` inside the world's home: one bounded run that must exit 0
/// and leave `control.json` in place. Idempotent — the D-16-2-K re-mint in
/// [`spawn_shell`] removes `control.json` and calls this a second time.
fn init_world(world: &JourneyWorld) {
    let init = maos_journey_test::run_bounded(world, &maos_bin(), &["init"], &[], 60, "maos init");
    assert!(
        init.code == Some(0),
        "maos init must exit 0; stderr:\n{}",
        init.stderr
    );
    let control = world.env()["MAOS_HOME"].to_string() + "/control.json";
    assert!(
        std::path::Path::new(&control).exists(),
        "control.json present"
    );
}

/// The EXACT banner line the shell renders once it is up — AC6 pins it
/// byte-for-byte (em-dash and double space included).
const J0_SHELL_BANNER: &str = "maos shell — type @hello-spirit <msg>  (Ctrl-D to exit)";

/// Spawn the `maos shell` PTY and wait bounded for the EXACT banner.
///
/// D-16-2-K: `maos init` probes the operator endpoint and records it in
/// `control.json`, but that port can be taken between init and the shell's
/// bind. The shell then fails fast — `… is already served by another MAOS
/// root (…) — EndpointInUse`, exit 78 — and the banner never appears. On
/// exactly that signature the world re-mints ONCE: remove `control.json`,
/// run `maos init` again, respawn, and record the re-mint on stderr. Any
/// other pre-banner failure, or a second missing banner, panics with the
/// screen.
fn spawn_shell(world: &JourneyWorld) -> Pty {
    let (pty, appeared, screen) = spawn_and_wait_for_banner(world);
    if appeared {
        return pty;
    }
    assert!(
        screen.contains("EndpointInUse"),
        "the shell died before its banner, and NOT on the D-16-2-K port race; \
         screen:\n{screen}"
    );
    eprintln!(
        "Story 16-2 / D-16-2-K — the probed port was taken between `maos init` \
         and the shell bind: re-minting ONCE (rm control.json && maos init)"
    );
    let control = std::path::PathBuf::from(&world.env()["MAOS_HOME"]).join("control.json");
    std::fs::remove_file(&control).expect("remove control.json for the re-mint");
    init_world(world);
    let (pty, appeared, screen) = spawn_and_wait_for_banner(world);
    assert!(
        appeared,
        "the banner must appear after the D-16-2-K re-mint; screen:\n{screen}"
    );
    pty
}

/// One spawn plus the bounded banner wait (15 s via the harness-owned
/// poll). Returns the PTY, whether the banner appeared, and the screen.
fn spawn_and_wait_for_banner(world: &JourneyWorld) -> (Pty, bool, String) {
    let cmd = maos_bin();
    let pty = Pty::spawn(&cmd, world);
    let appeared = pty.wait_for_screen(&[J0_SHELL_BANNER], std::time::Duration::from_secs(15));
    let screen = pty.screen().text().to_string();
    (pty, appeared, screen)
}

fn ctl(world: &JourneyWorld, args: &[&str]) -> maos_journey_test::BoundedRun {
    maos_journey_test::run_bounded(world, &maosctl_bin(), args, &[], 60, "maosctl")
}

/// Extract the halt id from the PTY screen (`halt <ULID> — …`).
fn halt_id_from(screen: &str) -> String {
    let start = screen.find("halt ").expect("the halt line renders") + "halt ".len();
    let candidate: String = screen[start..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    assert_eq!(
        candidate.len(),
        26,
        "the halt id is a 26-char ULID; screen:\n{screen}"
    );
    candidate
}

/// Assert the full TL/approval-log row set AC1 pins — test-side, against the
/// real stores the session wrote.
fn assert_j0_row_set(world: &JourneyWorld, halt_id: &str, spirit_pid: u32, clarification: &str) {
    let tl_path = std::path::PathBuf::from(&world.env()["MAOS_HOME"])
        .join("audit")
        .join("transparency.sqlite");
    let conn = rusqlite::Connection::open(&tl_path).expect("open TL");

    // Exactly one kind-3 row at the Spirit's pid whose payload halt_id is
    // the id (the planned-unload marker/receipt rows do not carry it).
    let raised: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transparency_log \
             WHERE kind = 3 AND spirit_pid = ?1 \
             AND payload_redacted LIKE '%' || ?2 || '%'",
            rusqlite::params![spirit_pid as i64, halt_id],
            |row| row.get(0),
        )
        .expect("count kind-3 rows");
    // Exactly ONE kind-3 row carries the id — the RAISED halt. (The halt was
    // resolved before EOF, so the planned-unload termination drains nothing
    // pending: its marker + synthetic term-… receipt carry no raised id.)
    assert_eq!(raised, 1, "exactly the raised halt row carries the id");

    // Exactly one approval row with the exact reasoning AC1 pins. COUNT
    // first: query_row alone would silently take the first of many rows on
    // a double-resolution regression (16-2 §A6 obligation (d)).
    let approvals: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approval_decision_log \
             WHERE target = 'hello-spirit' AND capability = 'halt.resolve'",
            [],
            |row| row.get(0),
        )
        .expect("count approval rows");
    assert_eq!(approvals, 1, "exactly one halt.resolve approval row");
    let reasoning: String = conn
        .query_row(
            "SELECT reasoning FROM approval_decision_log \
             WHERE target = 'hello-spirit' AND capability = 'halt.resolve'",
            [],
            |row| row.get(0),
        )
        .expect("the approval row exists");
    assert_eq!(
        reasoning,
        format!("halt={halt_id}: provided_context: {clarification}")
    );

    // Exactly one kind-4 COMPLETION row: intent prefix + `:completed`,
    // payload outcome + spirit_id. A concurrent REFUSED resolve over the
    // door adds a second kind-4 row ending `:halt_already_resolved`
    // (truthful per Trap 4), so the count — and both reads below — pin the
    // `:completed` row and can never pick a refusal row.
    let completions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transparency_log \
             WHERE kind = 4 AND intent LIKE 'operator.halt-resolve.%' \
             AND intent LIKE '%:completed'",
            [],
            |row| row.get(0),
        )
        .expect("count kind-4 completion rows");
    assert_eq!(
        completions, 1,
        "exactly one `…:completed` kind-4 completion row"
    );
    let intent: String = conn
        .query_row(
            "SELECT intent FROM transparency_log \
             WHERE kind = 4 AND intent LIKE 'operator.halt-resolve.%' \
             AND intent LIKE '%:completed'",
            [],
            |row| row.get(0),
        )
        .expect("the completion row exists");
    let payload_text: String = conn
        .query_row(
            "SELECT CAST(payload_redacted AS TEXT) FROM transparency_log \
             WHERE kind = 4 AND intent LIKE 'operator.halt-resolve.%' \
             AND intent LIKE '%:completed'",
            [],
            |row| row.get(0),
        )
        .expect("the completion payload exists");
    assert!(
        intent.starts_with("operator.halt-resolve.") && intent.ends_with(":completed"),
        "completion intent carries the outcome; got: {intent}"
    );
    let payload: serde_json::Value = serde_json::from_str(&payload_text).expect("payload parses");
    assert_eq!(payload["outcome"], "completed");
    assert_eq!(payload["spirit_id"], "hello-spirit");

    // The Lifecycle Journal carries Load and Halt for hello-spirit.
    let journal_path = std::path::PathBuf::from(&world.env()["MAOS_HOME"])
        .join("journal")
        .join("lifecycle.ndjson");
    let journal = std::fs::read_to_string(journal_path).expect("read journal");
    assert!(journal.contains("\"spirit_id\":\"hello-spirit\""));
    assert!(
        journal.contains("Load") && journal.contains("Halt"),
        "journal carries Load and Halt; journal:\n{journal}"
    );

    // The session's shell.turn and cap.issue rows exist at the Spirit's pid.
    for intent_sub in ["shell.turn", "cap.issue"] {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transparency_log \
                 WHERE spirit_pid = ?1 AND intent LIKE '%' || ?2 || '%'",
                rusqlite::params![spirit_pid as i64, intent_sub],
                |row| row.get(0),
            )
            .expect("count");
        assert!(n > 0, "{intent_sub} rows exist at the Spirit's pid");
    }
}

#[test]
fn j0_init_creates_config() {
    let home = tempfile::TempDir::new().unwrap();
    let output = std::process::Command::new(maos_bin())
        .args(["init"])
        .env("MAOS_HOME", home.path())
        .env("XDG_DATA_HOME", home.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn maos init");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "maos init should exit 0; stderr:\n{stderr}"
    );
    assert!(
        home.path().join("config.toml").exists(),
        "maos init should create config.toml"
    );
    assert!(
        stdout.contains("initialized"),
        "maos init should confirm initialization; stdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// AC6 — the banner test repaired from its null control (proven red at HEAD
/// with the fake-workspace recipe — the ANY-of needles matched the
/// manifest-error line of a dead shell): the EXACT banner must appear, and
/// the screen must NOT contain the failure strings. With the manifest
/// embedded (D-16-2-A) the same recipe PASSES — both recorded in the Debug
/// Log.
#[test]
fn j0_shell_banner_via_pty() {
    let (_guard, world) = j0_world();
    init_world(&world);

    let pty = spawn_shell(&world);

    let scr = pty.screen();
    let screen = scr.text().to_string();
    assert!(
        screen.contains(J0_SHELL_BANNER),
        "the EXACT banner line must appear; screen:\n{screen}"
    );
    assert!(
        !screen.contains("cannot read") && !screen.contains("admission failed"),
        "a failed shell must not pass this test; screen:\n{screen}"
    );
    drop(pty);
}

/// AC1 — the halt resolves over the DOOR (`maosctl halt resolve`), the REPL
/// renders it unprompted, and the Spirit proceeds with the context.
#[test]
fn j0_halt_resolved_over_the_door() {
    let (_guard, world) = j0_world();
    init_world(&world);

    let pty = spawn_shell(&world);

    // Step 1 — hello-spirit is a loaded Spirit: Running with a pid ≠ 0.
    let spirit_pid;
    assert!(
        wait_until(30, "spirit inspect shows Running", || {
            let out = ctl(&world, &["spirit", "inspect", "hello-spirit"]);
            out.code == Some(0)
                && out.stdout.contains("lifecycle_state: Running")
                && out
                    .stdout
                    .lines()
                    .any(|l| l.starts_with("pid:") && !l.contains("pid: 0"))
        }),
        "hello-spirit must be Running at a non-zero pid"
    );
    {
        let out = ctl(&world, &["spirit", "inspect", "hello-spirit"]);
        let pid_line = out
            .stdout
            .lines()
            .find(|l| l.starts_with("pid:"))
            .expect("pid line");
        spirit_pid = pid_line
            .split_once("pid:")
            .unwrap()
            .1
            .trim()
            .parse::<u32>()
            .expect("pid parses");
        assert_ne!(spirit_pid, 0);
    }

    // Step 2 — the directive halts: `[HALT …]` and `halt <id>`.
    pty.send_line("@hello-spirit refactor src/main.rs to be more idiomatic");
    assert!(
        pty.wait_for_screen(
            &["[HALT task.acceptance_criterion.ambiguous]"],
            std::time::Duration::from_secs(15)
        ),
        "the halt line"
    );
    let scr = pty.screen();
    let screen_now = scr.text().to_string();
    let halt_id = halt_id_from(&screen_now);

    // Step 3 — `halt list` shows exactly one row whose halt_id is the id;
    // the door resolve exits 0.
    let list = ctl(&world, &["halt", "list", "--spirit", "hello-spirit"]);
    assert_eq!(
        list.code,
        Some(0),
        "halt list exit 0; stderr:\n{}",
        list.stderr
    );
    let rows: Vec<&str> = list.stdout.lines().collect();
    assert_eq!(rows.len(), 1, "exactly one halt row; got:\n{}", list.stdout);
    let parsed: serde_json::Value = serde_json::from_str(rows[0]).expect("halt list row parses");
    assert_eq!(parsed["halt_id"], halt_id, "the id matches the screen");
    assert_eq!(parsed["record"], "raised");

    let resolve = ctl(
        &world,
        &[
            "halt",
            "resolve",
            &halt_id,
            "--spirit",
            "hello-spirit",
            "--kind",
            "provided-context",
            "--text",
            "idiomatic = clippy-clean, no unwrap",
        ],
    );
    assert_eq!(
        resolve.code,
        Some(0),
        "door resolve exit 0; stdout:\n{}\nstderr:\n{}",
        resolve.stdout,
        resolve.stderr
    );

    // Step 4 — the REPL renders the resolution unprompted and the Spirit
    // proceeds (the cassette text — consumed by THIS turn).
    assert!(
        pty.wait_for_screen(
            &[&format!("halt {halt_id} resolved: provided_context")],
            std::time::Duration::from_secs(15)
        ),
        "the resolution renders in the REPL"
    );
    assert!(
        pty.wait_for_screen(
            &["Hello! I am the MAOS hello-spirit reference implementation."],
            std::time::Duration::from_secs(15)
        ),
        "the Spirit proceeded with the cassette text"
    );

    // Step 5 — EOF: exit 0, the unload rows, no drain timeout, and the
    // audit read (AC1 step 5's tail).
    pty.send_eof();
    let status = pty.wait().expect("shell exits");
    assert!(status.success(), "shell exits 0");
    let scr = pty.screen();
    let screen = scr.text().to_string();
    assert!(
        !screen.contains("drain timed out"),
        "the audit writer drained; screen:\n{screen}"
    );

    let tl_path = std::path::PathBuf::from(&world.env()["MAOS_HOME"])
        .join("audit")
        .join("transparency.sqlite");
    {
        let conn = rusqlite::Connection::open(&tl_path).expect("open TL");
        let unload: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transparency_log WHERE intent = 'lifecycle.unload'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unload, 1, "one lifecycle.unload row");
        let term_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transparency_log \
                 WHERE kind = 3 AND spirit_pid = ?1 AND intent = 'planned_unload'",
                rusqlite::params![spirit_pid as i64],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            term_rows, 2,
            "one termination_marker + one termination_no_pending receipt"
        );
    }

    let audit = ctl(
        &world,
        &[
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "plain",
        ],
    );
    assert_eq!(
        audit.code,
        Some(0),
        "audit query exit 0; stderr:\n{}",
        audit.stderr
    );
    assert!(audit.stdout.contains("epistemic.halt"));
    assert!(audit.stdout.contains("task.acceptance_criterion.ambiguous"));
    assert!(audit.stdout.contains("operator.halt-resolve."));
    assert!(audit.stdout.contains(":completed"));

    assert_j0_row_set(
        &world,
        &halt_id,
        spirit_pid,
        "idiomatic = clippy-clean, no unwrap",
    );
}

/// AC4 — the SAME scene with the clarification typed IN the REPL: identical
/// approval-log and completion rows (one mechanism, two surfaces).
#[test]
fn j0_halt_resolved_in_the_repl() {
    let (_guard, world) = j0_world();
    init_world(&world);

    let pty = spawn_shell(&world);

    let spirit_pid;
    assert!(wait_until(30, "spirit inspect shows Running", || {
        let out = ctl(&world, &["spirit", "inspect", "hello-spirit"]);
        out.code == Some(0) && out.stdout.contains("lifecycle_state: Running")
    }));
    {
        let out = ctl(&world, &["spirit", "inspect", "hello-spirit"]);
        let pid_line = out.stdout.lines().find(|l| l.starts_with("pid:")).unwrap();
        spirit_pid = pid_line
            .split_once("pid:")
            .unwrap()
            .1
            .trim()
            .parse()
            .unwrap();
        assert_ne!(spirit_pid, 0);
    }

    pty.send_line("@hello-spirit refactor src/main.rs to be more idiomatic");
    assert!(pty.wait_for_screen(
        &["[HALT task.acceptance_criterion.ambiguous]"],
        std::time::Duration::from_secs(15)
    ));
    let scr = pty.screen();
    let screen_now = scr.text().to_string();
    let halt_id = halt_id_from(&screen_now);

    // The clarification — typed, not sent over the door.
    pty.send_line("idiomatic = clippy-clean, no unwrap");
    assert!(
        pty.wait_for_screen(
            &[&format!("halt {halt_id} resolved: provided_context")],
            std::time::Duration::from_secs(15)
        ),
        "the REPL renders its own resolution"
    );
    assert!(
        pty.wait_for_screen(
            &["Hello! I am the MAOS hello-spirit reference implementation."],
            std::time::Duration::from_secs(15)
        ),
        "the Spirit proceeds with the context"
    );

    pty.send_eof();
    let status = pty.wait().expect("shell exits");
    assert!(status.success());
    let scr = pty.screen();
    let screen = scr.text().to_string();
    assert!(!screen.contains("drain timed out"), "screen:\n{screen}");

    // SAME rows from the REPL as from the door — the proof of one mechanism.
    assert_j0_row_set(
        &world,
        &halt_id,
        spirit_pid,
        "idiomatic = clippy-clean, no unwrap",
    );
}
