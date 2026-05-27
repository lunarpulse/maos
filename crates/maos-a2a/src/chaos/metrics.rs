//! Per-agent metrics collection for the rotation chaos harness.
//!
//! At v0.5 the metrics are in-process — the harness's agent tasks write to a
//! `tokio::sync::Mutex<Vec<AgentRotationTimestamps>>`. Production wiring
//! would publish to the existing telemetry surface (`iac_handshake_duration_us`
//! histogram per architecture §4.7.1).

use crate::chaos::rotation::AgentRotationTimestamps;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    inner: Arc<Mutex<Vec<AgentRotationTimestamps>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record(&self, ts: AgentRotationTimestamps) {
        let mut g = self.inner.lock().await;
        g.push(ts);
    }

    pub async fn snapshot(&self) -> Vec<AgentRotationTimestamps> {
        let g = self.inner.lock().await;
        g.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn collector_records_and_snapshots() {
        let c = MetricsCollector::new();
        c.record(AgentRotationTimestamps {
            agent_id: "a".into(),
            t_0_ns: 0,
            t_1_ns: Some(1),
            t_2_ns: Some(2),
        })
        .await;
        let snap = c.snapshot().await;
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].agent_id, "a");
    }
}
