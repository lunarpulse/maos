#![forbid(unsafe_code)]

//! `maos-wasm-host` — wasmtime component-model adapter for WASM Spirit form.
//!
//! # Architecture (Story 11.1a, ADR-031/041)
//!
//! This crate implements the `SpiritHostPort` trait (from `maos-host`) for the
//! WASM component form. It resolves metadata into subprocess launch plans;
//! the contained runner validates the `maos:spirit@2.4.0` WIT world.
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
//! # Export control
//!
//! This crate is included by the default `maos-bin` features. The export
//! gate checks the default dependency closure and the release artifact shape.
//! No ECCN determination or counsel review is implied by that gate.

pub mod adapter;
pub mod codec;
pub mod config;
pub mod frame_bridge;
pub mod host_state;
pub mod lifecycle_wire;
pub mod recall_wire;
pub mod wit_guest;

pub use adapter::WasmHostAdapter;
pub use config::WasmHostConfig;
