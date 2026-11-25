# Story 23-4 — review round 3 (progress notes; newest last)

Report `agent://Rev234G3` was read first. D-23-4-16 and the binary size limit stay the operator's (untouched).

## R3-1 DONE — no snapshot copy, no read mark across sink I/O
- `enterprise_identity.rs` no longer calls `VACUUM INTO`. `maos_siem::export::run_tick_live` reads the LIVE log: each
  batch is read through `LogView::open_live` (a normal read-only connection, never `immutable=1`, one `BEGIN`
  transaction begun BEFORE the segment registry is read, so registry + live rows + segment list are one snapshot;
  sealed segments are quiesced, immutable) and the view is dropped BEFORE the sink is touched. Expired ranges are
  re-read per batch. The idle probe (`LogView::live_high_water`, one `max(rowid)`) returns before any view is opened
  when the cursor names the newest live row. A sink failure counts the backlog through a fresh short view; when
  that cannot be read the count is UNKNOWN (`buffered: Option<_>`, line "an unknown number of record(s)").
- Test `a_stalled_collector_pins_no_read_mark_and_leaves_no_snapshot_copy` (`siem_network_23_4.rs`): the fake emits
  `Held` once it HAS the ClientHello; during the stall no `*siem-snapshot*` file exists, 26 more audit inserts
  complete, `PRAGMA wal_checkpoint(TRUNCATE)` returns busy = 0, a rotation seals; on release rows 4..=30 arrive once, in
  order; an idle tick creates no file and dials nothing. Plant (view kept alive across `deliver`) FAILED: busy = 1.
- Behaviour change (tests adapted): the journal row of a gap is now delivered in the SAME tick (the next batch's fresh
  read carries it); the held-handshake test now expects the rest of the burst in the held tick.
- Spill-over: `remove_stale_snapshots` stays (it removes copies a previous binary left).

## R3-2 DONE — `FileSink` tail repair (`export.rs`)
- The sink records the file length before every append; a failed append leaves `dirty_since`; the next attempt cuts
  the file back to it (write permission only, so write-only files work). If the cut fails the error is
  `siem_file_tail_unrecoverable` and nothing is appended (cursor unchanged). The first use of a sink still recovers the
  tail by reading (needs read permission: a file that cannot be read is the documented limit); other errors are
  `siem_sink_file_io`, never discarded. The runtime now keeps ONE `FileSink` across ticks (in the tick slot) so the
  record survives; after a process restart the read-based recovery applies (documented limit).
- Tests: `a_short_write_is_cut_back_before_the_retry_even_on_a_write_only_file` (0200), `an_unrepairable_partial_tail_refuses_to_advance_the_cursor`
  (0400). Plant (repair error discarded) FAILED the second. Removed.

## R3-3 DONE — directory durability per file ENTRY (`export.rs`)
- The directory is synced only for an entry THIS sink created (`unsynced_entry` = file identity), retried until it has
  succeeded once, dropped if the pathname now names another file; a pre-existing entry owes nothing; `EACCES` on the
  directory open is tolerated (documented limit).
- Tests: `a_search_only_parent_directory_does_not_stop_appends_to_an_existing_file` (parent 0300),
  `a_replaced_file_does_not_inherit_the_directory_sync_state_of_the_old_one`. Plants: (a) sync debt inherited by any
  entry FAILED the replacement test; (b) every entry owes a sync and no EACCES tolerance FAILED both. Removed.

## R3-4 DONE — the pre-close drain fails closed on exhaustion (`tls_sink.rs`)
- `drain_pending` (a doc-hidden seam over any non-blocking `Read`) returns `Ok` only when the source ran dry; after
  `MAX_DRAIN_READS` = 64 successful reads it returns `siem_sink_drain_exhausted` (not an acknowledgement; the cursor
  stays). Test `the_drain_proceeds_at_63_records_and_refuses_at_exhaustion_with_an_alert_queued_behind` uses a SCRIPTED
  record source over an in-memory TLS pair (1.3 and 1.2): 63 records then dry => proceeds; 64 + alert => exhausted and
  the alert unread; exactly 64 then dry => exhausted (conservative); 3 + alert => `siem_sink_no_close_notify`.
  Plant (success on exhaustion) FAILED. Removed.

## R3-5 DONE — daemon-level witness with the right ordering (`main.rs`)
- The fake emits `Held` only when it HAS the ClientHello and its flight is ready; its gate cannot time out open (a
  connection still held after 30 s is dropped). The 13.5a boot seeds a row BEFORE the consumer spawns, the test awaits
  `Held` (exporter parked in sink I/O), THEN issues the governed request, asserts the reply and the row with the gate
  still closed (no event reported), THEN releases and waits for the governed row's own `externalId`. Plant (inline
  `forward_audit_to_siem()` in the governed path) FAILED with `a2a partition timeout ... after 5s`. Removed.

## R3-6 DONE — injected clock; real drain boundary (`tls_sink.rs`, `tls_sink_23_4.rs`)
- `drive_handshake` (doc-hidden, generic over `StepIo`) takes the clock and the cancel flag as arguments. Test
  `steady_progress_beyond_the_handshake_deadline_is_a_timeout_on_an_injected_clock`: an in-memory lockstep server hands
  the flight over one byte per read, every read succeeds (asserted: 0 timeouts, at least 5 reads), the injected clock
  advances 10 ms per read against a 100 ms budget => `siem_sink_timeout`; the same peer in a 1 h budget completes.
  Plant (`complete_io`) FAILED it and the cancel test. The relay witness and the KeyUpdate flood witness are gone
  (also `Mode::Flood`). `the_drain_checks_cancellation_and_the_deadline_at_every_read`: a flag raised after 7 reads and a
  clock passing the deadline mid-drain stop it before the cap. Plant (no per-read cancel/deadline) FAILED. Removed.

## R3-7 DONE — scripted ordering and the residual shown precisely (`tls_sink_23_4.rs`, fake)
- New fake `Mode::CloseWithoutReading` (alert + FIN WITHOUT reading anything; `alert_hold`/`alert_sent` gates) and a
  doc-hidden `TlsSink::with_close_hook` (runs before and after the pre-close look). Refused: the alert is proven
  written (`alert_sent`) and readable (`peek`) BEFORE the look, TLS 1.3 and 1.2 (`siem_sink_no_close_notify`).
  Accepted (the DOCUMENTED residual): a collector that reads nothing whose alert becomes readable only AFTER the look.
  The 32 MiB payload is gone. Plant (look disabled) FAILED the refused-case test. Removed.

## R3-8 DONE (largely moot) — `export.rs`
- The snapshot (and its leak on a failed source open) no longer exists (R3-1). The remaining half is done: a sink
  failure with an unreadable log reports `buffered: None` ("an unknown number of record(s)"), test
  `an_unreadable_log_after_a_sink_failure_reports_an_unknown_backlog_not_zero`; plant (`or(Some(0))`) FAILED.

## R3-9 DONE — `insertion.rs`
- `read_after(_, 0, _)` returns an empty batch. Test `a_zero_row_limit_returns_an_empty_batch`; plant FAILED. Also
  `the_live_view_reads_an_active_wal_and_releases_its_read_mark_when_dropped` (live view reads an active WAL, one
  consistent snapshot, the open view pins a mark (busy 1), dropping it releases it (busy 0)).

## Docs
- ADR-051 (cursor bullet), runbook (codes table row, limitation row), RELEASE-HOLDS 23-4.i and 23-4.j updated.

## Round 3 totals
- Green: maos-siem (tls-sink) `tls_sink_23_4` 27, `cursor_file_sink_23_4` 17; fault twins 2; `insertion_reader_23_4` 11;
  `siem_network_23_4` 23; 13.5a bin 4; `available_arm` 6; wiring/seam/online/uninstall suites green. 10x under 16 busy
  loops (tls_sink, cursor_file, 13.5a bin, siem_network, insertion_reader): 0 failures. Gates: env contract 167,
  dependency closure, composition root 11/20, kernel baseline 28,339 = pinned, `check-enterprise-identity` 10 legs,
  `cargo deny` ok, fmt clean, five `cargo check -p maos-bin` shapes. kloc: maos-siem 1,444, maos-audit 9,754, maos-bin 47,244,
  maos-domain 11,182, kernel 22,154; `over_budget []`.
- Stripped release 39,099,856 B (limit 39,069,022: over by 30,834; round 2 39,098,192). Open operator decision.
- Deviations: R3-D1 `cargo fmt --all` in write mode (formatter only). R3-D2 the idle probe does not re-verify the
  cursor anchor until a row is to be sent (documented in 23-4.j). R3-D3 behaviour changes with adapted tests: a gap's
  journal row is delivered in the same tick; a held tick now also delivers rows inserted during the stall. R3-D4 a
  crash-torn tail on a write-only file is not repaired and the in-memory pre-append offset does not survive a restart (documented,
  23-4.j). R3-D5 new doc-hidden public seams in `maos-siem::tls_sink` (`drive_handshake`, `drain_pending`, `StepIo`,
  `with_close_hook`) and `LogView::peak_lookahead`; new typed codes `siem_file_tail_unrecoverable` and
  `siem_sink_drain_exhausted` (runbook updated). R3-D6 the fixture of the 13.5a held test seeds one row before the consumer
  spawns and the test now runs only with `online-adapters`. R3-D7 `remove_stale_snapshots` stays for old-build leftovers.
  R3-D8 clippy on the touched crates: remaining warnings are in pre-existing code (`AuditError::Io` closure in
  `maos-audit`, `lib.rs:143`), not triaged. R3-D9 the 63/64 boundary is conservative: exactly 64 records then dry is also refused.
