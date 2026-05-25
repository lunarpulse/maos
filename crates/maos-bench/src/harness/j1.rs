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

#[cfg(test)]
mod tests {
    use super::*;

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
