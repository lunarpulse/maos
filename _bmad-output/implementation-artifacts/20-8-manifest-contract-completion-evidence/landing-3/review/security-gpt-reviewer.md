## Verdict

**overall_correctness: correct**  
**confidence: 0.92**

No patch-introduced security blocker found in `1fd26dcc` or the uncommitted precheck change. The real swap remains fail-closed; the precheck improvement retains some pre-existing advisory limitations.

**Counts:** P0: 0 · P1: 0 · P2: 0 · P3 findings: 0. Three residual observations below are not new regressions.

## Security conclusions / rejected candidates

1. **Case, Unicode, whitespace and duplicate-name bypass:** rejected. `declared_names` requires 1–128 bytes of lowercase ASCII `[a-z0-9._:-]`, rejects duplicate entries and bounds lists to 32 (`crates/maos-manifest/src/manifest.rs:4154–4183`). Comparison is exact. TOML parsing rejects duplicate keys before comparison; variant section/key spellings cannot satisfy a correctly declared predecessor invariant.

2. **Malformed scalar interpreted as absence:** fixed for schema ≥7. The successor parser now propagates extraction/serialization errors through `transpose()?` (`crates/maos-kernel-core/src/lifecycle/upgrade.rs:386–397`). The door helper treats an invalid successor declaration as keeping nothing, so a predecessor declaration still produces a refusal (`crates/maos-bin/src/admission.rs:686–703`). With no predecessor declaration, the coordinator’s parser rejects the malformed successor.

3. **Schema downgrade bypass:** rejected. A successor below schema 7 has no effective declaration and therefore drops every effective predecessor invariant. A below-schema-7 predecessor’s declarations remain intentionally ignored under the documented degradation policy.

4. **TOCTOU bypass of the actual swap:** rejected. Upgrade bytes are materialized into a `create_new`, mode-0600 snapshot (`operator_door.rs:387–402`). Cold-swap comparison and admission use that snapshot; planned hot-swap hops also snapshot before the orchestrator (`migration_plan.rs:227–242`). A precheck result does not authorize a subsequent swap: enforcement runs again.

5. **New predecessor/arbitrary-file disclosure:** rejected. The new precheck response exposes only the first dropped, validated predecessor invariant (`operator_door.rs:1773–1785`), not either manifest’s contents. The route remains authenticated before request routing (`crates/maos-control/src/lib.rs:988–998`).

6. **Raw dropped-name bytes in the abort receipt:** rejected. Both the saga reason and its compensation rendering receive the labelled colon-separated hexadecimal string, not the raw name (`coordinator.rs:204–222`; `saga.rs:145–160`). Groups contain at most four hex characters, avoiding the redactor’s 32-character token heuristic.

7. **New serde unwrap/expect violation:** rejected. No production serde-family unwrap/expect was added. The new `unwrap_or_default` sites concern file I/O or an `Option`, not serde calls. Receipt-decoding `expect`s are in integration-test code.

8. **Dropped cross-boundary verdict:** rejected. `Completed(Value)` forwards the new verdict unchanged as HTTP 200 (`maos-control/src/lib.rs:841–844`), and the CLI consumes `terminal_code: 2` generically (`maos-cli/src/door_client.rs:370–383`). No enum dispatch silently discards `SwapInvariantDropped`.

## Residual observations — not patch-introduced findings

| Posture | Evidence and impact | Fix sketch / witness need |
|---|---|---|
| **P3 — incomplete precheck coverage** | The new comparison skips when `admitted_manifest` is absent (`operator_door.rs:1773–1777`). Installed boot admits the declaration into the SCB but does not populate that retained-text map (`installed_spirits.rs:90–119`; `main.rs:4454–4496`). Such a predecessor can still receive a safe advisory verdict for a dropping successor; real enforcement remains intact. This behavior already existed at the base. | Compare the live SCB declaration, or retain installed-boot manifests. Witness: boot an installed schema-7 component, then precheck a dropping successor. Do not claim the existing precheck deferral fully closed without this coverage. |
| **Inherited hostile-path exposure** | Precheck’s earlier vetting read is already unbounded and accepts nonregular files (`main.rs:138–154`), as does the coordinator loader (`upgrade.rs:320–330`). FIFO/device paths can block or exhaust resources; TOML diagnostics can echo target-file excerpts. The added read preserves these hazards rather than introducing them. | Separate hardening scope: bounded reads with file-type checks on the opened descriptor, performed off async workers. Witness FIFO/device, oversized-file and parse-error disclosure behavior. No hostile-file experiment was run. |
| **P3 — audit verdict detail absent** | The new unsafe precheck still receives an operator completion row, but `Completed` is logged as `completed`, without its verdict/cause (`operator_door.rs:2699–2750`). Cold-swap refusal records its typed conflict; real hot-swap refusal additionally writes `HotSwapAborted`. No phantom load/admit row is introduced. | If durable advisory detail is required, persist the verdict and bounded identifier. Witness both safe/unsafe precheck rows. This follows the existing completion-receipt convention. |

## Verification limits

Read-only source/diff review only. No edits, builds, tests, gate executions, race experiments, hostile-path probes or runtime audit queries were performed. Existing test and gate evidence was read, not independently reproduced.