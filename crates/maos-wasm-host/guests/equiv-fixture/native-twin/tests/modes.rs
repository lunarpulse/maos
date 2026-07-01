//! Story 11.1b — behavioral verification of the native twin across all three
//! fixture modes.
//!
//! Each test drives the REAL `equiv-native-twin` binary over a real pipe:
//! encode a domain `IacFrame` to ADR-032 (Content-Length + canonical CBOR),
//! spawn the twin, write the frame, close stdin (→ `Halt::Voluntary`), and
//! decode the single emitted frame. This is the exact wire path the
//! cross-form gate exercises, so these tests pin the gate's NATIVE-form
//! oracle directly.
//!
//! - `identity`  → frame in == frame out (PASS case).
//! - `divergent` → `logical_clock += 1`, every other field preserved (FAIL case).
//! - `cosmetic`  → frame unchanged (PASS case); latency is asserted only loosely.

use std::io::Cursor;
use std::process::{Command, Stdio};
use std::time::Instant;

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskCompletePayload};
use maos_domain::invariants::i1::IntentClass;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i13::IntentLineage;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use smallvec::SmallVec;

use maos_wasm_host::codec::{decode_cbor, encode_cbor, read_frame, write_frame};

const FRAME_ID: [u8; 16] = [0xAB; 16];

fn sample_frame() -> IacFrame {
    let addr = FrameAddress {
        spirit_id: SpiritId("spirit-7".into()),
        host_id: Some(HostId("host-a".into())),
        role: Some(SpiritRole::Worker),
    };
    let mut to: SmallVec<[FrameAddress; 1]> = SmallVec::new();
    to.push(addr.clone());
    IacFrame {
        frame_id: FRAME_ID,
        timestamp_ns: 1_700_000_000_000,
        logical_clock: 42,
        from: addr,
        to,
        kind: FrameKind::TaskComplete,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskComplete(TaskCompletePayload {
            result: "done".into(),
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: IntentLineage::default(),
    }
}

/// Spawn the twin in `mode`, feed one frame, return the single emitted frame
/// plus the wall-clock elapsed (so the cosmetic-delay case can be loosely
/// checked). Panics on any protocol failure (prints stderr for diagnostics).
fn run_twin(mode: &str, input: &IacFrame) -> (IacFrame, std::time::Duration) {
    let cbor = encode_cbor(input).expect("encode input frame");

    let mut child = Command::new(env!("CARGO_BIN_EXE_equiv-native-twin"))
        .arg("--mode")
        .arg(mode)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn equiv-native-twin");

    {
        let mut stdin = child.stdin.take().expect("stdin");
        write_frame(&mut stdin, &cbor).expect("write frame to twin stdin");
        // Drop stdin → the twin reads EOF → `Halt::Voluntary` → clean exit.
    }

    let start = Instant::now();
    let output = child.wait_with_output().expect("wait for twin");
    let elapsed = start.elapsed();

    assert!(
        output.status.success(),
        "twin --mode {mode} exited {status}: {stderr}",
        status = output.status,
        stderr = String::from_utf8_lossy(&output.stderr)
    );

    let mut reader = Cursor::new(output.stdout);
    let frame_bytes = read_frame(&mut reader)
        .expect("read twin output")
        .expect("expected exactly one emitted frame");
    let frame: IacFrame = decode_cbor(&frame_bytes).expect("decode twin output");
    (frame, elapsed)
}

#[test]
fn identity_mode_is_a_byte_equal_echo() {
    let input = sample_frame();
    let (out, _) = run_twin("identity", &input);
    // Every field preserved — the PASS case.
    assert_eq!(out, input, "identity mode must echo the frame unchanged");
    assert_eq!(out.logical_clock, 42, "logical_clock must be unchanged");
}

#[test]
fn divergent_mode_bumps_logical_clock_by_exactly_one() {
    let input = sample_frame();
    let (out, _) = run_twin("divergent", &input);

    // The SINGLE divergence point — an invariant-bearing field.
    assert_eq!(out.logical_clock, 43, "logical_clock must be clock+1");

    // Every OTHER invariant field is preserved, so the gate can attribute the
    // flagged divergence to `logical_clock` alone.
    assert_eq!(out.frame_id, input.frame_id, "frame_id must be preserved");
    assert_eq!(out.timestamp_ns, input.timestamp_ns, "timestamp_ns must be preserved");
    assert_eq!(out.kind, input.kind, "kind must be preserved");
    assert_eq!(out.auto_marker, input.auto_marker, "auto_marker must be preserved");
    assert_eq!(out.from, input.from, "from must be preserved");
    assert_eq!(out.to, input.to, "to must be preserved");
    assert_eq!(out.payload, input.payload, "payload must be preserved");
}

#[test]
fn cosmetic_mode_preserves_invariants_but_adds_latency() {
    let input = sample_frame();
    let (out, elapsed) = run_twin("cosmetic", &input);

    // Cosmetic delay must NOT perturb any invariant field — PASS case.
    assert_eq!(out, input, "cosmetic mode must preserve every invariant field");
    assert_eq!(out.logical_clock, 42, "logical_clock must be unchanged");

    // The latency is observable but is NOT an invariant — assert it happened
    // (loosely, to stay above scheduler jitter). The gate classifies this form
    // as EQUIVALENT to identity precisely because only timing differs.
    assert!(
        elapsed.as_millis() >= 3,
        "cosmetic mode should add ~5ms latency, observed {elapsed:?}"
    );
}

#[test]
fn default_mode_is_identity() {
    // No `--mode` → defaults to identity.
    let input = sample_frame();
    let cbor = encode_cbor(&input).expect("encode");
    let mut child = Command::new(env!("CARGO_BIN_EXE_equiv-native-twin"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        write_frame(&mut stdin, &cbor).expect("write");
    }
    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let mut reader = Cursor::new(output.stdout);
    let bytes = read_frame(&mut reader).unwrap().unwrap();
    let out: IacFrame = decode_cbor(&bytes).unwrap();
    assert_eq!(out, input, "default mode must be identity");
}

#[test]
fn unknown_mode_exits_nonzero() {
    // A misconfigured harness must fail loudly, not silently fall back.
    let output = Command::new(env!("CARGO_BIN_EXE_equiv-native-twin"))
        .arg("--mode")
        .arg("bogus")
        .output()
        .expect("spawn");
    assert_ne!(output.status.code(), Some(0), "unknown mode must not succeed");
}
