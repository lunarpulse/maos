---
title: Backup & Restore Drill
sidebar_position: 3
description: Quarterly Transparency Log backup, cold restore, and Merkle cross-check procedure.
---

# Backup & Restore Drill (DR-1)

A quarterly drill to verify Transparency Log backup, cold restore, and Merkle integrity.

> Canonical runbook: `docs/runbooks/dr-1-restore-drill.md`

**Frequency:** quarterly
**Owner:** on-call SRE / platform operator
**Scope:** single-region Transparency Log backup + cold restore + Merkle cross-check

## Prerequisites

- `maosctl` binary available on PATH
- Active TL database (default path: `$MAOS_HOME/audit/transparency.sqlite`)
- Writable scratch directory for backup and restore targets
- Stopwatch / `time` command for RTO measurement

## Procedure

### 1. Create backup

```bash
maosctl backup create --dest /tmp/dr-drill/tl-backup.sqlite
```

Uses the SQLite online backup API to produce a WAL-checkpoint-consistent snapshot of the live TL. Safe to run while the kernel is writing.

### 2. Simulate crash

```bash
# Record the pre-crash latest timestamp for RPO verification
maosctl backup verify --backup /tmp/dr-drill/tl-backup.sqlite

# In a real drill, stop the kernel process
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

The restore command automatically runs a Merkle cross-check. To verify independently:

```bash
maosctl backup verify --backup /tmp/dr-drill/restored/transparency.sqlite
```

This recomputes the Merkle root from all `frame_id` values in the restored database and byte-compares against the source root. A mismatch indicates corruption during backup or restore.

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

## RTO measurement methodology

- **Start:** wall-clock time at `maosctl backup restore` invocation
- **Stop:** wall-clock time at first successful `maosctl audit query` returning >= 1 row
- **Target:** < 4 hours for prod-scale TLs

## Honest risk: R8-DR

Prod-scale 4h RTO is **CI-untestable**: the CI Transparency Log is trivially small (< 1000 frames, restores in < 1s). The quarterly manual drill with a production-scale database is the only reliable RTO measurement. CI exercises the code path (backup -> restore -> Merkle verify -> query) to catch regressions, but cannot validate wall-clock RTO at scale.

## Cleanup

```bash
rm -rf /tmp/dr-drill/
```
