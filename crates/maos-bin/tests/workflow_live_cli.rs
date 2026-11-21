#![cfg(feature = "network")]

//! Bounded LIVE `workflow compare` against a fake local Ollama server.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::{tempdir, TempDir};

const MODEL: &str = "llama3.1:8b";
const LEAK: &str = "LEAK-MARKER";
const MAX_CALLS: &str = "MAOS_WORKFLOW_MAX_INFERENCE_CALLS";
const DENY_INFERENCE: &str = r#"forbid(principal, action == Action::"provider.infer", resource);"#;
// (token unique to the excerpt, purchase_order_present, supporting quote); same order as the cases file.
const CASES: [(&str, bool, &str); 3] = [
    (
        "PO-731",
        true,
        "The buyer issued purchase order PO-731 for this invoice.",
    ),
    (
        "14 May",
        false,
        "Procurement confirms that no purchase order was issued for this invoice.",
    ),
    ("N/A (not issued)", false, "PO field: N/A (not issued)."),
];
const IDS: [&str; 3] = [
    "direct-assigned",
    "direct-missing",
    "abbreviation-unassigned",
];

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/workflows")
        .join(name)
}

// ---- fake Ollama -----------------------------------------------------------------------------

struct Fake {
    url: String,
    addr: SocketAddr,
    calls: Arc<AtomicUsize>,
    bodies: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn read_request(stream: &mut TcpStream) -> Option<String> {
    let (mut buf, mut chunk) = (Vec::new(), [0u8; 4096]);
    let head_end = loop {
        if let Some(at) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_lowercase();
    if !head.starts_with("post /api/chat") {
        return None;
    }
    let len = head.lines().find_map(|line| {
        line.strip_prefix("content-length:")?
            .trim()
            .parse::<usize>()
            .ok()
    })?;
    while buf.len() < head_end + len {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8(buf[head_end..head_end + len].to_vec()).ok()
}

impl Fake {
    /// `answer` maps each request body to the canned `Reply` the fake sends back.
    fn start(delay: Duration, answer: impl Fn(&str) -> Reply + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let bodies = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (c, b, s) = (calls.clone(), bodies.clone(), stop.clone());
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if s.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { continue };
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let Some(body) = read_request(&mut stream) else {
                    continue;
                };
                c.fetch_add(1, Ordering::SeqCst);
                b.lock().unwrap().push(body.clone());
                thread::sleep(delay);
                let reply = answer(&body);
                let status = reply.status;
                let payload = if status == 200 {
                    json!({"message": {"content": reply.text}, "done_reason": reply.done,
                           "prompt_eval_count": reply.usage.0, "eval_count": reply.usage.1})
                    .to_string()
                } else {
                    reply.text
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{payload}",
                    payload.len()
                );
            }
        });
        Self {
            url: format!("http://{addr}"),
            addr,
            calls,
            bodies,
            stop,
            thread: Some(thread),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// A canned provider reply. Status 200 wraps `text` in an Ollama envelope; other statuses send
/// `text` as the raw error body.
struct Reply {
    status: u16,
    text: String,
    done: &'static str,
    usage: (u64, u64),
}

impl Reply {
    fn ok(text: String, usage: (u64, u64)) -> Self {
        Self {
            status: 200,
            text,
            done: "stop",
            usage,
        }
    }

    fn done(self, done: &'static str) -> Self {
        Self { done, ..self }
    }

    fn status(status: u16, text: String) -> Self {
        Self {
            status,
            text,
            done: "stop",
            usage: (0, 0),
        }
    }
}

fn finding(present: bool, quote: &str) -> String {
    json!({"outcome": "finding_ready",
           "artifact": {"purchase_order_present": present, "quote": quote}})
    .to_string()
}

fn index(body: &str) -> usize {
    CASES
        .iter()
        .position(|case| body.contains(case.0))
        .expect("request carries a known excerpt")
}

/// The model's answer for case `i`: the labelled one, or a valid, cited answer with the label flipped.
fn truth(i: usize, correct: bool) -> String {
    finding(CASES[i].1 ^ !correct, CASES[i].2)
}

fn right(body: &str) -> Reply {
    Reply::ok(truth(index(body), true), (11, 7))
}

/// Per-excerpt request counter, so a script can vary answers across the repeats of one case.
#[derive(Default)]
struct Seen(Mutex<[usize; 3]>);

impl Seen {
    /// `(case index, number of earlier requests for that case)`.
    fn next(&self, body: &str) -> (usize, usize) {
        let i = index(body);
        let mut seen = self.0.lock().unwrap();
        seen[i] += 1;
        (i, seen[i] - 1)
    }
}

// ---- CLI plumbing ----------------------------------------------------------------------------

fn maos(dir: &Path, args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(dir)
        .env_clear()
        .env("HOME", dir)
        .env("MAOS_HOME", dir.join("maos"))
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CACHE_HOME", dir.join("cache"))
        .env("MAOS_SECRETS_BACKEND", "env")
        .args(args);
    cmd
}

fn replay(mut cmd: Command) -> Command {
    cmd.env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env(
            "MAOS_REPLAY_CASSETTE",
            example("invoice-evidence-missing.cassette.json"),
        );
    cmd
}

fn live(dir: &Path, fake: &Fake, args: &[&str], max: Option<&str>) -> Command {
    let mut cmd = maos(dir, args);
    cmd.env("MAOS_INFERENCE_MODE", "live")
        .env("MAOS_OLLAMA_URL", &fake.url);
    if let Some(max) = max {
        cmd.env(MAX_CALLS, max);
    }
    cmd
}

fn plan(dir: &Path, topology: &str, out: &str) -> Value {
    let topology = example(topology);
    let output = maos(dir, &["workflow", "plan", topology.to_str().unwrap(), out])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    stdout_records(&output)
        .into_iter()
        .find(|record| record.get("digest").is_some())
        .unwrap_or_else(|| panic!("missing digest: {output:?}"))
}

fn stdout_records(out: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

fn report(out: &Output) -> Option<Value> {
    stdout_records(out)
        .into_iter()
        .find(|record| record["event"] == "workflow_comparison")
}

struct Fixture {
    temp: TempDir,
    base: Value,
    candidate: Value,
}

impl Fixture {
    /// Deploys `invoice-evidence.toml` under `catalog` (keyless replay stages it; no inference runs)
    /// and plans the reroute as the candidate.
    fn new(catalog: &str) -> Self {
        let temp = tempdir().unwrap();
        let dir = temp.path();
        fs::create_dir(dir.join("maos")).unwrap();
        fs::copy(example(catalog), dir.join("maos/workflow-catalog.toml")).unwrap();
        let base = plan(dir, "invoice-evidence.toml", "base.json");
        let digest = base["digest"].as_str().unwrap();
        let apply = replay(maos(dir, &["workflow", "apply", "base.json", digest]))
            .output()
            .unwrap();
        assert!(apply.status.success(), "{apply:?}");
        let candidate = plan(dir, "invoice-evidence-reroute.toml", "candidate.json");
        assert_eq!(candidate["previous"], base["digest"]);
        Self {
            temp,
            base,
            candidate,
        }
    }

    fn dir(&self) -> &Path {
        self.temp.path()
    }

    fn deployments(&self) -> Vec<u8> {
        fs::read(self.dir().join("maos/workflow-deployments.sqlite")).unwrap()
    }

    fn compare_args(&self, extra: &[&str]) -> Vec<String> {
        let cases = example("invoice-evidence-live-cases.json");
        ["workflow", "compare", "candidate.json"]
            .into_iter()
            .chain([self.candidate["digest"].as_str().unwrap()])
            .chain([cases.to_str().unwrap()])
            .chain(extra.iter().copied())
            .map(str::to_owned)
            .collect()
    }

    fn compare(&self, mut cmd: impl FnMut(&[&str]) -> Command, extra: &[&str]) -> Output {
        let args = self.compare_args(extra);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        cmd(&args).output().unwrap()
    }

    fn compare_live(&self, fake: &Fake, extra: &[&str], max: Option<&str>) -> Output {
        self.compare(|args| live(self.dir(), fake, args, max), extra)
    }
}

fn audit(dir: &Path) -> Connection {
    Connection::open_with_flags(
        dir.join("maos/audit/transparency.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap()
}

fn watermark(db: &Connection) -> i64 {
    db.query_row(
        "SELECT coalesce(max(rowid), 0) FROM transparency_log",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

fn rows(db: &Connection, after: i64, like: &str) -> Vec<Value> {
    let mut stmt = db
        .prepare(
            "SELECT payload_redacted FROM transparency_log
             WHERE rowid > ?1 AND intent LIKE ?2 ORDER BY rowid",
        )
        .unwrap();
    stmt.query_map(rusqlite::params![after, like], |r| r.get::<_, Vec<u8>>(0))
        .unwrap()
        .map(|bytes| serde_json::from_slice(&bytes.unwrap()).unwrap_or(Value::Null))
        .collect()
}

#[track_caller]
fn assert_refused(out: &Output, hint: &str) {
    assert!(
        out.status.code().is_some() && !out.status.success(),
        "{out:?}"
    );
    assert!(report(out).is_none(), "no report on refusal: {out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
    assert!(stderr.contains(&hint.to_lowercase()), "{hint}: {out:?}");
}

#[track_caller]
fn assert_plan(summary: &Value, expected: [u64; 4], tokens: (u64, u64), exact: bool) {
    let [runs, ran, passed, abstained] = expected;
    assert_eq!(summary["runs"], runs, "{summary}");
    assert_eq!(summary["ran"], ran, "{summary}");
    assert_eq!(summary["passed"], passed, "{summary}");
    assert_eq!(summary["abstained"], abstained, "{summary}");
    assert_eq!(summary["failed"], ran - passed, "{summary}");
    assert!((summary["pass_rate"].as_f64().unwrap() - passed as f64 / runs as f64).abs() < 1e-9);
    assert!((summary["ran_rate"].as_f64().unwrap() - ran as f64 / runs as f64).abs() < 1e-9);
    // Exact when every response is accounted for; otherwise a lower bound (ran runs only).
    let got = (
        summary["input_tokens"].as_u64().unwrap(),
        summary["output_tokens"].as_u64().unwrap(),
    );
    let ok = if exact {
        got == tokens
    } else {
        got.0 >= tokens.0 && got.1 >= tokens.1
    };
    assert!(ok, "{summary}");
}

/// One column of a case side's per-run details, in run order (null where absent).
fn col(side: &Value, key: &str) -> Value {
    side["details"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.get(key).cloned().unwrap_or(Value::Null))
        .collect()
}

#[track_caller]
fn assert_clean(text: &str, what: &str) {
    let secrets = CASES.iter().flat_map(|c| [c.0, c.2]).chain([
        "Input JSON",
        "Read the invoice excerpt",
        LEAK,
        "upstream rejected",
    ]);
    for secret in secrets {
        assert!(!text.contains(secret), "{secret} leaked into {what}");
    }
}

// ---- tests -----------------------------------------------------------------------------------

#[test]
fn live_comparison_separates_accuracy_from_cost_and_changes_nothing() {
    let fx = Fixture::new("catalog-live.toml");
    // Per case, requests 0..3 are the baseline's (it runs first) and 3..6 the candidate's: the two
    // plans get different usage so swapped stats fail. The first case answers wrongly on repeat 2.
    let seen = Seen::default();
    let fake = Fake::start(Duration::from_millis(5), move |body| {
        let (i, n) = seen.next(body);
        let usage = if n < 3 { (11, 7) } else { (13, 5) };
        Reply::ok(truth(i, !(i == 0 && n % 3 == 1)), usage)
    });
    let db = audit(fx.dir());
    let before = watermark(&db);
    let deployed = fx.deployments();

    // 3 cases x repeat 3 x 2 plans x 1 inference step = 18 calls: the exact bound is accepted.
    let out = fx.compare_live(&fake, &["--repeat", "3"], Some("18"));
    assert!(out.status.success(), "{out:?}");
    let report = report(&out).expect("one workflow_comparison record");
    assert_eq!(stdout_records(&out).len(), 1, "{out:?}");

    assert_eq!(report["format"], 2);
    assert_eq!(report["mode"], "live");
    assert_eq!(report["workflow"], "invoice-evidence");
    assert_eq!(report["baseline_revision"], fx.base["digest"]);
    assert_eq!(report["candidate_revision"], fx.candidate["digest"]);
    let cases_text = fs::read(example("invoice-evidence-live-cases.json")).unwrap();
    assert_eq!(
        report["cases_sha256"],
        hex::encode(Sha256::digest(cases_text))
    );
    assert_eq!(report["repeat"], 3);
    assert_eq!(report["max_inference_calls"], 18);

    // Baseline: only the first case's right runs pass (the old terminal disagrees with the labels
    // on the missing-order cases: accuracy failures, never abstentions). Candidate: 2 + 3 + 3.
    let plans = &report["plans"];
    assert_plan(&plans["baseline"], [9, 9, 2, 0], (99, 63), true);
    assert_plan(&plans["candidate"], [9, 9, 8, 0], (117, 45), true);
    for side in ["baseline", "candidate"] {
        let p = &plans[side];
        assert_eq!(p["calls"], 9);
        let (total, max) = (
            p["latency_ms_total"].as_u64().unwrap(),
            p["latency_ms_max"].as_u64().unwrap(),
        );
        assert!(total >= 9 * 5 && max >= 5 && max <= total, "{p}");
    }
    assert_eq!(fake.calls(), 18);
    assert_eq!(fake.bodies.lock().unwrap().len(), 18);

    // Ordered per-run details and exact per-case counts: repeats are neither merged nor duplicated.
    let cases = report["cases"].as_array().unwrap();
    let ids: Vec<_> = cases.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, IDS);
    let (ready, request, hold) = (
        "ready_for_approval",
        "request_order",
        "hold_for_procurement",
    );
    let expected = [
        (
            ([true, false, true], [ready, request, ready]),
            ([true, false, true], [ready, hold, ready]),
        ),
        (([false; 3], [request; 3]), ([true; 3], [hold; 3])),
        (([false; 3], [request; 3]), ([true; 3], [hold; 3])),
    ];
    for (case, (baseline, candidate)) in cases.iter().zip(expected) {
        for (side, (passes, outcomes)) in [("baseline", baseline), ("candidate", candidate)] {
            let r = &case[side];
            assert_eq!((r["runs"].as_u64(), r["ran"].as_u64()), (Some(3), Some(3)));
            assert_eq!(r["passed"], passes.iter().filter(|p| **p).count(), "{case}");
            assert_eq!(col(r, "passed"), json!(passes), "{case}");
            assert_eq!(col(r, "observed_outcome"), json!(outcomes), "{case}");
            assert_eq!(col(r, "error"), json!([null, null, null]), "{case}");
        }
    }
    for side in ["baseline", "candidate"] {
        let checks = col(&cases[0][side], "artifact_checks");
        assert_eq!(checks[0][0]["mismatched_fields"], json!([]), "{checks}");
        assert_eq!(
            checks[1][0]["mismatched_fields"],
            json!(["purchase_order_present"])
        );
        assert_eq!(checks[2][0]["mismatched_fields"], json!([]));
    }

    // Excerpt and prompt reach the provider; the report line and the host evaluation row hold none
    // of it. (The kernel's standard `infer:` audit rows may carry the prompt: existing behavior.)
    for body in fake.bodies.lock().unwrap().iter() {
        assert!(body.contains(&format!(r#""model":"{MODEL}""#)) && body.contains("Input JSON"));
        assert!(CASES.iter().any(|case| body.contains(case.0)));
    }
    assert_clean(&String::from_utf8_lossy(&out.stdout), "the report line");

    // Governed path: every call is in the log, plus one host evaluation event; nothing deployed.
    assert_eq!(rows(&db, before, "infer:%").len(), 18);
    let evaluation = rows(&db, before, "workflow:evaluation");
    assert_eq!(evaluation.len(), 1);
    assert_eq!(evaluation[0]["repeat"], 3);
    assert_eq!(evaluation[0]["calls"], 18);
    assert_clean(&evaluation[0].to_string(), "the evaluation row");
    assert_eq!(
        fx.deployments(),
        deployed,
        "comparison must not change deployment state"
    );
}

#[test]
fn comparison_refuses_before_any_call() {
    let fx = Fixture::new("catalog-live.toml");
    let fake = Fake::start(Duration::ZERO, right);
    let db = audit(fx.dir());
    let before = watermark(&db);
    let deployed = fx.deployments();

    for (extra, max, hint) in [
        (&[][..], None, MAX_CALLS),
        (&[][..], Some("0"), MAX_CALLS),
        (&[][..], Some("many"), MAX_CALLS),
        (&["--repeat", "3"][..], Some("17"), "calls"), // bound is 18
        (&[][..], Some("5"), "calls"),                 // bound is 6
        (&["--repeat", "0"][..], Some("1000"), "repeat"),
        (&["--repeat", "21"][..], Some("1000"), "repeat"),
        (&["--repeat", "x"][..], Some("1000"), "repeat"),
    ] {
        let out = fx.compare_live(&fake, extra, max);
        assert_refused(&out, hint);
        assert_eq!(fake.calls(), 0, "{extra:?} {max:?}");
    }

    // k > 1 is a live-only feature: rules and replay mode refuse it before any work.
    let rules = fx.compare(|args| maos(fx.dir(), args), &["--repeat", "3"]);
    assert_refused(&rules, "repeat");
    let replayed = fx.compare(|args| replay(maos(fx.dir(), args)), &["--repeat", "3"]);
    assert_refused(&replayed, "repeat");

    assert_eq!(rows(&db, before, "infer:%").len(), 0);
    assert_eq!(rows(&db, before, "workflow:dispatch").len(), 0);
    assert_eq!(rows(&db, before, "workflow:evaluation").len(), 0);
    assert_eq!(fx.deployments(), deployed);
}

#[test]
fn governance_failures_abort_without_a_report_or_a_provider_call() {
    // Operator policy forbids provider inference: denied before the provider is contacted.
    let fx = Fixture::new("catalog-live.toml");
    let fake = Fake::start(Duration::ZERO, right);
    let db = audit(fx.dir());
    let before = watermark(&db);
    let denied = fx.compare(
        |args| {
            let mut cmd = live(fx.dir(), &fake, args, Some("6"));
            cmd.env("MAOS_PDP_POLICY_INLINE", DENY_INFERENCE);
            cmd
        },
        &[],
    );
    assert_refused(&denied, "denied");
    assert_eq!(fake.calls(), 0);
    assert_eq!(rows(&db, before, "infer:%").len(), 0);

    // The catalog pins `anthropic`, which has no key here: fail closed, never reroute to Ollama.
    let fx = Fixture::new("catalog.toml");
    let fake = Fake::start(Duration::ZERO, right);
    let db = audit(fx.dir());
    let before = watermark(&db);
    let deployed = fx.deployments();
    let out = fx.compare_live(&fake, &[], Some("6"));
    assert_refused(&out, "pinned provider 'anthropic' is not registered");
    assert_eq!(fake.calls(), 0);
    assert_eq!(rows(&db, before, "infer:%").len(), 0);
    assert_eq!(fx.deployments(), deployed);
}

#[test]
fn per_run_failures_abstain_with_fixed_diagnostics_and_wrong_answers_fail() {
    let fx = Fixture::new("catalog-live.toml");
    // Scripted per case by repeat index r (requests n, plan-relative r = n % 4; both plans alike).
    // Every failure body echoes the request and LEAK, which must never reach the report.
    let seen = Seen::default();
    let fake = Fake::start(Duration::ZERO, move |body| {
        let (i, n) = seen.next(body);
        let unknown = json!({"outcome": format!("{LEAK}-approved"),
            "artifact": {"purchase_order_present": CASES[i].1, "quote": CASES[i].2}});
        match (i, n % 4) {
            (0, 0) => Reply::ok(format!("{LEAK} not json {{"), (11, 7)),
            (0, 1) => Reply::status(500, format!("upstream rejected: {body} {LEAK}")),
            (0, 2) => Reply::ok(truth(0, true), (11, 7)).done("length"),
            (1, 0) => Reply::ok(unknown.to_string(), (11, 7)),
            (1, 2) => Reply::ok(truth(1, false), (11, 7)),
            _ => Reply::ok(truth(i, true), (11, 7)),
        }
    });
    let db = audit(fx.dir());
    let before = watermark(&db);

    // 3 cases x repeat 4 x 2 plans = 24 calls, all of them real requests (a 500 is not retried).
    let out = fx.compare_live(&fake, &["--repeat", "4"], Some("24"));
    assert!(
        out.status.success(),
        "ordinary run failures never abort: {out:?}"
    );
    let report = report(&out).expect("report is still produced");
    assert_eq!(report["repeat"], 4);
    assert_eq!(fake.calls(), 24);
    assert_eq!(fake.bodies.lock().unwrap().len(), 24);

    // Per plan: 12 runs, 4 abstained (malformed, 500, length, undeclared outcome), 8 ran.
    // Baseline passes only the first case's right run; the candidate passes 1 + 2 + 4.
    assert_plan(&report["plans"]["baseline"], [12, 8, 1, 4], (88, 56), false);
    assert_plan(
        &report["plans"]["candidate"],
        [12, 8, 7, 4],
        (88, 56),
        false,
    );
    for side in ["baseline", "candidate"] {
        assert_eq!(report["plans"][side]["calls"], 12);
    }

    let a_errors = json!([
        "malformed evidence JSON",
        "provider rejected (status 500)",
        "incomplete response",
        null
    ]);
    let b_errors = json!(["unknown outcome", null, null, null]);
    let expected = [
        (
            a_errors,
            [false, false, false, true],
            [false, false, false, true],
        ),
        (b_errors, [false; 4], [false, true, false, true]),
        (json!([null, null, null, null]), [false; 4], [true; 4]),
    ];
    let cases = report["cases"].as_array().unwrap();
    for (case, (errors, baseline, candidate)) in cases.iter().zip(expected) {
        for (side, passes) in [("baseline", baseline), ("candidate", candidate)] {
            let r = &case[side];
            assert_eq!(r["runs"], 4);
            assert_eq!(col(r, "error"), errors, "{case}");
            assert_eq!(col(r, "passed"), json!(passes), "{case}");
            assert_eq!(r["passed"], passes.iter().filter(|p| **p).count());
            let ran = errors.as_array().unwrap().iter().filter(|e| e.is_null());
            assert_eq!(r["ran"], ran.count(), "{case}");
        }
    }

    assert_clean(&String::from_utf8_lossy(&out.stdout), "the report line");
    let evaluation = rows(&db, before, "workflow:evaluation");
    assert_eq!(evaluation.len(), 1);
    assert_eq!(evaluation[0]["calls"], 24);
    assert_clean(&evaluation[0].to_string(), "the evaluation row");
}

#[test]
fn only_compare_may_run_live() {
    let fx = Fixture::new("catalog-live.toml");
    let fake = Fake::start(Duration::ZERO, right);
    let input = example("invoice-evidence-missing.json");
    let cases = example("invoice-evidence-live-cases.json");
    let (input, cases) = (input.to_str().unwrap(), cases.to_str().unwrap());
    let digest = fx.candidate["digest"].as_str().unwrap();
    let deployed = fx.deployments();
    let db = audit(fx.dir());
    let before = watermark(&db);

    for args in [
        ["workflow", "run", "invoice-evidence", input],
        ["workflow", "eval", "invoice-evidence", cases],
        ["workflow", "apply", "candidate.json", digest],
    ] {
        let out = live(fx.dir(), &fake, &args, Some("6")).output().unwrap();
        assert_refused(&out, "supported only by `workflow compare`");
        assert!(
            stdout_records(&out).is_empty(),
            "no result records: {out:?}"
        );
    }
    assert_eq!(fake.calls(), 0);
    assert_eq!(rows(&db, before, "infer:%").len(), 0);
    assert_eq!(rows(&db, before, "workflow:activated").len(), 0);
    assert_eq!(
        fx.deployments(),
        deployed,
        "apply must not deploy under live"
    );
}
