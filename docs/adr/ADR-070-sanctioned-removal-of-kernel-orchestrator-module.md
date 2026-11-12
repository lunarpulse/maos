---
Status: ACCEPTED — ratified 2026-11-12 by the operator under Story 21-5 (`21-5-orchestrator-port-and-move`), D-21-5-08 A (ask `d21`, accepted as recommended). Epic 21 (multi-host on the live substrate, W6); the ADR-069 relief valve, scheduled by ruling `s21`.
Gate: `xtask check-service-boundary` PASSED (0 NFR-Test-2 rows) against the surgically edited `docs/ci-baselines/kernel-surface-v0.1-beta.json` (401 → 380 items: 21 triples deleted, 1 hash refreshed); `xtask check-dependency-closure` PASSED with `maos-orchestrator-buffer` in `FORBIDDEN_CRATES`; structural test `xtask/tests/orchestrator_out_of_kernel_21_5.rs`
Decided: 2026-11-12
Accepted-in-PR: <PR_NUMBER>
Revisits: ADR-066 (the form: a kernel surface removal is sanctioned by an ADR because the baseline JSON cannot hold the note); ADR-069 §3 (21-5 as the pre-named relief valve)
Supersedes: nothing — sanctions a removal of public kernel symbols that moved, not died
---

# ADR-070 — Sanctioned removal of `maos_kernel_core::orchestrator`: the FR20 buffer and the parked FR54 gateway host leave the kernel

**Context.** FR20 says the Orchestrator instruction buffer is "NOT kernel-buffered" and architecture
§4.0.7 says the kernel embeds no orchestration policy; at `0baa76fb` the buffer, its registry, three
journal helpers and the parked FR54 gateway host lived in `crates/maos-kernel-core/src/orchestrator/`
(5 files, 817 tokei). ADR-069 named Story `21-5` as the relief valve for the kernel row; operator ruling
`s21` (2026-11-12) scheduled it. `check-service-boundary` treats a baseline triple that disappears as an
NFR-Test-2 violation (`check_service_boundary.rs:219-232`), and ADR-066's standing form is that a
removal is sanctioned by an ADR, because a key inside the baseline JSON would be silently destroyed by the
next re-emit. `prepaid` (2a) names surface *refreshes*, not removals — hence this record.

## Decision (a) — the removal is sanctioned

The module moves whole (D-21-5-01 A) to the NEW leaf crate `maos-orchestrator-buffer` (D-21-5-03 A). The
kernel keeps one call: `SpiritSchedulerAdapter::resume` calls
`maos_domain::orchestrator::OrchestratorResumePort::replay_pending(spirit_id)` on an
`Option<Arc<dyn OrchestratorResumePort>>` the composition root passes (D-21-5-02 A); the leaf's
`OrchestratorBufferRegistry` implements it. The 21 removed public triples, with their new home:

| Kind | Removed path (`maos_kernel_core::orchestrator::…`) | New home (`maos_orchestrator_buffer::…`) |
|---|---|---|
| enum | `buffer::OrchestratorBufferError` | same path |
| struct | `buffer::OrchestratorBuffer` | same path |
| struct | `registry::OrchestratorBufferRegistry` | same path |
| fn ×3 | `journal_orchestrator_queue`, `journal_director_lifecycle_action`, `journal_token_revocation` | crate root |
| struct ×5, trait ×1 | `echo_gateway::{EchoGatewayFactory, EchoGatewaySubmodule}`, `gateway_dispatcher::{GatewayDispatcher, GatewayInstance, GatewaySubmoduleRegistry, GatewaySubmoduleFactory}` | same path (FR54 host; still unwired, parked to v2.5) |
| use ×9 | the crate-root re-exports of the buffer, error, registry, and the six gateway types | crate-root re-exports |

No shim re-export remains in the kernel (clean cutover): the old path does not exist.

## Decision (b) — one refresh, not a removal

`scheduler::scheduler_loop::SpiritSchedulerAdapter` persists; its signature hash covers the private field's
type, which changed from the concrete registry to the domain port: `2daf34b24f2563ac…` →
`d2170d389c72d692…`. It is pre-approved by the standing rule that a refresh whose symbol is named in a
measured FLAG-Winston grant is pre-approved (`21-5-flag-winston-decision.md`). The two `use` leaves of the
type are unchanged.

## Decision (c) — the kernel cannot take the buffer back

`maos-orchestrator-buffer` joins `FORBIDDEN_CRATES` of `check-dependency-closure` (D-21-5-09 A): the kernel
closure must not contain it. Under the ruled port shape the kernel does not link the leaf at all; the
structural test fails if the module directory, a `pub mod orchestrator` in the kernel `lib.rs`, or a Rust
file naming the old path returns.

## Decision (d) — behaviour

The move changes no sequential result (equivalence goldens `crates/maos-bin/tests/orchestrator_equivalence_21_5.rs`,
recorded against the unmoved module at L1 and re-run unchanged against the leaf). One deliberate
difference, D-21-5-07 B: the resume replay holds the buffer's single lock; the pre-move replay drained and
re-enqueued in two lock scopes, so a producer racing a resume could fill the queue in between and the
replayed tail was refused (`resume re-enqueue failed … at capacity (32)`), losing queued director
instructions (witness `a_producer_racing_resume_neither_overtakes_nor_displaces_the_queue`, red 11/11
before, green after).

## Provenance of the repair

`docs/ci-baselines/kernel-surface-v0.1-beta.json` was **hand-edited surgically** — 21 item deletions and
one `signature_hash` refresh, 401 → 380 — never re-emitted (ADR-066 provenance; `--write-baseline` is not
used). `xtask/kernel-api-classes.toml` loses the nine `orchestrator` rows with their banner. The new trait
`OrchestratorResumePort` sits in `maos-domain` and is outside the kernel surface.
