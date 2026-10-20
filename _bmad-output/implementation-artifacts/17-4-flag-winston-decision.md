# 17-4 FLAG-Winston decision record

Story: `17-4-hooks-observer-and-activity-corpus` · HEAD `c49032a1` · ruled 2026-10-19 by Lunarpulse (operator) at story creation.

## Question

The Observer corpus floor (`prd/project-scoping-phased-development.md:145`) requires causal-ordering correctness ≥ 99 %. The Telemetry Stream has one broadcast channel per scalar tag (`crates/maos-kernel-core/src/telemetry/mod.rs:39-40,92-126`) and `ScalarTapEvent` carries no sequence, only a millisecond timestamp (`crates/maos-domain/src/invariants/i7.rs:43-54`; `crates/maos-capability/src/working_memory/store.rs:64-78`). A kernel-Δ 0 pump therefore cannot recover one emitter's publication order across tags (`A1,B1,A2,B2` can arrive `A1,A2,B1,B2`), and timestamps tie under bursts. Both non-author validators rated this a blocker.

| Option | Effect |
|---|---|
| **Publish-order `seq` (chosen)** | `seq: u64` on `ScalarTapEvent`, assigned per `WorkingMemoryStore` under its write lock; the pump releases in `seq` order; corpus identity = `seq`; every same-emitter program-order pair (cross-tag included) gated at ≥ 99 %. |
| Per-stream order, kernel-Δ 0 | Gate only same-(emitter, tag) pairs; cross-tag order reported, ungated — narrows the PRD metric. Declined. |

## Ruling D-17-4-FW-01

- **Charter:** kernel-core edits are exactly the two `ScalarTapEvent` struct literals in `crates/maos-kernel-core/src/telemetry/mod.rs`'s test module (`:175`, `:199`), each gaining the `seq` field. Production kernel code is unchanged: `CapabilityRegistryAdapter::set_scalar` (`capability/mod.rs:220-234`) passes the store's event through. Any other kernel-core/src edit returns as a charter request.
- **Grant:** ≤ +2 physical / ≤ +2 tokei — `src_lines` 26291 → ≤ 26293 (`xtask/kernel-core-baseline.toml:505`), kernel kloc 20527 → ≤ 20529 (`xtask/kloc.toml:263`), re-pinned exact at landing. Measured by construction (two one-line field insertions in multi-line literals; rustfmt keeps one field per line); the landing measurement after `cargo fmt --all` is authoritative.
- **Out-of-kernel rows** (`maos-domain`, `maos-capability`, `maos-bin`, `_aggregate_hardfail`) are not granted here: each is a measured request approved before landing (`xtask/kloc.toml:60-69`).
- **Re-pin set:** `kernel-core-baseline.toml` (`src_lines`, `telemetry/mod.rs` digest, `set_hash`, HISTORY), `xtask/tests/kernel_pin_content_hash_16_0.rs` live `26291` literals (`:820,838,847,858`), `xtask/kloc.toml:263`, `xtask/tests/recovery_lane_ceiling_rule.rs:268`. Kernel surface and API classes unchanged.
- **Review:** ◆ — the non-author §A6 net is non-degradable for this story.

## Preflight rulings (2026-10-19, party mode; operator Lunarpulse)

Four independent reviewers re-read the contract at `c49032a1`; the transport reviewer measured an isolated prototype (`17-4-evidence/preflight/transport-measurements.md`, patch `transport-firehose-prototype.patch`).

### D-17-4-PF-R1 — broad channel (amends FW-01's charter and grant)

- **Why:** the per-topic receivers plus a time-hold reorder buffer only made one emitter's cross-tag order likely — a forwarder stalled past the hold inverts irreparably — and subscribing only `[epistemic_policy]` tags hid every undeclared write (the store accepts any nonempty tag, `crates/maos-capability/src/working_memory/store.rs:43-62`).
- **Ruling:** `TelemetryStreamAdapter` gains one broad broadcast channel (existing capacity, shared by clones); `publish_event` sends to it first, cloning only when it has receivers; `pub fn subscribe_all(&self) -> broadcast::Receiver<ScalarTapEvent>`. Exact-topic channels and `drop_count` unchanged. FW-01's `seq` stays as a store-write-order identity label — never a reorder key.
- **Charter:** `crates/maos-kernel-core/src/telemetry/mod.rs` only. **Grant:** physical 26291 → ≤ 26304, tokei 20527 → ≤ 20538 (prototype exactly 26304 / 20538, including FW-01's two literals), exact at landing. Supersedes FW-01's ≤ 26293 / ≤ 20529.
- **Surface:** `check-service-boundary` hashes the struct including private fields, so `docs/ci-baselines/kernel-surface-v0.1-beta.json` is regenerated for `TelemetryStreamAdapter`'s two paths (`:1027`, `:1167`) and `xtask/kernel-api-classes.toml` gains `TelemetryStreamAdapter::subscribe_all = "data-movement"`. The re-pin set above gains these two artifacts.
- **Measured:** 1 500 alternating A/B writes from one emitter received in exact order; a capacity-2 receiver reported `Lagged(3)` once; `cargo check` of kernel/domain/capability/observer/researcher PASS; `--lib telemetry` 11 passed.
- **Out-of-kernel (not granted here):** `maos-domain` prototype +3 tokei against a zero-headroom pin; `maos-capability` +4 within headroom.

### D-17-4-PF-R2 — coverage matrix

AC5.3's "zero violations" was unmeetable (unrelated rows name unregistered gates). Ruling: record the violation set at the dev HEAD; 17-4 adds none and its FR55 / NFR-Obs-3 rows are clean. Existing debt stays with `20-3b-gate-retirement-and-coverage-generator`.

## Review ruling (2026-10-20, operator Lunarpulse)

### D-17-4-CR-01 — output-thread-only detach

- **Decision:** operator selected **“Allow output-thread detach only”** rather than a terminable helper process or Linux-only output. This explicitly amends AC2.9 and the blanket no-detach guardrail; it does not relax hook-task joins.
- **Contract:** stop accepting activity records at shutdown; use ordinary locked stdio without changing inherited descriptor flags; allow 250 ms for the output thread to drain, then detach only that thread if its output remains blocked. It owns no scheduler, Spirit or audit handles. Telemetry and consolidation tasks still join before unload and audit drain.
- **Accounting:** overflow and write errors are counted; diagnostics run only on the output thread. `shutdown_abandoned` counts records unconfirmed at detach, not proven losses: an in-flight write can still finish. `detached` exposes the exception. No new records are written after the worker observes abandonment. The optional final terminal notice is abandoned too when output detached, because `2>&1` may alias the blocked pipe.
- **Rejected implementation:** temporary nonblocking flags on inherited stdout/stderr cannot safely handle aliased descriptors and stdio-lock contention. That implementation was removed, not retained as a fallback.
- **Scope:** no new kernel change or grant. Any increased out-of-kernel ceiling requires its own measured operator approval.

### D-17-4-CR-02 — exact measured review ceilings

- **Approved by Lunarpulse, 2026-10-20:** after `cargo fmt --all` and `cargo run --locked -p xtask -- kloc-check --json`, while the story was still in review, the operator selected **“Approve exact pins.”**
- **Grant:** `maos-bin` 27194 → **27323** (+129); aggregate measured 178496 → **178625**, so `_aggregate_hardfail` 178497 → **178626** (+129, strict measured +1). No speculative headroom.
- **Driver:** review fixes R1–R8: output-thread isolation and bounded shutdown, finite/cancellable flush and unread-position accounting, exact-runtime dispatch views, generated class enumeration, shared serving shutdown ordering, and terminal-output backpressure handling.
- **Kernel unchanged:** 26304 physical lines / 20538 tokei / 95 files; no new kernel grant.
