# 20-13 closure note, 2026-11-12 (record-only; no code changed by this closure)

Commits: L1 `a2e37988` (+ corrections `a6b557c0`, hosted regression fix `a44c76d9`); L2 kernel `998b36e6`; L2 reviews `b270054b`; AC7.2 bin-side proof `42c50fc4`.

## Hosted proof (read with `gh run view` on 2026-11-12)

| Run | Commit | Result | What it shows |
|---|---|---|---|
| discipline 43175946640 | `637721bf` | success (`reproducible-build` failed; it is an ADVISORY job, `discipline.yml:75` `continue-on-error: true`; I did not read the cause) | Final-tree proof of the landed L2 (`998b36e6` is an ancestor of `637721bf`): `check-composition-root-completeness`, `workspace-test-suite`, `t3-smoke-busybox` (24.04, 26.04), `abi-diff`, `kloc-check`, `check-decision-register`, `check-exit-commands` all success. `abi-diff` here is the clean-tree `abi-diff` evidence. |
| `ci/20-13-verify` 43192091802 | `82adc3e6` | success, 5/5 | New bin-side test on x64 and arm (jobs 131093697864 24.04, 131093697365 26.04, 131093697444 24.04-arm, 131093697443 26.04-arm); Worker T3 leg `crypto_provider_t3_20_13::recording_provider_observes_worker_t3_lock` on the real T3 substrate, x64 only; `crypto_provider_20_13` (12), `crypto_real_shell_20_13`, `crypto_real_boot_20_13`; five recipe gates; release-build refusal. |
| `ci/20-13-verify` 43196481572 | `ce1e0ada` | success = plant detected | Kernel T3 arm planted with the inline `RingCryptoProvider`: `crypto_t3_admission_20_13` red at `:183` (job 131108653587). |

Branch `ci/20-13-verify` tip is the plant commit; it is not reverted and nothing from it is on `main`.

## AC table

| AC | State | Evidence |
|---|---|---|
| AC1 init flag, one selection, exit-78 refusal | LANDED (`a2e37988`, `a6b557c0`, `a44c76d9`) | `crypto_provider_20_13` |
| AC2 recording provider and routing | LANDED (`a2e37988`, `a6b557c0`) | `crypto_integrated_20_13` (in-process, four surfaces), `crypto_real_shell_20_13`, `crypto_real_boot_20_13` (per-mode split ruled); Worker T3 leg x64 hosted only |
| AC3 composition-root gate | LANDED (`a6b557c0`, kernel exception removed at `998b36e6`) | `check-composition-root-completeness`; L2 plant red at `security/mod.rs:363` |
| AC4.1-4.3 claims through the selected provider | LANDED (`a2e37988`, `a6b557c0`) | `evaluate_envelope_with`, `verify_publisher_sig_with`, `verify_installed`; D-20-13-17 ring-direct preserved |
| AC4.4 endpoint half | LANDED (`a6b557c0`) | `crypto_endpoint_drift_20_13.rs`; `crypto_provider_20_13::installed_claims_compare_actual_crypto_and_endpoint` against 20-4's `ProviderCatalog` (`installed_spirits.rs:25-37`) |
| AC4.5 `deferred-work` stamp | LANDED (`@a6b557c0b8ec`) for the daemon halves | The `maosctl install`/`import` remainder (`subcommands.rs:1408` still `None`) is `ownerless and open`, dispositioned by D-20-13-05 A |
| AC5 journal frame | LANDED | one frame per boot; `selection_journal_failure_stops_real_boot`; `Cargo.lock` pin test |
| AC6 docs | LANDED | `docs/maos.dev/run-maos.md`, `docs-site/docs/run-maos.md`, ko page (prose not read by the closure); coverage-matrix FR48/NFR-Sec-15 notes |
| AC7.1 KT3 ctor, `Default` removed | LANDED (`998b36e6`) | kernel 27,044 / 21,202 (+1/-1), one surface refresh `kt3` |
| AC7.2 failing-before at kernel level | LANDED (`998b36e6`) | `t3_admission_crypto_20_13.rs`; `landing-2/red/`, `landing-2/plants/` |
| AC7.2 bin-side recording proof | AMENDED; proven form LANDED (`42c50fc4`) | `crypto_t3_admission_20_13.rs`; hosted 43192091802; plant 43196481572 |
| AC7.3 gate entry removed | LANDED (`998b36e6`) | same |
| AC7.4 plant | LANDED | `landing-2/plants/p1-*` |

## CLAIM BOUNDARIES

1. **No daemon-level kernel T3 admission exists today.** AC7.2 as originally written (a daemon T3 admission recorded as a kernel `verify_signature`) cannot be proven: the shipped daemon never reaches the kernel T3 arm. The arm runs only when the effective tier is T3 (`crates/maos-kernel-core/src/security/mod.rs:362`); the effective tier is the strictest of three floors (`capability/cap_policy/mod.rs:225-247`), each defaulting to T2 (`maos-domain/src/invariants/i9.rs:90,122`); the manifest `[sandbox] tier` is not an input (`crates/maos-bin/src/admission.rs:296-302`); `maos-bin` writes no tier floor or `declared_tier` on the daemon path; and `maos run <worker manifest>` never calls `admit_spirit`, going through `admit_cli_wrapper_journaled` and the Worker lock check at `worker_spawn.rs:888`, which IS the live T3 lock gate. "No other writer" is a grep result, not a compiler proof. Closure cite check: every cite above was read; the only correction is that `maos-bin` does contain one floor writer, the 7.5a smoke arm (`main.rs:15695`, `trust_tier_floor.insert(Verified, T0)`, not a T3 path), besides the bench writer `maos-bench/src/halt_eval.rs:615-618`. Rule-10 row: `deferred-work.md` "The kernel T3 admission arm has no production caller in the shipped daemon".
2. The daemon instance at `main.rs:2257-2260` is NOT checked by a running test. It rests on the required constructor argument, `check-composition-root-completeness` and reading `main.rs`. `crypto_t3_admission_20_13` proves that the selector's recording provider, wired as the daemon wires it, is what the kernel arm verifies with (one row on the selected instance, none on a decoy).
3. The Worker T3 leg ran on x64 only; the recipe never provisions T3 on arm.
4. `prepush` was not run. L1's local attempt timed out at 3000 s (`landing-1/verification.md`). The hosted discipline run is a substitute, not an equivalent.
5. The `maosctl spirit install`/`import` callers stay ring-direct and vacuous on endpoint drift (labelled install-time pre-checks, D-20-13-05 A).
6. Ring is not a FIPS-validated module; the FIPS provider stays v1.5 (`deferred-work.md` NFR-Sec-15 row, ownerless and open).
7. Unverified here: the cause of the advisory `reproducible-build` failure; the ko docs prose; arm hosted Worker T3.

## Reviews

| Round | Reviewer | Result | File |
|---|---|---|---|
| L1 | Blind / Edge / Acceptance (`reviewer`), Test-infra (`light-reviewer`) | 0 blocker, 0 high, 6 medium (P1-P6, fixed in `a6b557c0`), 7 low, 3 defer | `landing-1/review/triage.md` |
| L1 | FKCS admission review | recorded | `landing-1/review/fkcs-admission-review.md` |
| L2 | GPT `gpt-6.1-sol` | 0 findings; not approval to mark complete (AC7.2 owed, now landed) | `landing-2/review/gpt-review.md` |
| L2 | GLM `glm-5.3` security | 2 P3 (rule-10 rows F1, F2) | `landing-2/review/security.md` |

The AC7.2 bin-side proof (`42c50fc4`) and this closure had no separate independent review.

## Records

`deferred-work.md`: FR38 drift row, daemon halves CLOSED `@a6b557c0b8ec`; L1 sub-bullet CLOSED; F1/F2 P3, ring-direct, `maosctl` remainder, cohort-daemon rows dispositioned under rule 10 (cohort row is in the gate's `ownerless_dispositioned` bucket); two new rows (T3 arm unreachable; instance identity and arm Worker leg). `tests/coverage-matrix.yaml` FR48 note extended.
