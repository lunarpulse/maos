#![forbid(unsafe_code)]

//! Story 19-3 (AC2.2–2.4, AC3.2, AC4.2) — the per-Worker `/workspace`, its
//! bounded metadata walk and the effect gate.
//!
//! * The walk and gate vectors are pure: they run everywhere, on a temp dir.
//! * The end-to-end vectors run the REAL `maos run` binary through a T3 Worker
//!   (`require_t3`: a named local skip, never a green skip on a declared-capable
//!   runner) with a fake `claude`/`codex` on a prepended `PATH` — the
//!   `worker_completion_2a.rs` planted-refusal pattern. Vector (a) is the
//!   committed proven-red for `RELEASE-HOLDS.md` row 16: a real-CLI-shaped
//!   "I have written the file" over an untouched workspace. **Plant:** delete
//!   the `gate_workspace_effect` call in `run_cli_wrapper_manifest` → (a) exits 0.
//!
//! Sequencing and distinct Worker pids over the epic run are proven by
//! `epic_dispatch_19_1.rs` (D-19-3-10 A); this file adds the workspace clause.

use std::path::{Path, PathBuf};

use maos_bin::worker_cli::{
    gate_workspace_effect, ClaudeCli, CodexCli, FixtureCli, ProbeCli, WorkerCli, WorkerCompletion,
    WorkerNonCompletion,
};
use maos_bin::worker_workspace::{
    diff, effect_json, snapshot, WalkError, WorkspaceEffect, WALK_ENTRY_CAP,
};

#[cfg(target_os = "linux")]
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

/// A fresh, self-removing directory. Files a T3 Worker wrote are owned by its
/// mapped subuid (D-19-3-01 A), so a retained workspace is removed through
/// `podman unshare` first — the documented operator cleanup.
struct Tmp(PathBuf);

impl Drop for Tmp {
    fn drop(&mut self) {
        let workspaces = self.0.join("workspaces");
        if workspaces.exists() && std::env::var("MAOS_T3_RUNTIME").as_deref() == Ok("podman") {
            let _ = std::process::Command::new("podman")
                .args(["unshare", "rm", "-rf"])
                .arg(&workspaces)
                .status();
        }
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tmp(tag: &str) -> Tmp {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("maos-19-3-{tag}-{nanos}-{}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create temp dir");
    Tmp(path)
}

fn write(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write test file");
}

fn completed() -> WorkerCompletion {
    WorkerCompletion::Completed {
        final_message: "I have written the file".to_string(),
    }
}

// ── the walk (D-19-3-03 A) ─────────────────────────────────────────────────

#[test]
fn an_unchanged_tree_diffs_empty() {
    let t = tmp("unchanged");
    let empty = snapshot(&t.0, WALK_ENTRY_CAP).expect("walk an empty workspace");
    assert!(empty.is_empty(), "a fresh workspace snapshots empty");
    std::fs::create_dir(t.0.join("src")).expect("mkdir");
    write(&t.0.join("src").join("main.rs"), "fn main() {}\n");
    write(&t.0.join("README"), "hi\n");
    let before = snapshot(&t.0, WALK_ENTRY_CAP).expect("walk before");
    let after = snapshot(&t.0, WALK_ENTRY_CAP).expect("walk after");
    assert_eq!(before.len(), 3, "two files and one directory: {before:?}");
    let effect = diff(&before, &after);
    assert!(
        effect.is_empty(),
        "an untouched tree is no effect: {effect:?}"
    );
    assert_eq!(effect, WorkspaceEffect::default());
}

#[test]
fn created_modified_and_removed_entries_are_counted() {
    let t = tmp("counted");
    write(&t.0.join("grows"), "1");
    write(&t.0.join("touched"), "same");
    write(&t.0.join("goes"), "x");
    write(&t.0.join("becomes-dir"), "x");
    let before = snapshot(&t.0, WALK_ENTRY_CAP).expect("walk before");

    write(&t.0.join("grows"), "22");
    std::fs::File::options()
        .write(true)
        .open(t.0.join("touched"))
        .and_then(|f| f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(7)))
        .expect("set mtime");
    std::fs::remove_file(t.0.join("goes")).expect("rm");
    std::fs::remove_file(t.0.join("becomes-dir")).expect("rm");
    std::fs::create_dir(t.0.join("becomes-dir")).expect("mkdir");
    std::fs::create_dir(t.0.join("new")).expect("mkdir");
    write(&t.0.join("new").join("h.txt"), "ok\n");
    let after = snapshot(&t.0, WALK_ENTRY_CAP).expect("walk after");

    assert_eq!(
        diff(&before, &after),
        WorkspaceEffect {
            created: 2,
            modified: 3,
            removed: 1,
        },
        "created: new/ + new/h.txt; modified: size, mtime-only and file→dir; removed: goes"
    );
}

#[test]
fn the_walk_refuses_over_the_cap_and_never_truncates() {
    let t = tmp("cap");
    std::fs::create_dir(t.0.join("d")).expect("mkdir");
    write(&t.0.join("a"), "");
    write(&t.0.join("d").join("b"), "");
    let at_cap = snapshot(&t.0, 3).expect("exactly the cap is admitted");
    assert_eq!(at_cap.len(), 3);
    // A nested entry past the cap refuses: the cap counts the whole tree.
    write(&t.0.join("d").join("c"), "");
    let refused = snapshot(&t.0, 3).expect_err("over the cap must refuse");
    assert!(
        matches!(refused, WalkError::OverCap { cap: 3 }),
        "typed refusal: {refused:?}"
    );
    let text = refused.to_string();
    assert!(
        text.contains("refused") && text.contains("never truncated"),
        "the refusal is legible: {text}"
    );
    assert_eq!(
        effect_json(&Err(refused)),
        serde_json::json!({ "refused": text })
    );
}

#[cfg(unix)]
#[test]
fn symlinks_are_recorded_and_never_followed() {
    let t = tmp("symlink");
    let outside = t.0.join("outside");
    let ws = t.0.join("ws");
    std::fs::create_dir_all(&outside).expect("mkdir outside");
    std::fs::create_dir_all(&ws).expect("mkdir ws");
    for i in 0..5 {
        write(&outside.join(format!("f{i}")), "outside");
    }
    std::os::unix::fs::symlink(&outside, ws.join("escape")).expect("dir symlink");
    std::os::unix::fs::symlink("/nonexistent/maos-19-3", ws.join("dangling")).expect("symlink");
    // Cap 2: following `escape` would walk five more entries and refuse.
    let snap = snapshot(&ws, 2).expect("a symlink is one entry, never a walk");
    assert_eq!(snap.len(), 2, "{snap:?}");
    for name in ["escape", "dangling"] {
        assert_eq!(
            snap.get(Path::new(name))
                .map(|meta| meta.file_type.is_symlink()),
            Some(true),
            "{name} is recorded as a symlink: {snap:?}"
        );
    }
    assert_eq!(
        diff(&Default::default(), &snap).created,
        2,
        "a symlink the Worker made is an effect"
    );
}

#[cfg(unix)]
#[test]
fn an_unlistable_directory_refuses_the_walk() {
    use std::os::unix::fs::PermissionsExt as _;
    let t = tmp("unlistable");
    let locked = t.0.join("locked");
    std::fs::create_dir(&locked).expect("mkdir");
    write(&locked.join("inside"), "x");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");
    let listable = std::fs::read_dir(&locked).is_ok();
    let result = snapshot(&t.0, WALK_ENTRY_CAP);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).expect("chmod back");
    if listable {
        eprintln!("SKIP: the walk refusal on an unlistable dir: this user bypasses mode 000");
        return;
    }
    match result {
        Err(WalkError::Io { path, .. }) => assert_eq!(path, locked),
        other => panic!("an unlistable entry must refuse the walk, never skip it: {other:?}"),
    }
}

// ── the gate (D-19-3-04 A, D-19-3-05 A, D-19-3-06 A) ────────────────────────

#[test]
fn only_the_real_agent_clis_require_a_workspace_effect() {
    assert!(ClaudeCli.requires_workspace_effect());
    assert!(CodexCli.requires_workspace_effect());
    assert!(!FixtureCli.requires_workspace_effect());
    assert!(!ProbeCli.requires_workspace_effect());
}

#[test]
fn a_real_cli_completion_over_an_empty_or_refused_diff_is_no_effect_evidence() {
    let one = WorkspaceEffect {
        created: 1,
        modified: 0,
        removed: 0,
    };
    for cli in [&ClaudeCli as &dyn WorkerCli, &CodexCli] {
        for effect in [
            Ok(WorkspaceEffect::default()),
            Err(WalkError::OverCap { cap: 1 }),
        ] {
            assert_eq!(
                gate_workspace_effect(cli, completed(), &effect),
                WorkerCompletion::NotCompleted(WorkerNonCompletion::NoEffectEvidence),
                "{} Completed over {effect:?}",
                cli.name()
            );
        }
        assert_eq!(
            gate_workspace_effect(cli, completed(), &Ok(one)),
            completed(),
            "{} with an effect completes",
            cli.name()
        );
        // Conjunctive: the diff never upgrades the adapter's own refusal.
        for refusal in [
            WorkerNonCompletion::PermissionDenied,
            WorkerNonCompletion::NoEffectEvidence,
            WorkerNonCompletion::TurnFailed,
        ] {
            let verdict = WorkerCompletion::NotCompleted(refusal);
            assert_eq!(
                gate_workspace_effect(cli, verdict.clone(), &Ok(one)),
                verdict,
                "{} keeps its oracle's refusal",
                cli.name()
            );
        }
    }
    assert_eq!(
        gate_workspace_effect(&ClaudeCli, completed(), &Ok(WorkspaceEffect::default())).label(),
        "not_completed:no_effect_evidence"
    );
}

#[test]
fn the_fixtures_are_reported_not_gated() {
    for cli in [&FixtureCli as &dyn WorkerCli, &ProbeCli] {
        assert_eq!(
            gate_workspace_effect(cli, completed(), &Ok(WorkspaceEffect::default())),
            completed(),
            "{} writes nothing by contract",
            cli.name()
        );
    }
    assert_eq!(
        effect_json(&Ok(WorkspaceEffect::default())),
        serde_json::json!({ "created": 0, "modified": 0, "removed": 0 })
    );
}

// ── end to end through a T3 Worker (AC2.4, AC3.2, AC4.2) ───────────────────

#[cfg(target_os = "linux")]
mod t3_runs {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    use super::{t3_substrate, tmp};

    const CLAUDE_ARGV: &str =
        r#"["--print", "--output-format", "json", "--bare", "--permission-mode", "acceptEdits"]"#;
    const CODEX_ARGV: &str = r#"["exec", "--sandbox", "workspace-write", "--json"]"#;
    const CLAUDE_WROTE_IT: &str = r#"echo '{"type":"result","subtype":"success","is_error":false,"num_turns":1,"result":"I have written the file.","permission_denials":[],"total_cost_usd":0.01}'"#;

    fn workspace_root() -> &'static str {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
    }

    fn profile_dir() -> PathBuf {
        std::env::current_exe()
            .expect("test executable path")
            .parent()
            .and_then(Path::parent)
            .expect("test profile directory")
            .to_path_buf()
    }

    /// A fake worker binary named `claude`/`codex` on a prepended `PATH`,
    /// selected by `select_worker_cli`'s basename dispatch. Under T3 its cwd
    /// is `/workspace` (K2), so a relative write lands in the workspace.
    fn plant_fake_cli(root: &Path, name: &str, body: &str) {
        let bindir = root.join("bin");
        std::fs::create_dir_all(&bindir).expect("mkdir bin");
        let path = bindir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\nexit 0\n")).expect("write fake cli");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake cli");
    }

    fn write_grant(root: &Path, image: &str, author: &str) -> PathBuf {
        let grants = root.join(format!("host-grants-{image}.toml"));
        std::fs::write(
            &grants,
            format!(
                "[[grant]]\nattested_image = \"{image}\"\nsigning_key_id = \"{author}\"\n\
                 permitted_tier = \"T3\"\npermitted_egress_destinations = []\n"
            ),
        )
        .expect("write grants");
        grants
    }

    /// A topology whose frame-borne (`host`) member is the `[cli_wrapper]`
    /// Worker, so the verdict is enforced (`worker_completion_2a.rs`).
    fn write_topology(root: &Path, command: &str, argv_prefix: &str, author: &str) -> PathBuf {
        let worker = root.join(format!("{command}-worker.toml"));
        std::fs::write(
            &worker,
            format!(
                "[cli_wrapper]\ncommand = \"{command}\"\nargv_prefix = {argv_prefix}\n\
                 output_shape_version = \"1.0.0\"\nrecovery_policy = \"respawn_fresh\"\n\
                 [cli_wrapper.posture]\nstdio_shape = \"ndjson_over_stdio\"\n\
                 control_channel = \"signals\"\nshutdown_signal = \"SIGTERM\"\n\
                 [sandbox]\ntier = \"T3\"\n[author]\nname = \"{author}\"\n"
            ),
        )
        .expect("write worker manifest");
        let topology = root.join("topology.toml");
        std::fs::write(
            &topology,
            format!(
                "[topology]\nname = \"workspace-19-3\"\n\n\
                 [[topology.spirits]]\nmanifest = \"{}/spirits/orchestrator/manifest.toml\"\n\n\
                 [[topology.spirits]]\nmanifest = \"{}\"\nhost = \"developer-remote-host\"\n",
                workspace_root(),
                worker.display()
            ),
        )
        .expect("write topology");
        topology
    }

    /// `maos run <args> --once` on a fresh data home, clean environment.
    fn run_maos(root: &Path, args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Output {
        let mut paths = vec![root.join("bin"), profile_dir(), profile_dir().join("deps")];
        if let Some(existing) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&existing));
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
        command.env_clear();
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
        command
            .arg("run")
            .args(args)
            .arg("--once")
            .env("HOME", t3_substrate::isolated_runtime_home(root))
            .env("XDG_DATA_HOME", root)
            .env("MAOS_HOME", root)
            .env("PATH", std::env::join_paths(paths).expect("valid PATH"));
        for (key, value) in env {
            command.env(key, value);
        }
        command
            .current_dir(workspace_root())
            .output()
            .expect("execute maos")
    }

    fn run_live(root: &Path, image: &str, author: &str, topology: &Path) -> Output {
        let grants = write_grant(root, image, author);
        run_maos(
            root,
            &[topology.to_str().expect("utf-8 topology path")],
            &[
                ("MAOS_HOST_GRANTS", grants.as_os_str()),
                ("MAOS_LIVE_AGENT", std::ffi::OsStr::new("1")),
            ],
        )
    }

    fn events(output: &Output, name: &str) -> Vec<serde_json::Value> {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|event| event["event"].as_str() == Some(name))
            .collect()
    }

    /// The one Worker's workspace: named by `cli_wrapper_loaded`, the same path
    /// on `worker_completion`, an existing directory under `<home>/workspaces`.
    fn the_workspace(root: &Path, output: &Output) -> (PathBuf, serde_json::Value) {
        let loaded = events(output, "cli_wrapper_loaded");
        let completion = events(output, "worker_completion");
        assert_eq!(
            (loaded.len(), completion.len()),
            (1, 1),
            "one Worker ran.\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let path = loaded[0]["workspace"]
            .as_str()
            .expect("cli_wrapper_loaded.workspace");
        assert_eq!(completion[0]["workspace"].as_str(), Some(path));
        let workspace = PathBuf::from(path);
        assert_eq!(workspace.parent(), Some(root.join("workspaces").as_path()));
        assert!(workspace.is_dir(), "the workspace is retained after exit");
        (workspace, completion[0].clone())
    }

    /// The `worker.completion-verdict` row's payload at the Worker's pid.
    fn verdict_row(root: &Path, output: &Output) -> serde_json::Value {
        let pid = events(output, "cli_wrapper_loaded")[0]["spirit_pid"]
            .as_i64()
            .expect("spirit_pid");
        let connection = rusqlite::Connection::open_with_flags(
            root.join("audit").join("transparency.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .expect("open transparency log");
        let mut statement = connection
            .prepare("SELECT spirit_pid, payload_redacted FROM transparency_log")
            .expect("prepare TL query");
        let rows: Vec<(i64, Vec<u8>)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query TL")
            .collect::<Result<_, _>>()
            .expect("read TL");
        let verdicts: Vec<serde_json::Value> = rows
            .into_iter()
            .filter(|(row_pid, _)| *row_pid == pid)
            .filter_map(|(_, payload)| serde_json::from_slice::<serde_json::Value>(&payload).ok())
            .filter(|payload| payload["event"] == "worker_completion_verdict")
            .collect();
        assert_eq!(verdicts.len(), 1, "one verdict row at the Worker's pid");
        verdicts[0].clone()
    }

    /// AC2.4(a) — THE row-16 vector. A clean claude result object claiming the
    /// write, over a workspace the Worker never touched. Before 19-3 this was
    /// `completed: true` and signable.
    #[test]
    fn claude_claiming_a_write_over_an_untouched_workspace_fails_the_run() {
        if !t3_substrate::require_t3("workspace no-effect claude Worker") {
            return;
        }
        let t = tmp("claude-noeffect");
        plant_fake_cli(&t.0, "claude", CLAUDE_WROTE_IT);
        let topology = write_topology(&t.0, "claude", CLAUDE_ARGV, "Anthropic");
        let out = run_live(&t.0, "claude", "Anthropic", &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "a claimed write over an empty diff must FAIL the run.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("did not complete") && stderr.contains("no_effect_evidence"),
            "stderr names the typed refusal.\nstderr:\n{stderr}"
        );
        let (_, completion) = the_workspace(&t.0, &out);
        assert_eq!(completion["completed"], false);
        assert_eq!(completion["completion"], "not_completed:no_effect_evidence");
        let none = serde_json::json!({ "created": 0, "modified": 0, "removed": 0 });
        assert_eq!(completion["workspace_effect"], none);
        let verdict = verdict_row(&t.0, &out);
        assert_eq!(verdict["verdict"], "not_completed:no_effect_evidence");
        assert_eq!(verdict["workspace_effect"], none);
        assert_eq!(verdict["workspace"], completion["workspace"]);
    }

    /// AC2.4(b) — the same object over a real write completes, and the file is
    /// in the reported workspace.
    #[test]
    fn claude_that_writes_into_its_workspace_completes() {
        if !t3_substrate::require_t3("workspace effect claude Worker") {
            return;
        }
        let t = tmp("claude-effect");
        plant_fake_cli(
            &t.0,
            "claude",
            &format!("echo ok > h.txt\n{CLAUDE_WROTE_IT}"),
        );
        let topology = write_topology(&t.0, "claude", CLAUDE_ARGV, "Anthropic");
        let out = run_live(&t.0, "claude", "Anthropic", &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "a claude run that wrote its file completes.\nstderr:\n{stderr}"
        );
        let (workspace, completion) = the_workspace(&t.0, &out);
        assert_eq!(completion["completed"], true);
        let one = serde_json::json!({ "created": 1, "modified": 0, "removed": 0 });
        assert_eq!(completion["workspace_effect"], one);
        assert_eq!(verdict_row(&t.0, &out)["workspace_effect"], one);
        assert!(
            workspace.join("h.txt").exists(),
            "h.txt is in {workspace:?}"
        );
    }

    /// AC2.4(c) — D-19-3-04 A: codex's native `file_change` is necessary, not
    /// sufficient. An applied change that left `/workspace` untouched refuses.
    #[test]
    fn codex_file_change_without_a_workspace_diff_fails_the_run() {
        if !t3_substrate::require_t3("workspace no-effect codex Worker") {
            return;
        }
        let t = tmp("codex-noeffect");
        plant_fake_cli(
            &t.0,
            "codex",
            r#"echo '{"type":"thread.started","thread_id":"t1"}'
echo '{"type":"item.completed","item":{"id":"i1","type":"file_change","changes":[{"path":"h.txt","kind":"add"}],"status":"completed"}}'
echo '{"type":"item.completed","item":{"id":"i2","type":"agent_message","text":"Created h.txt."}}'
echo '{"type":"turn.completed","usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1,"reasoning_output_tokens":0}}'"#,
        );
        let topology = write_topology(&t.0, "codex", CODEX_ARGV, "OpenAI");
        let out = run_live(&t.0, "codex", "OpenAI", &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            !out.status.success(),
            "a file_change with an empty diff must FAIL the run.\nstderr:\n{stderr}"
        );
        assert!(
            stderr.contains("did not complete") && stderr.contains("no_effect_evidence"),
            "stderr names the typed refusal.\nstderr:\n{stderr}"
        );
        let (_, completion) = the_workspace(&t.0, &out);
        assert_eq!(completion["completed"], false);
    }

    /// AC4.2 — D-19-3-07 A: no real agent CLI runs on the host during
    /// admission. The fake `claude`'s `--version` branch writes a host marker
    /// outside any workspace; after a run that DID spawn the Worker, the marker
    /// is absent. **Plant:** re-insert the host liveness probe → marker present.
    #[test]
    fn admission_never_executes_the_real_cli_on_the_host() {
        if !t3_substrate::require_t3("host-probe marker claude Worker") {
            return;
        }
        let t = tmp("host-marker");
        let marker = t.0.join("host-liveness-marker");
        plant_fake_cli(
            &t.0,
            "claude",
            &format!(
                "if [ \"$1\" = \"--version\" ]; then echo ran > '{}'; echo 'claude 0.0.0'; exit 0; fi\n\
                 echo ok > h.txt\n{CLAUDE_WROTE_IT}",
                marker.display()
            ),
        );
        let topology = write_topology(&t.0, "claude", CLAUDE_ARGV, "Anthropic");
        let out = run_live(&t.0, "claude", "Anthropic", &topology);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(
            events(&out, "cli_wrapper_loaded").len(),
            1,
            "the Worker was admitted and spawned, so admission ran.\nstderr:\n{stderr}"
        );
        assert!(
            !marker.exists(),
            "the fake claude ran on the host during admission (marker {marker:?} present)"
        );
    }

    /// AC3.2 — over one epic run, three distinct, existing workspaces, each
    /// named before its Worker's output and none shared; the fixture's effect
    /// is reported and not gated (D-19-3-04 A).
    #[test]
    fn each_epic_worker_gets_its_own_workspace() {
        if !t3_substrate::require_t3("epic three-workspace dispatch") {
            return;
        }
        let t = tmp("epic");
        let out = run_maos(
            &t.0,
            &[
                "spirits/topologies/j1-founder-loop-epic.toml",
                "--epic",
                "tests/fixtures/j1/epic.md",
            ],
            &[],
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "the epic run exits 0.\nstderr:\n{stderr}"
        );
        let loaded: Vec<String> = events(&out, "cli_wrapper_loaded")
            .iter()
            .map(|event| event["workspace"].as_str().expect("workspace").to_string())
            .collect();
        let completions = events(&out, "worker_completion");
        assert_eq!((loaded.len(), completions.len()), (3, 3), "three Workers");
        let distinct: std::collections::BTreeSet<&String> = loaded.iter().collect();
        assert_eq!(distinct.len(), 3, "no workspace is shared: {loaded:?}");
        let none = serde_json::json!({ "created": 0, "modified": 0, "removed": 0 });
        for (path, completion) in loaded.iter().zip(&completions) {
            let workspace = Path::new(path);
            assert_eq!(workspace.parent(), Some(t.0.join("workspaces").as_path()));
            assert!(workspace.is_dir(), "{path} exists after the run");
            assert_eq!(completion["workspace"].as_str(), Some(path.as_str()));
            assert_eq!(completion["completed"], true);
            assert_eq!(completion["workspace_effect"], none, "reported, not gated");
        }
    }
}
