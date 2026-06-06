# Sprint Change Proposal — 2026-06-06

**Type:** Direct Adjustment + **Charter Amendment** (Epic 8 scope extension; no MVP/journey-set change)
**Author:** Lunarpulse (dev) · **Decision body:** party-mode roundtable — John (PM) + Winston (architect) + Murat (TEA) + Amelia (dev) + Mary (analyst)
**Scope classification:** Moderate → Significant (charter amendment + 7 new stories registered; defects logged; no replan of other epics)

---

## Section 1 — Issue Summary

**Problem.** A two-round adversarial implementation audit (party-mode, 2026-06-06) asked whether Epic 8 delivers the PRD user journeys (J0 / J-Butler / J-Researcher / J1 / J4). Finding: **zero journeys are presentable end-to-end to a real user.** Epic 8 (stories 8.1–8.7, all `done`) shipped a substrate proven against deterministic fixtures, with three layers simulated everywhere:
1. **No cognition** — zero reference Spirit calls the Inference Port; all reasoning is deterministic Rust.
2. **No real I/O** — MCP is a fixture-replay provider (`butler/src/lib.rs:76-77`); Worker is a canned-output binary; the live `maos-a2a-tcp` transport carries no Mira/Nash traffic (loopback only).
3. **No runnable product** — `maos-bin` is 54 env-gated smoke arms; there is no `maos run` daemon, no `maos init`, no shell; `hello-spirit` is 0 LOC.

The hard integration work was disclosed-as-deferred at every story but **owned by no story** — the "homeless integration layer." The audit also surfaced **undisclosed security/invariant defects** that lived only in conversation: A2A peer-identity bypass (G8), consent-expiry dead code (G10), I12 records nothing, the Observer watchdog fails-open on NaN, and a `done`-but-not Butler halt regression (production `on_idle` never fires its halt).

**Discovery.** Party-mode roundtable convened to critique Epic 8 against the PRD journeys; agents audited the code with read-only tools and converged independently on the above.

---

## Section 2 — Impact Analysis

- **Epic impact:** Epic 8 only. Seven stories registered (8.9, 8.10, 8.11, 8.12, 8.13, 8.14a/b/c, 8.15 — 8.14 split into three). No change to other epics' scope.
- **Charter impact:** The Epic 8 "**Zero kernel KLOC**" mandate is **amended** to "reference Spirits AND their live runtime." The mandate is retained for stories 8.1–8.7; it is **retired for 8.11/8.12** (which require `maos-kernel-core` deltas — the daemon serving loop, the CliWrapper subprocess bridge, budget enforcement). Each kernel-touching story records a NEW pinned `maos-kernel-core` byte baseline (replacing 8.4's 15505 byte-identical assertion) and carries a FLAG-Winston note.
- **Dependency edges added:** 8.9 → 8.6/8.7 · 8.8 re-parented → 8.7 **+ 8.9** (fail-closed is moot while identity is forgeable) · 8.10 independent · 8.11 → 8.10 · 8.12 → 8.11 · 8.13 → 8.9 + 8.11 · 8.14a → 8.11 + **Epic 9.1** (cross-epic back-edge for `maos audit query`) · 8.14b/c → 8.11 + 8.14a · 8.15 → 8.11 + 8.14a.
- **Workspace impact:** new crates `maos-cli`, `maos-mcp`, `maos-notify-push`, and dev-only `maos-journey-test` (pin member deltas at dev time). `maos-kernel-core` byte-identical invariant retired for the two runtime stories only.
- **Defect-tracking impact:** the audit findings (G1–G10, I11 citer-auth, I12 content, NaN watchdog, Butler AC2) added to `deferred-work.md` under a party-mode-audit section, each mapped to its closing story.

---

## Section 3 — Recommended Approach

**Direct Adjustment + Charter Amendment** — extend Epic 8 in-place with a three-phase completion-delivery band, register the defects, and amend the charter. No rollback; the PRD journey set is unchanged (the journeys were always the target — Epic 8 simply had not delivered them yet).

**Three-phase sequencing:**
- **Phase 1 — Trust restoration (charter-safe):** 8.10·AC1 (Butler halt P0 hotfix) → 8.9 (A2A security) → 8.10 (invariant closure) → 8.8 (fail-closed, now gated on 8.9). Closes the undisclosed security holes + the false `done` before spending runtime budget.
- **Phase 2 — Live runtime spine (kernel-amended):** 8.11 (daemon + Inference Port, KEYSTONE) → 8.12 (CLI bridge → J1) ∥ 8.13 (live pair + push → J4).
- **Phase 3 — Journey surface:** 8.14a (J0 + CLI) → 8.14b (Butler MCP) ∥ 8.14c (Researcher MCP). Test track 8.15 (journey-acceptance harness) authored RED early, each journey flips its slice green.

**Per-journey "presentable" gate:** J0 = 8.14a + 9.1 · J-Butler = 8.10·AC1 + 8.11 + 8.14a + 8.14b · J-Researcher = 8.11 + 8.14a + 8.14c · J1 = 8.11 + 8.12 · J4 = 8.9 + 8.11 + 8.13.

**Effort:** this proposal = ~1 session (documentation). Downstream build = the largest single increment in Epic 8 (a live runtime). **Risk:** the kernel deltas (8.11/8.12) are the highest-risk; mitigated by the charter-safe Phase 1 landing first and by the 8.15 hermetic acceptance harness gating every journey.

---

## Section 4 — Detailed Change Proposals

### 4.1 Epic-8 markdown (`epic-8-…miranash-v03-v15.md`)
- **ADDED** a `⚙️ CHARTER AMENDMENT` blockquote after the Goal — records the audit finding, retires zero-KLOC for 8.11/8.12, keeps 8.1–8.7 zero-KLOC.
- **ADDED** an `# Epic 8 Completion Delivery — Stories 8.9–8.14` section with a 3-phase sequencing DAG + per-journey gate + the six story stubs (goal + AC sketches).
- **SPLIT** Story 8.14 → 8.14a (J0 surface + CLI) / 8.14b (Butler MCP) / 8.14c (Researcher MCP) to shorten the Butler-experienceable path.
- **ADDED** Story 8.15 (journey-acceptance test harness + red-phase suites); relocated the harness sub-AC out of 8.11·AC5 (which now exposes only the run-surface seam).

### 4.2 `epics/index.md`
- **ADDED** TOC links for Stories 8.9, 8.10, 8.11, 8.12, 8.13, 8.14a/b/c, 8.15 after the Story 8.8 link, under a Completion-Delivery sub-heading.

### 4.3 `epics/dependency-dag.md`
- **ADDED** under `E8 Reference Spirits` the 8.9–8.15 nodes with dependency arrows; re-parented 8.8 → 8.7 + 8.9.
- **ADDED** a cross-epic BACK-EDGE `9.1 → 8.14a` (J0 audit-query surface).
- **ADDED** sprint-plan **invariant 6** (the 3-phase completion-delivery ordering + per-journey gate + the "watch-it-work" shortest path + the test track).

### 4.4 `sprint-status.yaml`
- **ADDED** `8-9` … `8-15` rows (all `backlog`) with explanatory comments, after `8-8` and before `epic-8-retrospective`. `last_updated` marker bumped.

### 4.5 `deferred-work.md`
- **ADDED** a "Deferred from: party-mode implementation audit of Epic 8 (2026-06-06)" section enumerating G1–G10, the I11/I12 holes, the NaN watchdog, the pub-field bypass class, the Butler AC2 regression, and the six journey-presentability gaps — each mapped to its closing story.

### 4.6 Test artifact
- **ADDED** `_bmad-output/test-artifacts/atdd-checklist-8-14b-j-butler-acceptance.md` — ATDD red-phase scaffolds + implementation checklist for the J-Butler journey (the Story 8.15 exemplar; harness + 8 prioritized JB-tests, JB-3 pinning the Butler AC2 regression).

---

## Section 5 — Implementation Handoff

**Scope:** Minor (documentation) for this proposal; the downstream build is the largest increment in Epic 8 (live runtime + real I/O).

- **First move (P0):** Butler AC2 (8.10·AC1) as an immediate correct-course hotfix — it is a regression of a `done` story; JB-3 in the ATDD checklist pins it.
- **Highest-leverage security:** Story 8.9 (G8 turns the intake `verify_pinned` from `X==X` theater into a real check).
- **Keystone:** Story 8.11 (daemon + Inference Port) — nothing in Phase 2/3 is presentable without it.
- **Recommended dev model:** `claude-opus-4-8` for 8.9/8.10/8.11 (security + kernel); `claude-opus-4-8` for 8.12/8.13; per-journey 8.14a/b/c + 8.15 at dev-time discretion.
- Stories remain `backlog`; nothing is implemented by this proposal.

**Success criteria for this proposal:** epic markdown, `index.md`, `dependency-dag.md`, `sprint-status.yaml`, and `deferred-work.md` all reference Stories 8.9–8.15 consistently with correct dependency edges; the charter amendment is recorded; the audit defects are tracked in `deferred-work.md`. ✅ All applied.
