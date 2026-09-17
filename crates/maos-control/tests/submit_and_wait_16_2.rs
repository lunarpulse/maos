#![forbid(unsafe_code)]

//! Story 16-2 / §15 R4 — vectors for THE one submit-and-withdraw
//! implementation (`maos_control::submit_and_wait`), one per
//! [`SubmitOutcome`] variant, against a fake port whose submission state and
//! completion channel the test controls.

use std::sync::Arc;
use std::time::Duration;

use maos_control::{
    submit_and_wait, submit_and_wait_with_deadline, OperatorCommand, OperatorCommandPort,
    OperatorOutcome, OperatorSubmission, SubmitOutcome, COMMAND_QUEUED, COMMAND_STARTED,
};

/// A port whose submissions the test rigs: the state word starts where the
/// test puts it, and the completion channel is the test's own.
struct RiggedPort {
    state_word: u8,
    deliver: Option<OperatorOutcome>,
    drop_receiver: bool,
}

impl OperatorCommandPort for RiggedPort {
    fn submit(&self, _command: OperatorCommand, _deadline: Duration) -> OperatorSubmission {
        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(outcome) = self.deliver.clone() {
            tx.send(outcome).expect("send outcome");
        }
        if self.drop_receiver {
            drop(tx); // receiver sees Disconnected, not Timeout
        } else if self.deliver.is_none() {
            // Hold the sender open so the wait TIMES OUT rather than
            // disconnecting — the withdraw-CAS arms.
            std::mem::forget(tx);
        }
        OperatorSubmission {
            operation_id: "op-rigged-0001".to_string(),
            state: Arc::new(std::sync::atomic::AtomicU8::new(self.state_word)),
            completion: rx,
        }
    }

    fn spirit_status(&self, _spirit_id: &str) -> Option<maos_control::SpiritStatusRow> {
        None
    }

    fn daemon_status(&self) -> maos_control::DaemonStatusRow {
        maos_control::DaemonStatusRow {
            pid: 0,
            boot_nonce: "0".into(),
            version: "test".into(),
            spirit_ids: Vec::new(),
            audit_degraded: false,
            audit_drop_count: 0,
        }
    }

    fn orchestrator_status(&self, _spirit_id: &str) -> Option<maos_control::OrchestratorStatusRow> {
        None
    }

    fn applied_revocations(&self) -> Vec<maos_control::RevocationRow> {
        Vec::new()
    }
}

fn pause_command() -> OperatorCommand {
    OperatorCommand::Pause {
        spirit_id: "hello-spirit".into(),
    }
}

/// The handler ran and delivered — the outcome arrives untouched.
#[test]
fn completed_arrives_unchanged() {
    let port = RiggedPort {
        state_word: COMMAND_STARTED,
        deliver: Some(OperatorOutcome::Completed(
            serde_json::json!({"paused": true}),
        )),
        drop_receiver: false,
    };
    let outcome = submit_and_wait(&port, pause_command());
    assert_eq!(
        outcome,
        SubmitOutcome::Completed(OperatorOutcome::Completed(
            serde_json::json!({"paused": true})
        ))
    );
}

/// The completion channel closed without delivering — Internal, never a
/// fabricated outcome.
#[test]
fn disconnected_is_internal() {
    let port = RiggedPort {
        state_word: COMMAND_STARTED,
        deliver: None,
        drop_receiver: true,
    };
    let outcome = submit_and_wait(&port, pause_command());
    assert_eq!(outcome, SubmitOutcome::Internal);
}

/// Budget expired, withdraw CAS WON (still QUEUED) — the command never ran:
/// SpiritBusy, no operation_id handed out.
#[test]
fn queued_at_deadline_is_spirit_busy() {
    let port = RiggedPort {
        state_word: COMMAND_QUEUED,
        deliver: None,
        drop_receiver: false,
    };
    let started = std::time::Instant::now();
    let outcome = submit_and_wait_with_deadline(&port, pause_command(), Duration::from_millis(50));
    assert_eq!(outcome, SubmitOutcome::SpiritBusy);
    assert!(
        started.elapsed() >= Duration::from_millis(40),
        "the budget was actually waited"
    );
}

/// Budget expired, withdraw CAS LOST (port already STARTED) — the id is
/// findable because the task runs on and writes its row.
#[test]
fn started_at_deadline_is_handler_still_running_with_the_ports_id() {
    let port = RiggedPort {
        state_word: COMMAND_STARTED,
        deliver: None,
        drop_receiver: false,
    };
    let outcome = submit_and_wait_with_deadline(&port, pause_command(), Duration::from_millis(50));
    assert_eq!(
        outcome,
        SubmitOutcome::HandlerStillRunning {
            operation_id: "op-rigged-0001".to_string()
        }
    );
}
