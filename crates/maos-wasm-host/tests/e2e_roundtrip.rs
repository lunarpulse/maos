//! Story 11.1a AC3 / Task 9 — End-to-end round-trip.
//!
//! `form=WasmComponent` manifest -> daemon `resolve_launch` -> real runner
//! -> full ADR-032 round-trip THROUGH the guest's real `handle-frame` export
//! (not a host-side echo). Uses `echo_spirit_component.wasm`, a real
//! `maos:spirit@1.0` component built from `guests/echo-spirit` — NOT a core
//! module, NOT a host echo loop.
//!
//! This test validates the complete path without test-doubles:
//! 1. `SpiritHostPort::resolve_launch` resolves a WasmComponent request and
//!    rejects a non-conformant one (real wasmtime probe, not metadata-only).
//! 2. The resolved plan points at the real `maos-wasm-runner` binary.
//! 3. The runner subprocess is spawned and communicates over real pipes.
//! 4. A real domain `IacFrame` round-trips through the GUEST's `handle-frame`
//!    export, byte-identical end to end (guest is an identity Spirit, so the
//!    emitted frame equals the sent frame at the domain level — but the path
//!    goes through component instantiation + a typed WIT call, not a
//!    host-side `read_frame`/`write_frame` echo).

use std::io::{BufReader, BufWriter};
use std::sync::Arc;

use maos_domain::frame::{FrameAddress, IacFrame, TaskAssignPayload};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_host::{SpiritForm, SpiritHostPort, SpiritLaunchRequest, WireShape};
use maos_spirit_abi::identity::FrameKind;
use maos_wasm_host::codec;
use maos_wasm_host::config::WasmHostConfig;
use maos_wasm_host::WasmHostAdapter;

fn component_fixture_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../tests/fixtures/wasm/echo_spirit_component.wasm")
}

fn runner_binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    path.push("maos-wasm-runner");
    path
}

/// AC3/F4 §A6: the proven-red path MUST be real. A missing runner binary is
/// a test-environment defect, not a reason to silently pass — fail loud so
/// CI catches a build-skip immediately instead of reporting a false GREEN.
fn require_runner_binary() -> std::path::PathBuf {
    let path = runner_binary_path();
    assert!(
        path.exists(),
        "AC3 requires the real maos-wasm-runner binary at {} — \
         it was not built. Run `cargo test -p maos-wasm-host` (not `--lib`) \
         so cargo builds the [[bin]] target alongside the test harness.",
        path.display()
    );
    path
}

fn make_test_frame() -> IacFrame {
    IacFrame {
        frame_id: [7u8; 16],
        timestamp_ns: 123_456_789,
        logical_clock: 1,
        from: FrameAddress {
            spirit_id: "test-spirit".into(),
            host_id: None,
            role: None,
        },
        to: smallvec::smallvec![FrameAddress {
            spirit_id: "peer-spirit".into(),
            host_id: None,
            role: None,
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: maos_domain::frame::FramePayload::TaskAssign(TaskAssignPayload {
            goal: "test round-trip through the real guest".to_string(),
            scope: vec![],
            success_criteria: "byte-identical emitted frame".to_string(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::Kernel,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

// ── AC1: resolve_launch produces correct plan ──────────────────────────

#[test]
fn resolve_launch_wasm_component_produces_runner_plan() {
    let runner_path = require_runner_binary();

    let config = Arc::new(WasmHostConfig::new(runner_path.clone(), 1_000_000));
    let adapter = WasmHostAdapter::new(config, std::time::Duration::from_secs(5));

    let request = SpiritLaunchRequest {
        form: SpiritForm::WasmComponent,
        artifact: component_fixture_path(),
        form_config: vec![],
    };

    let plan = adapter
        .resolve_launch(&request)
        .expect("a real, conformant component must resolve");

    assert_eq!(
        plan.program,
        runner_path.to_string_lossy(),
        "program must be the runner binary"
    );
    assert!(
        plan.argv.contains(&"--component".to_string()),
        "argv must contain --component"
    );
    assert!(
        plan.argv.contains(&component_fixture_path()),
        "argv must contain the component path"
    );
    assert_eq!(plan.wire, WireShape::ContentLengthCbor);
}

#[test]
fn resolve_launch_native_subprocess_is_identity() {
    let runner_path = runner_binary_path();
    let config = Arc::new(WasmHostConfig::new(runner_path, 1_000_000));
    let adapter = WasmHostAdapter::new(config, std::time::Duration::from_secs(5));

    let request = SpiritLaunchRequest {
        form: SpiritForm::NativeSubprocess,
        artifact: "/usr/bin/my-spirit".to_string(),
        form_config: vec![],
    };

    let plan = adapter.resolve_launch(&request).unwrap();

    assert_eq!(plan.program, "/usr/bin/my-spirit");
    assert!(plan.argv.is_empty());
}

/// AC3: a present-but-bad `.wasm` is rejected at `resolve_launch` time
/// (admission gate), not just at runner-spawn time. This exercises the
/// adapter's real wasmtime conformance probe, not a `std::fs::metadata`
/// existence check.
#[test]
fn resolve_launch_rejects_non_conformant_component() {
    let runner_path = runner_binary_path();
    let config = Arc::new(WasmHostConfig::new(runner_path, 1_000_000));
    let adapter = WasmHostAdapter::new(config, std::time::Duration::from_secs(5));

    let dir = tempfile::tempdir().unwrap();
    let bad_wasm = dir.path().join("bad.wasm");
    std::fs::write(&bad_wasm, b"not a valid wasm").unwrap();

    let request = SpiritLaunchRequest {
        form: SpiritForm::WasmComponent,
        artifact: bad_wasm.to_str().unwrap().to_string(),
        form_config: vec![],
    };

    let err = adapter
        .resolve_launch(&request)
        .expect_err("a non-wasm file must be rejected at admission time");
    assert!(
        matches!(err, maos_host::SpiritHostError::InvalidComponent { .. }),
        "must be the typed InvalidComponent variant, got {err:?}"
    );
}

// ── AC3: Real subprocess ADR-032 round-trip THROUGH the guest ─────────

#[test]
fn real_runner_subprocess_adr032_roundtrip_through_guest() {
    let runner_path = require_runner_binary();

    let mut child = std::process::Command::new(&runner_path)
        .arg("--component")
        .arg(component_fixture_path())
        .arg("--fuel")
        .arg("1000000000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("cannot spawn runner at {}: {e}", runner_path.display()));

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let sent_frame = make_test_frame();
    let sent_bytes = codec::encode_cbor(&sent_frame).unwrap();

    let mut writer = BufWriter::new(stdin);
    codec::write_frame(&mut writer, &sent_bytes).unwrap();
    // Close stdin to signal EOF (Halt::Voluntary) after the one frame.
    drop(writer);

    let mut reader = BufReader::new(stdout);
    let emitted_bytes = codec::read_frame(&mut reader)
        .unwrap()
        .expect("the echo-spirit guest must emit exactly one frame back");

    let emitted_frame: IacFrame = codec::decode_cbor(&emitted_bytes).unwrap();

    // The guest is an IDENTITY Spirit (handle-frame returns the input frame
    // unchanged) — so the domain-level content must match on every field the
    // WIT world carries. This is NOT a host-side echo: the bytes were
    // decoded, lowered to WIT, passed through a real `call_handle_frame` on
    // an instantiated component, lifted back, and re-encoded.
    assert_eq!(emitted_frame.frame_id, sent_frame.frame_id);
    assert_eq!(emitted_frame.timestamp_ns, sent_frame.timestamp_ns);
    assert_eq!(emitted_frame.kind, sent_frame.kind);
    assert_eq!(emitted_frame.from.spirit_id, sent_frame.from.spirit_id);
    assert_eq!(emitted_frame.payload, sent_frame.payload);

    // No more frames after the single emission.
    assert!(
        codec::read_frame(&mut reader).unwrap().is_none(),
        "guest must not emit extra frames for a single inbound frame"
    );

    let status = child.wait().unwrap();
    assert!(
        status.success(),
        "runner must exit cleanly (code 0) after stdin EOF, got {status:?}"
    );
}

// ── AC3: InvalidComponent fails closed (typed, not bare exit code) ─────

#[test]
fn invalid_component_fails_closed_with_distinct_exit_code() {
    let runner_path = require_runner_binary();

    let dir = tempfile::tempdir().unwrap();
    let bad_wasm = dir.path().join("bad.wasm");
    std::fs::write(&bad_wasm, b"not a valid wasm").unwrap();

    let mut child = std::process::Command::new(&runner_path)
        .arg("--component")
        .arg(bad_wasm.to_str().unwrap())
        .arg("--fuel")
        .arg("1000000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let status = child.wait().unwrap();
    assert!(!status.success(), "invalid component must fail closed");
    // RunnerExit::InvalidComponent = 3 (runner.rs) — distinct from the
    // generic-error code 1, so a parent can attribute the cause from the
    // exit status alone (mirrors AC4's fuel-vs-T2 cause attribution).
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.code(),
            Some(3),
            "must use the distinct InvalidComponent exit code, got {status:?}"
        );
        let _ = status.signal(); // not signal-killed
    }

    // The runner must never write a truncated/partial frame to stdout before
    // failing closed (AC3: "never a truncated frame").
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let leaked = codec::read_frame(&mut reader).unwrap();
    assert!(
        leaked.is_none(),
        "an InvalidComponent failure must not leak a partial frame to stdout"
    );
}
