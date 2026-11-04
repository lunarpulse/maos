# CID-file lifecycle and adjacent cleanup scan

Baseline: `73e7afbcae81f4fb4858e9d52ded3d592bed636d`.

This is the custody-only source-line/count snapshot before Main added the
enumeration and S5 items. Final combined proof is in
[`combined-plants.md`](combined-plants.md); historical line numbers/counts below
are intentionally retained.

## Cause and authority

[Podman v4.9.3's cidfile contract](https://github.com/containers/podman/blob/v4.9.3/docs/source/markdown/options/cidfile.write.md), read directly from its raw primary source, says the file is removed along with the container (with a remote detached-run exception). Hosted run41803510235 confirms container removal succeeds before custody fails on both heads and the restored plant. The historical hosted log did not directly show the missing file; that causal interpretation remains source-supported inference for those old logs. The new hermetic stub actually deletes each corresponding cidfile before returning runtime absence, and reproduces the custody failure on original production code.

`purge_runtime.rs:65-143` collects runtime/state inventory before removal; container removal still precedes custody reclamation. Inventory now stores each valid spawn CID with its held no-follow directory descriptor (`:496-539`). Capture validates the held directory's euid ownership and exact0700 mode, and parses a regular no-follow `cid` read. Reclamation (`:542-574`) never reopens the CID file: it requires captured authority, a fresh bounded `container exists` exit1, and matching held/current device+inode with renewed ownership/mode checks before clearing contents. Final `rmdir_held` checks the path against that held directory again. Missing/invalid-at-inventory CID means no authority, not permission to remove an empty/missing-file spawn.

## Other consumers

A scoped grep over all `crates/maos-bin/src` for `cidfile|join("cid")|read[^\\n]*\\bcid\\b|cid_path` found only the inventory CID read in `purge_runtime.rs:520`. There is no second post-removal CID-file reader in maos-bin.

- `egress.rs:35-76` stops/joins proxies and attempts only an empty-root `remove_dir`; it does not recover a CID from disk.
- `egress.rs:240-315` allocates volatile Worker-state process/spawn directories, validates ownership/mode, and preserves compact durable socket names. It does not reclaim a spawn based on CID-file disappearance.
- `supervision.rs:2029-2041` uses only `remove_dir` for process-root cleanup and explicitly preserves non-empty state. Allocation at2221-2223 delegates to the same egress allocator. No additional cidfile-lifetime assumption was found.
- No kernel file, producer path shape, socket name, environment variable or fixture registration changed. The existing hosted-length egress vector was included in the green five-test egress suite.

ADR-067 and canonical/site English plus Korean run/cookbook paragraphs now distinguish an inventory-captured CID from a merely missing file after removal. No receipt schema, disposition vocabulary, live/unknown creator rule, shared-parent rule or same-UID trust-boundary change.

## Witness boundaries

The strengthened original AC5 witness retains its present-CID, unknown-CID, legacy-nonce, live-creator and receipt assertions. It adds a container whose stub removal really unlinks `cid`, while `pid` still exists. Two new negative vectors cover missing CID at inventory (retained and receipted) and path replacement during `rm` (both original held and replacement inodes retain their data).

The real hosted custody assertion now prints purge stdout/stderr, receipt-read result and directory listing on failure. This additional hosted diagnostic has not yet executed on a corrected hosted run.

The branch-only cut keeps both baseline required diagnostics and the removal plant diagnostic unchanged. Its existing no-op removal patch still passes `git apply --check`. Restored green now uses the shared exact-count1/required-PASS checker instead of rejecting33 filter exclusions; the new executed workflow test accepts a selected PASS with33 excluded tests and rejects explicit SKIP, a wrong required name, and zero selected tests. Hermetic expected count is19 (14 purge +5 egress); terminal count remains34.
