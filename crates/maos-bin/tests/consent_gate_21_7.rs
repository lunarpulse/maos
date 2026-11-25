//! Story 21-7 AC1 — the production consent granter and gate, proven at the built `maos` binary
//! with no Postgres: three installed signed `wasm-component` Spirits built from the `recall-probe`
//! fixture, a real `maos shell` root, the real mediated recall/fetch. "Consent" here is a
//! technical recall/delivery restriction, not legal consent.
//!
//! * `probe-a` declares `[consent.recall] probe-b = "header_only"`; `probe-b` declares
//!   `[consent.recall] probe-a = "withheld"`. Each sends the other one frame.
//! * `probe-a` also sends a frame to `probe-c`, which it did not grant: the gate refuses it.
//!
//! Proven red (run by the orchestrator, one image): removing the daemon's gate install reds the
//! uncovered-addressee assertions; removing the granter stamping (`ConsentPolicy::stamp` at the
//! session send seam) reds the header-only/withheld assertions.
#![cfg(all(target_os = "linux", feature = "network", feature = "wasm-host"))]
#![forbid(unsafe_code)]

#[path = "support/installed_shell_20_9.rs"]
mod shell;
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;

use rusqlite::{Connection, OpenFlags};
use sha2::Digest;
use shell::Shell;
use std::io::Write;
use std::time::{Duration, Instant};

const PROBE: &str = "recall_probe.wasm";
const A: &str = "probe-a";
const B: &str = "probe-b";
const C: &str = "probe-c";
/// Payloads. None parses as a probe command, so a receiver's report can never echo it.
const SENT_AB: &str = "payload-a-to-b-withheld-1f3a";
const SENT_BA: &str = "payload-b-to-a-withheld-7c9d";
const SENT_AC: &str = "payload-a-to-c-refused-5e21";
const WAIT: Duration = Duration::from_secs(60);

fn manifest(id: &str, grants: &[(&str, &str)]) -> String {
    let consent = if grants.is_empty() {
        String::new()
    } else {
        let rows: Vec<String> = grants
            .iter()
            .map(|(to, p)| format!("{to} = \"{p}\""))
            .collect();
        format!("[consent.recall]\n{}\n", rows.join("\n"))
    };
    format!(
        r#"[class]
name = "{id}"
description = "consent gate witness"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 9
min_substrate_version = "0.1.0-alpha.1"
forms = ["wasm-component"]
artifact = "dist/{PROBE}"
trust_tier = "local"
[posture]
default = "assistive"
allowed_max = "assistive"
[output_shape]
required_fields = ["introduction"]
[budget]
context_window_size = 4096
time_cap_seconds = 30
[resources]
fd_max = 64
[sandbox]
tier = "T2"
[capabilities.required.iac]
send = ["spirit:peer"]
[capabilities.required.log]
recall = true
fetch = true
{consent}"#
    )
}

fn hex(text: &str) -> String {
    hex::encode(text.as_bytes())
}

/// A read-only view of the live Transparency Log the daemon writes.
struct Journal(std::path::PathBuf);

impl Journal {
    fn open(&self) -> Option<Connection> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(&self.0, flags).ok()?;
        conn.busy_timeout(Duration::from_secs(5)).ok()?;
        Some(conn)
    }

    fn poll<T>(&self, what: &str, mut probe: impl FnMut(&Connection) -> Option<T>) -> T {
        let until = Instant::now() + WAIT;
        loop {
            if let Some(found) = self.open().and_then(|conn| probe(&conn)) {
                return found;
            }
            assert!(Instant::now() < until, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(200));
        }
    }

    /// The routed frame `from` sent to `to` (the governance rows carry no addressee).
    fn frame_between(&self, from: &str, to: &str) -> [u8; 16] {
        self.poll(&format!("the frame {from} -> {to}"), |conn| {
            let sql = "SELECT frame_id FROM transparency_log
                       WHERE from_spirit_id = ?1 AND to_spirit_id = ?2 AND kind = 0";
            let id: Vec<u8> = conn.query_row(sql, [from, to], |row| row.get(0)).ok()?;
            id.try_into().ok()
        })
    }

    fn recipients(&self, frame: [u8; 16]) -> Vec<String> {
        let conn = self.open().expect("journal readable");
        let mut stmt = conn
            .prepare(
                "SELECT recipient_spirit_id FROM transparency_log_recipients
                 WHERE frame_id = ?1 ORDER BY recipient_spirit_id",
            )
            .unwrap();
        let rows = stmt.query_map([&frame[..]], |row| row.get(0)).unwrap();
        rows.map(Result::unwrap).collect()
    }

    /// Payload text of every row at `pid` whose `intent` satisfies `keep`.
    fn rows(&self, pid: u32, keep: impl Fn(&str) -> bool) -> Vec<String> {
        let Some(conn) = self.open() else {
            return Vec::new();
        };
        let mut stmt = conn
            .prepare(
                "SELECT intent, payload_redacted FROM transparency_log
                 WHERE spirit_pid = ?1 ORDER BY rowid",
            )
            .unwrap();
        let rows = stmt
            .query_map([pid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .unwrap();
        rows.map(Result::unwrap)
            .filter(|(intent, _)| keep(intent))
            .map(|(_, body)| String::from_utf8_lossy(&body).into_owned())
            .collect()
    }

    fn receipts(&self, pid: u32, code: &str) -> usize {
        let intent = format!("log.fetch.refused:{code}");
        self.rows(pid, |row| row == intent).len()
    }

    /// The probe's reports at `pid`, oldest first: one per emission, read from the
    /// authorization record the session journals before anything moves (its claims carry the
    /// emitted `recall.probe` telemetry frame).
    fn reports(&self, pid: u32) -> Vec<String> {
        fn data(value: &serde_json::Value) -> Option<String> {
            match value {
                serde_json::Value::Object(map) => {
                    let is_report =
                        map.get("event_type").and_then(|v| v.as_str()) == Some("recall.probe");
                    match map.get("data").and_then(|v| v.as_str()) {
                        Some(text) if is_report => Some(text.to_owned()),
                        _ => map.values().find_map(data),
                    }
                }
                serde_json::Value::Array(items) => items.iter().find_map(data),
                _ => None,
            }
        }
        self.rows(pid, |intent| intent == "spirit.frame.authorization")
            .iter()
            .filter_map(|body| data(&serde_json::from_str(body).ok()?))
            .collect()
    }
}

struct Witness {
    shell: Shell,
    journal: Journal,
    pids: [u32; 3],
}

impl Witness {
    fn pid(&self, id: &str) -> u32 {
        self.pids[[A, B, C].iter().position(|known| *known == id).unwrap()]
    }

    /// Dispatch one script line to `id` as the operator (the shell's `@id line`).
    fn send(&mut self, id: &str, line: &str) {
        let stdin = self.shell.stdin.as_mut().expect("shell stdin open");
        writeln!(stdin, "@{id} {line}").unwrap();
        stdin.flush().unwrap();
    }

    /// [`Self::send`], then the first NEW report of `id` containing `marker`. Reports the
    /// Spirit emits for the frames it receives carry no such marker, so they cannot be mistaken
    /// for the answer.
    fn ask(&mut self, id: &str, line: &str, marker: &str) -> String {
        let pid = self.pid(id);
        let marked = |reports: Vec<String>| -> Vec<String> {
            reports
                .into_iter()
                .filter(|text| text.contains(marker))
                .collect()
        };
        let before = marked(self.journal.reports(pid)).len();
        self.send(id, line);
        self.journal
            .poll(&format!("{id}'s `{marker}` report of `{line}`"), |_| {
                marked(self.journal.reports(pid)).get(before).cloned()
            })
    }

    fn fetch(&mut self, id: &str, frame: [u8; 16]) -> String {
        self.ask(id, &format!("fetch {}", hex::encode(frame)), "fetch ")
    }

    /// Nothing the recipient can see — its reports and its own `log.*` operation rows —
    /// carries the sender's payload, raw or hex.
    fn assert_payload_unseen(&self, recipient: &str, payload: &str) {
        let pid = self.pid(recipient);
        let seen = self
            .journal
            .reports(pid)
            .into_iter()
            .chain(self.journal.rows(pid, |intent| intent.starts_with("log.")));
        for body in seen {
            assert!(!body.contains(payload), "{recipient} saw {payload}: {body}");
            assert!(
                !body.contains(&hex(payload)),
                "{recipient} saw hex of {payload}: {body}"
            );
        }
    }
}

#[test]
fn production_consent_granter_and_gate_without_postgres() {
    let components = [
        (A, manifest(A, &[(B, "header_only")])),
        (B, manifest(B, &[(A, "withheld")])),
        (C, manifest(C, &[])),
    ];
    let (shell, _packages) = shell::boot_installed(&components, PROBE);

    // The Postgres-free arm was taken, not assumed.
    let stderr = shell.stderr();
    assert!(
        stderr.contains("collective tier (Loom-lite) not configured"),
        "boot did not take the Postgres-free arm:\n{stderr}"
    );
    assert!(
        stderr.contains("Mailbox consent gate wired (Story 21-7)"),
        "{stderr}"
    );

    let pid_of = |id: &str| -> u32 {
        shell
            .field(id, "pid")
            .and_then(|pid| pid.parse().ok())
            .expect("the door reports a pid")
    };
    let pids = [pid_of(A), pid_of(B), pid_of(C)];
    let journal = Journal(shell.scratch.0.join("home/.maos/audit/transparency.sqlite"));
    let mut w = Witness {
        shell,
        journal,
        pids,
    };

    // Each granted Spirit sends the other one frame; A also addresses the uncovered C.
    w.send(A, &format!("task {B} {}", hex(SENT_AB)));
    w.send(B, &format!("task {A} {}", hex(SENT_BA)));
    w.send(A, &format!("task {C} {}", hex(SENT_AC)));
    let a_to_b = w.journal.frame_between(A, B);
    let b_to_a = w.journal.frame_between(B, A);
    let a_to_c = w.journal.frame_between(A, C);

    // The gate: C is never a recorded recipient; the others are, and only they.
    assert_eq!(w.journal.recipients(a_to_b), [B]);
    assert_eq!(w.journal.recipients(b_to_a), [A]);
    assert!(
        w.journal.recipients(a_to_c).is_empty(),
        "the uncovered addressee was recorded"
    );
    // The sender receives the typed rupture (journaled before it is emitted, I2).
    let rupture = w
        .journal
        .poll("the consent rupture quarantine row", |conn| {
            let sql = "SELECT payload_redacted FROM transparency_log
                   WHERE intent = 'consent.rupture.quarantine' AND from_spirit_id = ?1";
            let body: Vec<u8> = conn.query_row(sql, [A], |row| row.get(0)).ok()?;
            Some(String::from_utf8_lossy(&body).into_owned())
        });
    assert!(rupture.contains("consent_not_granted"), "{rupture}");
    assert!(rupture.contains(C), "{rupture}");

    // The granter: A's grant to B is header-only — listed without a payload, fetch refused.
    let listing = w.ask(B, "recall", "recall ok");
    assert!(
        listing
            .lines()
            .any(|line| line.starts_with("entry ") && line.ends_with(" false")),
        "B must see A's frame listed header-only (payload-available = false): {listing}"
    );
    let fetched = w.fetch(B, a_to_b);
    assert!(fetched.contains("fetch err consent-withheld"), "{fetched}");
    assert_eq!(w.journal.receipts(w.pid(B), "consent-withheld"), 1);
    // B's grant to A is withheld: fetch refused, never listed.
    let fetched = w.fetch(A, b_to_a);
    assert!(fetched.contains("fetch err consent-withheld"), "{fetched}");
    assert_eq!(w.journal.receipts(w.pid(A), "consent-withheld"), 1);
    // The uncovered addressee never received the frame: out of scope, as in V14.
    let fetched = w.fetch(C, a_to_c);
    assert!(fetched.contains("fetch err scope-violation"), "{fetched}");
    assert_eq!(w.journal.receipts(w.pid(C), "scope-violation"), 1);

    // No withheld payload byte reached any recipient-visible output or recipient-pid row.
    w.assert_payload_unseen(B, SENT_AB);
    w.assert_payload_unseen(A, SENT_BA);
    w.assert_payload_unseen(C, SENT_AC);

    // Evidence bound to the binary, the tree, the environment and the job.
    let binary = std::fs::read(env!("CARGO_BIN_EXE_maos")).unwrap();
    let env = |name: &str| std::env::var(name).unwrap_or_else(|_| "-".into());
    println!(
        "CONSENT-GATE-EVIDENCE binary_sha256={} sha={} job={} run={} unset=MAOS_LOOM_POSTGRES,\
         MAOS_LOOM_HOME_TEAM,MAOS_CROSS_TEAM_SHARE_PEER",
        hex::encode(sha2::Sha256::digest(&binary)),
        env("GITHUB_SHA"),
        env("GITHUB_JOB"),
        env("GITHUB_RUN_ID"),
    );
}
