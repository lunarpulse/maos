#![forbid(unsafe_code)]

//! `gen_fixture` — Story 1b.5b FR4 fixture generator.
//!
//! Synthesizes 1000 deterministic [`maos_audit::Fr4Entry`] records as NDJSON
//! and writes them to the path passed on argv. Used by
//! `scripts/gen_hello_spirit_fixture.sh` to produce the checked-in fixture
//! `crates/maos-audit/tests/fixtures/hello-spirit-1k.jsonl`.
//!
//! ## Design notes (per Story 1b.5b Decision D5)
//!
//! - **Zero kernel-core dependency.** The generator constructs `Fr4Entry`
//!   structs directly and writes them via `serde_json`. It does not open a
//!   `TransparencyLogAdapter`, does not depend on any kernel crate, and
//!   does not need `cargo tree -p maos-audit` to grow.
//! - **Hand-rolled LCG instead of `rand`/`rand_chacha`.** Numerical
//!   recipes 64-bit LCG (multiplier 6364136223846793005, increment
//!   1442695040888963407). Deterministic given a fixed seed; reproduces
//!   byte-identical output across runs and platforms.
//! - **Mediated calls only.** Every entry carries non-null
//!   `capability_token`, non-zero `spirit_pid` (1..=5), non-zero
//!   `boot_nonce`, a known `call_type` (`inference.call` or
//!   `capability.invocation`), and a monotonically increasing
//!   `timestamp_ns`. This satisfies AC2's "1000/1000 mediated" condition.
//!
//! ## Usage
//!
//! ```text
//! cargo run --quiet -p maos-audit --bin gen_fixture -- <out_path>
//! ```

use std::io::{BufWriter, Write};

use maos_audit::Fr4Entry;

/// Deterministic 64-bit Linear Congruential Generator (Numerical Recipes).
/// Reproducible across platforms; no external dep, no `unsafe`.
struct Lcg(u64);

impl Lcg {
    const MULTIPLIER: u64 = 6_364_136_223_846_793_005;
    const INCREMENT: u64 = 1_442_695_040_888_963_407;

    fn new(seed: u64) -> Self {
        Lcg(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        self.0
    }
}

/// Render bytes as lowercase hex without depending on `hex` crate.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = std::env::args()
        .nth(1)
        .ok_or("usage: gen_fixture <output-path>")?;

    // Deterministic seed. Documented in `scripts/gen_hello_spirit_fixture.sh`.
    const SEED: u64 = 0x5B_F0_1A_5B_5B_F0_1A_5Bu64;
    const ENTRY_COUNT: usize = 1000;
    const BOOT_NONCE: u64 = 0xCAFE_F00D_DEAD_BEEF;
    const BASE_TIMESTAMP_NS: u64 = 1_700_000_000_000_000_000;

    let mut rng = Lcg::new(SEED);
    let file = std::fs::File::create(&out_path)?;
    let mut writer = BufWriter::new(file);

    for i in 0..ENTRY_COUNT {
        // 16-byte frame_id, deterministic per entry.
        let mut frame_id = [0u8; 16];
        let hi = rng.next_u64().to_be_bytes();
        let lo = rng.next_u64().to_be_bytes();
        frame_id[..8].copy_from_slice(&hi);
        frame_id[8..].copy_from_slice(&lo);

        // 32-byte capability token, deterministic per entry — every entry
        // gets a non-null token (FR4: 100% mediation).
        let mut token = [0u8; 32];
        for chunk in token.chunks_mut(8) {
            chunk.copy_from_slice(&rng.next_u64().to_be_bytes());
        }

        // spirit_pid varies 1..=5 (non-zero per AC2 fixture spec).
        let spirit_pid: u32 = ((rng.next_u64() % 5) + 1) as u32;

        // Alternate call_type by index. ~50/50 split between inference.call
        // and capability.invocation matches the kernel-side pattern.
        let call_type: &'static str = if i % 2 == 0 {
            "inference.call"
        } else {
            "capability.invocation"
        };

        // Monotonically increasing timestamp_ns with deterministic jitter.
        let jitter = (rng.next_u64() % 1_000_000) + 1;
        let timestamp_ns = BASE_TIMESTAMP_NS + (i as u64 * 1_000_000) + jitter;

        let entry = Fr4Entry {
            call_id: hex(&frame_id),
            capability_token: hex(&token),
            spirit_pid,
            boot_nonce: BOOT_NONCE,
            call_type: call_type.into(),
            timestamp_ns,
        };

        let line = serde_json::to_string(&entry)?;
        writer.write_all(line.as_bytes())?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    eprintln!("gen_fixture: wrote {ENTRY_COUNT} entries to {out_path}");
    Ok(())
}
