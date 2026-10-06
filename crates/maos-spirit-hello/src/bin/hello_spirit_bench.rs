#![forbid(unsafe_code)]

use std::io;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    loop {
        let body = match maos_frame_codec::read_frame(&mut reader) {
            Ok(Some(body)) => body,
            Ok(None) => break,
            Err(error) => {
                eprintln!("invalid input frame: {error}");
                std::process::exit(1);
            }
        };

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

        maos_frame_codec::write_frame(&mut writer, &response_bytes).unwrap();
    }
}
