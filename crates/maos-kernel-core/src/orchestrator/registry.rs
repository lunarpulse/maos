#![forbid(unsafe_code)]

use std::sync::Arc;
use dashmap::DashMap;
use maos_spirit_abi::identity::SpiritId;
use super::buffer::OrchestratorBuffer;

/// Per-Host registry of Orchestrator-class buffer instances. One
/// `OrchestratorBuffer` per Orchestrator Spirit; lookup by `SpiritId`.
///
/// Held in `Arc` so the CLI one-shot arm + the (future Story 5.1)
/// supervised Orchestrator process see the same buffer. At v0.3-β the
/// one-shot arms instantiate a fresh registry per invocation — Story 5.1
/// will share it with the long-running supervisor.
#[maos_attrs::i9_exempt(reason = "orchestrator registry — DashMap of per-Spirit transient buffers; parallel to Mailbox::mpsc_senders")]
#[derive(Debug, Default)]
pub struct OrchestratorBufferRegistry {
    buffers: DashMap<String, Arc<OrchestratorBuffer>>,
}

impl OrchestratorBufferRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get-or-create the buffer for `spirit_id`. Same shape as
    /// `Mailbox::register_spirit` but idempotent (returns existing buffer
    /// instead of error on duplicate) because director-side enqueues
    /// arrive before any "registration" call.
    pub fn get_or_create(&self, spirit_id: &SpiritId) -> Arc<OrchestratorBuffer> {
        self.buffers
            .entry(spirit_id.0.clone())
            .or_insert_with(|| Arc::new(OrchestratorBuffer::new()))
            .value()
            .clone()
    }

    /// Look up an existing buffer without creating one. Returns `None`
    /// if the Spirit has never been queued to.
    pub fn get(&self, spirit_id: &SpiritId) -> Option<Arc<OrchestratorBuffer>> {
        self.buffers.get(&spirit_id.0).map(|r| r.value().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::identity::SpiritId;

    fn sid(s: &str) -> SpiritId {
        SpiritId::from(s)
    }

    #[test]
    fn get_or_create_returns_new_buffer_for_fresh_spirit() {
        let registry = OrchestratorBufferRegistry::new();
        let buf = registry.get_or_create(&sid("hello-spirit"));
        assert_eq!(buf.pending_count(), 0);
        assert_eq!(buf.capacity(), 32);
    }

    #[test]
    fn get_or_create_is_idempotent() {
        let registry = OrchestratorBufferRegistry::new();
        let buf1 = registry.get_or_create(&sid("hello-spirit"));
        let buf2 = registry.get_or_create(&sid("hello-spirit"));
        assert!(Arc::ptr_eq(&buf1, &buf2));
    }

    #[test]
    fn get_returns_none_for_unknown_spirit() {
        let registry = OrchestratorBufferRegistry::new();
        assert!(registry.get(&sid("unknown-spirit")).is_none());
    }

    #[test]
    fn get_returns_some_for_known_spirit() {
        let registry = OrchestratorBufferRegistry::new();
        registry.get_or_create(&sid("hello-spirit"));
        assert!(registry.get(&sid("hello-spirit")).is_some());
    }

    #[test]
    fn registry_is_send_and_sync() {
        fn _assert_send_sync<T: Send + Sync>(_: T) {}
        _assert_send_sync(OrchestratorBufferRegistry::new());
    }
}
