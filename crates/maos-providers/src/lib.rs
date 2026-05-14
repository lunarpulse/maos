#![forbid(unsafe_code)]

//! `maos-providers` — pluggable LLM provider drivers (ADR-005).
//!
//! v0.1-β ships the Anthropic driver (`complete` only). Streaming and
//! multi-provider CI matrix ship in Story 5.5b.

pub mod anthropic;
pub mod provider;

pub use anthropic::AnthropicProvider;
pub use provider::{Provider, ProviderError};
