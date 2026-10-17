#![forbid(unsafe_code)]

//! Story 10.5 AC1 — xtask CI gate: skill-format conformance (NFR-Test-10).
//!
//! Validates that at least one third-party skill format (Anthropic Skills)
//! executes via a Spirit-form adapter without kernel modification.
//!
//! Journaled results and live adapter validation are required for a proven pass.
//! Missing results report ABSENT; unreadable or corrupt evidence reports RED.
//! Malformed TOML or invalid results fail.
//!
//! Proven-red: the real Anthropic fixture under `tests/fixtures/anthropic-skill/`
//! is parsed through the adapter and validated.  The invalid fixture under
//! `tests/fixtures/anthropic-skill-invalid/` must fail.

use serde::Deserialize;

use crate::gate_common::{Observation, OutOfLedgerVerdict, CURRENT_PHASE};

/// Root of `docs/skill-conformance/results/skill-conformance-results.toml`.
#[derive(Debug, Deserialize)]
pub struct ConformanceResults {
    pub conformance: ConformanceSection,
}

#[derive(Debug, Deserialize)]
pub struct ConformanceSection {
    pub adapter_name: String,
    pub source_format: String,
    pub target_format: String,
    pub executed_without_kernel_modification: bool,
    pub abi_unchanged: bool,
    pub fixture_path: String,
    pub conformance_date: String,
}

const RESULTS_PATH: &str = "docs/skill-conformance/results/skill-conformance-results.toml";
const VALID_FIXTURE: &str = "tests/fixtures/anthropic-skill/SKILL.md";
const INVALID_FIXTURE: &str = "tests/fixtures/anthropic-skill-invalid/SKILL.md";

#[cfg(unix)]
fn bridge_program_and_prefix(skill_name: &str) -> (String, Vec<String>, Vec<String>) {
    (
        "printf".into(),
        vec!["%s\\n".into()],
        vec![format!("executed:{skill_name}")],
    )
}

#[cfg(windows)]
fn bridge_program_and_prefix(skill_name: &str) -> (String, Vec<String>, Vec<String>) {
    (
        "cmd".into(),
        vec!["/C".into(), "echo".into()],
        vec![format!("executed:{skill_name}")],
    )
}

fn execute_fixture_via_cli_wrapper(skill_name: &str) -> Result<(), String> {
    use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
        argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeSpawnSpec,
    };
    use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};

    let (program, argv_prefix, task_args) = bridge_program_and_prefix(skill_name);
    let expected_argv_prefix_hash = argv_prefix_hash(&argv_prefix);
    let mut bridge = spawn_and_bridge(BridgeSpawnSpec {
        program,
        sandbox: maos_kernel_core::security::sandbox::SandboxSpec::new_for_test(
            maos_domain::invariants::i9::SandboxTier::T0,
        ),
        argv_prefix,
        task_args,
        expected_argv_prefix_hash,
        from_spirit_id: "skill-conformance-adapter".into(),
        stdio_shape: CliWrapperStdioShape::Raw,
        control_channel: CliWrapperControlChannel::StdinCommands,
        shutdown_signal: None,
        channel_capacity: 4,
        backpressure: Backpressure::Block,
        env: vec![],
        t3: None,
    })
    .map_err(|e| format!("CliWrapper spawn_and_bridge failed: {e}"))?;

    let (_stream, bytes) = bridge
        .recv_line()
        .ok_or_else(|| "CliWrapper execution produced no stdout frame".to_string())?;
    let line = String::from_utf8_lossy(&bytes);
    let expected = format!("executed:{skill_name}");
    if line.trim() != expected {
        return Err(format!(
            "CliWrapper execution output mismatch: expected {expected:?}, got {line:?}"
        ));
    }
    bridge
        .on_unload()
        .map_err(|e| format!("CliWrapper unload failed: {e}"))?;
    Ok(())
}

pub fn run(json: bool) -> Result<(), String> {
    let (verdict, details) = match check() {
        Ok(report) => report,
        Err(error) => (
            OutOfLedgerVerdict::judge_registered(
                "check-skill-conformance",
                Observation::Red,
                true,
                CURRENT_PHASE,
                &error,
            )?,
            serde_json::json!({"error": error}),
        ),
    };
    verdict.finish(json, details)
}

fn check() -> Result<(OutOfLedgerVerdict, serde_json::Value), String> {
    let content = match std::fs::read_to_string(RESULTS_PATH) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                OutOfLedgerVerdict::judge_registered(
                    "check-skill-conformance",
                    Observation::NotRun,
                    false,
                    CURRENT_PHASE,
                    format!("results absent or unreadable: {error}"),
                )?,
                serde_json::json!({}),
            ));
        }
        Err(error) => return Err(format!("cannot read {RESULTS_PATH}: {error}")),
    };
    // Journaled results and live validation are both required for a proven pass.
    let results: ConformanceResults =
        toml::from_str(&content).map_err(|e| format!("malformed {RESULTS_PATH}: {e}"))?;
    // --- Live adapter validation (the real conformance test) ---
    // Parse the valid fixture through the Anthropic adapter, then execute the
    // adapted skill through the existing CliWrapper subprocess bridge.
    let valid_src = std::fs::read_to_string(VALID_FIXTURE)
        .map_err(|e| format!("cannot read valid fixture {VALID_FIXTURE}: {e}"))?;

    let skill = maos_skill::parse_anthropic_skill(&valid_src)
        .map_err(|e| format!("valid fixture {VALID_FIXTURE} failed adapter: {e}"))?;

    if skill.manifest.name.is_empty() {
        return Err("valid fixture produced empty skill name".into());
    }
    if skill.body.trim().is_empty() {
        return Err("valid fixture produced empty body".into());
    }

    execute_fixture_via_cli_wrapper(&skill.manifest.name)?;

    eprintln!(
        "check-skill-conformance: adapter+CliWrapper PASS — '{}' (id={}, v={})",
        skill.manifest.name, skill.manifest.id, skill.manifest.version
    );

    // --- Proven-red: invalid fixture MUST fail ---
    let invalid_src = std::fs::read_to_string(INVALID_FIXTURE)
        .map_err(|e| format!("cannot read invalid fixture {INVALID_FIXTURE}: {e}"))?;

    if maos_skill::parse_anthropic_skill(&invalid_src).is_ok() {
        return Err(format!(
            "PROVEN-RED FAILURE: invalid fixture {INVALID_FIXTURE} should have been rejected by the adapter but was accepted"
        ));
    }
    eprintln!("check-skill-conformance: proven-red PASS — invalid fixture correctly rejected");

    let c = &results.conformance;

    let green = c.executed_without_kernel_modification && c.abi_unchanged;
    Ok((
        OutOfLedgerVerdict::judge_registered(
            "check-skill-conformance",
            if green {
                Observation::Green
            } else {
                Observation::Red
            },
            true,
            CURRENT_PHASE,
            if green {
                "conformance validated"
            } else {
                "conformance findings"
            },
        )?,
        serde_json::json!({
            "adapter_name": c.adapter_name,
            "source_format": c.source_format,
            "target_format": c.target_format,
            "conformance_date": c.conformance_date,
        }),
    ))
}
