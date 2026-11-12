# Review result

**Overall correctness: incorrect.** Two P2 issues remain in the L2 machinery. No P0/P1 issue was established. One P3 assurance correction should be deferred under the operator policy.

**Confidence: 0.94.** Static review only: no cargo, tests, workflow dispatches, file writes, or git mutations.

## Findings

### F1 — P2: Restrict the empty-ledger fallback to the branch lookup
**Anchor:** `.github/workflows/release.yml:183–190`  
**Confidence:** 0.99

The shared error handler treats HTTP 404 from either API request as “perf-ledger branch absent.” If the branch lookup succeeds but `ledger.jsonl` is missing at the returned commit, the contents request returns 404; the workflow replaces that failure with an empty ledger. Consequently every metric becomes ABSENT and an rc accepts a broken ledger as legitimate absence.

**Fix sketch:** Handle branch lookup separately. Only its 404 may create an empty ledger; once the branch exists, every contents-fetch failure must refuse publication.

**Witness needed:** Extend the executed-shell harness with branch lookup success followed by contents HTTP 404; require failure before `check-release-precondition`. The current stub makes the contents request succeed unconditionally (`xtask/tests/release_gates_20_5r1.rs`, committed version:191).

### F2 — P2: Reconcile the ABSENT label with the computed perf verdicts
**Anchor:** `xtask/src/check_rc_readiness.rs:82–86`  
**Confidence:** 0.95

The readiness leg requires the literal `Perf verdicts: ABSENT` but never checks its truth. The subsequent perf leg accepts all-PASS or mixed PASS/ABSENT reports for rc (`xtask/src/check_release_precondition.rs:70–78`). Thus an rc whose notes assert ABSENT—and whose 20-7 waiver relies on that assertion—can pass while actual verdicts exist. This is explicitly permitted by the existing all-PASS test, so the two independent gates do not establish label honesty.

**Fix sketch:** Reconcile the committed label with the computed report in the CI-evidence stage. Preserve the intended PASS/ABSENT acceptance policy, but reject contradictory disclosure rather than treating the label alone as evidence of absence.

**Witness needed:** A combined vector with the exact ABSENT notes line and an all-PASS report, plus a mixed report; assert that the published disclosure agrees with the report.

### F3 — P3: Describe tag-push authority instead of private-key possession
**Anchor:** `docs/release/decisions-2026-11-12.md:16–17`  
**Confidence:** 0.99

The new statement that the signed-tag step “needs the key holder” overstates the enforced authorization boundary. The procedure uses ordinary `git tag` (`docs/release/tag-procedure.md:205`), while CI receives `RELEASE_SIGNING_KEY` automatically (`.github/workflows/release.yml:209–214`). With an unprotected release environment, an actor authorized to push an accepted tag can trigger artifact signing and publication without possessing the private key, provided the other gates pass. No tag-signature verification or key-holder approval is present.

**Fix sketch:** Distinguish the operator-only policy from enforcement: tag-push authorization triggers CI-held artifact signing; private-key possession is not checked. Do not claim cryptographically signed Git tags without implementing that separate requirement.

**Witness needed:** Documentation correction needs no live signing experiment. Any behavioral witness should use disposable keys and an isolated repository.

## Gate and security assessment

- **403, rate-limit and 5xx failures:** They refuse rather than becoming empty ledgers, absent a 404 diagnostic. The actual fail-open case is **F1**, not a generic transient-error bypass.
- **20-7 waiver:** Correctly limited to the named key, `in-progress`, an rc workspace version, and the exact perf label (`check_rc_readiness.rs:93,109–116,217`). `done` passes normally and is not reported WAIVED; backlog/review/non-rc do not receive the waiver.
- **Tag ancestry:** The unchanged workflow rejects commits outside `origin/main` ancestry before signing (`release.yml:116–124`). It allows older main ancestors, not only the tip. This check is not an independent defense against a tag writer changing the workflow itself.
- **Fuzz evidence:** The workflow resolves and logs the ledger tip, then fetches by that commit. No moving-ref second read was found. Matching uses full string equality, not a commit prefix (`check_release_precondition.rs:133`). Three targets, five distinct links per target, per-run grouping, maximum duplicate measurement and the 86,400-second threshold are enforced.
- **Aggregate conclusion:** Only `completed` plus `success` passes (`check_release_precondition.rs:199–209`). Failure, skipped and in-progress do not.
- **Aggregate workflow identity:** A different workflow/check producer named exactly `aggregate` is not distinguished: the parser retains no workflow identity and filters only by name. This weakness exists before L1, so it is **not an introduced finding**.
- **Interpolation:** Tag text enters through `env.TAG` and is quoted. Classifier output is restricted to `rc`, `ga`, or `alpha`; remaining interpolations are repository identity, commit SHA or static matrix values. No patch-introduced shell/YAML injection was established.
- **Permissions:** Publishing requests `contents: write` and `checks: read`; it does not request `id-token: write`. Fuzz collection has repository-content write authority. An unprotected `environment: release` supplies no reviewer authorization.
- **Signing/logs:** The reviewed signing code does not print the seed or pass its value on the command line (`xtask/src/release_verify.rs:169–184`). It receives the secret through the signing steps’ environment. No introduced secret-log exposure was found; live masking and dependency behavior were not verified.

## Tag classification and GA

| Tag | Classification | Result |
|---|---|---|
| `v1.0.0-rc.1` | rc | Readiness, aggregate, T3 and perf; ABSENT allowed; prerelease |
| `v1.0.0` | ga | Aggregate, T3 and all-PASS perf; no rc-readiness run; not prerelease |
| `v1.0.0-foo` | rejected | Cannot reach publication |

`contains(github.ref_name, '-')` is consistent with the accepted classifications.

**GA intentionally does not refuse for Holds 1–2.** The dated procedure and `RELEASE-HOLDS.md:86–107` consistently describe them as disclosure boundaries. Treating their removal as a defect would contradict the ruling. GA also does not execute the rc-only tree-readiness gate; that is explicit in the workflow, not an accidental dispatch omission.

## Tests and plants

The four requested test files exercise meaningful boundaries:

- Readiness tests isolate all six legs and test exact labels, status/version waiver restrictions, provisioning WARNs and advisory-token matching.
- Perf vectors cover PASS, ABSENT permission, FAIL/INDETERMINATE, foreign SHA, empty/malformed reports and the flag dependency.
- T3 vectors cover missing targets/links, T2 exclusion, per-run isolation, duplicates and the exact duration threshold.
- Workflow tests execute extracted shell, including classification, ancestry and API-error branches—not merely YAML substrings.

Persisted p1–p10 logs contain the advertised failing tests and restoration records. This is **inspection of recorded evidence, not a reproduced mutation run**.

Remaining coverage limits:

1. F1’s branch-success/content-404 case is absent.
2. No combined notes-versus-real-verdict truth test catches F2.
3. The foreign-SHA T3 vector uses an unrelated SHA; it would not catch weakening equality to a shared-prefix comparison. Add two full SHAs sharing the compared prefix.
4. The workflow harness stubs cargo; individual real-binary tests do not constitute an end-to-end producer/consumer JSON round trip.
5. Explicit 403/rate-limit vectors are absent, although the inspected control flow refuses them.

These coverage observations are not additional correctness findings.

## Serde error handling

No new production serde-result `unwrap`, `expect`, `unwrap_or`, or `unwrap_or_default` violation was found.

- New deserializations propagate errors with `map_err(...)?`.
- `conclusion.as_deref().unwrap_or("null")` formats an already-parsed optional field; it is pre-existing and does not suppress deserialization failure.
- `deny_leg`’s `unwrap_or_default` handles absent optional configuration after successful TOML parsing.
- Test-directory serde unwraps are outside the production gate’s scan.

The serde gate itself was **not run**.

## Eight release-note claim spot checks

Here, `E/` means `_bmad-output/implementation-artifacts/`. “Supported” means the repository source agrees—not that hosted evidence was independently authenticated.

| Draft claim | Repository evidence | Assessment |
|---|---|---|
| No rc LTS clock; one year begins at GA (`v1.0.0-rc.1.md:10–11`) | `docs/release/lts-policy.md:14–17` | Supported |
| Pen-test/ECCN unclaimed (`:17–20`) | `RELEASE-HOLDS.md:29–42` | Supported disclosure |
| Main run 43150757804, 199/199 (`:44`) | `E/20-2-deb-airgap-docker-and-formula-corrections-evidence/closure/closure-note.md:6` | Matching record; hosted result unverified |
| Packaging run, development-signed dist (`:48`) | Same closure note `:5,10–20` | Supported, with publication boundary |
| Schema 7 and hosted manifest plant (`:49`) | `E/20-8-manifest-contract-completion-evidence/closure/closure-note.md:10–18,25–28` | Supported; branch/tree limits disclosed |
| Crypto proof and detected plant (`:50`) | `E/20-13-pluggable-crypto-provider-selection-evidence/closure/closure-note.md:10–11,35–40` | Supported; Worker T3 is x64-only |
| Million-row/live-Worker erasure (`:51`) | `E/20-11-erasure-inventory-closure-evidence/closure/closure-note.md:7–19` | Supported; whole-test wall is not forget latency |
| Kernel/surface reduction (`:52`) | `E/21-5-orchestrator-port-and-move-evidence/closure/closure-note.md:12–17` | Matching recorded measurements |

## Rejected candidates and scope notes

Rejected as introduced defects: non-success aggregate acceptance; prefix matching in the implementation; waiver for backlog/non-rc/done; generic 403/5xx fail-open; stale moving-ref fuzz fetch; malformed perf JSON becoming ABSENT; intentional environment WARN; and GA requiring Holds 1–2.

At **d00802aa**, committed notes were not supplied to the GitHub Release action. During review, an unrelated working-tree overlay added `body_path`, an rc notes-file guard and a corresponding test. That overlay was left untouched; its execution is unverified and it is not counted as an outstanding finding above.

## Counts and unverified evidence

- **P0: 0 · P1: 0 · P2: 2 · P3: 1**
- **Correction round:** F1–F2.
- **Deferred assurance row:** F3.
- **Eight claim traces checked.**
- **Unverified:** hosted run conclusions/log authenticity, live secrets/environment/tag protections, live ledger tips, actual signing/publication, T3 completion, and all newly suggested witnesses. No checks were executed.