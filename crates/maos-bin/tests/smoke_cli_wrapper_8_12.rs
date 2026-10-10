#![forbid(unsafe_code)]

//! Story 8.12 · AC2/AC3/AC6 — live CliWrapper subprocess bridge under the daemon.
//!
//! Worker-subject scenarios use the signed T3 substrate on capable CI and emit
//! a named local skip when OCI is not provisioned. The separate admission/denial
//! control remains runnable without starting a Worker.
//!  1. `maos run spirits/worker/manifest.toml --once` admits the `[cli_wrapper]`
//!     Worker through the AC5 tier-grant + AC1 respawn gate + Story-7.4 journaled
//!     shape probe, spawns the REAL `worker-cli-fixture`, reports its real child
//!     PID, and durably journals the fixture's completion marker.
//!  2. The founder-loop topology runs through the production topology seam and
//!     drains cleanly; the real Worker launch still requires provisioned T3.
//!
//! Every production subprocess uses an isolated environment and data home.
//!
use std::process::Command;

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

fn add_t3_environment(command: &mut Command) {
    for key in [
        "MAOS_T3_RUNTIME",
        "MAOS_T3_IMAGE_LOCK_PATH",
        "MAOS_T3_IMAGE_TRUST_ANCHOR_PUB_HEX",
        "XDG_RUNTIME_DIR",
        "DBUS_SESSION_BUS_ADDRESS",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

fn production_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command.env_clear();
    add_t3_environment(&mut command);
    command.env("PATH", path_with_target_debug());
    command
}

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

fn path_with_target_debug() -> std::ffi::OsString {
    let profile = std::env::current_exe()
        .expect("test executable path")
        .parent()
        .and_then(std::path::Path::parent)
        .expect("test profile directory")
        .to_path_buf();
    let mut paths = vec![profile.clone(), profile.join("deps")];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("test PATH entries are valid")
}

struct IsolatedDataHome {
    path: std::path::PathBuf,
}

impl Drop for IsolatedDataHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn isolated_data_home(tag: &str) -> IsolatedDataHome {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-8-12-{tag}-{nanos}-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    IsolatedDataHome { path }
}

#[test]
fn maos_run_cli_wrapper_worker_spawns_real_subprocess() {
    if !t3_substrate::require_t3("maos run cli-wrapper Worker") {
        return;
    }
    let home = isolated_data_home("cw-run");
    let mut command = production_command();
    let output = command
        .args(["run", "spirits/worker/manifest.toml", "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(&home.path))
        .env("XDG_DATA_HOME", home.path.clone())
        // MAOS_HOME isolation avoids a host control.json endpoint collision.
        .env("MAOS_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");
    assert!(
        output.status.success(),
        "maos run worker --once must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let events: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    // (1) The CliWrapper Worker loaded with a host-granted tier.
    let loaded = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("cli_wrapper_loaded"))
        .expect("a cli_wrapper_loaded event");
    assert_eq!(
        loaded.get("granted_tier").and_then(|v| v.as_str()),
        Some("SandboxTier(3)"),
        "the worker's T3 tier request must be host-granted"
    );
    let child_pid = loaded
        .get("child_pid")
        .and_then(|v| v.as_u64())
        .expect("child_pid in the loaded event");
    assert_ne!(
        child_pid,
        std::process::id() as u64,
        "the worker must be a REAL subprocess (child_pid != this test pid)"
    );

    // (2) The bridge captured the fixture's stdout and exited cleanly.
    let exit = events
        .iter()
        .find(|e| e.get("event").and_then(|v| v.as_str()) == Some("cli_wrapper_exit"))
        .expect("a cli_wrapper_exit event");
    assert_eq!(exit.get("is_crash").and_then(|v| v.as_bool()), Some(false));
    let audit_db = home.path.join("audit").join("transparency.sqlite");
    let connection = rusqlite::Connection::open_with_flags(
        audit_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open Worker transparency log");
    let marker_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM transparency_log \
             WHERE payload_redacted LIKE '%worker: task complete%'",
            [],
            |row| row.get(0),
        )
        .expect("count fixture completion output rows");
    assert!(
        marker_rows >= 1,
        "the fixture's real completion output must be durably journaled"
    );
}

#[test]
fn founder_loop_journey_runs_with_real_worker_subprocess() {
    if !t3_substrate::require_t3("founder-loop delegated Worker") {
        return;
    }
    let home = isolated_data_home("founder");
    let mut command = production_command();
    let output = command
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(&home.path))
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_HOME", home.path.clone())
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    assert!(
        output.status.success(),
        "the founder-loop topology must exit 0; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    for spirit in ["orchestrator", "architect", "reviewer"] {
        assert!(
            stdout.contains(&format!("\"spirit_id\":\"{spirit}\"")),
            "the topology must load {spirit}; stdout:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("\"event\":\"drain\"") && stdout.contains("\"topology\":true"),
        "the founder-loop topology must terminate through the production drain seam; stdout:\n{stdout}"
    );
}

/// T5 — the CI/local split control, proven at the `maos run` level (not just the
/// unit gate): a HOST-GRANTED real agent CLI is still refused when
/// `MAOS_LIVE_AGENT` is unset — so CI (which never sets it) physically cannot
/// spawn a paid agent. Removing the gate call from the run path reds THIS test.
/// Linux-only: the T3 grant fails closed on non-Linux before the gate is reached.
#[cfg(target_os = "linux")]
#[test]
fn ci_local_split_refuses_a_granted_real_agent_without_the_live_flag() {
    let home = isolated_data_home("split");

    // A fake `codex` on PATH so binary resolution succeeds — the gate refuses
    // BEFORE the process is ever run, so the fake never has to do anything.
    let bindir = home.path.join("bin");
    std::fs::create_dir_all(&bindir).unwrap();
    let fake = bindir.join("codex");
    std::fs::write(&fake, "#!/bin/sh\nexit 0\n").unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    // Host-grant codex at T3 so we isolate the LIVE-flag gate (not the grant).
    let grants = home.path.join("host-grants.toml");
    std::fs::write(
        &grants,
        "[[grant]]\nattested_image = \"codex\"\nsigning_key_id = \"OpenAI\"\n\
         permitted_tier = \"T3\"\npermitted_egress_destinations = [\"api.openai.com\"]\n",
    )
    .unwrap();

    // A [cli_wrapper] worker manifest wrapping codex.
    let manifest = home.path.join("codex-worker.toml");
    std::fs::write(
        &manifest,
        "[cli_wrapper]\ncommand = \"codex\"\nargv_prefix = [\"exec\"]\n\
         output_shape_version = \"1.0.0\"\nrecovery_policy = \"respawn_fresh\"\n\
         [cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\n\
         control_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\
         [sandbox]\ntier = \"T3\"\n[author]\nname = \"OpenAI\"\n",
    )
    .unwrap();

    let mut command = production_command();
    let output = command
        .args(["run", manifest.to_str().unwrap(), "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(&home.path))
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_HOME", home.path.clone())
        .env("MAOS_HOST_GRANTS", &grants)
        .env("PATH", {
            let mut path = std::ffi::OsString::from(&bindir);
            path.push(":");
            path.push(path_with_target_debug());
            path
        })
        .env_remove("MAOS_LIVE_AGENT")
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a granted real agent CLI must be refused without MAOS_LIVE_AGENT; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("MAOS_LIVE_AGENT"),
        "the refusal must name the local opt-in (CI runs the fixture only, never a paid agent); stderr:\n{stderr}"
    );
}

/// §A6 review P10 (AC4.5 / F7) — the delegated goal reaches the frame VERBATIM,
/// including non-ASCII. Before 2e the goal was a hardcoded constant with no
/// override (`founder-loop: execute the delegated assignment from <host>`); the
/// dev's evidence for the override was a manual argv dump, never a committed
/// vector. The `delegation_routed` event is emitted at the frame itself, so the
/// exact `"goal":"<sentinel>"` string is the assertion — and the rehearsal
/// constant must NOT appear, proving override rather than append.
#[test]
fn delegated_goal_reaches_the_frame_verbatim_with_non_ascii() {
    if !t3_substrate::require_t3("founder-loop delegated goal Worker") {
        return;
    }
    let home = isolated_data_home("goal");
    let sentinel = "write a haiku to ./h.txt about a café — sentinel ✓ 2e-F7 §A6-P10";
    let mut command = production_command();
    let output = command
        .args(["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .env("HOME", t3_substrate::isolated_runtime_home(&home.path))
        .env("XDG_DATA_HOME", home.path.clone())
        .env("MAOS_HOME", home.path.clone())
        .env("MAOS_DELEGATED_GOAL", sentinel)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos-bin");
    assert!(
        output.status.success(),
        "the founder-loop topology must exit 0 with a goal set; stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("\"goal\":\"{sentinel}\"")),
        "the frame must carry the operator's goal byte-for-byte, non-ASCII intact:\n{stdout}"
    );
    assert!(
        !stdout.contains("founder-loop: execute the delegated assignment"),
        "the rehearsal constant must be OVERRIDDEN, not appended:\n{stdout}"
    );
}
