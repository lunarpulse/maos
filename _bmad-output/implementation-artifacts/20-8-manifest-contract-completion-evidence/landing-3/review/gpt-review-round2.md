## Verdict

**overall_correctness: correct**  
**confidence: 0.97**

Both earlier P2s are closed. No patch-introduced blocker or actionable regression found.

**Counts:** P0: 0 · P1: 0 · P2: 0 · P3: 0.

## Corrections confirmed

- **Live predecessor declaration:** `operator_door.rs:1791–1799` reads `runtime_snapshot().manifest.swap_invariants`, matching the source used by `coordinator.rs:192–202`. Missing or stale retained text no longer changes the hot-swap invariant verdict.
- **Malformed successor:** `operator_door.rs:1775–1788` completes coordinator parsing before comparison. A schema-7 scalar `swap_invariants = false` returns `precheck_failed`, not `SwapInvariantDropped`.

## Boundary cases and rejected candidates

| Case | Result |
|---|---|
| Missing SCB | The initial scheduler lookup refuses `spirit_not_loaded`; coordinator lookup also refuses absence. No ordinary missing-SCB path silently becomes safe. |
| SCB at another pid | Selection uses `spirit_id`, not an assumed pid. Scheduler resolution, coordinator resolution and the added lookup traverse the same ordered map; production wiring supplies `scheduler.scbs()` to the coordinator (`main.rs:3181–3182`). |
| Stale retained text | Hot precheck ignores it. A retained extra invariant cannot cause a false refusal; absent retained declarations cannot hide a live commitment. |
| Schema-6 successor | Both parsers give it no effective invariant declaration. Any live predecessor commitment is therefore dropped. |
| Schema-7 predecessor without invariants | The live preservation list is empty; the new comparison adds no refusal. Coordinator parsing still rejects malformed successors. |
| Unreadable target | Production vetting reads first and returns unsafe `VettingPreconditionFailed`; coordinator loading also rejects read failure. The later defaulted file read does not make this ordinary path safe. |
| Hot swap, migrator and upgrade hops | Both kernel policies reach `initiate_swap` (`upgrade.rs:137–164`); every planned hop invokes the orchestrator (`migration_plan.rs:229–249`). Enforcement is not bypassed. |
| Cold swap | The wrapper preserves the previous comparison semantics. Missing retained predecessor text still refuses `cold_swap_predecessor_unknown` before unload; a dropped invariant still refuses before unload (`operator_door.rs:1507–1567`). |

This establishes consistency of the **invariant decision**, not universal equivalence between precheck and every upgrade policy. Cold swap deliberately has additional recovery prerequisites and still uses retained text. Pre-existing halt/schema/migrator advisory limitations remain outside this correction.

### Verdict precedence

Reporting `SwapInvariantDropped` when a coordinator unsafe verdict also exists is acceptable for this reporting-only door: the result remains `safe:false`, terminal 2, and real enforcement reruns. It is **not exact diagnostic-order parity**: actual hot swap checks halt continuity before invariants. That distinction is documented and does not introduce an admission bypass.

### Disclosure, routing and serde handling

- Only the **first** missing predecessor name is returned. Names are validated, bounded declarations (`manifest.rs:4154–4183`); neither manifest nor the full preservation list is disclosed.
- `Completed(Value)` forwards the body as HTTP 200 (`maos-control/src/lib.rs:841–844`); CLI handling prints it and honors terminal 2 (`door_client.rs:370–383`). No dispatcher drops the new verdict.
- No production serde-family unwrap, expect, `unwrap_or` or `unwrap_or_default` was introduced. New defaults concern file I/O and declaration `Option`s. The serde gate excludes integration-test directories; the recorded correction gate reports zero violations.
- The corrected redaction plant now changes only `{hex}` to `{invariant}`. Its log records the persisted redaction marker at line 92, resolving the earlier appended-`0` evidence ambiguity.

## Witness assessment

- **Installed-component witness:** Strong end-to-end coverage. It establishes missing retained text through the cold-swap refusal, compares precheck with actual hot-swap refusal, checks unchanged pid, then proves a preserving successor prechecks and swaps successfully.
- **Staged stale-cache witness:** Adequate for the corrected **source-selection property**. It uses a real scheduler, SCB, coordinator and door while varying retained text. It does not exercise an actual swap/revert sequence, production vetting or actual-swap agreement; it must not be described as an end-to-end auto-revert witness. The record accurately discloses this limitation.
- **Malformed-scalar witness:** Correctly asserts typed parse refusal and absence of the structural verdict, with a no-invariant control. The control is not specifically a schema-7/no-section predecessor; that case is supported by source inspection rather than this dedicated runtime witness.
- **Plants:** `ignore-live-declaration` and `compare-before-kernel-parse` target the corrected dependencies directly. Recorded failures occur at the relevant assertions. `plants.json` lists three plants consistently.

## Accounting and verification limits

Pins are **38602 → 38628 (+26)**, within the supplied **+60** authorization; aggregate hardfail is **203277**, also +26. The ledger records aggregate measured **203276 + 1**. I checked the values and arithmetic, not an independent tokei measurement.

Read-only review only: **no builds, tests, gates or plants executed**. Saved evidence reports installed tests **2 passed**, stale-cache test **1 passed**, and daemon-door tests **48 passed / 1 ignored**. Hosted CI, real auto-revert execution, mutation restoration and fresh measurements were not independently verified. The recorded D11 ratchet remains **7 passed / 2 failed**, concerning unchanged xtask grant authorization—not this correction.