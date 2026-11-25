# Proven red — Story 23-4 (marker `E234-PLANT`)

Each plant was applied ALONE with the edit tool, the named test target run, the result recorded here,
then the plant removed. After the last removal `grep -rn E234-PLANT crates .github docs RELEASE-HOLDS.md
Cargo.lock xtask` printed nothing (exit 1) and `cargo fmt --all -- --check` was clean.

| # | AC | Plant (file) | Test target | Observed |
|---|---|---|---|---|
| P1 | AC1c | `maos-siem/src/tls_sink.rs`: the `anchors == 0` refusal in `client_config` disabled (`&& cfg!(any())`), so an empty CA bundle builds a client | `cargo test -p maos-siem --features tls-sink --test tls_sink_23_4` | 15 passed, 1 FAILED: `a_server_certificate_from_another_ca_is_refused_and_the_bundle_is_the_only_root` (panic at `tls_sink_23_4.rs:266`, the `client_config(b"", None)` refusal) |
| P2 | AC1e | `maos-siem/src/tls_sink.rs`: `TcpStream::connect((host, port))` instead of the vetted `SocketAddr` | same | FAILED incl. `the_outbound_policy_gates_the_host_class_and_the_connection_binds_to_what_it_vetted`: `first connection: SinkFailure { code: "siem_sink_connect_failed" }` (the real name `siem.test` does not resolve; the vetted stub address was never used) |
| P3 | AC2 (i) | `maos-siem/src/export.rs`: cursor saved BEFORE `sink.deliver` | same | 13 passed, 3 FAILED: `a_collector_that_closes_without_close_notify_does_not_move_the_cursor`, `every_wait_is_bounded_and_a_held_collector_never_moves_the_cursor`, `a_failure_between_the_ack_and_the_cursor_write_resends_exactly_that_batch` |
| P4 | AC2 / AC3 late insert | `maos-audit/src/insertion.rs`: `AND timestamp_ns >= 1000` in the read (a timestamp watermark) | same | 14 passed, 2 FAILED: `a_row_inserted_late_with_an_older_stamp_is_delivered`, `a_backlog_goes_out_in_bounded_batches_within_one_tick` |
| P5 | AC5 | `maos-bin/src/enterprise_identity.rs`: the exporter keeps `BEGIN IMMEDIATE` on the live file for the whole tick | `cargo test -p maos-bin --test siem_network_23_4 audit_writes_complete` | FAILED: `audit_writes_complete_while_the_collector_holds_the_handshake` — kernel panic in the TL writer `database is locked` (a burst insert waited on the exporter) |

A first attempt applied P1–P4 together: every test went red (16 of 16), which attributes nothing; the
plants above were then run one at a time.

## Substitution (numbered deviation 7 in the README)

The story's AC1c plant is "root store with webpki roots added". `webpki-roots` is not a dependency of
`maos-siem`, and an offline test cannot present a publicly trusted certificate, so that plant cannot be
red. P1 (an empty bundle accepted) is the observable half of "an empty bundle refuses every server"; the
other half (a foreign CA is refused) is asserted by the same test and passes on the shipped code.
