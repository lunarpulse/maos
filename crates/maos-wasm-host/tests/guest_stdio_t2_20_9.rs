//! Story 20-9 F1 — a guest that writes to stdout/stderr (a TypeScript `console.log` is exactly
//! this, via `wasi:cli`) must not kill the runner under the real T2 sandbox. The runner is
//! launched through the kernel's own `spawn_sandboxed` (Landlock + seccomp allow-list), the test
//! acts as the daemon, and the guest is the `lifecycle-probe` scripted with `say NAME TEXT`.
//!
//! Before the fix the first guest write reached wasmtime-wasi's lazily-built ambient tokio
//! runtime inside the sandbox; the runtime's driver setup is refused by the seccomp allow-list
//! (`Os { code: 1, PermissionDenied }`, `wasmtime-wasi runtime.rs:32`), the runner panicked
//! (exit 101) and the session read as `unplanned_crash`.
#![cfg(target_os = "linux")]

use maos_domain::frame::{FrameAddress, FramePayload, IacFrame, TaskAssignPayload};
use maos_domain::invariants::{i1::IntentClass, i3::FrameOrigin, i9::SandboxTier};
use maos_kernel_core::security::sandbox::{
    spawn_sandboxed, SandboxSpec, SandboxedChild, SpawnError,
};
use maos_spirit_abi::identity::FrameKind;
use maos_wasm_host::codec;
use maos_wasm_host::lifecycle_wire::{self as wire, LifecycleReply as Reply, LifecycleRequest};
use maos_wasm_host::recall_wire::{self, RecallReply, RecallRequest, WireBytes};
use std::io::{self, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{mpsc, LazyLock};
use std::time::Duration;

#[path = "../../../tests/harness/ci.rs"]
mod ci;
#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

static RUNNER: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("maos-wasm-runner"));
static PROBE: LazyLock<PathBuf> = LazyLock::new(|| fixtures::fixture("lifecycle_probe.wasm"));

struct Sandboxed {
    child: SandboxedChild,
    records: mpsc::Receiver<io::Result<Option<Vec<u8>>>>,
}

impl Sandboxed {
    /// Start the runner on the probe under T2, or `None` when this host cannot sandbox and is not
    /// declared capable (rule 11(b): in CI that refusal is a named failure, never a skip).
    fn start(test: &str) -> Option<Self> {
        let spec = SandboxSpec {
            tier: SandboxTier::T2,
            resolved_caps: Default::default(),
            declared_scopes: vec![maos_domain::invariants::i1::Scope::FsRead {
                subtree: PROBE.to_string_lossy().into_owned(),
            }],
            spirit_id: "lifecycle-probe-stdio".into(),
            output_shape_predicate: None,
        };
        let mut command = Command::new(&*RUNNER);
        command
            .arg("--component")
            .arg(&*PROBE)
            .args(["--fuel", "1000000000"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = match spawn_sandboxed(&spec, &mut command) {
            Ok(child) => child,
            Err(SpawnError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
                if !ci::skip_unless(
                    "T2 sandbox setup permitted by this host (CAP_SYS_ADMIN / no_new_privs)",
                    false,
                ) {
                    panic!(
                        "{test}: T2 spawn refused on a runner declared capable (rule 11(b)): {error}"
                    );
                }
                return None;
            }
            Err(error) => panic!("{test}: spawn_sandboxed failed: {error:?}"),
        };
        let stdout = child.child_mut().stdout.take().expect("piped stdout");
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
        Some(Self { child, records })
    }

    /// The next record on the runner's frame channel; `None` is its EOF (the runner is gone).
    fn record(&self) -> Option<Vec<u8>> {
        self.records
            .recv_timeout(Duration::from_secs(120))
            .expect("runner record deadline")
            .expect("valid stdout framing")
    }

    fn send(&mut self, body: &[u8]) {
        let stdin = self.child.child_mut().stdin.as_mut().expect("stdin open");
        maos_frame_codec::write_frame(stdin, body).expect("runner reads the request");
    }

    /// One lifecycle request; the probe's `recall` import is answered with an empty page.
    fn lifecycle(&mut self, request: &LifecycleRequest) -> Reply {
        self.send(&wire::encode_request(request).unwrap());
        loop {
            let body = self
                .record()
                .expect("the runner stays alive until the lifecycle export returns");
            if wire::is_reply(&body) {
                return wire::decode_reply(&body).unwrap();
            }
            assert!(
                recall_wire::is_tagged(&body),
                "only recall or lifecycle records"
            );
            assert!(matches!(
                recall_wire::decode_request(&body).unwrap(),
                RecallRequest::Recall { .. }
            ));
            let page = RecallReply::Page {
                entries: Vec::new(),
                next_cursor: None,
            };
            self.send(&recall_wire::encode_reply(&page).unwrap());
        }
    }

    /// One frame turn (`handleFrame`); returns the probe's report.
    fn turn(&mut self, script: &str) -> String {
        self.send(&codec::encode_cbor(&frame(script)).unwrap());
        let body = self
            .record()
            .expect("the runner stays alive until handleFrame returns");
        let emitted: IacFrame = codec::decode_cbor(&body).unwrap();
        assert_eq!(self.record(), Some(vec![]), "TurnComplete");
        match emitted.payload {
            FramePayload::TelemetryEvent(report) => report.data,
            other => panic!("unexpected payload {other:?}"),
        }
    }

    /// Close the guest's input, wait for it and return `(exit code, stderr)`.
    fn finish(mut self) -> (Option<i32>, String) {
        self.child.child_mut().stdin.take();
        assert_eq!(self.record(), None, "no stray records after the last turn");
        let status = self.child.wait().unwrap();
        let mut stderr = String::new();
        self.child
            .child_mut()
            .stderr
            .take()
            .expect("piped stderr")
            .read_to_string(&mut stderr)
            .unwrap();
        (status.code(), stderr)
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

fn assert_routed(stderr: &str, export: &str, text: &str) {
    for stream in ["stdout", "stderr"] {
        let line = format!("lifecycle-probe {stream} {export}: {text}");
        assert!(
            stderr.contains(&line),
            "the guest's {stream} write in `{export}` is routed to the runner's stderr; \
             missing `{line}` in:\n{stderr}"
        );
    }
}

/// F1 vector: stdout/stderr written inside `idle`, `swap-in` and `handleFrame` under the real T2
/// sandbox. Each export returns its reply, the same runner keeps serving, the frame channel stays
/// framing-clean (guest output never reaches it) and every written line reaches the host sink.
#[test]
fn guest_stdio_inside_lifecycle_exports_and_handle_frame_does_not_kill_the_runner_under_t2() {
    let Some(mut s) = Sandboxed::start("guest_stdio_t2") else {
        return;
    };
    assert_eq!(s.record(), Some(vec![]), "Ready control frame");

    // `handleFrame`: the script queues `idle` / `swap-in` writes and writes `frame` right now.
    let report = s.turn("say idle idle-text\nsay swap-in swap-text\nsay frame frame-text");
    assert!(report.contains("frames=1"), "{report}");

    assert_eq!(s.lifecycle(&LifecycleRequest::Idle), Reply::Done);
    assert_eq!(
        s.lifecycle(&LifecycleRequest::SwapIn { state: state(41) }),
        Reply::Done
    );
    // The runner is still the same live instance: the swapped-in counter continues from 41.
    assert!(s.turn("").contains("state counter=42 swap-in=41"));

    let (code, stderr) = s.finish();
    assert_eq!(code, Some(0), "{stderr}");
    assert_routed(&stderr, "frame", "frame-text");
    assert_routed(&stderr, "idle", "idle-text");
    assert_routed(&stderr, "swap-in", "swap-text");
    assert!(
        !stderr.contains("panicked"),
        "no runner panic (wasmtime-wasi runtime.rs / PermissionDenied):\n{stderr}"
    );
}

/// The negative control under the identical sandbox: the same guest without a single write runs
/// the same exports, so the vector above fails because of the write path, not the harness.
#[test]
fn the_same_guest_without_writes_runs_the_same_exports_under_t2() {
    let Some(mut s) = Sandboxed::start("guest_quiet_t2") else {
        return;
    };
    assert_eq!(s.record(), Some(vec![]), "Ready control frame");
    assert!(s.turn("").contains("frames=1"));
    assert_eq!(s.lifecycle(&LifecycleRequest::Idle), Reply::Done);
    assert_eq!(
        s.lifecycle(&LifecycleRequest::SwapIn { state: state(41) }),
        Reply::Done
    );
    let (code, stderr) = s.finish();
    assert_eq!(code, Some(0), "{stderr}");
    assert!(!stderr.contains("lifecycle-probe stdout"), "{stderr}");
}

/// A write in the shutdown hook (the runner's last guest call, after the input closes) is
/// routed too, and the stop stays a clean exit 0.
#[test]
fn guest_stdio_in_the_shutdown_hook_is_routed_and_the_stop_stays_clean_under_t2() {
    let Some(mut s) = Sandboxed::start("guest_stdio_shutdown_t2") else {
        return;
    };
    assert_eq!(s.record(), Some(vec![]), "Ready control frame");
    s.turn("say shutdown bye");
    let (code, stderr) = s.finish();
    assert_eq!(code, Some(0), "{stderr}");
    assert_routed(&stderr, "shutdown", "bye");
}
