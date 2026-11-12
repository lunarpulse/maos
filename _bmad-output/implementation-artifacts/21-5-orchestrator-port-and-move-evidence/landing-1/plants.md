# 21-5 L1 — plants and the red witness (2026-11-12, base `d00802aa`, kernel 27103 / 21256)

Suite: `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs` (`cargo nextest run --locked -p maos-bin --test
orchestrator_equivalence_21_5`, `env -u CI -u GITHUB_ACTIONS`). Baseline on the UNMOVED kernel: 5 pass; the sixth test
(`a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue`) is `#[ignore]`d and RED when run
(`--run-ignored only`), see below. Plants mutate one kernel source line, run the suite, then restore by copy-back from
`/tmp/l1/orig`, `cmp` equal and `touch`; `git status` shows no kernel change after each (observed). Raw logs in `plants/`.

| # | Plant (one line, restored) | Expected red | Observed |
|---|---|---|---|
| P1 | `orchestrator/mod.rs`: `"id={}: queued instruction …"` → `"id={}: ENQUEUED instruction …"` | (a) | FAIL `a_journal_rows_equal_the_fixture`; 4 pass, 1 skipped (`plants/p1-journal-reasoning.log`) |
| P2 | `orchestrator/buffer.rs`: `q.push_back(instruction)` → `q.push_front(instruction)` | (b) | FAIL `b_buffer_is_fifo_bounded_and_drains_in_order` (`plants/p2-enqueue-front.log`). (c) stays green: a front-pushing replay also reverses the pre-queued order, so the round trip coincides with the input; (c) is planted separately by P3 |
| P3 | `scheduler/scheduler_loop.rs`: `for instr in pending {` → `for instr in pending.into_iter().take(2) {` (the replay drops its last item) | (c) | FAIL `c_resume_replay_keeps_order_and_count` (`plants/p3-replay-drops-last.log`) |
| P4 | `orchestrator/registry.rs`: `get` → `Some(self.get_or_create(spirit_id))` | registry vector, never-queued resume | FAIL `b_registry_get_never_creates_…` and `c_resume_of_a_spirit_never_queued_to_creates_no_buffer` (`plants/p4-get-creates.log`) |

## Replay-race witness (D-21-5-07) — red against the unmoved replay

`a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue`: a full 32-item queue; a producer thread spins
`enqueue`; the main task does 200 pause/resume cycles through the real `SpiritSchedulerAdapter` (registry passed as the
replay's buffer source). A replay that is an identity on the queue leaves a full queue no room: the producer is never
accepted and the final queue is `1..=32`.

Observed on the unmoved `scheduler_loop.rs:490-505` (`raw: red/first-run-5-green-1-red.log`, `red/run-01.log`,
`red/run-08.log`, summary `red/ten-runs-summary.txt`): **10 of 10 runs red** (+ the first run), accepted producer
enqueues 573…1788 per run, each paired one-to-one with a `maos: resume re-enqueue failed: orchestrator buffer at
capacity (32)` line — a queued director instruction was **refused on re-enqueue and lost** — and the final queue holds
newer producer items ahead of/instead of `1..=32` (`left: (1638, [1663, 1653, …])`). The `eprintln!` path in the
kernel is therefore reachable under concurrency, not dead.

The test is committed `#[ignore]`d at L1 (hosted CI must stay green on the unmoved kernel); L2 (D-21-5-07 B, replay
under one lock in the leaf) removes the attribute and the same test must then be green — that flip is L2's
failing-before/green-after record.
