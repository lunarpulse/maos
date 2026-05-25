#![forbid(unsafe_code)]

use std::io::{self, BufRead, Read, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    loop {
        let mut header = String::new();
        match reader.read_line(&mut header) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        let content_length: usize = match header
            .trim()
            .strip_prefix("Content-Length: ")
            .and_then(|s| s.trim().parse().ok())
        {
            Some(n) => n,
            None => break,
        };

        let mut blank = String::new();
        if reader.read_line(&mut blank).unwrap_or(0) == 0 {
            break;
        }

        let mut body = vec![0u8; content_length];
        if reader.read_exact(&mut body).is_err() {
            break;
        }

        let task_id: u64 = serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|v| v["task_id"].as_u64())
            .unwrap_or(0);

        let response = serde_json::json!({
            "kind": "task.complete",
            "task_id": task_id,
            "response": "ok"
        });
        let response_bytes = serde_json::to_vec(&response).unwrap();

        let out_header = format!("Content-Length: {}\r\n\r\n", response_bytes.len());
        writer.write_all(out_header.as_bytes()).unwrap();
        writer.write_all(&response_bytes).unwrap();
        writer.flush().unwrap();
    }
}
