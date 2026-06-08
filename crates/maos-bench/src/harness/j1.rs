#![forbid(unsafe_code)]

//! J1 measurement — founder-loop CliWrapper IPC overhead.
//!
//! Spawns a `hello-spirit` subprocess as a synthetic CliWrapper-shaped
//! Spirit, sends ≥1000 echo invocations over the Spirit Wire Protocol,
//! and samples per-call round-trip latency.
//!
//! ## Subject
//!
//! `crates/maos-spirit-hello/` subprocess Spirit.
//!
//! ## Failure modes
//!
//! - Subprocess crash → `BenchError::SubprocessCrash`
//! - Subprocess hang → `BenchError::Hang` (1s timeout per invocation)
//! - Outlier handling: NO TRIMMING. Every sample lands in the histogram.

use crate::harness::build_journey_result;
use crate::report::JourneyResult;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const J1_P95_BUDGET_US: u64 = 25_000;
const PER_INVOCATION_TIMEOUT_MS: u64 = 1000;

#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    #[error("subprocess crashed: {0}")]
    SubprocessCrash(String),
    #[error("subprocess hang: invocation {0} timed out after {1}ms")]
    Hang(usize, u64),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialize(String),
}

pub struct J1Config {
    pub invocation_count: u64,
    pub spirit_binary: String,
}

impl Default for J1Config {
    fn default() -> Self {
        Self {
            invocation_count: 1000,
            spirit_binary: "hello-spirit-bench".to_string(),
        }
    }
}

fn spawn_bench_spirit(binary: &str) -> Result<Child, BenchError> {
    Command::new(binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| BenchError::SubprocessCrash(format!("failed to spawn '{}': {}", binary, e)))
}

fn send_frame(stdin: &mut dyn Write, task_id: u64, content: &str) -> Result<(), BenchError> {
    let frame = serde_json::json!({
        "kind": "task.assign",
        "task_id": task_id,
        "content": content
    });
    let payload = serde_json::to_vec(&frame)
        .map_err(|e| BenchError::Serialize(e.to_string()))?;
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(&payload)?;
    stdin.flush()?;
    Ok(())
}

fn read_frame(reader: &mut dyn BufRead) -> Result<String, BenchError> {
    let mut header = String::new();
    reader.read_line(&mut header)?;
    if header.is_empty() {
        return Err(BenchError::SubprocessCrash("stdin closed (subprocess dead)".into()));
    }
    let content_length: usize = header
        .trim()
        .strip_prefix("Content-Length: ")
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| BenchError::SubprocessCrash(format!("bad Content-Length header: {:?}", header)))?;

    let mut blank = String::new();
    reader.read_line(&mut blank)?;

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    String::from_utf8(body)
        .map_err(|e| BenchError::SubprocessCrash(format!("non-UTF8 response: {}", e)))
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }
    fn kill_and_reap(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.kill_and_reap();
    }
}

pub fn run_j1_measurement(config: &J1Config) -> Result<JourneyResult, BenchError> {
    let raw_child = spawn_bench_spirit(&config.spirit_binary)?;
    let mut guard = ChildGuard::new(raw_child);
    let child = guard.child.as_mut().unwrap();
    let mut stdin = child.stdin.take().expect("stdin not captured");
    let stdout = child.stdout.take().expect("stdout not captured");
    let mut reader = BufReader::new(stdout);

    let mut samples_us = Vec::with_capacity(config.invocation_count as usize);

    let measurement_result: Result<(), BenchError> = (|| {
        for i in 0..config.invocation_count {
            let content = format!("echo:{}", i);
            let t0 = Instant::now();
            send_frame(&mut stdin, i, &content)?;
            let _response = read_frame(&mut reader)?;
            let elapsed = t0.elapsed();

            if elapsed > Duration::from_millis(PER_INVOCATION_TIMEOUT_MS) {
                return Err(BenchError::Hang(i as usize, PER_INVOCATION_TIMEOUT_MS));
            }

            let latency_us = elapsed.as_micros() as u64;
            samples_us.push(latency_us);
        }
        Ok(())
    })();

    if measurement_result.is_err() {
        guard.kill_and_reap();
        return Err(measurement_result.unwrap_err());
    }

    drop(stdin);
    let status = child.wait()?;
    guard.child = None;

    if !status.success() {
        return Err(BenchError::SubprocessCrash(format!(
            "subprocess exited with {}",
            status
        )));
    }

    let result = build_journey_result("J1", config.invocation_count, &samples_us, J1_P95_BUDGET_US);
    Ok(result)
}

// ────────────────────────────────────────────────────────────────────────────
// Story 8.12 AC4 — J1 measured THROUGH THE REAL `runtime.rs` bridge.
//
// The synthetic floor above (`run_j1_measurement` against `hello-spirit-bench`)
// is retained. This variant drives a request→response round-trip through the
// REAL `spawn_and_bridge` reader-thread framing path, against a deterministic
// echo test-CLI doing ~zero work — so the measurement isolates the bridge IPC
// overhead (stdin write + child echo + reader-thread frame + bounded-channel
// deliver), NOT the agent's work and NOT the SQLite journal write.
//
// Polarity (Murat): on shared CI this is REPORTED Tier-2 evidence gated only on
// a generous ceiling (2×budget = 50ms P95) to catch real regressions without
// jitter-flake; the strict 25ms P95 is hard-gated on a pinned bench runner.
// Warmup iterations are discarded; N ≥ 100 (200 preferred); P50/P95/P99/max.
// ────────────────────────────────────────────────────────────────────────────

/// Generous CI ceiling (2× the §13.1 budget) — catches real regressions on a
/// shared runner without jitter-flake. The strict `J1_P95_BUDGET_US` is the
/// pinned-runner gate.
pub const J1_CI_CEILING_US: u64 = 50_000;

pub struct J1BridgeConfig {
    pub iterations: u64,
    pub warmup: u64,
}

impl Default for J1BridgeConfig {
    fn default() -> Self {
        Self {
            iterations: 200,
            warmup: 20,
        }
    }
}

/// Measure J1 IPC overhead through the real CliWrapper bridge. Returns the
/// `JourneyResult` (gated by the caller against either the strict budget or the
/// generous CI ceiling).
pub fn run_j1_bridge_measurement(config: &J1BridgeConfig) -> Result<JourneyResult, BenchError> {
    use maos_kernel_core::lifecycle::cli_wrapper::{
        argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeSpawnSpec,
    };
    use maos_kernel_core::security::manifest::{CliWrapperControlChannel, CliWrapperStdioShape};

    // Deterministic echo test-CLI: read a line on stdin, echo it on stdout. Pure
    // POSIX `sh`, zero work per iteration.
    let argv_prefix = vec!["-c".to_string()];
    let spec = BridgeSpawnSpec {
        program: "sh".to_string(),
        argv_prefix: argv_prefix.clone(),
        task_args: vec!["while IFS= read -r line; do printf '%s\\n' \"$line\"; done".to_string()],
        expected_argv_prefix_hash: argv_prefix_hash(&argv_prefix),
        from_spirit_id: "bench".to_string(),
        stdio_shape: CliWrapperStdioShape::NdjsonOverStdio,
        control_channel: CliWrapperControlChannel::StdinCommands,
        shutdown_signal: None,
        channel_capacity: 16,
        backpressure: Backpressure::Block,
        env: vec![],
    };
    let mut bridge =
        spawn_and_bridge(spec).map_err(|e| BenchError::SubprocessCrash(format!("bridge: {e:?}")))?;

    let total = config.warmup + config.iterations;
    let mut samples_us: Vec<u64> = Vec::with_capacity(config.iterations as usize);

    for i in 0..total {
        let t0 = Instant::now();
        bridge
            .write_stdin_line(format!("ping:{i}").as_bytes())
            .map_err(|e| BenchError::Io(e))?;
        let line = bridge
            .recv_line()
            .ok_or_else(|| BenchError::SubprocessCrash("echo CLI closed mid-bench".into()))?;
        let elapsed = t0.elapsed();
        // Sanity: the echoed line round-tripped.
        debug_assert!(!line.1.is_empty());
        if i >= config.warmup {
            samples_us.push(elapsed.as_micros() as u64);
        }
    }

    // Clean shutdown (Drop kills+reaps; explicit unload closes stdin first).
    let _ = bridge.on_unload();

    Ok(build_journey_result(
        "J1-bridge",
        config.iterations,
        &samples_us,
        J1_P95_BUDGET_US,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j1_bridge_overhead_within_ci_ceiling() {
        // Tier-2 reported evidence gated on the generous CI ceiling (50ms P95).
        let cfg = J1BridgeConfig {
            iterations: 120,
            warmup: 20,
        };
        let result = run_j1_bridge_measurement(&cfg).expect("bridge measurement");
        let p95 = result.p95_us;
        eprintln!(
            "J1-bridge: P50={}us P95={}us P99={}us max={}us (N={})",
            result.p50_us, result.p95_us, result.p99_us, result.max_us, cfg.iterations
        );
        assert!(
            p95 < J1_CI_CEILING_US,
            "J1 bridge IPC P95={p95}us exceeded the generous CI ceiling {J1_CI_CEILING_US}us \
             (real regression — fix our code, do NOT migrate to in-process to mask it)"
        );
    }

    #[test]
    fn j1_config_defaults() {
        let cfg = J1Config::default();
        assert_eq!(cfg.invocation_count, 1000);
        assert_eq!(cfg.spirit_binary, "hello-spirit-bench".to_string());
    }

    #[test]
    fn j1_budget_constant() {
        assert_eq!(J1_P95_BUDGET_US, 25_000);
    }

    #[test]
    fn bench_error_display() {
        let e = BenchError::SubprocessCrash("boom".into());
        assert!(e.to_string().contains("boom"));
    }

    #[test]
    fn send_frame_produces_valid_header() {
        let mut buf = Vec::new();
        send_frame(&mut buf, 42, "echo:test").unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains("\r\n\r\n"));
        assert!(output.contains("\"task_id\":42"));
        assert!(output.contains("echo:test"));
    }

    #[test]
    fn read_frame_parses_content_length() {
        let payload = r#"{"kind":"task.complete","task_id":1,"response":"ok"}"#;
        let input = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
        let mut reader = std::io::Cursor::new(input.as_bytes());
        let result = read_frame(&mut reader).unwrap();
        assert_eq!(result, payload);
    }

    #[test]
    fn read_frame_empty_stream_returns_crash() {
        let mut reader = std::io::Cursor::new(b"");
        let result = read_frame(&mut reader);
        assert!(result.is_err());
    }
}
