# Story 23-4 — review round 6 (progress notes; newest last)

`agent://Rev234G6` was read in full (including the stall finding). D-23-4-16, D-23-4-17 and the size limit stay the operator's.
STOP RULE honoured: nothing beyond the list.

## R6-1 DONE — no WAL read mark while the sources are opened (`maos-audit/src/segment.rs`, `insertion.rs`)
- The interleaving: the live read transaction used to be begun first, then the registry read and every sealed segment opened and
  read under it; a rotation's `wal_checkpoint(TRUNCATE)` under the audit writer mutex waited for that mark and audit inserts queued
  behind it (default 5 s busy timeout). Now `open_log_sources_live_with`: (1) the segment list is read by a short autocommit
  connection and the sealed segments opened with NO transaction held, (2) only then `BEGIN` and the registry re-read as the first
  statement of the (short) read transaction; an unchanged list means the opened segments and the live rows are one consistent view,
  a changed list (a rotation sealed more rows meanwhile) drops everything and retries (5 attempts, then a typed
  `siem_source_unavailable` tick error, "retry later": nothing blocks the writer). The batch read itself is metadata plus at most
  one batch of rows inside the transaction, which ends before any other I/O (round 3/4).
- Deterministic test `no_read_mark_is_held_while_the_live_sources_are_opened`: a doc-hidden `LogView::open_live_paused` callback runs at
  the exact point (segments open, no transaction yet); the test runs `PRAGMA wal_checkpoint(TRUNCATE)` from inside it and requires
  busy = 0, then reads the segment and live rows. Plant (a transaction begun before the segment opens, held through the pause) FAILED.
- NOT covered (R6-D2): the retry branch (a rotation landing between the two registry reads) has no test; the 5-second SQLite busy timeout of
  rotation itself is not ours to shorten.

## R6-9 (part) DONE — the typed gap/reset line survives a failing cursor persist (`export.rs`)
- The note is queued BEFORE `CursorStore::save` (gap) and before `move_aside` (reset), so a rename that succeeded followed by a failing
  directory sync still reports it (the next tick would load the moved cursor and never emit it). Test: the existing blocked-cursor test now also
  requires `siem_gap` in the failed tick's notes. No plant. A repeated failing persist repeats the line each tick (the consumer prints one line per
  distinct reason, not per note).

## R6-3, R6-4, R6-5 DONE — file sink (`export.rs`)
- `resolve_entry` follows EVERY symlink level (at most 16, then `siem_file_symlink_loop`) on every open; the resolved final entry is created with
  `O_EXCL` when missing, and the directory synced (first use, creation, replacement) is the one that CONTAINS that entry. After the append (and the
  truncation check) `still_the_entry` requires `nlink > 0` and the resolved path to name the same dev+inode, else
  `siem_file_replaced_during_append` (cursor kept, the next tick reopens and re-sends: at-least-once). Retention requirement: the old file must stay until the
  sink has reopened (documented in 23-4.j / runbook / ADR).
- Tests: `the_directory_behind_a_symlink_is_the_one_synced` (link in A to an existing file in B: the recorded sync target's parent is B),
  `a_chain_of_symlinks_is_followed_to_its_target_and_a_loop_is_a_typed_error` (chain, loop, directory target),
  `a_batch_appended_to_a_retired_file_is_not_acknowledged` (injectable writer renames and unlinks between write and sync). Plants: check disabled FAILED;
  two-level resolution FAILED the chain test; configured path used as the sync target FAILED three tests (R6-3, chain, dangling).

## R6-2 DONE — `MAOS_SIEM_FILE` that aliases cursor or audit state is a boot refusal (`siem_file_aliases_cursor`, exit 78)
- `maos_siem::export::aliases_audit_state(sink, audit_db)`: the sink is resolved through every symlink level, its directory canonicalised (deepest existing
  ancestor, so a not-yet-created `segments/...` path is covered); refused: `siem-cursor.json` and any `siem-cursor.json*` name (the `.tmp` and
  `.corrupt-*` forms) in the audit directory, the audit database file and `-wal`/`-shm`, anything under `<audit dir>/segments`, and (unix) any file with the
  same device+inode as one of the protected existing files (hard links, symlinks). `SiemBootRefusal::FileAliasesCursor` is checked in `main.rs` right after
  `configure_siem_from_env`, before the consumer or any append; the message names variables, never a path/value. Not a new env var (167 stays).
- Tests: `a_file_sink_that_aliases_the_cursor_or_the_audit_state_is_recognised` (direct, `.tmp`, `.corrupt-123`, db, `-wal`, `-shm`, a segment path, a two-level
  symlink, a hard link, and two clean paths) and boot test `boot_refuses_a_file_sink_that_aliases_the_cursor` (exit 78 + code, no path in the message, for direct,
  `.tmp`, symlink and the database; a distinct file boots). Plant (`refuse_file_aliases` always `Ok`) FAILED the boot test.
- Limits: the check is at boot (a link created later is not seen); a path that cannot be resolved here is left to the sink's own typed error.

## R6-6 DONE — the verifier's trust store is pinned (`tls_sink.rs`)
- `client_config` now builds the `WebPkiServerVerifier` itself from ONE immutable `Arc<RootCertStore>` and installs it with
  `dangerous().with_custom_certificate_verifier` (the same verifier `with_root_certificates` builds). Doc-hidden `client_config_pinned` returns that same `Arc`; the test
  counts the anchors in it for one and for two anchors. A root added between `root_store` and the verifier is therefore counted. Plant (an extra anchor
  pushed after `root_store`) FAILED the test. `root_store()` stays (doc-hidden) for the helper's own use.
- Limit: the test pins the store handed to the verifier builder; it cannot read the store back out of rustls' private `ClientConfig`.

## R6-7 DONE — production connect deadline and the write phase
- `the_production_connect_deadline_ends_a_connect_that_never_completes`: a loopback listener whose accept queue is filled by std clients until three consecutive
  500 ms probes cannot connect (the kernel drops the SYN), then the PRODUCTION sink (no connector) with a 500 ms connect budget must return `siem_sink_timeout`
  in under 30 s (an unbounded connect holds for the kernel's SYN retries). Linux only; otherwise or if the queue cannot be saturated it prints the reason and
  returns. Plant (`TcpStream::connect` instead of `connect_timeout`) FAILED after 133.6 s ("bounded by the connect budget: 133.586s"; the OS error is a timeout
  too, which is why the elapsed bound is the assertion).
- Write phase: the payload is now built BEFORE the write deadline starts (production change in `TlsSink::deliver`), the budget is 2 s and the test asserts a new
  `CloseStage::WriteBlocked` observation (raised by `flush` when bytes are still owed to the socket and the bound ran out or the write timed out) in addition to
  `siem_sink_timeout`, the fake's `Event::Held` (reads stopped) and the untouched cursor. No plant for the write observation (disclosed).
- Root cause found while writing it: a blocked TLS write returns `Ok(partial)` before its bound, so the timeout surfaces at the next `arm` rather than from the write itself; the first
  version of the observation looked only at the write error and failed; fixed by observing at both places.

## R6-8 DONE — fail-closed gates and the alert helper (test support)
- `fake_siem.rs`: the `CloseWithoutReading` alert and the normal acknowledgement both use `Gate::wait_open()`; a gate that stays closed past its bound ends the connection
  WITHOUT an alert or ack and sends the new `Event::GateTimedOut` (the exhaustive match in the 13.5a consumer test got an arm that fails). The unused `Gate::wait` was removed.
- `alert_is_queued(tcp, len)`: the length is the negotiated version's record (24 bytes TLS 1.3, 31 TLS 1.2) and EOF (`Ok(0)`) is NOT proof. For that to hold in TLS 1.2 the refusal test
  now holds the alert until the hook (after the handshake completes on both sides) releases it, so the client's handshake reads cannot swallow it.
- No plants for R6-8 (a plant would need a 30 s gate timeout to be observed; disclosed).

## R6-9 DONE — documentation
- ADR-051: "What acknowledged means" restated to the D-23-4-16 contract (a close does not prove the collector read the batch); the client key is read through the configured store, not
  the environment; cursor bullet describes the round-6 open order; file-sink bullet and refusal list updated (all symlink levels, bound 16, loop, directory target, replaced-during-append, retention covers open
  handles, `siem_file_aliases_cursor`). RELEASE-HOLDS 23-4.j and the runbook (codes table, boot refusals, rotation bullet) carry the same, and 23-4.j states AC4's exactly-once gap journaling is limited to the live dedupe window.
- Typed-note ordering after a post-rename sync error: see R6-9 (part) above.

## Totals (observed)
- `maos-siem --features tls-sink`: `tls_sink_23_4` 32, `cursor_file_sink_23_4` 27, lib 8, others 2+3+2; fault twins 2; `maos-audit insertion_reader_23_4` 14;
  `maos-bin`: `siem_network_23_4` 27, `enterprise_identity_wiring` 4, `enterprise_daemon_seam_13_5a` 24 (+1+1+1 unit filters), `online_adapters_23_0` 3, `maos_uninstall_16_4` 27 per the
  suite listing; `--bin maos story_13_5a` 4; `--lib available_arm` 6. All green.
- Gates: `check-env-contract` PASS (167, unchanged), `check-dependency-closure` PASS, `check-composition-root-completeness` PASS (11/20), `check-kernel-baseline` PASS
  (28,339 pinned = measured; kernel untouched), `check-enterprise-identity` PASS, `kloc-check --json` no over_budget (`maos-siem` 1,643, `maos-audit` 9,826, `maos-bin` 47,280,
  kernel 22,154), `cargo deny check` ok, `cargo fmt --all -- --check` ok, five `cargo check -p maos-bin` shapes finish clean, clippy: only the pre-existing `maos-siem/src/lib.rs:142-143`
  (the new `manual_contains` in `aliases_audit_state` was fixed).
- 10 rounds under 16 busy loops of: all of `maos-siem --features tls-sink`, `insertion_reader_23_4`, `siem_network_23_4`, `story_13_5a`: 0 failures.
- Stripped release `maos`: 39,146,960 B (round 5: 39,104,720 B; +42,240). The limit is 39,069,022 B: over by 77,938 B. The limit is the operator's; not touched.
- Plants applied one at a time and removed; `grep -rn E234-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` prints nothing.

## Numbered deviations
1. R6-1: the registry-changed retry branch (up to 5 attempts) has no test.
2. R6-9 note-ordering fix has no plant.
3. R6-7: no plant for the write-blocked observation; the connect plant takes ~134 s to fail.
4. R6-8: no plants (fixture-only changes).
5. R6-6: `client_config` now builds the verifier explicitly (`dangerous().with_custom_certificate_verifier(WebPkiServerVerifier ...)`) instead of `with_root_certificates`: same verifier, new
   public (doc-hidden) `client_config_pinned`.
6. R6-7: production change in `TlsSink::deliver` (payload built before the write deadline) and a new doc-hidden `CloseStage::WriteBlocked`; the production-connect test self-skips (with a printed reason) off Linux or if the accept queue cannot be saturated.
7. R6-2: the alias check is at boot only.
8. `cargo fmt --all` was run in write mode; earlier-round breaches (python edit, heredocs, round 0) stand. The shell was used to read files with `sed -n` and `grep` only.
9. D-23-4-16, D-23-4-17 and the binary-size limit remain the operator's; the build is 77,938 B over the limit.
