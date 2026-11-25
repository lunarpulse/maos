# Story 23-4 — review round 5 (progress notes; newest last)

Report `agent://Rev234G5` was NOT read (worked from Main's list; deviation R5-D0). D-23-4-16 and the size limit stay the
operator's.

## R5-1 DONE — `externalId` is the LAST token of every line (`maos-siem/src/lib.rs`)
- CEF is now `... msg=<payload> externalId=<frame id>` (the RFC 5424 line ends with the CEF, so the id ends the file line and
  the wire frame). The CEF escape test, the fake's `external_ids` parser and the 13.5a needle (`" externalId=<id>\n"`) follow.
- Test `no_proper_prefix_of_a_record_carries_a_complete_external_id`: for EVERY cut position of every written record no prefix
  contains `externalId=<complete id>`, and a trailing partial id is a strict prefix of the id. Plant (id moved back before
  `msg=`) FAILED. (The frame id is 32 hex characters, not 64.)

## R5-2 DONE — copytruncate is not supported; the detectable case is not acknowledged (`export.rs`)
- After the writer returned (write + `sync_data`) the sink compares the fd's size (`fstat`, no read permission) with the length
  it saw at open plus the bytes it wrote: a shorter file is `siem_file_truncated_during_append`, cursor kept, leading newline
  armed. Test `a_file_truncated_between_the_write_and_the_sync_does_not_advance_the_cursor` (an injectable writer truncates
  between write and sync); plant FAILED. A truncate AFTER our fstat is undetectable. The earlier round-4
  `a_copytruncated_file_keeps_receiving_aligned_records_at_its_new_end` test (truncate between ticks) still holds: records
  land at the new end.

## R5-3 DONE — one parent-directory sync per process start (`export.rs`)
- A file found existing on the first use of a sink owes one parent sync (EACCES tolerated for a file we did not create),
  `session_synced` records it. Test `a_pre_existing_file_syncs_its_parent_once_per_process_start` (injected counter: 1 call, not
  2 on the next tick); plant FAILED.

## R5-4 DONE — dangling symlink (`export.rs`)
- `open_append`: `O_EXCL` AlreadyExists + plain open NotFound + `read_link` => create the link's target with `O_EXCL`; creation
  and the directory sync are attributed to the TARGET's path (its parent). One level of link only. Test
  `a_dangling_symlink_is_followed_and_its_targets_directory_is_synced`; plant FAILED.

## R5-5 DONE (test oracle) — `CloseWithoutReading` never decrypts application data (`fake_siem.rs`)
- The mode is now its own state machine (`serve_close_without_reading`): raw TCP bytes are buffered and rustls is fed ONE
  TLS record per step; feeding stops the moment `is_handshaking()` turns false, so anything the client sent behind its
  Finished is never given to rustls. The last step's `plaintext_bytes_to_read()` must be 0 before the alert is released; else the
  connection ends as `Event::Refused("fixture defect: ...")` and both tests fail. The successful alert write and the asserted `peek`
  witness stay. No plant (the defect path cannot be reached with the shipped client).

## R5-6 DONE — a failing full check is latched (`export.rs` `IdleGuard`, test in `siem_network_23_4.rs`)
- `IdleGuard` latches `failing` after any failed tick and clears it only after a successful FULL-path tick (a cheap idle tick
  neither clears nor sets it); while latched every tick takes the full path, so `SiemReport` keeps the one failure line and
  prints "acknowledged again" only after the segment is restored and a full check passes. Test: 59 cheap ticks, the 60th fails,
  three more fail silently, the restored segment gives one recovery line. Plant (no latch) FAILED.

## R5-7 DONE — the key fixture is cross-platform (`siem_network_23_4.rs`)
- `audit_key_file` uses `getrandom::fill` (already a `maos-bin` dependency, no new one) and `#[cfg(unix)]` for the 0600 mode.

## R5-8 DONE (a, b, c, d; see gaps)
- (a) `TlsSink::with_connector` (doc-hidden seam): the nonzero-budget case now reaches the connector with a positive bound no
  larger than the budget and a socket that never completes (the connector returns `TimedOut`) gives `siem_sink_timeout`;
  the zero budget never reaches it. Plant (3,600 s bound instead of the remaining budget) FAILED.
- (b) the write-phase test goes through the EXPORTER (`run_tick`): the fake emits `Held` only after the handshake completed and it
  stopped reading, the test asserts it, asserts `siem_sink_timeout`, no cursor file, and `buffered == Some(1)`. NO plant (the write
  deadline is 300 ms but an unbounded write would hang the test for the 30 s fake bound, not fail fast).
- (c) the secret witness now requires every inspected file (database, WAL, cursor, sealed segments) to read successfully and
  non-empty (the WAL may be empty), and creates a real sealed segment (sealer + rotation) and asserts it was inspected.
- (d) `maos_siem::tls_sink::root_store` + `the_root_store_holds_exactly_the_anchors_of_the_configured_bundle`; plant (an extra
  anchor) FAILED. The literal "webpki-roots added" plant is still impossible offline; the evidence README asks for the acceptance
  change (see `README.md`, "AC1c").

## R5-9 DONE — evidence decisions
- OPERATOR DECISION D-23-4-17 (A recommended / B) is in the evidence `README.md`; RELEASE-HOLDS 23-4.c was reconciled with 23-4.i
  (and the runbook sentence). 23-4.j, the runbook (a rotation bullet) and ADR-051 carry the R5-1/R5-2/R5-3/R5-4 contract.

## Round 5 totals
- Green: maos-siem (tls-sink) `tls_sink_23_4` 31, `cursor_file_sink_23_4` 23; fault twins 2; `insertion_reader_23_4` 13;
  `siem_network_23_4` 26; 13.5a bin 4; `available_arm` 6; wiring/seam/online/uninstall suites green. 10x under 16 busy loops
  (tls_sink, cursor_file, 13.5a bin, siem_network, insertion_reader): the first pass had ONE failure,
  `an_alert_readable_before_our_pre_close_look_is_refused_in_tls_13_and_12` returned Ok: root cause, a single `peek` proves
  nothing under load (loopback delivery can lag the sender's `write`, and TLS 1.3 tickets could satisfy the peek). Fixed:
  `no_tickets` fake option + `alert_is_queued` polls until the whole alert record (24 / 31 bytes) or EOF is visible; then
  25x under 16 busy loops for `tls_sink_23_4`: 0 failures. Gates: env contract 167, dependency closure, composition root 11/20,
  kernel baseline 28,339 = pinned, `check-enterprise-identity` 10 legs, `cargo deny` ok, fmt clean, five `cargo check -p maos-bin` shapes.
  kloc: maos-siem 1,512, maos-audit 9,784, maos-bin 47,256, kernel 22,154; `over_budget []`.
- Stripped release 39,104,720 B (limit 39,069,022: over by 35,698; round 4 39,102,032). Open operator decision.
- Deviations: R5-D0 `agent://Rev234G5` not read. R5-D1 `cargo fmt --all` in write mode. R5-D2 no plants for R5-5 (fixture-defect
  path unreachable) and the R5-8b write-phase test; R5-8d models additive roots as an extra anchor (the literal webpki plant is
  impossible offline). R5-D3 a cheap idle tick is no longer free after any failed tick (every tick takes the full path until a full
  check passes). R5-D4 a file found existing now syncs its parent once per process start (the round-4 "owes none" rule is
  superseded). R5-D5 new doc-hidden seams: `tls_sink::root_store`, `TlsSink::with_connector`, `Options::no_tickets`. R5-D6 the
  1.2 readiness witness stays weaker than 1.3 (EOF accepted). R5-D7 clippy: remaining warnings are pre-existing (`maos-siem/src/lib.rs:143`).
