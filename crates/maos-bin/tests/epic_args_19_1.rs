#![forbid(unsafe_code)]

//! Story 19-1 (AC1.2) — `maos run … --epic <file>` parse vectors, driven
//! through the production `parse_run_args` (in-`src` tests are kloc-charged).
//! The run-time refusals (a manifest with no host-bearing entry, the
//! cross-host arm, a loopback goal, several Workers without `--epic`) and the
//! dispatch stop are driven through the production `orchestrator_root` here;
//! the spawned vectors below fire before any Worker exists, so none needs the
//! T3 substrate. The epic run itself is end to end in `epic_dispatch_19_1.rs`.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use maos_bin::orchestrator_root::{admitted_pid, dispatch_plan, dispatch_stop, load_epic};
use maos_bin::worker_spawn::{parse_run_args, RunArgs};
use maos_kernel_core::halt::HaltRegistry;

const EPIC_TOPOLOGY: &str = "spirits/topologies/j1-founder-loop-epic.toml";
const EPIC: &str = "tests/fixtures/j1/epic.md";
const ORCHESTRATOR: &str = "spirits/orchestrator/manifest.toml";
/// Any well-formed cassette: the Orchestrator's port is built, never called.
const CASSETTE: &str = "crates/maos-journey-test/cassettes/j-researcher/survey-distill.json";

fn parse(args: &[&str]) -> Result<Option<RunArgs>, String> {
    parse_run_args(args.iter().map(|arg| (*arg).to_owned()))
}

fn epic_fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/j1/epic.md")
}

fn workspace_root() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/../..")
}

/// A fresh, self-removing HOME / MAOS_HOME / XDG_DATA_HOME per run.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(tag: &str) -> Scratch {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "maos-19-1-args-{tag}-{nanos}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).expect("create scratch home");
    Scratch(path)
}

/// `maos run <args> --once` from the workspace root, in a clean environment
/// plus `env`, on `home`.
fn run_once(home: &Scratch, args: &[&str], env: &[(&str, OsString)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .env_clear()
        .arg("run")
        .args(args)
        .arg("--once")
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", &home.0)
        .env("MAOS_HOME", &home.0)
        .env("XDG_DATA_HOME", &home.0)
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_NOTIFY_DISABLE", "1");
    for (key, value) in env {
        command.env(key, value);
    }
    command
        .current_dir(workspace_root())
        .output()
        .expect("execute maos")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The names of the root's stdout events, in order.
fn event_names(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|event| event["event"].as_str().map(str::to_owned))
        .collect()
}

/// Transparency Log rows matching `predicate` (SQL over `kind`/`intent`); a
/// run refused before the log was created has none.
fn tl_count(home: &Scratch, predicate: &str) -> i64 {
    let db = home.0.join("audit").join("transparency.sqlite");
    if !db.exists() {
        return 0;
    }
    let connection = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .expect("open transparency log");
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM transparency_log WHERE {predicate}"),
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
}

/// Asserts the run failed with `needle` on stderr before any topology entry
/// loaded or any `task.assign` (TL kind 0) was written.
fn assert_refused_before_any_entry(home: &Scratch, output: &Output, needle: &str) {
    let stderr = stderr(output);
    assert!(
        !output.status.success(),
        "the run must fail; stderr:\n{stderr}"
    );
    assert!(stderr.contains(needle), "stderr:\n{stderr}");
    let events = event_names(output);
    assert!(
        !events
            .iter()
            .any(|name| name == "spirit_loaded" || name == "delegation_routed"),
        "nothing loads or emits before the refusal: {events:?}"
    );
    assert_eq!(tl_count(home, "kind = 0"), 0, "no task.assign row");
}

#[test]
fn epic_flag_carries_its_file_beside_once() {
    let run = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
        "tests/fixtures/j1/epic.md",
        "--once",
    ])
    .expect("a valid --epic run parses")
    .expect("a `run` invocation");
    assert_eq!(
        run,
        RunArgs {
            manifest_path: "spirits/topologies/j1-founder-loop-epic.toml".into(),
            live: false,
            once: true,
            scene: None,
            epic: Some("tests/fixtures/j1/epic.md".into()),
        }
    );
}

#[test]
fn no_epic_flag_leaves_epic_unset() {
    let run = parse(&["run", "spirits/topologies/j1-founder-loop.toml", "--once"])
        .expect("parses")
        .expect("a `run` invocation");
    assert_eq!(run.epic, None, "without --epic the run is today's run");
}

#[test]
fn epic_without_a_value_is_refused() {
    let error = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
    ])
    .expect_err("--epic with no value must refuse");
    assert!(
        error.contains("--epic needs a file"),
        "the refusal names the missing value: {error}"
    );
}

#[test]
fn epic_followed_by_a_flag_is_refused_not_swallowed() {
    let error = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
        "--once",
    ])
    .expect_err("--epic must not take the next flag as its file");
    assert!(error.contains("--epic needs a file"), "{error}");
}

#[test]
fn epic_given_twice_is_refused() {
    let error = parse(&[
        "run",
        "spirits/topologies/j1-founder-loop-epic.toml",
        "--epic",
        "a.md",
        "--epic",
        "b.md",
    ])
    .expect_err("two epics must refuse");
    assert!(error.contains("--epic is given twice"), "{error}");
}

#[test]
fn epic_with_a_butler_scene_is_refused() {
    for scene in [
        &["--clock", "compressed:3d", "--acceptance", "fixture.toml"][..],
        &["--clock", "compressed:3d"][..],
        &["--acceptance", "fixture.toml"][..],
    ] {
        let mut args = vec!["run", "spirits/butler/manifest.toml", "--epic", "e.md"];
        args.extend_from_slice(scene);
        let error = parse(&args).expect_err("--epic with --clock/--acceptance must refuse");
        assert!(
            error.contains("--epic") && error.contains("--clock/--acceptance"),
            "the refusal names both flags for {scene:?}: {error}"
        );
    }
}

#[test]
fn usage_lists_the_epic_flag() {
    let error = parse(&["run", "m.toml", "--bogus"]).expect_err("unknown flag refuses");
    assert!(
        error.contains("[--epic <file>]"),
        "the usage line names --epic: {error}"
    );
}

#[test]
fn epic_on_a_topology_with_no_host_bearing_entry_is_refused() {
    let error = load_epic(&epic_fixture(), 0, false)
        .expect_err("no host-bearing entry has no Worker to bind a section to");
    assert!(error.contains("host-bearing"), "{error}");
}

#[test]
fn epic_on_the_cross_host_arm_is_refused_before_reading_the_file() {
    let error = load_epic(std::path::Path::new("/nonexistent/epic.md"), 3, true)
        .expect_err("--epic on the cross-host arm must refuse");
    assert!(
        error.contains("loopback arm only") && error.contains("MAOS_DELEGATED_GOAL"),
        "{error}"
    );
}

#[test]
fn the_fixture_epic_splits_into_three_sections_for_three_workers() {
    let sections = load_epic(&epic_fixture(), 3, false).expect("three sections, three entries");
    assert_eq!(
        sections.iter().map(|s| s.ordinal).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(sections.iter().all(|s| s.text.starts_with("### Story")));
}

#[test]
fn a_count_mismatch_is_refused_naming_both_counts() {
    let error = load_epic(&epic_fixture(), 2, false).expect_err("3 sections vs 2 entries");
    assert!(
        error.contains("3 `### Story` sections vs 2 host-bearing entries"),
        "{error}"
    );
}

#[test]
fn a_loopback_goal_is_refused_when_set_non_blank_or_non_utf8_and_ignored_when_blank() {
    let refused = |goal: OsString| {
        dispatch_plan(None, 1, false, Some(goal.as_os_str()))
            .expect_err("a set loopback goal refuses")
    };
    assert!(refused("write a haiku".into()).contains("--epic"));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let error = refused(OsString::from_vec(b"write a haiku \xff".to_vec()));
        assert!(error.contains("MAOS_DELEGATED_GOAL"), "{error}");
    }
    assert!(dispatch_plan(None, 1, false, Some(OsStr::new(" \t")))
        .expect("a blank goal is unset")
        .is_none());
    assert!(dispatch_plan(None, 1, true, Some(OsStr::new("goal")))
        .expect("the cross-host arm reads the goal per frame")
        .is_none());
}

#[test]
fn several_host_bearing_workers_without_epic_are_refused_on_loopback_only() {
    let error = dispatch_plan(None, 3, false, None).expect_err("3 Workers, no --epic");
    assert!(
        error.contains("declares 3 host-bearing Worker entries and no --epic"),
        "{error}"
    );
    assert!(dispatch_plan(None, 1, false, None)
        .expect("one Worker keeps the rehearsal")
        .is_none());
    assert!(dispatch_plan(None, 3, true, None)
        .expect("cross-host Workers take MAOS_DELEGATED_GOAL")
        .is_none());
    let sections = dispatch_plan(Some(epic_fixture().as_path()), 3, false, None)
        .expect("three sections, three Workers")
        .expect("the epic's sections");
    assert_eq!(sections.len(), 3);
}

#[test]
fn an_unloaded_orchestrator_stops_dispatch_and_an_unadmitted_one_never_dispatches() {
    let registry = HaltRegistry::new();
    assert!(dispatch_stop(&registry, 7, Some(7), 2, 3).is_none());
    let stop = dispatch_stop(&registry, 7, None, 2, 3).expect("unloaded ⇒ stop");
    assert_eq!(stop["event"], "dispatch_halted");
    assert_eq!(stop["orchestrator_unloaded"], true);
    assert_eq!(stop["dispatched_sections"], 1);
    assert_eq!(stop["undispatched_sections"], serde_json::json!([2, 3]));
    assert!(dispatch_stop(&registry, 7, Some(8), 2, 3).is_some());
    let error = admitted_pid(&std::sync::atomic::AtomicU32::new(0)).expect_err("pid 0");
    assert!(error.contains("has no admitted pid"), "{error}");
    assert_eq!(admitted_pid(&std::sync::atomic::AtomicU32::new(9)), Ok(9));
}

/// D-19-1-03 A — the shipped epic topology without `--epic` is refused before
/// any entry loads (it used to run Worker 1, then fail at FR21).
#[test]
fn an_epic_topology_without_epic_is_refused_before_any_entry_loads() {
    let home = scratch("no-epic");
    let output = run_once(&home, &[EPIC_TOPOLOGY], &[]);
    assert_refused_before_any_entry(
        &home,
        &output,
        "declares 3 host-bearing Worker entries and no --epic",
    );
}

/// D-19-1-02 A — a set non-UTF-8 loopback goal is a set goal: refused naming
/// `--epic` before any entry loads, never read as unset.
#[cfg(unix)]
#[test]
fn a_non_utf8_delegated_goal_on_loopback_is_refused_naming_epic() {
    use std::os::unix::ffi::OsStringExt;
    let home = scratch("non-utf8-goal");
    let output = run_once(
        &home,
        &["spirits/topologies/j1-founder-loop.toml"],
        &[(
            "MAOS_DELEGATED_GOAL",
            OsString::from_vec(b"write a haiku \xff".to_vec()),
        )],
    );
    assert_refused_before_any_entry(
        &home,
        &output,
        "MAOS_DELEGATED_GOAL names a cross-host goal",
    );
    assert!(stderr(&output).contains("--epic"));
}

/// D-19-1-02 A — `--epic` on the cross-host arm is refused the moment the arm
/// is decided: no `cohort:crosshost-started` row, no boot-nonce banner.
#[test]
fn epic_on_the_cross_host_arm_is_refused_before_the_boot_nonce_is_published() {
    let home = scratch("cross-host");
    let config = cohort::host_a_config(&home.0);
    let output = run_once(
        &home,
        &[
            "spirits/topologies/j1-founder-loop-crosshost.toml",
            "--epic",
            EPIC,
        ],
        &[
            ("MAOS_COHORT_DAEMON_CONFIG", config.into_os_string()),
            ("MAOS_DELEGATED_GOAL", "19-1 cross-host epic probe".into()),
        ],
    );
    let stderr = stderr(&output);
    assert!(!output.status.success(), "stderr:\n{stderr}");
    assert!(
        stderr.contains("J1 delegation router = CROSS-HOST"),
        "the cross-host arm must be the one taken; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("--epic dispatches on the loopback arm only"),
        "stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("cross-host sender ready — boot_nonce"),
        "refused before the banner; stderr:\n{stderr}"
    );
    assert_eq!(
        tl_count(&home, "intent = 'cohort:crosshost-started'"),
        0,
        "refused before the boot-nonce row"
    );
    assert!(!event_names(&output)
        .iter()
        .any(|name| name == "delegation_routed"));
}

/// D-19-1-04 A — the standalone Orchestrator is bound to the Inference Port
/// under an explicit mode, and never with the mode unset (ADR-064).
#[test]
fn the_orchestrator_inference_seam_is_wired_only_under_an_explicit_mode() {
    const SEAM: &str = "orchestrator inference seam wired (provider ";
    let home = scratch("seam-replay");
    let output = run_once(
        &home,
        &[ORCHESTRATOR],
        &[
            ("MAOS_INFERENCE_MODE", "replay".into()),
            ("MAOS_REPLAY_CASSETTE", CASSETTE.into()),
        ],
    );
    let replayed = stderr(&output);
    assert!(output.status.success(), "stderr:\n{replayed}");
    assert!(replayed.contains(SEAM), "stderr:\n{replayed}");

    let home = scratch("seam-unset");
    let output = run_once(&home, &[ORCHESTRATOR], &[]);
    let unset = stderr(&output);
    assert!(output.status.success(), "stderr:\n{unset}");
    assert!(!unset.contains(SEAM), "stderr:\n{unset}");
}

/// Host A's cohort daemon config (the `f4_pairing.rs` fixture, trimmed): real
/// minted leaves and a signed cohort manifest, so the root takes the
/// cross-host arm. Host B is never dialled.
mod cohort {
    use std::path::{Path, PathBuf};

    use ed25519_dalek::SigningKey;
    use maos_a2a_core::PeerCertFingerprint;
    use maos_bin::delegation;

    fn mint_pems(dir: &Path) -> (PathBuf, PathBuf, String) {
        std::fs::create_dir_all(dir).expect("create identity directory");
        let key = rcgen::KeyPair::generate().expect("rcgen keypair");
        let cert = rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()])
            .expect("rcgen parameters")
            .self_signed(&key)
            .expect("rcgen self-signed certificate");
        let fingerprint = PeerCertFingerprint::from_cert_der(cert.der().as_ref()).to_string();
        let (cert_path, key_path) = (dir.join("own.cert.pem"), dir.join("own.key.pem"));
        std::fs::write(&cert_path, cert.pem()).expect("write certificate pem");
        std::fs::write(&key_path, key.serialize_pem()).expect("write private key pem");
        (cert_path, key_path, fingerprint)
    }

    fn signed_manifest(key: &SigningKey, a_fingerprint: &str, b_fingerprint: &str) -> String {
        use maos_cohort::{
            CohortAuthority, CohortManifest, CohortMember, ConsentMatrix, ManifestSignature,
            TeamEntry, COHORT_SCHEMA_V4, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
        };
        use maos_domain::ports::registry::SpiritId;
        use maos_domain::region::Region;
        use maos_domain::team::TeamId;

        let member = |host: &str, fingerprint: &str, team: &str| CohortMember {
            host_id: host.to_string(),
            fingerprint: fingerprint.to_string(),
            roles: vec!["worker".to_string()],
            team: Some(TeamId::new(team).expect("valid team id")),
        };
        let team = |team: &str, region: &str, datname: &str, spirit: &str| TeamEntry {
            team_id: TeamId::new(team).expect("valid team id"),
            region: Region::canonicalize(region).expect("valid region"),
            datname: datname.to_string(),
            members: vec![SpiritId::from(spirit)],
        };
        toml::to_string(
            &CohortManifest {
                schema_version: COHORT_SCHEMA_V4,
                cohort_id: "story-19-1-epic-args".to_string(),
                version: 1,
                authority: CohortAuthority {
                    threshold: 1,
                    keys: vec![hex::encode(key.verifying_key().to_bytes())],
                },
                members: vec![
                    member("host-a", a_fingerprint, "team-a"),
                    member("host-b", b_fingerprint, "team-b"),
                ],
                consent: ConsentMatrix::default(),
                reserved_intents: vec![
                    RESERVED_INTENT_REISSUE.to_string(),
                    RESERVED_INTENT_HALT_RECEIPT.to_string(),
                ],
                t_stale_secs: 120,
                teams: Some(vec![
                    team("team-a", "region-a", "maos_team_a", "spirit-a"),
                    team("team-b", "region-b", "maos_team_b", "spirit-b"),
                ]),
                signature: ManifestSignature { sig: String::new() },
                cross_team_consent: Vec::new(),
            }
            .signed_with(key),
        )
        .expect("serialize signed cohort manifest")
    }

    pub fn host_a_config(dir: &Path) -> PathBuf {
        let (a_cert, a_key, a_fingerprint) = mint_pems(&dir.join("host-a"));
        let (_, _, b_fingerprint) = mint_pems(&dir.join("host-b"));
        let authority = SigningKey::from_bytes(&[0x19; 32]);
        let manifest = dir.join("cohort.toml");
        std::fs::write(
            &manifest,
            signed_manifest(&authority, &a_fingerprint, &b_fingerprint),
        )
        .expect("write cohort manifest");
        let hex = b_fingerprint
            .strip_prefix("sha256:")
            .expect("minted fingerprint has sha256 prefix");
        let pin = format!("{{ algo = 'sha256', hex = '{hex}' }}");
        let config = format!(
            "manifest_path = '{manifest}'\nauthority_keys = ['{authority}']\n\
             local_host = 'host-a'\ncontrol_spirit = 'orchestrator'\n\n\
             [[peers]]\npeer_id = '{peer}'\nendpoint = 'tls://127.0.0.1:1'\n\
             cert_fingerprint = {pin}\nsend_allowlist = ['{intent}']\n\
             accept_allowlist = ['{intent}']\n\n[tcp]\nlisten_addr = '127.0.0.1:0'\n\
             own_cert_chain = '{cert}'\nown_private_key = '{key}'\n\
             peer_pins = [{{ peer_id = '{peer}', fingerprint = {pin}, boot_nonce = 47 }}]\n\n\
             [digest_summary]\nframes = 0\nhalts = 0\nconflicts = 0\n",
            manifest = manifest.display(),
            authority = hex::encode(authority.verifying_key().to_bytes()),
            peer = delegation::TO_HOST,
            intent = orchestrator::DELEGATION_CONSENT_INTENT,
            cert = a_cert.display(),
            key = a_key.display(),
        );
        let path = dir.join("host-a.daemon.toml");
        std::fs::write(&path, config).expect("write daemon config");
        path
    }
}
