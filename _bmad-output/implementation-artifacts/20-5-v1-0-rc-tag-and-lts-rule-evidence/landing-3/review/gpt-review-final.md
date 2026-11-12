# Review result

**Overall correctness: correct.** Both earlier P2 findings are closed. No new P0–P2 defect or fail-open was established; one documentation inconsistency is deferred as P3.

**Confidence: 0.96.** Read-only review of the 15-file uncommitted diff and relevant consumers. No builds, tests, dispatches, signing, or file writes were performed.

## Correction round

### F1 — Closed: branch-only empty-ledger fallback

`.github/workflows/release.yml:188–198` now separates the requests:

- Branch lookup HTTP 404 creates an empty ledger.
- Other branch-lookup failures refuse publication.
- Once the branch exists, **every contents-fetch failure refuses**, including HTTP 404.
- Contents are fetched at the logged commit, not a moving branch reference.

The new executed-shell vectors cover branch 404/403/rate-limit/500 and branch-success followed by contents 404/403/500.

### F2 — Closed: notes-versus-verdict reconciliation

`.github/workflows/release.yml:208–247` runs the new rc-only reconciliation before signing. An exact ABSENT label plus **any non-ABSENT row**, including PASS, refuses publication.

The “no exact label” no-op does not bypass enforcement: the earlier readiness step requires that label, checks tag/workspace-version equality, and therefore reads the same notes file. Empty, missing, malformed, or structurally invalid verdict reports refuse through the precondition and/or reconciliation step.

## Requested failure-mode checks

| Question | Assessment |
|---|---|
| Real `gh` 404 stderr | For the standard missing-branch JSON response, the expected diagnostic is `gh: Branch not found (HTTP 404)`. The checked CLI implementations append `(HTTP <status>)` to the API message; “Not Found” versus “Branch not found” does not affect this grep. [Current implementation](https://github.com/cli/cli/blob/trunk/pkg/cmd/api/api.go), [v2.60.1](https://github.com/cli/cli/blob/v2.60.1/pkg/cmd/api/api.go), and [v2.0.0](https://github.com/cli/cli/blob/v2.0.0/pkg/cmd/api/api.go) agree for normal JSON errors. This was source inspection, not a live `gh` execution. |
| Is stderr grepping robust? | Adequate for the normal GitHub response and checked versions, but not a structured HTTP-status contract. A diagnostic omitting the token causes **refusal**, not fail-open. No concrete new misclassification was found. |
| Stub fidelity | The error strings reflect that normal CLI formatting. The stubs omit the error-response JSON that real `gh` also writes to stdout, but that stdout becomes an unused failed-lookup `tip`; it does not change the fallback decision. |
| Harness `bash -c` versus hosted shell | Both relevant extracted scripts explicitly execute `set -euo pipefail`. Thus GitHub’s default `bash -e {0}` does not create the proposed pipefail discrepancy. The contents-error branch exits explicitly. [GitHub shell contract](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idstepsshell) |
| `check-perf-verdict … || true` | This is unchanged. Its stdout file is subsequently parsed by `check_release_precondition.rs:58–87`, which requires a matching SHA, nonempty rows, and accepted verdict strings. Empty/malformed crash output cannot become ABSENT. |
| Python quoting/injection | `<<'PY'` prevents shell expansion inside Python. TAG reaches a quoted argv value, not Python source. |
| Tags containing `/`, `..`, or shell syntax | The earlier anchored classifier, `release.yml:133–137`, rejects them before the new notes paths are consumed. |
| `body_path` plus generated notes | The pinned action reads the file and prepends its contents to generated notes, separated by two newlines. Existing-release updates also consume the supplied body. [Pinned implementation](https://github.com/softprops/action-gh-release/blob/3bb12739c298aeb8a4eeaf626c5b8d85266b0e65/src/github.ts) |
| Unreadable `body_path` fallback | The action warns and falls back to `body`, then generated notes when no body exists. The new `-s` guard catches missing/empty files; importantly, rc readiness and reconciliation have already successfully read the required notes. No intervening notes mutation was found. The guard alone is not a general readability check, but the complete current path is sufficient. [Pinned file loader](https://github.com/softprops/action-gh-release/blob/3bb12739c298aeb8a4eeaf626c5b8d85266b0e65/src/util.ts) |

The aggregate-shell harness still stubs cargo and is **not** an end-to-end release test. The added real-producer round trip verifies the all-ABSENT JSON/label interface; separate real-precondition vectors cover malformed reports and rejected verdicts.

## Version bump

The exact zero-context diff and numstat establish:

| Lockfile | Version substitutions |
|---|---:|
| Root `Cargo.lock` | **48** |
| Domain fuzz | 2 |
| Exec-deps fuzz | 1 |
| Manifest fuzz | 4 |
| Native twin | 5 |

Every changed lockfile line replaces `0.1.0-alpha.1` with `1.0.0-rc.1`. **No source, checksum, dependency, or other line changed.** The standalone substitutions occur in sourceless workspace-package blocks.

The native-twin `"tokio"` dependency is already committed in **fe44017a**; it is not part of this patch.

Other checks:

- The 48 inherited package versions agree with the workspace bump. The other 11 workspace members retain independent Spirit/example versions.
- Package rendering already maps rc.1 to **`1.0.0~rc.1` for Debian/RPM** and **`1.0.0rc1` for AUR**. Download URLs retain `v1.0.0-rc.1`; Debian changelog rendering uses the normalized token.
- `maos --version` and its relevant assertion derive from `CARGO_PKG_VERSION`.
- Existing alpha minimum-substrate requirements remain lower than this candidate under the actual hand-rolled comparison. High-version negative fixtures remain negative.
- ABI/surface baselines are not tied to workspace package-version equality. Schema `$id`s identify schemas, not this release.
- No workspace-version constraint in cargo-deny or internal path dependencies was found.
- STABILITY.md changes exactly the kernel-version and release-channel rows.

### Deferred P3 — Refresh the copied current-version tables

**Patch anchor:** `Cargo.toml:68`  
**Priority:** 3 · **Confidence:** 0.99

The bump leaves `docs-site/docs/migrate/abi-stability.md:21` and its Korean counterpart at `:22` advertising `0.1.0-alpha.1` as the **current** kernel version sourced from Cargo.toml. Readers now receive a value inconsistent with the candidate and STABILITY.md. Update both copied rows or reference the generated matrix instead. A static comparison is a sufficient witness; no execution is needed.

This is documentation-only and does not change the correctness verdict. The already-stale schema-4 row is pre-existing and is not counted separately.

## Release-note honesty

Both exact labels remain intact at **`docs/release/v1.0.0-rc.1.md:27–28`**.

The release-note claims trace to repository records:

| Notes content | Trace |
|---|---|
| Proof table, lines 47–57 | Closure notes for 20-1b, 20-2, 20-4, 20-8, 20-11, 20-13, 20-14 and 21-5; 20-9 remains explicitly in its closure round |
| Boundaries 1–6 and 25 | RELEASE-HOLDS, LTS policy, dated release decisions, provisioning checklist, and actual release gates |
| Boundaries 7–24 | Corresponding closure notes, including quarantine limits, timing amendments, platform limits, model-family exceptions and unclaimed publication |
| Boundary 26 and lifecycle limitations | 20-9 story/evidence, including `f1-guest-stdio/RESULT.md` and `ci/run-43193905404.md` |
| Operator rows | Sprint-status backlog rows and provisioning inventory |
| Residual limitations and upgrade bullets | The cited closure notes, including schema 7, WIT 2.4.0, crypto selection and surface reduction |

The corrected manifest-run identity matches the 20-8 record. The revised T3 paragraph correctly describes **worker-seconds**, consistent with `record-fuzz.py` summing worker counters; it makes no completion claim.

The signing clarification is consistent across the notes, decisions and procedure: an ordinary tag push triggers **CI-held artifact signing**; key-holder-only tagging is policy, not enforced private-key possession or Git-tag-signature verification.

`SECURITY.md:56,60` and the PRD DELTA agree with NFR-Maint-6: one-year commitment from GA, security-only after year one, and a two-year term deferred to the v1.5 decision.

## Recorded witnesses and remaining limits

Inspected landing-3 records show:

- Seven new tests red before correction.
- Final focused record: **76 passed, zero skipped**.
- Four mutation records red, each ending with restoration.
- Recorded serde gate: zero violations; repin check: in sync.

These are **recorded results, not checks rerun by this review**. Changed Rust is test code; no new production serde-result unwrap/expect/default suppression was introduced.

The recorded readiness command remains red for **20-9 in-progress**. Tag mode additionally requires 20-5 itself done. A clean patch review therefore is **not rc-tag authorization**.

### Rejected candidates

- Adjacent label lines rendering as one Markdown paragraph: unchanged lines, no gate defect.
- Historical alpha examples, “since version” entries and negative fixtures: not stale live pins.
- Missing-label reconciliation no-op: covered by the earlier readiness gate.
- GA not requiring Holds 1–2: deliberate ruling, not an omission.
- Preserved historical “needs the key holder” sentences: explicitly qualified by the new dated corrections.

## Counts

**P0: 0 · P1: 0 · P2: 0 · P3: 1 deferred documentation row.**

**Unverified:** hosted execution of this patch, installed runner `gh` version, live repository protections/secrets/ledger state, final-SHA T3 completion, packaging execution, signing/publication, and hosted-run authenticity beyond the inspected records.