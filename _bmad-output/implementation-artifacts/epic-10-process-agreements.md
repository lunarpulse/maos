# Epic 10 Process Agreements (§A5 / §A6 / §A7)

Ratified at the Epic 10 retrospective (2026-06-26). These are standing rules for all subsequent stories; they supersede the conflicting Epic 9 §A2 model rule. Apply at **sprint-planning preflight** and **review** gates.

---

## §A5 — Model tier is risk-gated per story (SUPERSEDES Epic 9 §A2)

Epic 9 §A2 mandated a flat "Tier-1 = opus-4-8 mandatory" classification. It was **defined but not followed** — Epic 10 Tier-1 stories (10.1b, 10.2, 10.4a/b/c) all ran on opus-4-6. Outcomes held only because §A6 review caught the gaps. New rule:

**Decide the model tier per story at preflight, by risk class:**

| Story touches… | Implement on | Review |
|----------------|-------------|--------|
| kernel-core, security/sandbox, consent/A2A, crypto, unsafe FFI | **opus-4-8** | §A6 full multi-layer mandatory |
| migration/data-integrity gates, ABI/stability surface | opus-4-8 preferred | §A6 full multi-layer mandatory |
| integration wiring, docs, i18n, tooling, CLI/xtask gates | opus-4-6 acceptable | §A6 multi-layer mandatory (see below) |

Record the chosen tier and the risk rationale in the story's preflight notes. "Tier-1/Tier-2" labels are retired in favor of the risk-class table.

## §A6 — Multi-layer adversarial review is non-negotiable below opus-4-8 (and on review failure)

Whenever a story is implemented below opus-4-8, **OR** the review subagents fail / rate-limit / degrade for any reason, completion is **blocked** until a clean full multi-layer pass exists:

- **Layers:** Blind Hunter · Edge Case Hunter · Acceptance Auditor · **Test-Infra Auditor**. The Test-Infra layer is mandatory — it was the layer dropped on 10.4c and degraded on 10.5, and it is what caught "wired-but-never-run" gates and missing proven-red vectors.
- **A degraded review is not a review.** Main-session self-review does not satisfy §A6. If subagents fail, re-run them; do not substitute. (Evidence: 10.5's self-review shipped a non-compiling AC3 and a fabricated AC5 — see `story-10-5-rereview-2026-06-26.md`.)
- For kernel/security/unsafe code, the review must include a **runtime-execution check**: confirm the code is actually exercised in CI, not merely compiled.

## §A7 — Gate-building vocabulary (the Epic 10 reflexes)

Every gate/test authored from here on is held to these six reflexes. A reviewer rejects on any violation:

1. **Derive-and-reconcile.** A count gate derives its numerator and denominator from per-record data and reconciles against any self-reported summary. A gate that reads `passed=N` from a TOML is a press release. (10.2 trial gate, 10.4a migration, 10.4b consent.)
2. **Real-subsystem proven-red.** Every gate ships a vector that feeds known-bad input and asserts the gate **fails**. Drive the real subsystem (live socket/DB/adapter/CliWrapper) — never a mock or loopback that physically cannot exhibit the bug. (10.4b confused-deputy on live TCP.)
3. **Feature-flag ≠ measurement.** Flipping a flag does not produce a number; a harness does. A gate that can only go red by editing a constant is theater. (10.2→10.4c canned J4.)
4. **Tripwire + FLAG-Winston for kernel deltas.** Any kernel-core change is re-pinned in `kernel-core-baseline.toml` HISTORY with the authorized surface named. Churn outside the named surface (even `cargo fmt`) is a silent budget draw and must be reverted or disclosed. (10.5 R3.)
5. **"WOULD HAVE BLOCKED at v1.5" banner = auditable hold.** Advisory phase emits the loud banner; disposition lives once in `gate-registry.toml` as `{v1_0, v1_5}`. No silent passes.
6. **Timing gates need both axes.** Absolute threshold (the contract) **and** regression-vs-baseline (the guard rail). One without the other misses silent decay. (10.4c P95 + criterion.)

**Language/identity reflex (added by 10.5 R2):** a presence/coverage gate over localized or translated artifacts must also verify **content identity** (right language, not just a present file). Coverage + locked-Latin-term gates cannot detect wrong-language content.
