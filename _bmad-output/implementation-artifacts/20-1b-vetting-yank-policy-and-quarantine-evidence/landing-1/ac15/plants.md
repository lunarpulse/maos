# AC15 observed witnesses and restored controls

Baseline: Main handed off clean `c656a6e7`. No kernel source was edited.
Every actual command/exit is in `commands.json`; stdout/stderr retain assertions.
All exercised mutations were copied back, compared byte-for-byte and touched;
all copy-back/cmp/touch exits are0. Saved sources live only in owned temporary
custody until end cleanup; they are not shipped runtime fallbacks.

`T` below is `crates/maos-registry/tests/registry_tls_20_1b.rs`.
Locations are final formatted source locations. Names inside the TLS module
execute as `tls::<name>`. The focused JUnit records26 passing real-wire cases;
the separate final positive adds actual authenticated TLS publication.

| Boundary | Named witness:line | Actual before / mutation observation | Observed corrected evidence |
|---|---|---|---|
| Install opt-in | T `http_install_requires_explicit_development_opt_in_ac15`:12 | Clean-source baseline101: implicit HTTP constructor accepted; client-mode plant100 | Focused pass |
| Server opt-in | T `server_requires_tls_or_explicit_loopback_development_ac15`:24 | Clean-source baseline101: implicit HTTP serve accepted; server-mode plant100 | Focused pass |
| Root authority | T `hostile_registry_wrong_configured_root_is_refused_ac15`:257; CLI counterpart:395 | Envelope-disabled plant101 accepts wrong-root TLS list | Focused pass; no CLI persistence |
| URI binding/root bootstrap | T `configured_root_is_bound_to_canonical_uri_before_contact_ac15`:267; `https_requires_configured_root_not_response_supplied_key_ac15`:886; CLI URI counterpart:413 | Configured-root/URI constructor plant100: both independent cases fail | Focused pass; canonical/alias vectors include non-loopback IP |
| Certificate hostname | T `interception_wrong_host_certificate_is_refused_ac15`:294 | Real TLS accept-any-verifier plant101 accepts wrong-host peer | Focused pass |
| Certificate lifetime | T `interception_expired_certificate_is_refused_ac15`:304 | Same real TLS plant101 accepts expired leaf | Focused pass |
| CA trust | T `interception_untrusted_ca_is_refused_ac15`:314 | Same real TLS plant101 accepts stranger CA | Focused pass |
| Missing signed envelope | T `envelope_missing_signature_is_refused_ac15`:743 | Envelope-disabled plant101 accepts unsigned view | Focused pass |
| URI/nonce binding | T `envelope_wrong_uri_is_refused_ac15`:747; `envelope_wrong_nonce_is_refused_ac15`:751 | Envelope-disabled plant101: each independent case fails | Focused pass |
| Complete authenticated head | T `envelope_missing_head_is_refused_ac15`:755; `envelope_incomplete_head_is_refused_ac15`:759 | Envelope-disabled plant101, plus independent head-only plant100 | Focused pass |
| Stripped withdrawal | T `envelope_stripped_yank_entry_is_refused_ac15`:763; `cli_omitted_yank_or_unauthenticated_head_refuses_before_persistence_ac15`:858 | Envelope-disabled API plant101; genuine rebuilt CLI plant101 actually installs the stripped-yank package | Focused pass; no registry persistence for all3 CLI variants |
| Redirect downgrade | T `https_redirect_never_downgrades_to_http_ac15`:767 | Real plaintext POST to the trap causes plant100 | Focused pass; trap sees no connection |
| Failure downgrade | T `handshake_failure_never_attempts_plaintext_ac15`:894 | Faithful first-write fallback plant101 creates a second/plaintext connection | Focused pass; no second connection |
| Both-end development | T `server_rejects_client_without_dev_handshake_ac15`:802; `loopback_development_requires_both_sides_and_keeps_yank_signatures_ac15`:787 | Handshake guard plant100 returns200 instead of403 | Focused pass; explicit both-end positive and unsigned deprecate refusal |
| Private key custody | T `private_key_custody_refuses_group_or_world_access_ac15`:126; `private_key_custody_refuses_seed_inside_package_store_ac15`:138 | Actual pre-enforcement101 and custody plant101: both cases fail independently | Focused pass; Unix opened-FD owner/private-mode checks, production builder outside-store checks |
| Genuine TLS install / yank | T `cli_tls_install_authenticates_before_persistence_ac15`:364; `cli_tls_yanked_version_is_refused_before_persistence_ac15`:817 | Pre-cutover CLI positive fails101 because TLS is not implemented; omission plant above witnesses persistence when protection is removed | Focused positive uses actual inventory-owned generation, not a guessed version directory; genuine signed yank refuses before any registry directory |
| TLS publication/metadata | T `configured_root_and_ca_authenticate_real_registry_ac15`:209 | Feature-before positive described above; no invented independent publication-before claim | Final actual TLS publish/manifest/artifact/yank/deprecate positive exit0 |
| Generated default-port URI | `crates/maos-registry/src/server.rs` `development_default_ports_use_the_same_canonical_uri_as_clients`:771 | Faithful old raw-address formatting plant101 emits `:80`, unlike canonical clients | Final unit exit0 for IPv4 and IPv6; inline test is charged in KLOC |

## Honest exclusions and incomplete probes

- The first test attempt did not compile because the fixture supplied `&Path`
  where `at_path` requires `PathBuf`; it is not an observed behavioral red.
- The pre-cutover CLI binary has its SHA256/mtime in `before-cli-binary.json`.
  Its positive HTTPS refusal really reported unimplemented TLS. The two original
  negative CLI probes used HTTP against the new explicit-handshake peer and failed
  their authentication-phase oracle; they do **not** prove old HTTPS accepted
  hostile keys/certificates. Final CLI root/URI cases now use TLS, and the genuine
  independent negative plants above are the security controls.
- An initial multi-control nextest command was fail-fast: only the client opt-in
  case ran. The four previously unrun controls were then executed with
  `--no-fail-fast`; all four failed. They were not silently counted as exercised.
- The first handshake fallback plant targeted response reads and passed because
  this TLS failure occurs during the first write. It is not credited. The
  faithful first-write fallback plant independently fails the connection trap.
- One positive inventory assertion guessed a plain version directory and failed
  after a successful install. It was corrected to use the actual installed index
  and inventory-owned manifest path; negative oracles now require no registry
  directory at all.
- Compile-only ownership/error-conversion mistakes are preserved in their logs,
  not relabelled as acceptance reds. No timing failure was retried to green.

## Limits

Unsigned literal-loopback development intentionally has no peer authentication,
confidentiality, or omission/freshness guarantee. With a configured URI-bound root,
HTTP responses/heads can be authenticated, but HTTP is never confidential.
A signing authority lying or signing a rollback to a fresh client is not defeated
by authenticated transport; key compromise is not claimed solved. Hosted TLS/dev
and Apple-toolchain recipe legs are authored only, unverified. Final KQ1 effects,
operator vetter-key lifecycle and broader identity-state limits remain separate.

## Hosted regression 42418118608: cohort audit-owner release

- Prescribed original `cargo test -p maos-bin --test root_shutdown_unload_16_3 -- --test-threads=1 cohort` exited0 with three cases in0.00s: no local signed OCI substrate. A separate `--exact ... --nocapture` disposition recorded the explicit `SKIP: T3 cohort root shutdown Worker` message. This is **not** a locally observed live-Worker red.
- New non-T3 real cohort-daemon witness `cohort_sigterm_without_worker_joins_local_yank_timer_and_drains` observed the production `audit writer cohort drain timed out after 10s` before the fix (101, test10.34s), with registry-fetch disabled and normal300s cadence.
- The AST release guard independently observed red101 against that same missing join. It now covers seven sender-release sites and eleven writer drains, including the shared macro and lexical branch isolation.
- Removing the production cohort join produced the same real daemon timeout (101, test10.31s) and AST red101. Copy-back, `cmp` and `touch` each exited0; restored full binary15/15 exited0 (0.64s test sum), with substrate-dependent cases honestly skipped locally.
- `declared_capable_cohort_worker_prerequisite_never_green_skips` launches the actual v5 case as a child with CI declared and its runtime absent. It asserts the explicit nonzero `declared-capable T3` refusal, proving a capable hosted runner cannot green-skip the missing signed substrate.
- No poll-interval-zero harness bypass and no per-binary budget exception were added. The hosted live-Worker correction remains unverified here; the branch recipe provisions the existing signed T3 action before its107-contract selection.
