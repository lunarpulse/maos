//! Fixture-replay MCP server — test-only helper for deterministic CI runs.
//!
//! Gated by `#[cfg(any(test, feature = "fixture_replay"))]`.  Mirrors the
//! `FixtureReplayProvider` precedent from `crates/maos-providers/src/fixture_replay.rs`.

use std::collections::VecDeque;
use std::sync::Mutex;

use maos_domain::ports::mcp::{McpRequest, McpResponse, McpTransportId};

use crate::transport::{McpTransport, McpTransportError};

/// Replay-based MCP transport — responds from a pre-loaded queue.
#[cfg(any(test, feature = "fixture_replay"))]
pub struct FixtureReplayMcpServer {
    responses: Mutex<VecDeque<Result<McpResponse, McpTransportError>>>,
    calls: Mutex<Vec<McpRequest>>,
    transport_id: McpTransportId,
}

#[cfg(any(test, feature = "fixture_replay"))]
impl FixtureReplayMcpServer {
    pub fn new(
        responses: Vec<Result<McpResponse, McpTransportError>>,
        transport_id: McpTransportId,
    ) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            calls: Mutex::new(Vec::new()),
            transport_id,
        }
    }

    /// Drain the recorded calls for assertion.
    pub fn take_calls(&self) -> Vec<McpRequest> {
        let mut guard = self.calls.lock().unwrap();
        std::mem::take(&mut *guard)
    }
}

#[cfg(any(test, feature = "fixture_replay"))]
impl McpTransport for FixtureReplayMcpServer {
    fn invoke(&self, request: McpRequest) -> Result<McpResponse, McpTransportError> {
        self.calls.lock().unwrap().push(request.clone());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| McpTransportError::Transport("fixture replay: response ring empty".into()))?
    }

    fn id(&self) -> McpTransportId {
        self.transport_id
    }
}
