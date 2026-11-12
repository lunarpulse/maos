# Story 20-13 landing-3: AC7.2 bin-side recording proof

Scope: tests and CI recipe only. No kernel file, no `src` file, no invariant doc touched.

## Finding (read before relying on the analysis)

The analysis assumed a daemon T3 admission would reach the kernel T3 arm. It cannot, in the shipped composition root:

- `SecurityManagerAdapter::admit_spirit` takes the T3 arm only when `effective_sandbox_tier == T3` (`crates/maos-kernel-core/src/security/mod.rs:362`).
- `effective_sandbox_tier` is `max(declared, trust_tier_floor, operator floor)` (`crates/maos-kernel-core/src/capability/cap_policy/mod.rs:225-247`). Every term defaults to `SandboxTier::T2` (`DEFAULT_FLOOR`, `crates/maos-domain/src/invariants/i9.rs:90,122`), and `declared_tier` is itself stored as the previous `effective` (`security/mod.rs:323`); the manifest `[sandbox] tier` is not an input (`crates/maos-bin/src/admission.rs:300-302`).
- grep over `crates/*/src` (excluding tests, benches) finds NO writer of `trust_tier_floor`, `spirit_tier_floor`, `global_sandbox_floor` or `declared_tier` in `maos-bin`; the only readers/writers are `cap_policy/mod.rs` itself, `security/mod.rs:495` (read) and `maos-bench/src/halt_eval.rs:615` (a bench). `enterprise_pdp_runtime.rs:249,487` update only the deny sets.
- `maos run <worker manifest>` does not call `admit_spirit` at all: it goes through `admit_cli_wrapper_journaled` and the lock check at `worker_spawn.rs:888`.

So no `maos` process, hosted or local, can show the kernel arm's `verify_signature` next to the Worker's: the kernel arm has no production caller today. A "count >= 2 in one daemon run" assertion is therefore not attainable and was not written. What is attainable, and is what story AC7.2 literally asks ("With `recording` (bin leg), L1 records no kernel `verify_signature`; L2 records one"), is below. This is [INFERENCE]-free for the grep facts above (observed), but the claim "no other writer" is a grep result, not a compiler proof.

## Test

`crates/maos-bin/tests/crypto_t3_admission_20_13.rs` (feature `crypto-recording`, one test, 0 podman):

1. `maos_bin::crypto_provider::select(Some(home))` with `[crypto] provider = "recording"`: the daemon's own selector, the value `main.rs:2260` (at the time of writing) passes as `Arc::clone(&crypto)`.
2. A probe call on the selected instance and a call on a second, decoy `select` instance read their `pid:instance` ids from `crypto-recording-identity.log`; the ids must differ.
3. Per-Spirit T3 floor is raised in the policy (the only way to reach the arm), a validly signed lock + trust anchor go in through the environment (the arm's no-config branch, the one the daemon's adapter takes), `SecurityManagerAdapter::new(policy, Arc::clone(&selected.provider))` admits at T3.
4. Asserts: exactly one new `verify_signature` row, carrying the selected instance's id (not the decoy's); a tampered lock is refused `T3AdmissionFailed` and its verification is also recorded on the selected instance.

Not proven by it: that `main.rs:2260` (at the time of writing) passes the selected instance. That stays proven by the required ctor argument, the exactly-one-construction gate (`check-composition-root-completeness`, green) and `main.rs:2240-2262` reading. The test proves the daemon's selected recording provider type, wired as the daemon wires it, is what the kernel arm verifies with.

## Local (observed, this checkout)

| Step | Command | Result | Log |
|---|---|---|---|
| green | `env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-bin --features crypto-recording --test crypto_t3_admission_20_13` | exit 0, 1 passed | `local/green.log` |
| plant (test-side): adapter built from the decoy instance (line 180) | same | exit 101, `crypto_t3_admission_20_13.rs:188`: `the kernel arm verified on a different instance than the selected one`, left `613215:2`, right `613215:1` | `local/plant-decoy.log`, `local/plant-decoy.diff` |
| clippy on the new target | `cargo clippy --locked -p maos-bin --features crypto-recording --test crypto_t3_admission_20_13` | exit 0, 0 diagnostics in the new file (existing lib/dep warnings only) | `local/clippy.log` |
| `python3 scripts/repin.py --check` | | PINS IN SYNC; `maos-bin 38619 38619 ok` (tests do not count; no repin applied, no overdraft) | |
| gates | `gates/*.json`: check-serde-error-handling, check-dev-model-tier, check-kernel-baseline, check-service-boundary, check-dev-record-completeness, check-epic-close-coherence, check-exit-commands, check-decision-register, check-env-contract, check-composition-root-completeness, invariant-lock (`changed-files.txt`) | all exit 0 | `gates/` |

The kernel-side plant (restore the inline ring) was NOT executed locally: it needs a kernel edit, which this assignment forbids. It was run hosted (below).

## Hosted (branch `ci/20-13-verify`, pushed from a temp clone of local HEAD 501d1d55; the clone is deleted)

| Run | Commit | Result | What it shows |
|---|---|---|---|
| 43183933114 | 3947a712 | failure (4 hermetic), apple-check cancelled | New test PASSED on all four images; `build-test-fixtures` then failed: wasm32-wasip2 target missing (recipe bug); apple-check: macos-14 unsupported (job 131066361556). Logs `run1/`. |
| 43186933794 | ac0e0306 | failure | Recipe fixed (target, macos-latest). New test PASS; `crypto_provider_20_13` 12/12, real_shell/real_boot 2/2; x64 Worker T3 leg PASS; `kloc-check` failed: tokei missing (recipe bug). Logs `run2/`. |
| **43192091802** | **82adc3e6** | **success, all 5 jobs** | The final recipe, clean tree. Jobs: hermetic ubuntu-24.04 **131093697864**, ubuntu-26.04 **131093697365**, ubuntu-24.04-arm 131093697444, ubuntu-26.04-arm 131093697443, apple-check 131093696975. Logs `run3/`. |
| 43196099681 | 563e7218 | failure | Plant recipe bug (shallow checkout has no HEAD~1). No claim. `run4/`. |
| **43196481572** | **ce1e0ada** | **success (plant detected)** | Kernel T3 arm planted with inline `RingCryptoProvider` (`ci/plant-kernel-inline-ring.diff`): `crypto_t3_admission_20_13` FAILED at `:183`: `kernel T3 admission must verify exactly once: ["7879:1", "7879:2"]`, left 2, right 3. Job **131108653587**, `run5/`. |

Hosted-proven in run 43192091802 (final recipe, clean tree):
- New bin-side recording proof PASS on x64 and arm, 4 jobs (24.04 job 131093697864, 26.04 131093697365, 24.04-arm 131093697444, 26.04-arm 131093697443).
- Worker T3 leg `crypto_provider_t3_20_13::recording_provider_observes_worker_t3_lock` PASS on the real T3 substrate, x64 only (jobs 131093697864 line 10260 and 131093697365 line 10001 in `run3/`). This is the first hosted pass of that leg on a recipe-verified tree.
- Gates in the recipe: check-env-contract, check-service-boundary, check-composition-root-completeness, kloc-check, check-kernel-baseline green on all four hermetic jobs; `cargo check --release --features crypto-recording` refuses with the expected message.
- `crypto_provider_20_13` (12), `crypto_real_shell_20_13` and `crypto_real_boot_20_13` (2) green.

Not proven: a daemon (`maos`) T3 admission recorded as a kernel `verify_signature` (unreachable, see Finding); `main.rs:2260` (at the time of writing) instance identity by a running test; ARM Worker T3 leg (the recipe never provisions T3 on arm); `abi-diff`, `prepush`, the final-tree discipline run.

Branch state: `ci/20-13-verify` tip is the PLANT commit (ce1e0ada). The clean recipe commit is 82adc3e6. Nothing was pushed to main or tags; no force.

## Files

Changed: `crates/maos-bin/tests/crypto_t3_admission_20_13.rs` (new), `20-13-…-evidence/ci/ci-20-13-verify.yml` (recipe: crypto test step, wasm32-wasip2 target, tokei, macos-latest). New evidence under this directory.
