//! Story 18-8 review EC-2/BH-6 — the era cache is never held across an
//! exchange: two calls on one `StreamableHttpTransport` overlap on the wire.
//!
//! Deterministic: the fake port parks each exchange until a second one is in
//! flight (bounded at 5 s). A transport that serialises its calls never gets a
//! second exchange in, and the first one gives up alone.

use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use maos_domain::ports::io_subsystem::{IoError, IoSubsystemPort};
use maos_domain::ports::mcp::McpRequest;
use maos_mcp::transport::streamable_http::StreamableHttpTransport;
use maos_mcp::transport::McpTransport;

#[derive(Default)]
struct Rendezvous {
    state: Mutex<(usize, usize)>, // (in flight, most ever in flight)
    changed: Condvar,
}

impl IoSubsystemPort for Rendezvous {
    fn http_get(&self, _url: &str) -> Result<Vec<u8>, IoError> {
        unreachable!("the transport only POSTs")
    }

    fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        let mut state = self.state.lock().unwrap();
        state.0 += 1;
        state.1 = state.1.max(state.0);
        self.changed.notify_all();
        let (mut state, _) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |state| state.1 < 2)
            .unwrap();
        state.0 -= 1;
        Ok(br#"{"jsonrpc":"2.0","id":1,"result":{"name":"t","content":[]}}"#.to_vec())
    }
}

#[test]
fn two_calls_on_one_transport_overlap() {
    let port = Arc::new(Rendezvous::default());
    let transport = Arc::new(StreamableHttpTransport::new(
        Arc::clone(&port) as Arc<dyn IoSubsystemPort>,
        "http://127.0.0.1:9/mcp".into(),
        None,
    ));
    let calls: Vec<_> = (0..2)
        .map(|_| {
            let transport = Arc::clone(&transport);
            std::thread::spawn(move || {
                transport.invoke(McpRequest::new(
                    "s".into(),
                    "t".into(),
                    serde_json::json!({}),
                ))
            })
        })
        .collect();
    for call in calls {
        call.join().unwrap().expect("each call succeeds");
    }
    assert_eq!(
        port.state.lock().unwrap().1,
        2,
        "a stalled exchange must not block a second call on the same transport"
    );
}
