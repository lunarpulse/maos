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

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn target_root() -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}
static RUNNER: LazyLock<PathBuf> = LazyLock::new(|| {
    let target = target_root();
    let output = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--offline",
            "--release",
            "-p",
            "maos-wasm-host",
            "--bin",
            "maos-wasm-runner",
            "--target-dir",
        ])
        .arg(&target)
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(crate_root())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "release runner build: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    target.join("release/maos-wasm-runner")
});
fn guest(name: &str, artifact: &str) -> PathBuf {
    // Separate target per guest: shared targets once corrupted fixture provenance.
    let target = target_root().with_file_name(format!("cargo-17-3d-guest-{name}"));
    let output = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--offline",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--manifest-path",
        ])
        .arg(crate_root().join(format!("guests/{name}/Cargo.toml")))
        .arg("--target-dir")
        .arg(&target)
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name} build: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    target.join(format!("wasm32-wasip2/release/{artifact}.wasm"))
}
static PROBE: LazyLock<PathBuf> = LazyLock::new(|| guest("recall-probe", "recall_probe"));
static COMPAT: LazyLock<PathBuf> = LazyLock::new(|| guest("compat-echo-2-0", "compat_echo_2_0"));
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
        let mut child = command(&PROBE).spawn().unwrap();
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
    fn send(&mut self, body: &[u8]) {
        maos_frame_codec::write_frame(self.stdin.as_mut().unwrap(), body).unwrap();
    }
    fn request(&self) -> RecallRequest {
        decode_request(&self.record().expect("recall request")).unwrap()
    }
    fn reply(&mut self, reply: RecallReply) {
        self.send(&encode_reply(&reply).unwrap());
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
fn recall_probe_maps_filter_page_fetch_and_open_labels() {
    let mut s = Session::start();
    s.control();
    s.send(&codec::encode_cbor(&frame("recall limit=3 kind=TaskAssign since=4 until=99 intent=737464 cursor=2:01010101010101010101010101010101\nfetch last\nfetch self")).unwrap());
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
    s.send(&codec::encode_cbor(&frame("recall\nfetch self\nrecall")).unwrap());
    assert!(matches!(s.request(), RecallRequest::Recall { .. }));
    if raw {
        s.stdin.as_mut().unwrap().write_all(body).unwrap();
        s.stdin.as_mut().unwrap().flush().unwrap();
    } else {
        s.send(body);
    }
    // Queue another frame: the broken client must not consume it after this export.
    if !raw {
        s.send(&codec::encode_cbor(&frame("recall")).unwrap());
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
    s.send(&codec::encode_cbor(&frame("recall\nfetch self")).unwrap());
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
    s.send(&codec::encode_cbor(&frame("recall\nfetch self\nrecall")).unwrap());
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
    s.send(&codec::encode_cbor(&frame("fetch self\nrecall")).unwrap());
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
    s.send(&codec::encode_cbor(&frame("fetch self\nrecall")).unwrap());
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
fn frozen_2_0_echo_runs_under_t2_on_2_1_runner_byte_identically() {
    let spec = SandboxSpec {
        tier: SandboxTier::T2,
        resolved_caps: Default::default(),
        declared_scopes: vec![Scope::FsRead {
            subtree: COMPAT.to_string_lossy().into_owned(),
        }],
        spirit_id: "compat-2-0-on-2-1".into(),
        output_shape_predicate: None,
    };
    let mut child = match spawn_sandboxed(&spec, &mut command(&COMPAT)) {
        Ok(child) => child,
        Err(SpawnError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
            if std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() {
                panic!("compat_2_0: T2 spawn refused on a runner declared capable (rule 11(b)): {error}");
            }
            eprintln!("SKIP compat_2_0: sandbox setup requires CAP_SYS_ADMIN / no_new_privs");
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
