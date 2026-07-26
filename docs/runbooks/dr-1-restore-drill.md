# DR-1: Transparency Log Restore Drill

**Frequency:** quarterly (calendar reminder)
**Owner:** on-call SRE / platform operator
**Scope:** single-region Transparency Log (TL) backup + cold restore + Merkle cross-check

---

## Pre-requisites

- `maosctl` binary available on PATH
- Active TL database (default path: `$MAOS_HOME/audit/transparency.sqlite`)
- Writable scratch directory for backup and restore targets
- Stopwatch / `time` command for RTO measurement

## Procedure

### 1. Create backup

```bash
maosctl backup create --dest /tmp/dr-drill/tl-backup.sqlite
```

This uses the SQLite online backup API to produce a WAL-checkpoint-consistent
snapshot of the live TL. Safe to run while the kernel is writing.

### 2. Simulate crash

```bash
# Record the pre-crash latest timestamp for RPO verification later
maosctl backup verify --backup /tmp/dr-drill/tl-backup.sqlite
```

In a real drill, stop the `maos` kernel process to simulate a crash:

```bash
kill -9 $(pidof maos)
```

### 3. Restore from backup

**Start the RTO timer here.**

```bash
time maosctl backup restore \
  --backup /tmp/dr-drill/tl-backup.sqlite \
  --target /tmp/dr-drill/restored/transparency.sqlite
```

### 4. Verify Merkle integrity (R-DR1)

The restore command automatically runs a Merkle cross-check. To verify
independently:

```bash
maosctl backup verify --backup /tmp/dr-drill/restored/transparency.sqlite
```

This recomputes the Merkle root from all `frame_id` values in the restored
database and byte-compares against the source root. A mismatch indicates
corruption during backup or restore.

### 5. Query test — first successful read

```bash
MAOS_AUDIT_DB=/tmp/dr-drill/restored/transparency.sqlite \
  maosctl audit query
```

**Stop the RTO timer when the query returns rows.**

### 6. Record results

| Metric       | Value                 |
| ------------ | --------------------- |
| Drill date   | YYYY-MM-DD            |
| Backup size  | N MB                  |
| Frame count  | N                     |
| Merkle match | YES / NO              |
| RTO measured | Ns (target: < 4h)     |
| Operator     | name                  |
| Notes        |                       |

---

## RTO measurement methodology

- **Start:** wall-clock time at `maosctl backup restore` invocation
- **Stop:** wall-clock time at first successful `maosctl audit query` returning ≥1 row
- **Target:** < 4 hours for prod-scale TLs

## Honest risk: R8-DR

Prod-scale 4h RTO is **CI-untestable**: the CI Transparency Log is trivially
small (< 1000 frames, restores in < 1s). The quarterly manual drill with a
production-scale database is the only reliable RTO measurement. CI exercises
the code path (backup → restore → Merkle verify → query) to catch regressions,
but cannot validate wall-clock RTO at scale.

## Legal holds in tenant mode

Team-shard Transparency Log backups do **not** contain `legal_holds`. Holds are
principal-global and remain in the Host-global artifact
`$MAOS_HOME/audit/transparency.sqlite`; the daemon attaches that authority to a
team shard at boot. Before a tenant-mode DR operation:

Team-scoped holds are **ABSENT** in this release. Story 13.6 owns the
authoritative semantic model and implementation; do not emulate them with
principal-global rows.

1. Run `maosctl legal-hold list` and retain the JSON with the drill evidence.
2. Back up the Host-global artifact separately from every team-shard TL.
3. Restore the Host-global artifact before starting a tenant daemon. An
   unbound or missing hold authority fails closed; it must never be treated as
   an empty hold set.
4. Re-run `maosctl legal-hold list` after restore and reconcile it byte-for-byte
   with the pre-drill inventory before permitting erasure operations.

## Reading an erasure artifact

A GDPR uninstall can emit two artifacts, and they answer different questions.
Do not submit either one alone as a complete Article 17 response.

| Artifact | What it attests |
|---|---|
| `<spirit>-<ns>-<root>.bundle` (erasure proof) | Per-category outcome for this run: `Removed { count }`, `VerifiedEmpty`, or `CoverageGap { reason }`. This is the authoritative record of what was and was not erased. |
| `regional-teardown-<region>-<ns>.json` | Signed attestation that the cascade completed **over the stores named in `forget_cascade.stores_covered`** — currently `private` and `principal_index`. It is scoped, not all-tier. |

Rules:

- Always read the receipt beside the proof bundle from the same run. A receipt
  verifying `Ok` does **not** mean every backend was erased; stores listed in
  `UNCOVERED_STORES` are excluded by construction and appear in the proof as a
  `CoverageGap`. The Shared tier is such a gap today, owned by Story `13-5h`.
- A `held` terminal (exit 3) may still carry a proof path. That proof is
  **partial**: it records the principals this run erased and lists the held
  principals as a legal-hold `CoverageGap`. It is never a complete-erasure
  submission. Releasing a hold changes eligibility only — it does not erase.
- A held run writes no regional teardown receipt. A held run is not a teardown.
- Terminal exit codes: `0` erased, `3` held, `4` not-found, `5` failed. The
  JSON terminal on stdout and the exit code are one contract; if they ever
  disagree, treat the run as failed and escalate.

## Cleanup

```bash
rm -rf /tmp/dr-drill/
```
