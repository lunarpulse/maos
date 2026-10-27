# Stale-tree plant (T2 / D-03) — fixture transport — 2026-10-27

Tree: committed candidate `38e5b52a` (clean; `HEAD^{tree}` = `6ed12f63dac3baf3bb32df606a5d66fa1f182148`, `HEAD~1^{tree}` = `e4e7cb953908323ada36ac6a2ccf950097b5ebe5`, so the two differ). Exclusive cargo use, target `/mnt/build/cargo`. Environment for every command: `env -u CI -u GITHUB_ACTIONS CARGO_TARGET_DIR=/mnt/build/cargo MAOS_FIXTURE_TRANSPORT=1 MAOS_FIXTURE_DIR=/tmp/maos-plant-fixtures`.

## Producer (seals the clean tree)

```text
cargo run -q --locked -p xtask -- build-test-fixtures --only forbidden-syscall-probe
```

Exit **0**, wall 7.53 s. Output: `build-test-fixtures: 1 artefact(s) → /tmp/maos-plant-fixtures/manifest.json`. Resulting entry (schema 3):

```json
"forbidden-syscall-probe": {"path":"/home/lunarpulse/dev_ws/crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/target/release/forbidden-syscall-probe","built_at":1793117524,"inputs":[…],"tree":"6ed12f63dac3baf3bb32df606a5d66fa1f182148","artifact":"ea4fc90adeb5cd5d244ad8bd7c7f121b150642eb"}
```

## Control (sealed manifest → pass)

The consumer needs only this fixture and no substrate: `maos-exec-deps::resolver_17_6 dynamic_probe_resolves_canonical_loader_and_libc_closure`, which reads `fixtures::fixture("forbidden-syscall-probe")`.

```text
cargo nextest run --locked --color never -p maos-exec-deps --test resolver_17_6 -E 'test(=dynamic_probe_resolves_canonical_loader_and_libc_closure)'
```

Exit **0**, wall 0.80 s: `PASS [   0.019s] (1/1) maos-exec-deps::resolver_17_6 dynamic_probe_resolves_canonical_loader_and_libc_closure` / `Summary … 1 test run: 1 passed, 18 skipped`.

## Plant (manifest tree → `HEAD~1^{tree}`)

Only the entry's `tree` value was replaced. The producer's compact byte layout was kept: `"tree":"6ed12f63dac3baf3bb32df606a5d66fa1f182148"` → `"tree":"e4e7cb953908323ada36ac6a2ccf950097b5ebe5"`. The same command as the control then gave exit **100**, wall 0.39 s:

```text
FAIL [   0.024s] (1/1) maos-exec-deps::resolver_17_6 dynamic_probe_resolves_canonical_loader_and_libc_closure
thread 'dynamic_probe_resolves_canonical_loader_and_libc_closure' (3103772) panicked at crates/maos-exec-deps/tests/../../../tests/harness/fixtures.rs:61:5:
test fixture stale — `forbidden-syscall-probe`: manifest tree e4e7cb953908323ada36ac6a2ccf950097b5ebe5 differs from checkout tree 6ed12f63dac3baf3bb32df606a5d66fa1f182148; rebuild fixtures: `cargo run -p xtask -- build-test-fixtures`
Summary [   0.025s] 1 test run: 0 passed, 1 failed, 18 skipped
```

The refusal names both trees and the `build-test-fixtures` remedy. The harness does not rebuild and does not fall back to mtimes.

Note on an earlier attempt: the first rewrite re-serialised the whole manifest as pretty-printed JSON (`indent=2`). The consumer then refused it as `test fixture missing — malformed fixture manifest in /tmp/maos-plant-fixtures/manifest.json; build fixtures first: \`cargo run -p xtask -- build-test-fixtures\`` (exit 100, `fixtures.rs:57:5`). That is still a refusal, but it is not the stale-tree path. The harness's hand-rolled reader accepts only the producer's layout, so the plant was redone from the original bytes with the single value swap above.

## Cleanup

- `rm -rf /tmp/maos-plant-fixtures`, then confirmed it is absent.
- `git status --porcelain` was empty after the producer, the control, the plant and the cleanup. The nested probe build dir is ignored: `git check-ignore -v crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe/target` → `.gitignore:7:target`.
