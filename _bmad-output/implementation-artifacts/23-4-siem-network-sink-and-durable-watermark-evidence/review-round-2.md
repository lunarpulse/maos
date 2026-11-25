# Story 23-4 — review round 2 (progress notes; newest last)

Report `agent://Rev234G2` was not read (worked from Main's list; deviation R2-D1).

## R2-1 DONE
- `maos_audit::wal_sidecar(db)` appends `-wal` to the FULL file name; `refuse_active_wal` uses it. The only other
  `with_extension("sqlite-wal")` guard in non-test code was the runtime snapshot decision
  (`enterprise_identity.rs`); it is gone: the daemon ALWAYS snapshots through a normal connection
  (`VACUUM INTO`), and `immutable=1` is used only on that copy and on sealed segments.
  (`purge.rs:2042` already appends `-wal`/`-shm`; remaining `with_extension("sqlite-wal")` uses are in tests whose
  databases are named `*.sqlite`.)
- Test: `siem_network_23_4::a_live_database_without_the_sqlite_extension_exports_rows_that_exist_only_in_the_wal`
  (`audit.db`, 5 rows only in `audit.db-wal`): pass.
- Plant E234-PLANT R2-1 (snapshot only when `db.with_extension("sqlite-wal")` exists): FAILED
  (`siem_source_unavailable`: the live file was opened immutable and unreadable). Removed.

## R2-2 and R2-3 DONE (`maos-siem/src/export.rs`, `FileSink`)
- R2-2: the append is write-only (`O_APPEND`); tail recovery is best-effort (needs read access, skipped when
  unreadable: documented limit) and runs only at the first attempt of a sink instance and after a failed append
  (`tail_clean` state). Test `a_write_only_sink_file_still_receives_batches` (mode 0200). Plant R2-2 (recovery
  required, as before) FAILED that test. Removed.
- R2-3: `dir_synced` is per sink instance; the directory sync is retried on every attempt until it has succeeded
  once, injectable via `FileSink::with_dir_sync`. Test `a_failed_directory_sync_is_retried_and_the_cursor_waits_for_it`
  (first sync fails: file exists, cursor absent; second tick re-sends and syncs). Plant R2-3 (no directory sync
  at all, standing in for "inferred from existence") FAILED. Removed. NOTE: the runtime builds a new `FileSink` per
  tick, so the flag resets every tick: it re-syncs the directory each tick (safe, cheap).

## R2-5 DONE (`tls_sink_23_4.rs`)
- New witness `a_collector_that_keeps_making_progress_still_hits_the_handshake_deadline`: a real handshake (server
  certificate padded with 300 SAN names, several kB) is relayed one byte at a time, permits granted by a feeder
  thread at full speed, so no single read waits, yet the flight outlasts the injected 20 ms deadline. Expected
  `siem_sink_timeout`. Plant R2-5 (the old `arm` once + `complete_io` loop) FAILED it (the old code let the
  handshake finish and returned Ok). Removed. Lesson recorded: without `set_nodelay` on the relay, Nagle/delayed
  ACK stalls made the old code time out too, so the first version of the witness passed against the plant.

## R2-6 DONE with a disclosed gap (`tls_sink.rs` `early_close_check`)
- The drain now checks the cancel flag and the write deadline on EVERY iteration and takes at most
  `MAX_DRAIN_READS` = 64 reads. Test `a_flood_of_valid_records_cannot_hold_the_export_past_its_deadlines`
  (fake `Mode::Flood`: KeyUpdate records until the client leaves; the call must return).
- GAP (R2-D2): plant R2-6 (cap and deadline removed) did NOT fail the test: rustls itself ends a KeyUpdate or
  plaintext flood (`tls_refused` / buffer limit) before the loop could spin, and a ticket flood is finite. I
  found no fixture that makes the unbounded loop observable without a timing assertion; the bound is therefore
  tested only as "returns", not proven red.

## R2-8 (a) tests DONE (docs: see below)
- `an_alert_already_readable_before_ours_is_refused_in_tls_13_and_12` (32 MiB frame, larger than loopback socket
  buffering, so the collector's alert is on the wire before our write ends; a 900 KB first version FAILED once in
  10x under 16 busy loops: the alert lost the race, which is a defect of that test, not of the code). Passes for both
  versions. `a_collector_that_closes_after_ours_without_keeping_the_batch_is_acknowledged`
  documents the residual with the new fake `Mode::DiscardThenAck`.

## R2-7 DONE with a disclosed gap (`maos-audit/src/insertion.rs` `LogView::read_after`)
- Pass 1 is now a bounded ordered k-way merge: ONE lookahead entry per source (`next_meta`, an indexed
  `LIMIT 1` lookup), duplicate ids across sources compared on the fly, chosen rows kept; no per-source 512-entry
  window, no second vector. `LogView::peak_lookahead()` (doc-hidden) is the memory proxy.
- Test `the_planner_holds_one_lookahead_per_source_however_large_the_backlog` (40 sealed segments + live file,
  401 rows): ids ordered, `peak_lookahead <= 41`, a 25-row batch equals the head of the 512-row batch.
- Test `a_frame_over_one_mebibyte_is_sent_alone_and_the_next_row_follows` (`cursor_file_sink_23_4.rs`): a 1 MiB+4 KiB
  row between two small ones gives `batches == 3`, the big line over 1 MiB in the file.
- Recorded caveat: batch BOUNDARIES (never row order) can differ from a full read, because the planner adds a
  600-byte per-row allowance while the exporter's real frame sizes include CEF escape expansion; the exporter's
  own post-render byte check (`MAX_BATCH_BYTES`) then cuts the batch.
- GAP (R2-D3): no plant for R2-7: the old planner's behaviour is not a one-line mutation, and the counter is the
  code's own measure, so a plant would only reintroduce the old function wholesale. The test's bound is the
  proxy the reviewer asked for; it was not proven red.

## R2-4 DONE (`maos-bin/src/main.rs`, module `story_13_5a_enterprise_daemon_seam`)
- Test `story_13_5a_governed_read_completes_while_the_network_sink_handshake_is_held`: the production runtime is
  built with `configure_siem` (NETWORK sink, `StubResolver`, generated CA) toward the fake SIEM in
  `Mode::HoldHandshake` (fake included via `#[path]` as `fake_siem_for_13_5a`), booted through
  `build_cohort_a2a_daemon_runtime` plus the production consumer spawn, the real digest request is answered
  (client side gets the correlated reply) while the gate is closed; then: the governed row is in the log, the
  collector reports `Held` and nothing was delivered; the gate opens and the governed row's own `externalId`
  arrives (`Acked`). Needed: a `TestWiring::FullHeldSiem` variant, `Fixture.held_siem`, three small helpers.
- Runs on a multi-thread runtime (4 workers): a first plant run on the default current-thread runtime PASSED
  because the blocked thread starved the test's own 5 s deadline. With multi-thread the plant (the governed
  path calls `forward_audit_to_siem()` inline) FAILED with `a2a partition timeout ... after 5s`. Removed.

## R2-8 (b) and (c) DONE
- RELEASE-HOLDS row 23-4.i, the ADR-051 amendment and the evidence README now carry the honest delivery contract;
  README has OPERATOR DECISION D-23-4-16 (A recommended, B, C). Not decided by the implementer.
- Plant R2-8a (`early_close_check` call disabled) FAILED `an_alert_already_readable_before_ours_is_refused_in_tls_13_and_12`
  and `a_collector_that_closes_first_is_not_an_acknowledgement`. Removed.

## Round 2 totals
- Suites green: maos-siem (tls-sink) incl. `tls_sink_23_4` 25, `cursor_file_sink_23_4` 12; fault twins 2; `insertion_reader_23_4` 9;
  `siem_network_23_4` 22; 13.5a bin 4; `available_arm` 6. 10x under 16 busy loops: first pass 1 failure (the 900 KB
  alert race, fixed with a 32 MiB frame), then 0/10 for `tls_sink_23_4`. Gates: env contract 167, dependency closure,
  composition root 11/20, kernel baseline 28,339 = pinned, `check-enterprise-identity` 10 legs, `cargo deny` ok, fmt
  clean, five `cargo check -p maos-bin` shapes. kloc: maos-siem 1,267, maos-audit 9,715, maos-bin 47,264, kernel 22,154.
- Stripped release: 39,098,192 B (limit 39,069,022: over by 29,170; main 38,911,056). Open operator decision.
- Deviations: R2-D1 `agent://Rev234G2` not read (worked from Main's list); R2-D2 R2-6 plant did not go red;
  R2-D3 no plant for R2-7; R2-D4 R2-3 plant is "no directory sync" rather than "skip when the file exists"; R2-D5 `cargo fmt --all`
  in write mode again; R2-D6 the R2-4 witness reads the collector's events with `try_recv` + `yield_now` polling (no sleeps).
  Clippy was run on maos-siem/maos-audit only as a smoke (pre-existing warnings elsewhere); not triaged per-line.
