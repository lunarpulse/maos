#![forbid(unsafe_code)]

//! Story 11.2b (AC4 / F4) — BLOCKING mechanical chokepoint gate.
//!
//! Proves the fail-closed region-identity guard covers EVERY Spirit-facing
//! collective read, by construction:
//!
//! 1. **Guard wired into both Spirit reads** — `LoomLiteStore::region_guard` is
//!    invoked in BOTH `read` and `scan` (the two `CollectiveMemoryPort`-serving
//!    methods). A future edit that adds a third Spirit read without the guard,
//!    or removes the guard from one, REDs here.
//! 2. **Provenance path is NOT Spirit-facing** — `read_all_rows_from` (the
//!    unguarded full-table provenance read) is NOT a member of the
//!    `CollectiveMemoryPort` trait; it is a store-internal inherent method
//!    serving replication-bundle build + test verification only.
//! 3. **Adapter routes through the guarded methods** — `LoomLiteAdapter` (the
//!    sole production `CollectiveMemoryPort` impl) calls `store.read` /
//!    `store.scan`, never `read_all_rows_from`.
//!
//! If any production Spirit-read consumed the provenance path, the guard would
//! leak → this chokepoint REDs → F4 is NOT ZERO-Δ → escalate as a separate
//! port-ABI story (never a silent kernel edit).
//!
//! This is a STATIC architecture test — no Postgres, no live run. It runs in
//! every `cargo test -p maos-loom-lite` and is asserted GREEN by the
//! `check-multi-region-slo` gate's live-read-region-identity leg.

/// The store source (region_guard + read + scan live here).
const STORE_SRC: &str = include_str!("../src/store.rs");
/// The adapter source (the production `CollectiveMemoryPort` impl).
const ADAPTER_SRC: &str = include_str!("../src/adapter.rs");
/// The port trait source (the Spirit-facing collective-memory contract).
const PORT_SRC: &str = include_str!("../../maos-domain/src/ports/collective_memory.rs");

/// Count non-comment occurrences of `needle` in `src` (strips `//` line
/// comments + `///` docs so a doc mention can't fake a real call site).
fn count_code_occurrences(src: &str, needle: &str) -> usize {
    src.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .filter(|line| line.contains(needle))
        .count()
}

fn function_body<'a>(src: &'a str, signature: &str) -> &'a str {
    let start = src
        .find(signature)
        .unwrap_or_else(|| panic!("missing function signature: {signature}"));
    let open = src[start..]
        .find('{')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("missing function body: {signature}"));
    let mut depth = 0usize;
    for (offset, byte) in src.as_bytes()[open..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open..=open + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated function body: {signature}");
}

#[test]
fn region_guard_wired_into_both_spirit_reads() {
    // The guard is invoked in BOTH read and scan. Two real call sites
    // (`self.region_guard(`) — a doc comment must NOT count (comments stripped).
    let guard_calls = count_code_occurrences(STORE_SRC, "self.region_guard(");
    assert_eq!(
        guard_calls, 2,
        "region_guard MUST be invoked in exactly the two Spirit-facing reads \
         (read + scan); found {guard_calls}. A missing guard on either read, or \
         a third unguarded Spirit read, is a fail-closed bypass (AC4)."
    );
}

#[test]
fn attestation_guard_wired_into_both_spirit_reads() {
    let guard_calls = count_code_occurrences(STORE_SRC, "self.attestation_guard(");
    assert_eq!(
        guard_calls, 2,
        "attestation_guard MUST be invoked exactly in read + scan; found {guard_calls}"
    );
}

#[test]
fn team_guard_is_exactly_the_four_guarded_entry_points() {
    for signature in [
        "pub async fn write(",
        "pub async fn read(",
        "pub async fn scan(",
        "pub async fn erase(",
    ] {
        let body = function_body(STORE_SRC, signature);
        assert_eq!(
            count_code_occurrences(body, "self.team_guard("),
            1,
            "{signature} must invoke team_guard exactly once before querying"
        );
    }

    assert_eq!(
        count_code_occurrences(STORE_SRC, "self.team_guard("),
        4,
        "team_guard must have exactly four call sites: write, read, scan, and erase"
    );

    for signature in ["pub async fn write_with_source(", "pub fn pool("] {
        let body = function_body(STORE_SRC, signature);
        assert_eq!(
            count_code_occurrences(body, "self.team_guard("),
            0,
            "{signature} is deliberately unguarded"
        );
    }
}

#[test]
fn provenance_path_not_exposed_via_collective_port() {
    // The unguarded full-table provenance read is NOT a CollectiveMemoryPort
    // member — it is a store-internal inherent method. If it were added to the
    // trait, a Spirit could bypass the guard → RED.
    assert!(
        !PORT_SRC.contains("read_all_rows_from"),
        "read_all_rows_from (the unguarded provenance path) MUST NOT be a member \
         of the CollectiveMemoryPort trait — exposing it would let a Spirit read \
         bypass the region-identity guard (AC4 ZERO-Δ / no-ABI condition)."
    );
    // Sanity: the port DOES expose read + scan (the guarded Spirit reads).
    assert!(
        PORT_SRC.contains("fn read") && PORT_SRC.contains("fn scan"),
        "CollectiveMemoryPort must expose read + scan (the guarded Spirit reads)"
    );
}

#[test]
fn adapter_routes_through_guarded_store_methods() {
    // The production CollectiveMemoryPort impl routes read/scan to the guarded
    // store methods — it must NOT call the unguarded provenance path.
    assert!(
        ADAPTER_SRC.contains("store.read(") || ADAPTER_SRC.contains("store.read ("),
        "LoomLiteAdapter::read must route through the guarded LoomLiteStore::read"
    );
    assert!(
        ADAPTER_SRC.contains("store.scan(") || ADAPTER_SRC.contains("store.scan ("),
        "LoomLiteAdapter::scan must route through the guarded LoomLiteStore::scan"
    );
    assert!(
        !ADAPTER_SRC.contains("read_all_rows_from"),
        "LoomLiteAdapter (the production Spirit-facing port impl) MUST NOT call \
         the unguarded provenance path read_all_rows_from — that would bypass the \
         region-identity guard (AC4 chokepoint)."
    );
}

#[test]
fn region_guard_does_not_reuse_downgrade_router() {
    // AC4 / F4: the store guard must NOT CALL DowngradeRouter::check_region_identity
    // (wrong home — the router carries the router's home, not the store's; reusing
    // it couples the store to the router and is the i64→u32-P1 inattention class).
    // We check for a real CALL (`check_region_identity(`), not a bare mention —
    // the region_guard doc comment legitimately names it to say it is NOT reused.
    let router_calls = count_code_occurrences(STORE_SRC, "check_region_identity(");
    assert_eq!(
        router_calls, 0,
        "LoomLiteStore::region_guard MUST NOT CALL DowngradeRouter::check_region_identity \
         (wrong home operand — AC4 / F4). The store guard compares against the store's own \
         config.home_region, not the router's home."
    );
}
