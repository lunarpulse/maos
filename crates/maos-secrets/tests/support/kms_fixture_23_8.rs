#![allow(dead_code)]

//! Story 23-8 — an in-process `maos.kms.v1` endpoint for the tests of
//! `maos-secrets` and `maos-bin`. A `std::net::TcpListener` on `127.0.0.1:0`; no
//! container, no network. It wraps with the reference `LocalMasterKeyKms`, so a
//! wrapped key is real ciphertext. Included by both test crates through
//! `#[path]`; it takes no dependency beyond `maos-secrets` and `maos-domain`.

use maos_domain::ports::KeyManagementPort;
use maos_secrets::LocalMasterKeyKms;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18u32, 12, 6, 0].iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn b64_decode(text: &str) -> Option<Vec<u8>> {
    let trimmed = text.trim_end_matches('=');
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0u32;
    for c in trimmed.bytes() {
        let value = ALPHABET.iter().position(|a| *a == c)? as u32;
        acc = (acc << 6) | value;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

/// How the endpoint answers every request.
#[derive(Clone, Debug)]
pub enum Mode {
    /// A correct `maos.kms.v1` endpoint.
    Normal,
    /// Accept the connection and never answer.
    Hang,
    /// `302` to a path of this same endpoint.
    Redirect,
    /// A fixed status and body for every request.
    Fixed(u16, String),
    /// Correct answers that carry a different `version`.
    WrongVersion,
}

#[derive(Clone, Debug)]
pub struct Req {
    pub method: String,
    pub path: String,
    pub authorization: Option<String>,
    pub body: String,
}

struct State {
    mode: Mutex<Mode>,
    requests: Mutex<Vec<Req>>,
    held: Mutex<Vec<TcpStream>>,
    local: LocalMasterKeyKms,
}

pub struct KmsFixture {
    pub addr: SocketAddr,
    state: Arc<State>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl KmsFixture {
    pub fn start() -> Self {
        Self::bind("127.0.0.1:0".parse().unwrap(), None)
    }

    /// Start again on the address of an earlier fixture (same wrapping key),
    /// after `stop`.
    pub fn restart(previous: &KmsFixture) -> Self {
        let mut attempts = 0;
        loop {
            match TcpListener::bind(previous.addr) {
                Ok(listener) => return Self::serve(listener, Some(Arc::clone(&previous.state))),
                Err(error) => {
                    attempts += 1;
                    assert!(
                        attempts < 1000,
                        "cannot rebind the fixture address: {error}"
                    );
                    std::thread::yield_now();
                }
            }
        }
    }

    fn bind(addr: SocketAddr, state: Option<Arc<State>>) -> Self {
        Self::serve(TcpListener::bind(addr).expect("bind fixture"), state)
    }

    fn serve(listener: TcpListener, state: Option<Arc<State>>) -> Self {
        let addr = listener.local_addr().unwrap();
        let state = state.unwrap_or_else(|| {
            Arc::new(State {
                mode: Mutex::new(Mode::Normal),
                requests: Mutex::new(Vec::new()),
                held: Mutex::new(Vec::new()),
                local: LocalMasterKeyKms::from_master_key(&[0x5a; 32]).unwrap(),
            })
        });
        *state.mode.lock().unwrap() = Mode::Normal;
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let state = Arc::clone(&state);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(stream) = stream {
                        handle(&state, stream);
                    }
                }
            })
        };
        Self {
            addr,
            state,
            stop,
            thread: Some(thread),
        }
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn set_mode(&self, mode: Mode) {
        *self.state.mode.lock().unwrap() = mode;
    }

    /// Close the listener: later connections are refused.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.state.held.lock().unwrap().clear();
    }

    pub fn requests(&self) -> Vec<Req> {
        self.state.requests.lock().unwrap().clone()
    }

    pub fn count(&self, method: &str, path: &str) -> usize {
        self.requests()
            .iter()
            .filter(|r| r.method == method && r.path == path)
            .count()
    }
}

impl Drop for KmsFixture {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.stop();
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<Req> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok()?;
    let mut data = Vec::new();
    let mut buf = [0u8; 2048];
    let (head_end, content_length) = loop {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
        if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&data[..pos]).to_string();
            let length = head
                .lines()
                .filter_map(|l| l.split_once(':'))
                .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, v)| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            break (pos + 4, length);
        }
    };
    while data.len() < head_end + content_length {
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        data.extend_from_slice(&buf[..n]);
    }
    let head = String::from_utf8_lossy(&data[..head_end]).to_string();
    let mut lines = head.lines();
    let mut first = lines.next()?.split_whitespace();
    let method = first.next()?.to_owned();
    let path = first.next()?.to_owned();
    let authorization = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
        .map(|(_, v)| v.trim().to_owned());
    let body = String::from_utf8_lossy(&data[head_end..head_end + content_length]).to_string();
    Some(Req {
        method,
        path,
        authorization,
        body,
    })
}

fn respond(stream: &mut TcpStream, status: u16, extra: &str, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

fn handle(state: &State, mut stream: TcpStream) {
    let Some(request) = read_request(&mut stream) else {
        return;
    };
    state.requests.lock().unwrap().push(request.clone());
    let mode = state.mode.lock().unwrap().clone();
    let version = match mode {
        Mode::WrongVersion => "maos.kms.v0",
        _ => "maos.kms.v1",
    };
    match mode {
        Mode::Hang => {
            state.held.lock().unwrap().push(stream);
            return;
        }
        Mode::Redirect => {
            respond(&mut stream, 302, "Location: /elsewhere\r\n", "");
            return;
        }
        Mode::Fixed(status, body) => {
            respond(&mut stream, status, "", &body);
            return;
        }
        Mode::Normal | Mode::WrongVersion => {}
    }
    let json: serde_json::Value = serde_json::from_str(&request.body).unwrap_or_default();
    let field = |name: &str| {
        json.get(name)
            .and_then(serde_json::Value::as_str)
            .and_then(b64_decode)
    };
    let answer = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => Some(serde_json::json!({"version": version, "status": "ok"})),
        ("POST", "/wrap") => field("plaintext")
            .and_then(|key| state.local.wrap_data_key(&key).ok())
            .map(
                |wrapped| serde_json::json!({"version": version, "wrapped": b64_encode(&wrapped)}),
            ),
        ("POST", "/unwrap") => field("wrapped")
            .and_then(|wrapped| state.local.unwrap_data_key(&wrapped).ok())
            .map(|key| serde_json::json!({"version": version, "plaintext": b64_encode(&key)})),
        _ => None,
    };
    match answer {
        Some(value) => respond(&mut stream, 200, "", &value.to_string()),
        None if request.path == "/unwrap" => {
            respond(&mut stream, 422, "", r#"{"error":"unwrap_failed"}"#);
        }
        None if request.path == "/wrap" => {
            respond(&mut stream, 422, "", r#"{"error":"wrap_failed"}"#);
        }
        None => respond(&mut stream, 404, "", "{}"),
    }
}
