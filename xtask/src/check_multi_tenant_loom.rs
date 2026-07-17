#![forbid(unsafe_code)]

//! Story 13.1 — physical multi-tenant Loom wall gate.
//!
//! Hermetic legs are [`BindingClass::Blocking`] at development HEAD. The two
//! live Postgres legs are [`BindingClass::AdvisorySubstrate`]: absence emits a
//! WOULD-HAVE-BLOCKED banner; presence makes any RED result blocking.

use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;

use crate::gate_common::{dev_enforced_red_blocks, emit_command, read_disposition, BindingClass};

const GATE_NAME: &str = "check-multi-tenant-loom";
const ABSENT_SUCCESSORS: &[&str] = &[
    "13.2 per-team cryptographic key boundary",
    "13.3 widened caller-facing tenant error taxonomy",
    "13.5b collective GDPR erase/legal-hold cascade",
    "13.5c production Spirit routing, refresh wiring, refusal audit, and TL isolation",
    "13.6 three-team product journey",
];

struct TestLeg {
    name: &'static str,
    class: BindingClass,
    args: &'static [&'static str],
}

#[derive(serde::Serialize)]
struct LegResult {
    name: &'static str,
    binding: &'static str,
    attempted: bool,
    substrate_present: bool,
    green: bool,
    detail: String,
}

impl LegResult {
    fn blocks(&self, class: BindingClass) -> bool {
        !self.green && dev_enforced_red_blocks(class, self.substrate_present)
    }
}

fn class_name(class: BindingClass) -> &'static str {
    match class {
        BindingClass::Blocking => "blocking",
        BindingClass::AdvisorySubstrate => "advisory-substrate",
    }
}

fn live_substrate_present() -> bool {
    ["MAOS_TEST_POSTGRES_TEAM_A", "MAOS_TEST_POSTGRES_TEAM_B"]
        .iter()
        .all(|name| std::env::var(name).is_ok_and(|value| !value.trim().is_empty()))
}

fn run_test_leg(leg: &TestLeg, substrate_present: bool) -> LegResult {
    if leg.class == BindingClass::AdvisorySubstrate && !substrate_present {
        return LegResult {
            name: leg.name,
            binding: class_name(leg.class),
            attempted: false,
            substrate_present: false,
            green: false,
            detail: "two-datname Postgres substrate absent".to_string(),
        };
    }

    let output = match Command::new("cargo").args(leg.args).output() {
        Ok(output) => output,
        Err(error) => {
            return LegResult {
                name: leg.name,
                binding: class_name(leg.class),
                attempted: true,
                substrate_present,
                green: false,
                detail: format!("could not start cargo: {error}"),
            };
        }
    };
    let transcript = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let non_vacuous = transcript.contains("running 1 test") && transcript.contains("1 passed");
    LegResult {
        name: leg.name,
        binding: class_name(leg.class),
        attempted: true,
        substrate_present,
        green: output.status.success() && non_vacuous,
        detail: if !output.status.success() {
            transcript
        } else if !non_vacuous {
            format!("vacuous: expected exactly one attempted passing test\n{transcript}")
        } else {
            "running 1 test; 1 passed".to_string()
        },
    }
}

fn write_step_summary(text: &str) {
    if let Ok(path) = std::env::var("GITHUB_STEP_SUMMARY") {
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| writeln!(file, "{text}"));
    }
}

pub fn run(json: bool) -> Result<(), String> {
    let disposition = read_disposition(GATE_NAME)?;
    if !matches!(
        disposition.get("v2_2").map(String::as_str),
        Some("blocking")
    ) {
        return Err(format!(
            "{GATE_NAME}: registry defect — v2_2 disposition must be blocking"
        ));
    }

    let live_present = live_substrate_present();
    let specs = [
        TestLeg {
            name: "three-site-chokepoint",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "read_path_chokepoint",
                "team_guard_is_exactly_the_three_spirit_entry_points",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "tenant-map-hermetic-matrix",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-bin",
                "--test",
                "tenant_map_13_1",
                "tenant_map_13_1_gate_matrix",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "manifest-option-a-plus-matrix",
            class: BindingClass::Blocking,
            args: &[
                "test",
                "-p",
                "maos-cohort",
                "--lib",
                "manifest::tests::tenant_manifest_option_a_plus_gate_matrix",
                "--",
                "--exact",
            ],
        },
        TestLeg {
            name: "two-datname-physical-absence",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "tenant_wall_two_datname_physical_absence_and_assignment_matrix",
                "--",
                "--ignored",
                "--exact",
            ],
        },
        TestLeg {
            name: "d1-forged-stamp-served-boundary",
            class: BindingClass::AdvisorySubstrate,
            args: &[
                "test",
                "-p",
                "maos-loom-lite",
                "--test",
                "tenant_wall_live",
                "tenant_wall_d1_forged_stamp_is_still_served_boundary",
                "--",
                "--ignored",
                "--exact",
            ],
        },
    ];
    let mut legs: Vec<(BindingClass, LegResult)> = specs
        .iter()
        .map(|spec| {
            let substrate = spec.class == BindingClass::Blocking || live_present;
            (spec.class, run_test_leg(spec, substrate))
        })
        .collect();

    let kernel_green = crate::check_kernel_baseline::check()?.passed;
    legs.push((
        BindingClass::Blocking,
        LegResult {
            name: "kernel-zero-delta",
            binding: class_name(BindingClass::Blocking),
            attempted: true,
            substrate_present: true,
            green: kernel_green,
            detail: if kernel_green {
                "kernel baseline actual=pinned=23202".to_string()
            } else {
                "kernel baseline mismatch".to_string()
            },
        },
    ));

    let blockers: Vec<&LegResult> = legs
        .iter()
        .filter_map(|(class, leg)| leg.blocks(*class).then_some(leg))
        .collect();
    let skipped_live: Vec<&LegResult> = legs
        .iter()
        .map(|(_, leg)| leg)
        .filter(|leg| !leg.attempted)
        .collect();
    let oracle_green = legs.iter().all(|(_, leg)| leg.green);

    if !skipped_live.is_empty() {
        let banner = format!(
            "## ⚠️ Multi-Tenant Loom Gate: WOULD HAVE BLOCKED SHIP (v2.2)\n\
             Live two-datname Postgres substrate was absent; skipped: {}.\n\
             Hermetic legs still bind at HEAD. ABSENT successors: {}.",
            skipped_live
                .iter()
                .map(|leg| leg.name)
                .collect::<Vec<_>>()
                .join(", "),
            ABSENT_SUCCESSORS.join("; ")
        );
        emit_command(json, "warning", &banner.replace('\n', " "));
        write_step_summary(&banner);
    }

    if !blockers.is_empty() {
        let detail = blockers
            .iter()
            .map(|leg| format!("{}: {}", leg.name, leg.detail))
            .collect::<Vec<_>>()
            .join("\n");
        emit_command(json, "error", &format!("{GATE_NAME} RED: {detail}"));
        write_step_summary(&format!("## ❌ Multi-Tenant Loom Gate: RED\n{detail}"));
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": blockers.is_empty(),
                "oracle_green": oracle_green,
                "advisory": blockers.is_empty() && !oracle_green,
                "disposition": disposition,
                "legs": legs.iter().map(|(_, leg)| leg).collect::<Vec<_>>(),
                "absent_successors": ABSENT_SUCCESSORS,
            })
        );
    } else if blockers.is_empty() {
        println!(
            "{GATE_NAME}: PASSED ({}; {} absent successors declared)",
            if oracle_green {
                "oracle green"
            } else {
                "live substrate advisory"
            },
            ABSENT_SUCCESSORS.len()
        );
    }

    if blockers.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{GATE_NAME}: {} blocking leg(s) RED",
            blockers.len()
        ))
    }
}
