# 20-14 creation — epic AC1 probe at HEAD (2026-11-03)

Binary: prebuilt `/mnt/build/cargo-maos/debug/{maos,maosctl}` (11:17 / 11:21 UTC). They were built from the working
tree, which carries a sibling's uncommitted 20-4 L1 edits; every file on the cost path is byte-identical to `8c809f9c`
(`git diff --stat HEAD -- crates/maos-kernel-core crates/maos-domain/src/cost.rs crates/maos-domain/src/ports/inference.rs crates/maos-cli crates/maos-iac crates/maos-audit` = empty). No cargo ran.

Isolated home under `/tmp/proto2014/rt.*` (deleted after the copy), from the repository root:

```bash
export HOME=$T/home XDG_DATA_HOME=$T/home/.local/share XDG_CONFIG_HOME=$T/home/.config XDG_STATE_HOME=$T/home/.local/state
maos init                                                                   # rc 0
for p in alice bob; do
  printf '@hello-spirit hello\n' | MAOS_SESSION_PRINCIPAL=$p MAOS_INFERENCE_MODE=replay \
    MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j0/shell-intro.json maos shell   # rc 0, rc 0
done
maosctl audit cost-reconcile --month 2026-11 --format ndjson --pricing xtask/provider-pricing.toml   # rc 0
maosctl audit cost-reconcile --month 2026-11 --format plain  --pricing xtask/provider-pricing.toml   # rc 0
```

Observed (`reconcile-head.ndjson`, `reconcile-head.txt`, `tl-rows-head.txt`):

- Two boots, two turns, two cost frames (50 in / 30 out each), both `principal: Unattributed`,
  `attribution_source: write_target_proxy`, no task field — `MAOS_SESSION_PRINCIPAL` is read by nothing.
- **One** report row: `host-unallocated`, `spirit_pid 1`, `replay / hand-authored-seed`, **100 in / 60 out** — the two
  sessions are pooled because both boots admitted hello-spirit at pid 1 and the report keys on the boot-local pid.
- Both boots write a `lifecycle.load` row naming `hello-spirit` at pid 1 (kind 7) — the `(boot_nonce, pid) → name`
  mapping D-20-14-05 A reads exists for the shell path (measurement 5 of `measurements.md`: answered for the shell;
  the seam Spirits remain owed).
- `cost_micro` is 0: `xtask/provider-pricing.toml` has no `(replay, hand-authored-seed)` row, so AC1 asserts tokens and
  rows, not money or `attributable_fraction` (which is computed over cost and reads 0.0 on a zero-price window).
