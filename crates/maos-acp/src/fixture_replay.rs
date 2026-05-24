//! Fixture-replay ACP client — test-only editor simulator.
//!
//! Gated by `#[cfg(any(test, feature = "fixture_replay"))]`.  Mirrors the
//! `FixtureReplayMcpServer` pattern from `maos-mcp`.

use std::collections::VecDeque;
use std::sync::Mutex;

use crate::frame::{AcpFrameIn, AcpFrameOut};

/// Replay-based ACP client — simulates an editor's NDJSON conversation.
#[cfg(any(test, feature = "fixture_replay"))]
pub struct FixtureReplayAcpClient {
    script: Mutex<VecDeque<AcpFrameIn>>,
    received: Mutex<Vec<AcpFrameOut>>,
}

#[cfg(any(test, feature = "fixture_replay"))]
impl FixtureReplayAcpClient {
    pub fn new(script: Vec<AcpFrameIn>) -> Self {
        Self {
            script: Mutex::new(script.into()),
            received: Mutex::new(Vec::new()),
        }
    }

    /// Pop the next inbound frame from the script.
    pub fn next_inbound(&self) -> Option<AcpFrameIn> {
        self.script.lock().unwrap().pop_front()
    }

    /// Record an outbound frame from the server.
    pub fn record_outbound(&self, frame: AcpFrameOut) {
        self.received.lock().unwrap().push(frame);
    }

    /// Drain the recorded frames for assertion.
    pub fn take_received(&self) -> Vec<AcpFrameOut> {
        std::mem::take(&mut *self.received.lock().unwrap())
    }
}
