#![forbid(unsafe_code)]

//! DRR (Deficit Round Robin) scheduler for the Transparency Log writer.
//!
//! Story 6.1 — AC3 / FR1–FR3.
//!
//! Sits between the IAC Bus and the `TransparencyLogAdapter`, ensuring no
//! single Spirit can monopolise the log channel.  Each Spirit receives a
//! 4 KiB quantum per round.  Frames are coalesced into batches of up to
//! 64 frames or 100 ms (FR2).  When a Spirit’s backlog exceeds twice its
//! quantum, a `BudgetWarning` IAC frame is emitted (FR3).

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, Instant};

use maos_domain::frame::IacFrame;
use maos_domain::iac_bus_types::IacBusError;
use maos_domain::invariants::i2::LogBeforeDeliver;
use maos_domain::invariants::i3::FrameOrigin;

use super::transparency_log::{FrameKind, FrameRowWrite, TransparencyLogAdapter};

// ── constants ───────────────────────────────────────────────

/// FR1: per-Spirit quantum.
const QUANTUM_BYTES: usize = 4 * 1024;

/// FR2: max frames per batch.
const BATCH_MAX_FRAMES: usize = 64;

/// FR2: max milliseconds to wait before flushing a partial batch.
const BATCH_MAX_MS: u64 = 100;

/// FR3: backlog threshold multiplier (2 × quantum).
const BACKPRESSURE_MULT: usize = 2;

// ── public API ──────────────────────────────────────────────

/// Handle to the DRR scheduler.  Clone to share between IAC Bus threads.
#[derive(Clone)]
pub struct DrrScheduler {
    tx: mpsc::UnboundedSender<Submission>,
}

struct Submission {
    frame: IacFrame,
    payload_bytes: Vec<u8>,
    tl_kind: FrameKind,
    spirit_pid: u32,
    intent_str: String,
    auto_marker: FrameOrigin,
    intent_lineage: Vec<u8>,
    /// j1-crosshost-2b AC3.2 — carries the typed row-write outcome so the
    /// duplicate-`frame_id` verdict survives the scheduler hop instead of being
    /// flattened back into `()` (which would re-hide the peer replay this repair
    /// exists to surface).
    done: oneshot::Sender<Result<LogBeforeDeliver<FrameRowWrite>, IacBusError>>,
}

impl DrrScheduler {
    /// Spawn the background processor and return a handle.
    pub fn new(
        transparency_log: Arc<TransparencyLogAdapter>,
        budget_warning_tx: mpsc::UnboundedSender<BudgetWarningEvent>,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(processor_loop(rx, transparency_log, budget_warning_tx));
        Self { tx }
    }

    /// Submit a frame for logging.  Returns once the frame has been
    /// persisted to the Transparency Log (I2 guarantee).
    pub async fn submit(
        &self,
        frame: IacFrame,
        payload_bytes: Vec<u8>,
        tl_kind: FrameKind,
        spirit_pid: u32,
        intent_str: String,
        auto_marker: FrameOrigin,
        intent_lineage: Vec<u8>,
    ) -> Result<LogBeforeDeliver<FrameRowWrite>, IacBusError> {
        let (done_tx, done_rx) = oneshot::channel();
        let sub = Submission {
            frame,
            payload_bytes,
            tl_kind,
            spirit_pid,
            intent_str,
            auto_marker,
            intent_lineage,
            done: done_tx,
        };
        self.tx
            .send(sub)
            .map_err(|_| IacBusError::SerializationFailed("drr scheduler closed".into()))?;
        done_rx
            .await
            .map_err(|_| IacBusError::SerializationFailed("drr scheduler dropped".into()))?
    }
}

// ── background processor ────────────────────────────────────

#[derive(Debug)]
pub struct BudgetWarningEvent {
    pub spirit_id: String,
    pub backlog_bytes: usize,
}

async fn processor_loop(
    mut rx: mpsc::UnboundedReceiver<Submission>,
    tl: Arc<TransparencyLogAdapter>,
    budget_warning_tx: mpsc::UnboundedSender<BudgetWarningEvent>,
) {
    let mut queues: HashMap<String, SpiritQueue> = HashMap::new();
    let mut batch: Vec<Submission> = Vec::with_capacity(BATCH_MAX_FRAMES);
    let mut tick = interval(Duration::from_millis(BATCH_MAX_MS));

    loop {
        let recv = rx.recv();
        tokio::pin!(recv);

        tokio::select! {
            biased;

            Some(sub) = &mut recv => {
                let sid = if sub.frame.from.spirit_id.as_str().is_empty() {
                    "<kernel>".to_string()
                } else {
                    sub.frame.from.spirit_id.as_str().to_string()
                };

                // Enqueue
                queues
                    .entry(sid.clone())
                    .or_insert_with(|| SpiritQueue::new(&sid))
                    .push(sub);

                // FR3 backpressure check
                let q = queues.get(&sid).unwrap();
                if q.backlog_bytes() > QUANTUM_BYTES * BACKPRESSURE_MULT {
                    let _ = budget_warning_tx.send(BudgetWarningEvent {
                        spirit_id: sid.clone(),
                        backlog_bytes: q.backlog_bytes(),
                    });
                }

                // If batch is full, flush immediately
                let batch_len = batch.len();
                if batch_len + 1 >= BATCH_MAX_FRAMES {
                    // Pull from DRR queues into batch
                    drain_drr(&mut queues, &mut batch, BATCH_MAX_FRAMES - batch_len);
                    flush_batch(&tl, &mut batch).await;
                }
            }

            _ = tick.tick() => {
                drain_drr(&mut queues, &mut batch, BATCH_MAX_FRAMES);
                if !batch.is_empty() {
                    flush_batch(&tl, &mut batch).await;
                }
            }

            else => {
                // Channel closed — drain remaining work and exit.
                // Note: in-flight submissions already dequeued via recv before close;
                // any remaining frames in per-Spirit queues are drained below.
                while !queues.values().all(|q| q.is_empty()) {
                    drain_drr(&mut queues, &mut batch, BATCH_MAX_FRAMES);
                }
                if !batch.is_empty() {
                    flush_batch(&tl, &mut batch).await;
                }
                break;
            }
        }
    }
}

// ── per-Spirit queue ────────────────────────────────────────

struct SpiritQueue {
    spirit_id: String,
    frames: VecDeque<Submission>,
    deficit: usize,
}

impl SpiritQueue {
    fn new(spirit_id: &str) -> Self {
        Self {
            spirit_id: spirit_id.to_string(),
            frames: VecDeque::new(),
            deficit: 0,
        }
    }

    fn push(&mut self, sub: Submission) {
        self.frames.push_back(sub);
    }

    fn backlog_bytes(&self) -> usize {
        self.frames.iter().map(|s| s.payload_bytes.len()).sum()
    }

    /// Try to dequeue up to `quantum` bytes worth of frames.
    /// Returns the dequeued submissions.
    fn drain_quantum(&mut self, quantum: usize) -> Vec<Submission> {
        self.deficit += quantum;
        let mut out = Vec::new();
        while let Some(front) = self.frames.front() {
            let size = front.payload_bytes.len();
            if size > self.deficit {
                break;
            }
            self.deficit -= size;
            out.push(self.frames.pop_front().unwrap());
        }
        out
    }

    fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

// ── DRR drain logic ─────────────────────────────────────────

/// Drain frames from per-Spirit queues using DRR until `max_frames`
/// have been collected or all queues are empty.
fn drain_drr(
    queues: &mut HashMap<String, SpiritQueue>,
    batch: &mut Vec<Submission>,
    max_frames: usize,
) {
    // Collect spirit IDs that currently have work
    let ids: Vec<String> = queues
        .values()
        .filter(|q| !q.is_empty())
        .map(|q| q.spirit_id.clone())
        .collect();

    if ids.is_empty() {
        return;
    }

    let mut round_robin = ids.iter().cycle();
    let mut remaining = max_frames.saturating_sub(batch.len());

    while remaining > 0 {
        let sid = match round_robin.next() {
            Some(s) => s.clone(),
            None => break,
        };

        let q = match queues.get_mut(&sid) {
            Some(q) if !q.is_empty() => q,
            _ => continue,
        };

        let mut drained = q.drain_quantum(QUANTUM_BYTES);
        let count = drained.len().min(remaining);
        batch.extend(drained.drain(..count));
        // Re-queue excess frames that were not taken into the batch
        for excess in drained.into_iter() {
            q.push(excess);
        }
        remaining = remaining.saturating_sub(count);

        // If no queues have work, stop
        if queues.values().all(|q| q.is_empty()) {
            break;
        }

        // Prevent infinite loop: if no progress was made this round, break
        if count == 0 {
            break;
        }
    }
}

// ── batch flush ─────────────────────────────────────────────

async fn flush_batch(tl: &Arc<TransparencyLogAdapter>, batch: &mut Vec<Submission>) {
    for sub in batch.drain(..) {
        let to_spirit_id = sub.frame.to.first().map_or("", |a| a.spirit_id.as_str());
        let result = tokio::task::block_in_place(|| {
            tl.insert_frame_event_with_id(
                Some(sub.frame.frame_id),
                sub.tl_kind,
                sub.spirit_pid,
                sub.frame.from.spirit_id.as_str(),
                to_spirit_id,
                None,
                std::str::from_utf8(&sub.intent_lineage).unwrap_or(""),
                &sub.payload_bytes,
                sub.auto_marker,
            )
        });
        if let Err(_e) = sub.done.send(Ok(result)) {
            eprintln!("DRR scheduler: subscriber dropped before frame ack");
        }
    }
}
