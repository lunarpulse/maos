# 20-3c creation measurements (2026-10-31, HEAD `3341f9e4`)

No cargo ran (parent rule: inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos`, no build, no test, no override). No
crate source in the tree was modified: every prototype was made on a `git archive HEAD` copy under
`/tmp/proto203c/<name>/` (deleted after measurement), `rustfmt --edition 2021` (1.9.0-stable) run on every changed
file, then measured with the prebuilt gate binary **from that copy's root**:
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto203c/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
(tokei 14.0.0 inside `kloc-check`). Every `.diff` here passes `git apply --check` against the working tree. Raw
results: `kloc-kernel-3341f9e4-and-prototypes.json`. Prototypes were not compiled ([INFERENCE] that they build).

## Instrument replication

| Figure | Method | In-tree and `/tmp` copy `base` | Pin |
|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26793**, 95 files, set hash `a7719696…`, changed/added/removed `[]`, rc 0 | `xtask/kernel-core-baseline.toml:505,915` |
| Kernel tokei | `kloc-check --json` | **20992** | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed=true`, `alarm=true`, `over_budget=[]`, rc 0) | maos-bin **33030** · maos-cli **6371** · maos-audit **7707** · maos-capability **1432** · maos-domain **9977** · maos-iac **8270** · maos-shell **602** · maos-a2a-tcp **1442** · maos-control **1496** · xtask **42841** · aggregate **188451** | `:517`, `:489` (6856 ceiling; E19 STOP 6382), `:409`, `:265`, `:425`, `:406`, `:691`, `:475` (1621), `:484` (1791; E19 STOP 1539), `:339`; `_aggregate_hardfail` `:718` = 188452 |

## Kernel-touching options, priced by throwaway prototype

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Also |
|---|---|---|---:|---:|---|
| K1 | Carry, don't drain: `validate_swap_halt_continuity` reads the predecessor's own pending halts via `drain_for_spirit_dry_run`, validates them, returns `SafeMigrated{n}` (or `SafeDrained{0}` when none); doc comment rewritten (D-20-3c-01 B) | `k1-carry-not-drain.diff` | 26793 → **26778 (−15)** | 20992 → **20993 (+1)** | pin `halt/mod.rs` |
| K1p | K1 + precheck counts and validates the predecessor's carried set, not every Spirit's (`precheck.rs:39,50-60`) (D-20-3c-01 A) | `k1p-carry-and-precheck.diff` | **26785 (−8)** | **20999 (+7)** | pins `halt/mod.rs`, `hot_swap/precheck.rs` |
| K2 | Drain with a typed receipt: coordinator calls `terminate_spirit(…, TerminationKind::SwapDrained, …)` before the I14 gate when the predecessor holds pending halts (D-20-3c-01 C) | `k2-drain-with-swap-receipt.diff` | **26808 (+15)** | **21006 (+14)** | **maos-domain +2** (variant + `as_str`); pin `hot_swap/coordinator.rs`; FR53 witness stays red |
| E1 | EC5 counted drop: no-runtime and failed delivery `record_drop(AuditDropSite::RateLimited, Closed)` (D-20-3c-02 B) | `e1-ratelimited-counted-drop.diff` | **26800 (+7)** | **20998 (+6)** | **maos-capability +1** (`RateLimited` site, `COUNT` 8 → 9) |
| E2 | EC5 synchronous journal: no runtime ⇒ `transparency_log.insert_frame_event(FrameKind::RateLimited, 0, …, FrameOrigin::Kernel)` (journaled, not delivered) (D-20-3c-02 C) | `e2-ratelimited-sync-journal.diff` | **26807 (+14)** | **21005 (+13)** | — |
| E3 | E2 + failed spawned delivery counted (`RateLimited` drop site) (D-20-3c-02 A) | `e3-ratelimited-journal-or-count.diff` | **26813 (+20)** | **21011 (+19)** | maos-capability +1 |
| **R** | **K1p + E3** (recommended) | `krec-k1p-e3-bundle.diff` | **26805 (+12)** | **21018 (+26)** | maos-capability +1; additive (= +7 + +19) |
| R′ | K1p + E1 | `kalt-k1p-e1-bundle.diff` | **26792 (−1)** | **21005 (+13)** | maos-capability +1 |

ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): R at 21018 tokei is 982 under the 22000 alarm; the
envelope is not headroom — R needs a FLAG-Winston grant row. Any option re-pins both kernel pins and the per-file
content hashes of the changed files (`xtask/kernel-core-baseline.toml:27-28` + EOF). No kernel pub signature changes
in K1/K1p/E1–E3 (K1p renames the unused `_predecessor_pid` parameter of `HotSwapPrecheck::check`; whether the surface
baseline hashes parameter names is [INFERENCE] — `check-service-boundary` at T0 decides). K2 adds a public variant to
`maos_domain::halt::TerminationKind`; exhaustive matches over it were not searched ([INFERENCE]).

Tests that assert the drain this story removes (kloc-free, rewritten with K1/K1p):
`crates/maos-kernel-core/tests/hot_swap_halt_continuity_corpus_integration.rs:90`
(`swap_continuity_with_seeded_halts_returns_safe_drained_nonzero`), `:140`
(`chain_walk_drains_a_real_pending_halt_at_every_hop_with_zero_dropped`); `hot_swap/precheck.rs:111` (inline unit test; whether inline
test lines count against the kernel pins was not checked) only if its seeded set changes.

## Out-of-kernel, priced by prototype

| Prototype (D-row) | Diff | Measured |
|---|---|---:|
| Member 17: `delegate(…, emitter_pid)` from `main.rs:4908`; `journal_completion(…, worker_pid)` with `worker_run.spirit_pid` (`main.rs:5394,5485`); host-B intake and outcome stay 0 (D-20-3c-03 A) | `p17-sender-pid-attribution.diff` | **maos-bin +2** (33030 → 33032); kernel 0 |
| All measured (R + member 17) | `kall` (= `krec` + `p17`) | kernel +12 / +26, maos-capability +1, maos-bin +2, **aggregate +29** (188480) |

## Runtime measurements at HEAD (prebuilt binaries, no cargo)

Binary: `/mnt/build/cargo-maos/debug/maos`, mtime 2026-10-31 20:21:22Z, built from the tree committed as `714a1428`
(20:22:25Z); `git log -1 -- crates` = `714a1428`, so the binary matches HEAD's crates [INFERENCE: mtime, not a
build-id comparison]. Isolated `HOME`/`MAOS_HOME`/`XDG_*` per run under `/tmp/c203c` (deleted).

| What | Command | Result |
|---|---|---|
| Member 11 | `runtime/oneshot-drain.sh` (3 runs per arm; `env -i`, isolated home; hello-spirit from a copied `spirits/hello-spirit`) | smoke-epic-4 rc 0, 5212/5236/5234 ms; smoke-spirit-5 rc 0, 5228/5218/5283 ms; smoke-supervision-5 rc 0, 6259/6271/6219 ms (its synthetic panic `main.rs:7684` is by design); hello-spirit rc 0, 5298/5310/5305 ms — **every run prints `audit writer drain timed out after 5s` then "complete"/"exiting cleanly"**. `maos shell --plain </dev/null`: rc 0, 304/335 ms, no timeout line. TL rows after hello-spirit: 4 (3/3), smoke-epic-4: 6 (3/3) — stable, so the rows land and the drain fails to observe completion (a live sender clone) [INFERENCE]. `runtime/oneshot-drain-3341f9e4.txt` |
| Member 16 | `runtime/endpoint-reproduction.py` | `maos init --plain` rc 0 records `127.0.0.1:<port>`; a foreign Python bind of that port **succeeds** (init holds nothing); `maos shell --plain` rc **78**: `operator endpoint … is already served by another MAOS root (…/control.json) — EndpointInUse`; holder released ⇒ rc 0, `operator HTTP listening on` the same port. Two independent reproductions (ports 38083, 40215). `runtime/endpoint-reproduction-3341f9e4.json` |

## Not measured here (owed to the dev pass)

1. Compilation of every prototype (`cargo check --locked -p maos-kernel-core -p maos-capability -p maos-domain -p maos-bin`).
2. The FR53 witness red at HEAD: `cargo test -p maos-kernel-core --test routed_halt_swap_20_3b -- --ignored` (red per the 20-3b record; not re-run).
3. `maos audit query --spirit recall-test` on the 17-3d root (D:1194's `Fr4SchemaViolation`; recorded, not re-run).
4. Whether `emit_rate_limited_frame`'s no-runtime branch is reachable from a production caller (EC5 reachability).
5. Out-of-kernel adds other than member 17 (forecast only, story §Kernel and kloc pricing).
6. The NFR-Ops-9 cadence ledger contents (`rpo-rto-cadence.yml:115-140`, `rto-evidence.toml` branch) — not inspected.
