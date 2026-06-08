#![forbid(unsafe_code)]

//! Notification surface dispatcher per architecture §7.4.
//!
//! "These [notification levels] are kernel-rendered, not Spirit-rendered.
//! A Spirit cannot bypass the user's notification policy by emitting a
//! different kind of event; the kernel intercepts every IAC frame whose
//! recipient is the human and routes it through the configured
//! notification surface."
//!
//! Domain types (`NotificationEvent`, `ApprovalClass`, `NotificationLevel`,
//! `NotificationSurface`) are defined in `maos-domain::notification` and
//! re-exported here.

use std::io::Write;
use std::sync::{Arc, Mutex};

pub use maos_domain::notification::{
    ApprovalClass, NotificationEvent, NotificationLevel, NotificationSurface,
};

/// Pluggable channel adapter — terminal / ACP editor / mobile push.
pub trait NotificationChannel: Send + Sync + 'static {
    fn surface(&self) -> NotificationSurface;
    fn dispatch(
        &self,
        event: &NotificationEvent,
        level: NotificationLevel,
    ) -> Result<(), NotificationError>;
}

/// Error raised by a notification channel.
#[derive(Debug, thiserror::Error)]
pub enum NotificationError {
    #[error("channel unavailable: {0}")]
    Unavailable(String),
    #[error("channel write failed: {0}")]
    WriteFailed(String),
}

/// Report returned by `NotificationDispatcher::dispatch`.
#[derive(Debug, Clone)]
pub struct DispatchReport {
    pub delivered: usize,
    pub errors: usize,
}

/// Fan-out dispatcher that sends events to all registered channels.
pub struct NotificationDispatcher {
    channels: Vec<Box<dyn NotificationChannel>>,
}

impl NotificationDispatcher {
    pub fn new() -> Self {
        Self { channels: vec![] }
    }

    pub fn register(&mut self, ch: Box<dyn NotificationChannel>) {
        self.channels.push(ch);
    }

    /// Dispatch an event to ALL registered channels.
    pub fn dispatch(
        &self,
        event: NotificationEvent,
        level: NotificationLevel,
    ) -> Result<DispatchReport, NotificationError> {
        let mut report = DispatchReport {
            delivered: 0,
            errors: 0,
        };

        for ch in &self.channels {
            match ch.dispatch(&event, level) {
                Ok(()) => report.delivered += 1,
                Err(_) => report.errors += 1,
            }
        }

        Ok(report)
    }
}

impl Default for NotificationDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Concrete channels ───────────────────────────────────────────────

/// Terminal channel — writes to an injected `Arc<Mutex<dyn Write + Send>>`.
///
/// Production wiring passes a stderr adapter; tests pass a `Vec<u8>` writer.
/// Mirrors the NO_COLOR / `--plain` accessibility cascade pattern from
/// `maos-cli::accessibility`.
pub struct TerminalChannel {
    writer: Arc<Mutex<dyn Write + Send>>,
    use_color: bool,
}

impl TerminalChannel {
    pub fn new(writer: Arc<Mutex<dyn Write + Send>>) -> Self {
        let use_color = std::env::var_os("NO_COLOR").is_none()
            && std::env::var_os("TERM")
                .map(|t| t != "dumb")
                .unwrap_or(false);
        Self { writer, use_color }
    }

    pub fn with_color(mut self, use_color: bool) -> Self {
        self.use_color = use_color;
        self
    }
}

impl NotificationChannel for TerminalChannel {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::Terminal
    }

    fn dispatch(
        &self,
        event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        let mut w = self
            .writer
            .lock()
            .map_err(|e| NotificationError::Unavailable(format!("lock poisoned: {e}")))?;

        match event {
            NotificationEvent::TaskAssigned {
                frame_id,
                from,
                goal,
            } => {
                let id_hex: String = frame_id
                    .iter()
                    .take(8)
                    .map(|b| format!("{b:02x}"))
                    .collect();
                if self.use_color {
                    let _ = writeln!(
                        w,
                        "\x1b[1;32m[maos]\x1b[0m Task assigned \x1b[1;36m{id_hex}\x1b[0m from {from}: {goal}"
                    );
                } else {
                    let _ = writeln!(w, "[maos] Task assigned {id_hex} from {from}: {goal}");
                }
            }
            NotificationEvent::ApprovalPrompt {
                decision_id,
                class,
                capability,
                reasoning,
            } => {
                if self.use_color {
                    let _ = writeln!(
                        w,
                        "\x1b[1;33m[maos]\x1b[0m Approval #{} required: {:?} / {} ({})",
                        decision_id,
                        class,
                        capability,
                        reasoning.as_deref().unwrap_or("no reasoning")
                    );
                } else {
                    let _ = writeln!(
                        w,
                        "[maos] Approval #{} required: {:?} / {} ({})",
                        decision_id,
                        class,
                        capability,
                        reasoning.as_deref().unwrap_or("no reasoning")
                    );
                }
            }
            NotificationEvent::Halt { payload } => {
                let id_prefix: String = payload.halt_id.chars().take(8).collect();
                if self.use_color {
                    let _ =
                        writeln!(
                        w,
                        "\x1b[1;31m[maos]\x1b[0m Halt {} tag={} value={} threshold={} policy={}",
                        id_prefix,
                        payload.tag,
                        payload.value,
                        payload.threshold.map(|t| t.to_string()).unwrap_or_else(|| "none".into()),
                        payload.policy_id,
                    );
                } else {
                    let _ = writeln!(
                        w,
                        "[maos] Halt {} tag={} value={} threshold={} policy={}",
                        id_prefix,
                        payload.tag,
                        payload.value,
                        payload
                            .threshold
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| "none".into()),
                        payload.policy_id,
                    );
                }
            }
            NotificationEvent::AnomalyFlagged {
                observer,
                subject,
                summary,
                confidence,
            } => {
                let pct = (confidence * 100.0) as u32;
                let short_summary: String = summary.chars().take(80).collect();
                if self.use_color {
                    let _ = writeln!(
                        w,
                        "\x1b[1;35m[maos]\x1b[0m Anomaly \x1b[1m{observer}\x1b[0m → {subject}: {pct}% — {short_summary}",
                    );
                } else {
                    let _ = writeln!(
                        w,
                        "[maos] Anomaly {observer} → {subject}: {pct}% — {short_summary}",
                    );
                }
            }
            _ => {
                let _ = writeln!(w, "[maos] Unknown notification event (future story)");
            }
        }

        Ok(())
    }
}

/// Story 5.5c — ACP editor channel (seam closed: wraps `maos-acp` impl).
pub struct AcpEditorChannel {
    inner: maos_acp::AcpEditorChannelImpl,
}

impl AcpEditorChannel {
    pub fn new(inner: maos_acp::AcpEditorChannelImpl) -> Self {
        Self { inner }
    }
}

impl NotificationChannel for AcpEditorChannel {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::AcpEditor
    }

    fn dispatch(
        &self,
        event: &NotificationEvent,
        level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        let level_str = match level {
            NotificationLevel::Immediate => "immediate",
            NotificationLevel::Queue => "queue",
            NotificationLevel::Digest => "digest",
        };
        let event_json = serde_json::to_value(event)
            .map_err(|e| NotificationError::WriteFailed(format!("serialize event: {e}")))?;
        self.inner
            .dispatch_event(event_json, level_str)
            .map_err(|e| NotificationError::WriteFailed(e))?;
        Ok(())
    }
}

/// Placeholder mobile-push channel.
///
/// Production mobile push lives in `maos-notify-push::MobilePushHttp`, which is
/// wired by the host from operator config. This placeholder returns a typed
/// error instead of panicking so no live dispatch path contains `unimplemented!()`.
pub struct MobilePushChannel;

impl NotificationChannel for MobilePushChannel {
    fn surface(&self) -> NotificationSurface {
        NotificationSurface::MobilePush
    }

    fn dispatch(
        &self,
        _event: &NotificationEvent,
        _level: NotificationLevel,
    ) -> Result<(), NotificationError> {
        Err(NotificationError::Unavailable(
            "mobile push channel requires maos-notify-push host wiring".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capture_writer() -> Arc<Mutex<Vec<u8>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn captured_output(w: &Arc<Mutex<Vec<u8>>>) -> String {
        String::from_utf8(w.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn terminal_channel_dispatches_task_assigned() {
        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);
        let event = NotificationEvent::TaskAssigned {
            frame_id: [1u8; 16],
            from: "director".into(),
            goal: "review the PR".into(),
        };
        ch.dispatch(&event, NotificationLevel::Immediate).unwrap();
        let out = captured_output(&w);
        assert!(out.contains("[maos] Task assigned"));
        assert!(out.contains("director"));
        assert!(out.contains("review the PR"));
    }

    #[test]
    fn dispatcher_with_zero_channels_returns_ok() {
        let dispatcher = NotificationDispatcher::new();
        let event = NotificationEvent::TaskAssigned {
            frame_id: [0u8; 16],
            from: "a".into(),
            goal: "b".into(),
        };
        let report = dispatcher
            .dispatch(event, NotificationLevel::Queue)
            .unwrap();
        assert_eq!(report.delivered, 0);
        assert_eq!(report.errors, 0);
    }

    #[test]
    fn dispatcher_with_one_channel_delivers() {
        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);
        let mut dispatcher = NotificationDispatcher::new();
        dispatcher.register(Box::new(ch));

        let event = NotificationEvent::TaskAssigned {
            frame_id: [2u8; 16],
            from: "d".into(),
            goal: "test".into(),
        };
        let report = dispatcher
            .dispatch(event, NotificationLevel::Immediate)
            .unwrap();
        assert_eq!(report.delivered, 1);
        assert_eq!(report.errors, 0);

        let out = captured_output(&w);
        assert!(out.contains("test"));
    }

    #[test]
    fn error_isolation_per_channel() {
        struct FailingChannel;
        impl NotificationChannel for FailingChannel {
            fn surface(&self) -> NotificationSurface {
                NotificationSurface::Terminal
            }
            fn dispatch(
                &self,
                _event: &NotificationEvent,
                _level: NotificationLevel,
            ) -> Result<(), NotificationError> {
                Err(NotificationError::Unavailable("fail".into()))
            }
        }

        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);

        let mut dispatcher = NotificationDispatcher::new();
        dispatcher.register(Box::new(FailingChannel));
        dispatcher.register(Box::new(ch));

        let event = NotificationEvent::TaskAssigned {
            frame_id: [0u8; 16],
            from: "x".into(),
            goal: "err-isolation".into(),
        };
        let report = dispatcher
            .dispatch(event, NotificationLevel::Immediate)
            .unwrap();
        assert_eq!(report.delivered, 1);
        assert_eq!(report.errors, 1);

        let out = captured_output(&w);
        assert!(out.contains("err-isolation"));
    }

    #[test]
    fn terminal_channel_renders_halt_event() {
        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);
        let payload = maos_domain::frame::EpistemicHaltPayload::new(
            "halt-abc123".into(),
            "claim.security".into(),
            0.42,
            Some(0.75),
            "pol-1".into(),
            "derived_from_x".into(),
        )
        .unwrap();
        let event = NotificationEvent::Halt { payload };
        ch.dispatch(&event, NotificationLevel::Immediate).unwrap();
        let out = captured_output(&w);
        assert!(out.contains("halt-abc"));
        assert!(out.contains("tag=claim.security"));
        assert!(out.contains("value=0.42"));
        assert!(out.contains("threshold=0.75"));
        assert!(out.contains("policy=pol-1"));
    }

    #[test]
    fn terminal_channel_halt_event_emits_zero_ansi_under_no_color() {
        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);
        let payload = maos_domain::frame::EpistemicHaltPayload::new(
            "halt-001".into(),
            "t".into(),
            0.0,
            None,
            "p".into(),
            "d".into(),
        )
        .unwrap();
        let event = NotificationEvent::Halt { payload };
        ch.dispatch(&event, NotificationLevel::Immediate).unwrap();
        let out = captured_output(&w);
        assert!(!out.contains('\x1b'));
    }

    /// Architecture §4.3.3 specifies exactly six approval classes in
    /// this order: readonly_scoped, readonly_search, mutating,
    /// exec_capable, control_plane, interactive.
    #[test]
    fn approval_classes_match_architecture() {
        let classes = [
            ApprovalClass::ReadonlyScoped,
            ApprovalClass::ReadonlySearch,
            ApprovalClass::Mutating,
            ApprovalClass::ExecCapable,
            ApprovalClass::ControlPlane,
            ApprovalClass::Interactive,
        ];
        // Verify all six exist and are distinct
        assert_eq!(classes.len(), 6);
        for i in 0..classes.len() {
            for j in (i + 1)..classes.len() {
                assert_ne!(classes[i], classes[j]);
            }
        }
    }

    #[test]
    fn terminal_channel_renders_anomaly_event() {
        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);
        let event = NotificationEvent::anomaly_flagged(
            "observer-1",
            "subject-1",
            "resource exhaustion detected on worker pool",
            0.85,
        )
        .unwrap();
        ch.dispatch(&event, NotificationLevel::Immediate).unwrap();
        let out = captured_output(&w);
        assert!(out.contains("observer-1"), "output: {out}");
        assert!(out.contains("subject-1"), "output: {out}");
        assert!(
            out.contains("85%"),
            "expected 85% confidence, output: {out}"
        );
        assert!(
            out.contains("resource exhaustion detected on worker pool"),
            "output: {out}"
        );
    }

    #[test]
    fn terminal_channel_anomaly_event_emits_zero_ansi_under_no_color() {
        let w = capture_writer();
        let ch = TerminalChannel::new(w.clone()).with_color(false);
        let event = NotificationEvent::anomaly_flagged("obs", "sub", "test anomaly", 0.5).unwrap();
        ch.dispatch(&event, NotificationLevel::Immediate).unwrap();
        let out = captured_output(&w);
        assert!(
            !out.contains('\x1b'),
            "NO_COLOR output contained ANSI escapes"
        );
    }
}
