#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]

//! Story 17-1 D-12 — the five `--network=none` scenarios of
//! `t3-escape-corpus-v0`, EXECUTED inside the verified T3 container through the
//! production bridge (`spawn_and_bridge` → `spawn_t3`), plus the M3 socket
//! custody pair (0600 refused, 0666 connects). Each blocked attempt must leave
//! exactly one kind-8 row whose `<what>` is the fixture's
//! `attempted_syscall_substring`, journaled by the bridge's confirmed write
//! (D-10) — never by the kind-21 echo.
//!
//! The other twenty scenarios are not executed here; they are listed by id
//! with their owner (`20-3a-gate-honesty`) so no reader counts them as passed.

#[path = "../../../tests/harness/t3_substrate.rs"]
mod t3_substrate;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::{fs::PermissionsExt, net::UnixListener};
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

const NETWORK_BLOCKER: &str = "--network=none";
const SPIRIT_PID: u32 = 1;
/// Linux `EACCES`: the remapped container uid may not open a 0600 socket.
const EACCES: i32 = 13;

struct Execution {
    exit: ExitCause,
    stdout: Vec<String>,
    blocks: Vec<(String, FrameOrigin)>,
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
fn the_five_network_none_scenarios_are_blocked_inside_verified_t3() {
    if !t3_substrate::require_t3("t3 network-escape executor") {
        return;
    }
    let corpus = T3EscapeCorpus::load_from(Path::new("fixtures/t3-escape-corpus-v0/"))
        .expect("t3-escape-corpus-v0 must exist");
    let (executed, labelled): (Vec<_>, Vec<_>) = corpus.scenarios.iter().partition(|scenario| {
        scenario
            .attack_payload
            .expected_runtime_blocker
            .contains(NETWORK_BLOCKER)
    });
    assert_eq!(executed.len(), 5, "five scenarios name `{NETWORK_BLOCKER}`");
    assert_eq!(labelled.len(), 20, "twenty scenarios stay unexecuted");
    for scenario in &labelled {
        println!(
            "T3-ESCAPE-UNEXECUTED {} owner=20-3a-gate-honesty",
            scenario.scenario_id
        );
    }
    for scenario in executed {
        let id = &scenario.scenario_id;
        let vector = &scenario.expected_outcome.attempted_syscall_substring;
        let run = execute(&["--maos-escape", vector], None);
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
        println!("T3-ESCAPE-EXECUTED {id} kind8=1 {evidence}");
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
