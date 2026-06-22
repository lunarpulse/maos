#![no_main]

//! Fuzz target — `IacFrame` wire deserialization (NFR-Sec-6).
//!
//! Exercises the `serde::Deserialize` impl of `maos_domain::frame::IacFrame`
//! against attacker-controlled bytes across BOTH on-wire/round-trip formats.
//!
//! ## CBOR crate decision (preflight N6 — RESOLVED)
//!
//! IacFrame's PRODUCTION on-wire path is `serde_json`: the JSON-RPC transport
//! (`maos-a2a-core/src/transport/json_rpc.rs`), `maos-a2a-tcp`, and `maos-iac`
//! all deserialize frames via `serde_json`. IacFrame is NEVER CBOR-serialized
//! in production. The workspace's canonical-CBOR crate is `serde_cbor = "0.11"`
//! (used by `maos-compliance/src/canonical_cbor.rs` for Merkle/digest hashing).
//! This harness therefore uses `serde_cbor` for the CBOR round-trip arm purely
//! to exercise the `Deserialize` impl against a second format — the production
//! wire format (JSON) is arm 1.
//!
//! ## SmallVec (preflight N5 — RESOLVED, NON-ISSUE)
//!
//! The harness deserializes raw `&[u8]` via serde, so `SmallVec`'s `Arbitrary`
//! impl is irrelevant — there is no `derive(Arbitrary)` struct and no manual
//! `Arbitrary` impl anywhere in this harness.
//!
//! ## Runtime configuration (REQUIRED — see docs/runbooks/fuzz-cadence.md)
//!
//! `serde_cbor` 0.11 (unmaintained) TRUSTS attacker-controlled CBOR length
//! prefixes and amplifies tiny inputs into multi-GB allocation requests. This is
//! a LIBRARY limitation, NOT a MAOS defect: IacFrame's production wire path
//! (arm 1, `serde_json`) is a streaming parser that never amplifies, and IacFrame
//! is never CBOR-serialized in production. To keep the harness from aborting on
//! these OOM-class requests, the target MUST be run with both:
//!   - `ASAN_OPTIONS=allocator_may_return_null=1:detect_leaks=0` — so ASAN
//!     returns NULL (→ serde `Err`, swallowed) instead of aborting on a >1 TB
//!     allocation-size-too-big request;
//!   - `-rss_limit_mb=0` — so libFuzzer's malloc hook does not abort on a
//!     multi-GB allocation request before ASAN can refuse it.
//! With these flags the 7.5 GB-class inputs execute in ~3 ms (allocation
//! refused → `Err` → swallowed); a 60 s soak completes with zero crashes.
//!
//! Both arms swallow `Err` — a deserialize failure is the EXPECTED,
//! non-crashing contract for malformed/adversarial bytes. The target reports a
//! bug only if serde (or a `Deserialize` impl it drives) panics/aborts.

use libfuzzer_sys::fuzz_target;
use maos_domain::frame::IacFrame;

fuzz_target!(|data: &[u8]| {
    // Arm 1 — PRODUCTION wire format: JSON (serde_json). This is the path
    // every real transport deserializes IacFrame with.
    let _ = serde_json::from_slice::<IacFrame>(data);

    // Arm 2 — canonical CBOR (serde_cbor): deserialize + round-trip. The
    // `if let Ok` performs the CBOR deserialize on EVERY input (preserving the
    // bare Deserialize-against-CBOR coverage that a prior redundant standalone
    // arm duplicated), then additionally round-trips (re-serialize ->
    // re-deserialize) to catch non-idempotent Serialize/Deserialize pairs that
    // a single-pass arm misses.
    if let Ok(frame) = serde_cbor::from_slice::<IacFrame>(data) {
        if let Ok(bytes) = serde_cbor::to_vec(&frame) {
            let _ = serde_cbor::from_slice::<IacFrame>(&bytes);
        }
    }
});
