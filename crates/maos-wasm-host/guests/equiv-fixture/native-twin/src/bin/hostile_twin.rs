//! `hostile-twin` — a native ADR-032 subprocess that misbehaves on purpose, for
//! the 17-3c governed-session gates. It speaks the same wire as
//! `equiv-native-twin` but each `--mode` emits what a hostile or broken guest
//! could: forged provenance, kernel-only kinds, floods, typed runner exits.
//!
//! The seed's first `to` address is the session's own Spirit id; the remaining
//! addresses are its delegated destinations (the `routed-relay` convention).

use std::io::{self, BufReader, BufWriter, Write};
use std::process::ExitCode;

use maos_domain::frame::{
    BudgetEnvelope, ConsentEnvelope, ConsentRequestPayload, FrameAddress, FramePayload, IacFrame,
    RetractPayload, TaskAssignPayload, TelemetryEventPayload,
};
use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::invariants::i8::A2AIntent;
use maos_frame_codec::{read_frame, write_frame};
use maos_spirit_abi::identity::{FrameKind, HostId, SpiritId};
use maos_wasm_host::codec::{decode_cbor, encode_cbor};

/// Frames past the ConsentRequest mailbox capacity (32) when self-addressed.
const SELF_FLOOD_FRAMES: usize = 33;
/// One more than the session's per-turn frame budget (64).
const FLOOD_FRAMES: usize = 65;
/// Diagnostic lines past the session's per-session row budget (256).
const NOISY_LINES: usize = 300;

fn main() -> ExitCode {
    let mode = match parse_mode() {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("hostile-twin: {error}");
            return ExitCode::from(64);
        }
    };
    if let Some(code) = mode.strip_prefix("exit-before-ready-") {
        return ExitCode::from(code.parse::<u8>().unwrap_or(64));
    }
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    if write_frame(&mut writer, &[]).is_err() {
        return ExitCode::from(1);
    }
    if mode == "ready-then-exit" {
        return ExitCode::SUCCESS;
    }
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    loop {
        let inbound: IacFrame = match read_frame(&mut reader) {
            Ok(Some(body)) => match decode_cbor(&body) {
                Ok(frame) => frame,
                Err(_) => return ExitCode::from(1),
            },
            Ok(None) => return ExitCode::SUCCESS,
            Err(_) => return ExitCode::from(1),
        };
        if mode == "exit-after-ready-3" {
            return ExitCode::from(3);
        }
        for frame in emit(&mode, inbound) {
            let Ok(body) = encode_cbor(&frame) else {
                return ExitCode::from(1);
            };
            if write_frame(&mut writer, &body).is_err() {
                return ExitCode::from(1);
            }
        }
        if write_frame(&mut writer, &[]).is_err() {
            return ExitCode::from(1);
        }
    }
}

fn emit(mode: &str, inbound: IacFrame) -> Vec<IacFrame> {
    let own = inbound.to.first().cloned();
    let mut onward = inbound.clone();
    if !onward.to.is_empty() {
        onward.to.remove(0);
    }
    match mode {
        "provenance" => {
            let mut forged = onward;
            forged.intent_lineage = IntentLineage::new(vec![A2AIntent::new("forged:lineage")]);
            forged.logical_clock = 999;
            forged.consent_envelope = Some(ConsentEnvelope {
                consent_id: [0x66; 16],
                granter: address("butler", None),
                timestamp_ns: 1,
                intent_class: Some(A2AIntent::new("forged:consent")),
                valid_until_ns: None,
            });
            vec![forged]
        }
        "refusals" => {
            let mut cross_host = onward.clone();
            cross_host.to = std::iter::once(address("scope-receiver", Some("evil-host"))).collect();
            let mut reserved = onward.clone();
            reserved.kind = FrameKind::BudgetExceeded;
            reserved.payload = FramePayload::BudgetExceeded(BudgetEnvelope {
                spirit_pid: 1,
                hook_name: "on_frame".into(),
                wall_ns: 1,
                cap_seconds: 1,
            });
            let mut wide = onward.clone();
            wide.payload = FramePayload::TaskAssign(TaskAssignPayload {
                goal: "too many scopes".into(),
                scope: (0..17)
                    .map(|index| Scope::McpCall {
                        server: "scope-tools".into(),
                        tool: format!("tool-{index}"),
                    })
                    .collect(),
                success_criteria: "refused".into(),
                posture_preferences: Default::default(),
                prior_distillate_ref: None,
            });
            let mut foreign_retract = onward.clone();
            foreign_retract.kind = FrameKind::Retract;
            foreign_retract.payload = FramePayload::Retract(RetractPayload {
                original_frame_id: inbound.frame_id,
                reason: "not mine to retract".into(),
                original_kind: None,
            });
            vec![cross_host, reserved, wide, foreign_retract, onward]
        }
        "flood" => {
            // Telemetry rides a 256-slot broadcast channel, so the destination
            // never back-pressures: only the per-turn frame budget can stop it.
            let mut telemetry = onward;
            telemetry.kind = FrameKind::TelemetryEvent;
            telemetry.payload = FramePayload::TelemetryEvent(TelemetryEventPayload {
                event_type: "flood".into(),
                data: String::new(),
            });
            vec![telemetry; FLOOD_FRAMES]
        }
        "self-flood" => {
            let mut request = inbound;
            request.to = own.into_iter().collect();
            request.kind = FrameKind::ConsentRequest;
            request.payload = FramePayload::ConsentRequest(ConsentRequestPayload {
                capability: "self".into(),
            });
            vec![request; SELF_FLOOD_FRAMES]
        }
        "noisy" => {
            let stderr = io::stderr();
            let mut stderr = stderr.lock();
            for line in 0..NOISY_LINES {
                let _ = writeln!(stderr, "hostile diagnostic {line}");
            }
            let _ = stderr.flush();
            vec![onward]
        }
        _ => vec![onward],
    }
}

fn address(spirit_id: &str, host: Option<&str>) -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from(spirit_id),
        host_id: host.map(|host| HostId(host.into())),
        role: None,
    }
}

fn parse_mode() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--mode"), Some(mode), None) => Ok(mode),
        _ => Err("usage: hostile-twin --mode <mode>".into()),
    }
}
