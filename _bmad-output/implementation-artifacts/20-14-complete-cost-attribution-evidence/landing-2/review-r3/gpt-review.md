## Verdict

**Overall correctness: incorrect — one P2 accounting-evidence issue.** Both requested production fixes hold; I found no additional replay, erasure, or codec regression. The remaining finding concerns substantiating the claimed grants, not runtime behavior.

**Counts:** P0: 0 · P1: 0 · P2: 1 · P3: 0.  
**Confidence:** 0.96.

## Finding

### [P2] Cite the ruling supporting the final KLOC approvals

**Location:** `_bmad-output/implementation-artifacts/20-14-complete-cost-attribution-evidence/landing-2/review-r2/evidence.md:31`  
**Confidence:** 0.97

The new accounting row labels capability **1454**, domain **10274**, and CLI **6612** “Main-adjudicated,” but cites the previous GPT review’s overdraft assessment, which explicitly describes pending amounts and says measurement consistency is **not approval**. The pinned KLOC ledger also retains pending-adjudication wording, while the new deferred row extends the adjudicated claim to IAC **8773** despite the round-2 evidence placing its increase “for Main adjudication.” Consequently, the committed evidence does not establish whether these final numeric ceilings are granted or still awaiting approval. Cite the actual dated numeric ruling and reconcile the current records, or consistently retain pending status. **No executable witness is needed:** this is a contradictory approval record; an external Main ruling may exist, but the cited review does not substantiate it.

## Requested correction checks

- **P1 completed-forget replay: closed.** `operator_door.rs:2388–2392` now calls `subject_in_frame`. I traced that consumer through decoded discovery, replacement, legal-hold protection, and certification. Receipt exclusions, the Distillate rule, and the 4,096-row fallback remain intact.
- **P2 field-specific decoding: closed.** `transparency_log.rs:2319–2336` decodes only kind-29 `principal_id` and `task_ref` strings. Provider/model labels and governance strings retain literal matching.
- **Codec:** typed serialization escapes already-`x:` identities, preventing new-writer ambiguity; typed readers and erasure decode once. No additional reachable defect found.

## AC1–AC6

| AC | Assessment |
|---|---|
| AC1 | Supported by real-command witnesses, including same-principal/two-task separation. |
| AC2 | Root mint binding, token-state storage, narrow delegates, and verified-token emission are present. |
| AC3 | Boot-bound principal, proxy fallback, SR-4 count-only ambiguity, and SR-3 erasure are supported. |
| AC4 | Task/source/Spirit grouping, legacy reads, Worker-CLI exclusion, and sum-then-round remain present. |
| AC5 | Guides and ADR amendments state the binding and its honesty limits. Final hosted/doc-gate closure remains unverified. |
| AC6 | Witness references and pins are recorded, but approval evidence needs reconciliation. Done-state closures remain open; the story correctly stays `in-progress`. |

Thus, **not every completion requirement is established**, despite source-level support for the functional ACs.

## Witness and plant evidence

The round-2 reds are genuine **runtime assertion failures**, not compile failures:

- Replay red and plant fail on identical operation IDs at `erasure_million_20_11.rs:436`.
- Decode-every-string red and plant fail on `61: cost model`.
- Never-decode plant fails on `61: principal`.

Recorded greens show **IAC cover 2 passed**, replay suite **5 passed/1 skipped**, bin cost **11 passed**, full IAC **197 passed**, and focused bin **99 passed/1 skipped**. Restoration markers and summaries agree; they are recorded evidence, not independently authenticated restoration proof. Earlier capability/kernel compile-red logs establish missing APIs, not runtime failures.

## Accounting and scope

Recorded measurements reconcile: **IAC 8773**, **bin 38564**, aggregate **202920**, hardfail **202921**; kernel unchanged at **27044 physical / 21202 tokei**, leaving **798** below STOP 22000. Round 2 changes no kernel source or surface hash. The intervening 20-13 landing explains the changed measurement base.

## Rejected candidates

- `host-unallocated` display collision: pre-existing in the supplied cumulative baseline.
- Ignored bind result: freshly issued tokens are present; no reachable failure demonstrated.
- `pid5.pass` versus `pid:5`: cosmetic, not misattribution.
- Codec context through arrays: not a supported typed identity-field shape.
- Legacy codec ambiguity and incomplete Unicode refusal: already disclosed residuals.
- Different measurement bases: explained by intervening commits, not erroneous accounting arithmetic.

## Limits

No builds, tests, plants, or gates were rerun. Hosted execution, clean-tree `abi-diff`, prepush, WASM/platform legs, million-row pending cases, and external adjudication authority remain unverified. No files were modified.