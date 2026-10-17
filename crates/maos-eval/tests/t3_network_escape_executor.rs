#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

//! Story 20-3a AC11: every executable versioned containment scenario (25 v0
//! scenarios = 24 executed + 1 withdrawn with its reason) runs through
//! verified production T3, plus the M3 socket-custody pair. Kind-8 evidence
//! comes only from the probe's observed-effect D-10 confirmed-write marker.
//! The runtime-independent identity/count check runs even on developer hosts.

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::{
    fs::PermissionsExt,
    net::{UnixListener, UnixStream},
};
use std::path::{Path, PathBuf};

use maos_domain::invariants::{i3::FrameOrigin, i9::SandboxTier};
use maos_eval::T3EscapeCorpus;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeSpawnSpec, ExitCause, T3Launch,
};
use maos_kernel_core::security::crypto::RingCryptoProvider;
use maos_kernel_core::security::manifest::{
    CliWrapperControlChannel as Control, CliWrapperStdioShape as Shape,
};
use maos_kernel_core::security::sandbox::t3::image_lock::{
    load_and_verify_lock, VerifiedImageAttestation,
};
use maos_kernel_core::security::sandbox::t3::image_verify::read_trust_anchor_pub;
use maos_kernel_core::security::sandbox::SandboxSpec;

const CORPUS: &str = "fixtures/t3-escape-corpus-v1";
// This is the execution plan, not a category/blocker-string selector. Deleting
// any executor must fail the runtime-independent exact corpus identity check.
const EXECUTORS: &[&str] = &[
    "capability_escape_cap_dac_override_003",
    "capability_escape_cap_net_raw_002",
    "capability_escape_cap_setuid_005",
    "capability_escape_cap_sys_admin_001",
    "capability_escape_cap_sys_ptrace_004",
    "filesystem_escape_etc_passwd_001",
    "filesystem_escape_home_004",
    "filesystem_escape_host_root_005",
    "filesystem_escape_proc_1_root_002",
    "filesystem_escape_var_log_maos_003",
    "network_escape_dns_resolution_003",
    "network_escape_ipv6_outbound_005",
    "network_escape_raw_socket_004",
    "network_escape_tcp_outbound_001",
    "network_escape_udp_outbound_002",
    "process_escape_fork_bomb_004",
    "process_escape_kill_pid_1_003",
    "process_escape_mount_005",
    "process_escape_proc_1_exe_002",
    "process_escape_ptrace_host_pid_001",
    "runtime_escape_docker_sock_001",
    "runtime_escape_image_pull_via_sock_003",
    "runtime_escape_podman_sock_002",
    "runtime_escape_runtime_config_rewrite_004",
];
const SPIRIT_PID: u32 = 1;
/// Linux `EACCES`: the remapped container uid may not open a 0600 socket.
const EACCES: i32 = 13;

struct Execution {
    exit: ExitCause,
    stdout: Vec<String>,
    blocks: Vec<(String, FrameOrigin)>,
}

struct HostWitness(std::process::Child);

impl HostWitness {
    fn spawn() -> Self {
        let mut witness = Self(
            std::process::Command::new("sleep")
                .arg("600")
                .spawn()
                .expect("spawn owned host PID witness"),
        );
        witness.assert_running();
        witness
    }

    fn assert_running(&mut self) {
        assert!(
            self.0.try_wait().expect("poll host witness").is_none(),
            "host witness exited"
        );
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", self.0.id()))
            .expect("live host witness stat");
        let state = stat
            .rsplit_once(") ")
            .expect("proc stat command terminator")
            .1
            .split_whitespace()
            .next()
            .expect("proc stat state");
        assert!(
            !matches!(state, "T" | "t" | "Z" | "X"),
            "host witness is not running: {state}"
        );
    }
}

impl Drop for HostWitness {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn probe_dispatch(source: &str) -> std::collections::BTreeSet<String> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '/' if chars.peek() == Some(&'/') => {
                for ch in chars.by_ref() {
                    if ch == '\n' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut depth = 1;
                while let Some(ch) = chars.next() {
                    if ch == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        depth += 1;
                    } else if ch == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                assert_eq!(depth, 0, "unterminated probe comment");
            }
            '"' => {
                let mut literal = String::from("\"");
                while let Some(ch) = chars.next() {
                    literal.push(ch);
                    if ch == '\\' {
                        literal.push(chars.next().expect("escaped character"));
                    } else if ch == '"' {
                        break;
                    }
                }
                tokens.push(literal);
            }
            ch if ch.is_ascii_alphanumeric() || ch == '_' => {
                let mut word = ch.to_string();
                while chars
                    .peek()
                    .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                {
                    word.push(chars.next().unwrap());
                }
                tokens.push(word);
            }
            ch if !ch.is_whitespace() => tokens.push(ch.to_string()),
            _ => {}
        }
    }
    let function = tokens
        .windows(2)
        .position(|pair| pair == ["fn", "escape"])
        .expect("probe escape function");
    let body = function
        + tokens[function..]
            .iter()
            .position(|token| token == "{")
            .unwrap();
    let mut depth = 0;
    let end = (body..tokens.len())
        .find(|&index| {
            match tokens[index].as_str() {
                "{" => depth += 1,
                "}" => depth -= 1,
                _ => {}
            }
            depth == 0
        })
        .expect("escape function closing brace");
    let start = body
        + tokens[body..end]
            .windows(3)
            .position(|pattern| pattern == ["match", "vector", "{"])
            .expect("escape vector dispatch")
        + 3;
    let mut vectors = std::collections::BTreeSet::new();
    let mut depth = 1;
    let mut pattern = true;
    for index in start..end {
        let token = tokens[index].as_str();
        if depth == 1 && pattern && token.starts_with('"') {
            vectors.insert(token.trim_matches('"').to_owned());
        }
        if depth == 1 && token == "=" && tokens.get(index + 1).is_some_and(|t| t == ">") {
            pattern = false;
        }
        match token {
            "{" | "(" | "[" => depth += 1,
            "}" | ")" | "]" => depth -= 1,
            "," if depth == 1 => pattern = true,
            _ => {}
        }
        if depth == 0 {
            break;
        }
        if token == "}" && depth == 1 {
            pattern = true;
        }
    }
    vectors
}

fn assert_probe_dispatch(source: &str, expected: &std::collections::BTreeSet<&str>) {
    let actual = probe_dispatch(source);
    let actual: std::collections::BTreeSet<_> = actual.iter().map(String::as_str).collect();
    assert_eq!(
        &actual, expected,
        "probe match arms must equal executable corpus vectors"
    );
}

#[test]
fn dispatch_accounting_rejects_removed_arm_retained_in_comment() {
    let source = include_str!("../test-fixtures/t3-network-escape-probe/src/main.rs");
    let expected = probe_dispatch(source);
    let expected: std::collections::BTreeSet<_> = expected.iter().map(String::as_str).collect();
    let arm = "\"container.escape.network.raw_socket\" => raw_icmp()?,";
    assert!(source.contains(arm));
    let mutated = source.replacen(arm, &format!("// {arm}"), 1);
    assert!(std::panic::catch_unwind(|| assert_probe_dispatch(&mutated, &expected)).is_err());
}

#[test]
fn dispatch_accounting_handles_alternations_and_stops_at_match_end() {
    let source = r#"fn escape() { match vector {
        "first" | "second" => { nested("not-an-arm"); }
        "third" => call(),
        other => fail(),
    }; match other { "not-a-vector" => () } }"#;
    assert_probe_dispatch(source, &["first", "second", "third"].into_iter().collect());
}

fn probe_binary() -> PathBuf {
    std::env::var_os("MAOS_T3_NETWORK_ESCAPE_PROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            Path::new(env!("CARGO_MANIFEST_DIR")).join(
                "test-fixtures/t3-network-escape-probe/target/x86_64-unknown-linux-gnu/release/t3-network-escape-probe",
            )
        })
}

fn verified_image() -> VerifiedImageAttestation {
    let anchor = read_trust_anchor_pub().expect("provisioned public anchor");
    load_and_verify_lock(&anchor, &RingCryptoProvider)
        .and_then(|lock| lock.default_entry())
        .expect("provisioned signed image lock")
}

fn execute(arguments: &[&str], egress_socket: Option<&Path>) -> Execution {
    let probe = probe_binary();
    assert!(
        probe.is_file(),
        "static probe missing at {}: build it with RUSTFLAGS='-C target-feature=+crt-static' \
         cargo build --locked --release --target x86_64-unknown-linux-gnu --manifest-path \
         crates/maos-eval/test-fixtures/t3-network-escape-probe/Cargo.toml",
        probe.display()
    );
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let prefix = vec![arguments[0].to_owned()];
    let state_dir = tempfile::tempdir().expect("fresh private T3 state directory");
    std::fs::set_permissions(state_dir.path(), std::fs::Permissions::from_mode(0o700))
        .expect("restrict T3 state directory");
    let spec = BridgeSpawnSpec {
        program: probe.display().to_string(),
        sandbox: SandboxSpec::new_for_test(SandboxTier::T3),
        expected_argv_prefix_hash: argv_prefix_hash(&prefix),
        argv_prefix: prefix,
        task_args: arguments[1..].iter().map(|arg| (*arg).to_owned()).collect(),
        from_spirit_id: "t3-escape-executor".into(),
        stdio_shape: Shape::Raw,
        control_channel: Control::Signals,
        shutdown_signal: None,
        channel_capacity: 64,
        backpressure: Backpressure::Block,
        env: vec![],
        t3: Some(T3Launch {
            image: verified_image(),
            boot_nonce: 17_12,
            container_name: format!("maos-t3-escape-{}-{nonce}", std::process::id()),
            egress_socket: egress_socket.map(Path::to_path_buf),
            state_dir: state_dir.path().to_path_buf(),
            payload_pidfd: true,
            launch_custody: None,
        }),
    };
    let mut bridge = spawn_and_bridge(spec)
        .expect("a declared-capable runner must launch the probe under verified T3");
    let journal = TransparencyLogAdapter::open_in_memory(17_12);
    bridge.pump_to_journal(&journal, SPIRIT_PID, "", "t3-network-escape-probe", &[]);
    let exit = bridge.wait_and_finalize(&journal, SPIRIT_PID, None, |_| {});
    let rows = journal
        .query_frames(FrameFilter::default())
        .expect("query the executor journal");
    let stdout = rows
        .iter()
        .filter(|row| row.kind == FrameKind::CliSubprocessOutput)
        .filter_map(|row| serde_json::from_slice::<serde_json::Value>(&row.payload_redacted).ok())
        .filter(|payload| payload["stream"] == "stdout")
        .filter_map(|payload| payload["line"].as_str().map(str::to_owned))
        .collect();
    let blocks = rows
        .iter()
        .filter(|row| row.kind == FrameKind::SandboxBlock)
        .map(|row| (row.intent.clone(), row.origin))
        .collect();
    Execution {
        exit: exit.cause,
        stdout,
        blocks,
    }
}

#[test]
fn executor_accounting_covers_every_versioned_scenario_without_a_runtime() {
    let corpus = T3EscapeCorpus::load_from(Path::new(CORPUS)).expect("versioned corpus");
    let expected: std::collections::BTreeSet<_> = corpus
        .scenarios
        .iter()
        .map(|s| s.scenario_id.as_str())
        .collect();
    let v0 = T3EscapeCorpus::load_from(Path::new("fixtures/t3-escape-corpus-v0"))
        .expect("v0 corpus the v1 methodology classifies");
    let declared: std::collections::BTreeSet<_> = v0
        .scenarios
        .iter()
        .map(|s| s.scenario_id.as_str())
        .collect();
    let actual: std::collections::BTreeSet<_> = EXECUTORS.iter().copied().collect();
    assert_eq!(
        actual.len(),
        EXECUTORS.len(),
        "duplicate executors forbidden"
    );
    assert_eq!(actual, expected, "each scenario needs its own executor");
    let attestation: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(CORPUS).join("methodology-attestation.json"))
            .expect("v1 methodology"),
    )
    .expect("valid methodology");
    let classified: std::collections::BTreeSet<_> = attestation["classifications"]
        .as_array()
        .expect("per-scenario classification")
        .iter()
        .map(|row| row["id"].as_str().expect("classification id"))
        .collect();
    assert_eq!(
        classified, declared,
        "every v0 expectation must be classified"
    );
    // A withdrawal is named, carries its reason, and has no executor; together
    // with the executed set it accounts for every declared v0 scenario.
    let withdrawn: std::collections::BTreeSet<_> = attestation["withdrawn"]
        .as_array()
        .expect("withdrawn list")
        .iter()
        .map(|row| {
            assert!(
                row["reason"].as_str().is_some_and(|r| !r.trim().is_empty()),
                "withdrawal without a recorded reason: {row}"
            );
            row["id"].as_str().expect("withdrawn id")
        })
        .collect();
    assert!(
        withdrawn.is_disjoint(&expected),
        "a withdrawn scenario cannot stay executable"
    );
    assert_eq!(
        expected
            .union(&withdrawn)
            .copied()
            .collect::<std::collections::BTreeSet<_>>(),
        declared,
        "{} declared scenarios must be executed or withdrawn by name",
        declared.len()
    );
    assert_eq!(
        attestation["scenario_count"].as_u64(),
        Some(expected.len() as u64)
    );
    // Read the actual probe dispatch, not a synthetic fixture: every declared
    // vector must have an implementation before a capable runner can claim it.
    let probe = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test-fixtures/t3-network-escape-probe/src/main.rs"),
    )
    .expect("real probe source");
    let vectors: std::collections::BTreeSet<_> = corpus
        .scenarios
        .iter()
        .map(|scenario| {
            scenario
                .expected_outcome
                .attempted_syscall_substring
                .as_str()
        })
        .collect();
    assert_probe_dispatch(&probe, &vectors);
    for scenario in &corpus.scenarios {
        let vector = &scenario.expected_outcome.attempted_syscall_substring;
        assert_eq!(
            scenario.attack_payload.command,
            vec![
                "/maos/spirit",
                "--maos-escape",
                vector,
                "$HOST_SENTINEL",
                "$HOST_PID"
            ]
        );
        assert!(scenario.expected_outcome.block_observed);
    }
    println!(
        "T3-ESCAPE-ACCOUNTED executors={} classified={} withdrawn={}",
        EXECUTORS.len(),
        classified.len(),
        withdrawn.len()
    );
}

#[test]
fn all_executable_scenarios_are_blocked_inside_verified_t3() {
    if !t3_substrate::require_t3("every executable escape-corpus-v1 scenario") {
        return;
    }
    let corpus = T3EscapeCorpus::load_from(Path::new(CORPUS)).expect("versioned corpus");
    let mut witness = HostWitness::spawn();
    let host_pid = witness.0.id().to_string();
    for id in EXECUTORS {
        let scenario = corpus
            .scenarios
            .iter()
            .find(|s| s.scenario_id == *id)
            .unwrap_or_else(|| panic!("executor {id} lacks a scenario"));
        let vector = &scenario.expected_outcome.attempted_syscall_substring;
        // A real host-side file, deliberately NOT mounted in the container.
        // Its existence and unchanged contents are checked around every attack.
        // The home scenario keeps its witness under the host `$HOME`: with
        // ENOENT-only acceptance, a mounted but non-traversable home surfaces
        // as EACCES and fails rather than passing.
        let directory = if *id == "filesystem_escape_home_004" {
            tempfile::tempdir_in(std::env::var_os("HOME").expect("host HOME"))
        } else {
            tempfile::tempdir()
        }
        .expect("host-only sentinel directory");
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o755))
            .expect("host sentinel directory must be traversable");
        let sentinel = directory.path().join(match *id {
            "filesystem_escape_var_log_maos_003" => "kernel.log",
            "filesystem_escape_home_004" => "home-secret",
            "runtime_escape_runtime_config_rewrite_004" => "containers.conf",
            _ => "host-secret",
        });
        let secret = format!("host-only-{id}-{}", std::process::id());
        std::fs::write(&sentinel, &secret).expect("create real host sentinel");
        std::fs::set_permissions(&sentinel, std::fs::Permissions::from_mode(0o644))
            .expect("host sentinel must be readable without DAC privilege");
        assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), secret);
        let socket_witness = matches!(
            *id,
            "runtime_escape_docker_sock_001"
                | "runtime_escape_podman_sock_002"
                | "runtime_escape_image_pull_via_sock_003"
        );
        let socket = directory.path().join("runtime.sock");
        let listener = socket_witness.then(|| {
            let listener = UnixListener::bind(&socket).expect("owned runtime socket witness");
            std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666))
                .expect("runtime witness accessible without DAC privilege");
            UnixStream::connect(&socket).expect("host runtime socket positive control");
            listener
        });
        witness.assert_running();
        let mut arguments = scenario.attack_payload.command[1..].to_vec();
        for argument in &mut arguments {
            if argument == "$HOST_SENTINEL" {
                let target = if socket_witness { &socket } else { &sentinel };
                *argument = target.to_str().expect("UTF-8 sentinel path").to_owned();
            } else if argument == "$HOST_PID" {
                *argument = host_pid.clone();
            }
        }
        let borrowed: Vec<_> = arguments.iter().map(String::as_str).collect();
        let run = execute(&borrowed, None);
        if let Some(listener) = listener {
            UnixStream::connect(&socket).expect("host runtime socket positive control held");
            drop(listener);
        }
        witness.assert_running();
        assert_eq!(
            std::fs::read_to_string(&sentinel).expect("host sentinel survives"),
            secret,
            "{id}: host file was modified"
        );
        assert_eq!(
            run.exit,
            ExitCause::Exited { code: 0 },
            "{id}: the attempt escaped or failed unexpectedly; stdout {:?}",
            run.stdout
        );
        let blocked = format!("T3-ESCAPE-BLOCKED {vector} errno=");
        let evidence = run
            .stdout
            .iter()
            .find(|line| line.starts_with(&blocked))
            .unwrap_or_else(|| panic!("{id}: no blocked attempt reported: {:?}", run.stdout));
        assert_eq!(
            run.blocks,
            vec![(format!("sandbox.block.{vector}"), FrameOrigin::Kernel)],
            "{id}: exactly one confirmed kind-8 row with the corpus vocabulary"
        );
        println!("T3-ESCAPE-EXECUTED {id} block_observed=true kind8=1 {evidence}");
    }
}

#[test]
fn the_egress_socket_mount_refuses_0600_and_connects_0666() {
    if !t3_substrate::require_t3("t3 egress socket custody") {
        return;
    }
    let directory = tempfile::tempdir().expect("socket directory");
    let socket = directory.path().join("egress.sock");
    let listener = UnixListener::bind(&socket).expect("host egress socket");
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let mut request = String::new();
            let reader = stream.try_clone().expect("socket clone");
            if BufReader::new(reader).read_line(&mut request).is_ok() && request == "PING\n" {
                let _ = stream.write_all(b"PONG\n");
            }
        }
    });

    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
    let private = execute(&["--maos-socket-reach"], Some(&socket));
    assert_eq!(private.exit, ExitCause::Exited { code: 0 });
    assert!(
        private
            .stdout
            .contains(&format!("T3-SOCKET-REFUSED errno={EACCES}")),
        "a 0600 socket must refuse the remapped container uid: {:?}",
        private.stdout
    );

    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666)).unwrap();
    let shared = execute(&["--maos-socket-reach"], Some(&socket));
    assert_eq!(shared.exit, ExitCause::Exited { code: 0 });
    assert!(
        shared.stdout.contains(&"T3-SOCKET-REACHABLE".to_owned()),
        "a 0666 socket must reach the host through the mount: {:?}",
        shared.stdout
    );
    assert!(private.blocks.is_empty() && shared.blocks.is_empty());
    println!("T3-NETWORK-MATRIX socket0600=refused socket0666=reachable");
}
