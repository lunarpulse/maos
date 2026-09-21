use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use maos_cli::door_client::{self, DoorConfig, DoorError};

fn server_with_response(response: Vec<u8>) -> (DoorConfig, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture door");
    let endpoint = listener.local_addr().expect("fixture address");
    let worker = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fixture timeout");
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).expect("read request");
        let _ = stream.write_all(&response);
    });
    (
        DoorConfig {
            endpoint,
            token: "fixture-token".to_owned(),
        },
        worker,
    )
}

fn exchange(response: Vec<u8>) -> Result<door_client::Response, DoorError> {
    let (config, worker) = server_with_response(response);
    let result = door_client::exchange(
        &config,
        "GET",
        "/v1/daemon",
        None,
        Duration::from_millis(100),
    );
    worker.join().expect("fixture worker");
    result
}

#[test]
fn successful_response_must_match_its_content_length() {
    let error = exchange(
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{}"
            .to_vec(),
    )
    .err()
    .expect("truncated success must fail");
    match error {
        DoorError::Unresponsive(detail) => assert!(detail.contains("truncated"), "{detail}"),
        _ => panic!("truncated response must be classified as unresponsive"),
    }
}

#[test]
fn response_larger_than_the_client_cap_is_refused() {
    let body = vec![b' '; 1024 * 1024];
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    response.extend_from_slice(&body);

    let error = exchange(response)
        .err()
        .expect("oversized response must fail before parsing");
    match error {
        DoorError::Unresponsive(detail) => assert!(detail.contains("exceeds"), "{detail}"),
        _ => panic!("oversized response must be classified as unresponsive"),
    }
}
