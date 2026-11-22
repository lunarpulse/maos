# Memory-tier quota (FR28, Story 21.6)

The kernel refuses a private-tier or shared-tier memory write that would take a
Spirit identity past its quota, with `MemoryError::QuotaExceeded { tier,
resource, used, requested, limit }`. It never compacts, evicts or truncates:
the Spirit frees space itself (overwrite smaller, or `delete`). The collective
tier is not charged.

## Where an operator sets it

One limit per tier, applied to every Spirit, resolved env → `~/.config/maos/operator.toml`
→ built-in defaults. A Spirit manifest cannot raise it.

```toml
[memory_quota]
private_bytes   = 67108864   # 64 MiB (default)
private_entries = 16384      # default
shared_bytes    = 16777216   # 16 MiB (default)
shared_entries  = 4096       # default
```

Environment overrides: `MAOS_MEMORY_QUOTA_PRIVATE_BYTES`, `MAOS_MEMORY_QUOTA_PRIVATE_ENTRIES`,
`MAOS_MEMORY_QUOTA_SHARED_BYTES`, `MAOS_MEMORY_QUOTA_SHARED_ENTRIES`. An invalid
value prints a warning and keeps the lower-precedence value. The defaults are
operator policy, not workload measurements.

## What is counted

- Unit: key bytes + value bytes (`MemoryValue::approximate_len`) and logical
  entries. Limits are inclusive. Filesystem block and SQLite page overhead are
  not counted.
- Owner: the stable storage identity bound at admission, so a restarted Spirit
  keeps its usage. Private usage is rebuilt from the durable spills on first use;
  memory-only entries do not survive a restart and are not counted after it.
  Shared usage is summed from the owner's rows.
- Shared rows written before the owner column existed stay readable, are not
  counted and cannot be deleted by a Spirit.
- An update is charged only the difference. A write that grows neither bytes nor
  entries succeeds even when the Spirit is full.
- The operator-provided halt context is charged but never refused; the Spirit's
  next growing write is refused while it is over a limit.
- A forget cascade credits every owner it erased from; a forget suspended by a
  legal hold credits nothing.
