# Operator decisions — 2026-11-12 de-drift (GA definition, WASM on, live legs, authors, ops rows, kernel asks)

Recorded 2026-11-12 at tree `7ec4b519` by a second session (a grand-plan review), not by the session that is
closing Epic 20. Source: the operator's chat answers to six numbered questions; the keys in backticks were
assigned by the recorder so later records can cite them. Supersedes only what is named below.
Earlier same-day records stand unless named: `decisions-2026-11-12-acceleration.md`,
`docs/release/decisions-2026-11-12.md`.

## Operator words

> 1. GA 정의: v1.0.0 require 3번 항목까지 요구
> 2. I do not care who restrict maos wasm features. what do we need to make it ON? is that only the legal
>    restriction? then we do it , ignore those outside pressure.
> 3. live key: what provider api key? if we cannot get it can we skip that? and move forward?
> 4. I have them but do not get blocked by them. make them minimal and optional. I want the agents to generate
>    from the instructions for agents. like hermes agent or pi agent knows how they extend themselves using the
>    docs and instructions and skills internally available to their agent harness system.
> 5. yes. drop the ops.
> 6. we can relax the kernel limitation to accelerate the development, less permissions to increase every time.
>
> drop and clean up those not core product features but restricting the development of the MAOS project. We
> move forward.

## Rulings

| # | Key | Ruling | Supersedes |
|---|---|---|---|
| 1 | `ga3` | `v1.0.0` GA requires, besides the Epic 20 exit: `21-3`, `21-6`, `21-7`, `21-8`, `21-9`, `21-10` and `later-registry-quarantine-closure`. Holds 1–2 stay claim boundaries (`ga` = boundary stands). `v1.0.0-rc.1` is not affected. | "cut v1.0.0 once engineering exits are green" (`ops-ga-tag-after-holds`, ruling `ga`) for the meaning of "engineering exits" |
| 2 | `wasm` = on | The WASM host is ON in the default artifact. Vehicle: `21-9-wasm-host-default-on`. Until it lands the tree stays OFF and `check-export-control` stays as it is. NFR-Comp-1 remains not claimed. | RELEASE-HOLDS "Standing mitigations"; `docs/compliance/export-counsel-precondition.md` as a merge precondition |
| 3 | `live` = skip | No paid provider key is required for anything. Every live leg is optional and never blocks a story, an epic exit, an rc or GA. `ops-j4-live-incident-recording` no longer blocks `21-1`. | Epic 21 "21-1 is blocked until the row is `done`" |
| 4 | `authors` = agents-first | Human external authors are optional and never block. The authoring gate moves to agents that generate a Spirit from shipped instructions. Vehicle: `21-11-agent-authoring-kit-and-agent-author-trial`. | The N=12 human-author denominator as a v1.0 gate (NFR-Onb-1 / NFR-Test-8 as read by `check_third_party_trial.rs:325`), once 21-11 lands |
| 5 | `drop` | Operator-lane rows that are not core product work are dropped (table below). | The 2026-11-12 `rc`/`ga` wording "stay named UNCLAIMED rows" for the dropped rows |
| 6 | `kfree` | Kernel growth inside the ADR-069 envelope needs no per-landing operator ask and no pre-priced grant (ADR-069 amendment of the same date). | `lim` = stop22 (the shared STOP is withdrawn); ADR-069 Decision 2's per-story grant ceremony |

`21-9` in the GA set is the recorder's placement, made because the ruling changes what the default artifact
contains and that is cheaper before the LTS clock starts than after. The operator's words for ruling 1 name
only the earlier list (invariant ledger, 21-3/21-6/21-7/21-8, quarantine closure).

## Facts measured for rulings 2 and 3

**Ruling 2 — why the WASM host is off.** Only the export-control precaution; no technical reason was found.

| Enforcement point | Location |
|---|---|
| Default feature set excludes it | `crates/maos-bin/Cargo.toml:15` (`default = ["network"]`), `:33` (`wasm-host`) |
| Gate that reds if it enters the default closure | `xtask/src/check_export_control.rs:194`, `:310` (`check_wasm_host_absent_from_default`) |
| Test that requires the OFF statement and the opt-in build line | `xtask/tests/decision_adrs_and_provisioning.rs` clauses (g) `STABILITY.md` `PRESERVED:export` fence, (h) `README.md` `--features wasm-host` |
| Written rationale | `docs/compliance/export-counsel-precondition.md`, `RELEASE-HOLDS.md` "Standing mitigations" |

What turning it on also needs: `maos-wasm-runner` shipped beside `maos` (it is a subprocess; `packaging/`,
`release.yml` and `Dockerfile` do not name it today); the release size bound of 35,651,584 B
(`xtask/src/prepush.rs:509`) re-measured with the engine in (not measured here); the two CI build variants
collapsed into one. `maos-bin` has 86 `feature = "wasm-host"` sites and CI already builds and tests the
feature-on variant, so the code path is exercised today.

**Ruling 3 — which keys.** `MAOS_ANTHROPIC_API_KEY` or `MAOS_OPENAI_API_KEY` (J0 live leg, J1 live take,
Orchestrator seam, nightly re-record); `MAOS_GEMINI_API_KEY` / `MAOS_KIMI_API_KEY` (20-4 cassettes); a vendor
CLI login for a live claude/codex Worker (`crates/maos-bin/src/env_contract.rs:89-108`). An Ollama driver exists
(`crates/maos-providers/src/ollama.rs`, `MAOS_OLLAMA_URL`), so a local model may take a live leg at no cost;
whether each leg accepts it was not verified.

## Ruling 5 — row dispositions

| Row | Disposition |
|---|---|
| `ops-google-calendar-oauth-mcp` | Removed. The fixture server carries the proof. |
| `ops-rpo-rto-cadence-proof` | Removed. No production deployment exists to prove. |
| `later-bedrock-vertex-and-kms-backends` | Removed. No driver exists and no journey depends on it. |
| `v25-canary-rollback-and-native-push` | Removed. The row itself said no journey depends on it. |
| `ops-stable-perf-runner` | Dropped as work. Key kept only because open `deferred-work.md` rows name it as Owner (the 20-7 runner-only rows); `check-dev-record-completeness` would red on a missing owner. Perf verdicts stay ABSENT. |
| `ops-nightly-cassette-rerecord` | Dropped as work. Key kept for the same reason. |
| `ops-external-cohort-and-pen-test` | Pen-test half dropped (claim boundary 1 in `RELEASE-HOLDS.md`). Human-author half optional (ruling 4). |
| `ops-brew-tap-and-aur-publication` | AUR half dropped. The tap is optional. |
| `ops-live-key-j0-leg`, `ops-live-worker-under-17-1-profile`, `ops-j1-live-agent-signed-take`, `ops-j4-live-incident-recording`, `ops-live-gemini-leg` | Optional, never blocking (ruling 3). Keys kept: closure notes, the rc notes and deferred rows cite them. |
| `ops-provisioning-secrets-and-accounts` | Kept, shrunk to `release` environment protection (optional for an rc). |
| `ops-rc-signed-tag`, `ops-ga-tag-after-holds` | Kept: the release acts. |
| `later-registry-quarantine-closure` | Kept and promoted to a GA precondition (ruling 1). |
| `v25-erasure-crash-reconciliation`, `v25-private-tier-erase-atomicity`, `v25-signed-transparency-log-artifact-identity` | Kept as v2.5 parking; not ruled on. |

The four removed rows are preserved verbatim in
`../implementation-artifacts/tracker-notes-archive-2026-11-12.md`. No code, test or workflow named them.

## Ruling 6 — what "relaxed" means here (recorder's reading, conservative)

- Inside the 23,000 envelope a kernel landing is self-service: measure at landing, re-pin with
  `scripts/repin.py`, write the HISTORY row in the same commit. No operator ask, no reverted-prototype pricing
  at story creation. Kernel landings are listed at the epic retrospective.
- ADR-066 surface-hash refreshes caused by such a landing are pre-approved on the same terms.
- The Epic 20 shared STOP (22,000) is withdrawn. The alarm (22,000) and the envelope (23,000) are unchanged
  because `xtask/tests/recovery_lane_ceiling_rule.rs` enforces them; the row stood at 20,429 tokei at the
  ruling, 2,571 below the envelope.
- The exact pin and the HISTORY row stay: they are what the gates check, and they are not a permission.
- Raising the envelope itself is one ADR line and one test constant; it was not asked for and was not done.

## Applied by this record (documents only; no source, workflow, pin or story status changed)

- `sprint-status.yaml`: operator lane regrouped (release acts / optional / dropped as work); four rows removed;
  `21-9`, `21-10`, `21-11` added as `backlog`; ruling lines on `21-1`, `ops-ga-tag-after-holds` and
  `later-registry-quarantine-closure`.
- `RELEASE-HOLDS.md`: GA preconditions; the WASM ruling; a correction to the stale "All PRD journeys served".
- `docs/release/v1.0.0-rc.1.md` §4: the two removed operator rows and the removed backend row.
- `docs/runbooks/provisioning-checklist.md`: the two rows whose owning sprint row was removed.
- `docs/adr/ADR-069-kernel-core-envelope-through-v1.md`: amendment for ruling 6.
- `epics/epic-21-multi-host-on-live-substrate-w6.md`, `epics/index.md`,
  `prd/project-scoping-phased-development.md`, `maos-grand-plan-2026-10-22.md`: pointers to this record.
- Side effect: `RELEASE-HOLDS.md` grew by 3 lines after its line 16 and by 10 after its old line 52; dated
  `RELEASE-HOLDS.md:<line>` cites in older records (for example the Epic 21 header and the provisioning checklist)
  were not re-pointed.

Checked with the prebuilt `xtask` binary (no cargo run; another session holds the cargo slot):
`check-decision-register`, `check-epic-close-coherence`, `check-dev-record-completeness`,
`check-exit-commands`, and `check-rc-readiness` for an unchanged verdict. Tree-bound cargo tests that read the
tracker were not run.

## Open work created by these rulings

1. Story sections for `21-9`, `21-10`, `21-11` at the Epic 20 retrospective preflight.
2. `21-1`: choose the cassette source (local-model recording or agent-authored) at story creation.
3. `21-11` re-rules `xtask/src/check_third_party_trial.rs:325` and the checklist row `external third-party
   authors`; until then the gate still wants 12 human authors.
4. `.github/workflows/journey-nightly.yml` still carries the live re-record steps that need
   `MAOS_ANTHROPIC_API_KEY`; removing them is a workflow change with count pins. Owner: `21-2`, the story that
   next touches that workflow (rule 10(b)).
5. The story-creation contract and `docs/testing/proof-contract.md` still describe reverted-prototype kernel
   pricing and per-row FLAG-Winston asks; they are to be aligned with ruling 6 by the session that owns them.
6. Not ruled, recommended for the Epic 20 retrospective: re-measure the lane's own success criteria
   (`sprint-change-proposal-2026-09-04.md:104`): FRs live from a product binary, and the governance to product
   code ratio. No measurement after 2026-09-04 was found.
