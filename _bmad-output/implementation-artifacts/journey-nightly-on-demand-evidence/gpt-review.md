## Verdict: SHIP this patch for the RC candidate

**overall_correctness:** correct  
**confidence:** 0.97  
**Findings:** none. **P0: 0 · P1: 0 · P2: 0 · P3: 0**

1. **Expression semantics and cron safety:** Missing properties evaluate to `''`; loose equality converts `''` to `0` and `true` to `1`, so `inputs.rerecord == true` is **false** on schedule. Dispatch boolean inputs remain booleans in `inputs`, unlike strings in `github.event.inputs`. Both job guards independently reject schedule events. Skipped dependencies cannot open this path; a step-level `!cancelled()` or `always()` cannot resurrect a skipped enclosing job. Researcher runs after earlier step failures only within the admitted job and with its additional opt-in. See GitHub’s [expressions](https://docs.github.com/en/actions/reference/workflows-and-actions/expressions), [dispatch inputs](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onworkflow_dispatchinputs), and [job dependencies](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idneeds).

2. **Other paid paths:** Searching all workflows found exactly four secret bindings: the probe and three record steps in `journey-nightly.yml`. All three `MAOS_INFERENCE_MODE: record` assignments are inside guarded `tier-2-rerecord`. Other secret-name mentions are `unset` commands in `discipline.yml`. No additional scheduled binding or record-mode assignment was found. Both hermetic jobs retain the daily cron.

3. **Pin test:** Adequate for this cutover: eight plants cover the changed guards, defaults, Researcher opt-in, and retained scheduling; `assert_ne!` prevents ineffective replacements from silently passing. **Not entirely non-brittle:** mutation strings depend on exact formatting/descriptions, and condition comparisons pin expression spelling. It is a shape regression test, not a GitHub expression interpreter or exhaustive future-job scanner. These limitations do not block this patch.

4. **Provisioning wording/evidence:** It explicitly says the secret **name** was confirmed and its value **never read**; it does not claim credential validity or a successful live recording. The evidence README records prior checks and mutations, not live-run proof.

Read-only review completed; no builds, tests, mutations, or paid dispatches performed. Approval is limited to this patch, not overall release readiness.