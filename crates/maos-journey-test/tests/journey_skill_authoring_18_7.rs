#![forbid(unsafe_code)]
//! Real daemon, authenticated operator door, reload and restart durability.
#[path = "support/binaries.rs"]
mod binaries;
#[path = "../../../tests/harness/forget_18_6.rs"]
mod h;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/skill-authoring-18-7.json";
const ID: &str = "researcher.survey-method";

struct Daemon {
    child: Child,
    log: Arc<Mutex<String>>,
}
impl Daemon {
    fn start(maos: &Path, home: &Path, manifest: &str) -> Self {
        let mut command = h::command(maos, home);
        command
            .args(["run", manifest])
            .env("MAOS_IDLE_FAST", "1")
            .env("MAOS_INFERENCE_MODE", "replay")
            .env("MAOS_REPLAY_STRICT", "1")
            .env("MAOS_REPLAY_CASSETTE", CASSETTE);
        let mut child = command
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut pipe = child.stderr.take().unwrap();
        let log = Arc::new(Mutex::new(String::new()));
        let sink = log.clone();
        std::thread::spawn(move || {
            let mut bytes = [0; 4096];
            while let Ok(n) = pipe.read(&mut bytes) {
                if n == 0 {
                    break;
                }
                sink.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&bytes[..n]));
            }
        });
        Self { child, log }
    }
    fn wait(&mut self, what: &str, mut predicate: impl FnMut() -> bool) {
        let end = Instant::now() + Duration::from_secs(15);
        while !predicate() {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "daemon exited waiting for {what}: {}",
                self.log.lock().unwrap()
            );
            assert!(
                Instant::now() < end,
                "waiting for {what}: {}",
                self.log.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn stop(mut self) {
        assert!(Command::new("kill")
            .args(["-INT", &self.child.id().to_string()])
            .status()
            .unwrap()
            .success());
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "clean shutdown: {}",
                    self.log.lock().unwrap()
                );
                break;
            }
            assert!(
                Instant::now() < end,
                "audit drain stalled: {}",
                self.log.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn queue(home: &Path) -> Vec<Value> {
    let path = home.join("skills/queue.json");
    if !path.exists() {
        return vec![];
    }
    let file: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(file["schema_version"], "maos.skill-queue.v2");
    file["pending"].as_array().unwrap().clone()
}
fn control(ctl: &Path, home: &Path, args: &[&str]) -> Value {
    let output = h::ctl(ctl, home, args);
    assert!(
        output.status.success(),
        "{:?}: stdout={} stderr={}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or(Value::Null)
}
fn approvals(home: &Path) -> Vec<(String, String, String, String)> {
    let db = rusqlite::Connection::open_with_flags(
        h::db(home),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut query=db.prepare("SELECT actor,target,capability,COALESCE(reasoning,'') FROM approval_decision_log ORDER BY decision_id").unwrap();
    query
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
}
fn binaries() -> (PathBuf, PathBuf) {
    (
        PathBuf::from(binaries::resolve("maos", option_env!("CARGO_BIN_EXE_maos"))),
        PathBuf::from(binaries::resolve(
            "maosctl",
            option_env!("CARGO_BIN_EXE_maosctl"),
        )),
    )
}
fn init(maos: &Path, home: &Path) {
    let mut command = h::command(maos, home);
    command.arg("init");
    h::checked(command);
}
fn find_entry(value: &Value, id: &str) -> Option<Value> {
    if value.get("entry_id").and_then(Value::as_str) == Some(id) {
        return Some(value.clone());
    }
    match value {
        Value::Object(values) => values.values().find_map(|v| find_entry(v, id)),
        Value::Array(values) => values.iter().find_map(|v| find_entry(v, id)),
        _ => None,
    }
}

#[test]
fn live_author_revision_restart_and_rejection_preserve_the_payload() {
    let (maos, ctl) = binaries();
    let home = tempfile::tempdir().unwrap();
    init(&maos, home.path());
    let mut first = Daemon::start(&maos, home.path(), "spirits/researcher/manifest.toml");
    first.wait("author payload", || {
        queue(home.path())
            .iter()
            .any(|e| e["entry_path"] == "author_self")
    });
    let author = queue(home.path())
        .into_iter()
        .find(|e| e["entry_path"] == "author_self")
        .unwrap();
    assert_eq!(author["id"], ID);
    assert_eq!(author["version"], "0.1.0");
    assert_eq!(author["state"], "Pending");
    let document = author["payload"]["content"]
        .as_str()
        .expect("durable authored document");
    assert!(document.contains(ID));
    let author_id = author["entry_id"].as_str().unwrap();
    control(&ctl, home.path(), &["skills", "approve", author_id]);
    first.wait("author admitted", || {
        queue(home.path())
            .iter()
            .any(|e| e["entry_id"] == author_id && e["state"] == "Admitted")
    });
    assert_eq!(
        std::fs::read_to_string(home.path().join(format!(".maos/skills/{ID}.md"))).unwrap(),
        document
    );
    control(&ctl, home.path(), &["unload", "researcher"]);
    control(
        &ctl,
        home.path(),
        &["load", "spirits/researcher/manifest.toml"],
    );
    control(&ctl, home.path(), &["start", "researcher"]);
    first.wait("revision payload", || {
        queue(home.path())
            .iter()
            .any(|e| e["entry_path"] == "revision_proposal")
    });
    let proposal = queue(home.path())
        .into_iter()
        .find(|e| e["entry_path"] == "revision_proposal")
        .unwrap();
    let content = &proposal["payload"]["content"];
    assert!(
        content["proposed_diff"]
            .as_str()
            .is_some_and(|diff| !diff.is_empty()),
        "durable proposal payload: diff missing: {proposal}"
    );
    assert!(
        content["telemetry_evidence"].is_object(),
        "durable proposal payload: evidence missing: {proposal}"
    );
    assert_eq!(content["target_skill_id"], ID);
    assert_eq!(content["target_version"], "0.1.0");
    assert_ne!(author["author"]["pid"], proposal["author"]["pid"]);
    assert_eq!(
        content["telemetry_evidence"]["spirit_pid"],
        proposal["author"]["pid"]
    );
    let proposal_id = proposal["entry_id"].as_str().unwrap();
    let target = format!("{ID}@0.1.0#{proposal_id}");
    for entry in [&author, &proposal] {
        let id = entry["entry_id"].as_str().unwrap();
        let sha = entry["payload_sha256"].as_str().unwrap();
        assert!(
            approvals(home.path())
                .iter()
                .any(|row| row.2 == "skill.admission.enqueue"
                    && row.3.contains(id)
                    && row.3.contains(sha)),
            "enqueue references sealed payload {id}"
        );
        let pid = entry["author"]["pid"].as_u64().unwrap() as u32;
        assert!(
            h::rows(home.path()).iter().any(|row| row.1 == pid
                && row.2.starts_with("cap.issue")
                && row.3.to_string().contains("SkillAuthorSelf")),
            "live mint at fixed author pid"
        );
        assert!(
            h::rows(home.path())
                .iter()
                .any(|row| row.1 == pid && row.2.starts_with("cap.verify")),
            "live verification at fixed author pid"
        );
    }
    first.stop();
    let mut second = Daemon::start(&maos, home.path(), "spirits/reviewer/manifest.toml");
    second.wait("restart door", || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "reviewer"])
            .status
            .success()
    });
    let shown = control(&ctl, home.path(), &["skills", "show", proposal_id]);
    let restored =
        find_entry(&shown, proposal_id).expect("door show returns durable proposal payload");
    assert_eq!(
        restored, proposal,
        "restart preserves target/version/diff/evidence and sha byte-for-byte"
    );
    control(&ctl, home.path(), &["skills", "reject", proposal_id]);
    second.wait("revision rejected", || {
        queue(home.path())
            .iter()
            .any(|e| e["entry_id"] == proposal_id && e["state"] == "Rejected")
    });
    assert!(
        queue(home.path())
            .iter()
            .any(|e| e["entry_id"] == author_id && e["state"] == "Admitted"),
        "revision rejection cannot flip the base"
    );
    assert_eq!(
        control(&ctl, home.path(), &["skills", "show", author_id])["state"],
        "Admitted",
        "revision rejection cannot flip the base"
    );
    assert!(
        approvals(home.path())
            .iter()
            .any(|row| row.1 == target && row.2 == "skill.admission.reject"),
        "revision has independent approval target"
    );
    assert!(
        !approvals(home.path())
            .iter()
            .any(|row| row.3.contains(document)
                || row.3.contains(content["proposed_diff"].as_str().unwrap())),
        "decision rows contain references, not content"
    );
    second.stop();
    let mut third = Daemon::start(&maos, home.path(), "spirits/reviewer/manifest.toml");
    third.wait("rejected queue entry is boot-inert", || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "reviewer"])
            .status
            .success()
    });
    third.stop();
}

#[test]
fn undeclared_live_researcher_is_refused_without_queue_mutation() {
    let (maos, ctl) = binaries();
    let home = tempfile::tempdir().unwrap();
    init(&maos, home.path());
    let mut daemon = Daemon::start(
        &maos,
        home.path(),
        "tests/fixtures/researcher-no-skill/manifest.toml",
    );
    daemon.wait("undeclared Researcher door", || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "researcher"])
            .status
            .success()
    });
    daemon.wait("undeclared author refusal", || {
        h::rows(home.path())
            .iter()
            .any(|row| row.2 == "skill.author.refused:capability-denied")
            || !queue(home.path()).is_empty()
    });
    assert!(
        queue(home.path()).is_empty(),
        "undeclared Researcher cannot enqueue"
    );
    assert!(!approvals(home.path())
        .iter()
        .any(|row| row.2 == "skill.admission.enqueue"));
    assert!(
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "researcher"])
            .status
            .success()
    );
    daemon.stop();
}

#[test]
fn rejected_spirit_authored_draft_does_not_block_the_next_boot() {
    let (maos, ctl) = binaries();
    let home = tempfile::tempdir().unwrap();
    init(&maos, home.path());
    let mut first = Daemon::start(&maos, home.path(), "spirits/researcher/manifest.toml");
    first.wait("authored draft", || !queue(home.path()).is_empty());
    let draft = queue(home.path())
        .into_iter()
        .find(|entry| entry["entry_path"] == "author_self")
        .unwrap();
    control(
        &ctl,
        home.path(),
        &["skills", "reject", draft["entry_id"].as_str().unwrap()],
    );
    assert_eq!(queue(home.path())[0]["state"], "Rejected");
    first.stop();
    let mut second = Daemon::start(&maos, home.path(), "spirits/reviewer/manifest.toml");
    second.wait("boot after rejected draft", || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "reviewer"])
            .status
            .success()
    });
    assert_eq!(queue(home.path())[0]["state"], "Rejected");
    second.stop();
}

#[test]
fn corrupt_queue_warns_at_boot_and_refuses_door_operations_without_overwrite() {
    let (maos, ctl) = binaries();
    let home = tempfile::tempdir().unwrap();
    init(&maos, home.path());
    let path = home.path().join("skills/queue.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let corrupt = b"{corrupt skill queue";
    std::fs::write(&path, corrupt).unwrap();
    let mut daemon = Daemon::start(&maos, home.path(), "spirits/reviewer/manifest.toml");
    daemon.wait("boot with corrupt queue", || {
        h::ctl(&ctl, home.path(), &["spirit", "inspect", "reviewer"])
            .status
            .success()
    });
    assert!(daemon
        .log
        .lock()
        .unwrap()
        .contains("proceeding from discovery + TL"));
    for args in [vec!["skills", "show", ID], vec!["skills", "reject", ID]] {
        assert!(!h::ctl(&ctl, home.path(), &args).status.success());
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }
    daemon.stop();
}

#[test]
fn no_data_home_does_not_abort_the_daemon_or_offline_children() {
    let (maos, _) = binaries();
    let scratch = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir_in(scratch.path()).unwrap();
    let mut command = h::command(&maos, home.path());
    command
        .args(["run", "spirits/reviewer/manifest.toml", "--once"])
        .env_remove("HOME")
        .env_remove("MAOS_HOME")
        .env(
            "MAOS_JOURNAL_PATH",
            home.path().join("journal/lifecycle.ndjson"),
        )
        .env("MAOS_ARCHIVE_DIR", home.path().join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", home.path().join("proofs"))
        .env("MAOS_AUDIT_DB", h::db(home.path()));
    let output = h::checked(command);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("skill queue unavailable"), "{stderr}");
    assert!(stderr.contains("HOME is unavailable"), "{stderr}");
    let mut child = h::command(&maos, home.path());
    child
        .env_remove("HOME")
        .env_remove("MAOS_HOME")
        .env(
            "MAOS_JOURNAL_PATH",
            home.path().join("journal/lifecycle.ndjson"),
        )
        .env("MAOS_ARCHIVE_DIR", home.path().join("archives"))
        .env("MAOS_ERASURE_PROOFS_DIR", home.path().join("proofs"))
        .env("MAOS_AUDIT_DB", h::db(home.path()))
        .env("MAOS_ONE_SHOT", "forget");
    let output = child.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Error: HOME is unavailable"), "{stderr}");
    assert!(
        stderr.contains("MAOS_FORGET_PRINCIPAL"),
        "offline child reaches its own argument validation: {stderr}"
    );
}

#[test]
fn rejected_discovered_skill_blocks_even_a_non_run_root() {
    let (maos, ctl) = binaries();
    let home = tempfile::tempdir().unwrap();
    init(&maos, home.path());
    let path = home.path().join(".maos/skills/package.rejected.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "---\nid='package.rejected'\nversion='1.0.0'\nname='Rejected'\ndescription='d'\n---\nRejected procedure\n").unwrap();
    control(&ctl, home.path(), &["skills", "reject", "package.rejected"]);
    let mut command = h::command(&maos, home.path());
    command.env("MAOS_ONE_SHOT", "hello-spirit");
    let output = command.output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rejected skill(s) block daemon startup"),
        "{stderr}"
    );
}
