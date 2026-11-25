//! Story 17-3b AC1 — real release-runner driver for the TypeScript component.
//!
//! The componentize-js artifact is supplied only by the Node-24 CI job. This
//! test intentionally remains ignored in ordinary host test runs, but an
//! explicit `--ignored` run fails loudly if that CI input is absent.

use std::io::{BufReader, BufWriter, Read, Write};

use maos_domain::frame::{
    ConsentEnvelope, FrameAddress, FramePayload, HaltPolicyOverride, IacFrame, PostureHint,
    PosturePreferences, PriorDistillateRef, TaskAssignPayload,
};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i8::A2AIntent;
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId, SpiritRole};
use smallvec::smallvec;

use maos_wasm_host::codec;
use maos_wasm_host::recall_wire::{
    self, RecallReply, RecallRequest, WireBytes, WireError, WireFilter,
};

#[path = "../../../tests/harness/layout.rs"]
mod layout;

/// The RELEASE runner: the `release` profile directory beside this test's own
/// profile directory, so a `--target <triple>` build keeps its
/// `<target>/<triple>/release/` (not `<target>/release/`).
fn runner_binary_path() -> std::path::PathBuf {
    layout::sibling_profile_dir(&layout::profile_dir(), "release").join("maos-wasm-runner")
}

fn address(role: Option<SpiritRole>) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId("ts-component-spirit".into()),
        host_id: Some(HostId("ts-component-host".into())),
        role,
    }
}

fn lineage(values: &[&str]) -> IntentLineage {
    IntentLineage::new(values.iter().map(|value| A2AIntent::new(*value)).collect())
}

fn every_scope() -> Vec<Scope> {
    vec![
        Scope::FsRead {
            subtree: "/read".into(),
        },
        Scope::FsWrite {
            subtree: "/write".into(),
        },
        Scope::NetHttps {
            domain: "api.example.test".into(),
        },
        Scope::ProcExec {
            binary: "/usr/bin/true".into(),
        },
        Scope::SubSpiritSpawn {
            class: "worker".into(),
        },
        Scope::ProviderInfer {
            provider: "example-provider".into(),
        },
        Scope::IacSend {
            peer_class: "director".into(),
        },
        Scope::MemRead {
            scope: "notes".into(),
        },
        Scope::MemWrite {
            scope: "notes".into(),
        },
        Scope::SelfTelemetryRead,
        Scope::LogRecall,
        Scope::LogFetch,
        Scope::DistillateWrite,
        Scope::McpCall {
            server: "tools".into(),
            tool: "search".into(),
        },
        Scope::CliSubprocessSpawn {
            cli_binary_path: "/usr/bin/git".into(),
            argv_prefix_hash: [0x41; 32],
            output_shape_version: "v1".into(),
        },
        Scope::GatewaySend {
            gateway_id: "gateway-a".into(),
            recipient: "recipient-a".into(),
        },
        Scope::SkillAuthorSelf,
        Scope::LoomRead,
        Scope::LoomWrite,
        Scope::LoomScan,
    ]
}

fn full_frame() -> IacFrame {
    IacFrame {
        frame_id: [0x17; 16],
        timestamp_ns: 17_003_000,
        logical_clock: 17,
        from: address(Some(SpiritRole::Director)),
        to: smallvec![FrameAddress {
            spirit_id: SpiritId("ts-component-peer".into()),
            host_id: Some(HostId("ts-component-host".into())),
            role: Some(SpiritRole::Worker),
        }],
        kind: FrameKind::TaskAssign,
        intent: IntentClass::HighPrivilege,
        payload: FramePayload::TaskAssign(TaskAssignPayload {
            goal: "TypeScript component lossless round trip".into(),
            scope: every_scope(),
            success_criteria: "canonical CBOR equality".into(),
            posture_preferences: PosturePreferences {
                preferred_posture: Some(PostureHint::Assistive),
                halt_policy_overrides: vec![HaltPolicyOverride {
                    tag: "confidence".into(),
                    recall_vs_precision: 0.25,
                }],
            },
            prior_distillate_ref: Some(PriorDistillateRef {
                digest_frame_id: [0xD1; 16],
                distillation_depth: 3,
                intent_lineage: lineage(&["source", "distillate"]),
            }),
        }),
        auto_marker: FrameOrigin::HumanAuthored,
        consent_envelope: Some(ConsentEnvelope {
            consent_id: [0xC0; 16],
            granter: address(Some(SpiritRole::Observer)),
            timestamp_ns: 17_003_001,
            intent_class: Some(A2AIntent::new("diagnosis-handoff:read-only-evidence")),
            valid_until_ns: Some(17_999_999),
            recall: maos_domain::frame::ConsentRecall::Full,
        }),
        intent_lineage: lineage(&["operator", "typescript-component"]),
    }
}

#[test]
#[ignore = "needs the componentize-js artifact; run by example-spirit-ts-tests with --ignored"]
fn typescript_component_round_trips_a_fully_populated_frame() {
    let component = std::env::var("MAOS_TS_COMPONENT").expect(
        "MAOS_TS_COMPONENT is required by rule-11(b): the componentize-js artifact exists only in example-spirit-ts-tests",
    );
    assert!(
        std::path::Path::new(&component).is_file(),
        "rule-11(b): MAOS_TS_COMPONENT must name the componentize-js artifact: {component}"
    );

    let runner = runner_binary_path();
    assert!(
        runner.is_file(),
        "AC1 requires maos-wasm-runner in the target's release directory: {}",
        runner.display()
    );

    let mut child = std::process::Command::new(&runner)
        .arg("--component")
        .arg(&component)
        .arg("--fuel")
        .arg("10000000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("cannot spawn {}: {error}", runner.display()));

    let input = full_frame();
    let input_bytes = codec::encode_cbor(&input).expect("input frame must canonically encode");
    let stdin = child.stdin.take().expect("runner stdin");
    let mut writer = BufWriter::new(stdin);
    maos_frame_codec::write_frame(&mut writer, &input_bytes).expect("write one ADR-032 frame");
    writer.flush().expect("flush inbound frame");

    let stdout = child.stdout.take().expect("runner stdout");
    let stderr = child.stderr.take().expect("runner stderr");
    let mut reader = BufReader::new(stdout);
    assert_eq!(
        maos_frame_codec::read_frame(&mut reader).unwrap(),
        Some(Vec::new()),
        "Ready"
    );
    let request = maos_frame_codec::read_frame(&mut reader)
        .expect("read skill author request")
        .expect("guest must author before recalling");
    assert!(matches!(
        recall_wire::decode_request(&request).expect("tagged skill author request"),
        RecallRequest::SkillAuthor { .. }
    ));
    let reply = recall_wire::encode_reply(&RecallReply::Skill {
        entry_id: "entry".into(),
        target: "example-spirit-ts.example@1.0.0".into(),
    })
    .expect("encode skill reply");
    maos_frame_codec::write_frame(&mut writer, &reply).expect("reply to skill author");
    writer.flush().expect("flush skill reply");
    let request = maos_frame_codec::read_frame(&mut reader)
        .expect("read recall request")
        .expect("guest must recall before emitting");
    assert_eq!(
        recall_wire::decode_request(&request).expect("tagged recall request"),
        RecallRequest::Recall {
            filter: WireFilter {
                limit: 16,
                kind: None,
                since_ns: None,
                until_ns: None,
                cursor: None,
                intent_filter: None,
            },
        },
    );
    let reply = recall_wire::encode_reply(&RecallReply::Page {
        entries: Vec::new(),
        next_cursor: None,
    })
    .expect("encode empty page");
    maos_frame_codec::write_frame(&mut writer, &reply).expect("reply to recall");
    writer.flush().expect("flush recall reply");
    let request = maos_frame_codec::read_frame(&mut reader)
        .expect("read fetch request")
        .expect("guest must fetch before emitting");
    assert_eq!(
        recall_wire::decode_request(&request).expect("tagged fetch request"),
        RecallRequest::Fetch {
            frame_id: WireBytes(input.frame_id.to_vec()),
        },
    );
    let reply = recall_wire::encode_reply(&RecallReply::Error(WireError::ScopeViolation))
        .expect("encode scope-violation");
    maos_frame_codec::write_frame(&mut writer, &reply).expect("reply to fetch");
    writer.flush().expect("flush fetch reply");
    drop(writer);
    let output_bytes = match maos_frame_codec::read_frame(&mut reader) {
        Ok(Some(frame)) => frame,
        Ok(None) => {
            drop(reader);
            let mut stderr_text = String::new();
            BufReader::new(stderr)
                .read_to_string(&mut stderr_text)
                .expect("read runner stderr");
            let status = child.wait().expect("wait for runner");
            panic!(
                "TypeScript guest emitted no ADR-032 frame; runner status: {status}; runner stderr: {stderr_text}"
            );
        }
        Err(error) => {
            drop(reader);
            let mut stderr_text = String::new();
            BufReader::new(stderr)
                .read_to_string(&mut stderr_text)
                .expect("read runner stderr");
            let status = child.wait().expect("wait for runner");
            panic!(
                "read emitted ADR-032 frame failed: {error}; runner status: {status}; runner stderr: {stderr_text}"
            );
        }
    };
    assert_eq!(
        maos_frame_codec::read_frame(&mut reader).unwrap(),
        Some(Vec::new()),
        "TurnComplete"
    );
    assert!(
        maos_frame_codec::read_frame(&mut reader)
            .expect("read end of TypeScript guest stream")
            .is_none(),
        "TypeScript guest must emit exactly one frame"
    );
    let mut stderr_text = String::new();
    BufReader::new(stderr)
        .read_to_string(&mut stderr_text)
        .expect("read runner stderr");
    let status = child.wait().expect("wait for runner");
    assert_eq!(status.code(), Some(0), "runner stderr: {stderr_text}");
    assert_eq!(
        output_bytes, input_bytes,
        "the release runner and TypeScript guest must preserve canonical CBOR bytes"
    );

    println!(
        "TS-COMPONENT-OUTCOME component={} exit={} canonical_cbor_bytes={}",
        component,
        status.code().unwrap_or_default(),
        output_bytes.len()
    );
}

/// One turn of the shipped TypeScript component under the release runner at `fuel`, whose
/// `author` import is answered with `author_reply`; every other request gets its ordinary
/// answer. Returns the runner's exit code, its stderr and every non-request frame it emitted.
fn drive_turn(fuel: u64, author_reply: RecallReply) -> (Option<i32>, String, Vec<Vec<u8>>) {
    let component = std::env::var("MAOS_TS_COMPONENT")
        .expect("rule-11(b): MAOS_TS_COMPONENT names the componentize-js artifact");
    let runner = runner_binary_path();
    assert!(
        runner.is_file(),
        "release runner missing: {}",
        runner.display()
    );
    let mut child = std::process::Command::new(&runner)
        .arg("--component")
        .arg(&component)
        .arg("--fuel")
        .arg(fuel.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("cannot spawn {}: {error}", runner.display()));
    let frame = full_frame();
    let input = codec::encode_cbor(&frame).expect("canonical frame");
    let mut writer = Some(BufWriter::new(child.stdin.take().expect("runner stdin")));
    let first = writer.as_mut().expect("open");
    maos_frame_codec::write_frame(first, &input).expect("write the inbound frame");
    first.flush().expect("flush");
    let mut reader = BufReader::new(child.stdout.take().expect("runner stdout"));
    let stderr = child.stderr.take().expect("runner stderr");
    let mut author_reply = Some(author_reply);
    let mut emitted = Vec::new();
    while let Ok(Some(frame)) = maos_frame_codec::read_frame(&mut reader) {
        let reply = match recall_wire::decode_request(&frame) {
            Ok(RecallRequest::SkillAuthor { .. }) => author_reply.take(),
            Ok(RecallRequest::Recall { .. }) => Some(RecallReply::Page {
                entries: Vec::new(),
                next_cursor: None,
            }),
            Ok(RecallRequest::Fetch { .. }) => Some(RecallReply::Error(WireError::ScopeViolation)),
            _ => {
                emitted.push(frame);
                None
            }
        };
        if let Some(reply) = reply {
            let bytes = recall_wire::encode_reply(&reply).expect("encode reply");
            let last = matches!(reply, RecallReply::Error(WireError::ScopeViolation));
            if let Some(mut open) = writer.take() {
                maos_frame_codec::write_frame(&mut open, &bytes).expect("write the reply");
                open.flush().expect("flush reply");
                // The turn ends after the last reply: closing stdin lets the runner exit.
                writer = (!last).then_some(open);
            }
        }
    }
    drop(writer);
    let mut stderr_text = String::new();
    BufReader::new(stderr)
        .read_to_string(&mut stderr_text)
        .expect("read runner stderr");
    (child.wait().expect("wait").code(), stderr_text, emitted)
}

/// 22-9 F1 — a denied `author()` is answered by the guest's own `catch`: it logs the decline
/// and the turn completes within the ordinary 10,000,000 fuel, with no `OutOfFuel` and no
/// exit 4. Measured (release runner, this frame): the turn needs more than 7,000,000 and at most
/// 8,000,000 fuel, denied or accepted alike, so 10,000,000 leaves a 20-30% margin, not more.
#[test]
#[ignore = "needs the componentize-js artifact; run by example-spirit-ts-tests with --ignored"]
fn typescript_component_declines_a_denied_author_without_running_out_of_fuel() {
    let (code, stderr, emitted) =
        drive_turn(10_000_000, RecallReply::Error(WireError::CapabilityDenied));
    assert!(!stderr.contains("OutOfFuel"), "runner stderr: {stderr}");
    assert_eq!(code, Some(0), "runner stderr: {stderr}");
    assert!(
        stderr.contains("Skill authoring declined: capability-denied"),
        "the decline is logged: {stderr}"
    );
    assert_eq!(
        emitted.len(),
        3,
        "Ready, one echoed frame and the turn-complete marker"
    );
}
