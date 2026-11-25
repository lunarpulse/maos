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

/// One attempted hot swap: what was in flight before it, what survived, and the verdict.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Swap {
    pub tokens_held: u32,
    pub tokens_kept: u32,
    pub frames_held: u32,
    pub frames_kept: u32,
    pub schema_before: u32,
    pub schema_after: u32,
    pub migrator_declared: bool,
    pub accepted: bool,
}

/// I6 observation: the swaps attempted against one Spirit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub swaps: Vec<Swap>,
}

/// I6 predicate: no swap, accepted or refused, drops an in-flight token or A2A frame, and an
/// accepted swap crosses a state-schema major only through a declared migrator.
pub fn check(obs: &Observation) -> Result<(), crate::invariants::Violation> {
    use crate::invariants::ensure;
    for s in &obs.swaps {
        ensure(
            s.tokens_kept == s.tokens_held,
            "I6",
            "swap dropped an in-flight call's token",
        )?;
        ensure(
            s.frames_kept == s.frames_held,
            "I6",
            "swap dropped an in-flight A2A frame",
        )?;
        let same_major = s.schema_before >> 16 == s.schema_after >> 16;
        ensure(
            !s.accepted || same_major || s.migrator_declared,
            "I6",
            "swap accepted an incompatible state schema",
        )?;
    }
    Ok(())
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
