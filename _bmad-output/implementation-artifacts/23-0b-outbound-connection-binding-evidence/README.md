# Story 23-0b evidence (agent Dev230b, host: local, branch e23-0b-outbound-binding-2026-11-25, base main 6534038f)

Env for every cargo command: `unset CI GITHUB_ACTIONS MAOS_17_2_CGROUP_MODE; CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/mnt/build/cargo-maos-e230b MAOS_FIXTURE_DIR=$CARGO_TARGET_DIR/maos-fixtures`.

Design as ruled 2026-11-25: D-23-0b-01 A (the vetted set travels through `IoSubsystemPort`), 02 a (`bind(&VettedUrl) -> Box<dyn IoSubsystemPort>`),
03 a (a port that cannot bind refuses host names, `outbound_unbound_refused`), 04 a (a new agent per bound request, no idle pool), 05 a (SNI +
untrusted-certificate proof, no positive https round trip), 06 a (no production `ResolveHost`), 07 a (23-0.a kept and marked closed, new row 23-0b.a).
Also in this change (independent review finding E23-2 and the 3fff::/20 ruling): the no-follow contract and `3fff::/20` refused.

## What changed

| File | Change |
|---|---|
| `crates/maos-domain/src/ports/io_subsystem.rs` | `bind` (default fails closed) and `never_follows_redirects` (default `false`) |
| `crates/maos-domain/src/outbound.rs` | `OutboundRefusal::Unbound`; `VettedUrl::{is_name,host_str,port,socket_addrs}`; `PolicedIo::new` returns `Result` and refuses a port that does not declare no-follow; names go through `bind`, literals never do; `classify_v6` refuses 3fff::/20; module doc |
| `crates/maos-kernel-core/src/io/mod.rs` | one `agent_builder()` for `new()` and `bind()`; `bind` (pinned resolver, `max_idle_connections(0)`); `never_follows_redirects() == true` |
| `crates/maos-egress/src/proxy.rs` | the resolved-answer check is a private `vet_resolved` (behaviour unchanged) so a unit test can pin 3fff::/20 without DNS |
| tests | `crates/maos-kernel-core/tests/io_binding_23_0b.rs` (+ fixture `tests/fixtures/io_binding_23_0b/{cert,key}.der`, a throwaway self-signed end-entity certificate for `rebind.maos.test`: CA:FALSE, SAN DNS, EKU serverAuth, EC P-256); new cases in `crates/maos-domain/tests/outbound_policy_23_0.rs`; `vet_resolved` unit test in `maos-egress` |
| records | `RELEASE-HOLDS.md` rows 23-0.a (closed for host names), 23-0.b (3fff removed), new 23-0b.a, 23-8.c successor; ADR-051 and ADR-061 amendments and the ADR index; 23-0 evidence README P3e |

## Per-AC result

| AC | Result |
|---|---|
| AC1(a) shapes | `cargo check --locked -p maos-bin --lib --bins --tests`: default exit 0; `--features network` exit 0; `--features wasm-host` exit 0; `--no-default-features --features network` exit 0; `--no-default-features` exit 101, the same red `--tests` class the 23-0 README records at its base (`maos_bin::delegation`, `maos_a2a_tcp` unresolved), not touched by this change. `cargo check -p maos-kernel-core --all-targets` with and without `--features io_call_journal`: exit 0. |
| AC1(b) | `outbound_policy_23_0` 26 passed; `io_binding_23_0b` 7 passed; `io_exchange_18_8` 6 passed; `io_redaction_and_status_20_4` 7 passed; `cargo test -p maos-egress` all green (including the new `vet_resolved` test) |
| AC1(c) | `check-composition-root-completeness` pass; `check-env-contract` pass, 162 registered, no new name; `check-dependency-closure` pass; `check-pub-field-constructors` pass; `check-service-boundary` ok; kernel gates: see Numbers |
| AC2 | `a_changed_second_answer_does_not_move_the_connection`: resolver called once, listener A one connection, listener B (127.0.0.2:P, Linux only) none |
| AC3 | `a_mixed_answer_is_refused_before_any_connection` (`outbound_link_local_refused`, A zero); `a_bound_port_refuses_a_different_host_or_port` (other host, other port, an IP literal: all refused by the pinned resolver, A zero) |
| AC4 | `the_tls_name_is_the_original_host` (SNI equals the name); `an_untrusted_certificate_for_the_name_is_refused` (self-signed end-entity certificate: a `Transport` error whose text names `UnknownIssuer`, SNI equals the name) |
| AC5 | `a_literal_ip_never_calls_bind_or_the_resolver` (kernel, resolver 0 calls, A reached, no SNI); `a_literal_ip_never_binds_and_a_name_binds_once_and_never_goes_raw` (domain, counting port) |
| AC6 | `a_redirect_on_the_bound_path_is_refused_with_no_location_and_no_body` (301/302/307/308, bound port sees exactly one call); the bound port comes from the same `agent_builder()` as `new()` |
| AC7 | `a_port_that_cannot_bind_refuses_host_names_and_is_never_called` |
| E23-2 | `a_transport_that_follows_redirects_is_refused_before_any_request` (a fake transport that would follow to `10.0.0.5` is refused at construction and records no connection); `a_bound_port_that_does_not_declare_no_follow_is_refused_before_any_request` |
| 3fff::/20 | `the_classifier_refuses_the_rfc_9637_documentation_block` (3fff::, 3fff:fff:ffff:ffff:ffff:ffff:ffff:ffff, 3fff:800::1 refused; 3ffe:ffff:: public and 4000:: reserved as before; 3fff:1000:: public); `a_name_answering_in_the_rfc_9637_block_is_refused`; egress `a_resolved_answer_in_the_rfc_9637_block_is_refused`; the `public_ip` equivalence test is updated deliberately and names the 3fff::/20 refusal as its only intended difference (the IPv6 sweep now also visits second segments 0x0fff and 0x1000) |
| AC8 | `MAOS_AUDIT_KEY` (random 64-hex, mode 0600, deleted after) exported: all of the above plus `maos-secrets --features http-kms --test external_kms_23_8` (23 passed) and `maos-bin --test external_kms_wiring_23_8` (20 passed) green. No `Cargo.toml` or lock file changed. `cargo deny check`: advisories, bans, licenses, sources ok. New tests 10 runs in a row under 16 busy loops: 10 of 10 passed. |

## Proven reds

Every plant carried the token `E230B-PLANT`, ran once, and was removed. `grep -rn E230B-PLANT crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` returned nothing after the last plant.
Files under `proven-red/` hold the failing lines of that run.

| File | Plant | Tests that fail |
|---|---|---|
| `AC2/unbound-agent.txt` | kernel `bind` returns the unpinned adapter | `a_changed_second_answer_does_not_move_the_connection`, `the_tls_name_is_the_original_host`, `an_untrusted_certificate_for_the_name_is_refused` (the name does not resolve, A sees nothing), `a_bound_port_refuses_a_different_host_or_port` |
| `AC3/netloc-check-removed.txt` | the pinned resolver answers every netloc | `a_bound_port_refuses_a_different_host_or_port` only |
| `AC4/url-host-rewritten-to-ip.txt` | pin by rewriting the URL host to the IP (domain and kernel) | `the_tls_name_is_the_original_host` and `an_untrusted_certificate_for_the_name_is_refused` (SNI absent), `a_bound_port_refuses_a_different_host_or_port` |
| `AC5/literals-routed-through-bind.txt` | literal IPs go through `bind` | `a_literal_ip_never_binds_and_a_name_binds_once_and_never_goes_raw` and four older `PolicedIo` tests |
| `AC6/bound-3xx-passed-through.txt` | the 3xx refusal in `http_post_exchange` disabled | `a_redirect_on_the_bound_path_is_refused_with_no_location_and_no_body` and the two older redirect tests |
| `AC7/unbound-falls-back-to-raw-string.txt` | a failing `bind` falls back to the raw-string path | `a_port_that_cannot_bind_refuses_host_names_and_is_never_called` only |
| `E23-2/construction-accepts-a-following-port.txt` | `PolicedIo::new` does not check the declaration | `a_transport_that_follows_redirects_is_refused_before_any_request` only |
| `3fff/block-admitted-again.txt` | the 3fff::/20 rule removed | the boundary test, the name test, the equivalence test, and the egress `vet_resolved` test |

Note on AC2's plant: the `.test` name cannot resolve, so an unbound agent fails by finding no address at all. That is the intended witness: the only way a connection reaches a fixture is through the vetted set.

## Numbers

| Gate | Value |
|---|---|
| kernel `src_lines` (physical, `check-kernel-baseline`) | pin 28,339, measured 28,372 (+33), 89 files, `io/mod.rs` the only changed file; measured set hash `3b23e28a5347f7122fe81bd7c4113b3ffb99324f8d16a9672e0cfef365307561`, `io/mod.rs` digest `9d8ef63b8e0fdd55fd0248e57907eccb38beca9e8d02ec8aaaffd3844b82ce45` |
| kernel tokei (`kloc-check`) | 22,173 (main 22,154, +19; forecast +22..+40); alarm 23,000, row 24,000: under both; `over_budget` empty |
| other kloc rows | maos-domain 11,229 (was 11,174, +55), maos-egress 1,775, maos-bin 46,402 (main figure not re-measured here) |
| stripped release `maos` | 38,911,056 B, limit 39,069,022 |

Kernel pin: re-pinned in the orchestrator's commit (`scripts/repin.py --apply --story 23-0b`): `src_lines` 28,372, set hash `3b23e28a...`, `io/mod.rs` digest `9d8ef63b...` as listed above.

## Not run / hosted only

- `aarch64-apple-darwin` check: the target is installed but `ring` needs a C cross compiler (`failed to run custom build command for ring v0.17.14`); not run. The new test uses no Linux-only API outside a `cfg!(target_os = "linux")` branch.
- Hosted legs (`workspace-test-suite`, `fuzz-build`, `cargo-deny` job, `macos-check`, `onb-nfr2-timing`): no hosted run id; local pre-verification only.
- No positive https round trip exists (D-23-0b-05 a); RELEASE-HOLDS 23-0b.a says so.

## Review round (GPT correctness, Claude security) — fixes

| Finding | Result |
|---|---|
| P2 AC6, failing bound GET | `a_failing_bound_get_reaches_the_caller_as_the_same_error_never_as_bytes`: the bound fixture's GET fails with a configured error (`status 302`, `status 500`, a decode error) through a host-name URL; the exact error reaches the caller, one bind, exactly one bound call, no raw-string call. Plant `proven-red/AC6/bound-get-error-swallowed.txt` (error mapped to empty bytes): that test and the older GET test fail. |
| P3 wire-level redirect | `a_bound_adapter_returns_a_redirect_and_never_follows_it_on_the_wire`: a real-socket listener answers every request with 302; `Location` points at listener B (127.0.0.2, Linux-only branch) or back at the vetted netloc; GET gives `Transport("... status 302")`, POST gives status 302 with an empty body and no header; A sees exactly four requests and none for `/landing`; B sees no connection. Plant `proven-red/AC6/bound-adapter-follows-redirects-on-the-wire.txt` (`redirects(5)` in `agent_builder`): red. The `to-b` case alone would also be refused by the pinned resolver, so the `to-self` case is what a followed redirect cannot hide behind. |
| P3 fixture certificate | Confirmed: the first `cert.der` had `basicConstraints critical, CA:TRUE` (the README's "no CA" was wrong). Regenerated with openssl as a throwaway end-entity certificate: EC P-256, `CA:FALSE`, SAN `DNS:rebind.maos.test`, EKU serverAuth, keyUsage digitalSignature, 100-year validity, self-signed; `key.der` regenerated with it. `an_untrusted_certificate_for_the_name_is_refused` now asserts the `UnknownIssuer` class (the test passes on the new certificate). Plant `proven-red/AC4/verifier-accepts-any-certificate.txt` (a rustls verifier that accepts any certificate on the kernel agent): the assertion fails. `.gitattributes` gains `*.der binary`. The key is a test fixture only. |
| P3 records | The kernel pin is the committed re-pin: `src_lines` 28,372, set hash `3b23e28a5347f7122fe81bd7c4113b3ffb99324f8d16a9672e0cfef365307561`, `io/mod.rs` digest `9d8ef63b8e0fdd55fd0248e57907eccb38beca9e8d02ec8aaaffd3844b82ce45` (the "not re-pinned / red until repin" text above described the state before the orchestrator's commit). The ADR-051 amendment says `PolicedIo` calls `OutboundPolicy::vet_url` (ADR-061 never named the method). RELEASE-HOLDS 23-0.a names the DNS64 residual and the unbound transports; 23-0.b states the egress behaviour change. |

What the plants prove, exactly:

- `AC2/unbound-agent.txt` proves that losing the pinning is detected (the `.test` name cannot resolve, so the unbound agent finds no address). It does NOT show a live rebind reaching listener B; the second-answer test shows the connection goes to A and B stays empty.
- `AC6/bound-3xx-passed-through.txt` proves response-level refusal at the exchange (`PolicedIo` turns a 3xx into `outbound_redirect_refused`). It is not a wire-level no-follow proof; the wire-level proof is the real-socket test and its plant above.

Egress behaviour change, precisely (also in RELEASE-HOLDS 23-0.b): a granted egress host whose resolved answers include ANY address in 3fff::/20 is refused entirely (mixed answers fail closed); an address in that block is refused as a literal in the shared policy; labs that use RFC 9637 space as public space stop working. Nothing refused before is admitted now; 64:ff9b::/96 was already refused.

## Open for 23-2 (recorded, not implemented here)

- The production `ResolveHost` must cap the number of answers it returns and run under a deadline (egress's `resolve_bounded` pattern); `PolicedIo` vets every answer and passes the whole set to the pinned resolver, so an unbounded answer list is unbounded work and an unbounded connect sequence.
- Error text on the bound path still contains the full URL, including userinfo and query (`io/mod.rs` `"ureq GET {url}: {e}"` and the POST equivalent, pre-existing): a caller must not log such an error for a URL that carries a credential.
