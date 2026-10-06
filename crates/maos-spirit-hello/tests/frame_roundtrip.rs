#![forbid(unsafe_code)]

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
    maos_frame_codec::write_frame(&mut stdin, &payload).unwrap();
    let body = maos_frame_codec::read_frame(&mut reader).unwrap().unwrap();
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
        maos_frame_codec::write_frame(&mut stdin, &payload).unwrap();
        let body = maos_frame_codec::read_frame(&mut reader).unwrap().unwrap();
        let response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(response["task_id"], i);
        assert_eq!(response["response"], "ok");
    }

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
