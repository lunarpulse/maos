# 20-14 creation — cites re-measured at HEAD `8c809f9c` (2026-11-03)

Method: every path read from a `git archive 8c809f9c` copy (`/tmp/proto2014/base`, deleted after measurement); the
working tree carries a sibling's uncommitted 20-4 L1 edits in `crates/maos-bin`, `crates/maos-domain/src/ports/secret_store.rs`,
`crates/maos-providers` and others — none in a file cited below except `crates/maos-bin/src/{main,env_contract}.rs`
(`git diff --stat HEAD -- crates/maos-domain/src/cost.rs crates/maos-domain/src/ports/inference.rs crates/maos-kernel-core crates/maos-cli docs/adr/ADR-046-*` = empty).
E20 = `planning-artifacts/epics/epic-20-ship-it-w5.md`; PF = `planning-artifacts/epic-20-preflight-2026-10-30.md`;
S = `implementation-artifacts/sprint-status.yaml`; RA = `planning-artifacts/routing-audit-2026-10-22.md`;
RI = `planning-artifacts/epics/requirements-inventory.md`; D = `implementation-artifacts/deferred-work.md`.

## C — epic / preflight / tracker cites (OLD → NEW)

| # | Where | OLD | NEW at `8c809f9c` | Disposition |
|---|---|---|---|---|
| C1 | E20:321, S:316 | kernel "pin at this filing 26361 physical / 20,594 tokei"; E20:323 "Current pin 26,409 / 20,618" | **26841 physical / 95 files / set hash `db39bc4a…` / 21042 tokei** (`xtask/kernel-core-baseline.toml` `src_lines = 26841`; `xtask/kloc.toml:264`), measured by `check-kernel-baseline --json` rc 0 and `kloc-check --json` rc 0 on the untouched copy | in place (E20:323), historical text kept |
| C2 | E20:325, E20:329 (AC3), S:316 | proxy `principal_ids_for_spirit_pid` `inference/mod.rs:375` | call `crates/maos-kernel-core/src/inference/mod.rs:404-407`; the fold `:408-425`; the function `crates/maos-iac/src/adapter/transparency_log.rs:1495-1517` | in place |
| C3 | PF:259, E20:323 | proxy `:383`, emission `:424-427` | as C2; emission `:437-461` (`schema_version: 1` `:438`, `WriteTargetProxy` `:448`, `insert_frame_event(CostAttribution, pid, None, …)` `:452-461`) | in place |
| C4 | E20:325, E20:328 (AC2), S:316 | emission site `inference/mod.rs:406-426` | `:398-461` (inside `complete` `:276-477`; the `InferenceCall` row with the token `:372-388`) | in place |
| C5 | E20:325, E20:323, PF:260 | `maosctl audit cost-reconcile` `cli.rs:667` (E20) / `:729` (PF) | `crates/maos-cli/src/cli.rs:746-763` (variant `CostReconcile` `:751`) | in place |
| C6 | E20:325, E20:323, PF:260, S:316 | `build_cost_report` `subcommands.rs:4614`, grouping `:4622-4645` (E20) / report row `:4722`, builder `:4790` (PF) | row `crates/maos-cli/src/subcommands.rs:4772-4782`, report `:4783-4796`, `build_cost_report` `:4842-4983` (key `(principal, spirit_pid, provider, model)` `:4850-4851`; `Ambiguous`/`Unattributed` → `host-unallocated` `:4868-4873`), entry `audit_cost_reconcile` `:5023-5100`; in-src test `:5862-5906` | in place |
| C7 | E20:323, E20:329, PF:259, S:316 | `AttributionSource::SessionPrincipal` "Future" `cost.rs:57-60` / `:57-61` | enum `crates/maos-domain/src/cost.rs:53-62`, "Future" doc `:60`, variant `:61`; constructed nowhere in `crates/` (grep) | in place (cite only) |
| C8 | E20:323, E20:325, E20:328, PF:258, S:316 | `no_task_ref_in_payload` `cost.rs:248-266` | `:247-268` (asserts no `task_ref`/`correlation_id`/`request_id` key) | in place |
| C9 | E20:329 | SR-4 `cost.rs:239-245` | `principal_ref_ambiguous_no_members` `:238-245` | same substance |
| C10 | E20:327 | forward-read `cost.rs:271` | `forward_read_tolerance_unmodeled_dimension` `:270-311` | same |
| C11 | E20:323, E20:325, PF:258 | `InferenceRequest` `inference.rs:30-44`; SCB `control_block.rs:258,278` | unchanged: `crates/maos-domain/src/ports/inference.rs:28-45` (`#[non_exhaustive]` `:29`, `new` `:47-65`); `crates/maos-kernel-core/src/scheduler/control_block.rs:258` (struct), `:278` (`task_assignments_in_flight: Mutex<Vec<_>>`) | same |
| C12 | E20:328 (AC2) | "`InferenceRequest` gains a correlation id (Added-only, `abi-diff`)" | `abi-diff` reads only `crates/maos-spirit-abi` (`xtask/src/abi_diff.rs:8`, `MANIFEST`); `InferenceRequest` and `CostAttributionPayload` live in `maos-domain` and are invisible to it. Additivity comes from `#[non_exhaustive]` + an unchanged `InferenceRequest::new` (26 call sites in the tree, 5 production: hello, Butler, Orchestrator, Researcher, maos-eval) — D-20-14-03 | in place + D-row |
| C13 | E20:328 (AC2), PRD `project-scoping-phased-development.md:412` | "SCB current-task marker set from the dispatched `task.assign`" | **No Inference Port caller is ever dispatched a `task.assign` at HEAD.** Production consumers: the delegation leg, which spawns a Worker subprocess (`crates/maos-bin/src/delegation.rs:10-14,540-560`) whose CLI spend goes through the egress proxy, never the port (`worker_cli.rs:739-753,809-819`); installed-Spirit dispatch (`installed_spirits.rs:141-176`, WASM engine off in published builds). The SCB ledger is written for Workers only (`supervision.rs:1787,2153,2290-2309`). The port's production callers mint per turn/pass: shell turn `shell_host.rs:112-122`; the shared seam mint `main.rs:4179-4200` used by Butler (`spirits/butler/src/lib.rs:1175`), Orchestrator (`orchestrator_root.rs:90`), Researcher (`researcher_root.rs:204`); one-shot hello `main.rs:9601-9615` | D-20-14-01/02 |
| C14 | E20:325, ADR-046 `:40` | SCB "has no current-task marker (`control_block.rs:273`)" | `:258`/`:278` (C11); still true | ADR amended at landing (AC5) |
| C15 | ADR-046 `:13`, `:15` | `validate_namespace_write()` a stub returning `true`; "no session-principal concept exists" | `validate_namespace_write` `memory/mod.rs:694-697` checks only non-empty/no-NUL; principal writes are authorized **per pid, not per principal** (`authorize_principal_writes` `:405-412`, enforced `:734`) — the proxy is still writable by the attributed Spirit. No session principal exists; the nearest boot-bound principal is `MAOS_BUTLER_PRINCIPAL` (`main.rs:4132-4150`, `env_contract.rs:216-220`, D-19-2-08 A); an IdP-verified principal exists only for governed mints under SSO (`worker_spawn.rs:418-430`, `enterprise_identity.rs:307-338`) | D-20-14-04 |
| C16 | RA:69, RA:130 | FR64 PRD phase `functional-requirements.md:122` | FR64 text `prd/functional-requirements.md:107`; `:122` is the category table row (v1.0 FR64, v1.1 CPU/storage) | cite kept, both valid |
| C17 | E20:321, S:316 | "amendment (b) inventory fixed at epic open" `prd/project-scoping-phased-development.md:406` | the scope-control amendments paragraph is `:406`; the DELTA-2026-10-25 ruling text `:412` | same |
| C18 | E20 AC4 | "the operator cost page" | **no cost page exists**: `cost-reconcile` appears only in `docs/adr/ADR-046-…md` (grep over `docs/`, `docs-site/`); operator pages are `docs/maos.dev/run-maos.md` (+ `docs-site/docs/run-maos.md`, `docs-site/i18n/ko/…/run-maos.md`) | D-20-14-10 |
| C19 | D:80, D:91 | FR64 owner `20-14`; NFR-Cost-1 "Ownerless and open" | unchanged lines; coverage rows `tests/coverage-matrix.yaml:899-904` (FR64 `gates: []`) and `:1103-1108` (NFR-Cost-1 `gates: []`) | AC6 |

## K — code facts the story relies on (all at `8c809f9c`)

| # | Fact | Cite |
|---|---|---|
| K1 | The cost frame is written by the kernel adapter after every successful call, in every inference mode including explicit replay (replay swaps the router's drivers, the adapter stays in the path) | `inference/mod.rs:390-461`; shared adapter `main.rs:4073-4084`, per-Spirit builder `:4089-4112` |
| K2 | The token the call presents is verified (signature + scope) before emission | `inference/mod.rs:250-272,290` |
| K3 | Payload deserialization tolerates absent and unknown *fields* (no `deny_unknown_fields`); an unknown *dimension key* is an `Err` | `cost.rs:85-106`, test `:270-311` |
| K4 | The report groups by boot-local `spirit_pid`; the TL row's `boot_nonce` is on `AuditEntry` but unused; `(boot_nonce, pid) → name` is recoverable from kind-19/kind-7 rows | `subcommands.rs:4850-4851`; `maos-audit/src/lib.rs:92-120,1679-1767` |
| K5 | SR-3: forget discovers principal-bearing frames by an any-kind body scan and scrubs every string naming the principal — a session principal in a cost payload is covered without new code | `transparency_log.rs:1519-1525,1555-…,1885-1953`; test `crates/maos-kernel-core/tests/forget_cascade_18_6.rs:184,432` |
| K6 | `cost.attribution` is an FR4 non-call kind regardless of a token | `crates/maos-audit/src/fr4_classifier.rs:61-76` |
| K7 | Every TL row timestamp is wall-clock, so `--month` bounds match | `transparency_log.rs:1276` |
| K8 | Payload struct literals outside `maos-domain`: kernel `inference/mod.rs:437`, `maos-cli/src/subcommands.rs:5874` (test), `crates/maos-audit/tests/erasure_smoke_test.rs:99` | grep `CostAttributionPayload {` |
| K9 | The J0 shell cassette answers 50 in / 30 out tokens, provider `replay`, model `hand-authored-seed` | `crates/maos-journey-test/cassettes/j0/shell-intro.json` entry 0 |
| K10 | Worker CLI spend is not attributed: the egress crate has no usage parser (grep `usage`/`input_tokens` over `crates/maos-egress/src` = 0) | `crates/maos-egress/src/{proxy,forwarder,upstream}.rs` |
