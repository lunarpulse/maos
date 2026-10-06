//! Story 17-3b AC1 — real release-runner driver for the TypeScript component.
//!
//! The componentize-js artifact is supplied only by the Node-24 CI job. This
//! test intentionally remains ignored in ordinary host test runs, but an
//! explicit `--ignored` run fails loudly if that CI input is absent.

use std::io::{BufReader, BufWriter, Read};

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

fn runner_binary_path() -> std::path::PathBuf {
    let mut path = std::env::current_exe()
        .expect("test binary path")
        .parent()
        .expect("deps directory")
        .parent()
        .expect("profile directory")
        .to_path_buf();
    path.push("maos-wasm-runner");
    path
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
        "AC1 requires the release maos-wasm-runner beside this test binary: {}",
        runner.display()
    );

    let mut child = std::process::Command::new(&runner)
        .arg("--component")
        .arg(&component)
        .arg("--fuel")
        .arg("1000000000")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|error| panic!("cannot spawn {}: {error}", runner.display()));

    let input = full_frame();
    let input_bytes = codec::encode_cbor(&input).expect("input frame must canonically encode");
    let stdin = child.stdin.take().expect("runner stdin");
    let mut writer = BufWriter::new(stdin);
    codec::write_frame(&mut writer, &input_bytes).expect("write one ADR-032 frame");
    drop(writer);

    let stdout = child.stdout.take().expect("runner stdout");
    let stderr = child.stderr.take().expect("runner stderr");
    let mut reader = BufReader::new(stdout);
    let output_bytes = match codec::read_frame(&mut reader) {
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
    assert!(
        codec::read_frame(&mut reader)
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
