# Cleanup

After the interrupted prepush, a one-time `/proc` process scan found no `cargo`, `cargo-nextest`, `rustc`, `rustdoc`, `clippy-driver` or `xtask` process. Removed owned `/tmp/20-8-*` plant copies, both metadata files, the hash-pinned verifier virtualenv, and `/tmp/208-wip`: 1,146 files, 31,340,782 logical bytes / 34,000,896 allocated bytes unlinked (allocated figure not adjusted for hard links). No target `deps`, `build`, or fingerprint files were touched.

Stale incremental sessions were **not pruned**: `/proc/1/fd` and `/proc/7/fd` were permission-denied, so the required absence of open handles cannot be established for every process. A sample `fuser -v` on one session returned 1/no visible users, but this is not proof against the inaccessible processes. This is a safety limitation, not a claim that all stale sessions were cleaned.
