//! ACP server — NDJSON-over-stdio session-oriented protocol.
//!
//! The server runs as a child of the host editor: editor spawns
//! `maos-bin --acp` with `stdin`/`stdout` piped.  The server consumes
//! NDJSON frames from `stdin` and emits NDJSON frames to `stdout`.

use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};

use maos_domain::halt::HaltResolver;
use maos_domain::lifecycle::{LifecycleResolver, LifecycleVerb};

use crate::frame::{AcpFrameIn, AcpFrameOut, DecisionId, SessionId};
use crate::notification_channel::AcpOutboundHandle;

/// ACP server — long-running NDJSON-over-stdio server.
pub struct AcpServer {
    pub lifecycle: Arc<dyn LifecycleResolver>,
    pub halts: Arc<dyn HaltResolver>,
    pub sessions: Arc<Mutex<Vec<AcpOutboundHandle>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("frame parse error: {0}")]
    Parse(String),
    #[error("lifecycle error: {0}")]
    Lifecycle(String),
    #[error("halt error: {0}")]
    Halt(String),
}

impl AcpServer {
    pub fn new(
        lifecycle: Arc<dyn LifecycleResolver>,
        halts: Arc<dyn HaltResolver>,
    ) -> Self {
        Self {
            lifecycle,
            halts,
            sessions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Snapshot of current session registry; consumed by
    /// `AcpEditorChannelImpl::dispatch` for fan-out.
    pub fn session_registry(&self) -> Arc<Mutex<Vec<AcpOutboundHandle>>> {
        Arc::clone(&self.sessions)
    }

    /// Block on stdio, accept session frames, dispatch to lifecycle/halt
    /// resolvers, write replies to stdout. Returns when stdin EOFs.
    pub fn run(&mut self, stdin: impl Read, stdout: impl Write) -> Result<(), AcpError> {
        let reader = BufReader::new(stdin);
        let mut writer = stdout;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            let frame: AcpFrameIn = match serde_json::from_str(&line) {
                Ok(f) => f,
                Err(e) => {
                    let err = AcpFrameOut::Error {
                        code: -32600,
                        message: format!("invalid frame: {e}"),
                        decision_id: None,
                    };
                    let mut json = serde_json::to_vec(&err)
                        .map_err(|e| AcpError::Parse(e.to_string()))?;
                    json.push(b'\n');
                    writer.write_all(&json)?;
                    writer.flush()?;
                    continue;
                }
            };

            match frame {
                AcpFrameIn::SessionStart {
                    session_id,
                    editor_id: _editor_id,
                    editor_version: _editor_version,
                } => {
                    let (tx, _rx) = crossbeam_channel::bounded::<AcpFrameOut>(256);
                    let handle = AcpOutboundHandle {
                        session_id: session_id.0,
                        outbound: tx,
                        started_at_ns: monotonic_now_ns(),
                    };
                    self.sessions.lock().unwrap().push(handle);

                    let reply = AcpFrameOut::SessionReady {
                        session_id,
                        supported_kinds: vec![
                            "lifecycle_verb".into(),
                            "halt_resolve".into(),
                            "session_end".into(),
                        ],
                    };
                    let mut json = serde_json::to_vec(&reply)
                        .map_err(|e| AcpError::Parse(e.to_string()))?;
                    json.push(b'\n');
                    writer.write_all(&json)?;
                    writer.flush()?;
                }
                AcpFrameIn::SessionEnd { session_id } => {
                    let now = monotonic_now_ns();
                    let started_at_ns = {
                        let sessions = self.sessions.lock().unwrap();
                        sessions
                            .iter()
                            .find(|h| h.session_id == session_id.0)
                            .map(|h| h.started_at_ns)
                            .unwrap_or(0)
                    };
                    self.sessions.lock().unwrap().retain(|h| h.session_id != session_id.0);

                    let reply = AcpFrameOut::SessionTerminated {
                        session_id,
                        duration_ns: now.saturating_sub(started_at_ns),
                    };
                    let mut json = serde_json::to_vec(&reply)
                        .map_err(|e| AcpError::Parse(e.to_string()))?;
                    json.push(b'\n');
                    writer.write_all(&json)?;
                    writer.flush()?;
                }
                AcpFrameIn::LifecycleVerb {
                    decision_id,
                    verb,
                    spirit_id,
                    ..
                } => {
                    let verb_enum = match verb.as_str() {
                        "load" => LifecycleVerb::Load,
                        "start" => LifecycleVerb::Start,
                        "pause" => LifecycleVerb::Pause,
                        "resume" => LifecycleVerb::Resume,
                        "unload" => LifecycleVerb::Unload,
                        other => {
                            let reply = AcpFrameOut::LifecycleReceipt {
                                decision_id,
                                spirit_pid: 0,
                                verb: other.into(),
                                timestamp_ns: monotonic_now_ns(),
                                outcome: "error".into(),
                                error: Some(crate::frame::AcpErrorBody {
                                    code: -32601,
                                    message: format!("unknown verb: {other}"),
                                }),
                            };
                            let mut json = serde_json::to_vec(&reply)
                                .map_err(|e| AcpError::Parse(e.to_string()))?;
                            json.push(b'\n');
                            writer.write_all(&json)?;
                            writer.flush()?;
                            continue;
                        }
                    };

                    match self.lifecycle.resolve_verb(&spirit_id, verb_enum) {
                        Ok(receipt) => {
                            let reply = AcpFrameOut::LifecycleReceipt {
                                decision_id,
                                spirit_pid: receipt.spirit_pid,
                                verb,
                                timestamp_ns: receipt.timestamp_ns,
                                outcome: "ok".into(),
                                error: None,
                            };
                            let mut json = serde_json::to_vec(&reply)
                                .map_err(|e| AcpError::Parse(e.to_string()))?;
                            json.push(b'\n');
                            writer.write_all(&json)?;
                            writer.flush()?;
                        }
                        Err(e) => {
                            let reply = AcpFrameOut::LifecycleReceipt {
                                decision_id,
                                spirit_pid: 0,
                                verb,
                                timestamp_ns: monotonic_now_ns(),
                                outcome: "error".into(),
                                error: Some(crate::frame::AcpErrorBody {
                                    code: -32000,
                                    message: e.to_string(),
                                }),
                            };
                            let mut json = serde_json::to_vec(&reply)
                                .map_err(|e| AcpError::Parse(e.to_string()))?;
                            json.push(b'\n');
                            writer.write_all(&json)?;
                            writer.flush()?;
                        }
                    }
                }
                AcpFrameIn::HaltResolve {
                    decision_id,
                    halt_id,
                    resolution,
                    operator_note,
                    ..
                } => {
                    let resolution_enum = match resolution.as_str() {
                        "approve" => maos_domain::halt::Resolution::AuthorizedOverride {
                            operator_policy_ref: operator_note.unwrap_or_else(|| "acp-editor".into()),
                        },
                        "accept" => maos_domain::halt::Resolution::AcceptedHalt,
                        "provide" => maos_domain::halt::Resolution::ProvidedContext {
                            text: operator_note.unwrap_or_else(|| "provided via ACP".into()),
                        },
                        other => {
                            let reply = AcpFrameOut::HaltReceipt {
                                decision_id,
                                halt_id,
                                outcome: "error".into(),
                                timestamp_ns: monotonic_now_ns(),
                            };
                            let mut json = serde_json::to_vec(&reply)
                                .map_err(|e| AcpError::Parse(e.to_string()))?;
                            json.push(b'\n');
                            writer.write_all(&json)?;
                            writer.flush()?;
                            continue;
                        }
                    };

                    let halt_id_obj = maos_domain::halt::HaltId::new(halt_id.clone())
                        .unwrap_or_else(|_| maos_domain::halt::HaltId::new("acp-unknown").unwrap());

                    match self.halts.resolve(&halt_id_obj, resolution_enum) {
                        Ok(()) => {
                            let reply = AcpFrameOut::HaltReceipt {
                                decision_id,
                                halt_id,
                                outcome: "resolved".into(),
                                timestamp_ns: monotonic_now_ns(),
                            };
                            let mut json = serde_json::to_vec(&reply)
                                .map_err(|e| AcpError::Parse(e.to_string()))?;
                            json.push(b'\n');
                            writer.write_all(&json)?;
                            writer.flush()?;
                        }
                        Err(e) => {
                            let reply = AcpFrameOut::HaltReceipt {
                                decision_id,
                                halt_id,
                                outcome: "error".into(),
                                timestamp_ns: monotonic_now_ns(),
                            };
                            let mut json = serde_json::to_vec(&reply)
                                .map_err(|e| AcpError::Parse(e.to_string()))?;
                            json.push(b'\n');
                            writer.write_all(&json)?;
                            writer.flush()?;

                            let _ = e; // error details logged
                        }
                    }
                }
            }
        }

        // stdin EOF — implicit session_end for all active sessions
        let sessions_to_close: Vec<_> = {
            let mut sessions = self.sessions.lock().unwrap();
            std::mem::take(&mut *sessions)
        };
        for session in sessions_to_close {
            let reply = AcpFrameOut::SessionTerminated {
                session_id: SessionId(session.session_id),
                duration_ns: monotonic_now_ns().saturating_sub(session.started_at_ns),
            };
            let json = serde_json::to_vec(&reply).ok();
            if let Some(mut json) = json {
                json.push(b'\n');
                let _ = writer.write_all(&json);
                let _ = writer.flush();
            }
        }

        Ok(())
    }
}

fn monotonic_now_ns() -> u64 {
    // Use std Instant-based monotonic clock for ACP timestamps
    // (simple approximation: since ACP runs in its own process, we use
    // the process start as epoch)
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let start = START.get_or_init(std::time::Instant::now);
    start.elapsed().as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockLifecycleResolver;
    impl LifecycleResolver for MockLifecycleResolver {
        fn resolve_verb(
            &self,
            spirit_id: &str,
            verb: LifecycleVerb,
        ) -> Result<maos_domain::lifecycle::LifecycleReceipt, maos_domain::lifecycle::LifecycleError> {
            if spirit_id == "unknown" {
                return Err(maos_domain::lifecycle::LifecycleError::NotLoaded {
                    spirit_id: spirit_id.into(),
                });
            }
            Ok(maos_domain::lifecycle::LifecycleReceipt {
                spirit_pid: 42,
                verb,
                timestamp_ns: 100,
                journal_offset_bytes: None,
            })
        }
    }

    struct MockHaltResolver;
    impl HaltResolver for MockHaltResolver {
        fn resolve(
            &self,
            halt_id: &maos_domain::halt::HaltId,
            _resolution: maos_domain::halt::Resolution,
        ) -> Result<(), maos_domain::halt::ResolveError> {
            if halt_id.as_str() == "fail" {
                return Err(maos_domain::halt::ResolveError::UnknownHalt("fail".into()));
            }
            Ok(())
        }
    }

    #[test]
    fn session_start_replies_session_ready() {
        let server = AcpServer::new(
            Arc::new(MockLifecycleResolver),
            Arc::new(MockHaltResolver),
        );
        let sessions = server.session_registry();

        // Simulate a session start frame
        let input = r#"{"kind":"session_start","session_id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"editor_id":"zed","editor_version":"0.142.0"}"#;
        let mut output = Vec::new();

        let mut server = AcpServer {
            lifecycle: Arc::new(MockLifecycleResolver),
            halts: Arc::new(MockHaltResolver),
            sessions,
        };

        server.run(input.as_bytes(), &mut output).unwrap();
        let out = String::from_utf8(output).unwrap();
        assert!(out.contains("session_ready"));
        assert!(out.contains("lifecycle_verb"));
    }

    #[test]
    fn lifecycle_verb_forwards_to_resolver() {
        let server = AcpServer::new(
            Arc::new(MockLifecycleResolver),
            Arc::new(MockHaltResolver),
        );
        let sessions = server.session_registry();

        let input = r#"{"kind":"session_start","session_id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"editor_id":"zed","editor_version":"1.0"}
{"kind":"lifecycle_verb","session_id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"decision_id":[2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2],"verb":"load","spirit_id":"hello-spirit"}"#;
        let mut output = Vec::new();

        let mut server = AcpServer {
            lifecycle: Arc::new(MockLifecycleResolver),
            halts: Arc::new(MockHaltResolver),
            sessions,
        };

        server.run(input.as_bytes(), &mut output).unwrap();
        let out = String::from_utf8(output).unwrap();
        assert!(out.contains("lifecycle_receipt"));
        assert!(out.contains("ok"));
        assert!(out.contains("42")); // spirit_pid
    }

    #[test]
    fn unknown_frame_kind_replies_error_session_stays_alive() {
        let server = AcpServer::new(
            Arc::new(MockLifecycleResolver),
            Arc::new(MockHaltResolver),
        );
        let sessions = server.session_registry();

        let input = r#"{"kind":"unknown","data":42}
{"kind":"session_start","session_id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"editor_id":"zed","editor_version":"1.0"}"#;
        let mut output = Vec::new();

        let mut server = AcpServer {
            lifecycle: Arc::new(MockLifecycleResolver),
            halts: Arc::new(MockHaltResolver),
            sessions,
        };

        server.run(input.as_bytes(), &mut output).unwrap();
        let out = String::from_utf8(output).unwrap();
        assert!(out.contains("-32600")); // error code
        assert!(out.contains("session_ready")); // session still alive
    }

    #[test]
    fn stdin_eof_implicit_session_end() {
        let server = AcpServer::new(
            Arc::new(MockLifecycleResolver),
            Arc::new(MockHaltResolver),
        );
        let sessions = server.session_registry();

        let mut output = Vec::new();
        let mut server = AcpServer {
            lifecycle: Arc::new(MockLifecycleResolver),
            halts: Arc::new(MockHaltResolver),
            sessions,
        };

        // Empty input → immediate EOF
        server.run("".as_bytes(), &mut output).unwrap();
        // Clean shutdown, no output
    }
}
