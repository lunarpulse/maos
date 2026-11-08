# Acceptance audit — 20-1b AC15/F5 + cohort teardown (commit f9921251, parent c656a6e7)

Auditor: `Review201bAC15.AcceptanceAuditor201b`. Read-only. HEAD sources read, not only the diff. Nothing was executed.
Aliases: T = `crates/maos-registry/tests/registry_tls_20_1b.rs`; S = `crates/maos-bin/tests/root_shutdown_unload_16_3.rs`; EV = `.../landing-1/ac15/`.

## Verdict

AC15's core mechanics are implemented and mostly witnessed: rustls client and server, URI-bound configured root, signed envelope with head, both-end loopback dev mode, and the cohort join fix. Four items do not hold as claimed:

- **AA-01 (High).** A hosted CI job (`epic-20-exit`) runs the registry over plain HTTP with no opt-in. It will go red on first hosted run.
- **AA-02 (Med).** The "rustls must not leak into no-default-features" evidence contradicts the commit's own dependency tree.
- **AA-03 (Med).** Contiguity, request-binding and head-vs-list checks have no failing-before witness. The "stripped entry" witnesses refuse for a different reason than the table says.
- **AA-04 to AA-06.** Witness gaps for AC15.2 (refuse before persistence), key custody and loopback vectors.

## Verified OK (no finding)

- **Witness line numbers.** Every T name:line in `ac-witness-table.md`, `plants.md` and the story AC15 table matches the `fn` lines at HEAD (12, 24, 126, 138, 209, 257, 267, 294, 304, 314, 364, 395, 413, 743–767, 787, 802, 817, 858, 886, 894). The `server.rs:771` unit test exists. `grep -c '#[test]'` in T gives 26, and `tls-junit.xml` has 26 testcases.
- **`main.rs` coordinates.** All 30 coordinates in `cohort-teardown-audit.md` match HEAD: join/drop/drain at 4538/4566/4593, 5898/5938/5970, 6120/6121/6137, 7013/7025/7060, macro 7104/7105, invocations 7650, 7806, 8021, 8318 and 9571, cohort 9297/9298/9327, and tail 9708/9813/9845.
  - Release sites: 7 (six direct plus the macro).
  - Drains: 11.
  - `rg audit_writer main.rs` shows no other await or abort of the writer.
- **Split count.** S has 10 `#[test]` outside `mod cohort` and 5 inside, so 10 + 5 = 15 matches the `discipline.yml` 10+5=15 guard. `--skip cohort` and the `cohort` filter partition cleanly.
- **Cohort teardown.**
  - `PollTask::shutdown` and `Drop` (`yank_policy.rs:428-445`) are as described.
  - The blocking registry fetch runs on a separate thread (`yank.rs:176-226`), so the new unbounded `shutdown().await` in the cohort arm cannot hang on a slow registry.
  - Before and plant logs show the exact 10 s timeout and the AST red (`before-cohort-new-oracles.stdout`, `before-cohort-ast-release-guard.stdout`, commands.md lines 104-110). Copy-back, cmp and touch are all exit 0.
- **Pins.**
  - `kloc.toml` values (registry 5668, bin 35589, cli 6429, spirit-cli 942, hardfail 196965) equal the story's measured figures.
  - `nextest.toml` adds `registry_tls_20_1b` to the serial group.
  - `test-budgets.toml` total is unchanged (comment only).
  - `SCANNED_SOURCE_FILES` (38) is unaffected, since no `maos-bin/src` file was added.
  - `env_contract` shows no new `MAOS_*` read. `MAOS_REGISTRY_URI` is already registered, and the new operator keys come from TOML only (`security.rs:27-56`).
  - `Cargo.lock` adds edges only, with no new package versions. syn is a dev-dependency only.
- **Plants are real.**
  - `plant-*.stdout` shows real red tests. `plant-certificate-validation` fails all 3 interception tests. `plant-response-envelope-api` fails all 6 envelope tests. `plant-no-plaintext-downgrade` shows a trap `accept()` returning Ok, so the client really connected.
  - Exit 100 in commands.md is nextest's test-failure code, not a build error.
- **Dev-mode ends.** Server `serve()` refuses bind addresses where `addr.ip().is_loopback()` is false (`server.rs:167-171`). The client requires `development_http` from TOML only, not env (`http.rs:25-31`). The wire header is enforced (T:802).
- **Redirects and fallback.** No redirect-follow or HTTP-fallback code exists. T:767 and T:894 observe a trap for each.

---

## AA-01 (High) — hosted `epic-20-exit` CI job runs plain HTTP without dev opt-in; not migrated, not mentioned

- **Where:**
  - `.github/workflows/discipline.yml:2934` — `maos-registry-server --bind 127.0.0.1:6789 --root ...`, no `--dev-http`.
  - `discipline.yml:2942-2943` — `maos-spirit publish --registry-uri http://127.0.0.1:6789/mcp`, then `maosctl spirit install ... --registry-uri http://127.0.0.1:6789/mcp`.
  - `discipline.yml:2968-2975` — the same sequence in the AC4 step.
  - The diff touches `discipline.yml` only at the root-shutdown lines (≈1864-1889).
- **Evidence:**
  - `server.rs:167-170`: with no TLS and no `--dev-http`, `serve()` returns `"registry requires TLS or explicit literal-loopback development mode"`.
  - `server.rs:309-327` (`build()`): with no flags it returns `"supply --tls-cert ... or explicit --dev-http"`.
  - `http.rs:25-31`: the client refuses `http://` unless `[registry].development_http = true`. Neither step writes `$HOME/.config/maos/operator.toml`. The step sets `HOME="$scratch/home"` and nothing else.
  - The harness (`tests/harness/registry_20_1.rs`) was updated for exactly this. The workflow, which is another operator network entrypoint, was not.
  - `rg 'epic-20-exit|discipline.yml'` in the story/evidence shows no mention. Dev record claim "all operator network entrypoints share the blocking registry leaf" is true for code but not for the shipped CI smoke.
- **Consequence:** `epic-20-exit` (publish → install → shell dispatch, and the AC4 installed-component run) fails at "registry readiness failed within 5 s" (server exited) on the next hosted push. Hosted-unverified is disclosed generally, but this is a known-red job, not an unknown.
- **Fix:**
  1. In both steps add `--dev-http` to the server launch.
  2. Before publish/install, add:
     `mkdir -p "$HOME/.config/maos" && printf '[registry]\ndevelopment_http = true\n' > "$HOME/.config/maos/operator.toml"`.
     The AC4 step exports its own `HOME`, so write it there too.
  3. Alternatively, migrate the job to TLS with a generated CA and `signing_pubkey`; that also makes it AC15 hosted evidence.
  4. Record the job as an AC15-affected hosted leg in the story and deferred-work.
- **Failing-before witness needed:**
  - Local repro from the step's own commands against built `maos-registry-server`: expect the server to exit nonzero with the message above.
  - Preferred: an xtask or test that scans `.github/workflows/*.yml` for `maos-registry-server` invocations lacking `--dev-http` or `--tls-cert`, and `registry-uri http://` without a `development_http` write.

## AA-02 (Med) — "rustls must not leak into the no-default-features build" is contradicted by the evidence, and no-TLS cfg branches are never compiled

- **Where:** story AC15 clause 5; dev record "normal dependency tree has no rustls edge" (story ≈line 693-694); `EV/no-default-normal-dependency-tree.stdout`.
- **Evidence:**
  - The tree captured by `cargo tree -p maos-bin --no-default-features --edges normal` contains `maos-registry` at lines 621-640, with `rustls v0.23.46 (*)` and `rustls-pemfile v2.2.0` beneath it.
  - The path is `maos-bin → maos-cli → maos-registry`. `crates/maos-cli/Cargo.toml:23` and `crates/maos-fkcs/Cargo.toml:12` depend on `maos-registry` with default features, which are now `["tls"]` (`maos-registry/Cargo.toml:41-43`).
  - `maos-bin/Cargo.toml:47` takes `maos-cli` non-optionally. So `maos-bin --no-default-features` compiles registry with `tls` on, and the `network`-gated `maos-registry/tls` forwarding is moot.
  - `rustls` also appears via `maos-a2a-core` and `maos-kernel-core` (pre-existing), so the literal claim "no rustls edge" can never hold. What the AC means is "no new registry TLS edge".
  - Gate coverage: commands.md lines 65-66, 86-87 run only `-p maos-bin --no-default-features`. Nothing runs `cargo check -p maos-registry --no-default-features` or `-p maos-spirit-cli --no-default-features`. The `#[cfg(not(feature = "tls"))]` code (`http.rs:36-40,110-111`; `server.rs:159-160,316-319,372-373`) is therefore compiled by no gate, local or hosted. The macOS leg is also default features.
- **Consequence:** Dev-record statement is false as written. The AC "no leak" is unmet unless reinterpreted. The no-TLS branches may be uncompilable, and clippy `-D warnings` on them is unobserved.
- **Fix:**
  1. Put `default-features = false` on `maos-registry` in `maos-cli`/`maos-fkcs`.
  2. Give `maos-cli` an additive `tls` feature, default-on for the `maosctl` binary.
  3. Forward it from `maos-bin`'s `network` feature as `maos-cli/tls`.
  4. Add two CI/xtask checks: `cargo check -p maos-registry -p maos-spirit-cli --no-default-features`, and `cargo tree -p maos-bin --no-default-features -e features -i maos-registry` must not list `tls`.
  5. Or restate the AC and dev record to what is true, with a deferred-work row.
- **Failing-before witness needed:** the `cargo tree ... -e features -i maos-registry` assertion above is red now, as the tree shows `tls`. Add `cargo check -p maos-registry --no-default-features` as a gate and record its first result (it may be red for unused-variable or import errors in the no-TLS branches).

## AA-03 (Med) — head contiguity, request-hash binding and head-vs-list ordering have no failing-before witness; "stripped entry" refuses on the wrong check

- **Where:** `security.rs:139-233` (`verify_response`, `verify_head`); T:612-640 (`Responder` attack arms), T:723-765; story claim "Authenticated lists must be contiguous through that head"; witness table "Signed complete head/stripping".
- **Evidence:**
  - For `StripEntry` and `InstallStrip` the fixture sets `sent = fixture_bytes(&body)` (entries emptied) but `signed` is the original response (T:~630-640, `let mut signed = sent.clone()` before the mutation). `response_envelope(.., &signed, head)` therefore hashes the original body. The client refuses at `claim.response_sha256 != Sha256(response)` (`security.rs:155-160`), before `verify_head` runs. T:763 and CLI:858 (the strip variants) are body-hash witnesses, not head-completeness witnesses.
  - The only tests that reach `verify_head`'s error paths are `MissingHead` (head None) and `ShortHead` (head=1 with an empty list). `plant-head-and-development-remaining.stdout` confirms the head-only plant fails just those two (plus the dev tests).
  - Nothing sends a correctly signed response that:
    - has a gap, for example entries `[2]` with head 2 and cursor 0, or a duplicate;
    - has a list longer than the head (entries `[1]`, head 0);
    - rolls back below the client cursor (since_ns=5, head 3);
    - carries a request-hash mismatch, meaning a valid envelope for a different request body.
  - Deleting `entry.sequence != next` (`security.rs:220-222`), or `claim.request_sha256 != ...` (`security.rs:156`), turns no test red.
- **Consequence:** The "contiguous", "rollback" and "request-bound" claims in the dev record and deferred-work row ("complete signed heads ... nonce/request/body/sequence continuity") rest on code that no test constrains. A refactor that weakens either check passes the whole gate.
- **Fix:** Add `Attack` variants in T and one `#[test]` each:
  - `GapList`: entries `[{seq:2}]`, head 2, signed over the sent body.
  - `HeadBehindList`: entries `[1]`, head `Some(0)`.
  - `DuplicateSeq`.
  - `CursorAheadOfHead`: call `yanks_since(5)` against head 3.
  - `WrongRequestHash`: `response_envelope` called with a different `request` slice.
  For `StripEntry`/`InstallStrip`, sign over `sent` and set head=1 so the head check is what fires, and keep a separate body-tamper case (`signed != sent`) for the hash check. Re-label the witness-table row accordingly.
- **Failing-before witness needed:** three independent plants: (a) delete the `entry.sequence != next` block; (b) delete the request-hash comparison; (c) delete the `head != Some(next)` comparison. Each must turn a distinct new test red while the others pass.

## AA-04 (Low-Med) — AC15.2 "hostile-registry and interception witnesses refuse before persistence" is shown only for root/URI/omission; certificate interception is API-level, substring-checked

- **Where:** T:294, 304, 314 (`error.contains("certificate")`); T:395, 413, 858 (CLI + `assert_not_persisted`).
- **Evidence:** the three certificate cases call `client.yanks_since(0)` and assert only the substring "certificate". They never run `cli_install` and never call `assert_not_persisted` (T:355-362). The isolation of the property (hostname, time, issuer) is correct by construction in `Certificates::new`, but the message check cannot tell which property fired.
- **Consequence:** The AC text is satisfied by inference through the shared code path rather than by a witness. A regression that persists before first contact would not be caught by the interception cases.
- **Fix:**
  - Run each of the three cases through `cli_install` using the real `maosctl`, and assert `!status.success()` plus `assert_not_persisted`.
  - Assert distinguishing rustls text: `NotValidForName`, `Expired`, `UnknownIssuer`. Check that the error preserves them rather than the generic "certificate".
  - If the error does not preserve them, state that in the table.
- **Failing-before witness needed:** plant that moves store creation before the first registry call in `install.rs:56-62`. The new CLI cases go red; the current ones do not.

## AA-05 (Low-Med) — key-custody witnesses do not cover the TLS private key location or the seed's permissions, and assert only `is_err()`

- **Where:** T:126-160; `server.rs:287-289`; `security.rs:238-254`.
- **Evidence:**
  - AC15.4: "Keep private key material outside package/installation storage".
  - `build()` checks `self.signing_key.iter().chain(self.tls_private_key.iter())` against the store root. T:138 sets only `signing_key` (`tls_private_key` stays `None` via `Default`), and T:126 uses `with_tls` directly, which has no store check.
  - Seed-file permissions (`private_key_file`, `security.rs:238`) are tested only through the TLS key (T:126). Deleting `.chain(self.tls_private_key.iter())` turns nothing red.
  - Both tests assert `is_err()` with no message, so an unrelated error (a bad PEM, a missing file) also passes.
  - The witness table discloses "no foreign-owner negative runtime claim", which is honest. The omission above is not disclosed.
- **Fix:**
  - Add a build()-level test with `tls_private_key` inside the store and development_http false, and one with a 0644 seed. Assert the specific message (`must live outside the package/inventory store`; `operator-owned, mode0600`).
  - Tighten T:126 and T:138 to assert those messages.
  - Add a note for non-Unix (no permission check at all; `security.rs:243-249` is `cfg(unix)`), a deferred-work row.
- **Failing-before witness needed:** plant removing `.chain(self.tls_private_key.iter())` (expect a new test red), and plant removing the mode check (T:126 red already; add one for the seed).

## AA-06 (Low) — loopback and canonical-URI vectors are thin; one deferred-work citation does not support its claim

- **Where:** T:267-292 (`configured_root_is_bound_to_canonical_uri_before_contact_ac15`); `security.rs:59-81`; deferred-work row "Exact registry-URI alias disposition".
- **Evidence:**
  - The vectors are `localhost`, `192.0.2.1`, query, fragment, userinfo, and a mismatched bound URI.
  - Not covered:
    - `http://[::ffff:127.0.0.1]/mcp` (the correct behavior is refuse, as `Ipv6Addr::is_loopback` is false for mapped addresses).
    - `http://[::1]:p/mcp` positive.
    - `127.0.0.2`, `0.0.0.0`.
    - Uppercase scheme/host, explicit default port (`:80`, `:443`), trailing slash, trailing-dot host.
    - The server-side build()/serve() combinations TLS+`--dev-http` (refused at `server.rs:162,311`), and TLS without a signing key.
  - A plant changing `literal_loopback` to `ip.to_canonical().is_loopback()` passes every current test.
  - The deferred-work row claims "`localhost` and `127.0.0.1` do not silently share root/origin/cursor/audience", evidenced by `configured_root_is_bound...` and `envelope_wrong_uri_is_refused_ac15`. Neither exercises origin or cursor separation. The first only refuses HTTP `localhost` under dev config. The origin/cursor alias witness is the L1 plant. The citation should not claim it.
- **Fix:**
  - Add the listed URI vectors to the test, with expected accept/refuse per `canonical_uri`.
  - Add the two server combos in `registry_server_bin_20_1.rs` or T.
  - Re-cite the deferred-work row to the L1 witness (`same-key-other-registry-identical`) plus T:267 for the URI-exactness part only.
- **Failing-before witness needed:** the `to_canonical()` plant above, and a plant dropping the `url.as_str() != uri` clause (`security.rs:69`); only the userinfo and query vectors would catch part of it today.

## AA-07 (Low) — `maos-cli` pin lowered 6856 → 6429 forfeits the booked 15-2 reserve; authority only in the dev record

- **Where:** `xtask/kloc.toml:489` diff; `15-2-kloc-ceiling-rebase.md:388` (6856 includes reserved growth for 16-1/18/20-1 work).
- **Evidence:** the dev record says "CLI's legacy reserved budget 6856 is downward re-pinned to exact 6429, not counted as source shrinkage". The story's kloc_grant states the downward re-pin; I found no operator or Main ruling row dedicated to it. The orchestrator-adjudicated D02 A rebase covers the ceilings, and the new ceiling is 6549 (pass auxiliary), not a forfeiture of 6856.
- **Consequence:** a future maos-cli change (`20-2` and later) loses roughly 427 lines of reserved headroom irrevocably, with no deferred-work row naming the forfeiture.
- **Fix:** either keep the pin at 6856 with the 6429 as measured consumed (as 20-1a did: "retain reserved ceiling 6856"), or add a deferred-work row and ruling reference that the forfeiture was intended.
- **Failing-before witness needed:** none (record-only).

## AA-08 (Low) — AST guard soundness limits (scope of "every arm")

- **Where:** S:66-170; claim in `cohort-teardown-audit.md` "This is a structural contract for the named root owners".
- **Evidence (verified by reading the visitor):**
  - The drain and release counts are lower bounds: `releases >= 7`, `drains >= 11`. Only `drop(audit_tx);` line equality is exact.
  - The drain detector recognizes `<audit_writer | &mut audit_writer>.await` or `any_call(.., audit_writer, ..).await`. It does not unwrap `Expr::Paren`/`Group`, `async {}`, `select!`/`timeout(.., async { ... })` bodies, or a macro call with the writer as a token argument. A new drain of those shapes is never checked and the lower bound can still pass.
  - The join is recognized only as a statement `yank_poller_task.shutdown().await;`. `let _ = ...` or a `match` arm expression does not count. That fails conservatively, which is fine.
  - The guard covers `main.rs` only (`include_str!("../src/main.rs")`). `await_audit_drain` and any helper that holds `audit_tx` in `supervision.rs` or `lib.rs` are out of scope, and the doc discloses this.
  - Soundness check passed: sibling-branch isolation (`visit_block` restores `joined`), the macro-definition content assertion, and the macro-invocation inheritance all work as described. The AST test was observed red before the fix and under the plant (`before-cohort-ast-release-guard.stdout`, `plant-cohort-yank-join-ast.stdout`).
- **Consequence:** A future 12th drain written in an unrecognized shape evades the guard silently.
- **Fix:**
  - Change `>= 11` and `>= 7` to exact `==`, so a new site forces a conscious update of the test.
  - Add a token-level cross-check: count occurrences of the ident `audit_writer` in `main.rs` outside comments, via `proc_macro2` token iteration, and assert `occurrences == drains + 1 (spawn) + 1 (let)`.
  - Unwrap `Paren`/`Group` in `is_writer`.
- **Failing-before witness needed:** a plant adding a 12th drain of shape `tokio::time::timeout(d, async { (&mut audit_writer).await }).await` without a join. It must go red, and does not today.

## AA-09 (Nit) — evidence honesty

- `plants.md` says "every actual command/exit is in `commands.json`... all copy-back/cmp/touch exits 0". That is true. `commands.md` lines 17-18 show `certificate-validation-save` exit 1 (custody dir not yet present) followed by a successful retry before the plant ran. The failed save is not listed under "Honest exclusions". No effect on the credited plant.
- `plants.md` credits "independent head-only plant 100" for the stripped-entry row. It covers `MissingHead` and `ShortHead` only (AA-03), not the strip cases.

## Pins and cross-checks that need no action

- `ac-witness-table.md` AC15 and cohort rows are backed by real tests and logs, with the exceptions in AA-03 and AA-04.
- `sprint-status.yaml` and `coverage-matrix.yaml` text matches the evidence: unverified hosted and Apple legs are disclosed, with no status flip.
- The deferred-work rows added for AC15 and the cohort regression are consistent with the evidence. Residuals not yet rowed: AA-02 (feature leak), AA-05 (non-Unix key permissions), and the fact that `ca_certificate` extends the bundled WebPKI roots rather than narrowing them (`security.rs:272-278`). The last is within D-20-1b-02 A ("platform/WebPKI trust with optional operator CA") but worth stating in `SECURITY.md`.
