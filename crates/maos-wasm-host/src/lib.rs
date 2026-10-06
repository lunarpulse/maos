#![forbid(unsafe_code)]

//! `maos-wasm-host` — wasmtime component-model adapter for WASM Spirit form.
//!
//! # Architecture (Story 11.1a, ADR-031/041)
//!
//! This crate implements the `SpiritHostPort` trait (from `maos-host`) for the
//! WASM component form. It resolves metadata into subprocess launch plans;
//! the contained runner validates the `maos:spirit@2.0.0` WIT world.
//!
//! The `maos-wasm-runner` binary is `BridgeSpawnSpec.program`, a real wasmtime
//! component runner speaking ADR-032 (Content-Length + CBOR) over stdio. It
//! runs inside ADR-031's T2 process boundary once launched by
//! `spawn_and_bridge`; the production launch is
//! `17-3c-wasm-spirit-on-the-bus-under-t2`.
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
pub mod frame_bridge;
pub mod host_state;
pub mod wit_guest;

pub use adapter::WasmHostAdapter;
pub use config::WasmHostConfig;
