# Story 23-4 — review round 1 (Rev234G, Rev234C via Main's work list)

The two review reports (`agent://Rev234G`, `agent://Rev234C`) were NOT read; the implementer worked from
Main's numbered list only (deviation R1-D1).

## Finding map (observed unless marked)

| # | Finding | Fix | Test | Plant (alone, then removed) |
|---|---|---|---|---|
| 1 | early `close_notify` counted as ack | `tls_sink.rs::early_close_check` (process pending records, then a non-blocking drain, before our `close_notify`) | `a_collector_that_closes_first_is_not_an_acknowledgement` (TLS 1.2; fake `Mode::CloseFirst`) | R1-1: check skipped → that test FAILED |
| 2 | expiry need not remove a prefix | `export.rs`: expired ranges interleaved with reads; rows below a range are delivered before the gap is journaled; `LogView::expired_ranges_after` sorted by id | `retained_rows_below_an_expired_range_are_delivered_before_its_gap_is_journaled` | R1-2: `retain` removed → 3 tests FAILED |
| 3 | anchorless gap cursor after a restore | `ResetReason::GapEvidenceLost`; `LogView::expired_segment_ends_at` | `a_gap_cursor_whose_expiry_evidence_is_gone_resets` | R1-3: arm disabled → that test FAILED |
| 4 | new file sink not durable | `FileSink` syncs the parent directory when it created the file | not testable without a crash; code only (limit) | — |
| 5 | partial append | `recover_tail` cuts back to the last `\n` (terminates a file with none) | `a_partial_tail_is_cut_back_before_the_retry_appends` | R1-5: recovery skipped → that test FAILED |
| 6 | journal idempotence | payloads are deterministic (`sink` tag, range, bad-cursor `fingerprint`); the writer skips an identical (intent, payload) row | lib: `a_blocked_cursor_write_journals_the_same_gap_payload_every_attempt`; real writer: `a_blocked_cursor_write_adds_one_gap_row_however_many_ticks_fail` (1 row after 4 failing ticks) | R1-6: dedupe off → 4 rows, FAILED |
| 6b | crash between journal and cursor save | guarantee: at most one row per cursor transition (same dedupe key); docs say so (export.rs module doc, RELEASE-HOLDS 23-4.j) | covered by the same dedupe | — |
| 7 | transient cursor read error reset the log | `Loaded::Io` → tick error `siem_cursor_io`, nothing journaled or re-sent | `an_unreadable_cursor_file_is_a_tick_error_not_a_reset` | R1-7: any error → Unreadable → FAILED |
| 8 | witness accepted any batch | the 13.5a proof now waits for a sink line whose `externalId` is the GOVERNED row's frame id; the held-handshake daemon-level test is NOT written (R1-D2) | `story_13_5a_*` (3, pass) | — |
| 9 | client key ignored in the encrypted-file store | `main.rs` reads `SiemClientKey` through the configured store whatever the backend | `boot_reads_the_client_key_from_the_encrypted_file_store` (key only in the sealed vault) | R1-9: env store → FAILED |
| 10 | widened bundle undetected | control test builds a widened root store by hand: it ACCEPTS the foreign collector; the shipped CA-only config refuses it | `a_widened_root_store_is_detected_by_the_ca_only_witness` | R1-10: a verifier that trusts every server → both CA-only witnesses FAILED |
| 11 | shutdown / per-step handshake | handshake driven record by record, reads sliced at 250 ms, deadline + cancel flag checked each step; `BatchSink::set_cancel`, `run_tick_cancellable`, `EnterpriseRuntime::cancel_siem_export` (consumer calls it on shutdown) | `a_cancel_stops_an_export_stuck_in_a_held_handshake`, `the_runtime_cancel_ends_an_export_held_in_the_handshake` (gate, no timing), `a_trickling_peer_cannot_stretch_the_handshake` | R1-11: handshake cancel check removed → FAILED (10 s hang, code `siem_sink_timeout`) |
| 12 | reader materialised 512 rows × sources | `LogView::read_after(after, max_rows, max_bytes)`: pass 1 ids/sizes, pass 2 only the chosen rows; an oversized single row is returned alone (cannot be split: stated exception to the 1 MiB bound) | `the_reader_honours_the_byte_bound_and_returns_an_oversized_row_alone` | — |
| 13 | read left `-wal`/`-shm` | sources opened `immutable=1` (`segment::open_log_sources_immutable`) | `reading_leaves_no_sidecar_and_the_database_stays_readable` | R1-13: plain open → FAILED |
| 14 | notes lost on failure | `TickFailure{error,tick}`; `SiemSinkDown.notes`; `SiemReport` prints them | `the_gap_note_survives_a_sink_failure_in_the_same_tick`, `a_gap_noted_before_a_sink_failure_is_not_lost` | R1-14: notes dropped → FAILED |
| P3a | `rustls-pemfile` direct dependency | removed from `maos-siem`; PEM via `rustls::pki_types::pem::PemObject`. `Cargo.lock` keeps it for `maos-a2a-tcp` etc., so the `deny.toml` exception text is true again; `cargo deny check` ok | existing TLS/mTLS tests | — |
| P3b | snapshot permissions / stale | created owner-only (0600) before `VACUUM INTO`; sidecars removed; stale `siem-snapshot-*` removed when the exporter is built | `stale_snapshots_are_removed_when_the_exporter_is_built`; no snapshot after failing/cancelled ticks | — |
| P3c | `dedup_by_key` merged different frames | an error | `the_same_insertion_id_under_two_frame_ids_is_an_error` | R1-P3: check disabled → FAILED |
| P3d | resolver thread per lookup | NOT changed; recorded in RELEASE-HOLDS 23-4.j | — | — |
| P3e | key PEM not zeroized | NOT changed; recorded in 23-4.j | — | — |

Plant sweep: `grep -rn E234-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` → no output (exit 1).

## Numbers

- Suites with a random 0600 `MAOS_AUDIT_KEY` (deleted): maos-siem `--features tls-sink` (tls_sink_23_4 21, cursor_file_sink_23_4 9, file/redaction/count suites), fault twins `--ignored` 2, maos-audit `insertion_reader_23_4` 8, maos-bin `siem_network_23_4` 21, wiring 4, seam 1, umbrella 3, uninstall 24, 13.5a lifecycle 3, unit `available_arm` 6 — all pass.
- 10× under 16 busy loops: `siem_network_23_4` (21), `tls_sink_23_4` (21), `cursor_file_sink_23_4` (9), 13.5a lifecycle (3): 40 runs, 0 failures.
- Gates: env-contract 167 PASS, dependency-closure PASS, composition-root PASS (11/20), kernel baseline PASSED 28,339 = pinned, check-enterprise-identity PASSED (10 legs), `cargo deny check` ok, fmt clean, all five `cargo check --locked -p maos-bin` shapes clean.
- kloc (`kloc-check`): maos-siem 1,231 (base 447), maos-audit 9,697, maos-bin 47,056, maos-domain 11,182, maos-kernel-core 22,154 (unchanged); `over_budget []`.
- **Stripped release `maos`: 39,098,320 B** (limit 39,069,022 B; over by 29,298 B; round-0 figure 39,082,192 B; main 38,911,056 B). Dropping the direct `rustls-pemfile` edge did not shrink it: the crate is still linked through `maos-a2a-tcp`, and the round-1 code (per-step handshake, planner, cursor/gap logic) adds more than the edge saved. The size-limit file was not touched; the operator decides.

## Deviations of this round (numbered; the 16 of the first round stand)

R1-D1. The two review reports were not read (worked from the list).
R1-D2. Item 8, daemon-level test "governed read completes while the handshake is held": not written. Reason: the lifecycle fixture builds its runtime through `from_ports` with a file sink; a network sink plus a held fake needs a second fixture variant and a daemon boot (~200 lines in `main.rs` tests). Replaced by: (a) the witness is now the governed row's own `externalId` (fixes the "any batch satisfies it" hole), (b) the consumer is spawned by the production helper, (c) the runtime-level held-handshake burst test and plant R1-… from round 0 (P5).
R1-D3. Item 1, TLS 1.3: not asserted. In 1.3 the client's handshake completes when it sends its Finished, before the server's post-handshake records; whether the early `close_notify` is already on the socket when the batch is written is a race that cannot be forced without a client-side test hook. TLS 1.2 is deterministic (same flight) and tested; the residual is RELEASE-HOLDS 23-4.i.
R1-D4. Item 4 (parent-directory sync) has code but no test: a crash between create and directory sync cannot be simulated in-process.
R1-D5. Plants were run for items 1, 2, 3, 5, 6, 7, 9, 10, 11, 13, 14 and P3c; none for items 4, 8, 12, P3a/b (no distinguishing test or a build-level change).
R1-D6. Item 11: the shutdown flag is checked at each 250 ms read slice and each batch boundary; a blocking `write_tls` bounded by the write timeout (10 s) can still delay a cancel by that bound.
R1-D7. Item 6: the writer dedupes against the LIVE log only (RELEASE-HOLDS 23-4.j).
R1-D8. Item 12: the plan uses stored bytes + a 600-byte allowance per row, then `run_tick` enforces the exact 1 MiB bound on the projected lines; a row larger than the bound goes alone.
R1-D9. `purge` still matches only `*.siem-snapshot-*.sqlite` (not `-wal`/`-shm` sidecars of a crashed snapshot); the daemon removes all three at boot.
R1-D10. `cargo fmt --all` was run in write mode again (formatter only); `--check` clean.
