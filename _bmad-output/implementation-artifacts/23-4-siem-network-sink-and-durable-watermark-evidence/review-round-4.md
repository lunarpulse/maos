# Story 23-4 — review round 4 (progress notes; newest last)

Report `agent://Rev234G4` was read first. D-23-4-16 and the binary size limit stay the operator's (untouched).

## FILE SINK SIMPLIFIED (B1, B2, B3, and the file-test gaps) — `maos-siem/src/export.rs`
- Removed: the pre-append length, the truncation repair, `recover_tail`, the known-clean-tail flag and the code
  `siem_file_tail_unrecoverable`.
- New contract: the sink only APPENDS (`O_APPEND`, no truncate, no read, never a byte it did not write). The FIRST append
  after process start, the FIRST after ANY failed append, and the first to a file that replaced the one written before,
  begin with one `\n` (skipped when the file is empty), which terminates any fragment; blank lines may therefore
  appear and downstream shippers must ignore them. The cursor moves only after the whole batch (leading newline
  included) was written and fsynced. Creation is decided at open: `O_EXCL` first, then plain append; a file this sink
  created owes a parent-directory sync, retried until it succeeds (no EACCES tolerance for a file we created). The
  identity (dev, inode) of the last written file is tracked: a path that now names another file owes a parent sync again
  (EACCES tolerated there) and takes the leading-newline rule. A pre-existing, unchanged file owes no directory sync.
- Tests (`cursor_file_sink_23_4.rs`): consecutive short writes then a clean retry (each attempt keeps the cursor; the final
  batch equals a clean run's; no record glued to a fragment); a crash fragment in a pre-existing file is terminated and
  untouched; a populated unrelated replacement file is neither truncated nor modified, records follow a newline; logrotate
  move (created file, injected failing directory sync keeps the cursor, the retry syncs, 3 sync calls in all); copytruncate
  (records land at the new end, aligned, no hole); a 0200 write-only file still receives batches; a 0300 parent directory
  with a pre-existing file. Plants: leading newline removed FAILED 3 tests; directory sync skipped for a created file FAILED 2;
  truncation reintroduced FAILED the replacement test. All removed.

## R4-1 DONE — transitions run after the read transaction ended (`export.rs`)
- The gap and the reset are DECIDED under the read transaction, then `drop(view)`, then executed (journal write,
  `move_aside`, cursor save + directory sync); a reset restarts the loop with a fresh view. The failure backlog count is
  bounded (`LogView::count_after_capped`, cap 5,000 rows: "at least N"; unknown when unreadable).
- Test `gap_and_reset_transitions_hold_no_wal_reader_while_they_journal` (live adapter, `run_tick_live`): the journal
  callback itself runs `PRAGMA wal_checkpoint(TRUNCATE)`; it can only return busy = 0 when the exporter holds no WAL reader at
  that instant (deterministic, no concurrent thread needed). Plant (view kept across the journal write, 3 sites) FAILED.
  `the_backlog_count_is_capped_and_reports_a_lower_bound` + plant (cap removed) FAILED.

## R4-2 DONE — idle probe checks the frame id; bounded diagnostics (`export.rs`, `insertion.rs`)
- `LogView::live_latest` reads `(rowid, frame_id)` of the newest live row together; the idle path needs both equal to the
  cursor; a mismatch falls through to the full path (reset + journal). `IdleGuard` makes every 60th consecutive idle tick
  (`FULL_CHECK_EVERY_IDLE_TICKS`) take the full path.
- LIVE-runtime tests (`siem_network_23_4.rs`): `a_restored_log_with_the_same_maximum_id_but_other_frames_is_reset_and_re_exported`
  (same ids 1..=5, other frame ids: one `siem.cursor.reset` row, all five new frame ids re-exported) and
  `the_idle_probe_does_not_hide_a_missing_segment_forever` (59 idle ticks cheap, the 60th reports `siem_source_unavailable`).
  Plants: frame id ignored FAILED the first; no periodic full check FAILED the second. NOTE: the restore is SIMULATED by
  rewriting the frame ids in place (a real restore of a second log file under a running daemon is not possible with
  the adapter holding the file); the state the exporter sees is the same.

## R4-3 DONE (test oracle) — `fake_siem.rs`, `tls_sink_23_4.rs`
- `CloseWithoutReading` is decided right after the handshake completes, BEFORE any application read; it reports the bytes it
  actually captured (none, by construction), writes the server's own final flight before waiting for the alert gate (needed for
  TLS 1.2), requires a successful alert write before it opens `alert_sent`, and both tests ASSERT the readiness witness: the
  `peek` result is `Ok(n > 0)` in TLS 1.3; in TLS 1.2 it may be `Ok(0)` because the alert can be consumed together with the
  server's final flight (still before the look: the look sees the close already processed). No plant for this oracle fix.

## R4-4 DONE — `siem_network_23_4.rs`
- The first tick's UNFILTERED `externalId`s must contain the persisted `frame_id` (hex) of the actual `siem.gap` row.
  Not planted separately (the earlier plant for R3-5 style regressions is not applicable); the assertion is direct.

## R4-5 DONE (see gaps)
- AC5: `a_real_frame_over_one_mebibyte_is_sent_alone_and_the_next_row_follows` (a real 1 MiB + 4 KiB row over TLS: three
  batches, the second holds one frame longer than 1 MiB on the wire); `the_connect_phase_has_its_own_bound` (an exhausted
  connect budget is `siem_sink_timeout` and nothing connected); `a_collector_that_stops_reading_turns_the_write_phase_into_a_timeout`
  (fake `Mode::StopReading`, 48 MiB batch, 300 ms write bound => `siem_sink_timeout`).
- AC6: `the_client_key_never_appears_in_the_transparency_log_after_a_run_that_uses_it`: a REAL mutual-TLS export (the fake demands
  a client certificate), then a byte search of the database, its WAL, the cursor and the sealed segments for the key's PEM body
  fragments and `PRIVATE KEY`.
- AC7: the keyed child-boot cases (`good-key`, the sealed store) pass a RANDOM `MAOS_AUDIT_KEY` file explicitly.
- AC1 (recorded, not fixed): TLS 1.2 is tested with ECDHE-ECDSA/AES-128-GCM, not ECDHE-RSA by name; the additive-webpki-roots
  plant was substituted by the empty-bundle plant (round 0, deviations 6 and 7).
- GAPS (R4-D2): no plants for the connect, write-phase, 1 MiB frame, AC6 and AC7 tests; the connect test exercises an
  exhausted budget, not a listener that never accepts (a kernel listener completes the TCP handshake from its backlog, so
  "never accepts" cannot be built without a raw socket library).

## Round 4 totals
- Green: maos-siem (tls-sink) `tls_sink_23_4` 30, `cursor_file_sink_23_4` 19; fault twins 2; `insertion_reader_23_4` 13;
  `siem_network_23_4` 26; 13.5a bin 4; `available_arm` 6; wiring/seam/online/uninstall suites green. 10x under 16 busy loops
  (tls_sink, cursor_file, 13.5a bin, siem_network, insertion_reader): 0 failures. Gates: env contract 167, dependency closure,
  composition root 11/20, kernel baseline 28,339 = pinned, `check-enterprise-identity` 10 legs, `cargo deny` ok, fmt clean, the
  five `cargo check -p maos-bin` shapes. kloc: maos-siem 1,451, maos-audit 9,784, maos-bin 47,256, maos-domain 11,182, kernel 22,154;
  `over_budget []`.
- Stripped release 39,102,032 B (limit 39,069,022: over by 33,010; round 3 39,099,856). Open operator decision.
- Deviations: R4-D1 `cargo fmt --all` in write mode (formatter only). R4-D2 no plants for the connect, write-phase, 1 MiB
  frame, AC6 and AC7 tests, the R4-3 oracle fix and R4-4 (see R4-5). R4-D3 the restore test rewrites frame ids in place
  (R4-2). R4-D4 the file sink treats a pre-existing unchanged file as owing no directory sync (as instructed) and tolerates
  `EACCES` only for a replacement. R4-D5 the leading newline is skipped when the file is empty. R4-D6 `run_tick_live` gained an
  `IdleGuard` parameter and `ExportError::Sink` / `SiemSinkDown` gained `at_least`; the `Appender` seam became `Writer`.
  R4-D7 the retry of a failed file append may duplicate a complete record the failed attempt already wrote (at-least-once, the
  fragment stays in the file; documented in 23-4.j). R4-D8 clippy: two warnings fixed with `allow` attributes
  (`too_many_arguments` on `drive`, a deprecated `fetch_update` name in a test); other warnings are in pre-existing code.
  R4-D9 TLS 1.2 readiness witness is weaker than 1.3's (peek may see EOF after the alert was consumed with the final flight).
