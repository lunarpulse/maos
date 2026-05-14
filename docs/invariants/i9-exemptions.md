---
id: I9-exemptions
title: I9 Exemption Register
---

# I9 Exemption Register

This file enumerates every `#[i9_exempt(reason = "...")]` site in the kernel-core tree.
Each entry must include a one-paragraph rationale signed by ≥2 maintainers.

## Entries

### `PolicyTable` — `crates/maos-kernel-core/src/capability/cap_policy/mod.rs`

**Reason:** operator policy table; structural-state caching per I9 — bounded TTL,
key=spirit_id, no parameter drift. The `Arc<ArcSwap<PolicyTableInner>>` pattern
enables read-mostly copy-on-write updates without blocking hot-path readers.

### `CapQuotaTracker` — `crates/maos-kernel-core/src/capability/cap_quota/mod.rs`

**Reason:** per-Spirit budget counter; structural-state caching per I9 — bounded
by Spirit lifetime, key=spirit_id, no parameter drift. The `DashMap<SpiritId, AtomicU64>`
shards concurrent access with negligible contention at v0.1-β's small Spirit counts.

### `CapTokensShardRing` — `crates/maos-kernel-core/src/capability/cap_tokens/mod.rs`

**Reason:** capability token shard ring lives inside the I9-whitelisted directory
`capability/cap_tokens/` per `xtask/i9-whitelist.toml`. Persistent state (the
64-shard `Arc<[CapShard; 64]>`) is structurally cached per I9 — bounded by token
TTL (≤60s high-privilege per ADR-023), keyed by token_id, no cross-key aggregation.
