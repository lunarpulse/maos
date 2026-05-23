#![forbid(unsafe_code)]

//! Log-recall port trait — participant-scoped, cursor-paginated read-side over
//! the Transparency Log with on-demand payload fetch and A2A consent honoring.

use crate::log_recall::{LogFetchResponse, LogRecallError, LogRecallFilter, LogRecallPage};

/// Participant-scoped, cursor-paginated read-side over the Transparency Log.
///
/// Every call is scoped to the calling Spirit (emitter-side at v0.3-β;
/// v0.5+ extends to recipient-side once the companion table lands).
/// The entry intentionally OMITS the raw payload; consumers MUST call
/// `fetch(frame_id)` for the payload.
pub trait LogRecallPort: Send + Sync + 'static {
    /// Class: data-movement
    fn recall(
        &self,
        spirit_pid: u32,
        filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError>;

    /// Class: data-movement
    fn fetch(
        &self,
        spirit_pid: u32,
        frame_id: [u8; 16],
    ) -> Result<LogFetchResponse, LogRecallError>;
}
