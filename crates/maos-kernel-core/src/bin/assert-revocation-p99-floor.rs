#![forbid(unsafe_code)]

//! Assertion binary for NFR-Rel-9 revocation propagation p99 floor.
//!
//! Parses the bench-emitted report at `tests/reports/revocation-propagation-*.json`
//! and asserts p99 ≤ floor.
//!
//! Usage:
//!   cargo run -p maos-kernel-core --bin assert-revocation-p99-floor -- \
//!     tests/reports/revocation-propagation-<sha>.json --floor-ns 5000000000

use std::env;
use std::path::Path;

#[derive(Debug, serde::Deserialize)]
struct Report {
    p99_ns: u64,
    p50_ns: u64,
    mean_ns: u64,
    n_iterations: usize,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <report.json> --floor-ns <nanoseconds>", args[0]);
        std::process::exit(1);
    }

    let report_path = &args[1];
    let floor_ns: u64 = args[3].parse().unwrap_or_else(|_| {
        eprintln!("Invalid floor-ns value: {}", args[3]);
        std::process::exit(1);
    });

    let path = Path::new(report_path);
    if !path.exists() {
        // If the specific file doesn't exist, try globbing for the pattern
        let parent = path.parent().unwrap_or(Path::new("."));
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.contains('*') {
            let entries: Vec<_> = std::fs::read_dir(parent)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_str()
                        .map(|s| s.contains("revocation-propagation"))
                        .unwrap_or(false)
                })
                .collect();
            if entries.is_empty() {
                eprintln!("No revocation propagation report found at {report_path}");
                std::process::exit(1);
            }
            let content = std::fs::read_to_string(entries[0].path()).unwrap();
            check_report(&content, floor_ns);
            return;
        }
        eprintln!("Report file not found: {report_path}");
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(path).unwrap();
    check_report(&content, floor_ns);
}

fn check_report(content: &str, floor_ns: u64) {
    let report: Report = serde_json::from_str(content).unwrap_or_else(|e| {
        eprintln!("Failed to parse report JSON: {e}");
        std::process::exit(1);
    });

    println!(
        "revocation-p99: n={}, p50={}ms, mean={}ms, p99={}ms, floor={}ms",
        report.n_iterations,
        report.p50_ns / 1_000_000,
        report.mean_ns / 1_000_000,
        report.p99_ns / 1_000_000,
        floor_ns / 1_000_000,
    );

    if report.p99_ns > floor_ns {
        eprintln!(
            "FAIL: p99 {}ms exceeds floor {}ms",
            report.p99_ns / 1_000_000,
            floor_ns / 1_000_000
        );
        std::process::exit(1);
    }

    println!("PASS: p99 within floor");
}
