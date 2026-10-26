//! Real component exports and real pipes: the test acts as the daemon.
#![cfg(target_os = "linux")]

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TelemetryEventPayload};
use maos_domain::invariants::{
    i1::{IntentClass, Scope},
    i3::FrameOrigin,
    i9::SandboxTier,
};
use maos_kernel_core::security::sandbox::{spawn_sandboxed, SandboxSpec, SpawnError};
use maos_spirit_abi::identity::FrameKind;
use maos_wasm_host::{codec, recall_wire::*};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, LazyLock};
use std::time::Duration;

#[path = "../../../tests/harness/ci.rs"]
mod ci;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

static RUNNER: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("maos-wasm-runner"));
static PROBE: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("recall_probe.wasm"));
static COMPAT: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("compat_echo_2_0.wasm"));
fn command(component: &Path) -> Command {
    let mut cmd = Command::new(&*RUNNER);
    cmd.arg("--component")
        .arg(component)
        .args(["--fuel", "1000000000"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    records: mpsc::Receiver<io::Result<Option<Vec<u8>>>>,
}
impl Session {
    fn start() -> Self {
        Self::component(&PROBE)
    }
    fn component(component: &Path) -> Self {
        let mut child = command(component).spawn().unwrap();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = mpsc::channel();
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
        Self {
            child,
            stdin,
            records: rx,
        }
    }
    fn record(&self) -> Option<Vec<u8>> {
        self.records
            .recv_timeout(Duration::from_secs(360))
            .expect("runner record deadline")
            .expect("valid stdout framing")
    }
    fn control(&self) {
        assert_eq!(self.record(), Some(vec![]));
    }
    fn send(&mut self, body: &[u8]) -> io::Result<()> {
        maos_frame_codec::write_frame(self.stdin.as_mut().unwrap(), body)
    }
    fn request(&self) -> RecallRequest {
        decode_request(&self.record().expect("recall request")).unwrap()
    }
    fn reply(&mut self, reply: RecallReply) {
        self.send(&encode_reply(&reply).unwrap())
            .expect("runner reads the reply");
    }
    fn report(&self) -> String {
        let body = self.record().expect("emitted report");
        assert!(!is_tagged(&body));
        match codec::decode_cbor::<IacFrame>(&body).unwrap().payload {
            FramePayload::TelemetryEvent(body) => body.data,
            other => panic!("expected report, got {other:?}"),
        }
    }
    fn finish(&mut self, code: i32) {
        self.stdin.take();
        assert_eq!(self.record(), None, "no extra frames or tagged records");
        let status = self.child.wait().unwrap();
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert_eq!(status.code(), Some(code), "{stderr}");
        if code == 6 {
            assert!(stderr.contains("recall protocol desync"), "{stderr}");
        }
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
        timestamp_ns: 123456789,
        logical_clock: 1,
        from: FrameAddress {
            spirit_id: "compat-baseline".into(),
            host_id: None,
            role: None,
        },
        to: Default::default(),
        kind: FrameKind::TelemetryEvent,
        intent: IntentClass::Standard,
        payload: FramePayload::TelemetryEvent(TelemetryEventPayload {
            event_type: "compat".into(),
            data: script.into(),
        }),
        auto_marker: FrameOrigin::Kernel,
        consent_envelope: None,
        intent_lineage: Default::default(),
    }
}
#[test]
fn context_report_returns_typed_status_and_context_frames_reach_handle_frame() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("context 3277\ncontext 3892")).unwrap())
        .unwrap();
    for (used, level) in [(3277, 1), (3892, 2)] {
        assert_eq!(s.request(), RecallRequest::Context { used_tokens: used });
        s.reply(RecallReply::Context {
            used,
            window: 4096,
            level,
            crossed: Some(level),
        });
    }
    assert_eq!(
        s.report(),
        "context ok 3277 4096 ContextLevel::Pressure ContextLevel::Pressure\ncontext ok 3892 4096 ContextLevel::Limit ContextLevel::Limit\n"
    );
    s.control();
    for (kind, name, used, threshold_permille) in [
        (FrameKind::ContextPressure, "pressure", 3277, 800),
        (FrameKind::ContextLimit, "limit", 3892, 950),
    ] {
        let envelope = maos_domain::frame::ContextEnvelope {
            spirit_pid: 71,
            context_used: used,
            context_window_size: 4096,
            threshold_permille,
        };
        let mut inbound = frame("");
        inbound.kind = kind;
        inbound.payload = match kind {
            FrameKind::ContextPressure => FramePayload::ContextPressure(envelope),
            _ => FramePayload::ContextLimit(envelope),
        };
        s.send(&codec::encode_cbor(&inbound).unwrap()).unwrap();
        assert_eq!(
            s.report(),
            format!("context-frame {name} 71 {used} 4096 {threshold_permille}\n")
        );
        s.control();
    }
    s.finish(0);
}

#[test]
fn context_report_not_admitted_is_typed_and_next_call_recovers() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("context 10\ncontext 20")).unwrap())
        .unwrap();
    assert_eq!(s.request(), RecallRequest::Context { used_tokens: 10 });
    s.reply(RecallReply::Error(WireError::CapabilityDenied));
    assert_eq!(s.request(), RecallRequest::Context { used_tokens: 20 });
    s.reply(RecallReply::Context {
        used: 20,
        window: 4096,
        level: 0,
        crossed: None,
    });
    assert_eq!(
        s.report(),
        "context err ContextError::NotAdmitted\ncontext ok 20 4096 ContextLevel::Within -\n"
    );
    s.control();
    s.finish(0);
}

// 2.1 added recall, not frame fields. Relabel the frozen guest's frames import
// in a temporary copy to exercise that exact same closed-enum export schema.
fn legacy_2_1_fixture(dir: &Path) -> PathBuf {
    let mut bytes = std::fs::read(&*COMPAT).unwrap();
    let name = b"maos:spirit/frames@2.0.0";
    let mut replaced = 0;
    for offset in 0..=bytes.len() - name.len() {
        if &bytes[offset..offset + name.len()] == name {
            bytes[offset + name.len() - 3] = b'1';
            replaced += 1;
        }
    }
    assert!(
        replaced > 0,
        "frozen guest must identify its frames interface"
    );
    let path = dir.join("legacy-2.1-export-schema.wasm");
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn legacy_2_1_export_schema_echoes_byte_identically() {
    let dir = tempfile::tempdir().unwrap();
    let component = legacy_2_1_fixture(dir.path());
    let mut s = Session::component(&component);
    s.control();
    let input = codec::encode_cbor(&frame("legacy-2.1-identity")).unwrap();
    s.send(&input).unwrap();
    assert_eq!(s.record(), Some(input));
    s.control();
    s.finish(0);
}

#[test]
fn legacy_2_0_and_2_1_reject_new_context_frames_with_typed_unrepresentable_exit() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_2_1 = legacy_2_1_fixture(dir.path());
    for component in [&*COMPAT, &legacy_2_1] {
        for kind in [FrameKind::ContextPressure, FrameKind::ContextLimit] {
            let mut s = Session::component(component);
            s.control();
            let body = maos_domain::frame::ContextEnvelope {
                spirit_pid: 71,
                context_used: 4000,
                context_window_size: 4096,
                threshold_permille: 950,
            };
            let mut inbound = frame("");
            inbound.kind = kind;
            inbound.payload = match kind {
                FrameKind::ContextPressure => FramePayload::ContextPressure(body),
                _ => FramePayload::ContextLimit(body),
            };
            s.send(&codec::encode_cbor(&inbound).unwrap()).unwrap();
            s.finish(5);
            let mut nested = frame("");
            nested.kind = FrameKind::Retract;
            nested.payload = FramePayload::Retract(maos_domain::frame::RetractPayload {
                original_frame_id: [7; 16],
                reason: "distilled".into(),
                original_kind: Some(kind),
            });
            let mut s = Session::component(component);
            s.control();
            s.send(&codec::encode_cbor(&nested).unwrap()).unwrap();
            s.finish(5);
        }
    }
}

#[test]
fn recall_probe_maps_filter_page_fetch_and_open_labels() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("recall limit=3 kind=TaskAssign since=4 until=99 intent=737464 cursor=2:01010101010101010101010101010101\nfetch last\nfetch self")).unwrap())
        .expect("runner reads the frame");
    let expected = WireFilter {
        kind: Some("TaskAssign".into()),
        since_ns: Some(4),
        until_ns: Some(99),
        limit: 3,
        intent_filter: Some("std".into()),
        cursor: Some(WireCursor {
            last_timestamp_ns: 2,
            last_frame_id: WireBytes(vec![1; 16]),
        }),
    };
    assert_eq!(s.request(), RecallRequest::Recall { filter: expected });
    let entry = WireEntry {
        frame_id: WireBytes(vec![9; 16]),
        timestamp_ns: 42,
        kind: "FutureOpenKind".into(),
        intent: "intent with spaces".into(),
        payload_available: true,
    };
    s.reply(RecallReply::Page {
        entries: vec![entry],
        next_cursor: Some(WireCursor {
            last_timestamp_ns: 42,
            last_frame_id: WireBytes(vec![9; 16]),
        }),
    });
    assert_eq!(
        s.request(),
        RecallRequest::Fetch {
            frame_id: WireBytes(vec![9; 16])
        }
    );
    s.reply(RecallReply::Fetched {
        frame_id: WireBytes(vec![9; 16]),
        timestamp_ns: 42,
        kind: "FutureOpenKind".into(),
        intent: "intent with spaces".into(),
        payload: WireBytes(vec![0, 255, 127]),
        origin: "spirit-auto".into(),
    });
    assert_eq!(
        s.request(),
        RecallRequest::Fetch {
            frame_id: WireBytes(vec![7; 16])
        }
    );
    s.reply(RecallReply::Error(WireError::ScopeViolation));
    let report = s.report();
    assert!(
        report.contains("recall ok 1 cursor=42:09090909090909090909090909090909"),
        "{report}"
    );
    assert!(report.contains("entry 09090909090909090909090909090909 42 FutureOpenKind 696e74656e74207769746820737061636573 true"),"{report}");
    assert!(report.contains("fetch ok 09090909090909090909090909090909 42 FutureOpenKind 696e74656e74207769746820737061636573 00ff7f spirit-auto"),"{report}");
    assert!(report.contains("fetch err scope-violation"), "{report}");
    s.control();
    s.finish(0);
}
fn hostile(body: &[u8], raw: bool) {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("recall\nfetch self\nrecall")).unwrap())
        .expect("runner reads the frame");
    assert!(matches!(s.request(), RecallRequest::Recall { .. }));
    if raw {
        s.stdin.as_mut().unwrap().write_all(body).unwrap();
        s.stdin.as_mut().unwrap().flush().unwrap();
    } else {
        s.send(body).expect("runner reads the hostile reply");
    }
    // Queue another frame: the broken client must not consume it after this export.
    // 18-0 AC4(c): the runner exits after its latching reply, so this write may
    // race that exit; EPIPE here IS the latch. Only BrokenPipe is accepted, and
    // only on this post-latch write; the latch itself is proven below.
    if !raw {
        match s.send(&codec::encode_cbor(&frame("recall")).unwrap()) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => {}
            Err(error) => panic!("post-latch write failed other than EPIPE: {error}"),
        }
    }
    assert_eq!(
        s.report(),
        "recall err unavailable\nfetch err unavailable\nrecall err unavailable\n"
    );
    s.control();
    s.finish(6);
}
#[test]
fn untagged_reply_latches_broken_after_export() {
    hostile(&codec::encode_cbor(&frame("not a reply")).unwrap(), false);
}
#[test]
fn unknown_tag_reply_latches_broken_after_export() {
    hostile(&[0xd9, 0xea, 0x62, 0xf6], false);
}
#[test]
fn undecodable_reply_latches_broken_after_export() {
    hostile(&[0xd9, 0xea, 0x61, 0xff], false);
}
#[test]
fn oversized_reply_latches_broken_after_export() {
    hostile(b"Content-Length: 16777217\r\n\r\n", true);
}
#[test]
fn partial_reply_header_latches_broken_after_export() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("recall\nfetch self")).unwrap())
        .expect("runner reads the frame");
    s.request();
    s.stdin
        .as_mut()
        .unwrap()
        .write_all(b"Content-Length: 12")
        .unwrap();
    s.stdin.take();
    assert_eq!(
        s.report(),
        "recall err unavailable\nfetch err unavailable\n"
    );
    s.control();
    s.finish(6);
}
#[test]
fn eof_outstanding_closes_client_but_finishes_export_and_exits_zero() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("recall\nfetch self\nrecall")).unwrap())
        .expect("runner reads the frame");
    s.request();
    s.stdin.take();
    assert_eq!(
        s.report(),
        "recall err unavailable\nfetch err unavailable\nrecall err unavailable\n"
    );
    s.control();
    s.finish(0);
}
#[test]
fn recall_outside_handle_frame_is_unavailable_without_records() {
    let mut s = Session::start();
    s.control();
    s.finish(0);
}
#[test]
fn reply_too_large_is_typed_and_does_not_break_following_call() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("fetch self\nrecall")).unwrap())
        .expect("runner reads the frame");
    assert!(matches!(s.request(), RecallRequest::Fetch { .. }));
    s.reply(RecallReply::Error(WireError::ReplyTooLarge(
        REPLY_BYTE_BOUND,
    )));
    assert!(matches!(s.request(), RecallRequest::Recall { .. }));
    s.reply(RecallReply::Page {
        entries: vec![],
        next_cursor: None,
    });
    assert_eq!(
        s.report(),
        "fetch err reply-too-large\nrecall ok 0 cursor=none\n"
    );
    s.control();
    s.finish(0);
}
#[test]
fn wrong_reply_variant_is_protocol_desync() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("fetch self\nrecall")).unwrap())
        .expect("runner reads the frame");
    s.request();
    s.reply(RecallReply::Page {
        entries: vec![],
        next_cursor: None,
    });
    assert_eq!(
        s.report(),
        "fetch err unavailable\nrecall err unavailable\n"
    );
    s.control();
    s.finish(6);
}
#[test]
fn frozen_2_0_echo_runs_under_t2_on_2_2_runner_byte_identically() {
    let spec = SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![Scope::FsRead {
            subtree: COMPAT.to_string_lossy().into_owned(),
        }],
        spirit_id: "compat-2-0-on-2-2".into(),
        output_shape_predicate: None,
    };
    let mut child = match spawn_sandboxed(&spec, &mut command(&COMPAT)) {
        Ok(child) => child,
        Err(SpawnError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
            if !ci::skip_unless(
                "T2 sandbox setup permitted by this host (CAP_SYS_ADMIN / no_new_privs)",
                false,
            ) {
                panic!("compat_2_0: T2 spawn refused on a runner declared capable (rule 11(b)): {error}");
            }
            return;
        }
        Err(error) => panic!("compat_2_0 spawn failed: {error:?}"),
    };
    // Exact input from T0's unchanged-runner evidence; frozen echo is identity.
    let input = codec::encode_cbor(&frame("frozen-2.0-identity")).unwrap();
    maos_frame_codec::write_frame(&mut child.child_mut().stdin.take().unwrap(), &input).unwrap();
    let mut reader = BufReader::new(child.child_mut().stdout.take().unwrap());
    assert_eq!(
        maos_frame_codec::read_frame(&mut reader).unwrap(),
        Some(vec![])
    );
    let output = maos_frame_codec::read_frame(&mut reader).unwrap().unwrap();
    assert!(!is_tagged(&output));
    assert_eq!(output, input);
    assert_eq!(
        maos_frame_codec::read_frame(&mut reader).unwrap(),
        Some(vec![])
    );
    assert_eq!(maos_frame_codec::read_frame(&mut reader).unwrap(), None);
    let status = child.wait().unwrap();
    let mut stderr = String::new();
    child
        .child_mut()
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert_eq!(status.code(), Some(0), "{stderr}");
}
