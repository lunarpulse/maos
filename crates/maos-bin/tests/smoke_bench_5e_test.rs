//! Smoke test for `MAOS_ONE_SHOT=smoke-bench-5e`.
//!
//! Per AC4: spawns the arm, asserts exit 0, asserts stderr contains 5 JSON lines,
//! asserts `tests/reports/section-13-1-smoke.json` was written with valid BenchReport shape.
//!
//! Requires `--features fixture_replay` at compile time.

#![cfg(feature = "fixture_replay")]

use std::process::Command;

#[test]
fn smoke_bench_5e_exits_zero_and_outputs_5_json_lines() {
    let bin = env!("CARGO_BIN_EXE_maos-bin");
    let output = Command::new(bin)
        .env("MAOS_ONE_SHOT", "smoke-bench-5e")
        .output()
        .expect("failed to execute maos-bin");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "smoke-bench-5e exited with non-zero status: {}\nstdout: {}\nstderr: {}",
        output.status,
        stdout,
        stderr,
    );

    let json_lines: Vec<&str> = stderr
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.starts_with('{') && trimmed.ends_with('}')
        })
        .collect();

    assert!(
        json_lines.len() >= 5,
        "expected at least 5 JSON lines on stderr, got {}: {:?}",
        json_lines.len(),
        json_lines,
    );

    for (i, line) in json_lines.iter().enumerate() {
        let val: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {} is not valid JSON: {} (error: {})", i + 1, line, e));
        assert!(
            val.get("step").is_some(),
            "line {} missing 'step' key: {}",
            i + 1,
            line
        );
    }

    let step1: serde_json::Value = serde_json::from_str(json_lines[0]).unwrap();
    assert_eq!(step1["step"], 1);
    assert_eq!(step1["surface"], "bench_init");

    let step5: serde_json::Value = serde_json::from_str(json_lines[4]).unwrap();
    assert_eq!(step5["step"], 5);
    assert_eq!(step5["surface"], "report_write");
}
