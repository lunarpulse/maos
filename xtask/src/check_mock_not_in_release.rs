#![forbid(unsafe_code)]

//! Story 4.1 A2 — fail the build if `MockHaltResolver` symbol appears
//! in the release-mode `target/release/maos` binary symbol table.
//!
//! Mechanism: invoke `cargo build --release -p maos-bin` (or assume
//! already built), then run `nm` (Linux/macOS) or `dumpbin /symbols`
//! (Windows) over the output binary; grep the output for
//! `MockHaltResolver`. Match = fail.
//!
//! Why this exists: Epic 3 wired `MockHaltResolver` in production
//! `main.rs` as a v0.3-β bootstrap; Story 4.1 swaps it for
//! `KernelHaltResolver`. Without this gate, a future regression
//! (e.g., revert of the swap, or a new arm that re-uses Mock for
//! "convenience") would land silently — see Epic 3 retro §What Was
//! Challenging §2.

use std::path::Path;
use std::process::Command;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub binary_path: String,
    pub forbidden_symbols_found: Vec<String>,
}

const FORBIDDEN_PRODUCTION_SYMBOLS: &[&str] = &["MockHaltResolver", "FailingHaltResolver"];

pub fn run(binary_path: &str, build_first: bool, json: bool) -> Result<(), String> {
    if build_first {
        let status = Command::new("cargo")
            .args(["build", "--release", "-p", "maos-bin", "--locked"])
            .status()
            .map_err(|e| format!("cargo build invocation failed: {e}"))?;
        if !status.success() {
            return Err("cargo build --release -p maos-bin failed".into());
        }
    }
    let report = check(binary_path)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!("check-mock-not-in-release: PASSED (zero forbidden symbols)");
    } else {
        eprintln!(
            "check-mock-not-in-release: FAILED — forbidden symbols found: {:?}",
            report.forbidden_symbols_found
        );
    }
    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "forbidden symbols in release binary: {:?}",
            report.forbidden_symbols_found
        ))
    }
}

pub fn check(binary_path: &str) -> Result<Report, String> {
    let symbols = extract_symbols(Path::new(binary_path))?;
    let forbidden: Vec<String> = symbols
        .iter()
        .filter(|s| {
            FORBIDDEN_PRODUCTION_SYMBOLS
                .iter()
                .any(|fs| s.contains(*fs))
        })
        .cloned()
        .collect();
    Ok(Report {
        passed: forbidden.is_empty(),
        binary_path: binary_path.to_string(),
        forbidden_symbols_found: forbidden,
    })
}

fn extract_symbols(binary_path: &Path) -> Result<Vec<String>, String> {
    if !binary_path.exists() {
        return Err(format!("binary not found: {}", binary_path.display()));
    }

    #[cfg(target_os = "linux")]
    {
        let bin_str = binary_path
            .to_str()
            .ok_or_else(|| "binary path is not valid UTF-8".to_string())?;
        let output = Command::new("nm")
            .args(["--demangle", bin_str])
            .output()
            .map_err(|e| format!("nm invocation failed: {e}"))?;
        if !output.status.success() {
            return Err("nm exited with non-zero status".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().map(|s| s.to_string()).collect())
    }

    #[cfg(target_os = "macos")]
    {
        let bin_str = binary_path
            .to_str()
            .ok_or_else(|| "binary path is not valid UTF-8".to_string())?;
        let output = Command::new("nm")
            .args(["-gU", bin_str])
            .output()
            .map_err(|e| format!("nm invocation failed: {e}"))?;
        if !output.status.success() {
            return Err("nm exited with non-zero status".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().map(|s| s.to_string()).collect())
    }

    #[cfg(target_os = "windows")]
    {
        let bin_str = binary_path
            .to_str()
            .ok_or_else(|| "binary path is not valid UTF-8".to_string())?;
        let output = Command::new("dumpbin")
            .args(["/symbols", bin_str])
            .output()
            .map_err(|e| format!("dumpbin invocation failed: {e}"))?;
        if !output.status.success() {
            return Err("dumpbin exited with non-zero status".into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().map(|s| s.to_string()).collect())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = binary_path;
        Err("unsupported OS for symbol extraction".into())
    }
}
