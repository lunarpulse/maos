//! Story 20-9 AC1 — the real runner against real components, with the test acting as the
//! daemon: every lifecycle request reaches its export, imports are callable only inside the
//! acting exports, and a guest without the export is answered `Unsupported`.
#![cfg(target_os = "linux")]

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::{i1::IntentClass, i3::FrameOrigin};
use maos_spirit_abi::identity::FrameKind;
use maos_wasm_host::codec;
use maos_wasm_host::lifecycle_wire::{self as wire, LifecycleReply as Reply, LifecycleRequest};
use maos_wasm_host::recall_wire::{self, RecallReply, RecallRequest, WireBytes};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, LazyLock};
use std::time::Duration;

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

static RUNNER: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("maos-wasm-runner"));
static PROBE: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("lifecycle_probe.wasm"));

fn frozen(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/wasm/frozen")
        .join(name)
}

struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    records: mpsc::Receiver<io::Result<Option<Vec<u8>>>>,
}

impl Session {
    fn start(component: &Path) -> Self {
        let mut child = Command::new(&*RUNNER)
            .arg("--component")
            .arg(component)
            .args(["--fuel", "1000000000"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, records) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let record = maos_frame_codec::read_frame(&mut reader);
                let done = !matches!(record, Ok(Some(_)));
                if tx.send(record).is_err() || done {
                    break;
                }
            }
        });
        let session = Self {
            child,
            stdin,
            records,
        };
        assert_eq!(session.record(), Some(vec![]), "Ready control frame");
        session
    }
    fn record(&self) -> Option<Vec<u8>> {
        self.records
            .recv_timeout(Duration::from_secs(360))
            .expect("runner record deadline")
            .expect("valid stdout framing")
    }
    fn send(&mut self, body: &[u8]) {
        maos_frame_codec::write_frame(self.stdin.as_mut().unwrap(), body).unwrap();
    }
    /// One lifecycle request. Recall requests the guest raises inside the export are answered
    /// with an empty page, and counted.
    fn lifecycle(&mut self, request: &LifecycleRequest) -> (Reply, usize) {
        self.send(&wire::encode_request(request).unwrap());
        let mut recalls = 0;
        loop {
            let body = self.record().expect("a record before the runner exits");
            if wire::is_reply(&body) {
                return (wire::decode_reply(&body).unwrap(), recalls);
            }
            assert!(
                recall_wire::is_tagged(&body),
                "only recall or lifecycle records"
            );
            assert!(matches!(
                recall_wire::decode_request(&body).unwrap(),
                RecallRequest::Recall { .. }
            ));
            recalls += 1;
            let reply = RecallReply::Page {
                entries: Vec::new(),
                next_cursor: None,
            };
            self.send(&recall_wire::encode_reply(&reply).unwrap());
        }
    }
    /// One frame turn; the probe answers with its report, the echo guest with the frame.
    fn turn(&mut self, script: &str) -> String {
        self.send(&codec::encode_cbor(&frame(script)).unwrap());
        let body = self.record().expect("emitted frame");
        let emitted: IacFrame = codec::decode_cbor(&body).unwrap();
        assert_eq!(self.record(), Some(vec![]), "TurnComplete");
        match emitted.payload {
            FramePayload::TelemetryEvent(report) => report.data,
            FramePayload::TaskAssign(task) => task.goal,
            other => panic!("unexpected payload {other:?}"),
        }
    }
    fn finish(&mut self) {
        self.stdin.take();
        assert_eq!(self.record(), None, "no stray records");
        let status = self.child.wait().unwrap();
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert_eq!(status.code(), Some(0), "{stderr}");
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn frame(script: &str) -> IacFrame {
    IacFrame {
        frame_id: [7; 16],
        timestamp_ns: 1,
        logical_clock: 1,
        from: FrameAddress {
            spirit_id: "operator".into(),
            host_id: None,
            role: None,
        },
        to: Default::default(),
        kind: FrameKind::TaskAssign,
        intent: IntentClass::Standard,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: script.into(),
            scope: Vec::new(),
            success_criteria: String::new(),
            posture_preferences: Default::default(),
            prior_distillate_ref: None,
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}

fn state(counter: u64) -> WireBytes {
    WireBytes(counter.to_le_bytes().to_vec())
}

fn telemetry() -> LifecycleRequest {
    LifecycleRequest::Telemetry {
        seq: 41,
        spirit_id: "probe".into(),
        tag: "queue.depth".into(),
        value: 0.25,
        timestamp_ms: 1_700_000_000_000,
    }
}

#[test]
fn every_lifecycle_request_reaches_its_export_and_imports_follow_the_rule() {
    let mut s = Session::start(&PROBE);
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Probe),
        (Reply::Supported(true), 0)
    );
    // The acting exports may call `recall` (one in-band request each); the swap steps may not.
    assert_eq!(s.lifecycle(&LifecycleRequest::Idle), (Reply::Done, 1));
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Consolidate {
            batch: WireBytes(vec![1, 2, 3])
        }),
        (Reply::Done, 1)
    );
    assert_eq!(s.lifecycle(&telemetry()), (Reply::Done, 1));
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Schedule {
            payload: WireBytes(b"every-second".to_vec())
        }),
        (Reply::Done, 1)
    );
    assert_eq!(s.lifecycle(&LifecycleRequest::Pause), (Reply::Done, 0));
    assert_eq!(s.lifecycle(&LifecycleRequest::Resume), (Reply::Done, 0));
    // Three frames, then the counter is the snapshot.
    for _ in 0..3 {
        s.turn("");
    }
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Snapshot),
        (Reply::State(state(3)), 0)
    );
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Migrate { state: state(3) }),
        (Reply::State(state(1003)), 0)
    );
    assert_eq!(
        s.lifecycle(&LifecycleRequest::SwapIn { state: state(77) }),
        (Reply::Done, 0)
    );
    let report = s.turn("");
    for expected in [
        "counts idle=1 consolidate=1 telemetry=1 schedule=1 pause=1 resume=1 frames=4",
        "state counter=78 swap-in=77 migrate=1003",
        "imports consolidate=ok idle=ok migrate=unavailable pause=unavailable resume=unavailable \
         schedule=ok snapshot=unavailable swap-in=unavailable telemetry=ok",
        "last telemetry=queue.depth@41 schedule=every-second consolidate=3",
    ] {
        assert!(
            report.contains(expected),
            "missing `{expected}` in:\n{report}"
        );
    }
    s.finish();
}

#[test]
fn scripted_refusals_are_typed_and_leave_the_instance_serving() {
    let mut s = Session::start(&PROBE);
    s.turn("fail idle\ndecline snapshot\nfail swap-in");
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Idle).0,
        Reply::Failed("scripted idle".into())
    );
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Snapshot).0,
        Reply::Declined("scripted snapshot".into())
    );
    assert_eq!(
        s.lifecycle(&LifecycleRequest::SwapIn { state: state(5) }).0,
        Reply::Failed("scripted swap-in".into())
    );
    // A state that is not the probe's shape is the guest's own typed failure.
    assert!(matches!(
        s.lifecycle(&LifecycleRequest::SwapIn { state: WireBytes(vec![1, 2]) }).0,
        Reply::Failed(detail) if detail.contains("not 8")
    ));
    assert_eq!(s.lifecycle(&LifecycleRequest::Idle).0, Reply::Done);
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Snapshot).0,
        Reply::State(state(1))
    );
    s.finish();
}

#[test]
fn a_guest_without_the_lifecycle_export_answers_unsupported_and_keeps_running() {
    for (label, component) in [
        ("2.2 guest", frozen("echo_spirit_2_2_0.wasm")),
        ("2.3 guest", frozen("echo_spirit_2_3_0.wasm")),
        ("2.4 guest without the export", fixtures_echo()),
    ] {
        let mut s = Session::start(&component);
        assert_eq!(
            s.lifecycle(&LifecycleRequest::Probe),
            (Reply::Supported(false), 0),
            "{label}"
        );
        assert_eq!(
            s.lifecycle(&LifecycleRequest::Idle),
            (Reply::Unsupported, 0),
            "{label}"
        );
        assert_eq!(
            s.lifecycle(&LifecycleRequest::Snapshot),
            (Reply::Unsupported, 0),
            "{label}"
        );
        // The instance is untouched: the next frame turn is served.
        assert_eq!(s.turn("still here"), "still here", "{label}");
        s.finish();
    }
}

fn fixtures_echo() -> PathBuf {
    fixtures::fixture("echo_spirit.wasm")
}

#[test]
fn the_legacy_2_0_guest_answers_unsupported_too() {
    let mut s = Session::start(&fixtures::fixture("compat_echo_2_0.wasm"));
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Probe),
        (Reply::Supported(false), 0)
    );
    assert_eq!(
        s.lifecycle(&LifecycleRequest::Pause),
        (Reply::Unsupported, 0)
    );
    s.finish();
}
