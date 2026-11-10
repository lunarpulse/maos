# Test-infrastructure audit — 20-13 L1 (commit a2e37988, parent be3b9763)

Scope: `crates/maos-bin/tests/crypto_provider_20_13.rs`, `crypto_provider_t3_20_13.rs`,
`tests/support/crypto_integrated_20_13.rs`, `tests/support/shell_world_16_2.rs`,
`erasure_uninstall_13_5b.rs`, `crates/maos-registry/tests/publisher_provider_20_13.rs`,
`crates/maos-compliance/tests/evaluator_test.rs` (additions), `xtask/tests/ci_shape_18_0.rs`,
`crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs`, `xtask/test-budgets.toml`,
`tests/coverage-matrix.yaml`. Supporting source read for vacuity judgment:
`crates/maos-bin/src/crypto_provider.rs`, `main.rs` diff, `maos-shell/src/lib.rs` diff,
`build_test_fixtures.rs`, `check_mock_not_in_release.rs`, `.config/nextest.toml`,
`.github/workflows/ci/ci-20-13-verify.yml`, `tests/harness/fixtures.rs`, `t3_substrate.rs`,
`Cargo.toml` profiles. Read-only; nothing was executed (no cargo/git mutation per assignment).

## Direct answers to the audit questions

- **Non-vacuous / would red if broken?** Yes for every vector I traced. Presence-style asserts use
  exact line matches (`crypto_provider_20_13.rs:168-171`, `:478-480`), the installed-claim boot
  vector asserts a real refusal message and then a real admitting boot
  (`crypto_provider_20_13.rs:283-291,300-311`), the CRL count is pinned exactly `prior + 1` on the
  integrated path (`crypto_integrated_20_13.rs:228-233`). Failing-before/plant runs recorded in
  `verification.md` (select fallback, CRL second instance, omitted frame, manifest crypto +
  endpoint, publisher RejectingProvider→Ok, shared-unpinned evaluator, strict-Z classifier,
  door second instance) match the vectors present in the files.
- **Integrated test truly one boot?** Yes. The package is written before the boot pass
  (`crypto_integrated_20_13.rs:112-125`); `InstalledSpirits::boot` and the daemon boot
  (`boot_and_drive_with_shared`, `main.rs:16530-16581`) run in one process on shared `Arc`s; the
  ready/resume handshake orders CRL import strictly between daemon-ready and daemon-governance, so
  no restart and no un-ordered writes. No flaky sleep: only bounded polls (10 ms/10 s timeout with
  a success condition, `crypto_integrated_20_13.rs:222-229`).
- **Identity check meaningful?** Yes. `RecordingCryptoProvider` stamps
  `"{pid}:{instance}"` from a process-global `AtomicU64` counter incremented per construction
  (`crypto_provider.rs:126-145`); the test asserts every row shares `rows[0].0`
  (`crypto_integrated_20_13.rs:248-254`). A second `select()` or a direct `ring` construction
  changes/omits identities — and the recorded door-only second-instance plant failed (exit 101,
  `artifact://15792`), so the check has demonstrated teeth.
- **Negative control real?** Yes, on every file audited:
  - `publisher_provider_20_13.rs:64`: `!verify_publisher_sig_with(&pkg, &RejectingProvider)` —
    fails if the `_with` variant ignores its provider; the recorded plant (provider returns
    `Ok(())`) reds the file.
  - `evaluator_test.rs:68-92`: pins both halves of D-20-13-17 (ring-direct empty-vs-nonempty
    drifts; self-attested admits) with a field-exact `ContextDrift` match; recorded inverse
    witnesses exist (exit 101 `artifact://15754`, plant `artifact://15769`).
  - `ci_shape_18_0.rs`: every count/name token is independently removed and the gate must reject
    (`:138-147`, `:186-196`).
  - `cgroup_ceiling_smoke.rs:689-695`: R/S/D explicitly rejected (see TI-4 for oracle shape).
- **Sleeps/flakes?** No unconditional sleeps. The CRL door readiness loop is bounded (45 s
  deadline, 100 ms poll) with a daemon-exited guard (`crypto_provider_20_13.rs:462-480`); all
  other waits are timeout-bounded polls or handshake-ordered. Serial-group registration
  (`.config/nextest.toml`) + test-budget exception for the T3 binary are consistent.
- **Hidden skips?** None found. No `#[ignore]` in any audited file. `#![cfg(feature =
  "network")]` is inert: `network` is a default feature (`maos-bin/Cargo.toml:14`). The
  feature-gated tests (`crypto-recording`) are explicitly run by the story CI recipe with
  `--features crypto-recording` (both the `--bin maos` unit test and the integration test),
  and the release compile_error is asserted to fail with the exact message
  (`ci-20-13-verify.yml:27-34`). The T3 test's local early-return is the repo's named policy
  (`t3_substrate.rs:14-33` prints the SKIP reason; CI provisions the substrate and runs it on
  non-ARM), i.e. a named skip, not a hidden one.
- **ci_shape counts derived from source?** Yes — `source_test_names` parses
  `recall_17_3d.rs` / `recall_wire.rs` with `syn` and derives names and counts
  (`xtask/tests/ci_shape_18_0.rs:101-131`); the suite-count table row for 17-3d is likewise
  derived (`:1553-1560`). Only the workflow file itself still carries literal counts, which is
  the design (source is the oracle, workflow is checked against it).
- **cgroup X-state oracle**: running/sleeping still rejected — `Some("R"|"S"|"D")` are asserted
  live in the dedicated test (`cgroup_ceiling_smoke.rs:692-695`), and the real assertion site uses
  the same oracle (`:773-782`). Accepting `"X"` is correct Linux semantics (`EXIT_DEAD` in
  `fs/proc/array.c`'s `task_state_array`); the recorded plant (restore strict-only `Z`) reds on
  `X` (`artifact://15724`). Two residual leniencies in TI-4.
- **test-budgets.toml**: single new exception (`maos-bin::crypto_provider_t3_20_13`, 31 s,
  owner 20-13), matches `.config/nextest.toml` serial-group addition. Reasonable; see TI-5 for
  the asymmetry with the unaudited-for-budget CRL door deadline.
- **coverage-matrix.yaml**: FR48 gains the `check-composition-root-completeness` gate, which is
  registered in `gate-registry.toml` (the workspace-run failure was real and was fixed); notes
  correctly scope the FIPS half to v1.5 and kernel T3 to L2. AC6.2 satisfied.

## Findings

### TI-1 — MEDIUM — changed-value re-selection is never witnessed end-to-end
- **Where:** `crates/maos-bin/tests/crypto_provider_20_13.rs:81-101`
  (`reselection_preserves_unrelated_configuration`).
- **What:** The only re-selection vector re-inits with the **same** provider (`ring` → `ring`).
  It passes even if `maos-shell::record_crypto_provider` (`maos-shell/src/lib.rs:802-841`) never
  writes anything: file-byte-equality holds for a no-op, and the `ring -> ring` stdout comes from
  the print, not the write. No test inits `--crypto-provider recording` over an existing `ring`
  home, so D-02 A's headline behaviour — the `[crypto]` value actually flips, staged atomically,
  unrelated keys preserved, effective at next boot — has no witness. The fresh-home write path is
  covered (`init_persists_selected_provider`), the rewrite path is not.
- **Why it matters:** A regression that makes re-selection a silent no-op (or drops unrelated
  keys when the value differs) ships green.
- **Fix:** In the re-selection test, second init uses `recording`; assert stdout contains
  `ring -> recording`, parse `config.toml` `crypto.provider == "recording"`, and boot once more
  with the recording fixture expecting a `crypto-recording.log` line.
- **Failing-before witness:** plant `record_crypto_provider` to compute and return
  `Ok(Some(previous))` without writing the file — on HEAD the whole
  `crypto_provider_20_13` suite stays green (existing vector can't see it), while the added
  changed-value asserts red.

### TI-2 — LOW — journaled frame `version` is never asserted
- **Where:** `crates/maos-bin/tests/crypto_provider_20_13.rs:340-341`
  (`selected_provider_is_journaled_once_per_boot`).
- **What:** The test pins `payload["provider"]` and `payload["source"]` but not
  `payload["version"]`. The version pin lives only at the table level
  (`ring_version_matches_lockfile`, `:69-77`), so `crypto_provider::journal`
  (`crypto_provider.rs:91-104`) could emit a stale/hardcoded version and every test stays green.
- **Fix:** Assert `payload["version"] == maos_bin::crypto_provider::validate("ring").unwrap().version`
  in the journal vector (one line).
- **Failing-before witness:** plant `journal` to write `"version": "0.0.0"` — HEAD suite green;
  the added assert reds.

### TI-3 — LOW — "one instance per process" is identity-checked only on the daemon path
- **Where:** `crypto_provider_20_13.rs:104-153` (one-shot boot) and `:394-493` (CRL door) read
  only `crypto-recording.log` (bare op names). The `pid:instance` identity log is asserted only
  by `crypto_integrated_20_13.rs:248-254` (daemon).
- **What:** AC1.2 requires every non-daemon root to `select` itself, once. If a one-shot or smoke
  arm called `select()` twice (two recording instances in one process), all CLI vectors still
  pass — the plain log cannot distinguish instances.
- **Fix:** In `recording_provider_observes_boot_token_signing`, additionally read
  `home/maos/crypto-recording-identity.log` and assert all rows share one `pid:instance` prefix
  (mirrors the integrated check, ~5 lines).
- **Failing-before witness:** plant a second `select()` in the one-shot arm — HEAD CLI suite
  green; the added single-identity assert reds.

### TI-4 — LOW — cgroup oracle: `None` treated as dead; lowercase `x` rejected
- **Where:** `crates/maos-kernel-core/tests/cgroup_ceiling_smoke.rs:684-686`.
- **What:** `descendant_is_dead` accepts `None`, but its only production caller always passes
  `Some(...)` (`:775-779` `expect`s first), so the `None` arm is unit-test-only leniency that a
  future caller could inherit. Conversely `Some("x")` — also a dead state
  (`task_state_array`: `"x (dead)"`) — is rejected, so a kernel reporting `x` for the reaped
  descendant would false-red the real assertion (the ARM incident reported `X`).
- **Fix:** `matches!(state, None | Some("Z" | "X" | "x"))` (or drop the `None` arm and tighten
  the unit test) — 1 line, semantics unchanged for the observed ARM case.
- **Failing-before witness:** `descendant_terminal_state_accepts_reaped_or_dead_but_not_live`
  with `Some("x")` in the loop fails on HEAD. [INFERENCE] that `x` can actually occur for this
  workload; the fix is cheap hardening either way.

### TI-5 — LOW — CRL door test's 45 s deadline exceeds the 30 s default test budget
- **Where:** `crates/maos-bin/tests/crypto_provider_20_13.rs:462` vs `xtask/test-budgets.toml`
  (no exception for `maos-bin::crypto_provider_20_13`; default 30 s per the T3 entry's comment).
- **What:** A hosted run where the operator door becomes ready between 30 s and 45 s passes the
  test but fails `check-test-cost`. The T3 binary got an owned exception; this binary did not,
  yet it carries the larger internal bound.
- **Fix:** Lower the deadline to 30 s, or add a `test-budgets.toml` exception with the same
  justification pattern as the T3 entry.
- **Failing-before witness:** none runnable here (budget gate not executed per assignment);
  inconsistency is direct from the two files.

### TI-6 — LOW — refused installed-boot leg does not pin the exit code
- **Where:** `crates/maos-bin/tests/crypto_provider_20_13.rs:283-291`.
- **What:** The drift-refusal boot asserts only stderr content
  ("installed spirit 'drift-spirit' refused at boot" + "CryptoProvider"); the sibling init/run
  refusal vectors all pin exit 78. A regression that reports the refusal but exits 0 (e.g.
  continues with the Spirit unloaded-but-alive) could still match the message.
- **Fix:** `assert_eq!(refused_boot.status.code(), Some(<the typed boot-refusal code used by
  installed_spirits>));` next to the stderr assert.
- **Failing-before witness:** plant `InstalledSpirits::boot` to log the refusal and continue —
  HEAD vector green; the exit-code assert reds.

## Explicitly verified, no finding

- `crypto_provider_t3_20_13.rs`: gate is the named `require_t3` policy; when it runs it does real
  work (signed lock via fixture binary, log-then-success ordering, isolated runtime home);
  `worker-cli-fixture` resolved via profile dir on PATH.
- `erasure_uninstall_13_5b.rs` addition: exactly pins the D-16-2-F boundary against the new
  journal frame — prior-boot selection frame ⇒ exit 5/`failed`/no proof files
  (`:704-729`), while the pre-existing empty-Log vector still pins exit 4/`not_found`
  (`:694-701`), and the uninstall exclusion matches only the *current* boot's frame
  (`main.rs:10850-10869`: boot_nonce + spirit_pid 0 + kind + intent + origin).
- `publisher_provider_20_13.rs`: ring equivalence (both entry points agree), tamper matrix
  (manifest/artifact/signature) refused on both paths, provider-consultation proven by
  `RejectingProvider`.
- `shell_world_16_2.rs`: pure refactor — `kernel_world`/`regression_door_with_ops` delegate with
  `RingCryptoProvider`/`None`, so every pre-existing 16-2 consumer keeps HEAD behaviour (D-20-13-17
  ring-direct preserved); only the new integrated witness passes the selected provider and a CRL
  anchor.
- Release containment of the recording provider: non-default feature; `compile_error!` on
  `not(debug_assertions)` (`crypto_provider.rs:109-110`); no `debug-assertions` override in
  `[profile.release]` (`Cargo.toml:100-113`); `RecordingCryptoProvider` in
  `FORBIDDEN_PRODUCTION_SYMBOLS` (`check_mock_not_in_release.rs:29-33`); CI asserts both the
  release compile failure and the symbol gate.
- Fixture hygiene: `fixtures::fixture("maos-crypto-recording")` is the 20-12 prebuilt-fixture
  contract (missing/stale ⇒ panic naming `build-test-fixtures`), never in-test cargo; the new
  `build_test_fixtures.rs:123-146` entry matches the other `Fixture` rows.

## Unverified

- Hosted legs (Worker T3 execution, ARM/Apple `ci-20-13-verify` jobs, the branch-only recipe
  itself) — not runnable locally per the evidence and assignment; the 31 s T3 budget exception is
  explicitly "pending a hosted measurement".
- Runtime exit code of the refused installed boot (TI-6) — I did not run the binary; the exact
  code to pin must be read from the refusal path at fix time.
