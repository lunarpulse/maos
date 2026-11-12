# 21-5 creation — cites re-measured at HEAD `0baa76fb` (2026-11-12)

Method: every path read from a `git archive 0baa76fb` copy (`/tmp/c215/head`, files touched after extraction; deleted
after measurement). The working tree carries Dev2014d's uncommitted 20-14 corrections. At the first check (`git status`)
they touched kernel `capability/mod.rs`, `inference/mod.rs`, maos-capability, maos-cli, maos-domain `cost.rs`, maos-iac
`transparency_log.rs`, maos-bin `shell_host.rs` and docs ×3 — none a file cited below. At the closing check they also
touched `xtask/kernel-core-baseline.toml` (`src_lines = 27043`), `xtask/kloc.toml` (`maos-kernel-core = 21203`),
`xtask/tests/{kernel_pin_content_hash_16_0,recovery_lane_ceiling_rule}.rs`, `sprint-status.yaml` and
`deferred-work.md`: every figure here is HEAD's, and the dev re-bases at T0. E21 = `planning-artifacts/epics/epic-21-multi-host-on-live-substrate-w6.md`;
PF-R2 = `planning-artifacts/epics/epic-15-21-preflight-r2-2026-09-05.md`; SN21 = `planning-artifacts/review-2026-09-04/preflight-r2/epic-21.md`;
S = `implementation-artifacts/sprint-status.yaml`; D = `implementation-artifacts/deferred-work.md`;
ACC = `planning-artifacts/decisions-2026-11-12-acceleration.md`; K = `crates/maos-kernel-core/src`.

## C — epic / preflight / tracker cites (OLD → NEW)

None of these is applied to the epic: the creation brief forbids editing epics and the tracker while a dev agent
is working. The OLD→NEW pairs are owed (`records-owed.md` §A).

| # | Where | OLD | NEW at `0baa76fb` | Disposition |
|---|---|---|---|---|
| C1 | E21:26, :46, :92, :94 | `orchestrator/` 5 files 220/90/531/97/90 = 1028 physical, 817 tokei | **unchanged**: `wc -l` 220/90/531/97/90 = 1028; `tokei` 14.0.0: 817 code, 3 comments, 106 blanks (+90 Markdown doc lines) | holds |
| C2 | E21:26, :94 | kernel pin "24472 … 24474 after 15-1"; kloc ceiling `kloc.toml:195` "18933" | `xtask/kernel-core-baseline.toml:505` `src_lines = 27040`, `:1070` set hash `5551f890…`, 95 files; `xtask/kloc.toml:264` `maos-kernel-core = 21201` (measured 21201, zero headroom) | owed |
| C3 | E21:26, :94 | port-type lines at `scheduler_loop.rs:89,111` | field `K/scheduler/scheduler_loop.rs:89`, ctor param **`:113`**, init `:141`; the only use is the resume replay **`:490-505`** (uncited in E21) | owed |
| C4 | E21:94 | `maos-bin` users `main.rs:1995` (registry ctor), `:5696` `journal_orchestrator_queue`, `:5788` `journal_director_lifecycle_action`, `:5859` `journal_token_revocation` | ctor `crates/maos-bin/src/main.rs:2355-2356`; into the scheduler `:3037`, the operator door `:3617`, the orchestrator root `:5491`, status JSON `:6091-6093`, drops `:4756,7264,7339,9533,10050`. The journal calls moved to the door (16-1): `crates/maos-bin/src/operator_door.rs:2028` (`journal_orchestrator_queue`), `:1031,1142` (`journal_director_lifecycle_action`), `:2178` (`journal_token_revocation`); door field/param `:518,573`; queue/recall `:2040-2095`; status `:2852-2854`; root drain `crates/maos-bin/src/orchestrator_root.rs:31,693-708` (uncited) | owed |
| C5 | E21:94 | kernel tests `gateway_dispatcher_fr54.rs`, `gateway_uninstall_fr65_v05.rs`, **`nfr_perf_4_pause_resume_latency.rs`** | first two exist; the third was renamed to **`crates/maos-kernel-core/tests/lifecycle_journal_write_latency.rs`** by `75e1e493` (19-2 AC9; `git log --name-status`) | owed |
| C6 | E21:94 | "`spirits/orchestrator/tests` imports of `maos_kernel_core::orchestrator`" | `spirits/orchestrator/tests/fixtures_pin.rs:6`, `orchestrator_buffer.rs:8` (dev-dependency `spirits/orchestrator/Cargo.toml:21`) | holds (made exact) |
| C7 | — (uncited in E21) | — | maos-bin test support importing the registry: `crates/maos-bin/tests/support/shell_world_16_2.rs:23,323` (used by `shell_host_16_2.rs`, `erasure_million_20_11.rs`), `support/worker_revoke_root.rs:270` (used by `worker_freeze_19_2.rs`) | new |
| C8 | E21:94 | `check-cohort-mesh` re-runs the pin at `xtask/src/check_cohort_mesh.rs:674` | **`:703`** (`crate::check_kernel_baseline::check()`); the gate also shells out to `cargo build -p maos-bin` and test legs (`:26,48,157`) | owed |
| C9 | E21:94 | `check-workspace-count` `xtask/src/check_workspace_count.rs:5-7` | doc `:3-8`; sentinel parse `:91-116`; declared count `4-kernel-design.md:117` (**58**, post 17-1); `Cargo.toml:3-62` 58 members | holds; count owed 58 → 59 |
| C10 | E21:92 | §4.0.7 at `4-kernel-design.md:185` | heading `:180`, the bullet **`:186`** | owed |
| C11 | E21:94 | `xtask/loom-blocklist.toml:14` `Orchestrator`; check-loom not red | holds: `:14`; `check-loom --json` rc 0 at HEAD and on prototype KP | holds |
| C12 | E21:94, E21:28 | new kloc row `maos-orchestrator-buffer = 917` | already present, forward-declared: `xtask/kloc.toml:682-684` (15-2 measured move, measured 0 today) | holds |
| C13 | E21:28, :46, :92 | `maos-domain` +30–60 for the port | prototype KP measured **+3** (`OrchestratorResumePort`, one method); `kloc.toml:425` `maos-domain = 10217` zero headroom | owed |
| C14 | E21:26, :94, :119; PF-R2:59 (D-J) | "re-pinned in tokei units because 21-4 lands first"; order 21-4 → 21-5 | 21-4 is `backlog` (S:414); ruling `s21` (ACC:34) puts 21-5 after 20-9. Both instruments are re-pinned in their current units by every Epic-20 kernel landing (e.g. 20-14 L2, `kloc.toml:264`) — D-21-5-04 | owed |
| C15 | S:415 | "orchestrator/ (1028 LOC) behind a maos-domain port; pin re-pinned negative" | holds as a summary; the key's FR20 basis is new (C17) | holds |
| C16 | ADR-069:42-45, :56 | 21-5 "≈ −1,028 lines" relief valve, optional until the alarm | holds; measured −1041 physical / −827 tokei (KP) | holds |
| C17 | — (E21 cites §4.0.7 only) | — | PRD FR20 `prd/functional-requirements.md:54`: "(**NOT kernel-buffered** — Orchestrator-class Spirit logic uses kernel checkpoint/resume primitives)"; at HEAD the buffer is in `K/orchestrator/buffer.rs`. The move closes an FR20 drift, not only §4.0.7 | new |
| C18 | — | — | PRD FR54 `prd/functional-requirements.md:68`: "kernel hosts the lifecycle and capability-scope contracts" — the `GatewayDispatcher` (`K/orchestrator/gateway_dispatcher.rs:84-365`) has **no production caller** (grep `crates/maos-bin` = 0; only the two kernel tests); FR54 is parked to v2.5 (same line) — D-21-5-01 | new |
| C19 | D:1463 | owner row cites maos-iac "exact 8270, `xtask/kloc.toml:406`" | `xtask/kloc.toml:406` `maos-iac = 8770`; row **Owner: `21-5-orchestrator-port-and-move`** by ruling `r19_6_d04` (19-6 story `:181`) — D-21-5-09 | owed |
| C20 | E21:94 | `Cargo.toml` members +1 | `Cargo.toml:3-62`; `STABILITY.md:30` `workspace crates | 58` is generated (`stability-matrix --check` reds on prototype KP) | new |

## K — code and gate cites (all at `0baa76fb`)

| # | Cite | Fact |
|---|---|---|
| K1 | `K/lib.rs:37` | `pub mod orchestrator; // Story 3.4 …` |
| K2 | `K/orchestrator/mod.rs:19-29,38-97` | four `pub mod`s, re-exports, three `journal_*` fns over `crate::iac::transparency_log::TransparencyLogAdapter` (= `maos_iac::transparency_log`, `K/iac.rs:13` `pub use maos_iac::*`) |
| K3 | `K/orchestrator/buffer.rs:20-95` | `OrchestratorBuffer` (Mutex<VecDeque>, cap 32), `enqueue` `:46-56`, `dequeue_at_safe_point` `:62-67`, `recall_all_pending` `:72-80` |
| K4 | `K/orchestrator/registry.rs:19-45` | `OrchestratorBufferRegistry` (DashMap), `get_or_create` `:32-38`, `get` `:42-44` |
| K5 | `K/orchestrator/gateway_dispatcher.rs:19-26` | deps: `dashmap`, `maos_manifest`, `maos_spirit_abi::gateway` only — no `crate::` import |
| K6 | `K/orchestrator/echo_gateway.rs:83` | only `crate::` use: `crate::orchestrator::gateway_dispatcher::GatewaySubmoduleFactory` |
| K7 | `K/scheduler/scheduler_loop.rs:490-505` | resume replay: `recall_all_pending()` then re-`enqueue` each, `eprintln!` on failure; only in-kernel consumer |
| K8 | `K/isolation/mod.rs:10-12` | doc "parallel-shaped to `halt/` and `orchestrator/`" — stale after the move |
| K9 | `K/halt/output_markers.rs:14` | `i9_exempt` reason text "parallel to OrchestratorBuffer" — still true of the workspace type; no edit needed |
| K10 | `crates/maos-kernel-core/Cargo.toml:10-60,97-108` | no dependency on any orchestrator crate; dev-deps listed |
| K11 | `xtask/src/check_service_boundary.rs:219-254` | removed `(kind,path,hash)` triple ⇒ violation; new triple needs a non-`other` class; `:530-584` hash = full item for structs (private fields included), signature for fns, path text for `use` leaves |
| K12 | `docs/ci-baselines/kernel-surface-v0.1-beta.json` | 401 items; orchestrator entries at `:187,377,382,387,817,822,827,832,837,842,847,1042,1582-1622`; `SpiritSchedulerAdapter` struct `:897` hash `2daf34b2…`, `use` `:1722` |
| K13 | `xtask/kernel-api-classes.toml:367-375` | nine `maos_kernel_core::orchestrator::*` rows, all `supervision` |
| K14 | `xtask/abi-baseline/{v0.1-alpha-pre-abi-freeze,v1-pre-bump}.txt`; `xtask/src/abi_diff.rs:8` | `abi-diff` reads `crates/maos-spirit-abi` only; no prototype touches it (`diff -rq` clean) |
| K15 | `xtask/kernel-core-baseline.toml:1120-1124` | content hashes of the five `orchestrator/*.rs` files |
| K16 | `xtask/tests/kernel_pin_content_hash_16_0.rs:34,820,838,847,858` | `PINNED_ENTRIES = 95`; three readers assert `27040` |
| K17 | `xtask/tests/recovery_lane_ceiling_rule.rs:279,347-353,361-363,396-407` | `RATIFIED_AT_EASING = 21_201` (ceiling ≤ it); envelope 23 000; alarm 22 000 ⇒ `ENVELOPE ALARM` in the row |
| K18 | `xtask/src/check_dependency_closure.rs:42-64,67` | `FORBIDDEN_CRATES` (includes `maos-egress` `:63`), `CHECKED_CRATES = [maos-kernel-core, maos-domain]`; CI `discipline.yml:4315-4327` |
| K19 | `xtask/serde-error-allowlist.toml:733,736` | `crates/maos-kernel-core/tests/gateway_uninstall_fr65_v05.rs:147,148` — already stale at HEAD (those lines are struct-literal fields); gate rc 0 at HEAD and on KP |
| K20 | `.github/workflows/discipline.yml:1227` | `cargo test -p orchestrator --locked` (runs `orchestrator_buffer`, `fixtures_pin`) |
| K21 | `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/15-full-spectrum-v2-2.md:100` | §15.5 Decision 2(a): ceilings move downward any time, including when measured lines physically move out of a crate |
| K22 | `docs/adr/ADR-066-sanctioned-kernel-surface-removal-and-verified-type-signatures.md:22-29,53-61,65-71` | a genuine removal is sanctioned by an ADR; the baseline is hand-edited surgically, never re-emitted |
| K23 | ACC:28-31 (`prepaid` 2a) | a surface-hash **refresh** whose symbol is named in a measured FLAG-Winston grant is pre-approved; a removal is not named there |
| K24 | `crates/maos-kernel-core/tests/lifecycle_journal_write_latency.rs:18-19,50-63` | imports `maos_kernel_core::iac::transparency_log` and `::orchestrator::OrchestratorBufferRegistry`; times row write + `recall_all_pending` |
| K25 | `crates/maos-kernel-core/tests/gateway_{dispatcher_fr54,uninstall_fr65_v05}.rs:5-13` | import the module and `maos_kernel_core::capability::cap_tokens::init_monotonic_base` (= `maos_capability::cap_tokens`, `K/capability/mod.rs:19`) |
| K26 | callers of `SpiritSchedulerAdapter::new` | 40 files; only `crates/maos-bin/src/main.rs:3026-3039` passes `Some(registry)` (`:3037`); 35 test files (maos-bin 23, maos-kernel-core 12) and 2 kernel benches pass `None`; 2 are xtask P1 fixtures (`xtask/tests/fixtures/p1-{clean,violation}/src/main.rs`, parsed, not compiled) |
