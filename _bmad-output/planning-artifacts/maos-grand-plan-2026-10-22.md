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

## 2026-11-02 four-story ruling addendum

These rows supersede older forecasts/sequence assumptions only for the named stories; the historical snapshot above is unchanged. All four remain `ready-for-dev`, not implemented or closed. Source: each story's §Decisions/frontmatter and [current tracker](../implementation-artifacts/sprint-status.yaml).

| Story | Operator ruling | Planning consequence |
|---|---|---|
| [20-2](../implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections.md) | D-20-2-01…14 as recommended, ask s202; D09 A confirmed | Kernel 0; bin 33534, xtask 44860, aggregate 192316 ceilings. Default-build netns/TL witness, no shipped stub air-gap artifact; rendered recipes, signed deb/RPM attachments; publication waits for credentials. |
| [20-4](../implementation-artifacts/20-4-gemini-driver-and-endpoint-pin.md) | D06 D and D-20-4-01…11 as recommended, ask s204 | 20-4 owns default selector; model overrides stay on their lane. R = KA + KIO1 + KIO2 +13/+9, kernel ceiling 26867/21060; L1 kernel-free, L2 last. Kimi live proof shares the Gemini operator row after L2; FR38 runtime-context drift routed to 20-13. |
| [20-10](../implementation-artifacts/20-10-purge-residue-removal.md) | D-20-10-01…10 as recommended, ask s2010; -09 B | Kernel 0; bin 34548, aggregate 192876 ceilings. Starts when picked, not after 20-2; binary hint/sentence reserved to 20-2. Ownership is not the report's name prefix; `TerminationUnconfirmed` consumed, never emitted. |
| [20-11](../implementation-artifacts/20-11-erasure-inventory-closure.md) | D-20-11-01…10 as recommended, ask s2011; -02 B, -03 B, -04 A | KS +10/+6, kernel ceiling 26861/21054; L1 kernel-free, L2 last. Large-log EC-13 is D:1331; all-boot lineage/halts and both proof verifiers covered. |

Later kernel landings re-base on exact landed pins. Expected measured cumulative: 21042 + 68 (20-1b) + 9 (20-4) + 6 (20-11) + 33 (20-8) + 25 (20-14) − 1 (20-13) + 21 (20-9) = **21203** tokei. Shared STOP **21209**; D-20-9-15 A (ask `d_batch4`, 2026-11-03) sets **21230 for 20-9 L2**; D-20-14-12 C (ask `d2014_12`, 2026-11-03) sets **21250 for the combined 20-9 L2 + 20-14 L2 cumulative**. ADR-069 alarm **22000** unchanged. These are planning records, not landing measurements; kernel requests bind at dev start.

## 2026-11-12 status addendum: Epic 20 delivery roadmap, history and open items

Written at HEAD `1fd26dcc` (local equals `origin/main`; `git log origin/main..HEAD` was empty when checked). Everything above is unchanged. This addendum supersedes earlier statements only where a dated line says so: the Part I/II status words for Epic 20 (e.g. "ready-for-dev" for 20-2/20-13, "backlog" for 20-1b) are superseded by the tracker values below. Sources: `git log`, [tracker](../implementation-artifacts/sprint-status.yaml), each story file and evidence directory, `xtask/kernel-core-baseline.toml` HISTORY, `deferred-work.md`, and `gh run list/view` read on 2026-11-11/12. Anything I could not confirm is marked **unverified** or **[INFERENCE]**. Story-file `Status:` lines for 20-1b, 20-2, 20-7 and 20-13 still read `backlog`/`ready-for-dev` and are stale against the tracker (not edited here).

### (a) Epic 20 story table

Commits are on `main` unless marked. "Landing" = a commit that carried code; "corr" = review-correction commit; "hosted" = a commit that fixed a hosted-run failure. Models are what the story's Dev Agent Record / review artifacts name; where a record names none, that is stated. Review layers are counted from the review files present in the evidence directories.

| Story | Tracker status | Landings (shas) | Remaining | Dev model(s) / reviews | Pins at landing |
|---|---|---|---|---|---|
| 20-1a registry publish/install exit path | done (2026-11-04) | `850e4c00` (candidate), `8143326b` (corr), `47631284`, `d3e3f586`, `ac1fc12a` (hosted), `03ee4a4e` (inherited-deferral record) | none; F4/F5/F6/mutation auth moved to 20-1b. Main discipline `41910498869` @ `cb0abc06` recorded NOT fully green (known 20-7 flake + cascade). Hosted verify `41521253027` green per tracker | creation opus-5-5; impl `openai-codex/gpt-6.1-sol`. Review plan in story: opus-5.5 Blind/Edge/Acceptance, glm Test-Infra (planned, outcome files not located) | kernel 0 |
| 20-1b vetting/yank/quarantine | **in-progress** (was backlog; reconciled 2026-11-12) | L1 `62faff63`, corr `c656a6e7`; AC15 `f9921251`, corr `a13140b8`; L2 final kernel `608735d2`; r1 `a2bbd687` (hosted fixes, quarantine claims narrowed) | hosted run `43104682092` green on the landed tree (`1fd26dcc`); AC11.5 hosted T3 leg; AC15 hosted TLS/dev/macOS legs (unverified); prepush + independent review of r1; quarantine restart persistence and memory/scalar/release-verb/readback gaps (deferred rows, "quarantine closure" follow-up); ADR-037 I5 flip. Not done | L1 `gpt-6.1-sol`; L2 `anthropic/claude-sonnet-5-5` (Dev201bK). Reviews: L1 blind/edge/acceptance/security + triage; AC15 triage + security; L2 `gpt-review` + `security` (the AC15 and L2 review notes are in `9e8c7737` and the L2 evidence) | L1 kernel 26841/21042 unchanged; L2 kernel 26967/21141; r1 bin 38446, audit 8308, hardfail 202719 |
| 20-2 deb/air-gap/docker/formula | **in-progress** (was ready-for-dev; reconciled) | `93d39cc7` (candidate), `ebf2ab03` (corr), `fd4f60f1`, `96a1dce2`, `d84b83dd`, `bdfc1430` (hosted-1..4) | operator publication legs (Homebrew tap/AUR, Docker Hub credentials, signed attachments); closing review/record. Branch-only verify run `42308080980` (workflow ci-20-2-verify @ `21431e39`, branch `ci/20-2-verify`) concluded success per `gh`; not recorded in the story | creation opus-5-5; impl `gpt-6.1-sol` (Dev202). Reviews: triage of 4 layers (blind/edge/acceptance/test-infra), security layer by orchestrator | candidate: bin 34823, xtask 44673, aggregate 194099; kernel 0 |
| 20-3a gate honesty | done (2026-10-17) | `389a07e9`..`39cd7966` | none | per story (not re-read here) | n/a |
| 20-3b gate retirement | done (2026-10-30) | `2e694cd6`, `00ddd7d3` (L2), `714a1428`, closing `3341f9e4` | tracker: final run `41243340929` 184/189, NOT fully green (known 20-7 flakes); AC2 generator not built (NO-GO) | per story (not re-read here) | kernel 26793/20992, xtask 42841, aggregate 188451 (tracker) |
| 20-3c audit truth / halt residuals | done (2026-11-01) | `ffd19698`, `a41a143f` (L2 kernel), `1a1b11f3`, `42489813` | none; final run `41305010381` 189/189 (tracker) | per story (not re-read here) | kernel 26801/21011 |
| 20-4 gemini driver + endpoint pin | in-progress (done flip reverted 2026-11-11, `345ae30f`) | L1 `2752e750`, corr `73e7afbc`, hosted `cb0abc06`; L2 kernel `251dc692` | hosted run `43104682092` green on the landed tree (`1fd26dcc`); prepush or exception; L2 review disposition; commit-qualified deferred closures; live Gemini/Kimi operator legs (paid keys absent) | creation opus-5-5; L1 `gpt-6.1-sol` (Dev204L1); L2 sonnet-5-5 (Dev204L2). Reviews: L1 layered triage + security; L2 GPT review (named in the revert) | L1 bin 34504, providers 1679, aggregate 193140; L2 kernel 26854/21051 |
| 20-5 v1.0-rc tag + LTS rule | in-progress | L1 `0e455c4e` (candidate), corr `e322c7ad` | release-environment provisioning (check-rc-readiness red until the operator provisions it); RC tag/clock operator-lane (signing needs the key holder); rc ships perf ABSENT-labelled (D-20-5-09 B, ruled 2026-11-12, supersedes 'waits for 20-7 L2'); hosted legs unverified | creation opus-5-5; L1 sonnet-5-5 (Dev205L1). Reviews: GPT (3 P2) + GLM security (1 HIGH/4 MEDIUM/4 LOW/1 INFO per story note); 1 round | xtask 45333, aggregate 202018 (hardfail 202019); after `89b64566` follow-up xtask 45379, aggregate clean 202112 |
| 20-6 macOS compile guard | done (2026-10-07) | `86d5f8f0`, `f88a1968`, `d371b5a4` | none | per story (not re-read here) | n/a |
| 20-7 stable-runner perf gates | in-progress | L1 `1fd4de31`, corr `9f64dd02`, rulings/hosted record `a8af3215` | L2 shadow/binding (needs `ops-stable-perf-runner`, §O O1-O7; O3/O7 are operator prerequisites, O4 rootless podman perf host); owns timing/I/O half of AC5/EC-13 and the 20-7-owned flake family | not re-read here | kernel 26801/21011 -> 26841/21042 (+40/+31) |
| 20-8 manifest contract completion | in-progress | L1 `5610f9d6`, corr `37baeedf`, hosted `be3b9763`; L2 final kernel `692b1e59`, corr `1fd26dcc` | hosted run `43104682092` green on the landed tree (`1fd26dcc`); AC7 hosted T2 legs + plant and genuine 2.1/2.2 guests still unverified; review of `1fd26dcc`; deferred rows (precheck gap, below-schema-7 sections, serialization) | L1 `openai-codex/gpt-6.1-terra`; L2 sonnet-5-5 (Dev208K). Reviews: L1 4 layers (`Review208L1`, review notes `64bf998d`); L2 `gpt-review` + `security` | L1 bin 35914, manifest 4738, domain 10198, xtask 44700, aggregate 198235; L2 kernel 27009/21178 then 27014/21181, bin 38475, aggregate 202750 |
| 20-9 spawned-form lifecycle hooks | in-progress | L1 `9f03064b`; corr `c8e5d035`, `f56959c2`, `fba6af2a`, `459a606a`; hosted `89b64566` | L2: AC7, AC8 (kernel row R +16/+21 priced), AC9; hosted AC6 unverified; the L1 tree is inside the green run `43104682092` (`1fd26dcc`), but no AC6 claim is made | sonnet-5-5 (Dev209L1/L1b/L1c; the correction rounds name Dev209L1d). Reviews: GPT x4 (`gpt-review`, r2, r3, r4) and GLM-5.3 security x4 | r4 bin 38019; r5 bin 38061, wasm-host 3143; tree aggregate hardfail 202067 at `459a606a`, 202113 after the follow-up (hosted-followup evidence) |
| 20-10 purge residue removal | done (2026-11-04) | `75538bf7`, `16c35ef0`, `8c809f9c`, `ef134444`, `76efc19d` | none; hosted verify `41815301963` 7/7 green (tracker); proof-retention policy ownerless/open | creation opus-5-5; impl `gpt-6.1-sol`; the record says no independent review claimed | bin 34191 at initial landing, kernel unchanged |
| 20-11 erasure inventory closure | in-progress | L1 `cbed4dad`, corr `88c7277c`, hosted `063c13cf`; L2 kernel `689a6a53`, corr `5ff0ead1`, r3 `c2beedb4` | AC5/EC-13: the 1M vector PASSED in the green run `43104682092` (43.176 s, workspace-test-suite; story closure/final review of all clauses not claimed); AC6 `ac6_running_t3_worker_uninstall_proves_live_inventory` PASS there, signed-T3 live Worker unverified; private/index hold lease (needs its own grant) | creation opus-5-5; L1 `gpt-6.1-sol` (Dev2011b); L2 dev id Dev2011k, model not named in the record I read. Reviews: L1 4-layer triage; L2 `gpt-review` + `gpt-review-r3`, `security` + `security-r3` | L2 kernel 26880/21069; corr 26891/21075; r3 26889/21073, domain 10215, bin 38362, hardfail 202506 |
| 20-12 test-cost budget | done (2026-10-23) | `1c463d2f`, `e1d09f0b`, `a4cb586d` | none | dev opus-5-5; review `claude-fable-5-1` non-author (tracker) | xtask +1432 ratified |
| 20-13 pluggable crypto provider | **in-progress** (was ready-for-dev; reconciled) | L1 `a2e37988`, corr `a6b557c0`, hosted `a44c76d9` | AC7 kernel row KT3 (L2, last); hosted legs unverified (Apple toolchain, T3) | creation opus-5-5; the record names no L1 implementer. Reviews: 1 round of 4 layers (`reviewer` x3 + `light-reviewer`) + security note | aggregate 198835 (hardfail 198836); kernel 26841/21042 unchanged |
| 20-14 complete cost attribution | in-progress | L1 `f68c7de6`; corr `2ae11192`, `553336f6`, `fc7373f0`; hosted `f8b236a8`; review notes `6b81204e`, `29f9a3ea`, `870117ee` | L2 kernel AC1-AC3 (KT +35/+25 priced) and AC6 closure; hosted recipe unverified | creation opus-5-5; the record names no L1 implementer. Reviews: initial 4 layers + GLM-5.3 security; OpenAI correction reviews x3 | after round 3: domain 10215, CLI 6609 (ceiling 6610), iac 8770, hardfail 199199; kernel unchanged |
| 20-15 build-once nextest archive | done (2026-10-28) | `d0b20053`..`b35c9284` | none; landing run `40609213235` 191/191 | per story (not re-read here) | xtask +38, conmon kernel +11/+9 |
| 20-16 prepush drift gate | done (2026-10-28) | with 20-15 | none | per story (not re-read here) | xtask +1020 (exactly at STOP), kernel 0 |

Model policy, as the records show it: creation records are `anthropic/claude-opus-5-5`; implementation records name `openai-codex/gpt-6.1-sol` (20-1a, 20-1b L1, 20-2, 20-4 L1, 20-10, 20-11 L1), `gpt-6.1-terra` (20-8 L1) and, from 2026-11-11 records onward, `anthropic/claude-sonnet-5-5` (20-1b L2, 20-4 L2, 20-5, 20-8 L2, 20-9). The 20-8 record states the 2026-11-09 rule that the independent reviewer must be a different family; reviewer artifacts name OpenAI "gpt" and GLM-5.3 (security). Earlier reviewer/dev model changes (sonnet/haiku/opus) were not reconstructed beyond these records; **unverified**: the exact 2026-11-09 effective date of "Sonnet-5.5 dev agents" (the first record I found that names it is dated 2026-11-11).

### (b) Kernel ledger

Measured physical/tokei, from `xtask/kernel-core-baseline.toml` HISTORY (95 files throughout):

| Event (commit) | physical | tokei | delta |
|---|---|---|---|
| 20-3c L2 (`a41a143f`) | 26801 | 21011 | +8/+19 |
| 20-7 L1 (`1fd4de31`) base for Epic-20 ledger | 26841 | 21042 | +40/+31 |
| 20-4 L2 (`251dc692`) | 26854 | 21051 | +13/+9 |
| 20-11 L2 (`689a6a53`) | 26880 | 21069 | +26/+18 |
| 20-11 L2 review corrections (`5ff0ead1`, ask `s2` = allow) | 26891 | 21075 | +11/+6 |
| 20-11 L2 round 3 (`c2beedb4`) | 26889 | 21073 | -2/-2 |
| 20-1b L2 (`608735d2`) | 26967 | 21141 | +78/+68 (= grant) |
| 20-8 L2 (`692b1e59`) | 27009 | 21178 | +42/+37 (priced +38/+33) |
| 20-8 L2 corrections (`1fd26dcc`) | 27014 | 21181 | +5/+3 |

Limits: shared STOP **21250** for the whole cumulative measured ledger at each landing (operator ask `halt_grant`, 2026-11-04; supersedes the former 21209 / 20-9-only 21230 / joint 21250 exception). 21250 - 21181 = **69 tokei left**. ADR-069: alarm 22,000 and envelope 23,000 "production lines" (ADR-069 text; I read these as the tokei measure, since physical 27,014 already exceeds 23,000; the assignment text called 23,000 "physical", which I could not confirm); 22,000 - 21,181 = 819. The 20-8 L2 relies on x2 ceiling 27043/21207 and 20-1b on 27045/21209, both inside 21250.

Remaining kernel rows with the priced figures in the stories (planning, not measured): 20-14 L2 AC1-AC3 KT +35 physical / +25 tokei; 20-13 L2 AC7 KT3 +1 / -1; 20-9 L2 row R +16 / +21; 20-9's other open kernel rows named in the 20-8 baseline entry (coordinator.rs expect window, archive.rs atomic record) are unpriced; 20-11 private/index hold lease is unpriced and needs its own grant; 20-8 precheck invariant gap needs an operator-ruled priced row (unpriced); 20-4 has no open kernel row that I found. The quarantine-closure follow-up is forecast +30..+80 tokei, **unmeasured** [INFERENCE]. Derived arithmetic [INFERENCE]: 21181 + 25 - 1 + 21 = 21226 (24 under the STOP) before any quarantine-closure work; with it 21256..21306, i.e. above 21250, so that follow-up needs either a re-ruled STOP or must land before the others.

Operator-ruled surface-hash refreshes (ADR-066 hand-edits in `docs/ci-baselines/kernel-surface-v0.1-beta.json`, each one symbol): `HaltRegistry` (20-11 L2: `b6d65243...` -> `a917eddc...`), `ScheduleWatchdog` (20-11 review corrections, ask `s2`: `2feabbd7...` -> `5e58c7d5...`), `CapabilityRegistryAdapter` (20-1b L2: `bb8e817f...` -> `6e32a642...`), `SpiritManifestBundle` (20-8 L2: `e295e7ff...` -> `ca1ac1ec...`). 20-4 L2 changed no public surface.

### (c) Hosted state

`gh run list --branch main --workflow discipline --limit 12` (all `push` events) showed **12 of 12 concluded `failure`** (2026-11-08 to 2026-11-11) at the first read; the table below keeps those 12 as history. **Update 2026-11-12:** the newest run `43104682092` on `1fd26dcc` concluded **success** (attempt 1, 199 jobs, 0 non-success; created 2026-11-11T21:37Z, completed 22:35Z; `gh run view` and `gh run list --limit 3` re-read by me). It is the first fully green discipline run since `41305010381` (20-3c, 2026-11-01) among the runs I looked at.

| Run | Commit | Red jobs (from `gh run view`) | Recorded cause / status |
|---|---|---|---|
| `43104682092` | `1fd26dcc` | none (199/199 success) | **green**; covers the pushed batch `608735d2`, `692b1e59`, `a2bbd687`, `1fd26dcc` |
| `43034068630` | `c2beedb4` | check-perf-counts, onb-nfr2-timing, check-serde-error-handling, release-dry-run x2 (amd64, arm64), check-multi-tenant-loom, workspace-test-suite, check-test-budget, v1-0-ship-gate, v1-5-ship-gate, aggregate | Only the ac6 worker-variant 60 s wait is recorded (deferred-work row, fixed in `a2bbd687`, **hosted result unverified**). The subject of `a2bbd687` names further fixes (FR4 historical shape, yank rejected-row dedup, serde gate, 34 MiB size limit); mapping them to individual red jobs is [INFERENCE] |
| `43000183420` | `89b64566` | cargo-deny, workspace-test-suite, ship gates, aggregate | cargo-deny: RUSTSEC-2026-0486 (rustix) seen in the failed log; `c402c29b` bumps rustix 1.1.4 -> 1.1.6 (lock-only; "cargo deny clean locally", hosted unverified). workspace-test-suite: the failed log shows `xtask/tests/d11_xtask_ceiling_ratchet.rs` assertions "current xtask grant is unauthorized" / "a ceiling above retirement requires a later authorized grant" (resolved: in `43104682092` the workspace-test-suite job is success and the log shows `xtask::d11_xtask_ceiling_ratchet` tests PASS; the intervening fix commit was not identified) |
| `42987393747` | `459a606a` | check-dev-model-tier, check-test-budget, wasm-host-tests x4, ship gates, aggregate | Diagnosed and fixed in `89b64566` (evidence `20-9-...-evidence/hosted-followup-42987393747/`): sonnet-5-5 allowlist (ask `tier`), `lifecycle_probe.wasm` missing from the producer list, `hook_pump_17_4` budget 56 s; those three jobs are absent from the next run's red list |
| `42955256177` | `59b51a5e` | check-multi-tenant-loom, ship gates, aggregate | cause not recorded in the repo; the loom whole-package leg runs the 1M vector (ruling `m1`, `89b64566` skips it) [INFERENCE] |
| `42948802707` | `f8b236a8` | check-multi-tenant-loom, workspace-test-suite, ship gates | not recorded |
| `42935031860` | `fc7373f0` | cgroup-ceiling x8, wasm-host-tests x4, ship gates | count pins 23 -> 24 fixed by `f8b236a8` (spawned_scope_17_3c) and `59b51a5e` (cgroup_ceiling_workflow_17_2); wasm-host cause not recorded |
| `42913419818` | `a44c76d9` | check-multi-tenant-loom, ship gates | not recorded |
| `42807966928` | `a6b557c0` | journey-hermetic-tier-1, check-multi-tenant-loom, workspace-test-suite, ship gates | no-home regression (fixed `a44c76d9`); `journey_forget_18_6` pending outcome, cause unknown (ownerless-and-open row) |
| `42722028970` | `be3b9763` | workspace-test-suite, cgroup-ceiling-delegated (26.04-arm), ship gates | stale recall workflow-shape counts and an ARM descendant in state `X` (20-13 story note) |
| `42707262457` | `37baeedf` | cargo-deny, windows-check, macos-check, check-ko-coverage, wasm-host-tests x4, ship gates | cargo-deny RUSTSEC-2026-0459 (`libsqlite3-sys 0.28.0`, ignored in `be3b9763`); recall wire/ko glossary fixes in `be3b9763` |
| `42533166219` | `a13140b8` | workspace-test-suite, ship gates | known 20-7 snapshot-owner timing family; the 100k and 1M AC5 vectors passed (tracker) |
| `42464850336` | `f9921251` | epic-20-exit, workspace-test-suite, ship gates | not recorded (`a13140b8` subject mentions an epic-20-exit dev-mode fix) |

Newest push `1fd26dcc`: discipline `43104682092` is **green** (see above). `608735d2` (20-1b L2 kernel), `692b1e59` (20-8 L2 kernel), `a2bbd687` and `1fd26dcc` were pushed together, so that one run covers the landed tree of all of them (no separate runs exist for the first three). Each earlier red job whose name appears in that run's job list is **green in `43104682092`** (all 199 jobs succeeded): cargo-deny, check-multi-tenant-loom, workspace-test-suite, check-test-budget, wasm-host-tests (all four OS/arch variants), check-serde-error-handling, onb-nfr2-timing, check-perf-counts, check-dev-model-tier, release-dry-run (amd64 and arm64), journey-hermetic-tier-1, epic-20-exit, the cgroup-ceiling legs, macos-check, windows-check, check-ko-coverage, and the ship gates/aggregate. Per-job verdicts for the earlier reds are therefore resolved on this tree, but the individual causes that the records never stated remain unknown. In the workspace-test-suite log, `maos-bin::erasure_million_20_11 ac5_million_row_pending_forget_finishes_and_repeats_without_corruption` **PASS 43.176 s** (flagged SLOW > 30 s) and `erasure_live_20_11 ac6_running_t3_worker_uninstall_proves_live_inventory` **PASS 2.510 s** (read from job `130804810431` via `gh api .../logs`). The loom leg skips the 1M test by ruling `m1`. Still **unverified** from this run: signed-T3 live Worker legs, real Apple/Windows execution (only compile checks `macos-check`/`windows-check` are known green), branch-only legs (`ci-20-*-verify`), the `ac6` worker variant's T3 substrate behaviour beyond the PASS line, and every operator-lane item.

### (d) Known open items and operator lane (rows needing money/accounts/hardware are unclaimed per the 2026-11-12 ruling)

- **AC5/EC-13 1M oracle (20-11).** Evidence `20-11-erasure-inventory-closure-evidence/landing-1/ac5-investigation/README.md`: not a regression after L1; the handler's wall time is about 21 s on this host's overlay `/tmp` (15.53 s on btrfs; hosted first completion 15.54 s) while the oracle allows `route_budget()` 10 s + one more 10 s = 20 s, so it fails locally on nearly every run and passes only on a faster host (hosted serial pass `42533166219`). Update 2026-11-12: it PASSED hosted in `43104682092` (workspace-test-suite, 43.176 s, above the 30 s SLOW line but inside the oracle's unchanged deadlines); AC5 closure still waits for the final 20-11 review of all clauses. The loom leg skips this one test (ruling `m1`).
- **20-7 L2** (O1-O7, `ops-stable-perf-runner`; unclaimed, needs a bare-metal runner; per the 2026-11-12 ruling the 20-5 rc no longer waits for it: perf ABSENT-labelled, D-20-5-09 B).
- **`release` environment protection**: unverified (`GET .../environments/release` returned 404 per the deferred-work F3 row; a second row from the 20-5 L1 review); `check-rc-readiness` is red until provisioned; the T3 fuzz-ledger trust anchor is an unsigned JSON on a mutable branch (20-5 GLM F4/F1 row).
- **Purge deletes erasure-proofs vs NFR-Aud-12**: ownerless-and-open, dispositioned by the 20-10 closing record (2026-11-04); the operator has not ruled.
- **rusqlite >= 0.39 bump** (RUSTSEC-2026-0459 ignored; 7 crates; trigger = first story that bumps it).
- **Cohort daemon skips installed-package boot re-verification**: a potential security question for the operator, ownerless-and-open.
- **Quarantine closure follow-up** (ruling `q` = `narrow`, 2026-11-12): the fence is capability-mediated only, not isolation; five deferred rows; kernel +30..+80 tokei unmeasured; needs its own FLAG-Winston row and prototype.
- **Binary size limit 32 -> 34 MiB (35,651,584 B)**: ruled (ask `size`, 2026-11-12) and implemented in `a2bbd687` (`onb_nfr2_timing.sh`, `prepush.rs`, pin test); `onb-nfr2-timing` is green in `43104682092`.
- **Dev-model allowlist** gains `sonnet-5-5` (ask `tier` = allow, 2026-11-11, `89b64566`); the next run no longer lists check-dev-model-tier as red.
- **Precheck / degraded rows**: `HotSwapCoordinator::precheck` reports neither a dropped swap invariant (20-8, needs an operator-ruled priced kernel row) nor the successor's real accepted versions (20-3c-era row); `AuditDropReason::Full` counts a failed `RateLimited` delivery; all ownerless-and-open unless noted.
- **Privacy limits rows** (20-14): principal matcher is case/zero-width/normalization-sensitive (GLM F-1); Spirit-name columns are outside erasure (GLM F-3; owner 20-14 L2 AC3.4 plus an ADR-045 follow-up).
- **Operator's untracked `spikes/` and `*gpui*` files** are not committed and inflate a local `kloc-check`; the 20-9 hosted-followup evidence measures "clean" pins on a `git archive HEAD` copy for that reason (e.g. aggregate clean 202112 vs local totals).

### (e) Operator rulings, 2026-11-04 to 2026-11-12

Only rulings whose record I found are listed with date and effect. Orchestrator ("Main") adjudications are listed separately because they are not operator rulings.

| Date | Ask | Effect |
|---|---|---|
| 2026-11-04 | `halt_grant` | D-20-11-11 A / O4 (+16/+12 kernel, +4 domain); shared STOP 21250 for the whole cumulative ledger |
| 2026-11-04 | `halt_landing` | 20-11 L1 = AC1, AC2, AC4-AC7 kernel-free; AC3 moves to the final KS + O4 kernel landing |
| 2026-11-04 | `d205` | D-20-5-01...16 as recommended (D02 A ceilings; D-20-5-09 A rc waits for 20-7 L2; ignored advisories need a row with an owner or "ownerless and open") |
| 2026-11-06 | `f1b` | inherited F4/F5/F6 and mutation auth become 20-1b AC14-AC17 |
| 2026-11-07 | `f5rows` | D-20-1b-01/02/03 A/A/A (URI-bound registry signing key, rustls transport, BOTH-end dev opt-in); AC15 second kernel-free pass |
| 2026-11-08 | (ask ids unrecorded) | 20-1b AC15 review: D4 operator "configured CA replaces public roots, otherwise bundled Mozilla"; D5 operator "exact CLI 6429 confirmed, former 6856 reserve forfeited" (tracker; ids `ac15d1/4/5` not found) |
| 2026-11-09 | `ac13i2` A | keep I2 fail-stop; journal 20-8 declarations before pid issuance |
| 2026-11-09 | `enc208` A | declaration digest/publisher in colon-delimited hex, exact-byte decoder |
| 2026-11-09 | `i13doc` A | `docs/invariants/I13.md` stays untouched (ADR-037 invariant-lock) |
| 2026-11-11 | `tier` = allow | `sonnet-5-5` allowlisted in `check-dev-model-tier`; 20-9/20-5 records unchanged |
| 2026-11-11 | `m1` option 3 only | loom whole-package leg skips only the 1M vector |
| 2026-11-11 | `s2` = allow | 20-11 L2 correction kernel growth (26891/21075) inside the 26893/21078 ceiling |
| 2026-11-12 | `q` = narrow | quarantine = capability-mediated fence only; no kernel code added; gaps are deferred rows |
| 2026-11-12 | `size` = raise | binary size limit 32 -> 34 MiB |

Named in the assignment but **no record found in the repository (date and effect unverified, not listed above)**: `d2`..`d5`, `o1`..`o6`, `secf2`, `p5w`, `p13w`, `d1w`, `ac15d1/4/5` (see the 2026-11-08 row), `push14`..`push27`, and the "push approvals pattern". Orchestrator adjudications disclosed to the operator: 2026-11-09 20-8 domain ceiling 10174 -> 10205; 2026-11-10 D-20-13-17 (ring-direct evaluation); 2026-11-11 20-9 correction rounds 3-5 and the 20-11 round 3 overdraft; 20-14 CLI/iac/domain ceiling overdrafts; 20-1b AC15 rebase at `c656a6e7`.

### (f) Incidents, recorded honestly

- **Deleted 20-5 untracked files.** The 20-5 landing record states the untracked files were lost and rebuilt from the agent transcript, with logs regenerated (20-5 story, Landing 1 Verification). The root cause is not further documented in the repository.
- **Amended-commit mishap, unrequested pin-fix push, pathspec mistake.** Reported by Main in this assignment, not independently documented in the repository: (1) an amended commit swept WIP into a notes commit and was repaired before the push; (2) `59b51a5e` (cgroup_ceiling_workflow_17_2 expected-count 23 -> 24, authored 2026-11-11 03:49 UTC, confirmed by `git log` and by the hosted run `42955256177` on it) was pushed without a fresh ask; (3) a pathspec mistake left a commit without its code, amended before push. I verified only that `59b51a5e` exists and was pushed; the other two details are unverified.

### Big picture (tracker counts re-read 2026-11-12)

- **Done epics:** E0 and E2-E14 are all `done` (every `epic-N` key and its retrospective; the tracker has no `epic-1` key). E15 6/6, E16 7/7, E17 14/14, E18 9/9 and E19 7/7 stories are done.
- **E20: 9 of 19 done, 10 in-progress, 19 total** (done: 20-1a, 20-3a, 20-3b, 20-3c, 20-6, 20-10, 20-12, 20-15, 20-16; in-progress: 20-1b, 20-2, 20-4, 20-5, 20-7, 20-8, 20-9, 20-11, 20-13, 20-14). `epic-20` is `in-progress` and `epic-20-retrospective` is `backlog`.
- **E21:** 0 of 8 (all backlog). **Ops rows:** 16, of which 2 are done (`ops-first-signed-tag`, `ops-darwin-rehearsal`) and 14 backlog. **v25 parked:** 4. **Later:** 1 (`later-bedrock-vertex-and-kms-backends`).
- **GA gates:** RELEASE-HOLDS.md lists Hold 1 (external penetration test, NFR-Sec-7) and Hold 2 (export-control classification, NFR-Comp-1), both formerly listed as required before `v1.0.0`; **superseded 2026-11-12: claim boundaries, not GA preconditions (see the 2026-11-12 operator ruling subsection)**. The LTS rule (STABILITY.md, 20-5): a 1-year commitment from the LTS clock-start; a release candidate (`-rc.N`) carries no LTS and never starts the clock; the clock starts at `v1.0.0`, recorded in `xtask/release-channel.toml` by `ops-ga-tag-after-holds` (currently "pending"). Whether either hold has cleared is not established by the files I read.

### 2026-11-12 operator ruling: Holds 1–2, rc perf, kernel STOP

Operator words: "we are not doing NFR-Sec-7 and NFR-Comp-1 as we do not have resources and companies to do that. same applies to ops-* work as no resources, only AI agents we have." Answers: (1) `ga` = boundary: Holds 1–2 (NFR-Sec-7 external pen-test; NFR-Comp-1 ECCN) are claim boundaries, not GA preconditions; `v1.0.0` may be tagged without them, but the release notes and RELEASE-HOLDS state: no external penetration test was performed; no ECCN determination was made; assurance is AI-agent review plus the hermetic CI proof only; NFR-Sec-7/NFR-Comp-1 may not be claimed met; WASM-engine-off-by-default stays; the signed tag needs the key holder (an operator action AI agents cannot perform). (2) `rc` = b (D-20-5-09 B): rc ships with perf verdicts ABSENT and labelled; agents dispatch the T3 fuzz chain and provision what the `gh` token allows; money/accounts/hardware rows stay named UNCLAIMED rows; the 20-7 L2 binding phase stays unclaimed (needs a bare-metal runner). (3) `kb` = keep: kernel shared STOP stays 21,250 tokei; ask again before quarantine-closure work. This supersedes the 'Start the operator lane' item D and the addendum lines stating that Holds 1–2 gate GA or that the rc waits for 20-7 L2.
