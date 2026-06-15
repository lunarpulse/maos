//! Story 9.4 R-AG1 — air-gap no-network-symbols CI gate.
//!
//! Scans the `maos-bin` air-gap binary's symbol table for network-related
//! symbols (reqwest, hyper client, tokio::net::TcpStream, socket, DNS crates)
//! and FAILS if any are present. A dirty-fixture bite test proves the gate
//! can actually catch violations (a gate that can't fail is theater).

use std::path::Path;
use std::process::Command;

/// Network-related symbol substrings that MUST NOT appear in an air-gap binary.
const NETWORK_SYMBOLS: &[&str] = &[
    "reqwest",
    "hyper::client",
    "hyper_util::client",
    "tokio::net::TcpStream",
    "tokio::net::tcp",
    "TcpListener",
    "connect_tcp",
    "getaddrinfo",
    "dns_lookup",
    "trust_dns",
    "hickory_resolver",
    "StreamableHttpTransport",
    "MobilePushHttp",
    "TcpA2ATransport",
];

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Report {
    pub passed: bool,
    pub binary_path: String,
    pub network_symbols_found: Vec<String>,
    pub dirty_fixture_result: Option<DirtyFixtureResult>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DirtyFixtureResult {
    pub fixture_path: String,
    pub correctly_rejected: bool,
    pub symbols_found: Vec<String>,
}

pub fn run(
    binary_path: &str,
    build_first: bool,
    dirty_fixture: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if build_first {
        eprintln!("check-air-gap: building air-gap binary...");
        let status = Command::new("cargo")
            .args([
                "build",
                "-p",
                "maos-bin",
                "--no-default-features",
                "--features",
                "air-gap",
            ])
            .status()
            .map_err(|e| format!("cargo build invocation failed: {e}"))?;
        if !status.success() {
            return Err(
                "cargo build -p maos-bin --no-default-features --features air-gap failed".into(),
            );
        }
    }

    let report = check(binary_path, dirty_fixture)?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| format!("json serialization: {e}"))?
        );
    } else if report.passed {
        println!("check-air-gap: PASSED (zero network symbols in air-gap binary)");
        if let Some(df) = &report.dirty_fixture_result {
            if df.correctly_rejected {
                println!(
                    "check-air-gap: dirty-fixture bite PASSED ({} network symbols detected)",
                    df.symbols_found.len()
                );
            }
        }
    } else {
        eprintln!(
            "check-air-gap: FAILED — network symbols found: {:?}",
            report.network_symbols_found
        );
        if let Some(df) = &report.dirty_fixture_result {
            if !df.correctly_rejected {
                eprintln!(
                    "check-air-gap: dirty-fixture bite FAILED — gate did NOT detect network symbols in fixture"
                );
            }
        }
    }

    if report.passed {
        Ok(())
    } else {
        Err(format!(
            "network symbols in air-gap binary: {:?}",
            report.network_symbols_found
        ))
    }
}

pub fn check(binary_path: &str, dirty_fixture: Option<&str>) -> Result<Report, String> {
    let symbols = extract_symbols(Path::new(binary_path))?;
    let network_found = find_network_symbols(&symbols);

    let dirty_fixture_result = match dirty_fixture {
        Some(fixture_path) => {
            let fixture_symbols = extract_symbols(Path::new(fixture_path))?;
            let fixture_network = find_network_symbols(&fixture_symbols);
            Some(DirtyFixtureResult {
                fixture_path: fixture_path.to_string(),
                // The dirty fixture MUST contain network symbols — if the gate
                // doesn't find them, the gate itself is broken.
                correctly_rejected: !fixture_network.is_empty(),
                symbols_found: fixture_network,
            })
        }
        None => None,
    };

    let mut passed = network_found.is_empty();
    // If dirty fixture was requested but gate failed to detect its symbols,
    // the gate is unreliable → overall FAIL.
    if let Some(df) = &dirty_fixture_result {
        if !df.correctly_rejected {
            passed = false;
        }
    }

    Ok(Report {
        passed,
        binary_path: binary_path.to_string(),
        network_symbols_found: network_found,
        dirty_fixture_result,
    })
}

fn find_network_symbols(symbols: &[String]) -> Vec<String> {
    symbols
        .iter()
        .filter(|s| NETWORK_SYMBOLS.iter().any(|ns| s.contains(ns)))
        .cloned()
        .collect()
}

fn extract_symbols(binary_path: &Path) -> Result<Vec<String>, String> {
    if !binary_path.exists() {
        return Err(format!("binary not found: {}", binary_path.display()));
    }

    let bin_str = binary_path
        .to_str()
        .ok_or_else(|| "binary path is not valid UTF-8".to_string())?;

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("nm")
            .args(["--demangle", bin_str])
            .output()
            .map_err(|e| format!("nm invocation failed: {e}"))?;
        if !output.status.success() {
            // nm may fail on stripped binaries; try objdump as fallback
            let output = Command::new("objdump")
                .args(["-t", "-C", bin_str])
                .output()
                .map_err(|e| format!("objdump invocation failed: {e}"))?;
            if !output.status.success() {
                return Err("both nm and objdump failed on the binary".into());
            }
            let text = String::from_utf8_lossy(&output.stdout);
            return Ok(text.lines().map(|s| s.to_string()).collect());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().map(|s| s.to_string()).collect())
    }

    #[cfg(target_os = "macos")]
    {
        // `-g` shows all global symbols (both defined and undefined). The
        // previous `-gU` only listed undefined externals, missing defined
        // networking symbols that were statically linked into the binary.
        let output = Command::new("nm")
            .args(["-g", bin_str])
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
        let _ = bin_str;
        Err("unsupported OS for symbol extraction".into())
    }
}
