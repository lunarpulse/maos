#![cfg(all(feature = "network", target_os = "linux"))]
#![forbid(unsafe_code)]
//! Real door cascade. T0: both admission paths currently default SCB schedules;
//! AC6.3 therefore uses live AC2 lineage, not a fabricated schedule.fire claim.
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;
#[path = "../../../tests/harness/forget_18_6.rs"]
mod h;
#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3;
use maos_domain::ports::DistillationPort as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Live {
    child: std::process::Child,
    events: Arc<Mutex<Vec<serde_json::Value>>>,
    stderr: std::fs::File,
}
impl Drop for Live {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn exercise(worker: bool) {
    if worker
        && !t3::require_t3(
            "20-11 AC6 live Worker uninstall (AC2 fallback; schedule admission absent)",
        )
    {
        return;
    }
    let dir = tempfile::tempdir().expect("short isolated home");
    let home = dir.path();
    let binary = Path::new(env!("CARGO_BIN_EXE_maos"));
    let ctl = binary.with_file_name("maosctl");
    let mut init = h::command(binary, home);
    init.arg("init");
    h::checked(init);
    drop(h::Stores::open(home, 2011));
    if !worker {
        let registry = home.join(".local/share/maos/registry");
        let generation = registry.join("installed/refused-installed/refused");
        std::fs::create_dir_all(&generation).expect("refused generation");
        std::fs::write(
            generation.join("manifest.toml"),
            "[[schedule]]\nid='refused'\ncadence_secs=1\n",
        )
        .expect("refused manifest");
        // Missing signed_package.json forces the REAL boot verifier refusal.
        std::fs::write(
            registry.join("installed-index.json"),
            br#"{"refused-installed":"refused-installed/refused"}"#,
        )
        .expect("installed index");
    }
    let manifest = home.join("scheduled.toml");
    let base = if worker {
        assert!(fixtures::profile_binary("worker-cli-fixture").is_file());
        "[cli_wrapper]\ncommand='worker-cli-fixture'\nargv_prefix=['--maos-worker','--maos-fixture-mode=hang']\noutput_shape_version='1.0.0'\nskill_bundle=['maos-bridge']\nrecovery_policy='respawn_fresh'\n[cli_wrapper.posture]\nstdio_shape='ndjson_over_stdio'\ncontrol_channel='signals'\nshutdown_signal='SIGTERM'\n[sandbox]\ntier='T3'\n[author]\nname='MAOS Project'\n".to_owned()
    } else {
        include_str!("../../../spirits/researcher/manifest.toml").to_owned()
    };
    std::fs::write(&manifest,format!("{base}\n[lifecycle]\nenabled_hooks=['on_start','on_schedule','on_unload']\n[[schedule]]\nid='daily'\ncadence_secs=1\nrate_limit_per_hour=3600\nprincipal_revocability=false\n")).expect("schedule manifest");
    let key = home.join("audit.key");
    std::fs::write(&key, hex::encode([0x61; 32])).expect("key");
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).expect("secure key");
    let mut run = h::command(binary, home);
    run.args(["run", manifest.to_str().expect("manifest path")])
        .env("MAOS_SCHEDULE_FAST", "1")
        .env("MAOS_AUDIT_KEY", &key);
    if worker {
        run.env("HOME", t3::isolated_runtime_home(home))
            .envs(std::env::vars().filter(|(key, _)| {
                key.starts_with("MAOS_T3_")
                    || matches!(key.as_str(), "XDG_RUNTIME_DIR" | "DBUS_SESSION_BUS_ADDRESS")
            }));
        let mut paths = vec![fixtures::profile_dir()];
        if let Some(path) = std::env::var_os("PATH") {
            paths.extend(std::env::split_paths(&path));
        }
        run.env("PATH", std::env::join_paths(paths).expect("PATH"));
    }
    let stderr = tempfile::tempfile().expect("stderr");
    let mut child = run
        .stdout(Stdio::piped())
        .stderr(stderr.try_clone().expect("stderr handle"))
        .spawn()
        .expect("real root");
    let stdout = child.stdout.take().expect("stdout");
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&events);
    std::thread::spawn(move || {
        use std::io::BufRead as _;
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Ok(value) = serde_json::from_str(&line) {
                sink.lock().expect("events").push(value);
            }
        }
    });
    let mut live = Live {
        child,
        events,
        stderr,
    };
    let end = Instant::now() + Duration::from_secs(120);
    let (pid, id, cid) = loop {
        let loaded = live
            .events
            .lock()
            .expect("events")
            .iter()
            .find(|row| row["event"] == "cli_wrapper_loaded")
            .cloned();
        if let Some(row) = loaded {
            let id = row["spirit_id"].as_str().expect("worker id").to_owned();
            if h::ctl(&ctl, home, &["spirit", "inspect", &id])
                .status
                .success()
            {
                break (
                    row["spirit_pid"].as_u64().expect("pid") as u32,
                    id,
                    Some(row["container_id"].as_str().expect("CID").to_owned()),
                );
            }
        }
        if !worker {
            let loaded = h::rows(home).into_iter().find(|r| r.2 == "lifecycle.load");
            if let Some((_, pid, _, _)) = loaded {
                if h::ctl(&ctl, home, &["spirit", "inspect", "researcher"])
                    .status
                    .success()
                {
                    break (pid, "researcher".to_owned(), None);
                }
            }
        }
        assert!(
            live.child.try_wait().expect("status").is_none(),
            "root exited: {}",
            diagnostics(&live)
        );
        assert!(
            Instant::now() < end,
            "no live Spirit: {}",
            diagnostics(&live)
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let boot:i64=rusqlite::Connection::open(h::db(home)).expect("db").query_row("SELECT boot_nonce FROM transparency_log WHERE spirit_pid=?1 AND intent='lifecycle.load' ORDER BY rowid DESC LIMIT 1",[pid],|r|r.get(0)).expect("live boot identity");
    let stores = h::Stores::open(home, boot as u64);
    stores.tl.insert_frame_event(
        maos_kernel_core::iac::FrameKind::TaskAssign,
        pid,
        None,
        "live.lineage.source",
        b"principal-free live source",
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );
    let writer = maos_kernel_core::iac::distillate::DistillateWriter::new(
        Arc::clone(&stores.tl),
        Arc::new(()),
    );
    let digest = writer
        .write_distillate(
            pid,
            maos_domain::distillation::DistillationRequest::new(
                vec![stores.tl.last_frame_id()],
                1,
                maos_domain::distillation::DigestPayload::Text("live lineage witness".into()),
                None,
            )
            .expect("request"),
        )
        .expect("canonical live digest")
        .digest_frame_id;
    drop(stores);
    let output = h::ctl(&ctl, home, &["uninstall", &id]);
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let terminal = h::json(&output);
    let proof: maos_audit::erasure::proof::ErasureProof = serde_json::from_slice(
        &std::fs::read(terminal["proof_path"].as_str().expect("proof path")).expect("bundle"),
    )
    .unwrap_or_else(|e| panic!("bundle {e}"));
    assert!(proof.categories.iter().any(|c|c.name=="intent_lineage" && matches!(c.status,maos_audit::erasure::proof::CategoryStatus::Removed{count} if count>=1)));
    for (name, limitation) in [
        ("pending_halts", "final kernel"),
        ("scheduled_invocations", "pending KS"),
    ] {
        assert!(proof.categories.iter().any(|c|c.name==name && matches!(&c.status,maos_audit::erasure::proof::CategoryStatus::CoverageGap{reason} if reason.contains(limitation))));
    }
    assert!(proof
        .erased_frame_proofs
        .iter()
        .any(|frame| frame.frame_id.replace(':', "") == hex::encode(digest)));
    maos_audit::erasure::proof::verify_erasure_proof(
        &proof,
        &maos_audit::sealed_export::derive_pubkey(&[0x61; 32]),
    )
    .expect("verifiable live bundle");
    // Story 20-9 AC3 carries `[[schedule]]` at admission, so the watchdog fires `daily` while
    // the Spirit lives. After the uninstall completed it must never fire again.
    let fires = || {
        h::rows(home)
            .iter()
            .filter(|r| r.1 == pid && r.2 == "schedule.fire:daily")
            .count()
    };
    let settled = fires();
    for _ in 0..5 {
        std::thread::sleep(Duration::from_millis(150));
        assert!(
            fires() <= settled,
            "a forgotten Spirit's schedule fired again after the uninstall completed"
        );
    }
    assert!(
        !h::ctl(&ctl, home, &["spirit", "inspect", &id])
            .status
            .success(),
        "SCB remains"
    );
    if let Some(cid) = cid {
        let result = Command::new(std::env::var("MAOS_T3_RUNTIME").expect("runtime"))
            .args(["container", "exists", &cid])
            .output()
            .expect("runtime exists");
        assert_eq!(
            result.status.code(),
            Some(1),
            "container remains or query failed: {result:?}"
        );
        assert!(
            h::rows(home).iter().any(|r| r.1 == pid
                && r.2 == "cli.subprocess.exit"
                && r.3["container"]["removed"] == true),
            "existing teardown must attest removal"
        );
    }
    if !worker {
        drop(live);
        // Standalone `run <manifest>` intentionally does not enumerate installed
        // packages. The plain serving root is the real installed-boot surface.
        let mut serving = h::command(binary, home);
        // Bare `maos` serves through the shell: inherited EOF shuts its door.
        // Daemon retains Child.stdin until drop, keeping this root alive.
        serving.env("MAOS_AUDIT_KEY", &key).stdin(Stdio::piped());
        let serving = h::Daemon::spawn(serving);
        h::wait("installed package refusal", &serving, || {
            serving
                .diagnostics()
                .contains("installed spirit 'refused-installed' refused at boot")
        });
        let refused = h::ctl(&ctl, home, &["uninstall", "refused-installed"]);
        assert!(!refused.status.success());
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("spirit_not_loaded"),
            "{refused:?}"
        );
        drop(serving); // Release the root's store locks, then recover offline.
        let mut offline = h::command(binary, home);
        offline
            .env("MAOS_ONE_SHOT", "uninstall")
            .env("MAOS_SPIRIT_ID", "refused-installed")
            .env("MAOS_AUDIT_KEY", &key);
        let output = h::checked(offline);
        let terminal = h::json(&output);
        let proof: maos_audit::erasure::proof::ErasureProof = serde_json::from_slice(
            &std::fs::read(terminal["proof_path"].as_str().expect("offline proof"))
                .expect("bundle"),
        )
        .unwrap_or_else(|e| panic!("proof {e}"));
        assert!(proof
            .categories
            .iter()
            .any(|category| category.name == "installed_package"
                && matches!(
                    category.status,
                    maos_audit::erasure::proof::CategoryStatus::Removed { count: 1 }
                )));
        maos_audit::erasure::proof::verify_erasure_proof(
            &proof,
            &maos_audit::sealed_export::derive_pubkey(&[0x61; 32]),
        )
        .expect("offline recovery proof");
        assert!(!home
            .join(".local/share/maos/registry/installed/refused-installed")
            .exists());
    }
}
fn diagnostics(live: &Live) -> String {
    use std::io::{Read, Seek};
    let mut file = live.stderr.try_clone().expect("stderr");
    file.rewind().expect("rewind");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("read stderr");
    text
}
#[test]
fn ac2_live_door_erases_principal_free_lineage() {
    exercise(false);
}
#[test]
fn ac6_running_t3_worker_uninstall_proves_live_inventory() {
    exercise(true);
}
