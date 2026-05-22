//! `maos-kernel-core` — the MAOS kernel composition surface.
//!
//! Story 1b.3: crate-level `#![forbid(unsafe_code)]` removed.
//! Every existing module retains its own inner `#![forbid(unsafe_code)]`.
//! The `security/sandbox/` subtree is the sole deliberate `unsafe` zone
//! (OS sandboxing: `pre_exec`, Landlock, seccomp, setrlimit, FFI).
//! See Dev Notes → The `unsafe` decision in Story 1b.3.
//!
//! Per architecture §4.0.2 the kernel is organized as:
//!   - One supervisor: `scheduler` (Spirit Scheduler)
//!   - Four supervised services: `security`, `memory`, `iac`, `capability`
//!   - Two internal modules: `io`, `telemetry`
//!
//! At v0.1-α every module is an **empty hexagonal adapter shell** — port
//! traits live in `maos-domain::ports`; this crate declares the adapter
//! types that will (post v0.1-α) implement those ports. No runtime state,
//! no impl blocks, no async primitives. See architecture §4.0.8 four-property
//! test and §4.0.1 hexagonal/actor split.
//!
//! Story 1b.x lands runtime logic into these shells. Story 1a.3 ships
//! `CryptoProvider`. Story 1a.4 ships `maosctl`. Story 2.2 upgrades
//! `xtask check-service-boundary` from stub to P1–P4 enforcement.

pub mod api;        // surface-classification anchor for NFR-Test-2
pub mod scheduler;  // supervisor — Spirit Scheduler (architecture §4.1)
pub mod security;   // supervised service — Security Manager (§4.3)
pub mod memory;     // supervised service — Memory Manager (§4.2)
pub mod iac;        // supervised service — IAC Bus (§4.5)
pub mod journal;    // supervised internal — Lifecycle Journal per I10 / §4.1 (Story 1b.1)
pub mod capability; // supervised service — Capability Registry (§4.6)
pub mod compliance; // supervised internal — ComplianceClaim structural validator (Story 1b.4)
pub mod io;         // internal module at v0.1 — I/O Subsystem (§4.4)
pub mod telemetry;  // internal module at v0.1 — Telemetry Stream (§4.7)
pub mod inference;  // internal module at v0.1 — Inference Port adapter (Story 1b.4)
pub mod halt;       // Story 3.3 — HaltResolver trait + resolution journaling (Story 4.1 fills mechanism)
pub mod hot_swap;   // Story 5.2 — Hot-Swap Coordinator + CBOR codec + saga compensation
pub mod supervision;   // Story 5.3 — Crash / hang / silent-failure detection + cold restart
pub mod orchestrator;  // Story 3.4 — Orchestrator instruction buffer + journal helpers
pub mod isolation;     // Story 4.5 — Cross-Spirit isolation corpus runner (NFR-Sec-14)
