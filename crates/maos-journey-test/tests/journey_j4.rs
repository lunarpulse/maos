#![forbid(unsafe_code)]

//! J4 — Mira/Nash bilateral pair journey (Grade A: real `maos run` topology).
//!
//! The topology path loads Mira and Nash through the production composition root.
//! The oracle is typed topology/drain output; the older smoke success marker is
//! no longer accepted as the sole signal.
//!
//! ## Anti-fake control
//!
//! The `ConsentRupture` oracle (`j4_earned_consent_rupture_typed_oracle`) runs the deny
//! topology (`j4-mira-nash-deny.toml`: Nash's accept list admits nothing) as a serving root,
//! asks Mira once through the operator door, and reads the kind-22 row the production deny
//! path (`A2ARouterCore::handle_intake`, through the rupture sink the router installs) wrote
//! before it refused. It must go RED if that emission is removed. It imports the production
//! `ConsentRupturePayload` + `RuptureReason::IntentAllowlistMismatch` types from
//! `maos_domain::frame` — never local fakes.

#[path = "support/binaries.rs"]
mod binaries;

use maos_journey_test::AuditDb;
use std::collections::BTreeSet;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn maos_bin() -> String {
    binaries::resolve("maos", option_env!("CARGO_BIN_EXE_maos"))
}

/// Parse stdout lines as JSON, collecting successfully parsed values.
fn parse_json_lines(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

#[test]
fn j4_mira_nash_topology_run_once() {
    let home = tempfile::TempDir::new().unwrap();
    let output = Command::new(maos_bin())
        .args(["run", "spirits/topologies/j4-mira-nash.toml", "--once"])
        .env("XDG_DATA_HOME", home.path())
        .env("MAOS_HOME", home.path())
        // 15-6 §A6 review P1: cassette-free child; never inherit a
        // job-level MAOS_INFERENCE_MODE (nightly rerecord leg).
        .env_remove("MAOS_INFERENCE_MODE")
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn mira-nash topology");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Story 9.6 — topology Mira now has its EpistemicScalarPort wired via
    // ButlerOrchestratorAdapter, so the J4 topology loads and drains successfully.
    assert!(
        output.status.success(),
        "mira-nash topology should exit 0; stderr:\n{stderr}\nstdout:\n{stdout}"
    );

    // Structured JSON assertions — no string scraping.
    let events = parse_json_lines(&stdout);
    assert!(
        !events.is_empty(),
        "topology stdout must contain JSON events; raw stdout:\n{stdout}"
    );

    // Collect spirit_ids from spirit_loaded events.
    let loaded_ids: BTreeSet<&str> = events
        .iter()
        .filter(|e| e.get("event").and_then(|v| v.as_str()) == Some("spirit_loaded"))
        .filter_map(|e| e.get("spirit_id").and_then(|v| v.as_str()))
        .collect();
    let expected: BTreeSet<&str> = ["mira", "nash"].iter().copied().collect();
    assert_eq!(
        loaded_ids, expected,
        "topology must load exactly {{mira, nash}}; got {loaded_ids:?}"
    );

    // Assert a drain event with topology: true exists.
    let has_topology_drain = events.iter().any(|e| {
        e.get("event").and_then(|v| v.as_str()) == Some("drain")
            && e.get("topology").and_then(|v| v.as_bool()) == Some(true)
    });
    assert!(
        has_topology_drain,
        "J4 topology run should terminate through the drain-complete seam; events:\n{events:?}"
    );

    // The old smoke success-marker oracle must not appear.
    assert!(
        !stderr.contains("live TCP + real HTTP mobile-push J4 journey complete"),
        "success-marker-only smoke oracle must not be used by the Grade-A path; stderr:\n{stderr}"
    );
}

/// ConsentRupture oracle — uses production types from `maos_domain::frame`.
///
/// Serves the deny topology, hands Mira one incident and reads the transparency log: exactly one
/// `ConsentRupture` row (kind 22) whose payload is a `ConsentRupturePayload` rejecting the
/// recipient with `IntentAllowlistMismatch`, and Mira's answer says the advisory was denied.
#[test]
fn j4_earned_consent_rupture_typed_oracle() {
    // Production type imports — compile-error if the types are removed/renamed.
    use maos_domain::frame::{ConsentRupturePayload, FramePayload, RuptureReason};

    // ConsentRupture = kind discriminator 22 in the transparency log.
    const CONSENT_RUPTURE_KIND: i64 = 22;
    const INCIDENT: &str =
        r#"{"subject":"payments-api","metric":"error_rate","observed":0.42,"baseline":0.02}"#;

    /// The serving root, killed on every exit path.
    struct Serving(std::process::Child);
    impl Drop for Serving {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let audit = AuditDb::temp();
    let tl_path = audit.transparency_log_path();
    let command = |program: &str| {
        let mut command = Command::new(program);
        command
            .env("HOME", audit.path())
            .env("XDG_DATA_HOME", audit.path().join("xdg"))
            .env("MAOS_HOME", audit.path())
            .env("MAOS_NOTIFY_DISABLE", "1")
            .env_remove("MAOS_INFERENCE_MODE")
            .current_dir(workspace_root());
        command
    };
    let maosctl = binaries::resolve("maosctl", None);
    let init = command(&maos_bin())
        .arg("init")
        .output()
        .expect("maos init");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );

    let stderr_path = audit.path().join("serve.stderr");
    let mut serving = Serving(
        command(&maos_bin())
            .args(["run", "spirits/topologies/j4-mira-nash-deny.toml"])
            .stdout(std::process::Stdio::null())
            .stderr(std::fs::File::create(&stderr_path).unwrap())
            .spawn()
            .expect("failed to spawn the deny topology"),
    );
    let stderr = || std::fs::read_to_string(&stderr_path).unwrap_or_default();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(90);
    loop {
        let agents = command(&maosctl)
            .arg("agents")
            .output()
            .expect("maosctl agents");
        let listing = String::from_utf8_lossy(&agents.stdout);
        if agents.status.success()
            && ["mira", "nash"].iter().all(|id| {
                listing
                    .lines()
                    .any(|line| line.starts_with(&format!("{id}\t")) && line.contains("Running"))
            })
        {
            break;
        }
        if let Some(status) = serving.0.try_wait().unwrap() {
            panic!("the deny topology exited early: {status}\n{}", stderr());
        }
        assert!(
            std::time::Instant::now() < until,
            "the cast never came up\n{}",
            stderr()
        );
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let ask = command(&maosctl)
        .args(["ask", "mira", INCIDENT, "--wait"])
        .output()
        .expect("maosctl ask");
    let shown: serde_json::Value =
        serde_json::from_slice(&ask.stdout).expect("ask --wait prints the task");
    assert_eq!(ask.status.code(), Some(0), "{shown}\n{}", stderr());
    let answer: serde_json::Value =
        serde_json::from_str(shown["result"].as_str().expect("a result")).expect("Mira's JSON");
    assert!(
        answer["advisory"]
            .as_str()
            .is_some_and(|advisory| advisory.starts_with("denied:")),
        "Mira reports the denied advisory: {answer}"
    );

    // Query the transparency log for ConsentRupture rows (kind=22).
    assert!(
        tl_path.exists(),
        "transparency log must be created by the topology run at {tl_path:?}"
    );
    let conn =
        rusqlite::Connection::open_with_flags(&tl_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open transparency log for ConsentRupture query");
    let rows: Vec<Vec<u8>> = conn
        .prepare("SELECT payload_redacted FROM transparency_log WHERE kind = ?1")
        .unwrap()
        .query_map([CONSENT_RUPTURE_KIND], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "exactly one ConsentRupture row (kind={CONSENT_RUPTURE_KIND}) from the production deny \
         path A2ARouterCore::handle_intake; stderr:\n{}",
        stderr()
    );
    let payload: FramePayload =
        serde_json::from_slice(&rows[0]).expect("the row is a typed FramePayload");
    let FramePayload::ConsentRupture(rupture) = payload else {
        panic!("kind-22 row is not a ConsentRupture payload");
    };
    let rupture: ConsentRupturePayload = rupture;
    assert_eq!(rupture.rejected.len(), 1);
    assert_eq!(
        rupture.rejected[0].reason,
        RuptureReason::IntentAllowlistMismatch
    );
    assert_eq!(rupture.rejected[0].address.spirit_id.as_str(), "nash");
}
