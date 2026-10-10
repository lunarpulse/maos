#![cfg(unix)]

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgePoll;
use maos_kernel_core::lifecycle::cli_wrapper::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeError, BridgeSpawnSpec, ExitCause,
    SubStream,
};
use maos_kernel_core::security::{
    manifest::{CliWrapperControlChannel as Control, CliWrapperStdioShape as Shape},
    sandbox::SandboxSpec,
};
use std::{process::Command, time::Duration};

fn shell(script: String, tier: SandboxTier, control: Control, shape: Shape) -> BridgeSpawnSpec {
    let prefix = vec!["-c".into()];
    BridgeSpawnSpec {
        program: "/bin/sh".into(),
        sandbox: SandboxSpec::new_for_test(tier),
        expected_argv_prefix_hash: argv_prefix_hash(&prefix),
        argv_prefix: prefix,
        task_args: vec![script],
        from_spirit_id: "boundary-worker".into(),
        stdio_shape: shape,
        control_channel: control,
        shutdown_signal: None,
        channel_capacity: 4,
        backpressure: Backpressure::Block,
        env: vec![],
        t3: None,
    }
}

#[test]
fn t3_without_verified_launch_data_never_runs_the_program() {
    let home = tempfile::tempdir().unwrap();
    let effect = home.path().join("bare-spawn-effect");
    let script = format!("printf forbidden > '{}'", effect.display());
    match spawn_and_bridge(shell(script, SandboxTier::T3, Control::Signals, Shape::Raw)) {
        Err(BridgeError::SandboxRefused(_)) => {}
        Err(error) => panic!("unexpected refusal class: {error}"),
        Ok(mut bridge) => {
            bridge.on_unload().unwrap();
            panic!("a T3 program was launched without verified container data");
        }
    }
    assert!(!effect.exists(), "the refused program must have no effect");
}

#[test]
fn all_nine_t3_postures_refuse_stdin_or_require_a_verified_launch() {
    for control in [Control::Signals, Control::StdinCommands, Control::NamedPipe] {
        for shape in [Shape::Raw, Shape::NdjsonOverStdio, Shape::JsonRpcOverStdio] {
            let unsupported = control != Control::Signals || shape == Shape::JsonRpcOverStdio;
            match spawn_and_bridge(shell("exit 0".into(), SandboxTier::T3, control, shape)) {
                Err(BridgeError::ControlUnsupported(observed, _)) if unsupported => {
                    assert_eq!(observed, control)
                }
                Err(BridgeError::SandboxRefused(_)) if !unsupported => {}
                Err(error) => panic!("wrong refusal for {control:?}/{shape:?}: {error}"),
                Ok(mut bridge) => {
                    bridge.on_unload().unwrap();
                    panic!("T3 admitted {control:?}/{shape:?} without a verified launch");
                }
            }
        }
    }
}

#[test]
fn newline_frames_accept_the_limit_and_refuse_one_byte_over() {
    for bytes in [
        maos_frame_codec::MAX_FRAME_BYTES,
        maos_frame_codec::MAX_FRAME_BYTES + 1,
    ] {
        let home = tempfile::tempdir().unwrap();
        let body = home.path().join("body");
        std::fs::File::create(&body)
            .unwrap()
            .set_len(bytes as u64)
            .unwrap();
        let script = format!("cat '{}'", body.display());
        let mut bridge =
            spawn_and_bridge(shell(script, SandboxTier::T0, Control::Signals, Shape::Raw)).unwrap();
        let observed = bridge.poll_frame(Duration::from_secs(5));
        if bytes == maos_frame_codec::MAX_FRAME_BYTES {
            match observed {
                Ok(BridgePoll::Data(SubStream::Stdout, frame)) => {
                    assert_eq!(frame.len(), bytes);
                    assert!(frame.iter().all(|byte| *byte == 0));
                }
                _ => panic!("a record at the frame limit was not delivered intact"),
            }
        } else {
            assert!(
                matches!(observed, Err(BridgeError::Io(_))),
                "an oversized record must be a framing failure, never data or clean EOF"
            );
        }
        bridge.on_unload().unwrap();
    }
}

/// A hostile Worker can stream bytes without ever sending a newline or closing
/// stdout. The reader must stop at the frame bound and refuse; an unbounded
/// reader would keep buffering and never answer.
#[test]
fn an_unterminated_oversized_record_is_refused_at_the_bound_without_waiting_for_eof() {
    let script = format!(
        "head -c {} /dev/zero; sleep 30",
        3 * maos_frame_codec::MAX_FRAME_BYTES
    );
    let mut bridge =
        spawn_and_bridge(shell(script, SandboxTier::T0, Control::Signals, Shape::Raw)).unwrap();
    let observed = match bridge.poll_frame(Duration::from_secs(5)) {
        Err(BridgeError::Io(_)) => None,
        Err(error) => Some(format!("refusal {error}")),
        Ok(BridgePoll::Data(stream, frame)) => {
            Some(format!("{stream:?} data, {} bytes", frame.len()))
        }
        Ok(_) => Some("no frame and no refusal within 5 s".into()),
    };
    bridge.on_unload().unwrap();
    assert_eq!(
        observed, None,
        "an unterminated record past the frame bound must be refused promptly"
    );
}

#[test]
fn a_plain_bridge_drops_ambient_credentials_and_keeps_its_explicit_environment() {
    const MODE: &str = "EGRESS_17_1_ISOLATED_TEST";
    if std::env::var_os(MODE).is_none() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "a_plain_bridge_drops_ambient_credentials_and_keeps_its_explicit_environment",
                "--nocapture",
            ])
            .env(MODE, "1")
            .env("CODEX_API_KEY", "public-codex-sentinel")
            .env("ANTHROPIC_API_KEY", "public-anthropic-sentinel")
            .env("MAOS_OPERATOR_ENV_SENTINEL", "public-operator-sentinel")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "isolated child-environment contract failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let script = "test -z \"${CODEX_API_KEY+x}\" && test -z \"${ANTHROPIC_API_KEY+x}\" && test -z \"${MAOS_OPERATOR_ENV_SENTINEL+x}\" && test \"${EXPLICIT_WORKER_ENV+x}\" = x";
    let mut spec = shell(script.into(), SandboxTier::T0, Control::Signals, Shape::Raw);
    spec.env
        .push(("EXPLICIT_WORKER_ENV".into(), "explicit-nonsecret".into()));
    let mut bridge = spawn_and_bridge(spec).unwrap();
    let journal = TransparencyLogAdapter::open_in_memory(0);
    bridge.pump_to_journal(&journal, 1, "", "boundary", &[]);
    let exit = bridge.wait_and_finalize(&journal, 1, None, |_| {});
    assert_eq!(
        exit.cause,
        ExitCause::Exited { code: 0 },
        "ambient credentials reached the child or the explicit environment was lost"
    );
}
