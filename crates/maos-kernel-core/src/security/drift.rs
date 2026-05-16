#![forbid(unsafe_code)]

//! Drift event channel — asynchronous notification of capability scope
//! mismatches between declared and observed invocations.
//!
//! Story 2.1 ships the channel surface and registration call site. The
//! runtime detector that actually emits events into this channel ships
//! at Story 9.x.

use tokio::sync::mpsc;

use maos_domain::invariants::i1::Scope;

/// Bounded channel capacity for drift events.
pub const DRIFT_CHANNEL_CAP: usize = 256;

/// Drift event — fired when a Spirit's observed capability invocation
/// diverges from its declared capability scopes.
#[derive(Debug, Clone)]
pub enum DriftEvent {
    /// A Spirit invoked a capability not in its declared set.
    CapabilityScopeDrift {
        /// Spirit process ID.
        spirit_pid: u32,
        /// Scopes declared in the Spirit's manifest.
        declared: Vec<Scope>,
        /// The scope actually invoked.
        observed: Scope,
    },
}

/// Create a bounded mpsc channel for drift events.
///
/// The `Sender` is held by the `SecurityManagerAdapter`; the `Receiver`
/// is consumed by the runtime drift detector (Story 9.x).
pub fn make_drift_channel() -> (mpsc::Sender<DriftEvent>, mpsc::Receiver<DriftEvent>) {
    mpsc::channel(DRIFT_CHANNEL_CAP)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i1::Scope;

    #[test]
    fn drift_channel_sends_and_receives() {
        let (tx, mut rx) = make_drift_channel();
        let event = DriftEvent::CapabilityScopeDrift {
            spirit_pid: 42,
            declared: vec![Scope::ProviderInfer {
                provider: "anthropic".into(),
            }],
            observed: Scope::ProviderInfer {
                provider: "openai".into(),
            },
        };
        tx.try_send(event.clone()).expect("channel should not be full");
        let received = rx.try_recv().expect("should receive event");
        assert!(matches!(received, DriftEvent::CapabilityScopeDrift { spirit_pid, .. } if spirit_pid == 42));
    }

    #[test]
    fn drift_channel_try_send_does_not_block_on_full() {
        let (tx, _rx) = make_drift_channel();
        for i in 0..DRIFT_CHANNEL_CAP * 2 {
            let event = DriftEvent::CapabilityScopeDrift {
                spirit_pid: i as u32,
                declared: vec![],
                observed: Scope::ProviderInfer {
                    provider: "test".into(),
                },
            };
            // After the first CAP events, try_send returns Err but never panics.
            let _ = tx.try_send(event);
        }
    }
}
