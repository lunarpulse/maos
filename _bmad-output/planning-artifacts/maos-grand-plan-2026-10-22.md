# maos grand plan: historical reconstruction and 2026-10-27 snapshot

**Authority and scope.** This document preserves a plan and a dated status snapshot; it is not a live tracker. [`sprint-status.yaml`](../implementation-artifacts/sprint-status.yaml) is authoritative for current story, epic, operator-lane and action-item status. Story records, the PRD and ADRs govern their respective contracts. Later tracker changes supersede this snapshot.

The historical plan below reconstructs the assistant's text at **line 1011** of `/home/lunarpulse/.omp-sessions/2026-10-22T08-47-34-801Z_01a162cb-5391-73fb-ba59-fd77b52e8c60.jsonl` (the response was recorded on 2026-10-23). It describes the 2026-10-22 audit and re-baseline and the session's then-current state. Later compaction summaries in that session are not the source of the plan. The original source is a local session file, not a repository artifact. Historical statuses, host measurements and forecasts below are deliberately **not** assertions about today.

## Part I — Original plan, as recorded

### Diagnosis

The original diagnosis was not simply bad estimation: stories had been called done when behavior existed only in a test harness, not in the product binary. The September review found **148 stories done but only 22 of 66 FRs live from a product binary**. At the October re-baseline only v0.1/J0 was considered real; v0.3/v0.5 had not assembled Butler inference, applied T2/T3 isolation and Observer execution into the product. The E15–E21 recovery lane was moving faster than its own estimates, but each preflight kept finding missing routing and adding scope. See [September correction, lines 14–28](sprint-change-proposal-2026-09-04.md#L14-L28) and [PRD re-baseline, lines 392–394](prd/project-scoping-phased-development.md#L392-L394).

Review cost had also become a product-delivery constraint. The diagnosis identified nested Cargo invocations inside tests, repeatedly built fixtures, multiple image/proof runs, local `CI=true` environment failures, and a heavily loaded host. Its 2026-10-22 measurements included 1,871 seconds of summed libtest time, 1,259 seconds in one nested-Cargo test, and 33 test files invoking builds; the then-current host measurements are historical only. See [review-cost diagnosis, lines 6–28](review-cost-diagnosis-2026-10-22.md#L6-L28).

### Seven planning lanes: original state

| Lane | Original plan and status at the time |
|---|---|
| **A — Requirements routing audit: done** | Check each FR against the actual AC text of its owner story. The audit added nine stories, filed 19 AC widenings and identified four decisions. The original audit's 26 → 35 backlog is a historical count. |
| **B — E17 scope freeze: partial** | Put the rule against adding stories after epic open into the DELTA; the original plan still asked whether to move 17-10 to E20/E21. The actual rule permits security defects and proven-red exit dependencies as exceptions, rather than an absolute ban. |
| **C — Re-baseline on observed velocity: done** | Use the observed 2.36 calendar days/story and +30% scope growth; project v1.0-rc around 2027-01-11. The rc date is the later of engineering completion and the operator-lane critical path. |
| **D — Start the operator lane immediately: not started in the original plan** | Secrets/accounts, performance runner, external penetration test (Hold 1), export-law review (Hold 2), and at least three external authors were backlog. This parallel human/external lane was identified as the true long pole. |
| **E — Decide the kernel ≤20 KLOC ceiling: not yet decided** | DECIDE-4 was required before E18; the plan reported 20,594/20,594, zero headroom, and seven unpriced FLAG-Winston items. Choose ADR re-ratification or make the 21-5 extraction mandatory. |
| **F — Reduce evidence/document fixed cost: partial** | 20-12 addressed test cost. The original plan still owed format/weight cleanup for story Markdown over 100 KB, a roughly 270 KB tracker and roughly 300 KB `deferred-work.md`; those were then-current observations, not current measurements. |
| **G — Tracker hygiene: not started in the original plan** | Reconcile 12 stale action items, including already-satisfied E14-A1 and E15-A2, rather than leave them misleadingly open. |

Sources: [routing audit, lines 7–53](routing-audit-2026-10-22.md#L7-L53); [PRD re-baseline and scope rule, lines 392–408](prd/project-scoping-phased-development.md#L392-L408). The snapshot and decision outcomes are separate in Part II.

### Review and test cost rollout: original phases

- **Phase 0 — operator/workstation, not code:**
  - **0.1:** one Cargo session, with session-specific target directories proposed at the time, after lock waits of roughly nine minutes. **That target-directory proposal is superseded; see Part II.**
  - **0.2:** `cargo sweep` of the then-847 GB target and devcontainer `"init": true` for the then-531 zombies; not yet done in the original plan.
  - **0.3:** address swap, including the then-proposed `swapoff -a && swapon -a`, swappiness 150 → 10 and a container memory limit. The original plan reported free RAM but 21 GB remaining in swap; this was a historical observation, not present host status or authorization to run those commands.
  - **0.4:** unset `CI` for local execution; 20-12's local SKIP profile made this durable. Marked done in the original plan.
- **Phase 1:** review 17-9 using the reduced re-execution contract; marked done at `f1db3356`.
- **Phase 2:** pull forward 20-12: nextest and test budgets; no unmarked Cargo execution inside tests; fixture prebuild; CI proof rules; in-place plant protocol. Marked done at `e1d09f0b`; the original plan recorded warm local per-binary span **1,340 → 292 seconds**, with the CI run still unconfirmed at that time. That span is not the same metric as the earlier cold libtest sum.
- **Phase 3:** apply the new rules to 17-10 and 17-11 and remeasure velocity at the E17 retrospective; originally not started.

The underlying proposal is in [review-cost diagnosis, lines 30–44](review-cost-diagnosis-2026-10-22.md#L30-L44); the historical local measurements and metric distinction are in [PRD, lines 400–404](prd/project-scoping-phased-development.md#L400-L404).

### Decisions originally outstanding

1. **FR48 crypto provider**, before E18 preflight: PRD placed it in v0.5 while the lane parked it post-v1.0.
2. **FR64 cost attribution**, before E20: PRD placed it in v1.0 while the lane parked it as later.
3. **FR54 gateway contracts**, before E21: the v2.5 label also parked kernel lifecycle/scope machinery.
4. **NFR-Maint-1 ceiling**, before E18: ADR re-ratification versus making 21-5's reduction required.
5. **Make `check-test-budget` blocking**, after a CI calibration run for 20-12.
6. **Whether to commit `_bmad/custom/` review-rule files**, then gitignored, immediately.

These are a historical decision list, not a current list of open decisions. See [audit, lines 66–71](routing-audit-2026-10-22.md#L66-L71) and [proof contract, lines 3–8 and 73–81](../../docs/testing/proof-contract.md#L3-L8).

### Historical product roadmap and holds

The original session's **2026-10-22-based +30% projection**, not a current forecast, was:

| Sequence | Milestone | Historical projected date |
|---|---|---|
| E17 | Finish 17-11 security, 17-10, and the retrospective | The 20-12 pull-forward was expected to move this a few days later than the prior projection. |
| E18 | v0.3/v0.5: Butler inference and halt measurements | approximately **2026-11-21** |
| E19 | v0.8 / J1 founder loop | approximately **2026-12-09** |
| E20 | v1.0.0-rc.1 | approximately **2027-01-11** |
| E21 | v1.5–v2.2 substrate reality | approximately **2027-02-04** |

In parallel, accounts/secrets and the perf runner feed the rc path; GA and its LTS clock require Hold 1 (external pen test) and Hold 2 (export counsel), as well as the external-author cohort. The original sequence called for ≥3 outside authors as a preliminary gate toward **N=12**. E18's then-preflight input was seven stories including 18-7 and four AC widenings in 18-1–18-4. The 20-12 pull-forward was expected to reduce later story cost, to be remeasured at the E17 retro.

The PRD's separate **2026-10-22** no-growth projection put E18 at approximately 2026-11-14, E19 at 2026-11-28, rc at 2026-12-24 and E21 at 2027-01-11; its +30% column put E19/rc/E21 at the dates above. These are different historical assumptions, not conflicting current promises. The rc rule was explicitly the later of the engineering projection and the operator critical path. See [PRD, lines 394–398](prd/project-scoping-phased-development.md#L394-L398) and [audit, line 53](routing-audit-2026-10-22.md#L53).

### Original E17 retrospective handoffs

The plan sent these to `epic-17-retrospective`: the watchdog refused-poll observation hook requiring a kernel grant; the `check-test-budget` blocking flip; the `expensive-tests` CI job shape; 17-7/17-4 timing flakes; and four pre-existing CI reds found during 17-9 development—cargo-deny advisories, the `MAOS_IDLE_FAST` environment contract, guest prefetch, and a loom flake. Their eventual named vehicles and closures are in [E17 retro, lines 81–100](../implementation-artifacts/epic-17-retro-2026-10-24.md#L81-L100).

### Original immediate priority sequence

1. Check 20-12 CI and record CI job-hours per story.
2. Operator: address swap, init and Cargo-session organization; start `ops-provisioning-secrets-and-accounts`.
3. Decide DECIDE-4 before E18.
4. Complete security story 17-11, then 17-10, then E17 retro, then E18.

## Part II — Evidence-backed snapshot as of 2026-10-27

### A–G: what changed

- **A — done.** The routing audit was filed and all four original routing decisions have been ruled; their implementation stories may still be backlog. The owner-claims-clause requires the owning story's AC to state the routed behavior. [Audit, lines 7–53 and 66–71](routing-audit-2026-10-22.md#L7-L53); [PRD, lines 406–410](prd/project-scoping-phased-development.md#L406-L410).
- **B — done as a planning rule and E17 closure.** Epic inventory is fixed at open except for a security defect or proven-red exit dependency; other discoveries go to the next preflight. E17 closed 14/14, including 17-10 and 17-11; moving 17-10 out is no longer an outstanding question. [PRD, line 406](prd/project-scoping-phased-development.md#L406); [tracker, lines 255–274](../implementation-artifacts/sprint-status.yaml#L255-L274).
- **C — re-baseline performed, not a fresh prediction.** The original dates and velocity remain dated historical projections. E17 and E18 have since closed and the scope has evolved; no new calendar projection is asserted here. [PRD, lines 392–398](prd/project-scoping-phased-development.md#L392-L398); [tracker, lines 255–286](../implementation-artifacts/sprint-status.yaml#L255-L286).
- **D — operator lane open.** Provisioning and external work are not equivalent to engineering exits. Open rows include `ops-provisioning-secrets-and-accounts` (including `fuzz-ledger` branch, initial ledger and write access, and Docker Hub/account work), `ops-stable-perf-runner`, `ops-external-cohort-and-pen-test`, `ops-brew-tap-and-aur-publication`, `ops-ga-tag-after-holds`, live-key J0, live Worker, nightly cassette rerecord, Calendar OAuth, signed J1 live take, J4 live recording, signed rc tag and live Gemini. The stable runner's sourcing is a rented single-tenant dedicated bare-metal server, not a VPS. Hold 1 pen test and Hold 2 export counsel remain GA holds; the ≥3 authors are preliminary, while the current v1.0 gate requires 12. The first signed tag and Darwin rehearsal are already done, not open. [Tracker, lines 325–340](../implementation-artifacts/sprint-status.yaml#L325-L340); [provisioning checklist, lines 23–44](../../docs/runbooks/provisioning-checklist.md#L23-L44); [release holds, lines 24–27](../../RELEASE-HOLDS.md#L24-L27).
- **E — decision done.** ADR-069 bounds the measured kernel-core production instrument at **≤23,000 tokei**, with alarm at **22,000**, through v1.0.0 GA; exact pins, zero headroom and measured FLAG-Winston grants remain. At E18 close it was **20,610 tokei / 26,367 physical lines**, distinct instruments. 21-5 stays optional until the alarm fires. [Audit, line 71](routing-audit-2026-10-22.md#L71); [ADR-069](../../docs/adr/ADR-069-kernel-core-envelope-through-v1.md); [E18 retro, lines 11–13](../implementation-artifacts/epic-18-retro-2026-10-27.md#L11-L13).
- **F — partial.** 20-12 is done, while `20-15-ci-build-once-nextest-archive` and `20-16-prepush-drift-gate` are in progress, not landed. Neither their in-progress contracts nor the local test-cost improvement prove the broader historical document/evidence-weight problem solved. CI remained compile-dominated: the E17 retro measured job-hours **13.4 → 12.7–13.0**, a much smaller improvement than the local test span. [Tracker, lines 309–313](../implementation-artifacts/sprint-status.yaml#L309-L313); [E17 retro, lines 67–79](../implementation-artifacts/epic-17-retro-2026-10-24.md#L67-L79).
- **G — partial.** E14-A1/A3/A4, E15-A2/A3 and E16-A5/A6/A7/A8 have evidence-backed done records; E16-A6 is closure of the **vehicle decision**, not completion of 19-2. E11-A6 remains in progress, with D11 routed to backlog 21-4; E16-A3 remains in progress because routing the retrospective bucket did not eliminate recurrence. Do not infer all original 12 stale items are closed. [Tracker, lines 369–372 and 425–509](../implementation-artifacts/sprint-status.yaml#L369-L372).
- **F — later update, 2026-10-27 (record weight).** The sprint tracker went from 317,468 to 177,482 bytes by moving 32 closed rows verbatim into `tracker-notes-archive-2026-10-27.md` (keys and statuses unchanged). `deferred-work.md` went from 350,888 to 331,921 bytes by moving Tier A closed bodies into `deferred-work-archive-2026-10-27.md` (owners and dispositions unchanged; the E16-A3 cite was re-pointed). Six stories were archived as a trial into `.archive/*-history-2026-10-27.md`: 17-1, 16-6, 13-3b (governing AC draft, rework brief and D-entries restored inline), j1-crosshost-2b, 15-4 and 16-1. The remaining 49 stories over 100 KB were left untouched by operator choice.
- **G — later update, 2026-10-27 (action items and workstation).** E11-A6 and E16-A3 remain in-progress; none of the vehicles above closed them. Phase 0.2/0.3 were measured on 2026-10-27, superseding the "unknown" wording in those bullets: 5,854 zombies under PID 1 `sleep`; swap about 100% used (zram plus file, 32 GB); swappiness 150; container `memory.max` unlimited; `/mnt/build/cargo` 303 GB. The devcontainer has `init: true` configured, with the operator-approved restart pending a host-side recreate. Host swap, sysctl and limits are unchanged by operator choice. Model-role substitution remains session-only.

### Decision dispositions

The four routing decisions were **accepted by the operator**, not left open:

1. **FR48 (DECIDE-1): split.** v1.0 gets the provider-selection mechanism and kernel surfaces through `20-13`; the FIPS 140-3 validated default module is v1.5.
2. **FR64 (DECIDE-2): split.** v1.0 gets task and session-principal token attribution and reconciliation via `20-14`; CPU-time and storage-I/O dimensions move to v1.1.
3. **FR54 (DECIDE-3): keep v2.5**, including gateway lifecycle/scope machinery; the PRD was amended, and no 21-9 was created.
4. **NFR-Maint-1 (DECIDE-4): ADR-069 increase**, as described above.

See [routing audit, lines 68–71](routing-audit-2026-10-22.md#L68-L71). The test-budget decision is also resolved: 18-0 calibrated it and made it blocking at v1.0; a local prepush budget reading is advisory, not equivalent to CI calibration. [Proof contract, lines 73–81 and 153–155](../../docs/testing/proof-contract.md#L73-L81). The tracked proof contract is the source of truth; `_bmad/custom/` copies are per-checkout, gitignored mirrors. No evidence here establishes a separate explicit decision to commit those files. [Proof contract, lines 3–8](../../docs/testing/proof-contract.md#L3-L8).

### Phases 0–3 now

- **0.1 — partial, updated rule:** use **one Cargo process/session at a time with the one shared `CARGO_TARGET_DIR=/mnt/build/cargo`**; do not create per-session targets or run a second Cargo build against it. The 20-15 creation record reports this discipline; the proof contract forbids a second target for plants. This is not evidence of current host-wide compliance. [20-15 record, line 17](../implementation-artifacts/20-15-ci-build-once-nextest-archive.md#L17); [proof contract, lines 24–30](../../docs/testing/proof-contract.md#L24-L30).
- **0.2 — workstation fixes not established complete:** cleanup/sweep and an init to reap zombies remain uncompleted in this plan; current target size, zombie count and init state are **unknown**, not asserted. The original numbers date from the diagnosis. [Diagnosis, lines 13–14, 24–25 and 41–43](review-cost-diagnosis-2026-10-22.md#L13-L25).
- **0.3 — host swap issue unresolved in the evidence:** current swap use, pressure, swappiness and container limits are **unknown**. The old observations are not current measurements. Before any destructive or host-wide action—including a target purge/sweep, swap cycling, configuration or container changes—preserve the shared build cache, user data and in-progress/uncommitted work; assess effects and obtain explicit operator approval. The historical command is not approval.
- **0.4 — complete:** 20-12's local profile unsets CI and uses explicit local SKIPs while retaining fail-closed CI. [20-12 record](../implementation-artifacts/20-12-test-cost-budget-and-review-execution-contract.md); [tracker, line 309](../implementation-artifacts/sprint-status.yaml#L309).
- **Phases 1–3 — complete:** 17-9, 17-10 and 17-11 completed under the reduced contract; E17 retro remeasured local and CI costs and routed the next lever. Further cost work is 20-15/20-16, not evidence that those stories are done. [Tracker, lines 271–274 and 309–313](../implementation-artifacts/sprint-status.yaml#L271-L274); [E17 retro, lines 55–79](../implementation-artifacts/epic-17-retro-2026-10-24.md#L55-L79).

### Epic order, exact accepted prep decisions and evidence

**E17 is done; E18 is done, 9/9.** E18's final landing run **40396208547**, at `271e2a33`, recorded 189/189 successful jobs, 13.71 job-hours and 73.9 minutes wall; it is evidence of that landing tree, not of the unlanded prep stories. The E19 §R preflight was subsequently ratified; the earlier E18 retro readiness table was superseded by its own §10. `20-15` and `20-16` must land **before 19-1**. `19-7-repin-consent-operator-surface` is backlog and must precede 19-4; it owns the restart-detected → pin-invalidated → operator re-pin → delivery proven-red precondition. [Tracker, lines 275–293 and 312–313](../implementation-artifacts/sprint-status.yaml#L275-L293); [E18 retro, lines 125–142](../implementation-artifacts/epic-18-retro-2026-10-27.md#L125-L142).

**E18 retro rulings R1–R5:** one ratified epic-level per-crate envelope from measured story forecasts plus 30%; build-once CI prep before E19; required prepush drift checks before push; retain the tracked model-role split; and ratify the E19 preflight before 19-1. The later creation decisions below refine the prep contracts. [E18 retro, lines 73–81](../implementation-artifacts/epic-18-retro-2026-10-27.md#L73-L81).

**E19 preflight, operator-ratified 2026-10-27:** D19-1 **A** epic-level envelopes; D19-2 **A** T3 per-run workspace mount with measured kernel grant +12 physical/+9 tokei; D19-3 **A** unchanged hermetic exit with CI T3 substrate provisioning; D19-4 **A** reuse the shared inference-port builder, per-pass mint and context budget; D19-5 **A** use the existing `MorningDigestRenderer`; D19-6 **B** make the re-pin consent surface the new 19-7 before 19-4. [E18 retro, lines 129–138](../implementation-artifacts/epic-18-retro-2026-10-27.md#L129-L138).

**20-15 accepted D01–D08: C/B/A/A/A/A/A/A.** Respectively: archive test binaries plus fixtures and the specified release/debug binaries; one producer per architecture with both image consumers; fixture manifest tree-ID provenance with dirty-tree refusal; extract/remap at the producer workspace path; run nextest with equivalent count/marker pins; run-scoped upload/download artifacts; remove cache only from consumers without widening to other priced levers; and own an Epic-20 measured xtask grant, **STOP above +38**. Same gate set and runtime matrices, no silent consumer rebuild. The projected savings in the creation record are inferences, not measured landing savings. [20-15 decisions, lines 95–110](../implementation-artifacts/20-15-ci-build-once-nextest-archive.md#L95-L110).

**20-16 accepted D01–D08: C/A/A/A/A/A/A/A.** Respectively: workspace nextest plus **unconditional** `cargo test -p maos-kernel-core --lib` and workspace doctests; static CI count/marker pin readers; change-triggered expensive/tool-dependent legs with explicit skips and missing triggered tools red; self-execute the CI gate commands; use merge-base with `origin/main` plus working tree and untracked files without fetching; document and record the required prepush run rather than install a hook or create a second CI authority; own an Epic-20 measured xtask grant, **STOP above +1020**; and retain the local route-manifest plant with byte-identical restoration. After both prep stories land, the E19 xtask envelope rebases to **landed xtask +202**, rather than charging either prep story against its old 46364 → 46566 envelope. [20-16 decisions, lines 82–95](../implementation-artifacts/20-16-prepush-drift-gate.md#L82-L95).

**Hosted T1 spike 40489534163 is narrow evidence, not story completion.** Two 24.04 producers (x64 and arm64) and four 24.04/26.04 consumers ran one selected `maos-wasm-host::e2e_roundtrip` test from same-run archives with compiler guards; matching workspace paths and producer/consumer artifact hashes were recorded. It does not prove other archive families, release bundles, fixture or cgroup jobs, prepush, or billing savings. Job IDs were not retained; success is the session-provided run outcome corroborated by the six downloaded records, not a downloaded report-job log. [T1 evidence, lines 7–15, 52–90 and 103–124](../implementation-artifacts/20-15-evidence/t1-spike.md#L7-L15).

**Proof boundary:** ordinary hermetic/unsigned CI evidence must not be presented as a signed live operator take. E19 CI deliberately leaves `tier2-live-agent-signed` absent; the named human-signed paid live take is `ops-j1-live-agent-signed-take`, and `PROVEN_LIVE_SIGNED` is operator-only for that leg. Separately, the `CI_EVIDENCE_AUDIT_KEY` is recorded **present** and used to sign a particular CI audit outcome; it is not an absent provisioning item and does not turn every CI run into a signed live operator proof. [E19 epic, lines 11–20 and 29–32](epics/epic-19-founder-loop-w4.md#L11-L20); [E18 retro, lines 118–121](../implementation-artifacts/epic-18-retro-2026-10-27.md#L118-L121).

### Session boundaries and citation limitations

- The session reports `f889b3e4` and `84e0e659` as **local, unpushed** commits, with `origin/main` at `271e2a33` and uncommitted 20-15/20-16 work in progress. The E18 retro independently records `f889b3e4` as local ([lines 11–13](../implementation-artifacts/epic-18-retro-2026-10-27.md#L11-L13)); the exact remote position and both-commit unpushed state are session-provided, not independently established by a repository-file citation here.
- Any model overrides used in this session are **session-only**, not a durable change to tracked model policy. The tracked role policy remains [proof contract, lines 55–71](../../docs/testing/proof-contract.md#L55-L71); no tracked policy change is implied by an override.
- The original session plan is recoverable at the local JSONL path and line above, but that file is not portable with this repository. No repository citation establishes a present host swap/init/sweep state, explicit resolution of the old `_bmad/custom/` commit question, or a fresh current calendar forecast; those remain unknown rather than invented.
- **2026-10-28 snapshot (history only):** 20-15 and 20-16 landed on main (271e2a33..b35c9284, operator-authorized) and the discipline run 40609213235 @ b35c9284 is green (attempt 1, 191/191 jobs). CI cost per landing run fell from 13.71 to 10.63 job-hours and from 73.8 to 43.3 minutes wall (evidence `_bmad-output/implementation-artifacts/20-15-evidence/ci/run-40609213235/`). The "local, unpushed" and "uncommitted 20-15/20-16 work in progress" statements above describe 2026-10-27 and are superseded by this landing; E19 xtask envelope rebase is recorded in `epics/epic-19-founder-loop-w4.md` (derived arithmetic, confirm at the 19-1 preflight).
