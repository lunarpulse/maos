# 17-1 FLAG-Winston decision record

**Date:** 2026-10-06  
**Status:** `ratified` 2026-10-06 by the operator (Lunarpulse) — FW-01…FW-04 as below, with variant **B** and the aggregate raise exact at landing (≤ +1182). Proposed earlier the same day.  
**Scope:** story creation and a **throwaway measurement prototype only**. At creation this record changed no runtime, kernel, baseline, KLOC ledger, ratification file, ADR or model allowlist, and the story stayed `backlog`; the operator's ruling (§Disposition) then wrote the ADR-061 amendment and moved the story to `ready-for-dev`. The kernel and KLOC figures move only in 17-1's landing commit.  
**Model after:** `17-3c-early-start-flag-winston-decision.md` (a FLAG-Winston is an architectural *location charter*, not a numerical grant; D-17-3c-ES-04).

## Decision

| ID | Decision | Status |
|---|---|---|
| **D-17-1-FW-01** | The kernel **locations** for 17-1 are `lifecycle/cli_wrapper/runtime.rs`, `security/sandbox/t3/spawn.rs`, `t3/argv.rs`, `lifecycle/cli_wrapper/admission.rs` and — by the measurement — `lifecycle/cli_wrapper/lifecycle.rs` (a test struct literal, +1). `security/manifest.rs`, named by the epic, is a 9-line `pub use maos_manifest::*` shim and carries **0**. | **Ratified 2026-10-06** — a charter, not a number |
| **D-17-1-FW-02** | The **numerical ceiling**: variant **B** (delete the dead `ci_default_guard`) — `src_lines` 25293 → **25353** (+60), kloc 19529 → **19564**; or variant **A** (keep it) — 25425 (+132), 19621. The landing commit re-measures after `cargo fmt --all` and re-pins the exact figure; it may not exceed the ratified one without a new request. | **Ratified 2026-10-06: variant B** (≤ 25353 / ≤ 19564) |
| **D-17-1-FW-03** | Out-of-kernel ceilings: `maos-domain` 9270 → **9274** (measured); `maos-manifest` **no raise** (4445 measured, +29 free); `maos-bin`, `maos-egress` (phantom row 780 → exact), `xtask` — exact measured raises **at landing**; **`_aggregate_hardfail` 170884** — the aggregate has **+450** free against an envelope of **+585…+1182**, so an exact raise at landing is part of the request. | **Ratified 2026-10-06:** exact at landing; `_aggregate_hardfail` raise ≤ +1182 |
| **D-17-1-FW-04** | **ADR-061 M3 amendment** (text below): transport by a daemon-owned Unix socket bind-mounted into a `--network=none` container; *on host loopback* superseded; bearer daemon-minted; stale cites corrected. | **Ratified 2026-10-06** — written as `## Amendment — Story 17-1 (transport; ratified 2026-10-06)`, body-anchored in `xtask/tests/decision_adrs_and_provisioning.rs` |
| **D-17-1-FW-05** | Story status: **`backlog`** until FW-02…FW-04 are ruled and Q1–Q7 of the story are answered. The parent moves it to `ready-for-dev` (17-6 precedent, `f86b5d4f`). | **Done 2026-10-06** — `ready-for-dev` |
| **D-17-1-FW-06** | Optional further kernel offset, **unmeasured**: `t3/cap_audit_bridge.rs` (35 lines) and `t3/quarantine.rs` (73 lines) have zero callers (R17-83). Not taken: they are not this story's code. | Named only |

## Evidence and boundary

Four read-only scouts re-measured every cite of epic §17-1, the header and §R4–§R8 at `0d73cac3` (kernel-core; maos-bin + spirits + tests; CI + xtask + governance + schema; docs + ADR + deferred-work + precedent stories). Their findings are filed as epic §R9 (R17-70…R17-89) and as the story's P1–P22. The two facts that decided the kernel figure:

- **The seam is `spawn_child`** (`runtime.rs:513-532`, 17-6's): `T0 | T3 => cmd.spawn()` bare at `:523-526`. `SpawnedBridge.child` is `Option<BridgeChild>` (`Plain`/`Sandboxed`), so T3 needs a third variant wrapping `SandboxedContainerChild` (R17-5 re-derived); `BridgeSpawnSpec` carries neither the verified image nor a `T3SpawnContext`.
- **Under T3, `env_clear()` on the `Command` clears the podman client**, not the container (podman never forwards the client environment): the container's environment is `--env=NAME` flags with values on the client's environment (never `NAME=VALUE` — argv is readable). `env_clear()` still applies at the one `Command` build for every other tier, which closes the 17-3c deferred row.

## Isolated executable measurement (2026-10-06)

**Method.** Detached worktree at `0d73cac3`; the kernel-side changes written to compile (`cargo check -p maos-kernel-core --lib`; then `maos-bin`, `maos-bench` and `xtask`, which depend on `BridgeSpawnSpec`, with mechanical `t3: None` / `egress: vec![]` scaffolding), `cargo fmt --all`, then `check-kernel-baseline` (physical lines — the pinned number) and `kloc-check --json` (tokei 14.0.0 — the budgeted number). All prototype code was **reverted** with `git checkout`; the final tree changes only story, epic, tracker, deferred-work and evidence files. Evidence in `17-1-evidence/`: `prototype.diff` (variant A), `prototype-with-dead-guard-deleted.diff` (variant B, the whole prototype), `measurements.json`.

| Counter | Base | Variant A | Δ | Variant B | Δ |
|---|---:|---:|---:|---:|---:|
| Kernel `src/` physical lines | 25,293 | 25,425 | **+132** | 25,353 | **+60** |
| Kernel tokei (`kloc-check`) | 19,519 | 19,621 | **+102** | 19,564 | **+45** |
| Kernel files | 98 | 98 | 0 | 98 | 0 |
| `kloc.toml` ceiling / raise needed | 19,529 | — | +92 | — | +35 |
| `maos-domain` tokei | 9,270 | 9,274 | +4 | 9,274 | +4 |
| `maos-manifest` tokei | 4,445 | 4,447 | +2 | 4,447 | +2 |
| Aggregate tokei (with scaffolding) | 170,434 | 170,549 | +115 | 170,492 | +58 |

| Kernel file (`src_lines`, net) | A | B |
|---|---:|---:|
| `lifecycle/cli_wrapper/runtime.rs` | +67 | −4 |
| `lifecycle/cli_wrapper/admission.rs` | +27 | +27 |
| `security/sandbox/t3/spawn.rs` | +23 | +23 |
| `security/sandbox/t3/argv.rs` | +14 | +14 |
| `lifecycle/cli_wrapper/lifecycle.rs` | +1 | +1 |
| `lifecycle/cli_wrapper/mod.rs` | 0 | −1 |
| `security/manifest.rs` | 0 | 0 |

**What the figure contains:** the T3 argv extras (`--volume=…:/proxy/egress.sock:rw`, `--http-proxy=false`, `--env=NAME`); piped `spawn_t3` with a client environment allowlist and the guard built *before* `inspect_container_host_pid`; `BridgeChild::Container` with a `kill` that also runs `<runtime> kill`; `T3Launch` and `BridgeSpawnSpec.t3` (a T3 spec without it is `SandboxRefused`, never bare); `env_clear()` at the `Command` build; a bounded `read_newline_delimited` (the 17-3c deferred row); a stderr-marker kind-8 branch in `pump_to_journal` (T3, stderr only, confirmed write); `check_egress` + two parameters on `admit_cli_wrapper_journaled` + a probe `env_clear()`.

**What it does not contain (the dev adds, and the landing figure is re-measured):** `#[cfg(unix)]` on the pump branch (+1); any test placed under `src/` (put new tests under `tests/`); a bounded post-exit drain if the R17-22 measurement under `spawn_t3` shows the defect does not dissolve; surface-pin rows. The prototype's admission doc-comment sits above `check_egress` (should sit above `admit_cli_wrapper_journaled`) and `T3SpawnContext.env` trips `check-service-boundary`'s P3 — both are measurement artifacts, fixed when re-derived.

**Gates on the prototype tree** (variant B): `check-kernel-baseline`, `kloc-check`, `check-fkcs` (reads the same pin), `check-service-boundary` (unclassified new public symbols + P3) fail — each **only** because of the pending grant / surface pin; `check-exit-commands` and `gen-abi-docs` also failed on the first run — an environment fault of the measuring shell (`CARGO_TARGET_DIR=/mnt/build/cargo` was exported, so the `target/debug/{maos,maosctl}` and `target/doc/*.json` those gates read were outside the worktree's `target/`), not of the prototype; the other eight pass. **On the final docs-only tree (own target, variable unset) all 14 gates and `cargo fmt --all --check` pass and `cargo test --offline --locked -p xtask` reports 941 passed.**

**Scratch proof that the grant is what is missing** (variant B applied, then reverted): `src_lines = 25353` plus the `--emit-pin` `[kernel_src]` block spliced into a scratch copy → `check-kernel-baseline: PASSED (maos-kernel-core/src = 25353 lines, 98 files, pinned 25353)`; scratch `kloc.toml` kernel 19564 / domain 9274 / bin 24605 → `kloc-check --json` `passed: true`, `over_budget: []`. Not exercised in the scratch (it is the landing commit's work, listed in the story): the four `25293` literals in `kernel_pin_content_hash_16_0.rs`, `RATIFIED_AT_EASING`, the surface pin.

## Proposed ADR-061 amendment (for ratification; to be written under `## Amendment — Story 17-1 (transport; ratified <date>)` with `require_body_contains` anchors, precedents ADR-060 `:185`, ADR-062 `:115`)

> The Decision's *a daemon-side proxy on host loopback* is superseded. A `--network=none` container cannot reach host loopback by construction, and the two mechanisms that can — `slirp4netns:allow_host_loopback=true` (M1) and `pasta` (M2) — were measured to reach the **internet** on ubuntu-24.04 (podman 4.9.3) and ubuntu-26.04 (podman 5.7.0) (17-3a Q6, runs 36207588609, 36206431636). The Worker's only reachable endpoint is a **daemon-owned Unix socket** bind-mounted into the container at `/proxy/egress.sock` (mode 0666 inside a daemon-owned 0700 directory: a 0600 socket is refused under the rootless user-namespace remap), served inside the container by a forwarder on `127.0.0.1:18080`; `--network=none` is retained. The bearer is **minted by the daemon's composition root**, not the kernel (the Decision never required a kernel mint), stored only as a hash by the proxy, and verified at the proxy boundary. The proxy journals every refusal by a **confirmed** write (`TransparencyLogAdapter::insert_frame_event`, now `transparency_log.rs:612`) before it returns the refusal. The Context's *+65–130 … 18935/18935* kernel paragraph is replaced by the measured figure ratified for the story. Order is unchanged: proxy and bearer, then the T3 route, then `env_clear` and the guard inversion in the same commit.

## Grant protocol (this record grants none of it)

The operator decides FW-02/03/04. If approved, the **landing commit** re-pins in one change, in this order (`kernel-core-baseline.toml:786-821` at `d371b5a4`; `:782-807` at creation): class rows in `xtask/kernel-api-classes.toml` first → regenerate the surface pin → `src_lines` + HISTORY (line 505 stays line 505) → file digests + `set_hash` (`:684`; the 17-3c CI repair re-hashed `linux.rs` line-neutrally after creation) → the four literals in `kernel_pin_content_hash_16_0.rs` → `kloc.toml` rows (kernel, domain, bin, `maos-egress`, xtask, `_aggregate_hardfail`) → `RATIFIED_AT_EASING` → `check-service-boundary` **and** `check-kernel-baseline` at HEAD, with every addition and removal enumerated in the story record. If the work lands before the grant, it leaves those files unchanged and returns the request with the measured figure. ⚠ **Zero slack (2026-10-07 frontier re-verification):** the ruled ceiling 25353 equals the variant-B prototype exactly, and the story already names lines the prototype omits; a landing figure above 25353 is a new request (story Q10), never a re-pin.

## Disposition

- **Story:** created, validated against the create-story checklist, **`backlog`** — *story created 2026-10-06; awaiting operator kernel grant*. 17-1 does **not** start development until the operator rules FW-02…FW-04 and Q1–Q7.
- **Operator questions:** Q1 (variant A/B + aggregate), Q2 (ADR amendment), Q3 (one story or two), Q4 (exit lines 2/3 on 24/26), Q5 (admission executes the manifest binary on the host; `deny_unknown_fields` owner), Q6 (hermetic-suite sweep), Q7 (escape-corpus scope) — in the story.
- **Ruling (2026-10-06, Lunarpulse):** FW-02 variant B; FW-03 exact at landing, aggregate ≤ +1182; FW-04 ratified; Q3 one story / two commits; Q4 fix at origin; Q5 probe → `19-3`, `deny_unknown_fields` → `20-3c`; Q6 D-8 triage; Q7 five `--network=none` scenarios. Story moved `backlog` → `ready-for-dev`; rulings table in the story §Operator questions.
