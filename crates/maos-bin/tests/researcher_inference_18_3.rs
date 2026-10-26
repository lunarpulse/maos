#![forbid(unsafe_code)]

//! Story 18-3 — the process-level witness (AC1.6, AC4.5, AC5): the real
//! `maos` binary (`env!("CARGO_BIN_EXE_maos")`, no nested cargo) over a
//! scratch `HOME`, a cleared environment and no provider key —
//! `MAOS_OLLAMA_URL` unset, exactly as `epic-18-exit` runs exit line 3.
//!
//! - Exit line 3: the committed strict cassette replays one completion per
//!   seed claim through the kernel adapter, the TL `infer:` frames and the
//!   one `ProviderInfer` issuance name `anthropic` (D-18-3-02 B), the root
//!   prints `researcher_survey`, the seed's contradiction writes an
//!   `epistemic.halt` (D-18-3-09 A) and the distillate is a TL row (AC4.1).
//! - Empty, drifted and short cassettes and the legacy (mode-unset) arm over
//!   an empty cassette exit 1, the cause named ON the root's refusal line
//!   (review TIA-3: the replay port's own print does not count).
//! - Mode unset stays deterministic; an unregistered declared provider
//!   refuses at boot and mints nothing; a `--live` pass with no MCP server
//!   refuses typed, never surveying the seed (review ruling D-18-3-R2).
//! - A serving root binds a door-loaded Researcher like the boot arm, under
//!   explicit replay and under the legacy cassette arm (D-18-3-04 A; review
//!   BH-1); the root's binding mints once per pass on ONE instance
//!   (D-18-3-03 A; review TIA-2, library level).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{
    InferenceError, InferenceRequest, InferenceResponse, ProviderAttribution, StopReason,
    TokenUsage,
};
use maos_domain::ports::InferencePort;
use maos_kernel_core::iac::distillate::DistillateWriter;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_spirit_abi::ctx::{CapabilityHandle, Ctx, MailboxHandle};
use maos_spirit_abi::lifecycle::Spirit as _;
use serde_json::Value;

const MANIFEST: &str = "spirits/researcher/manifest.toml";
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/survey-distill.json";
const BUDGET: Duration = Duration::from_secs(45);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn command(binary: PathBuf, scratch: &Path) -> Command {
    let mut cmd = Command::new(binary);
    cmd.current_dir(workspace_root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", scratch)
        .env("MAOS_HOME", scratch.join("home"))
        .env("XDG_DATA_HOME", scratch.join("xdg"))
        // Review TIA-6: never the unix default keyring (a desktop key would
        // make the zero-config cases a paid live call).
        .env("MAOS_SECRETS_BACKEND", "env");
    cmd
}

fn maos(scratch: &Path) -> Command {
    command(PathBuf::from(env!("CARGO_BIN_EXE_maos")), scratch)
}

/// `maos run <researcher> --once` with `env`.
fn once(scratch: &Path, extra: &[&str], env: &[(&str, &str)]) -> Output {
    maos(scratch)
        .args(["run", MANIFEST, "--once"])
        .args(extra)
        .envs(env.iter().copied())
        .output()
        .expect("run maos")
}

/// Strict explicit replay of `cassette` (exit line 3's environment).
fn replay(scratch: &Path, cassette: &str) -> Output {
    once(
        scratch,
        &[],
        &[
            ("MAOS_INFERENCE_MODE", "replay"),
            ("MAOS_REPLAY_STRICT", "1"),
            ("MAOS_REPLAY_CASSETTE", cassette),
        ],
    )
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn events(out: &Output) -> Vec<Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Every TL row as (kind, intent, payload), read-only.
fn tl_rows(scratch: &Path) -> Vec<(i64, String, Vec<u8>)> {
    let db = scratch.join("home/audit/transparency.sqlite");
    if !db.exists() {
        return Vec::new();
    }
    let conn = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open the TL read-only");
    // The daemon creates the table after the file: an early read sees none.
    let Ok(mut statement) =
        conn.prepare("SELECT kind, intent, payload_redacted FROM transparency_log ORDER BY rowid")
    else {
        return Vec::new();
    };
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get::<_, Option<Vec<u8>>>(2)?.unwrap_or_default(),
            ))
        })
        .expect("query the TL");
    rows.map(|row| row.expect("TL row")).collect()
}

fn infer_intents(scratch: &Path) -> Vec<String> {
    tl_rows(scratch)
        .into_iter()
        .filter(|(_, intent, _)| intent.starts_with("infer:"))
        .map(|(_, intent, _)| intent)
        .collect()
}

/// `ProviderInfer` issuance audit payloads.
fn provider_infer_issuances(scratch: &Path) -> Vec<String> {
    tl_rows(scratch)
        .into_iter()
        .filter(|(_, intent, _)| intent.starts_with("cap.issue"))
        .map(|(_, _, payload)| String::from_utf8_lossy(&payload).into_owned())
        .filter(|payload| payload.contains("ProviderInfer"))
        .collect()
}

/// A variant of the committed cassette in `dir`.
fn variant(dir: &Path, edit: impl FnOnce(&mut Value)) -> String {
    let mut cassette: Value =
        serde_json::from_str(&std::fs::read_to_string(workspace_root().join(CASSETTE)).unwrap())
            .unwrap();
    edit(&mut cassette);
    let path = dir.join("variant.json");
    std::fs::write(&path, serde_json::to_vec(&cassette).unwrap()).unwrap();
    path.display().to_string()
}

#[test]
fn exit_line_3_replays_one_completion_per_seed_claim_through_the_declared_provider() {
    let scratch = tempfile::tempdir().unwrap();
    let out = replay(scratch.path(), CASSETTE);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(
        err.contains("researcher inference seam wired (provider `anthropic`)"),
        "{err}"
    );
    let survey = events(&out)
        .into_iter()
        .find(|e| e["event"] == "researcher_survey")
        .unwrap_or_else(|| panic!("no researcher_survey event; stderr:\n{err}"));
    for section in [
        "findings",
        "open_questions",
        "confidence_map",
        "bibliography",
    ] {
        assert!(survey.get(section).is_some(), "missing {section}: {survey}");
    }
    let entries = 3;
    assert_eq!(survey["completions_consumed"], entries);
    assert_eq!(survey["findings"].as_array().unwrap().len(), entries);

    // AC5.1 — survey and 18-7 authoring share the pass token and provider.
    let infers = infer_intents(scratch.path());
    assert_eq!(infers.len(), entries + 1, "{infers:?}");
    assert!(
        infers.iter().all(|i| i.starts_with("infer:anthropic->")),
        "{infers:?}"
    );
    let issued = provider_infer_issuances(scratch.path());
    assert_eq!(issued.len(), 1, "one mint for the pass: {issued:?}");
    assert!(issued[0].contains("anthropic"), "{issued:?}");

    // AC4.5 / AC4.1 — the contradiction halts in the kernel; the distillate
    // is written through the I11 writer.
    let rows = tl_rows(scratch.path());
    assert!(
        rows.iter()
            .any(|(kind, intent, _)| *kind == 3 && intent == "methodology_conflict"),
        "no epistemic.halt row for methodology_conflict"
    );
    assert!(
        rows.iter().any(|(kind, _, _)| *kind == 11),
        "no distillate row"
    );
    assert!(events(&out)
        .iter()
        .any(|e| e["event"] == "halt" && e["render"] == "halted on methodology_conflict"));
}

#[test]
fn survey_cassette_failures_are_fatal_but_authoring_exhaustion_preserves_output() {
    let scratch = tempfile::tempdir().unwrap();
    let cases: [(&str, fn(&mut Value), &str); 3] = [
        (
            "empty",
            |c| c["entries"] = Value::Array(Vec::new()),
            "exhausted",
        ),
        (
            "drifted",
            |c| c["entries"][1]["prompt_sha256"] = "ab".repeat(32).into(),
            "drift",
        ),
        (
            "short",
            |c| {
                c["entries"].as_array_mut().unwrap().pop();
            },
            "exhausted",
        ),
    ];
    for (name, edit, cause) in cases {
        let home = scratch.path().join(name);
        std::fs::create_dir_all(&home).unwrap();
        let cassette = variant(&home, edit);
        let out = replay(&home, &cassette);
        let err = stderr(&out);
        assert_eq!(
            out.status.code(),
            Some(if name == "short" { 0 } else { 1 }),
            "{name}: {err}"
        );
        assert!(
            refused_naming(&err, cause),
            "{name}: the refusal line must name `{cause}`: {err}"
        );
        assert!(
            events(&out)
                .iter()
                .any(|e| e["event"] == "researcher_survey")
                == (name == "short"),
            "{name}: only authoring exhaustion retains the completed survey"
        );
    }
}

/// The root's `--once` refusal line itself names `cause` (review TIA-3).
fn refused_naming(err: &str, cause: &str) -> bool {
    err.lines().any(|l| {
        (l.contains("researcher on_idle pass refused") || l.contains("skill authoring refused"))
            && l.contains(cause)
    })
}

#[test]
fn the_legacy_cassette_arm_refuses_an_exhausted_cassette_typed() {
    let scratch = tempfile::tempdir().unwrap();
    let cassette = variant(scratch.path(), |c| c["entries"] = Value::Array(Vec::new()));
    let out = once(scratch.path(), &[], &[("MAOS_REPLAY_CASSETTE", &cassette)]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("researcher cassette-replay inference wired"),
        "{err}"
    );
    assert!(refused_naming(&err, "exhausted"), "{err}");
}

#[test]
fn mode_unset_stays_deterministic_and_touches_no_model() {
    let scratch = tempfile::tempdir().unwrap();
    let out = once(scratch.path(), &[], &[]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(0), "{err}");
    assert!(err.contains("researcher deterministic survey"), "{err}");
    assert!(!events(&out)
        .iter()
        .any(|e| e["event"] == "researcher_survey"));
    assert!(infer_intents(scratch.path()).is_empty());
}

#[test]
fn an_unregistered_declared_provider_refuses_boot_and_mints_nothing() {
    let scratch = tempfile::tempdir().unwrap();
    let out = once(
        scratch.path(),
        &["--live"],
        &[("MAOS_OLLAMA_URL", "http://127.0.0.1:9")],
    );
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains(
            "researcher inference: no inference provider is configured for its declared \
             providers `anthropic`, `openai`"
        ),
        "{err}"
    );
    assert!(provider_infer_issuances(scratch.path()).is_empty());
}

#[test]
fn a_live_pass_without_an_mcp_server_is_refused_typed_never_seeded() {
    let scratch = tempfile::tempdir().unwrap();
    let out = once(scratch.path(), &["--live"], &[("MAOS_OLLAMA_URL", "skip")]);
    let err = stderr(&out);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("researcher inference seam wired (provider `anthropic`)"),
        "{err}"
    );
    assert!(refused_naming(&err, "no MCP server is configured"), "{err}");
    assert!(
        !tl_rows(scratch.path())
            .iter()
            .any(|(_, intent, _)| intent == "researcher:seed_claim"),
        "a --live pass journals no seed claim"
    );
}

// ── AC5.3 / AC5.4 — the serving root ─────────────────────────────────────

fn maosctl(scratch: &Path, args: &[&str]) -> Output {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_maos")).with_file_name("maosctl");
    assert!(
        path.is_file(),
        "maosctl not found at {} — run `cargo build -p maos-cli` first",
        path.display()
    );
    command(path, scratch)
        .args(args)
        .output()
        .expect("run maosctl")
}

fn wait_until(what: &str, diagnostics: impl Fn() -> String, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + BUDGET;
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!(
        "timed out after {BUDGET:?} waiting for {what}; stderr:\n{}",
        diagnostics()
    );
}

fn has_durable_author(scratch: &Path) -> bool {
    std::fs::read(scratch.join("home/skills/queue.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .is_some_and(|queue| {
            queue["pending"]
                .as_array()
                .is_some_and(|entries| entries.len() == 1)
        })
}

fn drain(mut pipe: impl Read + Send + 'static) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&buffer);
    std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(n) = pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    });
    buffer
}

struct Daemon(Option<Child>);

impl Drop for Daemon {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn a_serving_root_mints_per_pass_and_binds_a_door_loaded_researcher() {
    let scratch_dir = tempfile::tempdir().unwrap();
    let scratch = scratch_dir.path();
    std::fs::create_dir_all(scratch.join("home")).unwrap();
    let init = maos(scratch).arg("init").output().expect("maos init");
    assert!(init.status.success(), "{}", stderr(&init));
    // Boot consumes three survey completions and one authoring completion.
    // The pending author entry suppresses authoring on the door-loaded pass.
    let cassette = variant(scratch, |c| {
        let entries = c["entries"].as_array_mut().unwrap();
        let again = entries.clone();
        entries.extend(again.into_iter().take(3));
    });
    let mut child = maos(scratch)
        .args(["run", MANIFEST])
        .env("MAOS_IDLE_FAST", "1")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", &cassette)
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the serving root");
    let _stdout = drain(child.stdout.take().unwrap());
    let stderr_buf = drain(child.stderr.take().unwrap());
    let _daemon = Daemon(Some(child));
    let err = || stderr_buf.lock().unwrap().clone();

    wait_until("the boot pass's durable author submission", err, || {
        has_durable_author(scratch)
    });
    for args in [
        &["unload", "researcher"][..],
        &["load", MANIFEST][..],
        &["start", "researcher"][..],
    ] {
        let out = maosctl(scratch, args);
        assert!(out.status.success(), "maosctl {args:?}: {}", stderr(&out));
    }
    wait_until("the door-loaded Researcher's inference calls", err, || {
        infer_intents(scratch).len() >= 7
    });
    let infers = infer_intents(scratch);
    assert!(
        infers.iter().all(|i| i.starts_with("infer:anthropic->")),
        "{infers:?}"
    );
    // Two Researcher instances (boot, door) — one issuance each; the
    // per-pass mint on ONE instance is the library test below (TIA-2).
    let issued = provider_infer_issuances(scratch);
    assert_eq!(issued.len(), 2, "one issuance per pass: {issued:?}");
    assert_eq!(
        err()
            .matches("researcher inference seam wired (provider `anthropic`)")
            .count(),
        2,
        "the boot arm and the door load both bind: {}",
        err()
    );
}

/// Review BH-1 (D-18-3-04 A) — under the legacy (mode-unset) cassette arm a
/// door-loaded Researcher is bound to its own replay of the cassette, like
/// the boot arm, never unbound.
#[test]
fn a_legacy_cassette_root_binds_a_door_loaded_researcher() {
    let scratch_dir = tempfile::tempdir().unwrap();
    let scratch = scratch_dir.path();
    std::fs::create_dir_all(scratch.join("home")).unwrap();
    let init = maos(scratch).arg("init").output().expect("maos init");
    assert!(init.status.success(), "{}", stderr(&init));
    let mut child = maos(scratch)
        .args(["run", MANIFEST])
        .env("MAOS_IDLE_FAST", "1")
        .env("MAOS_REPLAY_STRICT", "1")
        .env("MAOS_REPLAY_CASSETTE", workspace_root().join(CASSETTE))
        .env("MAOS_NOTIFY_DISABLE", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the serving root");
    let _stdout = drain(child.stdout.take().unwrap());
    let stderr_buf = drain(child.stderr.take().unwrap());
    let _daemon = Daemon(Some(child));
    let err = || stderr_buf.lock().unwrap().clone();

    wait_until("the boot pass's durable author submission", err, || {
        has_durable_author(scratch)
    });
    for args in [
        &["unload", "researcher"][..],
        &["load", MANIFEST][..],
        &["start", "researcher"][..],
    ] {
        let out = maosctl(scratch, args);
        assert!(out.status.success(), "maosctl {args:?}: {}", stderr(&out));
    }
    wait_until("the door-loaded Researcher's issuance", err, || {
        provider_infer_issuances(scratch).len() >= 2
    });
    assert_eq!(
        err()
            .matches("researcher cassette-replay inference wired")
            .count(),
        2,
        "the boot arm and the door load both bind the cassette: {}",
        err()
    );
    assert!(!err().contains("pass refused"), "{}", err());
}

/// Answers every synthesis prompt (the mint is what is counted).
struct Echo;

impl InferencePort for Echo {
    fn complete(&self, req: InferenceRequest) -> Result<InferenceResponse, InferenceError> {
        Ok(InferenceResponse {
            text: format!("Digest of {} bytes.", req.prompt.len()),
            stop_reason: StopReason::StopSequence,
            usage: TokenUsage {
                input_tokens: 1,
                output_tokens: 1,
            },
            provider_attribution: ProviderAttribution {
                provider_id: "anthropic".into(),
                endpoint_url: "stub://".into(),
                model_id: None,
            },
        })
    }
}

/// Review TIA-2 (D-18-3-03 A, AC5.3) — the root's production binding
/// (`researcher_root::bind`) mints once per `on_idle` pass on ONE instance;
/// an admission-time token would mint once for both passes.
#[test]
fn the_root_binding_mints_once_per_pass_on_one_instance() {
    let mints = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&mints);
    let seam = butler::ButlerSeam {
        inference: None,
        mint: Arc::new(move |pid, _provider: &str| {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(CapabilityToken::new(
                TokenId([7; 16]),
                pid,
                u64::MAX,
                [0; 64],
            ))
        }),
        digest: Arc::new(butler::MorningDigestRenderer::new(
            PathBuf::new(),
            PathBuf::new(),
            Arc::new(|_, _| {}),
            Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
        )),
        rollover: false,
        clock: Arc::new(maos_spirit_sdk::clock::SystemClock::UTC),
        acceptance: None,
        scene_butler: Arc::default(),
        context: None,
    };
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(183));
    let host = maos_bin::researcher_root::ResearcherHost::new(
        Arc::clone(&tl),
        Arc::new(DistillateWriter::new(Arc::clone(&tl), Arc::new(0u8))),
        false,
        Arc::new(maos_kernel_core::iac::log_recall::LogRecallAdapter::new(
            Arc::clone(&tl),
        )),
    );
    let researcher = maos_bin::researcher_root::bind(
        researcher::Researcher::new(),
        Arc::new(Echo),
        "anthropic".into(),
        &seam,
        Arc::new(|| Some(7)),
        &host,
    );
    for pass in 1..=2 {
        researcher.on_idle(&mut Ctx::for_rust_inproc_hook(
            CapabilityHandle(0),
            MailboxHandle(0),
        ));
        assert!(
            matches!(
                researcher.take_pass_error(),
                Some(researcher::ResearcherError::SkillAuthor(
                    researcher::SkillPortError::QueueUnavailable
                ))
            ),
            "pass {pass} refused"
        );
        assert!(researcher.last_output().is_some(), "pass {pass} surveyed");
        assert_eq!(mints.load(Ordering::SeqCst), pass, "one mint per pass");
    }
}
