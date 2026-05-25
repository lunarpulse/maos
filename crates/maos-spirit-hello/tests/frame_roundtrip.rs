#![forbid(unsafe_code)]

use std::io::{BufRead, Read, Write};
use std::process::{Command, Stdio};

#[test]
fn frame_roundtrip_single_frame() {
    let bin = env!("CARGO_BIN_EXE_hello-spirit-bench");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn hello-spirit-bench");

    let mut stdin = child.stdin.take().expect("stdin not captured");
    let stdout = child.stdout.take().expect("stdout not captured");
    let mut reader = std::io::BufReader::new(stdout);

    let request = serde_json::json!({
        "kind": "task.assign",
        "task_id": 42u64,
        "content": "echo:test"
    });
    let payload = serde_json::to_vec(&request).unwrap();
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    stdin.write_all(header.as_bytes()).unwrap();
    stdin.write_all(&payload).unwrap();
    stdin.flush().unwrap();

    let mut resp_header = String::new();
    reader.read_line(&mut resp_header).unwrap();
    assert!(
        resp_header.contains("Content-Length:"),
        "bad header: {:?}",
        resp_header
    );

    let content_length: usize = resp_header
        .trim()
        .strip_prefix("Content-Length: ")
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let mut blank = String::new();
    reader.read_line(&mut blank).unwrap();

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).unwrap();
    let response: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(response["kind"], "task.complete");
    assert_eq!(response["task_id"], 42);
    assert_eq!(response["response"], "ok");

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn frame_roundtrip_multiple_frames() {
    let bin = env!("CARGO_BIN_EXE_hello-spirit-bench");
    let mut child = Command::new(bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn hello-spirit-bench");

    let mut stdin = child.stdin.take().expect("stdin not captured");
    let stdout = child.stdout.take().expect("stdout not captured");
    let mut reader = std::io::BufReader::new(stdout);

    for i in 0..10u64 {
        let request = serde_json::json!({
            "kind": "task.assign",
            "task_id": i,
            "content": format!("echo:{}", i)
        });
        let payload = serde_json::to_vec(&request).unwrap();
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());
        stdin.write_all(header.as_bytes()).unwrap();
        stdin.write_all(&payload).unwrap();
        stdin.flush().unwrap();

        let mut resp_header = String::new();
        reader.read_line(&mut resp_header).unwrap();

        let content_length: usize = resp_header
            .trim()
            .strip_prefix("Content-Length: ")
            .unwrap()
            .trim()
            .parse()
            .unwrap();

        let mut blank = String::new();
        reader.read_line(&mut blank).unwrap();

        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(response["task_id"], i);
        assert_eq!(response["response"], "ok");
    }

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
