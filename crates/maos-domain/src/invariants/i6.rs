//! I6: Hot-swap preserves Capability Tokens for in-flight tool calls.
//!
//! The new Spirit inherits the token, not the call. In-flight A2A frames
//! at the predecessor are inherited by the successor under a drain-barrier.
//!
//! # Enforcement
//!
//! - **v0.1**: `—` (not yet enforced; design-aspirational).
//! - **v0.3**: `runtime` — hot-swap state-transfer + token preservation
//!   operational.
//! - **v0.5 / v0.9 / v1.0 / v1.5**: `runtime` (unchanged).
//!
//! # Invariant statement (doctest)
//!
//! ```
//! use maos_domain::invariants::i6::{InvariantI6, HotSwapState};
//!
//! let _marker: InvariantI6 = InvariantI6;
//! // At v0.1-α we codify the state-machine vocabulary only.
//! let state = HotSwapState::PreSwapOut;
//! assert!(matches!(state, HotSwapState::PreSwapOut));
//! ```

/// I6 marker type — Hot-swap preserves Capability Tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI6;

/// Hot-swap state-machine vocabulary — runtime enforcement deferred to v0.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum HotSwapState {
    /// Predecessor has received swap signal; tokens frozen.
    PreSwapOut = 0,
    /// Predecessor state archived; tokens transferred to successor.
    SwapOutComplete = 1,
    /// Successor activated; tokens valid under successor context.
    SwapInComplete = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hot_swap_state_discriminants() {
        assert_eq!(HotSwapState::PreSwapOut as u8, 0);
        assert_eq!(HotSwapState::SwapOutComplete as u8, 1);
        assert_eq!(HotSwapState::SwapInComplete as u8, 2);
    }
}
