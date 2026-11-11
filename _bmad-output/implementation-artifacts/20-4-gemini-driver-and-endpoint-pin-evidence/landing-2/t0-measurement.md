# 20-4 L2 — T0 re-measurement of kernel row R (2026-11-11)

Base: `89b64566` (`git archive HEAD` extracted to `/tmp/t204l2`, no cargo). `git log a8af3215..HEAD -- crates/maos-kernel-core/src`
is empty: the kernel sources are byte-identical to the creation base, so the creation prototype
(`creation/r-ka-kio1-kio2-bundle.diff`, never compiled then) applied with `patch -p1` without fuzz or offsets and was
re-formatted with `rustfmt --edition 2021`.

Instrument: the prebuilt `/mnt/build/cargo-maos/debug/xtask`, `CARGO_MANIFEST_DIR=/tmp/t204l2/xtask`, from the copy root:

| Measure | Pin at HEAD | Prototype R | Delta | Grant / ceiling |
|---|---:|---:|---:|---|
| kernel physical (`check-kernel-baseline --json`, `actual-lines`) | 26841 | **26854** | +13 | +13 / 26867 |
| kernel tokei (`kloc-check --json`, `maos-kernel-core`) | 21042 | **21051** | +9 | +9 / 21060 |
| aggregate tokei | 202112 | 202121 | +9 | — (`_aggregate_hardfail` → 202122) |
| kernel file set | 95 / `db39bc4a…` | 95 / `2c223219…`; changed `inference/mod.rs`, `io/mod.rs`; added/removed `[]` | | |

Result: **exactly the granted shape; no STOP**. Cumulative kernel 21051 < shared STOP 21250 < ADR-069 alarm 22000.
Raw: `t0-kernel-baseline-prototype.json`, `t0-kloc-prototype.json` (both rc 1 because the unmodified pins were
compared, as expected; the figures are the measurement).

The repo edit applied the same diff; the in-tree formatted tree reproduces the prototype's set hash exactly
(`2c22321948fb89df25f58b4abcb54e3ff81bd2c5ffa0c8f8429d27562164375b`), i.e. the landed bytes equal the priced bytes.

Hermetic-witness measurements owed by the story (creation `measurements.md`, "Not measured here" 1–2):

1. Compilation against locked ureq 2.12.1: `ureq::Error::Transport(t)` with `t.kind() == ureq::ErrorKind::BadHeader`
   compiles and fires (the AC8.1 witness is green after, red before).
2. Whether ureq validates a header before connecting was not isolated: the witness binds a loopback listener that never
   accepts, so a connect (kernel backlog) succeeds either way; the `BadHeader` error is returned without any
   response, and the red-before message (`Bad Header: invalid header 'Authorization: Bearer sk-secrettail'`,
   `logs/red-before-head.log`) shows the failure is the header, not a connection refusal.
