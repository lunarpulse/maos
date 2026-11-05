# Hosted A: local partial-index proxy, not a callgrind pass

Hosted run 41962667293 observed 1,049,938.8 routed instructions versus the 997,449.662 baseline (+5.3%, tolerance 2%). Neither baseline nor tolerance was changed.

Schema initialization now drops the former full kind/intent indexes and creates new predicate-only indexes (`idx_tlog_distillate_rowid WHERE kind = 11`, `idx_tlog_redistill_request_rowid WHERE intent = 'distillate.redistill.requested'`). The affected readers pin those names. The outcome index is retained. The existing-database migration witness and the proxy both passed (`index-final-1.stdout:3-6`, cargo test exit 0).

Observed proxy (`index-final-1.stderr:76`), 20,000 ordinary appends:

| Indexes | SQLite pages | Page-accounted bytes | SQLite VM steps |
| --- | ---: | ---: | ---: |
| Former full indexes | 860 | 3,522,560 | 1,640,000 |
| Predicate-only indexes | 742 | 3,039,232 | 1,640,000 |

This is 118 fewer pages / 483,328 fewer page-accounted bytes. It is not a measurement of filesystem allocated bytes, routed native instructions, or hosted wall time. SQLite opcode counts do not price native B-tree work; the unchanged VM-step count is not evidence that callgrind's instruction delta is zero.

Local availability probe: `command -v valgrind` exited 1 with no output. The hosted routed-instruction leg therefore remains **UNVERIFIED**; its unchanged 2% gate is the judge. No local cheap proxy is substituted for that acceptance result.
