# Epic 21 — Multi-Host on Live Substrate (W6)

**Status:** `backlog` — Round 2 correct-course 2026-09-04 (`sprint-change-proposal-2026-09-04-round2.md`, Lunarpulse-ratified; Fork C). Replaces the morning's Epic 20 file. `sprint-status.yaml` is authoritative for status.

**Wave / duration:** 5–7 weeks (measured) · **Makes true:** v1.5 J4 and v2.2 J3 + Reza exercised by the journeys they were built for; identity durable across restarts; one kernel instrument

**Closes:** J3/J4/Reza fixture-by-default; FR23b (part), NFR-Sec-13 own-leaf, NFR-Rel-6, NFR-Maint-1 instrument; D1, D4a/b/c, D11, D13, D14; the 63 unregistered env reads

**Hermetic exit command (CI, no secrets; the epic's first CI job, red until green):**

```
cargo run -p xtask -- demo-reza                       # already red on an absent DB; runs the 3-team journey --include-ignored
# nightly: zero #[ignore] journey tests without a named owner
```

**Operator-lane legs (never in the exit):**
- live J4 with a provider key (≥300 calls per run)

**Kernel-Δ:** ZERO kernel-Δ @24472 for 21-1..21-4; **21-5 is FLAG-Winston and negative** (about −1000 lines, re-pin in its commit).

**Confidence rules (ratified 2026-09-04 R2, binding):** 1 verb-first (exit verbs exist at HEAD or are AC1 in this epic; `check-exit-commands`) · 2 hermetic exit, live optional (paid/human/clock legs are operator-lane rows) · 3 foundations first · 4 cite-or-cut (every AC names file:line + kernel-Δ) · 5 spike before build · 6 size from measurement (+30%) · 7 one decision per fork · 8 inventory from the tree.

**Dev-gate:** external holds (NFR-Sec-7, NFR-Comp-1) = GA ledger only. **Model/review:** frontier allowlist + §A6 net binding; non-degradable on stories marked ◆.

---

## Stories

| Key | Story | Closes · Δ |
|---|---|---|
| `21-1-j4-incident-fixture-and-live-mira-nash` | J4 incident fixture and live Mira/Nash | J4 · NFR-Test-4 · kernel-Δ 0 |
| `21-2-nightly-postgres-and-ignored-journeys` | Nightly Postgres and the ignored journeys | J3/Reza · NFR-Scale-5 · kernel-Δ 0 |
| `21-3-durable-tofu-and-self-leaf-rotation` | Durable TOFU and self-leaf rotation | NFR-Sec-13/Rel-6 · RELEASE-HOLDS rows 12, 14-2(c) · kernel-Δ 0 |
| `21-4-one-instrument-and-env-registry` | One instrument and the env registry | D1/D4a/D4b/D4c/D11/D13/D14 · kernel-Δ 0 |
| `21-5-orchestrator-port-and-move` | Optional: orchestrator port and move ◆ FLAG-Winston (negative) | §4.0.7 naming · kernel-Δ ≈ −1000 |

AC1 is always the command and what the operator observes; every AC cites the code it changes; `bmad-create-story` may tighten but not add uncited ACs.

### 21-1-j4-incident-fixture-and-live-mira-nash — J4 incident fixture and live Mira/Nash

*Closes · Δ:* J4 · NFR-Test-4 · kernel-Δ 0

- **AC1 (command).** `MAOS_INFERENCE_MODE=replay maos run spirits/topologies/j4-mira-nash.toml --incidents tests/fixtures/j4/incidents-50.jsonl --once` resolves 50 incidents from recorded model turns and prints the close rate (≥45/50 asserted) under a compressed scenario clock.
- **AC2.** A prompt→`AnomalySignal` adapter feeds Mira (`spirits/mira/src/lib.rs:274` takes a signal, not a prompt); Mira and Nash gain inference seams; manifests carry live provenance instead of `maos.mira.deterministic-diagnostic-v1`.
- **AC3.** The J4 topology triggers the `handle_intake` deny path so `journey_j4.rs:117` is un-ignored.
- **AC4.** The 300-row κ corpus stays a separate hermetic leg (it is not the incident corpus).

### 21-2-nightly-postgres-and-ignored-journeys — Nightly Postgres and the ignored journeys

*Closes · Δ:* J3/Reza · NFR-Scale-5 · kernel-Δ 0

- **AC1 (command).** `journey-nightly.yml` provisions the three team DBs with `.github/actions/provision-loom-substrate` (exists; `discipline.yml:2930-2967`) and runs `-p maos-bin --test cross_team_crossing_13_6b` and `-p maos-a2a-tcp --test t_12_4a_digest_read` with `--run-ignored`; it reds on absence (no `services:` today).
- **AC2.** The J3 capture test (`t_12_4a:593`) is un-ignored and `journey_j3` consumes the captured mesh output, not `fixtures/j3/day-30-raw.json`.
- **AC3.** `check-reza-production-path`'s `tl-phase-b` leg is Blocking with the Postgres job (`check_reza_production_path.rs:457`).
- **AC4.** Zero `#[ignore]` journey tests without a named owner (`journey_butler.rs:285` → 18-4).

### 21-3-durable-tofu-and-self-leaf-rotation — Durable TOFU and self-leaf rotation

*Closes · Δ:* NFR-Sec-13/Rel-6 · RELEASE-HOLDS rows 12, 14-2(c) · kernel-Δ 0

- **AC1 (command).** A daemon restart preserves pins: `GET /v1/a2a/peer-versions` shows the rotated fingerprint after restart (durable `TofuPinStore` in `maos-persistence`, kept as `tofu.rs:90-93` names it; `TcpA2ATransport.pins` typed to the trait; a sync read path for `verify_pinned_sync`, `tofu.rs:457`).
- **AC2.** Own-leaf rotation is live: `swap_serving_cert` (`transport.rs:541`, in-place `ResolvesServerCert`) gains a production caller driven by a signed self-reissue; `T_grace` raised to cold+I with both 14-2a legs re-derived (`cert_rotation_trigger_14_2a.rs:720-732`, `cert_rotation_14_2a.rs:723-742`) — 14-2d's design record.
- **AC3.** The post-grace refusal carries a machine-readable token (`journal_peer_identity_refusal` gains a detail field, `router.rs:502-507`) per ADR-063.
- **AC4.** Listen-side per-peer scoping (`transport.rs:1341`).

### 21-4-one-instrument-and-env-registry — One instrument and the env registry

*Closes · Δ:* D1/D4a/D4b/D4c/D11/D13/D14 · kernel-Δ 0

- **AC1 (command).** `check-kernel-baseline` and `kloc-check` read one instrument (tokei code); `kernel-core-baseline.toml` parses as TOML (invalid at line 29 today); the ≤25K "kernel-crate-set" ceiling is authored as `xtask/kernel-crate-set.toml` + ADR-057 amendment, or its citation retired.
- **AC2.** `check_env_contract.rs:119` scans the workspace; the registry moves to `maos-domain` with its 15-2 allowance; the 63 unregistered `MAOS_*` reads (16 names in kernel-core) are registered without a kernel edit; `EnvStability::Secret` classifies provider keys (D4c).
- **AC3.** `MAOS_REGION_HOME` is reconciled against `TeamEntry.region` at daemon boot (D4a; reads at `maos-domain/region.rs:84-324`, `operator_config.rs:177-349`).
- **AC4.** `maos-persistence` is retained (21-3); the three phantom `kloc.toml` budgets are deleted; the "consolidate 19 canonical / 10 SigningKey" claim is dropped (17 functions in distinct domains, 6 keys).

### 21-5-orchestrator-port-and-move — Optional: orchestrator port and move ◆ FLAG-Winston (negative)

*Closes · Δ:* §4.0.7 naming · kernel-Δ ≈ −1000

- **AC1 (command).** `crates/maos-kernel-core/src/orchestrator/` (5 files, 1028 LOC) leaves kernel-core behind a `maos-domain` port; `scheduler_loop.rs:89,111` hold the port; the `main.rs` users (`:1995,5696,5788,5859`) updated; the pin re-pinned negative and `check-cohort-mesh` (which re-runs it, `:604`) green.

## Dependencies

Epic 20 (20-3 evidence states for the nightly; 20-2 packages for multi-host runbooks). 17-3b before 21-2.
