//! ACP notification channel — kernel-facing dispatch surface.
//!
//! Holds the session registry from `AcpServer`; provides a `dispatch`
//! method that fans out notification frames to all connected ACP editor
//! sessions.  The `NotificationChannel` trait impl lives in
//! `maos-director-surface` (avoiding a circular dep).

use std::sync::{Arc, Mutex};

use crate::frame::AcpFrameOut;

/// Per-session outbound handle held by the server's session registry.
#[derive(Debug, Clone)]
pub struct AcpOutboundHandle {
    pub session_id: [u8; 16],
    pub outbound: crossbeam_channel::Sender<AcpFrameOut>,
    pub started_at_ns: u64,
}

/// Kernel-facing ACP notification channel.
///
/// The `NotificationChannel` trait impl is in `maos-director-surface`'s
/// `AcpEditorChannel` wrapper (avoids `maos-acp` → `maos-director-surface`
/// circular dep).
pub struct AcpEditorChannelImpl {
    sessions: Arc<Mutex<Vec<AcpOutboundHandle>>>,
}

impl AcpEditorChannelImpl {
    pub fn new(sessions: Arc<Mutex<Vec<AcpOutboundHandle>>>) -> Self {
        Self { sessions }
    }

    /// Fan-out a notification event to all connected editor sessions.
    ///
    /// Returns `Ok(n)` where `n` is the number of sessions that received
    /// the frame, or an error if all sessions are full/disconnected.
    pub fn dispatch_event(
        &self,
        event_json: serde_json::Value,
        level: &str,
    ) -> Result<usize, String> {
        let frame = AcpFrameOut::NotificationDispatch {
            level: level.into(),
            event: event_json,
        };

        let sessions = self.sessions.lock().map_err(|e| format!("lock poisoned: {e}"))?;

        if sessions.is_empty() {
            return Ok(0);
        }

        let mut delivered = 0usize;
        for session in sessions.iter() {
            match session.outbound.try_send(frame.clone()) {
                Ok(()) => delivered += 1,
                Err(crossbeam_channel::TrySendError::Full(_)) => {
                    // TODO: integrate cap_audit::record_drop when cross-crate audit bridge is available
                    // For now, the caller (AcpEditorChannel in maos-director-surface) should check
                    // the return count and log accordingly.
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    // session dead — GC'd on next session.end
                }
            }
        }

        if delivered == 0 {
            return Err("all sessions full or disconnected".into());
        }
        Ok(delivered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_channel(cap: usize) -> (
        AcpEditorChannelImpl,
        crossbeam_channel::Receiver<AcpFrameOut>,
        Arc<Mutex<Vec<AcpOutboundHandle>>>,
    ) {
        let (tx, rx) = crossbeam_channel::bounded::<AcpFrameOut>(cap);
        let handle = AcpOutboundHandle {
            session_id: [1u8; 16],
            outbound: tx,
            started_at_ns: 0,
        };
        let sessions = Arc::new(Mutex::new(vec![handle]));
        let channel = AcpEditorChannelImpl::new(Arc::clone(&sessions));
        (channel, rx, sessions)
    }

    #[test]
    fn dispatch_with_one_session_delivers_frame() {
        let (channel, rx, _sessions) = test_channel(4);
        let event = serde_json::json!({"TaskAssigned": {"from": "director", "goal": "test"}});
        let n = channel.dispatch_event(event, "immediate").unwrap();
        assert_eq!(n, 1);

        let received = rx.try_recv().unwrap();
        match received {
            AcpFrameOut::NotificationDispatch { level, .. } => {
                assert_eq!(level, "immediate");
            }
            other => panic!("expected NotificationDispatch, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_with_full_session_returns_error() {
        let (tx, _rx) = crossbeam_channel::bounded::<AcpFrameOut>(1);
        tx.try_send(AcpFrameOut::Error {
            code: 0,
            message: "prefill".into(),
            decision_id: None,
        })
        .unwrap();

        let handle = AcpOutboundHandle {
            session_id: [1u8; 16],
            outbound: tx,
            started_at_ns: 0,
        };
        let sessions = Arc::new(Mutex::new(vec![handle]));
        let channel = AcpEditorChannelImpl::new(sessions);
        let event = serde_json::json!({"TaskAssigned": {"from": "director", "goal": "test"}});
        let result = channel.dispatch_event(event, "immediate");
        assert!(result.is_err());
    }

    #[test]
    fn dispatch_with_zero_sessions_returns_ok_zero() {
        let sessions = Arc::new(Mutex::new(vec![]));
        let channel = AcpEditorChannelImpl::new(sessions);
        let event = serde_json::json!({"TaskAssigned": {"from": "d", "goal": "t"}});
        let n = channel.dispatch_event(event, "immediate").unwrap();
        assert_eq!(n, 0);
    }
}
