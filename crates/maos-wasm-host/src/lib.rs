#![forbid(unsafe_code)]

//! `maos-wasm-host` — wasmtime component-model adapter for WASM Spirit form.
//!
//! # Architecture (Story 11.1a, ADR-031/041)
//!
//! This crate implements the `SpiritHostPort` trait (from `maos-host`) for the
//! WASM component form. It validates `.wasm` components against the
//! `maos:spirit@1.0` WIT world and resolves launch requests into concrete
//! subprocess launch plans pointing at the `maos-wasm-runner` binary.
//!
//! The `maos-wasm-runner` binary IS `BridgeSpawnSpec.program` — it is a real
//! wasmtime component runner that speaks ADR-032 (Content-Length + CBOR) over
//! stdio. The kernel's existing `spawn_and_bridge` launches it unchanged.
//!
//! # Decision D2 (11.1a preflight)
//!
//! This crate is SEPARATE from `maos-host` per ADR-041 isolation and
//! cargo-deny dependency-closure containment. `maos-bin` depends on both;
//! `wasmtime`/`wasmtime-wasi`/`wit-bindgen` are confined to this crate.
//!
//! # Export control (AC6)
//!
//! This crate is behind `--features wasm-host` (OFF by default) in `maos-bin`.
//! A CI gate asserts it is ABSENT from the shippable artifact set. The
//! distributable form must NOT be finalized before export counsel clears
//! the 5D002.c.1 classification question.

pub mod adapter;
pub mod codec;
pub mod config;
pub mod conformance;
pub mod frame_bridge;
pub mod host_state;
pub mod wit_guest;

pub use adapter::WasmHostAdapter;
pub use config::WasmHostConfig;
