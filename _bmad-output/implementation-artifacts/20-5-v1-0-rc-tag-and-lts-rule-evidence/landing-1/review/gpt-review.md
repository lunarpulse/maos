## Verdict: incorrect

**3 findings: 3 P2; no P0/P1.** Reviewed committed content at `c3dcc7bd`; no builds, tests, or plants executed. No write tool was available, so this message is the report.

### 1. Reject pre-release tags in the LTS clock record

- **Priority:** P2
- **Confidence:** 0.98
- **File:** `/home/lunarpulse/dev_ws/xtask/src/stability_matrix.rs:388–392`

`tag_ok` accepts any nonempty plain tag name, including `v1.0.0-rc.1`. A record pairing that tag with a valid hexadecimal SHA therefore renders an LTS clock-start, even while the release-channel row says the RC clock has not started; regenerating STABILITY.md then makes `--check` accept the contradiction. Require `clock_start_tag == "v1.0.0"` for this v1.0 clock record. **Failing-before witness needed:** valid-SHA records naming an RC, alpha, or unrelated GA tag must be rejected; the `v1.0.0` record must remain accepted.

### 2. Propagate failures from the hosted readiness proof

- **Priority:** P2
- **Confidence:** 1.00
- **File:** `/home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-5-v1-0-rc-tag-and-lts-rule-evidence/ci/ci-20-5-verify.yml:55–58`

The readiness step saves the command status but never asserts it, and its final `echo` returns success. Because errexit is disabled, invalid or absent JSON also fails only the Python command, not the step. Consequently, both an always-green readiness regression and a compilation failure can produce a green hosted proof. Explicitly require the expected refusal status and validate the report identity, five legs, and `passed: false`, propagating validation failures. **Failing-before witnesses needed:** an all-green report, empty output, malformed JSON, and a command/tool failure must each fail this step.

### 3. Distinguish a T3 policy refusal from command failure

- **Priority:** P2
- **Confidence:** 1.00
- **File:** `/home/lunarpulse/dev_ws/_bmad-output/implementation-artifacts/20-5-v1-0-rc-tag-and-lts-rule-evidence/ci/ci-20-5-verify.yml:39–42`

Every nonzero `cargo run` result is treated as the required T3 refusal. A build failure or an older binary rejecting `--fuzz-ledger` therefore passes this proof without executing the chain check—the latter failure is present in the committed failing-before logs. Build separately with mandatory success, then require the gate’s refusal exit status and diagnostic naming the missing T3 chain and candidate SHA. **Failing-before witnesses needed:** compilation failure and unsupported-option output must fail; the genuine missing-chain refusal must pass.

## Remaining review results

- Schema peel preserves strict parsing, rejects wrong types without coercion, bounds the reported literal, and reaches the standalone, topology, and receiving-host spawn paths.
- Production ledger fetching fails closed; per-run grouping prevents cross-run summation and deduplicates links.
- Variant route mappings are consistent across slugs, catalog URLs, static checks, and Playwright. The baseline increase is consistent: `137 + (44 − 6) + 1 = 176`.
- Recorded focused/workspace counts and plant outcomes agree with their logs; no residual `PARTIAL` fragments were found. This does not independently establish reconstruction byte fidelity.
- Recorded pins remain within the ruled ceilings. RUSTSEC-2026-0459’s ignore predates this patch; the two newly documented unmaintained ignores remain explicitly open.
- No new platform-specific dependency was found in the production xtask additions.

**Operator decisions remain outstanding:** ABI ratification—the row is correctly `proposed`—hosted proof, candidate-SHA T3 dispatch, L2 completion, and eventual tagging/signing. This commit does not authorize a tag or start an LTS clock.