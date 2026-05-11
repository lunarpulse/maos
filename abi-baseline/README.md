# ABI Baseline

This directory stores public-API snapshots of `crates/maos-spirit-abi` used by the `abi-diff` CI gate (AC4).

## Format

Each baseline is a JSON file produced by the `cargo xtask abi-diff` command. The file contains:

- `abi_version`: the current ABI version integer
- `items`: a sorted array of public API items, each with `kind`, `name`, and `signature`

## Update Procedure

1. When the ABI surface changes in a way that requires a version bump, update `ABI_VERSION` in `crates/maos-spirit-abi/src/lib.rs` (or `src/version.rs` post-Story-1a.1).
2. Run `cargo xtask abi-diff --base <previous-tag>` to verify the diff passes.
3. After merge, generate a new baseline snapshot and commit it here with the tag name.

## Baselines

- `v0.1-alpha-pre-abi-freeze.json` — initial snapshot from Story 0.1's placeholder ABI surface.
