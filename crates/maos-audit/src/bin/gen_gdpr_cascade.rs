#![forbid(unsafe_code)]

//! Story 9.2 — GDPR Article 17 cascade corpus generator.
//!
//! Produces deterministic SHA-pinned JSONL fixtures for the erasure spine:
//!   * `gdpr-cascade-v0.jsonl` — 50 scenarios across 6 strata
//!   * `gdpr-cascade-probe-v0.jsonl` — 100 independent leakage probes
//!
//! Usage:
//!   cargo run --quiet -p maos-audit --bin gen_gdpr_cascade -- corpus <out>
//!   cargo run --quiet -p maos-audit --bin gen_gdpr_cascade -- probe <out>

use std::io::{BufWriter, Write};

use serde::Serialize;

/// Deterministic 64-bit Linear Congruential Generator (Numerical Recipes).
struct Lcg(u64);

impl Lcg {
    const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
    const INCREMENT: u64 = 1_442_695_040_888_963_407;

    fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.0
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max.max(1)
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Debug, Clone, Serialize)]
struct GdprCascadeScenario {
    scenario_id: String,
    principal: String,
    spirit_pid: u32,
    secondary_spirit_pid: Option<u32>,
    boot_nonce: u64,
    schema: String,
    key: String,
    value: String,
    canary: Option<String>,
    distillate_embedded: bool,
    legal_hold_reason: Option<String>,
    expected_outcome: String,
    reused_pid: bool,
    reused_principal: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct GdprLeakageProbe {
    probe_id: String,
    principal: String,
    spirit_pid: u32,
    query_type: String,
    expected_subject_access_len: usize,
    canary: Option<String>,
}

fn generate_cascade(out_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut lcg = Lcg::new(0x_9_2_00_00_00_00_00_00_00u64);
    let file = std::fs::File::create(out_path)?;
    let mut writer = BufWriter::new(file);

    let stratum_counts = [
        ("cross_spirit", 10),
        ("distillate_canary", 20),
        ("legal_hold", 9),
        ("forced_failure", 1),
        ("pid_reuse", 5),
        ("zero_entry", 5),
    ];

    let schemas = ["chat", "calendar", "tasks", "profile", "logs"];
    let mut scenario_idx = 0usize;

    for (stratum, count) in stratum_counts {
        for _ in 0..count {
            scenario_idx += 1;
            let idx = scenario_idx;
            let principal = format!("user-{:03}@example.org", idx);
            let spirit_pid = 100 + (idx as u32);
            let secondary_spirit_pid = if stratum == "cross_spirit" {
                Some(spirit_pid + 1_000)
            } else {
                None
            };
            let boot_nonce = if stratum == "pid_reuse" {
                1
            } else {
                idx as u64
            };
            let schema = schemas[lcg.next_usize(schemas.len())].to_string();
            let key = format!("key-{}", hex(&lcg.next_u64().to_le_bytes()[..4]));
            let value = format!("value-{}", hex(&lcg.next_u64().to_le_bytes()));
            let canary = if stratum == "distillate_canary" {
                Some(format!(
                    "CANARY-gdpr-{idx:03}-{}",
                    hex(&lcg.next_u64().to_le_bytes()[..8])
                ))
            } else {
                None
            };
            let distillate_embedded = stratum == "distillate_canary";
            let legal_hold_reason = if stratum == "legal_hold" {
                Some(format!("legal-hold:case-{}", 1000 + idx))
            } else {
                None
            };
            let expected_outcome = match stratum {
                "legal_hold" => "held",
                "zero_entry" => "not_found",
                "forced_failure" => "failed",
                _ => "erased",
            };
            let reused_pid = stratum == "pid_reuse";
            let reused_principal = if reused_pid {
                Some(format!("reused-user-{:03}@example.org", idx))
            } else {
                None
            };

            let scenario = GdprCascadeScenario {
                scenario_id: format!("gdpr-cascade-{idx:03}"),
                principal,
                spirit_pid,
                secondary_spirit_pid,
                boot_nonce,
                schema,
                key,
                value,
                canary,
                distillate_embedded,
                legal_hold_reason,
                expected_outcome: expected_outcome.to_string(),
                reused_pid,
                reused_principal,
            };
            writer.write_all(serde_json::to_string(&scenario)?.as_bytes())?;
            writer.write_all(b"\n")?;
        }
    }

    writer.flush()?;
    Ok(())
}

fn generate_probe(out_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut lcg = Lcg::new(0x_9_2_FF_00_00_00_00_00_00u64);
    let file = std::fs::File::create(out_path)?;
    let mut writer = BufWriter::new(file);

    // 100 probes: 50 control principals that must survive the cascade,
    // 50 phantom principals that must remain absent, plus canary tokens from
    // the distillate-canary stratum are scanned separately by the test harness.
    for i in 0..100usize {
        let probe_id = format!("gdpr-probe-{i:03}");
        let (query_type, expected_len) = if i % 2 == 0 {
            ("control_principal", 1)
        } else {
            ("phantom_principal", 0)
        };
        let principal = format!("probe-{i:03}@example.org");
        let spirit_pid = 10_000 + (i as u32);
        let canary = if i % 3 == 0 {
            Some(format!(
                "CANARY-probe-{i:03}-{}",
                hex(&lcg.next_u64().to_le_bytes()[..8])
            ))
        } else {
            None
        };

        let probe = GdprLeakageProbe {
            probe_id,
            principal,
            spirit_pid,
            query_type: query_type.to_string(),
            expected_subject_access_len: expected_len,
            canary,
        };
        writer.write_all(serde_json::to_string(&probe)?.as_bytes())?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: gen_gdpr_cascade <corpus|probe> <out_path>");
        std::process::exit(2);
    }

    match args[1].as_str() {
        "corpus" => generate_cascade(&args[2]),
        "probe" => generate_probe(&args[2]),
        other => {
            eprintln!("unknown mode: {other}");
            std::process::exit(2);
        }
    }
}
