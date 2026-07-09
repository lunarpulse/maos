# Reality-Check Review — 15-full-spectrum-v2-2.md + 2026-07-06 companion edits

**Lens:** every factual claim verified against the repo at HEAD (branch `epic-11`, 2026-07-06), not taken on assertion.
**Deliverable:** `_bmad-output/planning-artifacts/architecture-maos-minimal-opus/15-full-spectrum-v2-2.md` + dated edits to `appendix-d-terminal-shape-sketches.md`, `14-open-questions.md`, `13-phased-roadmap.md`, `10-journey-traceability.md`, `12-architecture-decision-records.md`.

**VERDICT: PASS-WITH-FIXES.** The large majority of claims were genuinely reality-checked — gate names/dispositions, ADR divergences, the 23081 pin, the 11.2b/10.4a precedents, and the OQ dispositions all verify against the repo, several verbatim. The fixes cluster in two places: story-status overclaims in the "shipped by Epic 11" framing (§15.1 / §13 v2.0 row), and §15.5's KLOC numerology, which misreads the instruments it cites in three distinct ways.

---

## 1. Findings requiring fixes

### F1 (HIGH) — "SHIPPED by Epic 11" overclaims three unshipped stories and a nonexistent ADR

Evidence: `_bmad-output/implementation-artifacts/sprint-status.yaml` (last_updated 2026-07-06) and `docs/adr/` listing.

- Actual Epic-11 story states: 11.0/11.1a/11.1b/11.2a/11.2b/11.3/11.4a **done**; **11.4b done 2026-07-06** (same-day close — see note below); **11.4c `ready-for-dev`** (story *created* 2026-07-06, zero implementation, preflight party-mode still "RECOMMENDED before dev-story"); **11.5 `backlog`**; **11.7 `backlog`**; 11.6 dropped.
- **`docs/adr/` contains NO ADR-051 file.** The live registry tops out at ADR-050. ADR-051 is a *reservation* named inside the 11.4c sprint-status entry ("NEW ADR-051") — nothing more.
- §13 v2.0 row header says "**SHIPPED by Epic 11, 2026-07**" and its Adds list includes "enterprise identity/at-rest/SIEM (ADR-051, Story 11.4c); FKCS infrastructure + v2.0 trial infra" — i.e., it claims as shipped: a ready-for-dev story, two backlog stories (11.5, 11.7), and an ADR that does not exist.
- §15.1's table is titled "What v2.2 stands on (**already shipped by Epic 11**)" and row 5 lists "enterprise identity/at-rest/SIEM (ADR-051, Story 11.4c)" as consumed substrate. v2.2 designs (§15.3/§15.6 Enterprise reference Spirit) lean on 11.4c work that has not been built.
- Additionally, Epic 11 is on branch `epic-11`, un-merged; sprint-status is explicit that merge to the shippable line is still gated on the two external v1.5 holds. §13's row does disclose the holds (good), but "SHIPPED" remains wrong for ~3 of ~11 stories.

**Fix:** §15.1 and the §13 v2.0 row must split "shipped (11.1a–11.4b, gates green)" from "in flight (11.4c ready-for-dev; 11.5/11.7 backlog)"; cite ADR-051 only as "reserved, lands with 11.4c." Note for the caller: the review brief's premise that 11.4b is `review` is itself stale — the repo says 11.4b closed **done** 2026-07-06 with ADR-024 flipped binding-v2.0, so the escape-detector rows in §15.1/§13 are *correct*; the overclaim is confined to 11.4c/11.5/11.7/ADR-051.

### F2 (HIGH) — §15.5's "NFR-Maint-1 breached in letter" uses the wrong instrument; the breach is unproven and plausibly false

- NFR-Maint-1 (`prd/non-functional-requirements.md:130`): "Kernel trusted core ≤ 20 KLOC **excluding tests** through v2.0."
- The 23,081 figure §15.5 cites is `xtask/kernel-core-baseline.toml` `src_lines` — a raw line count of `maos-kernel-core/src` that **explicitly includes in-src test code and doc comments** (the file's own HISTORY: 9.4b "+185 … merge-blocking TEST code the line-count gate includes"; 11.2a "+59, **incl. test LOC** … remainder is doc comments + test code").
- The tests-excluding production measure the repo actually maintains (`xtask/kloc.toml`, tokei "production Rust code only") recorded `maos-kernel-core` at **17,687 LOC at the Epic-10 retro (2026-06-26)** — comfortably **under** 20 KLOC. Post-Epic-11 kernel deltas are ~+76 src lines total (23023→23081, mostly test/doc per HISTORY), so production LOC remains ≈17.7K.
- Consequence: ADR-055's rationale ("restating ≤20K is rejected as **already-false**") rests on a metric mismatch. By the NFR's own excluding-tests letter, ≤20K may still be TRUE. The reality check §15.5 opens with was itself not reality-checked against the instrument the NFR names.

**Fix:** restate with both instruments explicitly: "23,081 raw src lines (baseline gate, includes in-src tests/doc); ≈17.7K production LOC (kloc.toml measure, the NFR's excluding-tests letter) — under 20K on the NFR's own metric, but with ~2.3K headroom that Phase-3/4 debt makes unearned." The ADR-055 decision (pin-protocol-as-instrument) survives; its "already-false" justification does not.

### F3 (HIGH) — "≤25 KLOC kernel-crate-set, alarm at 23.5K" names no existing instrument, and §15.7's gate column points at the wrong keys

- `xtask/kloc.toml` has exactly two aggregate keys: `_aggregate_alarm = 16000` and `_aggregate_hardfail = 103000` — both cover the **entire workspace** (measured 102,171 at Epic-10 retro), not any kernel-crate-set. No "kernel-crate-set aggregate" key exists anywhere in xtask.
- §15.7 row 055's Gate column says "`check-kernel-baseline` + `xtask/kloc.toml` aggregate" — `check-kernel-baseline` gates one crate's raw src count (23081), and the kloc aggregate gates the whole workspace at 103K. **Neither measures what ADR-055 constrains.** The "alarm at 23.5K" is a NEW instrument that would need a new key + xtask logic; the text never says so, and as written it invites misreading the existing `_aggregate_alarm` as related (it is not — the brief's suspicion is confirmed: this is a new instrument presented without declaring itself one).
- The metric identity is also unpinned: 23.5K vs the baseline-style raw count (23,081 today = ~400 lines of headroom for a *crate set*, absurd) vs tokei production LOC (~17.7K core today = generous) give wildly different meanings.

**Fix:** ADR-055 must (a) declare the kernel-crate-set aggregate as a NEW `kloc.toml` key with its enumerated member-crate list, (b) pin which counter (tokei production LOC vs baseline raw src) defines the 25K/23.5K numbers, (c) correct the §15.7 gate column.

### F4 (MEDIUM) — §15.5's "per-crate ceilings deliberately unraised" is contradicted by kloc.toml history, and the "residual ≤6,000" target rests on stale arithmetic

- "ADR-038 per-crate ceilings are not raised … deliberately unraised": `xtask/kloc.toml` records the Epic-10 retro (2026-06-26) raising `maos-kernel-core` **17000→17750**, plus xtask 15000→22500, maos-cli 2000→3750, maos-domain 7000→8100, maos-iac, maos-manifest, maos-bench, and the aggregate **90000→103000**. The lived discipline is "raised only via retro-ratified tight-measured-residual bumps" — a defensible discipline, but not "unraised." Only the *original 6,000 decomposition target* was never amended.
- "Residual `maos-kernel-core` ≤ 6,000 LOC … per the phased plan in `xtask/kloc.toml`": the plan's "~5,400 post-Phase-4 residual" was computed from **Epic-5-era sizes** (crate at ~21,370 pre-Phase-1/2). At the Epic-10 measured 17,687, the remaining planned extractions (Phase 3 ~1,000 + Phase 4: scheduler 1,961 + memory 1,659 + hot-swap 1,317 + supervision 569 = **~6,506**) leave a residual of **~11,181 — nearly 2× the 6,000 target**. Committing "decomposition completes inside the v2.2 wave" at ≤6,000 requires either substantially more extraction scope than the kloc.toml plan lists, or an honest restatement of the reachable residual.

**Fix:** reword point 2 to "never raised to fit, only to retro-ratified measured residuals; the 6,000 decomposition target itself never amended," and re-derive point 3's residual from current numbers (or scope the extra extractions explicitly).

### F5 (LOW) — minor precision items

- §15.3's `team_guard` inherits `region_guard` "ratified placement" correctly, but the shipped `region_guard` is 2-operand and enforces provenance-**presence**, not cryptographic validity (11.2b review Decision D1, Refined-A; forged-stamp-is-served is a documented residual threat with a named v2.x successor: trusted-applied-root-registry). If `team_guard` copies the pattern for a *tenant* wall — a stronger adversarial setting — §15.3 should carry the D1 caveat forward rather than silently inherit presence-semantics.
- 11.4c is `ready-for-dev` (created 2026-07-06), not `backlog` — worth stating precisely wherever status is cited.
- §14 OQ-1 disposition: "HSIS 300-scenario composite" is a derived figure (NFR-Rel-3: 6 class corpora × 50 scenarios); correct, but worth a citation since HSIS is elsewhere always quoted as "50-scenario per class."

---

## 2. Claims verified CORRECT (reality-checked and confirmed)

| Claim | Verified against | Result |
|---|---|---|
| Kernel-core pin = **23081**, zero-Δ-by-default, every delta a FLAG-Winston HISTORY entry | `xtask/kernel-core-baseline.toml` (`src_lines = 23081`; full HISTORY ledger 15505→23081) | CONFIRMED |
| All six v2.0 gates exist: `check-wasm-form-equiv`, `check-cross-region-consensus`, `check-multi-region-slo`, `check-scale-churn`, `check-enterprise-pdp`, `check-escape-detector` | `xtask/gate-registry.toml` gates list + `[[ship_gate]]` blocks | CONFIRMED — each carries `{ v1_0 = "advisory", v1_5 = "advisory", v2_0 = "blocking" }`, exactly as §13's v2.0 row claims; `check-escape-detector` present (11.4b closed done 2026-07-06); `check-cross-form-equiv` correctly relabeled CLI-wrapper advisory |
| §12.0.1 divergence: live ADR-031 = WASM component-model form, binding-v2.0 | `docs/adr/index.md`, `ADR-031-wasm-component-model-spirit-form.md` | CONFIRMED |
| §12.0.1 divergence: live ADR-039 = per-module unsafe-code policy (planning's "reserved for predicate stdlib" stale); App-D.3/§15.8 fresh-number note | `docs/adr/ADR-039-per-module-unsafe-code-policy.md` (binding-v0.1) vs planning §12 "Reserved: ADR-039" | CONFIRMED — the STALE marker added to planning §12 is accurate |
| §12.0.1 divergence: live ADR-040 = rust-inproc measurement gate (binding-v0.5, superseded by ADR-031); planning's 040 = threat-model split | `docs/adr/ADR-040-*.md`, index; planning §12 line 44 ("040 Threat-model split Sec-14a/14b") | CONFIRMED |
| §13.1 disposition: J4 <10ms met by subprocess, "proven by the 10.4c real-kernel harness" | sprint-status `10-4c…: done` — real in-kernel scalar.tap, P95=1µs at HEAD, gate renamed `check-j4-latency` | CONFIRMED |
| ADR-049 content: `canonical_kv_leaf`, `CrossRegionReplicationBundle`, per-region Merkle/payload/row-count oracle, `region_guard`, TL-anchored = signing key not frame-crossing, CrossRegionReadmit re-pin | `docs/adr/ADR-049-cross-region-collective-memory-consensus.md` | CONFIRMED verbatim |
| ADR-050 content: out-of-kernel `PolicyDecisionPort` + in-process Cedar, fail-closed, deny-wins subtract-only, 7-leg gate | `docs/adr/ADR-050-enterprise-pdp-integration.md` | CONFIRMED |
| ADR numbers **052–055 unused** | `docs/adr/` directory listing (max = 050; no 051–055 files) | CONFIRMED |
| ADR-051 "reserved by Story 11.4c" (as a reservation, not a landed ADR) | sprint-status 11.4c entry ("NEW ADR-051") | CONFIRMED as reservation — but see F1 for the "shipped" framing |
| 11.2b precedent: `region_guard` is store-internal `LoomLiteStore` below `CollectiveMemoryPort`, explicitly NOT `DowngradeRouter`, NOT kernel `ReadEntryPoint`, with BLOCKING chokepoint grep-proof | `_bmad-output/implementation-artifacts/11-2b-cortex-3-region-pilot-multi-region-slo.md` (F4, D4, AC4) | CONFIRMED verbatim (see F5 for the D1 presence-vs-validity nuance) |
| 11.2b precedent: distinct-`datname` witness (`SELECT current_database()`, A≠B≠C hard-fail) + pre-replication physical-absence controls | same artifact (F2, AC1) | CONFIRMED verbatim — §15.3's "ratified precedent demanded" phrasing is accurate |
| 10.4a precedent: byte-identical Merkle migration = engine-independent canonical leaves + payload oracle + row-count oracle, independently re-derived per backend | `_bmad-output/implementation-artifacts/10-4a-*.md` (landmines 4–5, AC2) + `check-migration-merkle` in gate-registry | CONFIRMED |
| §14 dispositions: OQ-3 (11.3 re-pin playbook @ N=30, two-surface detection), OQ-5 (11.2a 16/16 live + 11.2b SLO gate), OQ-6 (11.4a PDP wired into daemon lifetime), OQ-9 (migration corpus + canonical_kv_leaf) | sprint-status entries 11-3/11-2a/11-2b/11-4a (all done) + artifacts | CONFIRMED; all 10 OQ items carry Disposition lines |
| §15.1 substrate rows 1–4 (11.1a/b, 11.2a/b, 11.3, 11.4a) shipped; §15.2 "11.3 envelope proves N=30 under churn" | sprint-status (all done) | CONFIRMED |
| App-D ledger D.1–D.5 dispositions consistent with §15.2/§13.1/§15.8/§15.4; D.4 stays deferred with named trigger | appendix-d diff vs §15 | CONFIRMED, internally consistent |
| §10.7 edits: readiness analysis unchanged, additive status annotations only; Reza enabler split (b)/(c)/3-region shipped vs (a)/(d)/(e) → v2.2 | 10-journey-traceability diff + ADR index + sprint-status | CONFIRMED |

## 3. Verdict rationale

The deliverable's *precedent* and *registry* claims — the ones easiest to assert from memory and hardest to fake-check — were demonstrably checked against the repo (several are verbatim quotes of the 11.2b/10.4a artifacts and the live ADR index, including self-aware corrections like the ADR-039/040 number collisions). The failures are concentrated where the document summarizes *status* and *numbers*: the "shipped" framing swallows three unshipped stories and a nonexistent ADR (F1), and §15.5 — ironically the section titled "Reality check" — builds its constitutional argument on a metric mismatch (F2) and proposes ceiling numbers whose named instruments don't measure them (F3) atop a stale decomposition estimate (F4). None of the fixes invalidates the §15 designs themselves; F1–F4 are all correctable in text before party-mode ratification, and F2/F3 *must* be, since ADR-055's ratification argument currently rests on them.
