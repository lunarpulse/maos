---
baseline_commit: "**`2fd10398`** (\"15-5-decision-adrs-and-provisioning-checklist\"), working tree **clean** (`git status --short` empty; the only 2026-09-09 mtimes are 15-5's own landed files). Every figure below was measured at this commit, not inherited. Gate sweep, all exit 0: `check-empty-kernel` PASSED (0 violations) · `check-service-boundary` PASSED (0 violations) · `kloc-check` PASSED (aggregate=157231) · `check-env-contract` PASS (88 registered, 0 violations) · `check-j1-loopback-delegation` PASS · `check-exit-commands` PASS (7 epics, 31 tokens, 6 owed) · **`check-kernel-baseline` PASSED (`maos-kernel-core/src` = 24474, pinned 24474)**. `cargo test -p xtask --test decision_adrs_and_provisioning` = **3 passed** (green; §3 is why that matters). Ceilings/measured: `maos-bin` **17349/20109 (+2760)** (`xtask/kloc.toml:405`) · `maos-shell` 305/500 (`:572`) · `maos-journey-test` 493/1104 (`:562`) · `maos-spirit-hello` 365/1000 (`:328`) · **`xtask` 43409/43444 (+35)** (`:252`) · **`maos-kernel-core` 18935/18935 (ZERO)** (`:199`). ⚠ **T0 re-measures — this block is a snapshot, not a licence** (`feedback_grant_is_a_global`). ⚠ **Beware a poisoned `target/`**: a scratch copy of the repo built with a shared `CARGO_TARGET_DIR` leaves `decision_adrs_and_provisioning` binaries whose `CARGO_MANIFEST_DIR` points outside the repo; the test then reports every file missing. `touch xtask/tests/decision_adrs_and_provisioning.rs` before trusting a red."
depends_on: "**15-5 — SATISFIED at HEAD.** `docs/adr/ADR-064-one-inference-mode-selector.md` is committed and `ACCEPTED` (frontmatter `Status:`/`Gate:`/`Decided:`/`Accepted-in-PR:`/`Revisits:`/`Supersedes:`), its `## Consumers` names `15-6-inference-replay-seam`, and `docs/adr/index.md:53` carries its row. Per **F11** this story **cites** ADR-064 and does not restate its precedence rules — except where this preflight measured a clause false, which is §4 and is handled by amending the ADR in this commit, not by drifting from it. **15-2 — SATISFIED**: the `maos-bin` policy ceiling 17109 → 20109 landed (`kloc.toml:405`) and the ledger row books `15-6 +150–300` (`:404`). ⚠ **That booked figure is too small — see §11.** 15-1 and 15-3 are not preconditions for this story's code, but their landings moved every `main.rs` line number the epic cites (§8)."
blocks: "**Five epics, and the epic file under-states it by three.** `epics/dependency-dag.md:214` says `15-6 → 18-*, 19-*, 20-4, 21-1`; `epic-15…:202` says *\"15-6 before Epic 18\"*. Measured against the tree, the selector is a **hard token in Epic 16's and Epic 20's own exit blocks**: `epic-16…:16` (exit line 1, `maos shell` under `MAOS_INFERENCE_MODE=replay` + the j0 cassette — consumed by 16-2 via `journey_j0`) and `:20` (exit line 5, Butler); `epic-20…:22` (exit line 9, consumed by 20-1). Full consumer set verified in the tree: `epic-16…:16,:20,:26,:37,:78,:79,:108` · `epic-18…:14,:16,:17,:22,:29` · `epic-19…:14,:23,:57,:59,:84,:89` · `epic-20…:22,:34,:43,:113` · `epic-21…:52,:93`. Both the dag row and the epic's Dependencies line are corrected in this commit (§12)."
spec_alignment: "⚠ **THE EPIC IS AMENDED IN THE SAME COMMIT, AND SO IS ADR-064.** Operator directive in force (`feedback_pay_effort_early_deferring_drifts`, rule 9): *no drift in spec; the story and the epic say the same thing, or the epic is amended in the same commit; and never let a story be merely BETTER than its epic.* Four adversarial scouts plus a live-binary probe sweep disproved premises in this story's own section and in the ADR it is ordered behind. The four that change what gets built: **(1) AC1's command already exits 0 at HEAD, for the wrong reason — every guarantee it names is a green that proves nothing** (§1, ten measured probes); **(2) the recorder never writes `response.text`, so record→replay yields empty completions, silently** (§2) — a ship-blocker sitting directly under this story's title; **(3) this story reds 15-5's own ADR gate twice, and 15-5 is `done`** (§3, proven red); **(4) ADR-064's `## Context` is false about `UnconfiguredProvider`, so its F15 live-predicate cannot fire in a default boot** (§4, measured). OLD→NEW list in §12. ⚠ **ROUND-TABLE 2026-09-09 (§13) then found FIVE defects in the story ITSELF, five for five, every one of them the story's own §1 diagnosis** — the citation repair repeated the defect it repaired (R6 committed while enforcing R6, and invalidated by the story's own T2); the seam ships a documented UNMEDIATED INJECTION PATH once `MAOS_REPLAY_CASSETTE` goes `UserFacing`; the `provenance` requirement was a schema break hiding inside an unchanged `v1`; the re-keyed anti-rot anchor was a receipt (third `maos summon-dragon` sighting in this room); and D-15-6-A needed a fact the router does not model. **Four rulings changed: C REVERSED (router-level seam, spike-gated), A NARROWED (ADR-064's Decision stands), D RE-RULED (lenient reader, strict CI), and G + H added.** Sizing ruled WHOLE, Dana's dissent recorded."
split_from: "Not a split. Authored from `epics/epic-15-foundations-w0.md:191-199` (the `### 15-6-…` section) under Round 3, closing preflight **§2.4** and fork **D-D**. ⚠ **SIZING RE-DERIVED, NOT INHERITED.** The R2 preflight priced this as a maos-bin wiring story at +150–300 lines on the premise of *\"two inference consumers\"* and *\"a generic record/replay seam at `InferencePortAdapter`\"* (`preflight-r2/epic-15.md:39,:79`). Measurement says the wiring is the small half. The story also owns: a **record/replay round-trip that has never worked** (§2), an **anti-rot re-key of a gate landed 24 hours earlier** (§3), a **ratified predicate that cannot fire** (§4), an **AC that is unfalsifiable by construction** (§5), a **nightly leg that is four repairs from its own promise** (§6), and a **branch structure that cannot express the lattice by editing branch bodies** (§7). Measured cost `+268…+408` charged in `maos-bin`, `×1.3` (rule 6) = **`+349…+531`** (re-derived in round 2 after the D-15-6-C refinement) — against 2760 headroom, so it fits ~6×, but it **exceeds the booked ledger row and the row is re-booked in this commit** (§11). **Re-measured duration: 3–5 d** (was implicitly 1.5–2.5 d as a wiring story). **NOT SPLIT**: §2 and §3 are load-bearing for §1's own control — a story that ships the selector without the recorder repair ships a `record` mode that produces cassettes which replay as nothing, and a story that ships either without the re-key reds the epic's exit line 1."
kernel_grant: "**NONE and none needed. ZERO kernel-Δ @24474, and the pin is measured green at HEAD** (`check-kernel-baseline: PASSED (maos-kernel-core/src = 24474, pinned 24474)`; pin at `xtask/kernel-core-baseline.toml:481`, counter `xtask/src/check_kernel_baseline.rs:30-31,62-63` = **raw `.rs` line count**, comments and blanks included, recursively under `crates/maos-kernel-core/src` **only**). `crates/maos-kernel-core/src/inference/mod.rs` is **inside** the pin and is also at **zero kloc headroom** — one added line reds two gates plus `crates/maos-a2a-tcp/tests/t11_t12_chaos_absence.rs:193-212`. Epic AC1 already forbids touching it and this story does not. `crates/maos-domain/src/ports/inference.rs` is **outside** the pin (charged to `maos-domain`, +497) and is **also untouched** — the `InferencePort` trait has exactly one method (`inference.rs:20-25`) and every wrapper this story needs already exists or is built in `maos-bin`. ⚠ **`check-kernel-baseline` is NOT in the epic's exit line 1** — the kernel-Δ-0 claim has no gate in the block the story was handed; T0 runs it explicitly."
kloc_grant: "**NO NEW GRANT ASKED. The existing `maos-bin` policy ceiling covers it — but the LEDGER ROW is re-booked, in this commit, from `+150–300` to `+350–550`.** `maos-bin` measured **17349/20109 = +2760** (`kloc.toml:405`). Measured plan: `inference_mode.rs` +150–200 · `cassette_replay.rs` two `Provider` re-skins +60–100 · `main.rs` resolve + map replacement +25–40 · `main.rs` researcher re-key +20–35 · `cassette_replay.rs` provenance + `text` +20–35 · `lib.rs` +3–8 · `env_contract.rs` ≈0 · `worker_spawn.rs` −5 → **+268…+408**, ×1.3 = **+349…+531** (⚠ RE-DERIVED in round 2 after the D-15-6-C refinement). ⚠ **Put every proven-red vector in `crates/maos-bin/tests/` (kloc-excluded, `kloc_check.rs:303`) behind `pub mod inference_mode;` in `crates/maos-bin/src/lib.rs`** — the repo's own doctrine (`crates/maos-bin/src/lib.rs:32`, `:52`: *\"an in-`src` test module is budget-charged and CI-invisible\"*). An inline `#[cfg(test)] mod tests` in `src/inference_mode.rs` adds +120–180 charged and doubles the spend for nothing. ⚠ **`xtask` has 35 lines of headroom** (43409/43444, `:252`) — this story needs **zero** `xtask/src` change; the §3 re-key lives in `xtask/tests/` and is **free**. ⚠ **A GRANT IS A GLOBAL** — 15-1+15-3+15-5 already moved `maos-bin` 17109 → 17349 (+240) against a booked `~10`; T0 re-measures and does not trust this line. ⚠ **`kloc.toml:512` is a DEAD hazard** — 15-2 replaced the silent post-header skip with a hard error (`kloc_check.rs:246-258`) and the only table header is now `[in_progress_decomposition]` at `:597`, below every budget row. Do not import it."
model: "frontier-class allowlist {opus-4-6, opus-4-7, opus-4-8, gpt-5.5, gpt-5.6, glm-5.1, glm-5.2, glm-5.3, opus-5, equiv}. `FRONTIER_FAMILIES` (`xtask/src/check_dev_model_tier.rs:45-48`) is the machine-checked set; `equiv` is prose, NOT a match token. Keep the literal `allowlist {` (`check_dev_model_used_populated.rs:302`)."
review: "§A6 full-layer net (Blind + Edge Case + Acceptance + Test-Infra + non-author runtime). ⚠ **Acceptance is load-bearing and §1 is why.** This story's headline command **exits 0 at HEAD**; a reviewer who runs AC1's happy path and sees green has reviewed nothing. **The reviewer must: (a)** run the ten-probe table in §1 against the *pre-change* binary and confirm probes 3–7, 9 and 10 behave as recorded, then against the built binary and confirm each flipped — a green AC1 without the negative legs is the vacuity this epic exists to remove; **(b)** run `cargo test -p xtask --test decision_adrs_and_provisioning` and confirm the ADR-064 anchors were re-keyed **forward** (assert presence of `MAOS_INFERENCE_MODE`, assert absence of `--replay-llm`) and not merely deleted — a dropped anchor is the failure the gate's own doc forbids (`decision_adrs_and_provisioning.rs:229-233`); **(c)** verify the record→replay round-trip test actually asserts the **text** survives (§2) — a round-trip that asserts only `stop_reason` passes on the broken recorder; **(d)** confirm the `provenance` reader can fail: plant a cassette with a missing/garbage `provenance` and see it red (§5) — a field nothing reads cannot be a control; **(e)** confirm `check-env-contract`, `check-kernel-baseline` and `kloc-check` are green and that no `xtask/src` line was added; **(f)** confirm `journey_researcher.rs:46`/`:122`'s literal `\"deterministic survey\"` assertion still passes — it is the only mechanical proof of the unset-default branch in the repo; **(g)** read the T0 spike result and confirm the seam that shipped is the one the measurement licensed — if the port-wrap fallback was taken, the FR47 asymmetry must be FILED, not absorbed (§13 finding 2); **(h)** confirm the re-keyed anchors point at the conflict refusal and the seam location, NOT at the `env_contract.rs` registry row, and run D-15-6-G's falsifier — delete the refusal, keep the row: the naive anchor stays green, these must red (§13 finding 4); **(i)** confirm the `provenance` corpus test asserts its DENOMINATOR before its predicate — an empty glob must red (§13 finding 3); **(j)** if the spike refused, confirm **D-15-6-I** was executed in full — AC1 carries the non-mediation sentence AND a `RELEASE-HOLDS.md` claim-boundary row exists **citing the runnable §1 probe** (D-15-6-L; nothing machine-reads that file, so this clause is the control); a port-wrap fallback with neither is the fallback taken dishonestly; **(k)** run a one-shot `record` to a temp cassette, confirm the file exists **after a normal exit**, then delete the `flush()` call and confirm it reds — `Drop` alone cannot be trusted under the router seam because nothing drops the router (D-15-6-J); **(l)** run `record` against a Spirit that makes no completion and confirm it exits **non-zero** (D-15-6-K) — silence-and-exit-0 is the defect, not the baseline."
---

# 15-6 — Inference record/replay seam for every inference consumer

Status: **done.**

> **The capability:** *Every inference consumer in this tree picks live, record or replay from one
> environment variable; when the mode cannot be honoured the process says so and exits non-zero
> instead of quietly doing something else; and a cassette that was recorded actually replays what
> the model said.*

**Closes:** preflight **§2.4** (no hermetic seam outside Researcher) · fork **D-D** (one replay
selector) as implemented, ADR-064 as amended · the four spellings (`--live`, `--replay-llm`,
`MAOS_JOURNEY_MODE`, cassette-presence) collapsed to one · the `journey-nightly.yml` `--live`
defect · the record/replay round-trip that has never produced a replayable cassette.

---

## What this story actually is

The epic's four ACs describe a rename plus one missing branch. Ten probes against the HEAD binary
say the seam is not missing — it is **present, unreachable, and silently wrong in three places**:

1. **The headline command already passes.** `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=… maos
   run spirits/researcher/manifest.toml --once` exits **0** at HEAD and prints
   `researcher cassette-replay inference wired` — not because the mode was read (nothing reads it)
   but because cassette *presence* selects replay. Every guarantee AC1 names is satisfied at HEAD by
   an unrelated mechanism. §1.
2. **`record` does not record the answer.** `CassetteRecordPort::complete` builds an entry with
   `stop_reason`, `usage` and `provider_attribution` and **no `text` key**
   (`cassette_replay.rs:212-232`); the reader takes `response.text` with `.unwrap_or("")`
   (`:53-57`). Record a cassette, replay it, and every completion is the empty string, with no
   error anywhere. §2.
3. **This story reds a gate that landed yesterday.** 15-5's `decision_adrs_and_provisioning`
   asserts, as an anti-rot anchor for ADR-064, that `MAOS_INFERENCE_MODE` is **absent** from
   `env_contract.rs` and that `--replay-llm` is **still accepted**. Those are this story's AC4 and
   AC1. Proven red. §3.

So this is not "wire a selector." It is **the story that makes replay mean something** — and the
three epics ordered behind it (16, 18, 19, 20, 21) each put the selector in an exit block on the
assumption that a replayed run reproduces a recorded one. At HEAD that assumption is false in both
directions: nothing selects, and nothing round-trips.

---

## 1. 🔴 AC1's COMMAND EXITS 0 AT HEAD. EVERY GUARANTEE IT NAMES IS A GREEN THAT PROVES NOTHING.

Measured against `./target/debug/maos` at `2fd10398`, on a shell with **no** `MAOS_*`, `ANTHROPIC*`,
`OPENAI*` or `OLLAMA*` variable set. `<r>` = `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json`,
`<j0>` = `crates/maos-journey-test/cassettes/j0/shell-intro.json`.

| # | command | HEAD exit | HEAD observable | must become |
|---|---|---|---|---|
| 1 | `MAOS_REPLAY_CASSETTE=<r> maos run …/researcher/manifest.toml --once` | 0 | `researcher cassette-replay inference wired` | **unchanged** (ADR-064 clause 1) |
| 2 | `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=<r> …` | **0** | same line — **the mode is never read** | 0, and selected *by the mode* |
| 3 | `MAOS_INFERENCE_MODE=replay …` (no cassette) | **0** | `researcher deterministic survey (no --live; zero network)` | **non-zero, typed** |
| 4 | `MAOS_INFERENCE_MODE=totally-bogus …` | **0** | `researcher deterministic survey` | **non-zero, typed** |
| 5 | `MAOS_INFERENCE_MODE=live …` (no real provider) | **0** | `researcher deterministic survey` | **non-zero, typed** |
| 6 | `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=<r> … --live` | **0** | `researcher live-inference seam wired (--live)` | **non-zero, typed** |
| 7 | `MAOS_INFERENCE_MODE=record …` (no cassette, no `--live`) | **0** | `researcher deterministic survey` | **non-zero, typed** |
| 8 | `MAOS_INFERENCE_MODE=record MAOS_REPLAY_CASSETTE=<tmp> …` | **0** | cassette replay selected from cassette *presence* (valid cassette — T0's row 8; an unreadable path exits 1 on init failure) — **`record` itself is unreachable without `--live`** | **non-zero, typed**: a keyless record run completes nothing and the flush refuses zero successful responses (D-15-6-K); the record→replay **text** round trip is proven hermetically (AC4) — *amended by §A6 review D1, 2026-09-11* |
| 9 | `printf '@hello-spirit hi\n' \| MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=<j0> maos shell` | 0 | `Hello, I am the MAOS reference Spirit. Inference transport error — the configured provider is unreachable.` | prints the cassette's own text |
| 10 | `maos shell </dev/null` **with** vs **without** both variables | 0 / 0 | **`diff` of the two outputs is EMPTY** | outputs differ |

**Probe 10 is the cleanest statement of the defect the epic is trying to name.** The shell's output
is byte-identical with and without a cassette, because the shared adapter at `main.rs:3170` reaches
`maos_shell::run_shell` at `:3283-3288` with no cassette branch anywhere in between.

**Probe 9 gives AC1 a real observable.** The j0 cassette's entry 0 is
`"text": "Hello! I am the MAOS hello-spirit reference implementation. I can help you explore the
MAOS kernel and its Spirit ecosystem."` (`shell-intro.json:13`). At HEAD the shell prints an
inference **transport error** instead. That is a hermetic, falsifiable, red-at-HEAD → green-after
observable, and it is exactly what `epic-16…:16` (exit line 1) and `epic-20…:22` (exit line 9)
already promise.

⚠ **The consequence for AC1's wording.** As written, AC1's contract ("`…replay…` and `…record…`
work for every inference consumer") is discharged by probe 2, which passes today. AC1 is rewritten
to **name the ten probes as its control**, with the six that exit 0 at HEAD and must exit non-zero
(or print differently) as the proof obligation. Rule: *a story whose headline command passes before
the story is written has no control.*

---

## 2. 🔴 THE RECORDER DOES NOT RECORD THE ANSWER. RECORD → REPLAY IS SILENTLY EMPTY.

`crates/maos-bin/src/cassette_replay.rs`, `impl InferencePort for CassetteRecordPort` (`:205-239`),
builds each entry as:

```rust
"response": {
    "stop_reason": …,
    "usage": { "input_tokens": …, "output_tokens": … },
    "provider_attribution": { "provider_id": …, "endpoint_url": …, "model_id": … },
},
```

There is **no `"text"` key**. `InferenceResponse.text` exists (`maos-domain/src/ports/inference.rs:92`)
and is simply dropped. The reader, in the same file:

```rust
let text = resp.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();   // :53-57
```

**So every replayed turn of a recorded cassette returns the empty string, and nothing errors.**
There is no schema check that could catch it — the file has **no serde derives at all**; parsing is
hand-rolled `serde_json::Value` traversal (§5).

**Blast radius, all measured:**

- The epic's own operator row `ops-nightly-cassette-rerecord` (`epic-15…:47`) promises to *"refresh
  the 15-6 cassettes with `provenance: live-record`"*. Every cassette that row produces replays as
  nothing.
- `epic-18…:14,:16,:17` assert that a replayed run's rendered output **equals the entry's
  `response.text`**. Against a re-recorded cassette that assertion compares against `""`.
- The recorder also writes a hardcoded `"model_id": "live-recorded"` (`:187`) — a *different string*
  from the `"live-record"` the epic wants as a provenance value, and `"live-record"` is already in
  the tree as the **`session`** argument (`main.rs:4650` → `CassetteRecordPort::new`'s 4th parameter
  is `session`, `:166`, serialized at `:189`). Anyone grepping for the epic's proposed enum value
  finds a false positive.
- The recorder writes **on `Drop` only** (`:178-203`), so a `SIGKILL` loses the session and a write
  failure is unreportable (`Drop` cannot return `Err`; `:194-201` `eprintln!`s).

**This is the story's own title failing.** A "record/replay seam" whose record half cannot produce a
replayable artifact is not a seam. AC4 is new and non-negotiable: the round trip is proven
hermetically — record through a stub provider into a temp cassette, replay it, assert the **text**
survives. A round-trip test that asserts only `stop_reason` passes on the broken recorder.

---

## 3. 🔴 THIS STORY REDS 15-5's OWN GATE, TWICE — AND 15-5 IS `done`. PROVEN RED.

`xtask/tests/decision_adrs_and_provisioning.rs` landed at HEAD (2026-09-09) and is **green**
(`3 passed`, measured). Its ADR-064 anti-rot arm (`:388-421`):

```rust
"064" => {
    …  require_absent(number, context, "MAOS_INFERENCE_MODE",
           "crates/maos-bin/src/env_contract.rs", &env_contract, "MAOS_INFERENCE_MODE", findings);
    …  require_contains(number, context, "--replay-llm",
           "crates/maos-bin/src/worker_spawn.rs", &worker_spawn,
           "\"--replay-llm\" => live = false", findings);
}
```

`require_absent` (`:199-218`) fires the moment `env_contract.rs` gains the token — **which is
15-6 AC4 verbatim**. `require_contains` fires the moment `--replay-llm` leaves `worker_spawn.rs:59`
— **which is ADR-064's stated retirement, owned by 15-6**. Both also assert that the ADR's
`## Context` still *names* the token (`:208-212`), so the ADR prose moves with the code or the
anchor reds on the other clause.

**Proven, not reasoned.** HEAD was extracted to a scratch copy, only the minimal AC4 registry row
and the `--replay-llm` retirement were applied, and the real test was run:

```
Story 15-5 decision/provisioning contract failed:
ADR-064 anti-rot anchor moved: crates/maos-bin/src/env_contract.rs now contains `MAOS_INFERENCE_MODE`
ADR-064 anti-rot anchor moved: crates/maos-bin/src/worker_spawn.rs no longer contains `"--replay-llm" => live = false`
test result: FAILED. 2 passed; 1 failed
```

**This is a Blocking red on the epic's own exit line 1.** `cargo test --workspace --no-fail-fast`
runs as job `workspace-test-suite` (`.github/workflows/discipline.yml:3423`, command `:3439`) and is
enrolled in **two** `needs:` lists — `v1-0-ship-gate` (`:3491`) and `aggregate` (`:3697`).

**Nothing hands this off.** The epic's §15-6 section does not mention it; 15-5's story file notes
only that *"`check-env-contract` fires on 15-6's commit"* — a different gate. This is the
third recorded instance of the shape **"a control whose owner has already shipped"**
(`project_story_13_6e_split`).

**The sanctioned move is re-keying, not deletion.** The gate's own doc says so (`:229-233`): *"a
moved anchor must be re-pointed in the ADR, never silently dropped."* AC7 re-keys **forward** —
`require_contains(MAOS_INFERENCE_MODE in env_contract.rs)` and
`require_absent("--replay-llm" from worker_spawn.rs)` — so the anchor keeps biting in the new
direction, and the fixture context at `:841` and the fixture files at `:909-917` move with it.
`xtask/tests/` is kloc-excluded, so this costs **zero** budget.

---

## 4. 🔴 ADR-064's `## Context` IS FALSE ABOUT `UnconfiguredProvider`, SO F15's LIVE PREDICATE CANNOT FIRE

ADR-064 states, and F15 ratifies:

> When no real provider is configured, `main.rs:3155-3159` installs `UnconfiguredProvider` under the
> `anthropic` identifier … `live` requires a real configured provider. `UnconfiguredProvider`
> remains installed for the unset compatibility path but is explicitly flagged and does not satisfy
> the live predicate.

**Measured with zero provider environment variables set, the tree does not install it.**

| provider | constructor | keyless result | evidence |
|---|---|---|---|
| Anthropic | `AnthropicProvider::new` | **`Err(Unconfigured)`** — reads `MAOS_ANTHROPIC_API_KEY` | `maos-providers/src/anthropic.rs:36-37` |
| OpenAI | `OpenAiProvider::new` | `Err` (same shape) | `main.rs:3129-3137` |
| **Ollama** | `OllamaProvider::new` | **always `Ok`** — no credential, default URL `http://localhost:11434` | `maos-providers/src/ollama.rs:30-40`; `main.rs:3139-3151` |

So `providers_map` is **never empty** unless the operator sets `MAOS_OLLAMA_URL` to `""` or
`"skip"`. `main.rs:3155-3159`'s `if providers_map.is_empty()` does not fire, `default_id` is
`Some("ollama")`, and the probe output confirms it verbatim — `maos: Ollama provider registered`,
with **no** `maos: no providers configured` line.

**Consequence:** F15's predicate as ratified guards a state the default boot does not reach, and
**admits the state that actually breaks**. Probe 9 measured it: a registered Ollama at
`localhost:11434` with nothing listening produced
`Inference transport error — the configured provider is unreachable` and **exit 0**. A control that
cannot fire in the environment it is written for is a null control — this repository's signature
failure mode (`project_epic_12_retro_outcomes`, 11 instances).

**Ruled (D-15-6-A, below): implement the ratified predicate AND extend it minimally so it can
fire** — `live` requires a provider that is not `UnconfiguredProvider` **and** whose registration
came from an explicit operator signal (a credential for `anthropic`/`openai`; an explicitly-set
`MAOS_OLLAMA_URL` for `ollama`). A default-guessed localhost endpoint is not "a real configured
provider" in the sense F15 intends. **Blast radius is zero**: `MAOS_INFERENCE_MODE=live` is a new
path with no callers at HEAD, and `--live` (unset-mode, clause 1) is untouched. ADR-064's `## Context`
and `## Decision` are amended in this commit (§12) — the ADR is normative and stays normative; what
changes is a sentence this preflight measured false.

---

## 5. 🔴 THE EPIC'S ORIGINAL AC2 (`provenance`) IS UNFALSIFIABLE BY CONSTRUCTION, AND ALL THREE CASSETTES ARE DEAD

**There is nothing to break, which is the problem.** `cassette_replay.rs` has **no serde derives,
no `deny_unknown_fields`, no `#[serde(default)]`, no deserialize struct at all** — `from_file`
(`:21`) reads exactly two top-level keys, `schema_version` (`:27-35`) and `entries` (`:38`), by
`Value::get`. `recorded_at`, `model_id`, `spirit_id` and `session` are **written and never read**;
`sequence` is written and never read (replay is pure cursor order, `:118`). Adding a `provenance`
key to the three JSON files is a **guaranteed no-op for every existing reader**, and repo-wide grep
finds **no** byte-compare, golden, hash or schema gate over cassettes.

**And the artifacts AC2 re-stamps are consumed by nobody:**

| cassette | loaded by | actually replayed? |
|---|---|---|
| `cassettes/j-butler/on-idle-halt.json` | **nothing** (`journey_butler.rs` never calls `.cassette(…)`) | no |
| `cassettes/j0/shell-intro.json` | `journey_j0.rs:57-60` → harness copies it to a tempfile and exports `MAOS_REPLAY_CASSETTE` (`maos-journey-test/src/lib.rs:118-123`) | **no** — the shell path reads neither variable |
| `cassettes/j-researcher/survey-distill.json` | `journey_researcher.rs:69-78` | **no** — its own spawn passes `--live` (`:81`), and `--live` wins at `main.rs:4470`, before the cassette arm at `:4657` |

All three carry `"model_id": "hand-authored-seed"` on **line 4** (AC2's citation is exact), an
all-zero wildcard `prompt_sha256` on line 10, and `recorded_at: 2026-06-11T00:00:00Z` on line 3.

⚠ **Also**: each cassette carries a *second* `model_id` inside `response.provider_attribution`
(`:16`) — and that is the one the reader actually consumes (`cassette_replay.rs:90-93`). AC2 names
only the top-level one.

**Ruled (D-15-6-D): `provenance` gets a reader that can fail** (story AC5). `CassetteReplayPort::from_file`
requires `provenance ∈ {"seed","live-record"}` and returns `Err` otherwise, in the same style as the
existing `schema_version` refusal (`:27-35`). That makes a missing or garbage value a hard red with
a plantable vector, costs ~10 charged lines, and needs no new surface. `CassetteRecordPort`'s `Drop`
serializer writes `"provenance": "live-record"`; the three seeds are re-stamped `"seed"`. Downstream
epics already assume this (`epic-19…:84` commits its J1 cassette *"with `provenance: seed`"`).

⚠ **Consequence AC2 does not state:** requiring the field makes every future cassette carry it.
That is intended and is why the requirement is stated in the ADR amendment, not only in code.

---

## 6. 🔴 THE EPIC'S ORIGINAL AC3 ("LATENT RED") IS FOUR REPAIRS FROM TRUTH, AND ONE OF THEM ARMS A GATE THAT THEN HARD-FAILS

`.github/workflows/journey-nightly.yml` is **127 lines** and **byte-identical on `main`** — the
defect is not a `recovery-lane` artifact. The `tier-2-rerecord` job (`:65`) runs, verbatim
(`:90-94`):

```
cargo nextest run \
  -p maos-journey-test \
  --test-threads 1 \
  --no-fail-fast \
  --live
```

The epic's original AC3 is right that nextest rejects `--live` there (no `--` separator; `run`'s grammar takes bare
positional filters, and no `--live` option exists in the pinned `CARGO_NEXTEST_VERSION: "0.9.89"`,
`journey-nightly.yml:10`). ⚠ **That rejection is derived from the grammar, not executed** —
`cargo-nextest` is not installed on this machine and the repo has no `.config/nextest.toml` or
`.cargo/config.toml` that could alias the flag. It does not matter which way it resolves: `--live` is
a `maos run` flag (`worker_spawn.rs:54`) that the journey tests already pass to the process they
spawn (`journey_researcher.rs:81`), so it is wrong in that position either way. But **"latent red"
understates it in four ways**:

1. **The leg has never executed.** `needs: [tier-2-journey, secret-gate]` (`:67`) plus
   `if: needs.secret-gate.outputs.has_anthropic_key == 'true'` (`:69`). Without the secret the job is
   **skipped**, and a skipped job is a red nowhere.
2. **The flag is aimed at the wrong process.** `--live` is a `maos run` flag
   (`worker_spawn.rs:54`), and the journey tests already hardcode it into the command they spawn
   (`journey_researcher.rs:81`). There was never a `--live` for nextest to forward.
3. **Even repaired, it cannot refresh anything.** `ReplayProvider::cassette` copies the cassette to a
   **tempfile** and exports *that* path (`maos-journey-test/src/lib.rs:346-362`, `:118-123`);
   `CassetteRecordPort` writes to exactly the path it was handed. The recorder writes into a temp
   file the test drops, and `Upload refreshed cassettes` (`:96-101`) uploads the **unchanged,
   checked-in** files.
4. **And `record` is unreachable without `--live` anyway** — `main.rs:4642`'s `MAOS_JOURNEY_MODE`
   check sits *inside* the `if run.live` block opened at `:4470`.

⚠ **The arming hazard.** `xtask/src/cassette_age_gate.rs` (`MAX_AGE_DAYS = 14`, `:4`) sees all three
cassettes at ~90 days — 3/3 stale. It returns `Ok` **only** because the stamp file `.tier2-first-success`
(`:3`; resolved against `stamp_dir.unwrap_or(cassette_dir)` and checked at `:57-65`) does not exist,
and **nothing in the tree writes it**. It runs on every push (`discipline.yml:494`) and in the
nightly (`journey-nightly.yml:40`), permanently green by vacuity. **The moment Tier-2 succeeds once
and something writes that stamp, the gate hard-fails on the seed cassettes.** This story repairs the
leg's *env and flags* and fixes the tempfile defect (effort, not scope — §11), and **deliberately
does not create the stamp**. That decision is recorded, not silent.

---

## 7. 🔴 THE LATTICE CANNOT BE EXPRESSED BY EDITING BRANCH BODIES, AND THE CONFLICT CHECK MUST FIRE 185 LINES EARLIER THAN THE EPIC THINKS

The Researcher selector is **one nested `if`, not three peers**, and its head is at `main.rs:4470`,
not `:4633`:

```
4470  if run.live {                                   // ← the head of the selector
4471-4619   … ~150 lines of live MCP wiring (LiveResearcherMcpPort, :4601) …
4633        let researcher_inference = InferencePortAdapter::new(…)   // ← what the epic cites
4642        if std::env::var("MAOS_JOURNEY_MODE").as_deref() == Ok("record") {
4643-4644       … requires MAOS_REPLAY_CASSETTE …
4646            CassetteRecordPort::new(Box::new(researcher_inference), …, "live-record".into())
4652-4654   } else { Arc::new(researcher_inference) }
4657  } else if let Ok(cassette_path) = std::env::var("MAOS_REPLAY_CASSETTE") {
4658-4660   let strict = MAOS_REPLAY_STRICT == "1";
4661-4665   CassetteReplayPort::from_file(…)
4676  } else {
4677-4679   eprintln!("maos run: researcher deterministic survey (no --live; zero network)")
4680  }
```

Three consequences the epic does not state:

- **`record` structurally requires `--live`.** ADR-064 clause 2 makes a set mode authoritative, so
  `MAOS_INFERENCE_MODE=record` must record **without** `--live`. That cannot be done by editing
  branch bodies; the whole `if/else if/else` becomes a **resolved mode computed once, before the
  branch**.
- **Clause 3 (`replay` + `--live` = typed error) must fire before `main.rs:4285`.** That is Butler's
  own `if run.live`, which opens live MCP transports. Refusing at `:4470` means the process has
  already dialled MCP before it declines the configuration.
- **The whole block sits inside a `spirit_id == "researcher"` arm**, so lifting it to a shared
  helper is a real refactor, not a copy.

**Resolution point (D-15-6-B):** parse and validate the *configuration* (mode token, cassette
presence, `--live` conflict) immediately after `providers_map`/`default_id` are finalised at
`main.rs:3160` — which is **before** the shell branch at `:3178` and **before** Butler's `:4285` —
and evaluate the *live-provider predicate* at the same point, where the map is in scope. One
resolution, three consumers, no duplicated precedence.

---

## 8. 🔴 THE CONSUMER INVENTORY IS WRONG IN FOUR PLACES, AND TWO OF THEM ARE COMPILE-BREAKING

| epic claim | measured |
|---|---|
| *"Butler is `McpClientPort`-only (`spirits/butler/src/lib.rs:812-825`)"* | **FALSE.** Butler **declares inference**: `spirits/butler/manifest.toml:20-21` `provider.complete = ["anthropic.claude-3-haiku-20240307"]`. The cited lines are a doc comment on `fn block_on_sync` (`:812`, `:815`) and a panic string (`:825`). What is true: Butler's Rust has **zero** `InferencePort` references — a *declared-but-unwired* consumer, wired by 18-1. |
| *"the shared adapter … consumed by shell/hello-spirit via `main.rs:3281-3284`"* | **INCOMPLETE.** The binding has five arms: shell (`:3282`, moved), `drop(inference)` (`:4021`, `:4969`), `smoke_multi_provider_5` (`:7109`), and **`MAOS_ONE_SHOT=hello-spirit` (`:8092`, `maos_spirit_hello::run(&inference, token)`)** — a second live consumer the epic does not name, registered at `verbs.rs:155` and covered by `crates/maos-bin/tests/one_shot_hello.rs`. |
| — | ⚠ **`smoke_multi_provider_5` takes the CONCRETE type**: `fn smoke_multi_provider_5(inference: InferencePortAdapter, …)` (`main.rs:8793-8794`), called at `:7109` under `#[cfg(feature = "fixture_replay")]`. Rebinding `inference` to `Arc<dyn InferencePort>` **earlier than `:7109` breaks the `fixture_replay` build.** The wrap must happen at `:3281-3282` (inside the shell arm) and at `:8092`, not at `:3170`. |
| *"the eight Spirits that declare no inference (`spirits/*/tests/spirit_smoke.rs` …)"* | **count right, citation wrong.** 11 Spirit directories; 3 declare `provider.complete` (butler `:21`, hello-spirit `:12`, researcher `:47`); 8 do not. But only 9 of 11 have a `spirit_smoke.rs`, and only **four** mention inference (`mira:6,99,128`, `nash:6,109`, `observer:6,112`, `orchestrator:8,71`). The file documents 4 of the 8. |
| *"`InferencePortAdapter` itself (`kernel-core/src/inference/mod.rs:38`) is untouched"* | **CONFIRMED and enforced**: `:38` is the struct; the trait has one method (`maos-domain/src/ports/inference.rs:20-25`); the file is inside the kernel pin **and** at zero kloc headroom. |

⚠ **The visibility worry is misdirected.** `cassette_replay` is bin-private (`mod cassette_replay;`
at `main.rs:31`, absent from `lib.rs`) — but the wrap happens **in `main.rs`, where it is already in
scope** (used that way at `:4646`, `:4661`). `run_shell` already takes
`Arc<dyn InferencePort + Send + Sync>` (`maos-shell/src/lib.rs:165-170`). **`maos-shell` needs zero
delta** and keeps its 195-line headroom for Epic 16. One ordering caveat: `CassetteRecordPort::new`
takes `Box<dyn …>`, so a record wrap needs `Box::new(inference)` before the `Arc` at `:3281-3282`.

---

## 9. 🔴 THE EPIC'S ORIGINAL AC4 (`UserFacing` PROMOTION) IS DECORATIVE, AND THE GATE IT RELIES ON HAS THREE SILENT BYPASSES

`EnvStability` (`env_contract.rs:8-11`) has exactly two variants and **zero readers anywhere in the
workspace**: `env_contract::lookup` (`:481-483`) has no production callers, `check_env_contract.rs`
never parses the `stability:` line, `STABILITY.md` contains no `MAOS_` token, and `maos --help`
renders `verbs::VERBS` only (`verbs.rs:87-130`). Registering `MAOS_INFERENCE_MODE` as `UserFacing`
and promoting `MAOS_REPLAY_CASSETTE` with it is a **declaration with no gate behind it**. That is
still the right declaration (F14 is ratified, and a `UserFacing` mode whose required argument is
`HarnessOnly` is incoherent) — but the story must not claim it is enforced. Building the reader is
**not** funded here; it is filed (§ Declared cut lines).

**Three bypasses the dev must not walk into**, all in `xtask/src/check_env_contract.rs`:

1. **String literal required** (`:62-79`). `const MODE: &str = "MAOS_INFERENCE_MODE"; env::var(MODE)`
   is **invisible** to the gate — the variable escapes the contract entirely. Read it as a literal.
2. **The registry is parsed by line prefix**, not AST (`:103-115`: `trimmed.starts_with("name: \"MAOS_")`).
   A row collapsed onto one line registers **nothing**. Keep the 5-line `EnvVar { name / purpose /
   stability }` shape.
3. **It walks `crates/maos-bin/src` only** (`:119-126`). A read from `maos-shell` or
   `maos-journey-test` is unenforced (workspace-wide coverage is 21-4 / D-I). **Keep the read at the
   `main.rs` composition root and pass the resolved mode inward.**

⚠ **One-directional.** The gate flags unregistered *reads*; it never flags a registered row with no
reader. Retiring `MAOS_JOURNEY_MODE` therefore requires **explicitly deleting** `env_contract.rs:259-263`
— no gate will notice a leftover dead row.

---

## 10. 🔴 `--replay-llm` IS NOT A NO-OP, SO RETIRING IT IS A BEHAVIOUR CHANGE

`worker_spawn.rs:59` is `"--replay-llm" => live = false,` inside an **order-sensitive** `for` loop
(`:52-70`). `maos run m --live --replay-llm` yields `live == false`; the reverse order yields
`true`. Both the comment at `:56-58` and ADR-064's own text call it a no-op — true only when
`--live` is absent. The historical documented command string uses the losing order
(`_bmad-output/test-artifacts/atdd-checklist-8-14b-j-butler-acceptance.md:177,198,223,239`:
`maos run butler --live --replay-llm`, which **does not run live**).

Measured reach: `--replay-llm` appears in **zero** `.rs` files outside `worker_spawn.rs:56,59` and
the 15-5 gate test, and in **zero** workflows or scripts. Retirement is safe — but it is a
*behaviour* removal, not a token deletion, and the ADR sentence that calls it a no-op is corrected
in this commit (§12).

⚠ **Related, and also corrected:** ADR-064's *"Ten existing `maos run … --once` callers rely on those
unset semantics"* is not reproducible. Measured: **35 invocation sites across 18 files**
(`grep -rn -- '"--once"' crates spirits xtask` = 36, one of which is the parser arm itself), of which
**8** load the Researcher, and **zero** set `MAOS_REPLAY_CASSETTE` explicitly. "Ten" is the count of
`spirits/researcher/manifest.toml` *references*, two of which pass `--live` **with** a cassette set
and therefore do not rely on unset semantics at all.

---

## 11. SIZING, BUDGET, AND THE LEDGER ROW THIS STORY MUST RE-BOOK

| file | change | charged lines |
|---|---|---|
| `crates/maos-bin/src/inference_mode.rs` (NEW) | mode enum, parse, lattice clauses 1–5, typed errors, live-provider predicate | **+150–200** |
| `crates/maos-bin/src/cassette_replay.rs` (**round 2**) | `CassetteReplayProvider` + `CassetteRecordProvider` — a re-skin of the two existing types onto `Provider` (`&InferenceRequest`, `ProviderError`, `credential_fingerprint`) | **+60–100** |
| `crates/maos-bin/src/main.rs` `~:3160` | resolve once; **replace every `providers_map` entry** when the mode is set. ⚠ Round 2 **removes** the three per-consumer wraps (`:3281-3282`, `:8092`, and the Researcher wrap) from the default path — they return only on a D-15-6-I refusal | **+25–40** (was +50–70) |
| `crates/maos-bin/src/main.rs` `:4470-4680` | re-key the researcher selector onto the resolved mode; preserve unset behaviour byte-for-byte | **+20–35** |
| `crates/maos-bin/src/cassette_replay.rs` | `"text"` in the recorder (§2); `provenance` write + required read (§5) | **+20–35** |
| `crates/maos-bin/src/lib.rs` | `pub mod inference_mode;` + its doctrine doc-comment | **+3–8** |
| `crates/maos-bin/src/env_contract.rs` | +5 (`MAOS_INFERENCE_MODE`), promote `MAOS_REPLAY_CASSETTE`, −5 (`MAOS_JOURNEY_MODE`) | **≈0** |
| `crates/maos-bin/src/worker_spawn.rs:54-59` | retire `--replay-llm` + its comment | **−5** |
| `crates/maos-journey-test/src/lib.rs` | record path writes to the source cassette, not the tempfile copy (§6.3) | +15–30 |
| `crates/maos-bin/tests/*`, `xtask/tests/decision_adrs_and_provisioning.rs`, ADR-064, `journey-nightly.yml`, 3 cassettes | | **free** (kloc excludes any path containing `tests`, `kloc_check.rs:303`) |

⚠ **RE-DERIVED IN ROUND 2, because the seam changed and the story's own §11 went stale inside the
same document that indicts stale budgets.** `maos-bin` total **`+268…+408`** measured; ×1.3 (rule 6)
= **`+349…+531`**, against 2760 headroom — it still fits ~5×. Nothing is cut and no crate needs a
grant. The ledger re-book (AC6) therefore lands at **`15-6 +350–550`**, not the `+300–450` this document carried an hour ago.
⚠ **T0 re-measures both the estimate and the seam** — the spike decides which column of this table is
real, so the ledger row is written *after* T0, never before.

⚠ **But the ledger row is wrong and is re-booked in this commit.** `kloc.toml:404` books
`15-6 +150–300`. The measured plan exceeds it. Silently drawing the difference from Epics 16–21's
share is exactly the reservation-vs-grant confusion this epic's own 15-2 was written to end. AC6
re-books the row to `15-6 +350–550` with a one-line `MEASURED GRANT`-style note, in the same commit
as the lines it authorizes.

⚠ **Two traps priced in:**
- **The 18-file doorbell.** `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:869` is
  `const SCANNED_SOURCE_FILES: [(&str, &str); 18]` with an `include_str!` per file, and `:948-952`
  asserts the array length equals the count of `crates/maos-bin/src/*.rs`. A new `inference_mode.rs`
  makes that 19. The test's own comment (`:861-868`) calls it *"a doorbell, not a wall, and the
  sanctioned move is to ring it"* — 15-3 rang it 17 → 18. **Ring it: 18 → 19, in the same commit.**
  (The alternative — extending an existing file to dodge it, the 14-2b precedent recorded at
  `kloc.toml:405` — is refused here: the doctrine at `crates/maos-bin/src/lib.rs:32,:52` wants the module in the
  library so `crates/maos-bin/tests/` can name it, and that is worth one array bump.)
- **`xtask` has 35 lines.** This story needs **zero** `xtask/src` change. A single stray helper there
  reds `kloc-check`.

---

## Decisions ratified in this story (rule 7 — one decision per fork)

| id | fork | ruling | rejected, and why |
|---|---|---|---|
| **D-15-6-A** | F15's live predicate cannot fire (§4) | ⚠ **NARROWED at the 2026-09-09 round-table — this is no longer a change to the ratified Decision.** F15 already has the shape: *"`UnconfiguredProvider` remains installed but is **explicitly flagged**."* Ollama-at-the-guessed-default is **another flagged fallback**, not a new predicate — **one boolean computed in the block that already reads `MAOS_OLLAMA_URL` (`main.rs:3139-3151`)**, carried beside the `UnconfiguredProvider` flag F15 already mandates. ADR-064's **`## Decision` stands unchanged**; only its `## Context` is corrected, because the Context was wrong about the world. | The story's first form — *"a provider registered from an explicit operator signal"* — needed a fact **the router does not model**, so it would have threaded a second source of truth about providers out of the composition root. Implementing F15 literally — the predicate never fires in a default boot and admits a dead `localhost:11434`; a null control. Probing the provider — network in the hermetic path. |
| **D-15-6-B** | where the mode is resolved (§7) | **One** resolution immediately after `providers_map`/`default_id` are finalised (`main.rs:3160`) — before the shell arm (`:3178`), before Butler's `--live` MCP dial (`:4285`), before the researcher selector head (`:4470`). Config validation and the provider predicate happen together, where the map is in scope. | Editing the three researcher branch bodies — cannot express `record` without `--live`, and refuses clause 3 only after MCP transports are open. |
| **D-15-6-C** | seam placement: wrap the port vs. install a replay `Provider` in the router | ⚠ **REVERSED at the 2026-09-09 round-table, then REFINED in round 2.** Default is the **router-level `CassetteProvider`**, and it **replaces the VALUE of every `providers_map` entry, not just `default_id`** — ⚠ **values only: the key set and `default_id` are PRESERVED**, because `router.default_id()` is read at `main.rs:3186` (the shell's `default_provider`) and `:8066` (the one-shot token provider id) — in replay mode *no provider is live*, which is the semantic, so whatever identifier the capability token names resolves to the cassette. Every replayed completion therefore still passes through `InferencePortAdapter::complete` — capability check (`inference/mod.rs:262`), Transparency-Log row, IAC frame, rate limiter — and **every** consumer replays with no per-consumer wrap. `credential_fingerprint()` is overridden with a hash of the **cassette path**, mirroring Ollama's endpoint-hash precedent (`ollama.rs:44-48`), because the trait's default is a `u64::MAX` sentinel its own doc says every provider MUST override (`provider.rs:21-30`). Gated by **rule 5 (spike before build)**: T0 proves a replayed completion through the adapter mints and verifies its `Scope::ProviderInfer` and lands a TL row, **and** that an exhausted cassette under `MAOS_REPLAY_STRICT=1` still exits non-zero after `ProviderError` crosses the adapter's error path. ⚠ **REPLAY does not attach the rate limiter** (`.with_rate_limiter(...)`, `main.rs:3176`); **`record` and `live` DO.** ⚠ *(Caught on read-back, not in the room: "the mode-set path does not attach the limiter" was **wrong for `record`** — record makes real provider calls, on the paid path, and is exactly what the limiter exists to throttle. The rule is per-mode, not per-set-ness.)* A **replayed** completion makes no provider call and therefore consumes no provider quota. Without this, replay becomes rate-limited **for the first time** — it has never been, because it bypassed the adapter — against a default `anthropic` quota of **1000 rpm** (`rate_limit.rs:279`; `openai` 3500, `ollama` effectively unlimited), and a fast hermetic replay would throttle itself intermittently on calls it never makes (`epic-21…:52` replays fifty incidents). Refusal is governed by **D-15-6-I**. | **Registering under `default_id` alone was ALSO wrong** and would have re-introduced the same defect wearing a different hat: Lunarpulse's own probe shows `default_id` is **`"ollama"`** on a keyless box (`maos: Ollama provider registered`) while `spirits/hello-spirit/manifest.toml:12` declares `provider.complete = ["anthropic.claude-haiku-4-5-20251001"]` — so the seam's correctness would vary with **whether Ollama happens to be installed**: green on a maintainer's laptop, a different colour in CI. Replacing every entry removes the choice and the environment dependence. The original ruling (port-wrap) rejected the router seam on a scope argument that was **wrong** — it assumed a *new* `"replay"` identifier; replacing the existing identifiers makes scopes match by construction, and is strictly *more* correct than HEAD, which mints `Scope::ProviderInfer { provider: "replay" }` (`main.rs:4668`, `:4831`) for an identifier that appears in **no manifest in this repository** and is never verified, because the verifier is the port replay replaced. ⚠ Two mechanisms is real but bounded: the unset arm is frozen legacy that ADR-064 itself calls the compatibility path, with a named successor — a migration, not duplication. |
| **D-15-6-D** | how `provenance` becomes falsifiable (§5) | ⚠ **RE-RULED at the 2026-09-09 round-table: lenient at runtime, strict in CI.** `CassetteReplayPort::from_file` **accepts** a cassette without `provenance` (so `maos.journey.cassette/v1` keeps meaning one thing) and refuses only a *present-but-illegal* value. The **requirement** lives in a corpus test in `crates/maos-journey-test/tests/` (uncharged; `journey-hermetic-tier-1` is in `aggregate.needs`, `discipline.yml:3673`): every file under `cassettes/**` carries `provenance ∈ {seed, live-record}`, with the **denominator asserted first** (≥3 cassettes) so an empty glob reds. Planted vectors: absent · garbage · empty glob. The recorder writes `live-record`; the three seeds are re-stamped `seed`. | **A required read in `from_file` is a schema break hiding inside an unchanged version string** — a v1 file written yesterday would be rejected by a reader still claiming v1 (Boundary). Bumping to `/v2` is drift: epics 18, 19 and 21 all cite `v1` by name. A field nothing reads (the epic's wording) — unfalsifiable. A new `xtask/src` gate — 35 lines of headroom. |
| **D-15-6-E** | an authoritative mode with no wired consumer (Butler, §8) | **Validate globally, wire incrementally.** The selector is parsed and enforced for every `maos run`/`maos shell` regardless of which Spirit loads; a Spirit with no inference seam is unaffected and inherits the seam when its own story wires it (18-1 Butler, 19-3 Orchestrator, 21-1 Mira/Nash). `epic-16…:20`'s Butler line is therefore *accepted and inert until 18-1*, and the epic says so. | Failing when any loaded Spirit lacks a seam — breaks `epic-16…:20`. Silently ignoring the mode per-consumer — restores the vacuity `epic-18…:22` already records. |
| **D-15-6-F** | the cassette-age gate arming hazard (§6) | Repair the nightly leg's env, flags **and** the tempfile defect; **do not** create `.tier2-first-success`. Recorded here so the next author finds the reason rather than the absence. | Writing the stamp — arms a 14-day gate against three 90-day seeds and hard-fails `discipline.yml:494` on every push. |
| **D-15-6-G** | what the re-keyed anti-rot anchor should assert (round-table) | ⚠ **The story's first form was a receipt.** *"`MAOS_INFERENCE_MODE` present in `env_contract.rs`"* asserts a fact **`check-env-contract` already guarantees** — it passes forever whether or not the selector does anything. The anchors instead point at **what a future author could delete with no gate noticing**: (a) the **conflict refusal** in `inference_mode.rs` (the lattice), and (b) the **seam location**, so ADR-064 becomes the single place the seam is written. **Falsifier:** delete the refusal and keep the registry row — the old anchor stays green, the new one reds. | Anchoring to the registry row — third sighting of `maos summon-dragon` in this room (15-3, 15-5, here): a string that exists proves nothing about the control. |
| **D-15-6-H** | who owns the seam's location, so it stops going stale (round-table) | **ADR-064 carries it once, machine-checked by D-15-6-G(b); the five consuming epics cite the ADR, not `main.rs:<line>`.** The §12 row-12/13/14 line repairs stay as today's truth and are explicitly marked **perishable** — T2 moves `main.rs:4470-4680` in this very story, so they are re-derived at the landing commit and never hand-maintained again. | Hand-repairing twelve `file:line` citations across three epics — which is R6 (14-2c, *cite symbols never lines*) committed **in the act of fixing R6 violations**, and invalidated by the story's own T2. |
| **D-15-6-I** | what happens if the T0 spike REFUSES (round 2 — Grumbal's question, previously unruled) | **The story ships; the CLAIM shrinks.** The fallback does not change *what* is shipped — the unmediated replay path is HEAD's behaviour already — it changes **what may be claimed about it**. On a measured refusal: (a) the per-consumer port wrap is taken (`main.rs:3281-3282`, `:8092`, and the Researcher arm); (b) **AC1's own text gains the sentence** *"a replayed completion is not mediated and produces no Transparency-Log row"*; and (c) that sentence is filed as a **`RELEASE-HOLDS.md` claim boundary row**, NOT to `deferred-work.md` — five epics make *"MAOS replays hermetically"* a CI claim, and a qualifier on a claim belongs attached to the claim. **Falsifier: if nobody wrote the sentence, the fallback was not taken honestly.** | Stalling five epics on one measurement (John: the story ships either way). Filing to `deferred-work.md` (Vex: wrong weight for a claim boundary; Dana's objection that `RELEASE-HOLDS` is a GA-ledger surface was heard and overruled on exactly that ground). Fixing whatever refuses — scope creep on a story already carrying eight ACs. |
| **D-15-6-J** | who writes the cassette under the router seam (round 3) | ⚠ **`Drop` is not enough any more, and this is the reversal's own defect one layer out.** `CassetteRecordPort` writes on `Drop` (`cassette_replay.rs:178-203`); under the router seam the recorder lives inside `Arc<MultiProviderRouter>`, there are **four `Arc::clone(&router)` sites** and **no `drop(router)` anywhere in `main.rs`** (measured) — `drop(inference)` at `:4021`/`:4969` drops the adapter, not the router. **A `record` run can therefore exit without writing the cassette at all.** Ruling: `CassetteRecordProvider` gains an explicit **`flush()`** called at the drain points (`main.rs:4021`, `:4969`) and before `run_shell` returns; `Drop` is demoted to a backstop. **Falsifier:** a one-shot `record` run that exits normally writes the file; delete the `flush()` call and it does not. | Leaving `Drop` as the only writer — the story reversed a ruling to fix *"record produces a file that replays as nothing"* and would have shipped *"record produces no file at all"*. |
| **D-15-6-K** | `record` that captures nothing (round 3) | **`flush()` errors on zero entries.** `Drop` today returns early on an empty entry list (`cassette_replay.rs:181-183`), so *no provider configured*, *provider unreachable* and *the Spirit made no calls* all produce **silence and exit 0** — the exact false-success shape this story exists to kill. One line, and it catches all three. ⚠ **This is a property of the RECORDER, not a mode predicate, so ADR-064's Decision still does not move** (D-15-6-A's narrowing is preserved). | Extending F15's live predicate to `record` (Wildcard proposed, then withdrew) — it would have re-opened the ADR Decision to catch a subset of what one line in the recorder catches. |
| **D-15-6-L** | the fallback's boundary is filed to a document nobody reads (round 3) | **Named, not gold-plated.** `grep -rn RELEASE-HOLDS xtask/src .github/workflows` is **empty** — eleven claim-boundary rows, zero machine readers. This story does **not** build that gate (35 lines of `xtask` headroom, and a gate for a branch resolved before code is written is the gold-plating Dana exists to stop). Instead: if D-15-6-I's fallback is taken, the `RELEASE-HOLDS.md` row **cites the runnable probe** from §1 that demonstrates the boundary, and **review clause (j) is binding on a non-author**. Recorded so the next reader finds the reason rather than the absence. | Silently filing it — *"a field nothing reads cannot be a control"* is this story's own §5, and it applies to prose too. Building a `RELEASE-HOLDS` reader — a lane, not a task. |

---

## Acceptance Criteria (8)

- **AC1 (command).** The ten-probe table in §1 is the control, run against the built `maos` binary,
  and **the six legs that exit 0 at HEAD flip**. Specifically:
  `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-researcher/survey-distill.json maos run spirits/researcher/manifest.toml --once`
  exits **0** and replays *because the mode said so* (probe 2), while probes **3, 4, 5, 6 and 7**
  each exit **non-zero** with a typed configuration error naming the offending variable, and probe
  **1** is byte-unchanged (ADR-064 clause 1). The **new observable** is probe 9: with
  `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j0/shell-intro.json`,
  `maos shell` fed `@hello-spirit …` prints the cassette's own
  `"Hello! I am the MAOS hello-spirit reference implementation…"` (`shell-intro.json:13`) instead of
  HEAD's `Inference transport error — the configured provider is unreachable`, and probe 10's
  `diff` of the with/without-cassette shell runs is **no longer empty**.
  ⚠ **Falsifier:** reverting only the mode-resolution call must restore every HEAD row of the table.
  ⚠ **AC1 must not be claimed on the strength of probe 2 alone** — probe 2 passes at HEAD (§1).
  Mode precedence, unset compatibility, missing-cassette behaviour, live-provider eligibility,
  strictness orthogonality and retired-flag behaviour are **as ratified in ADR-064 F11–F15 and as
  amended by D-15-6-A**; this AC does not redefine them.

- **AC2 (the selector).** A new `crates/maos-bin/src/inference_mode.rs`, `pub mod`-ed from
  `crates/maos-bin/src/lib.rs` (doctrine: `crates/maos-bin/src/lib.rs:32`, `:52` — *"an in-`src` test module is budget-charged and CI-invisible"*), resolves
  `MAOS_INFERENCE_MODE ∈ {record, replay, live}` **once**, at `main.rs:~3160` — after
  `providers_map`/`default_id` are finalised (`:3155-3160`) and **before** the shell arm (`:3178`),
  Butler's `--live` MCP dial (`:4285`) and the researcher selector head (`:4470`) — per D-15-6-B.
  It implements ADR-064's lattice exactly: **unset** ⇒ today's selection byte-for-byte (`--live`
  wins; else cassette presence selects replay; else deterministic); **set** ⇒ authoritative;
  `replay` + `--live` ⇒ typed error; `record`/`replay` without `MAOS_REPLAY_CASSETTE` ⇒ typed error;
  an unrecognised value ⇒ typed error; `live` ⇒ the D-15-6-A predicate. Errors are a `thiserror` enum
  **in `maos-bin`** (precedent `tenant_map.rs:14-40` `TenantMapBootError`; `thiserror = "2"` already a
  dependency, `Cargo.toml:101`), surfaced through the existing `String`/`Box<dyn Error>` `?` chain so
  the exit code is non-zero. ⚠ **Not in `maos-domain`** — `xtask/error-catalog.toml`'s `scan_dirs`
  excludes `maos-bin` but includes `maos-domain`, where a new variant immediately owes a six-field
  `[[error]]` row or `error-catalog-check` reds. ⚠ **T0 runs the D-15-6-C spike before any of
  this is written** (epic confidence **rule 5**, *spike before build*): register a stub `Provider`
  under the resolved `default_id`, drive one replayed completion through `InferencePortAdapter`, and
  confirm `Scope::ProviderInfer { provider: <default_id> }` mints **and verifies** at
  `inference/mod.rs:262` and that a Transparency-Log row lands. Provider-level is the default seam;
  the port-wrap is the fallback **only** on a measured refusal, and that refusal is filed. `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:869`
  `SCANNED_SOURCE_FILES` goes **18 → 19** with the new file, in the same commit (§11).

- **AC3 (every inference consumer, through the mediated path).** Under the **D-15-6-C** seam a set
  mode replaces the **value** of every `providers_map` entry with a cassette-backed `Provider` — in
  replay mode no provider is live, which is the semantic — so **every** inference consumer replays
  through `InferencePortAdapter`. ⚠ **Values only: the key set and `default_id` are preserved**
  (`router.default_id()` is read at `main.rs:3186` and `:8066`). ⚠ **`replay` does not attach the rate limiter**
  (`main.rs:3176`); **`record` and `live` do** — record makes real provider calls on the paid path
  and is precisely what the limiter exists to throttle. A replayed completion consumes no quota
  because it consumes no provider, and without this exemption replay becomes rate-limited for the
  first time against a default `anthropic` 1000 rpm (`rate_limit.rs:279`) — an intermittent
  self-throttle on calls it never makes. This AC is discharged by the router, not by three copies of a branch, and
  **FR47 holds on the replayed path**: capability check (`inference/mod.rs:262`), Transparency-Log
  row, IAC frame and rate limiter all still run. The Researcher arm (`main.rs:4470-4680`) takes the
  adapter path whenever the mode is set, and its **unset** three-branch behaviour is byte-identical
  (clause 1). ⚠ **Two hazards the port-wrap plan carried DISAPPEAR here**: `inference` is never
  rebound, so `smoke_multi_provider_5`'s concrete-type trap (`main.rs:8793-8794`, called `:7109`
  under `#[cfg(feature = "fixture_replay")]`) is untouched, and the **`MAOS_ONE_SHOT=hello-spirit`
  consumer (`main.rs:8092`)** — which the epic does not name — is covered for free because it takes
  the same router. ⚠ **On a measured spike refusal only**, the fallback is the per-consumer port
  wrap — the shell arm (`main.rs:3281-3282`, before `maos_shell::run_shell` at `:3283`;
  `Box::new(inference)` before the `Arc` when recording), the `:8092` arm, and the Researcher arm,
  with **both hazards back in scope** and `cargo check -p maos-bin --features fixture_replay` as the
  proof — governed by **D-15-6-I**: AC1 gains the non-mediation sentence and it is filed as a
  `RELEASE-HOLDS.md` claim boundary, not to `deferred-work.md`.
  `crates/maos-kernel-core/src/inference/mod.rs` and `crates/maos-domain/src/ports/inference.rs` are
  **untouched**; `maos-shell` takes **zero** delta (it already accepts
  `Arc<dyn InferencePort + Send + Sync>`, `lib.rs:165-170`). Butler
  (`spirits/butler/manifest.toml:20-21`, a *declared but unwired* consumer) is validated-but-inert
  per D-15-6-E until 18-1. ⚠ **Two green tests constrain the shape**: `journey_researcher.rs:46`
  (PTY) and `:122` (stderr) assert the literal `"deterministic survey"` (`main.rs:4677-4679`), and
  `journey_researcher.rs:80-95` sets a cassette **and** passes `--live` — so cassette-presence stays
  a selector only while `MAOS_INFERENCE_MODE` is unset and must never become an implicit explicit
  mode.

- **AC4 (the record→replay round trip, §2).** The recorder writes `"text": resp.text` into the
  entry's `response` object (`cassette_replay.rs:205-239`), so a recorded cassette replays the
  model's answer instead of `""`. ⚠ **Record is SYMMETRIC with replay (round 2):** under the
  D-15-6-C seam the recording wrapper composes the inner **`Provider`**, not the `InferencePort` —
  record and replay then sit at the **same layer**, one mechanism rather than two, and record keeps
  the mediation it already has. The three measured `Provider`-trait deltas apply to both wrappers:
  `complete` takes **`&InferenceRequest`** (not by value), returns **`ProviderError`** (cassette
  exhaustion maps `InferenceError::ProviderTransport` → `ProviderError::Transport`), and
  **`credential_fingerprint()` MUST be overridden** — the trait default is a `u64::MAX` sentinel
  whose own doc says every provider must override it, and leaving it would share a rate-limit bucket
  with unconfigured drivers (`provider.rs:17-31`); override with a hash of the **cassette path**,
  mirroring Ollama's endpoint-hash precedent (`ollama.rs:44-48`).
  ⚠ **The recorder gains an explicit `flush()` (D-15-6-J) and it errors on zero entries (D-15-6-K).**
  Under the router seam `Drop` alone cannot be relied on — the recorder lives inside
  `Arc<MultiProviderRouter>`, there are four `Arc::clone(&router)` sites and **no `drop(router)`
  anywhere in `main.rs`**, so a `record` run can exit having written nothing. `flush()` is called at
  the drain points (`main.rs:4021`, `:4969`) and before `run_shell` returns; `Drop` stays as a
  backstop. It **errors on an empty entry list** (today `Drop` returns early at
  `cassette_replay.rs:181-183`), so *no provider*, *unreachable provider* and *the Spirit made no
  calls* stop being silence-and-exit-0. ⚠ **`flush()` is idempotent** — it marks itself done so the
  `Drop` backstop cannot double-write or double-error. ⚠ **Deliberate and slightly surprising:** an
  interactive `maos shell` under `record` in which the operator types nothing exits **non-zero**.
  That is the intended reading — *you asked to record and recorded nothing* — and it is stated here
  so it is not discovered as a bug. **Falsifiers:** a one-shot `record` that exits normally
  writes the file, and deleting the `flush()` call reds it; a `record` run that captures nothing
  exits non-zero. Proven **hermetically** by a test in
  `crates/maos-bin/tests/` (kloc-free): record through a stub `InferencePort` into a temp cassette,
  drop the port, re-open it with `CassetteReplayPort::from_file`, and assert the replayed
  `InferenceResponse.text` **equals the recorded text**. ⚠ A round-trip assertion over `stop_reason`
  or `usage` alone passes on the broken recorder and is not this AC. The mislabelled
  `"model_id": "live-recorded"` (`:187`) and the `session: "live-record"` argument (`main.rs:4650`)
  are corrected to a real `model_id` from `resp.provider_attribution.model_id` and a session name
  that is a session name.

- **AC5 (provenance, §5).** Cassette `schema_version` stays `maos.journey.cassette/v1`
  (`cassette_replay.rs:27-35`; the epic cites `:28-33`). A top-level `provenance` field is added and
  is **load-bearing without breaking `v1`** (D-15-6-D): `CassetteReplayPort::from_file` **accepts** a
  cassette with no `provenance` and refuses only a *present-but-illegal* value — a required read
  would be a schema break hiding inside an unchanged version string. The **requirement** is a corpus
  test in `crates/maos-journey-test/tests/` (uncharged; `journey-hermetic-tier-1` is in
  `aggregate.needs`, `discipline.yml:3673`): every file under `cassettes/**` carries
  `provenance ∈ {seed, live-record}`, with the **denominator asserted first** (≥3 cassettes) so an
  empty glob reds. Planted vectors: absent · garbage · empty glob. `CassetteRecordPort`'s `Drop` serializer
  (`:185-190`) writes `"live-record"`. The three seed cassettes —
  `crates/maos-journey-test/cassettes/{j0/shell-intro.json, j-researcher/survey-distill.json,
  j-butler/on-idle-halt.json}` (note the crate prefix the epic omits), each `"model_id":
  "hand-authored-seed"` at line 4 — are re-stamped `"provenance": "seed"`. `journey_j0.rs` and
  `journey_researcher.rs` assert the label. ⚠ The requirement is stated in the ADR amendment as well
  as in code, because every future cassette (`epic-19…:84`'s J1, `epic-21…:52`'s J4) inherits it.

- **AC6 (the environment contract, §9 + §11).** `crates/maos-bin/src/env_contract.rs`:
  `MAOS_INFERENCE_MODE` is registered `EnvStability::UserFacing` in the **5-line**
  `EnvVar { name / purpose / stability }` shape (the `:1-13` format; the gate parses the registry by
  line prefix, `check_env_contract.rs:103-115`); `MAOS_REPLAY_CASSETTE` (`:255`) is **promoted**
  `HarnessOnly → UserFacing` per F14; `MAOS_REPLAY_STRICT` (`:265`) stays `HarnessOnly` and
  orthogonal per F13; **`MAOS_JOURNEY_MODE`'s row (`:259-263`) is explicitly deleted** together with
  its only reader (`main.rs:4642`) — the gate is one-directional and will not notice a dead row. **`--replay-llm` is
  retired** (`worker_spawn.rs:54-59`; usage strings `:34,:65,:78`) — ⚠ it is **not** a no-op (§10), so
  this is a behaviour removal, not a token deletion. The
  env read is a **string literal at the `main.rs` composition root**, never a `const` and never from
  outside `crates/maos-bin/src` (both are silent bypasses, §9). `cargo run -p xtask -- check-env-contract`
  exits 0. In the same commit, `xtask/kloc.toml:404`'s ledger row is re-booked `15-6 +150–300` →
  `15-6 +350–550` with a measured note (§11); `cargo run -p xtask -- kloc-check` and
  `cargo run -p xtask -- check-kernel-baseline` both exit 0 (`maos-kernel-core/src` still 24474).

- **AC7 (the anti-rot re-key, §3).** `cargo test -p xtask --test decision_adrs_and_provisioning`
  exits 0 with all **three** tests green, after both ADR-064 anchors are re-keyed **forward** in
  `xtask/tests/decision_adrs_and_provisioning.rs:388-421` — and, per **D-15-6-G**, re-keyed at
  **what a future author could delete with no gate noticing**, not at a fact another gate already
  owns: (a) `require_contains` the **conflict refusal** in `crates/maos-bin/src/inference_mode.rs`
  (the lattice itself), (b) `require_contains` the **seam location** the ADR now carries once
  (D-15-6-H), and (c) `require_absent(…"\"--replay-llm\" => live = false"… worker_spawn.rs)`
  replacing the old `require_contains`; the fixture context string at `:841` and the fixture files at
  `:909-917` move with them. ⚠ **`require_contains("MAOS_INFERENCE_MODE" in env_contract.rs)` is
  NOT an acceptable re-key** — `check-env-contract` already guarantees it, so it passes forever
  whether or not the selector does anything. **Falsifier for the whole AC:** delete the conflict
  refusal and keep the registry row — the naive anchor stays green, these anchors red. `docs/adr/ADR-064-one-inference-mode-selector.md`'s `## Context` is rewritten to
  describe the post-15-6 tree (the anchors also assert the Context still *names* each token,
  `:186-198`, `:208-212`), and `## Decision` absorbs D-15-6-A and the `provenance` requirement.
  ⚠ **Deleting an anchor instead of re-pointing it is forbidden by the gate's own doctrine**
  (`:229-233`). ⚠ `xtask/tests/` is kloc-excluded — this AC costs zero budget and must not add a line
  under `xtask/src` (35 lines of headroom).

- **AC8 (the nightly leg, §6).** `.github/workflows/journey-nightly.yml`: the misdirected `--live`
  argument is removed from the nextest invocation (`:94`, it sits after `--no-fail-fast` with no
  `--` separator; it is a `maos run` flag, not a nextest one — the grammar rejection itself is
  derived, not executed, §6); the `tier-2-rerecord` leg (`:65`) sets **`MAOS_INFERENCE_MODE: record`** in place
  of `MAOS_JOURNEY_MODE: record` (`:87`); and the harness defect that made the leg incapable of its
  own purpose is fixed — `ReplayProvider` (`crates/maos-journey-test/src/lib.rs:346-362`, `:118-123`)
  points the **record** path at the source cassette instead of the tempfile copy, so
  `Upload refreshed cassettes` (`:96-101`) ships changed files — **proven hermetically** (the paid
  leg cannot be a control): record through a stub provider and assert the file **on disk** changed,
  in `crates/maos-journey-test/tests/`. The paid execution itself remains the
  operator row `ops-nightly-cassette-rerecord` and is **not** claimed here. ⚠ Per **D-15-6-F**, this
  story **does not** create `crates/maos-journey-test/cassettes/.tier2-first-success` — creating it
  arms `cassette_age_gate.rs:57-65` against three 90-day seed cassettes under a 14-day limit and
  hard-fails `discipline.yml:494` on every push. The reason is recorded in the workflow, not left as
  an absence.

---

## Declared cut lines

Named so a reviewer can tell a boundary from an omission.

1. **Butler's inference port is not wired.** Butler declares `provider.complete`
   (`spirits/butler/manifest.toml:20-21`) and has zero `InferencePort` references in its Rust.
   `epic-16…:20` and `epic-18…:14` both run it under the selector; per **D-15-6-E** the selector is
   validated and inert for Butler until **18-1**. The epic's Butler sentence is corrected (§12) so
   the boundary is written down rather than implied.
2. **Orchestrator / Mira / Nash seams** stay with 19-3 and 21-1; this story gives them one function
   to call instead of a branch to copy.
3. **`EnvStability` has no reader** (§9). The `UserFacing` promotion is a declaration. Building a
   reader (a `maos --help`/docs surface that renders the user-facing env set) is **filed to
   `deferred-work.md`**, routed to **16-4** (`docs/maos.dev/run-maos.md`, `epic-16…:79`) — it is a
   documentation surface, not a selector.
4. **NOT a cut line any more — this is now CLOSED by default.** ⚠ At HEAD, replay bypasses
   `InferencePortAdapter::complete` (`kernel-core/src/inference/mod.rs:248`, capability check
   `:262`): no capability check, no Transparency-Log row, no IAC frame, no rate limiter — so a
   hermetic replay journey audits **less** than the live run it stands in for, and every Epic-18/21
   assertion over a recorded turn proves the Spirit's behaviour and nothing about the kernel's.
   Worse: HEAD *mints* `Scope::ProviderInfer { provider: "replay" }` (`main.rs:4668`, `:4831`) for an
   identifier that appears in **no manifest in this repository**, and nothing verifies it because the
   verifier is the port replay replaced. ⚠ **And as of AC6 this is a `UserFacing`, documented input**
   — an unmediated path from an operator-supplied file into a Spirit's model output, with no audit
   row. That is a category, not an exploit, and it is why **D-15-6-C was reversed**: the router-level
   seam keeps FR47 whole. **Filed only on a measured spike refusal**, and then loudly.
5. **`cassette-age-gate` stays vacuous** (D-15-6-F). Deciding who writes `.tier2-first-success` and
   what happens to three 90-day seeds is an **operator** decision; filed with the epic's
   `ops-nightly-cassette-rerecord` row.
6. **The workspace-wide env surface** (reads outside `crates/maos-bin/src`) remains **21-4 / D-I**.
   This story stays inside the gate's scan by construction (§9).

---

## Dev notes

**Read before writing a line.**

- **Measure first (T0).** Re-run the frontmatter's gate sweep and `kloc-check` at your actual HEAD.
  A grant is a global, not a reservation; 15-1+15-3+15-5 already spent +240 of `maos-bin` against a
  booked `~10`.
- **`touch xtask/tests/decision_adrs_and_provisioning.rs` before you believe a red from it.** A
  scratch copy of the repo built with a shared `CARGO_TARGET_DIR` leaves a binary whose
  `CARGO_MANIFEST_DIR` points outside this tree; the test then reports every file missing and looks
  catastrophically red for no reason. Verified during this preflight.
- **There is a second git worktree** at `.claude/worktrees/agent-a9a6d76f71ebb443b` (detached
  `ee3422d0`, a stale pre-15-x tree where the shared adapter sits at `main.rs:3110`). Exclude it from
  every grep or you will get doubled, wrongly-numbered hits.
- **The composition root is one ~6,700-line `async fn main`** (`main.rs:1659-1661` to the next
  top-level `fn` at `:8346`). Both adapter sites live inside it. Existing error idiom at both is
  `String` via `?` into `Box<dyn Error>` — non-zero exit is already plumbed; "typed" is the only new
  part.
- **Do not reword `"maos run: researcher deterministic survey (no --live; zero network)"`**
  (`main.rs:4677-4679`). `crates/maos-journey-test/tests/journey_researcher.rs:46` (PTY screen) and `:122` (stderr) both
  assert that literal, and it is the only mechanical proof of the unset-default branch in the repository.
- **`journey_researcher.rs:80-95` sets a cassette AND passes `--live`.** Under ADR-064 clause 1 that
  must keep passing. An implementation that treats cassette-presence as an implicit `replay` and then
  conflicts it with `--live` **reds this test**. Cassette-presence stays a *selector* only when
  `MAOS_INFERENCE_MODE` is unset; it never becomes an implicit explicit mode.
- **`journey_j0.rs:69,:72,:74` asserts banner strings only** and will pass identically before and after
  this story. AC1's probe 9 is the real proof; 16-2 tightens the journey test itself.
- **`InferencePortAdapter` is not in `SERVICE_ADAPTERS`** (`xtask/src/check_service_boundary.rs:19-27`),
  which is why two production constructions are already legal and why this story cannot red P1.
- **Where things actually are** (measured at `2fd10398`; downstream epics cite these ~34 lines low
  and are corrected in §12): shared adapter `main.rs:3170`; shell return `:3283-3288`; providers
  `:3119-3159`; drops `:4021`, `:4969`; `smoke_multi_provider_5` call `:7109`, definition `:8793`;
  one-shot hello `:8092`; Butler `--live` `:4285`; researcher selector head `:4470`, adapter `:4633`,
  `MAOS_JOURNEY_MODE` `:4642`, record wrap `:4646`, cassette arm `:4657`, strict `:4658-4660`,
  deterministic `:4676-4679`, close `:4680`.

**Testing standards.** Proven-red vectors go in `crates/maos-bin/tests/` (kloc-free) and name
`inference_mode` items through `crates/maos-bin/src/lib.rs`. Every AC that asserts a refusal carries
a planted vector that reds *before* the fix; a clause without its own vector is a refactor pending a
red, not a proven control (`decision_adrs_and_provisioning.rs:4-11` is the house style).

### Project Structure Notes

- New file: `crates/maos-bin/src/inference_mode.rs` + `pub mod inference_mode;` in
  `crates/maos-bin/src/lib.rs` → **ring the 18 → 19 doorbell** at
  `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs:869` in the same commit.
- No new crate, no new crate edge, no `xtask/src` line, no `maos-shell` line, no kernel line.
- Variance from the epic: the epic implies the seam is two wraps; measurement makes it one resolved
  mode plus **three** consumer wraps, a recorder repair and an anti-rot re-key (§8, §2, §3). The epic
  is amended to match in this commit (§12).

### References

- [Source: `_bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md#15-6-inference-replay-seam` (`:191-199`)]
- [Source: `docs/adr/ADR-064-one-inference-mode-selector.md` — F11–F15, amended by AC7 / D-15-6-A]
- [Source: `_bmad-output/implementation-artifacts/15-5-decision-adrs-and-provisioning-checklist.md#AC6` (`:806-826`), F11–F15 (`:656-660`)]
- [Source: `_bmad-output/planning-artifacts/epics/requirements-inventory.md` — **FR47** (Spirits obtain inference only via the kernel Inference Port), **NFR-Aud-3** (ADR-028 trace-shape replay determinism — orthogonal; ADR-064 governs mode selection, not cassette content)]
- [Source: `_bmad-output/planning-artifacts/epics/dependency-dag.md:213-214`]
- [Source: `_bmad-output/planning-artifacts/epics/epic-16-…:16,:20,:26,:37,:78,:79,:108` · `epic-18-…:14,:16,:17,:22,:29` · `epic-19-…:14,:23,:57,:59,:84,:89` · `epic-20-…:22,:34,:43,:113` · `epic-21-…:52,:93`]

---

## Tasks / Subtasks

- [x] **T0 — measure, do not inherit** (AC: all)
  - [x] Re-run the seven gates in the frontmatter plus `check-kernel-baseline`; record exits.
  - [x] `cargo run -p xtask -- kloc-check`; record `maos-bin`, `xtask`, `maos-kernel-core`,
        `maos-journey-test` measured/ceiling. Re-book `kloc.toml:404` only against **your** number.
  - [x] `touch xtask/tests/decision_adrs_and_provisioning.rs && cargo test -p xtask --test decision_adrs_and_provisioning` — must be **3 passed** before you change anything.
  - [x] Run the §1 probe table against the pre-change binary and paste the ten rows into the Dev
        Agent Record. This is the story's baseline red.
  - [x] **The D-15-6-C spike (epic rule 5, before any code is written).** Register a stub `Provider`
        in **every** `providers_map` entry; drive one replayed completion through
        `InferencePortAdapter`; confirm its `Scope::ProviderInfer` mints **and verifies**
        (`inference/mod.rs:262`) and that a Transparency-Log row lands.
  - [x] Spike sub-item: exhaust the cassette under `MAOS_REPLAY_STRICT=1` and confirm the process
        still exits **non-zero** once `ProviderError::Transport` crosses the adapter's error path.
        Epic 18 uses `MAOS_REPLAY_STRICT` three times; if the adapter softens it, the flag stops
        meaning what those three exit lines assume.
  - [x] Spike sub-item: confirm the key set and `default_id` survive the value replacement —
        `router.default_id()` feeds `main.rs:3186` and `:8066`.
  - [x] Spike sub-item: confirm a `record` run's cassette is on disk **after a normal exit**, not
        only at process teardown (D-15-6-J: there is no `drop(router)`).
  - [x] Record both results. **Green ⇒ provider-level seam. Refused ⇒ D-15-6-I** (port-wrap, AC1
        gains the non-mediation sentence, and it is filed as a `RELEASE-HOLDS.md` claim boundary —
        not `deferred-work.md`).
  - [x] Re-derive §11's line estimate against whichever seam the spike licensed, **then** write the
        `kloc.toml:404` ledger row. Never before.
- [x] **T1 — the selector** (AC: 2)
  - [x] `crates/maos-bin/src/inference_mode.rs`: mode enum, literal-string parse, ADR-064 lattice
        clauses 1–5, D-15-6-A predicate, `thiserror` error enum.
  - [x] `pub mod inference_mode;` in `crates/maos-bin/src/lib.rs` with its doctrine doc-comment.
  - [x] `cohort_daemon_smoke_13_5c.rs:869` `; 18]` → `; 19]` + the `include_str!` row.
  - [x] Resolve once at `main.rs:~3160`; assert by test that the refusal happens before `:3178`,
        `:4285` and `:4470`.
- [x] **T2 — the seam** (AC: 3)
  - [x] **Default path (spike green):** `CassetteReplayProvider` / `CassetteRecordProvider` replace
        **every** `providers_map` entry at `main.rs:3139-3160` when the mode is set; a set mode
        routes every consumer through `InferencePortAdapter`. Assert the TL row and the capability
        check in a test. `credential_fingerprint()` overridden with a hash of the cassette path
        (`ollama.rs:44-48` precedent) — the trait default is a `u64::MAX` sentinel.
  - [x] Values only — preserve the key set and `default_id`. Do **not** attach `.with_rate_limiter`
        on the mode-set path (D-15-6-C, round 3).
  - [x] Re-key the researcher selector `main.rs:4470-4680`; **unset path byte-identical**
        (`journey_researcher.rs:46`/`:122` guard it).
  - [x] **Fallback path (spike refused) only:** per-consumer port wraps at `main.rs:3281-3282`
        (`Box::new` before `Arc` on the record path) and `:8092`; file the FR47 asymmetry.
  - [x] `cargo check -p maos-bin --features fixture_replay` (the `smoke_multi_provider_5`
        concrete-type trap, `:8793`).
- [x] **T3 — repair the recorder** (AC: 4)
  - [x] `"text": resp.text` in `cassette_replay.rs:212-232`; real `model_id`; honest `session`.
  - [x] Record composes the inner **`Provider`**, not the `InferencePort`, so record and replay sit
        at the same layer (one mechanism). Map exhaustion `InferenceError::ProviderTransport` →
        `ProviderError::Transport`.
  - [x] Explicit `flush()` at `main.rs:4021`, `:4969` and before `run_shell` returns; `Drop` becomes
        a backstop (D-15-6-J). `flush()` **errors on zero entries** (D-15-6-K). Plant both reds.
  - [x] Hermetic round-trip test in `crates/maos-bin/tests/` asserting the **text** survives.
- [x] **T4 — provenance: lenient reader, strict corpus** (AC: 5)
  - [x] `from_file` **accepts** absent `provenance`, refuses a present-but-illegal value; write
        `"live-record"` in `Drop`; re-stamp the three seeds `"provenance": "seed"`.
  - [x] Corpus test in `crates/maos-journey-test/tests/`: every `cassettes/**` file carries a legal
        `provenance`, **denominator (≥3) asserted first**; planted vectors absent · garbage · empty
        glob.
- [x] **T5 — environment contract and ledger** (AC: 6)
  - [x] Add `MAOS_INFERENCE_MODE` (`UserFacing`, 5-line shape); promote `MAOS_REPLAY_CASSETTE`;
        delete `MAOS_JOURNEY_MODE` row `:259-263` **and** its reader `main.rs:4642`.
  - [x] Retire `--replay-llm` (`worker_spawn.rs:56-59`, usage strings `:34,:65,:78`).
  - [x] Re-book `kloc.toml:404`; run `check-env-contract`, `kloc-check`, `check-kernel-baseline`.
- [x] **T6 — re-key the anti-rot anchors and amend the ADR** (AC: 7)
  - [x] Re-key `decision_adrs_and_provisioning.rs:388-421` per **D-15-6-G**: anchor (a) the conflict
        refusal in `inference_mode.rs`, (b) the seam location ADR-064 now carries once, (c)
        `require_absent` for `--replay-llm`. **Not** the `env_contract.rs` registry row — a receipt.
        Move the fixture context `:841` and fixture files `:909-917`.
  - [x] Prove D-15-6-G's falsifier: delete the conflict refusal, keep the registry row — the naive
        anchor stays green, these red.
  - [x] Rewrite ADR-064 `## Context` for the post-15-6 tree; fold D-15-6-A, the `provenance`
        requirement, the corrected `--replay-llm` "no-op" sentence and the corrected caller count.
  - [x] `cargo test -p xtask --test decision_adrs_and_provisioning` = 3 passed, planted reds still red.
- [x] **T7 — the nightly leg** (AC: 8)
  - [x] `journey-nightly.yml`: drop `--live` (`:94`), `MAOS_JOURNEY_MODE` → `MAOS_INFERENCE_MODE`
        (`:87`); record path writes the source cassette (`maos-journey-test/src/lib.rs:346-362`).
  - [x] Hermetic control for the harness repair: record through a stub and assert the file **on
        disk** changed (the paid leg cannot be a control).
  - [x] Comment in the workflow recording D-15-6-F (why `.tier2-first-success` is not created).
- [x] **T8 — rule 9: land the remaining spec edits** (AC: 7, and all)
  - [x] §12 **rows 1–14 already landed with this spec** (epic-15 §15-6 re-derived 4 → 8 ACs, the
        header's fourth refinement round, the Dependencies line, `dependency-dag.md:214`, and the
        stale cites in epics 16/18/19). **Verify they are still present** at your HEAD — another
        agent may have touched these files — and apply **rows 15–18** (the ADR-064 amendments) with
        the code, since the anti-rot anchors must flip in the same commit as the tree.
  - [x] File the two Declared-cut-line items (FR47 mediation asymmetry; `EnvStability` reader) to
        `deferred-work.md` with routing.
- [x] **T9 — close** (AC: all)
  - [x] Re-run the §1 probe table; paste the after-rows beside the T0 rows.
  - [x] `cargo test --workspace --no-fail-fast` (epic exit line 1) and the full gate sweep.

### Review Findings

_(§A6 net, 2026-09-11: Blind + Edge Case + Acceptance + Test-Infra + non-author runtime. 3 decision-needed, 11 patch, 0 deferred, 0 dismissed. All findings verified non-author at runtime or statically; falsifiers executed in a scratch worktree.)_

- [x] [Review][Decision] **HIGH — AC1 probe 8 is unmet as written and needs spec reconciliation** — §1's row 8 says record "records, and the file replays with its text"; measured, a keyless `record` run exits 1 (`flush refused: zero successful inference responses`) because the Researcher makes no completions without `--live`, and the success path is proven only by the hermetic stub round-trip. The Dev Record's reinterpretation (T9) is documented but contradicts the AC's letter; either amend AC1's row-8 expectation (zero-completion refusal IS the D-15-6-K behavior) or add a stub-provider probe. Note also §1's row-8 HEAD observable ("deterministic survey") was itself not reproducible: with `MAOS_REPLAY_CASSETTE` set to any path, HEAD selected cassette replay (exit 0, valid file) or failed init (exit 1, invalid file) — T0's row-8 observable ("cassette replay selected") is the correct one. — **RESOLVED by Lunarpulse 2026-09-11: amend AC1 row 8** (zero-completion refusal is the specified behavior; hermetic round-trip stays the success-path proof; §1's row-8 HEAD observable corrected to T0's).
- [x] [Review][Decision] **HIGH — replay exhaustion is converted to a false success on the hello-spirit surface** — measured: `MAOS_INFERENCE_MODE=replay` + an exhausted (zero-entry) cassette through `maos shell` exits **0** printing the canned "Inference transport error — the configured provider is unreachable" text (`maos-spirit-hello/src/lib.rs:98-110` converts `Err(InferenceError::ProviderTransport)` to `Ok(HelloResponse)`; `cassette_replay.rs:194-201` maps exhaustion to `ProviderError::Transport`). Violates the story's own capability statement ("when the mode cannot be honoured the process says so and exits non-zero"); indistinguishable from HEAD's broken probe 9. Fix location is ambiguous: distinguish exhaustion as its own error kind (adapter/provider mapping) vs. gating hello's fallback arm when a cassette mode is explicit. The T0 spike proved exhaustion non-zero only at the adapter level, not for this consumer. — **RESOLVED by Lunarpulse 2026-09-11: distinct error kind** (exhaustion gets its own `ProviderError` variant surfaced as a distinct `InferenceError` that hello's transport-fallback arm does not swallow; typed refusal, non-zero exit).
- [x] [Review][Decision] **LOW — recorder `credential_fingerprint()` hashes the cassette path, contrary to the `Provider` contract** — AC4 itself mandated this ("override with a hash of the cassette path"), but `provider.rs:21-29` documents the value as identifying credential bytes; record keeps the rate limiter, so `RateLimited` frames label a file path, and all cassettes on one path share a limiter bucket (`cassette_replay.rs:352-360`). Spec-compliant but contract-violating: keep the ruling (accepting the telemetry/limiter-identity cost) or forward the inner provider's fingerprint on the record path. — **RESOLVED by Lunarpulse 2026-09-11: keep the ruling**; document the tradeoff in ADR-064.
- [x] [Review][Patch] **HIGH — nightly rerecord leg reds on its own package run** (job-level `MAOS_INFERENCE_MODE=record` reaches cassette-free tests) [.github/workflows/journey-nightly.yml:86-93] — **FIXED 2026-09-11**: cassette-free children scrub the inherited mode (`Pty::spawn` env_remove when the world has no cassette; `env_remove` at the raw `Command` sites j1×4/j4×2/researcher/jb3-helper); cassette-bearing worlds keep it (they are the re-record targets). Verified: full package green under `MAOS_INFERENCE_MODE=record cargo test`.
- [x] [Review][Patch] **MEDIUM — live predicate admits empty credentials** (`MAOS_ANTHROPIC_API_KEY=''` and `MAOS_OLLAMA_URL=' '` both measured passing `Live { explicit: true }`, exit 0) [crates/maos-bin/src/main.rs:3119-3152] — **FIXED**: trimmed-key check gates the Anthropic flag; `trim()` on the Ollama explicit-URL test. Verified: both vectors now exit 1 `LiveProviderUnavailable`.
- [x] [Review][Patch] **MEDIUM — present non-UTF-8 `MAOS_INFERENCE_MODE` silently treated as unset** [crates/maos-bin/src/main.rs:3174] — **FIXED**: `var_os` read + new `InferenceModeError::NonUtf8Mode`. Verified: raw `\xff\xfe` exits 1 typed.
- [x] [Review][Patch] **MEDIUM — recorder still hardcodes top-level `"model_id": "live-recorded"`** [crates/maos-bin/src/cassette_replay.rs:284-287] — **FIXED**: derived from the first entry's `provider_attribution.model_id` (`unattributed` fallback).
- [x] [Review][Patch] **MEDIUM — standalone cli_wrapper worker path returns before record finalization** [crates/maos-bin/src/main.rs:4209] — **FIXED**: routed through the fallible flush before `return Ok(())`.
- [x] [Review][Patch] **MEDIUM — flush drain calls are unpinned (D-15-6-J falsifier fails)** [crates/maos-bin/tests/inference_mode_15_6.rs] — **FIXED**: `record_with_zero_completions_exits_non_zero` subprocess test (shell EOF + record + temp cassette ⇒ non-zero + `zero successful inference responses` + no file); `Drop` cannot set an exit code, so the drain call is now load-bearing.
- [x] [Review][Patch] **LOW — seam anchor is a disjunction** [xtask/tests/decision_adrs_and_provisioning.rs:407-415] — **FIXED**: per-arm anchors `CassetteReplayProvider::from_file(cassette, *strict)` and `CassetteRecordProvider::new(` beside the loop anchor; fixture content + two targeted planted reds (each arm deleted alone); ADR-064 Context names both constructions.
- [x] [Review][Patch] **LOW — shell replay test asserts the mode banner, not the cassette text** [crates/maos-bin/tests/inference_mode_15_6.rs:97-118] — **FIXED**: feeds `@hello-spirit hi`, asserts the cassette's exact text and the absence of the transport-error fallback.
- [x] [Review][Patch] **LOW — rate-limiter polarity has no assertion** [crates/maos-bin/tests/inference_mode_15_6.rs:35-43] — **FIXED**: `rate_limiter_polarity_is_mode_scoped` covers replay-exempt vs record/live/unset×3.
- [x] [Review][Patch] **LOW — empty-string `MAOS_REPLAY_CASSETTE` changes unset-mode behavior** [crates/maos-bin/src/main.rs:3175-3177] — **FIXED**: empty stays a present value. Verified: unset+empty exits 1 `cassette read: "": No such file or directory` (HEAD parity).
- [x] [Review][Patch] **LOW — shell-path flush error order swallows the causal failure** [crates/maos-bin/src/main.rs:3358-3372] — **FIXED**: shell failure wins; flush error on that path is reported, not fatal-masked; success-path refusal stays fatal.
- [x] [Review][Patch] **HIGH — AC1 row 8 amendment** (from Decision 1) — **APPLIED**: §1 row 8 rewritten (zero-completion typed refusal is the specified post-state; HEAD observable corrected to T0's cassette-presence row; hermetic round-trip remains the success-path proof).
- [x] [Review][Patch] **HIGH — distinct exhaustion error kind** (from Decision 2) — **APPLIED**: `CassetteReplayProvider` maps exhaustion/drift to `ProviderError::Serde` → `InferenceError::MalformedResponse` (hello propagates it instead of converting to the canned success); and `maos-shell` fails the turn (`maos: error: …` + non-zero exit). Constraint honored: a new `ProviderError` variant would force a kernel-core match-arm edit inside the zero-headroom pin, so the existing `Serde` vehicle carries the distinct `InferenceError`. Verified: exhausted cassette through `maos shell` exits 1; good cassette exits 0 with text. Deviation from AC4's letter ("exhaustion maps to `ProviderError::Transport`") is recorded here.
- [x] [Review][Patch] **LOW — ADR-064 records the fingerprint tradeoff** (from Decision 3) — **APPLIED**: the Decision section carries the ruling sentence (record-mode rate-limit identity intentionally keyed by cassette path, not credential bytes).

---

## 12. Epic OLD→NEW edits (rule 9)

**Rows 1–10 are APPLIED IN THIS SPEC COMMIT** (`0773ec6e` set the precedent: *"spec landed, and the
epic amended to match"*). Epic-15's §15-6 was re-derived at `2fd10398`, grown **4 → 8 ACs numbered
1:1 with this story's**, and a **fourth refinement round** paragraph added to the epic header.

| # | file | OLD | NEW | state |
|---|---|---|---|---|
| 1 | `epic-15…` §15-6 AC1 | *"the Researcher-private adapter at `main.rs:4633` … are both wrapped at the composition root"* | resolve **once** at `:~3160`; **three** consumer wraps — shell `:3281-3282`, **`MAOS_ONE_SHOT=hello-spirit` `:8092`**, researcher `:4470-4680`; the selector head is **`:4470`**, not `:4633`; `smoke_multi_provider_5` (`:8793`) pins the concrete type | **applied** |
| 2 | same, AC1 | *"Butler is `McpClientPort`-only (`spirits/butler/src/lib.rs:812-825`)"* | **Butler DECLARES inference** (`spirits/butler/manifest.toml:20-21`), *declared-but-unwired*; the cite was a doc comment on `block_on_sync`; `spirits/*/tests/spirit_smoke.rs` documents **four** of the eight, not eight | **applied** |
| 3 | same, AC1 | AC1 discharged by its happy path | AC1 discharged by the **ten-probe table** — the command already exits 0 at HEAD (§1) | **applied** |
| 4 | same, **new AC2/AC3** | — | the selector, the lattice that cannot be built by editing branch bodies, the typed-error home (`maos-bin`, **not** `maos-domain`), and the two green tests that constrain the shape | **applied** |
| 5 | same, **new AC4** | — | **the recorder does not write `response.text`** (`cassette_replay.rs:212-232` vs `:53-57`) — record→replay is silently empty (§2) | **applied** |
| 6 | same, AC2 → **AC5** | *"a `provenance` field is added"*, paths `cassettes/…` | **required and refused** by `from_file` (D-15-6-D); paths are `crates/maos-journey-test/cassettes/…`; no serde struct and no cassette gate exists; all three cassettes are inert | **applied** |
| 7 | same, AC4 → **AC6** | *"`MAOS_REPLAY_CASSETTE` remains the cassette path (`:255`)"* | **promoted `HarnessOnly → UserFacing`** per F14; `MAOS_JOURNEY_MODE`'s row `:259-263` **deleted**; `--replay-llm` retired (not a no-op, §10); ledger `:404` re-booked `+150–300` → `+350–550`; the 18 → 19 doorbell | **applied** |
| 8 | same, **new AC7** | — | **this story reds `decision_adrs_and_provisioning.rs` twice** and re-keys both anchors forward (§3) | **applied** |
| 9 | same, AC3 → **AC8** | *"`journey-nightly.yml:94` no longer passes `--live` …"* | + the leg is secret-gated and **has never executed**, and even repaired cannot refresh a checked-in cassette (tempfile copy) — AC8 fixes that too; `.tier2-first-success` **deliberately not created** (D-15-6-F) | **applied** |
| 10 | same, **Dependencies** + Stories table + header | *"15-6 before Epic 18"* | *"15-6 before Epics **16**, 18, 19, **20** and 21"*; table row marked *8 ACs (was 4), refined 2026-09-09*; fourth-refinement-round paragraph added | **applied** |
| 11 | `epics/dependency-dag.md:214` | `15-6 → 18-*, 19-*, 20-4, 21-1` | `15-6 → **16-2**, 18-*, 19-*, **20-1**, 20-4, 21-1` | **applied** |
| 12 | `epic-16…:26,:108` | `run_shell` at `:3249`, `maos init` `:1679`, `maos shell` `:1730`, shell consumer `main.rs:3247-3250` | `:3283`, `:1699`, `:1743`, `:3281-3283` **+ the `:8092` one-shot consumer** | **applied — ⚠ PERISHABLE** |
| 13 | `epic-18…:22,:74` | `MAOS_REPLAY_STRICT` `main.rs:4624-4626`; cassette read `:4623`/`:4623-4641` | `:4658-4660`; `:4643`/`:4657-4665`, selector head `:4470` | **applied — ⚠ PERISHABLE** |
| 14 | `epic-19…:59,:84` | *"the shared adapter is `main.rs:3136`"*; template `:4590-4641`; strict read `:4624` | `:3170`; template `:4633-4680`; `:4658`; and the Orchestrator's third adapter **calls** `inference_mode` rather than copying the branch | **applied — ⚠ PERISHABLE** |

⚠ **Rows 12–14 are today's truth and nothing more (D-15-6-H).** They repair `file:line` citations
that went stale — by writing twelve new `file:line` citations, which is **R6** (14-2c: *cite symbols,
never lines*) committed in the act of fixing R6 violations. **T2 of this story moves
`main.rs:4470-4680`, so they are wrong again in this story's own commit.** They are re-derived at the
landing commit, and the durable fix is D-15-6-H: ADR-064 carries the seam location **once**,
machine-checked by D-15-6-G(b), and the five consuming epics cite the ADR instead of a line number.

**Rows 15–18 land with the CODE, not with this spec**, because the ADR-064 anti-rot anchors must flip
in the same commit as the tree they describe (AC7), and because row 17 carries a **design change to a
decision the operator ratified on 2026-09-08** — flagged in Open question 1, not pre-ratified here.

| # | file | OLD | NEW | state |
|---|---|---|---|---|
| 15 | `docs/adr/ADR-064-…md` `## Context` | *"When no real provider is configured, `main.rs:3155-3159` installs `UnconfiguredProvider`…"* | **measured false** — `OllamaProvider::new` is infallible and needs no credential (`ollama.rs:30-40`; `main.rs:3139-3151`), so a keyless boot registers *ollama* and `providers_map` is never empty (§4) | with the code |
| 16 | same, `## Context` | *"accepts `--replay-llm` as a no-op"* / *"Ten existing `maos run … --once` callers"* | **order-sensitive `live = false`**; **35 sites across 18 files**, 8 Researcher-loading, **zero** setting `MAOS_REPLAY_CASSETTE` (§10) | with the code |
| 17 | same, `## Decision` | F15's predicate rejects only `UnconfiguredProvider` | ⚠ **NO CHANGE — D-15-6-A was narrowed at the round-table.** F15 already says the fallback is *flagged*; the guessed-Ollama default simply **joins the flagged set**. The Decision stands; only the Context (row 15) was wrong about the world | with the code |
| 18 | same, `## Decision` + `## Context` | — | + `provenance ∈ {seed, live-record}` required **of every cassette in the corpus** (D-15-6-D — the CI denominator, not the reader, so `v1` keeps meaning one thing); + **the seam location, stated once** (D-15-6-H) and machine-checked by the re-keyed anchors (D-15-6-G), so the five consuming epics cite the ADR instead of a line number | with the code |

---

## 13. Round-table 2026-09-09 — five findings, four rulings changed

The room convicted the story with the story's own §1 diagnosis, **five for five**, and every finding
is a *green that proves nothing*.

| # | finding | who | ruling |
|---|---|---|---|
| 1 | **The citation repair repeated the defect it was repairing.** §12 rows 12–14 fixed twelve stale `file:line` cites by writing twelve new ones — R6 (14-2c, *cite symbols never lines*) committed in the act of enforcing R6 — into a 6,700-line `main.rs` that **T2 of this story edits**, so the repair is invalid in its own commit. | Level, Mary, Grumbal | **D-15-6-H**: ADR-064 carries the seam location once, machine-checked; the five epics cite the ADR. Rows 12–14 marked PERISHABLE and re-derived at the landing commit. |
| 2 | **The seam ships a documented, unmediated injection path.** AC6 makes `MAOS_REPLAY_CASSETTE` `UserFacing`, and replay bypasses `InferencePortAdapter` — no capability check, no TL row, no IAC frame. HEAD already mints `Scope::ProviderInfer { provider: "replay" }` (`main.rs:4668`, `:4831`) for an identifier in **no manifest**, unverified because the verifier is the port replay replaced. *"I don't have an exploit. I have a category."* | Vex, Amelia, Murat | **D-15-6-C REVERSED.** Router-level `CassetteProvider` under the existing `default_id` keeps FR47 whole and collapses AC3; the scope argument that rejected it assumed a *new* `"replay"` id and was wrong. Gated by a **rule-5 spike** in T0; port-wrap is the measured fallback. |
| 3 | **The `provenance` requirement was a schema break inside an unchanged version string.** A required read in `from_file` rejects a `v1` file written yesterday while still claiming `v1`; bumping to `/v2` is drift (epics 18/19/21 cite `v1`). | Boundary, Yui, Murat, Paige | **D-15-6-D re-ruled: lenient at runtime, strict in CI.** Reader accepts absent, refuses illegal; the requirement is a corpus test in kloc-free `crates/maos-journey-test/tests/` with the **denominator asserted first**. |
| 4 | **The re-keyed anti-rot anchor was a receipt.** *"`MAOS_INFERENCE_MODE` present in `env_contract.rs`"* asserts what `check-env-contract` already guarantees — third sighting of `maos summon-dragon` in this room (15-3, 15-5, here). | Splinter, Level, Yui, Sally | **D-15-6-G**: anchor the **conflict refusal** and the **seam location** — what a future author could delete with no gate noticing. Falsifier: delete the refusal, keep the row; the naive anchor stays green. |
| 5 | **D-15-6-A needed a fact the router does not model.** *"registered from an explicit operator signal"* would thread a second source of truth about providers out of the composition root. | Wildcard, Winston, Level | **D-15-6-A narrowed.** F15 already flags its fallback; the guessed-Ollama default **joins the flagged set** — one boolean where `MAOS_OLLAMA_URL` is already read. **ADR-064's Decision stands unchanged**; only its Context was wrong about the world. |

**Also ruled.** *Doorbell* — 18 → 19 stands (Yui: the assertion's own message is *"so new files cannot
evade this negative"*; routing around it is what put 14,120 lines in `main.rs`, which is the same
complaint as finding 1). *Harness tempfile* — EFFORT confirmed, but only after Dana's hit landed
(a repair on a leg no test can reach); it ships **with a hermetic control**: record through a stub,
assert the file on disk changed. *Sizing* — **WHOLE**, fourth consecutive refusal: splitting AC4/AC5
off ships an AC1 that claims `record` works while `record` produces cassettes that replay as `""`,
and 18-1 would be the story that discovers it. **Dana's dissent recorded: outvoted on coupling, not
on cost — *"the coupling argument gets stronger every time you use it, which is how coupling
arguments always look right up until they're an excuse."***

### Round two, same day — the four edges the reversal itself opened

The room's own improvement left four loose ends. Closed in one round.

| # | edge | ruling |
|---|---|---|
| 1 | **Grumbal's question was never ruled** — what happens if the T0 spike refuses? Left as "port-wrap and file it", which is a fork with no decision, i.e. a **rule-7 gap** the dev agent would have closed under time pressure. | **D-15-6-I.** The story ships; the *claim* shrinks. The fallback changes nothing about what is shipped — the unmediated path is HEAD's behaviour — only what may be claimed. AC1 gains the non-mediation sentence and it becomes a **`RELEASE-HOLDS.md` claim boundary**, not a `deferred-work.md` row. Dana's "RELEASE-HOLDS is a GA-ledger surface" was heard and overruled on exactly that ground. |
| 2 | **`default_id` was the wrong identifier and re-introduced the defect the reversal fixed.** Lunarpulse's own probe shows `default_id` is **`"ollama"`** on a keyless box while `hello-spirit`'s manifest declares `anthropic.*` — a seam whose correctness varies with whether Ollama is installed. Vex: *"green on the maintainer's laptop and a different colour in CI."* | **D-15-6-C refined:** replace **every** `providers_map` entry. In replay mode no provider is live — that is the semantic. One `for` loop; the environment dependence and the choice both disappear. |
| 3 | **The reversal left `record` unspecified and three trait deltas uncited** (rule 4, *cite-or-cut*). Measured: `Provider::complete` takes **`&InferenceRequest`** and returns **`ProviderError`** (`provider.rs:17-31`), and `credential_fingerprint()`'s default is a **`u64::MAX` sentinel** its own doc says every provider MUST override — leaving it shares a rate-limit bucket with unconfigured drivers. | **AC4 extended.** Record composes the inner `Provider` too, so both halves sit at one layer. Fingerprint hashes the **cassette path**, mirroring `ollama.rs:44-48`. Boundary's catch became a spike sub-item: exhaust a cassette under `MAOS_REPLAY_STRICT=1` and prove the exit is still non-zero once `ProviderError` crosses the adapter — Epic 18 leans on that flag three times. |
| 4 | **§11's budget went stale inside the document that indicts stale budgets**, because the seam it priced was reversed an hour earlier. | **Re-derived:** three per-consumer wraps out, two `Provider` re-skins in → `+268…+408`, ×1.3 = **`+349…+531`**; ledger re-book moves `+300–450` → **`+350–550`**. And T0 now writes the ledger row **after** the spike, never before. Mary: *"say it out loud or it happens silently."* |

**Two hazards deleted by the refinement**, which Yui called the tell: `inference` is never rebound, so
`smoke_multi_provider_5`'s concrete-type trap is untouched, and the `MAOS_ONE_SHOT=hello-spirit`
consumer is covered for free.

### Round three — the five the router seam itself created

Same shape a third time: the fix was right and it opened new ground. All five measured, none speculative.

| # | finding | ruling |
|---|---|---|
| 1 | **A `record` run can exit having written nothing.** The recorder writes on `Drop`; under the router seam it lives inside `Arc<MultiProviderRouter>` with **four `Arc::clone(&router)` sites and no `drop(router)` anywhere in `main.rs`** (measured). Grumbal: *"you reversed a ruling to fix 'record produces a file that replays as nothing,' and the reversal produces no file at all."* | **D-15-6-J** — explicit `flush()` at the drain points and before `run_shell` returns; `Drop` demoted to a backstop. Falsifier: delete the call, the one-shot record reds. |
| 2 | **`Drop` returns early on an empty entry list** (`cassette_replay.rs:181-183`), so *no provider*, *unreachable provider* and *the Spirit made no calls* are all **silence and exit 0** — the story's own false-success shape. | **D-15-6-K** — `flush()` errors on zero entries. One line, catches all three. ⚠ A **recorder** property, not a mode predicate, so **ADR-064's Decision still does not move** — Wildcard proposed extending F15's live predicate to `record` and withdrew it as the more expensive way to catch a subset. |
| 3 | **"Replace every entry" is ambiguous** and one reading breaks `router.default_id()`, read at `main.rs:3186` (the shell's `default_provider`) and `:8066` (the one-shot token provider id). | Folded into **D-15-6-C**: replace the **values**; the key set and `default_id` are preserved. Said in the ruling, not left to the reviewer. |
| 4 | **Replay becomes rate-limited for the first time.** It never was — it bypassed the adapter. Routed through, it meets a default `anthropic` quota of **1000 rpm** (`rate_limit.rs:279`); replay is fast because it makes no network call, and `epic-21…:52` replays fifty incidents. Vex: *"a hermetic test that throttles itself on a quota for calls it never makes."* Murat: *"and it would fail intermittently, which is the worst kind."* | Folded into **D-15-6-C**: the mode-set path does **not** attach `.with_rate_limiter(...)`. A replayed completion consumes no provider quota because it consumes no provider. |
| 5 | **D-15-6-I files the fallback's boundary to a document nobody reads.** `grep -rn RELEASE-HOLDS xtask/src .github/workflows` is **empty** — eleven claim-boundary rows, zero machine readers. Splinter: *"the fallback's control is a document, in the story whose §5 says a field nothing reads cannot be a control."* | **D-15-6-L** — named, not gold-plated. The row cites the runnable §1 probe, and review clause (j) is binding on a non-author. No new `xtask` gate (35 lines of headroom; a gate for a branch resolved before code is written is gold-plating). Dana, quietly: *"that's the first time he's stopped short of a gate."* Splinter: *"I stop short when the gate wouldn't fire."* |

**Addendum, caught on read-back rather than in the room — recorded because hiding it would be the
session's own failure mode.** Round 3's rate-limiter ruling was written as *"the mode-set path does
not attach the limiter"*, which is **wrong for `record`**: record makes real provider calls on the
paid path and is exactly what the limiter exists to throttle. The rule is **per-mode** — `replay`
exempt, `record` and `live` throttled. Two smaller ones folded with it: `flush()` must be
**idempotent** so the `Drop` backstop cannot double-write, and D-15-6-K means an interactive
`maos shell` under `record` with no operator input exits **non-zero** — deliberate, and written down
so it is not later discovered as a bug.

**Grumbal's close, and it is now the review instruction:** *"Your citations repeated the mistake they
were fixing. Your seam ships a documented unmediated injection path and calls it a cut line. Your
schema change hides inside an unchanged version string. Your anti-rot anchor is a receipt. And your
predicate needed a fact your own router doesn't model. Five for five — and every one of them is the
story's own §1 diagnosis."*

---

## Dev Agent Record

### Agent Model Used

`openai-codex/gpt-5.6-sol`

### Debug Log References

**T0 baseline — the §1 probe table run against the PRE-change binary.** Paste all ten rows with
exit codes and the observable line. This is the story's proven red; AC1 cannot be closed without it.

| # | command | exit | observable |
|---|---|---|---|
| 1 | cassette only; Researcher `--once` | 0 | `researcher cassette-replay inference wired` |
| 2 | `replay` + Researcher cassette | 0 | same line; mode remained unread |
| 3 | `replay`, no cassette | 0 | `researcher deterministic survey (no --live; zero network)` |
| 4 | invalid mode | 0 | deterministic survey |
| 5 | `live`, no explicitly configured provider | 0 | deterministic survey; guessed Ollama registered |
| 6 | `replay` + cassette + `--live` | 0 | `researcher live-inference seam wired (--live)` |
| 7 | `record`, no cassette | 0 | deterministic survey |
| 8 | `record` + existing temp cassette | 0 | cassette replay selected; source file unchanged |
| 9 | replay J0 cassette through shell | 0 | `Inference transport error — the configured provider is unreachable` |
| 10 | shell EOF without/with replay variables | 0 / 0 | outputs byte-identical |

**T0 gate sweep.** `check-empty-kernel` · `check-service-boundary` · `kloc-check` ·
`check-env-contract` · `check-j1-loopback-delegation` · `check-exit-commands` ·
`check-kernel-baseline` · `cargo test -p xtask --test decision_adrs_and_provisioning`.

All eight invocations exited 0: `check-empty-kernel` (0 violations),
`check-service-boundary` (0 violations), `kloc-check` (aggregate 157231),
`check-env-contract` (88 registered, 0 violations), `check-j1-loopback-delegation`,
`check-exit-commands` (7 epics, 31 tokens, 6 owed), `check-kernel-baseline`
(24474/24474), and `decision_adrs_and_provisioning` (3 passed).

**T0 kloc.** `maos-bin` 17349/20109 · `xtask` 43409/43444 · `maos-kernel-core`
18935/18935 · `maos-journey-test` 493/1104 · aggregate 157231.

**T0 spike.** A temporary kernel-local test replaced the values for all
`anthropic`/`openai`/`ollama` router entries with one stub provider while preserving the key set and
`default_id = ollama`. A mediated completion minted and verified
`Scope::ProviderInfer { provider: "anthropic" }`, returned the stub response, and wrote one
`InferenceCall` Transparency-Log row. Replacing the same values with a strict-exhaustion transport
error crossed the adapter as non-zero `InferenceError::ProviderTransport`. The temporary test
passed and was removed byte-for-byte. Provider-level seam selected. The pre-change record probe
left its cassette unchanged, confirming the router-owned recorder needs the explicit idempotent
`flush()` required by D-15-6-J/K; normal-exit persistence is a mandatory implementation red.
The selected plan remains +268–408 charged `maos-bin` lines; ×1.3 = +349–531, so the ledger is
re-booked to +350–550 before production code.

**T1–T3 selector, provider seam, and recorder.** The production selector now resolves once before
router construction and rejects four invalid configurations before any inference consumer is
wired. Red-first integration tests cover the lattice and refusal order. Replay replaces every
router value while retaining the three keys and default; a mediated replay test proves capability
verification and one `InferenceCall` Transparency-Log row. Record wraps every provider, hashes the
cassette path for its credential fingerprint, preserves full response text/usage/attribution, and
round-trips the flushed file through the strict replay provider. Explicit flush is idempotent,
fails loudly with zero successful responses, and runs after shell, topology, single-Spirit, hello,
and graceful-daemon inference boundaries; `Drop` is only a backstop. Targeted evidence:
`cargo test -p maos-bin --test inference_mode_15_6` (8 passed) and
`cargo check -p maos-bin --features fixture_replay` (exit 0).

**T4 provenance.** `CassetteReplayPort::from_file` preserves v1 compatibility by accepting an
absent provenance field while refusing any present value outside `seed|live-record`; recorder
output now carries `live-record`. The three checked-in cassettes are stamped `seed`. The new corpus
gate asserts its denominator before validation and its planted missing, garbage, and empty-corpus
vectors fail. Evidence: `cargo test -p maos-bin --test inference_mode_15_6` (9 passed) and
`cargo test -p maos-journey-test --test cassette_provenance_15_6` (3 passed).

**T5 environment and ledger.** `MAOS_INFERENCE_MODE` is registered as `UserFacing`;
`MAOS_REPLAY_CASSETTE` is promoted to the same stability and now describes both record and replay.
The obsolete `MAOS_JOURNEY_MODE` read/row and accepted-no-op `--replay-llm` parser branch are gone;
an integration test pins the retired flag as an unknown argument. Actual measured `maos-bin` growth
is exactly +324 lines (17,349 → 17,673), below the re-booked +550 ceiling with 2,436 lines of
headroom. Evidence: `check-env-contract` PASS (88 registered, 0 violations), `kloc-check` PASS
(aggregate 157,559), and `check-kernel-baseline` PASS (24,474/24,474).

**T6 ADR anti-rot.** ADR-064 now describes the implemented selector, fallback eligibility,
provider-value seam, retired flag semantics, measured caller inventory, and split runtime/CI
provenance contract. Its anchors target the `ReplayLiveConflict` behavior, the
`providers_map.values_mut()` seam, and absence of the retired parser arm. Fixture mutations prove
all three directions, including the required falsifier where the environment registry receipt
remains while the conflict refusal disappears. Evidence:
`cargo test -p xtask --test decision_adrs_and_provisioning` (3 passed).

**T7 nightly recording.** The paid job now selects `MAOS_INFERENCE_MODE=record`, no longer passes
the invalid `--live` test-runner argument, and documents why it must not arm the stale-cassette
stamp. `ReplayProvider` retains both source and replay-copy paths: record mode exposes the source,
while unset/replay tests keep their isolated tempfile. A subprocess-isolated control routes stub
output through the harness-selected path and verifies the source file's bytes and cassette payload
changed. Evidence: `cargo test -p maos-journey-test --test recording_source_15_6` (1 passed,
1 ignored child) and `cargo test -p maos-journey-test --test journey_researcher` (3 passed);
`check-exit-commands` PASS.

**T8 specification alignment.** Re-verified the fourth-round Epic-15 rewrite, dependency DAG edge,
and ADR-064 citations in Epics 16, 18, 19, 20, and 21. ADR-064 and its index row carry the
implemented selector, compatibility lattice, mediated provider-value seam, provenance contract,
retired surfaces, and consuming stories. `deferred-work.md` routes the missing `EnvStability`
reader to 16-4 and records the FR47 fallback as closed while the mediated seam remains true, with
its contingency claim boundary routed to `RELEASE-HOLDS.md`.

**T9 post-change runtime probes.** Re-ran §1 against the built binary with provider/key variables
unset. Probe 10 initially remained byte-identical, so the closure stayed red; added an
empty-input-safe shell mode receipt and a subprocess regression test, then re-ran probes 9–10.

| # | after exit | after observable |
|---|---:|---|
| 1 | 0 | Unset mode still prints `researcher cassette-replay inference wired`. |
| 2 | 0 | Explicit mode prints `researcher inference seam wired (Replay { … explicit: true })`. |
| 3 | 1 | `MAOS_INFERENCE_MODE=replay requires MAOS_REPLAY_CASSETTE`. |
| 4 | 1 | Unsupported `MAOS_INFERENCE_MODE` value is named with the accepted values. |
| 5 | 1 | `MAOS_INFERENCE_MODE=live` refuses the guessed-default provider. |
| 6 | 1 | `MAOS_INFERENCE_MODE=replay conflicts with --live`. |
| 7 | 1 | `MAOS_INFERENCE_MODE=record requires MAOS_REPLAY_CASSETTE`. |
| 8 | 1 | No completion occurred; flush refuses zero successful responses and writes no cassette. The hermetic stub round-trip proves the successful record path and preserved response text. |
| 9 | 0 | Prints the cassette's `Hello! I am the MAOS hello-spirit reference implementation…`; no transport error. |
| 10 | 0 / 0 | Diff is non-empty; the explicit run emits `maos shell: MAOS_INFERENCE_MODE=replay`. |

**T9 closure validation.** `cargo test --workspace --no-fail-fast` passed 4,199 tests across 502
suites with 119 ignored. The closure sweep passed `check-empty-kernel` (0 violations),
`check-service-boundary` (0 violations), `kloc-check` (aggregate 157,559),
`check-env-contract` (88 registered, 0 violations), `check-j1-loopback-delegation`,
`check-exit-commands` (7 epics, 31 tokens, 6 owned), `check-kernel-baseline`
(24,474/24,474), and `decision_adrs_and_provisioning` (3 passed). Rust-analyzer reports no
diagnostics in the changed integration test and only pre-existing inactive-code hints in `main.rs`.
`graphify update .` rebuilt the repository graph after the code changes.

### Completion Notes List

- Implemented ADR-064's single authoritative inference-mode selector and typed refusal lattice,
  preserving unset behavior while routing replay and record through the mediated router/provider
  seam for all current consumers.
- Repaired cassette persistence: response text, provenance, explicit idempotent flush, fallible
  zero-entry shutdown, provider attribution, and stable provider-key/default/rate-limit semantics.
- Retired `--replay-llm` and `MAOS_JOURNEY_MODE`; registered the user-facing environment contract;
  corrected the nightly paid leg and harness source-recording path.
- Re-keyed ADR anti-rot checks, aligned the recovery-lane specifications, stamped and validated the
  checked-in cassette corpus, and routed the two declared cut-line outcomes.
- Closed the ten-probe runtime control. Probes 3–7 now refuse invalid configurations; probe 9
  returns the cassette text; probe 10 is observably non-identical. Probe 8's no-call form fails
  closed without writing a file, while the hermetic stub round-trip proves successful recording.
- Full workspace suite and all eight closure gates pass. No kernel-source delta; final
  `maos-bin/src` is 17,673 lines with 2,436 lines of policy headroom.

### File List

- `.github/workflows/journey-nightly.yml`
- `_bmad-output/implementation-artifacts/15-6-inference-replay-seam.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/planning-artifacts/epics/dependency-dag.md`
- `_bmad-output/planning-artifacts/epics/epic-15-foundations-w0.md`
- `_bmad-output/planning-artifacts/epics/epic-16-one-daemon-one-door-j0-w1.md`
- `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md`
- `_bmad-output/planning-artifacts/epics/epic-19-founder-loop-w4.md`
- `crates/maos-bin/src/cassette_replay.rs`
- `crates/maos-bin/src/env_contract.rs`
- `crates/maos-bin/src/inference_mode.rs` (new)
- `crates/maos-bin/src/lib.rs`
- `crates/maos-bin/src/main.rs`
- `crates/maos-bin/src/worker_spawn.rs`
- `crates/maos-bin/tests/cohort_daemon_smoke_13_5c.rs`
- `crates/maos-bin/tests/inference_mode_15_6.rs` (new)
- `crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json`
- `crates/maos-journey-test/cassettes/j-researcher/survey-distill.json`
- `crates/maos-journey-test/cassettes/j0/shell-intro.json`
- `crates/maos-journey-test/src/lib.rs`
- `crates/maos-journey-test/tests/cassette_provenance_15_6.rs` (new)
- `crates/maos-journey-test/tests/recording_source_15_6.rs` (new)
- `docs/adr/ADR-064-one-inference-mode-selector.md`
- `xtask/kloc.toml`
- `xtask/tests/decision_adrs_and_provisioning.rs`

Pre-existing working-tree changes not modified by this implementation:
`_bmad-output/implementation-artifacts/intent-lineage-coverage-report.md` and
`_bmad-output/party-mode/memories/installed/.memlog.md`.

### Change Log

- 2026-09-10 — Implemented and validated Story 15-6's mediated inference record/replay seam,
  selector contract, cassette integrity controls, nightly recording path, and specification
  alignment; moved story to review.

---

## Open questions for the operator

**None blocking.** All four were closed across the two 2026-09-09 round-table rounds (§13).

1. ~~Confirm you want the T0 spike to rule the seam~~ — **closed by D-15-6-I.** The spike rules it,
   and the refusal branch is now a ruled decision rather than a judgement left to the dev agent:
   the story ships either way, and what changes is what may be claimed (a `RELEASE-HOLDS.md` claim
   boundary, not a `deferred-work.md` row).
2. ~~Confirm the harness tempfile fix ships as EFFORT~~ — **closed.** It ships as EFFORT **with a
   hermetic control** (record through a stub, assert the file on disk changed), which was Dana's
   condition and is the reason her objection was withdrawn rather than overruled.
3. ~~D-15-6-A extends a ratified F15~~ — **dissolved.** Narrowed to a factual correction of
   ADR-064's Context; the Decision is unchanged.
4. ~~Who owns the stale cross-epic citations~~ — **answered, and inverted.** Nobody should: the line
   repairs are perishable and this story's own T2 invalidates them. ADR-064 carries the seam location
   once, machine-checked (D-15-6-G/H).

**One thing worth your eye, not a question:** D-15-6-I routes a refused spike to `RELEASE-HOLDS.md`.
That is a GA-ledger surface, and Dana's objection to using it is on the record. If you would rather a
refused spike stop the story instead, that is a one-line change to D-15-6-I — but the room's reason
for shipping stands: the unmediated path is HEAD's behaviour today, so stalling five epics does not
remove it, it only delays naming it.
