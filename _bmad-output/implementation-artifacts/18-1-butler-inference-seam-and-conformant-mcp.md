---
story_id: '18.1'
story_key: 18-1-butler-inference-seam-and-conformant-mcp
created: 2026-10-25
context_commit: c816c544
baseline_commit: c816c544
security_sensitive: true
kernel_grant: "◆ FLAG-Winston — RATIFIED 2026-10-25 by the operator (Lunarpulse) with D-18-1-10 A: **+2 physical / +1 tokei** in `crates/maos-kernel-core/src/inference/{mod,router}.rs` (prototype-measured at creation, E16-A7, reverted: pin 26361 → 26363, `maos-kernel-core` 20594 → 20595, 95 files unchanged), inside the ADR-069 envelope (≤ 23,000, alarm 22,000; `docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`). Exact figures re-measured at landing; the pin (`xtask/kernel-core-baseline.toml:505`) and ceiling (`xtask/kloc.toml:264`) move in one commit with a dated HISTORY note citing this key. `18-8-conformant-mcp-client` moves the pin too (`io/mod.rs`); whichever lands second re-measures."
kloc_grant: "Measured at landing after `cargo fmt --all` (RECOVERY-LANE CEILING RULE, `xtask/kloc.toml:60-70,86-95`); forecasts re-measured after the 2026-10-25 split, [INFERENCE] unless marked: maos-bin +60–120 (27809, zero headroom); maos-kernel-core +1 tokei (measured); maos-domain 0 (D-18-1-10 A touches no domain type); maos-mcp, maos-journey-test 0 (the MCP half and its fixture moved to 18-8; this story's journey-test edits are tests, uncharged); `spirits/butler`, `spirits/digest` ungoverned (`kloc.toml:2`; `xtask/src/kloc_check.rs:310-311`). `_aggregate_hardfail` = measured + 1 (181565 / 181566 at c816c544). Test files uncharged; in-`src` `#[cfg(test)]` code IS charged (tokei counts it)."
review: "bmad-code-review under `docs/testing/proof-contract.md` (focused suites + enrolled gates; workspace once in CI). Reviewer model `anthropic/claude-opus-5-5` ONLY — operator ruling 2026-10-25: fable-5.1 declined for the security layer too. AC7 (capability authorization of every provider dispatch) and AC1's issuance scope are security-critical; they are reviewed on opus-5-5 with the Security layer named in the review record. ◆ FLAG-Winston: Winston signs off the kernel diff before landing."
---

# Story 18.1: Butler inference seam

Status: ready-for-dev

<!-- Created 2026-10-25 at c816c544 by bmad-create-story under the lighter creation contract (no party-mode preflight). RULED 2026-10-25 by the operator (Lunarpulse): every D-18-1 row below carries its ruling; D-18-1-01 B split the MCP half (old AC2, AC5; D-18-1-06/07/12) to `18-8-conformant-mcp-client`. The story key keeps its name (tracker keys are not renamed); the title drops "conformant MCP". Creation ran cargo as the host's only cargo process (one target dir, /mnt/build/cargo); every kernel prototype was measured and reverted (figures in §Kernel pricing). -->

> **The capability:** *Butler thinks through the Inference Port — its proposal comes from a recorded (or live) completion, a malformed one is a red exit, and every provider it can reach is one its capability names; the morning digests the running Spirits produce meet FR17's floors.*

**Closes:** S3 (Butler third; epic `:9`) · FR17 on the production path for both digest-shipping classes, Butler and the Digest class (`prd/functional-requirements.md:51`; D-18-1-09 B) with the routing-audit widening (`routing-audit-2026-10-22.md:35`: first-session ≤ 30 s, every field, 100-digest floors) · FR58 Butler zero-config response (`functional-requirements.md:92`) · `deferred-work.md:1213` (router fallback/empty id, AC7) — the only `deferred-work.md` row whose Owner is this key (grep at c816c544). Epic contract: `epic-18-spirits-that-think-w3.md` §18-1 (`:121-136`, corrected and ruled in place) and §R (`:44-81`).

**Does NOT close, and says so:**
- The conformant MCP client and the stdio early-close witness (old AC2, AC5) — **split to `18-8-conformant-mcp-client`** (D-18-1-01 B; epic §18-8). This story's AC numbering keeps AC2 and AC5 as "moved" so cites to AC1/3/4/6/7 stay stable.
- Exit lines 2–4 (18-2, 18-3, 18-4). The `epic-18-exit` job this story creates (D-18-1-11 A) runs line 1 and its twin only.
- `deferred-work.md:1212` — the Researcher's keyless explicit replay stays 18-3's (AC5). D-18-1-02 A gives 18-3 the mechanism; it does not switch the Researcher arm.
- Live completions (`ops-nightly-cassette-rerecord`, epic `:29`) — operator lane, never gating.
- The `Clock` port (18-4); `now_ns` is `SystemTime` until it lands (epic AC3).
- No entitlement work (R18-P4; 17-11 enforces it at issuance, `crates/maos-kernel-core/src/capability/cap_policy/mod.rs:159-166`). AC4 is met by 17-5 (R18-P3).

## Story

As **the director who leaves Butler running**,
I want **Butler's anticipatory proposal to come from the model through the capability-mediated Inference Port — replayable in CI from a cassette, refused loudly when the completion is malformed or a provider is not authorized — and the morning digests the running Spirits produce to meet FR17's floors**,
so that **v0.3's "Butler thinks" is a product path, not a seed scenario: the exit line replays a recorded completion, and nothing the token does not name can execute.**

**Contract:** epic-18 §18-1 (AC1, AC3, AC4, AC6, AC7; `_bmad-output/planning-artifacts/epics/epic-18-spirits-that-think-w3.md:121-136`, corrected and ruled in place) and §R (`:44-81`: R18-1…R18-7, R18-9, R18-10, R18-22, R18-23, R18-P3, R18-P4); exit line 1 and its provenance (`:14`, `:22`); confidence rules 9–11 (`:38-40`); `docs/testing/proof-contract.md` (binding).

## Current-tree corrections — read before implementing

Measured at `c816c544` by two read-only scouts (Butler/inference seam; MCP/kloc), direct reads of every cited range, throwaway kernel prototypes (§Kernel pricing, all reverted), and — for the rows now owned by 18-8 — the MCP specification. Rows about the old AC2/AC5 moved with the split into `18-8-conformant-mcp-client.md`; they are not repeated here.

| Premise (epic / §R / deferred row) | Fact at `c816c544` and consequence |
|---|---|
| AC1 cites `cassette_replay.rs:170-173`, `researcher/src/lib.rs:527-534`, `butler/src/lib.rs:337-370`, `:636-645` | `complete` is `crates/maos-bin/src/cassette_replay.rs:171-174` (impl `:170`); `with_deferred_inference_port` `spirits/researcher/src/lib.rs:528-535`; `pub struct Butler` `spirits/butler/src/lib.rs:337-369`; `let user_preference_drift = scenario.preference_alignment…` `:638-647`. Every other AC1 cite holds (`main.rs:3614-3616`, `:3625-3667`, `:5083-5285`, `:647-716`, `:665-684`, `:5478-5534`, `:5535-5574`, `:5729-5747`, `:5876-5888`/`:5885`, `:5850`, `:5862-5866`; `manifest.toml:20-21,57-60`; `cap_policy/mod.rs:159-166`). Corrected in place. |
| AC1: the test mints Butler's declared `anthropic` scope; exit line 1 replays keyless in CI (epic `:14`, rule 2) | **The keyless explicit-replay router has no `anthropic` key.** Providers register only when configured (`main.rs:3533-3569`); Ollama registers lazily at the default URL and becomes the default (`:3571-3589`); only an empty map (e.g. `MAOS_OLLAMA_URL=skip`) gets the `anthropic` placeholder (`:3591-3595`). Explicit replay replaces each *existing* driver (`:3644-3646`, "the router's public key set and default remain unchanged", `:3625`). Butler declares only `anthropic` (`spirits/butler/manifest.toml:21`). So a router-default mint (the Researcher explicit arm's pattern, `main.rs:5510-5514`) would be refused at issuance (17-11), and an `anthropic` token is refused at use because the adapter checks the token against `req.provider_id.or(default)` (`crates/maos-kernel-core/src/inference/mod.rs:256-262`). Same defect as `deferred-work.md:1212` (18-3's), now on 18-1's exit line. → D-18-1-02 (RULED A). |
| AC1: an `output_shape` violation exits non-zero; proven red = a completion lacking a required field | Butler's `NotificationPayload` is `pattern/confidence/evidence/conflict_summary` (`spirits/butler/src/lib.rs:91-100`) — **no `options`**, while the manifest requires `["pattern","confidence","evidence","options"]` (`manifest.toml:60`). Every conflict pass (the daemon seed has one: `main.rs:665-684`, events 540–600 and 570–630) already violates the shape and only `eprintln!`s (`main.rs:5885`). `crates/maos-journey-test/tests/journey_butler.rs:168-202` (JB-5) **pins exit 0** on a violation (`:186-189`). → D-18-1-03 (RULED A: `options` on both paths, JB-5 flips). |
| Exit line 1 asserts the rendered proposal equals the cassette's `response.text` (`on-idle-halt.json:13`, epic `:22`) | Text is at `:14` (wildcard hash `:11`, seed `model_id` `:5`, `provenance: seed` `:3`); it is prose with no `confidence`/`evidence` values. The cassette is re-authored to the D-18-1-03 JSON contract (seed provenance kept). Four `maos-bin` daemon tests export it under `MAOS_INFERENCE_MODE=replay` (`maosctl_load_16_6.rs:43,124-125,595-596`; `one_daemon_one_door_16_1.rs:40,127-128,881-882`; `root_shutdown_unload_16_3.rs:71,834-835,843-844`; `wasm_form_admission_17_3b.rs:12,161-162,215-216`) — once Butler infers, those daemons consume it (one entry; a second pass exhausts it). → D-18-1-04 (RULED A), focused set. |
| Daemon Butler can call inference for the life of the process | The binding token is minted once at admission with `ttl_secs = 60` (`main.rs:5733-5746`; TTL enforced `crates/maos-capability/src/cap_tokens/mod.rs:235`). A daemon's `on_idle` (`main.rs:4723`) after 60 s would be `CapabilityDenied`. `--once` is unaffected (mint `:5733` → fire `:5832`). → D-18-1-05 (RULED A: mint per pass). |
| AC6: "for each digest-shipping class"; 100-digest floors; FR58 | **Butler:** the floors are proven for `Butler::morning_digest` at library level against a seeded TL (`spirits/butler/tests/hallucination.rs:187-254`, negative controls `:256-288`); `morning_digest` has no `maos-bin` caller (`crates/maos-bench/src/harness/j0.rs:83`, `journey_butler.rs:151`, `spirits/butler/tests/{hallucination.rs:152,digest.rs:122}`). **Digest class** (ruled in by D-18-1-09 B): the J3 team digest is `derive_team_digest` (`spirits/digest/src/lib.rs:219`; `TeamDigest` `:184-194`: counts, `narrative`, `clause_sources` I11 map), rendered in production only under `--once` by `render_j3_digest_scene` (`crates/maos-bin/src/main.rs:445-504`, called `:5850-5875`), which ingests each summary/receipt into the TL and stamps its `source_log_ref` from the new frame (`:460-485`) and makes an `output_shape` violation fatal (`:5862-5866`; fields `spirits/digest/manifest.toml:18`, `time_cap_seconds = 10` `:22`). `spirits/digest` has **no `tests/`** and no 100-digest corpus; its unit tests are in `src/lib.rs:475-666`; the one daemon-level witness is `crates/maos-journey-test/tests/journey_j3.rs:18-77` (`j3_day_30_digest_renders_and_persists_via_real_daemon`). The team digest has no FR17 (d) trust-bar and no anomaly-confidence field. FR58 v0.3 = Butler within the J0 evaluator budget (NFR-Onb-2 ≤ 5 min, `prd/non-functional-requirements.md:126`). → D-18-1-08, D-18-1-09 (RULED), CR-8, CR-9. |
| AC7: `deferred-work.md:1203`; kernel-Δ 0 measured at creation [INFERENCE] | The row is `deferred-work.md:1213` (corrected in place in §R R18-23 and AC7). **Kernel code is required**: only `InferencePortAdapter::complete` (`inference/mod.rs:256-262,294-303`) and `MultiProviderRouter` (`router.rs:32-35,46-74`) see both the token and the fallback ids. The existing unit test `fallback_503_routes_to_secondary` (`inference/mod.rs:588-664`) **asserts the defect** (a `primary`-scoped token is answered by `secondary`). No production request sets fallbacks (`spirits/researcher/src/lib.rs:939`, `crates/maos-spirit-hello/src/lib.rs:108-118` pass empty lists; `dispatch_with_fallback`'s only non-test caller is the router-only smoke arm `main.rs:9576`). The manifest `[providers] fallback` section is parsed (`crates/maos-manifest/src/manifest.rs:2363,2465-2470`) and feeds `ProviderSwitched` journaling (`crates/maos-kernel-core/src/security/mod.rs:261`), never a request. → D-18-1-10 (RULED A), §Kernel pricing. |
| AC7: "an empty id maps to the default" | The adapter takes `Some("")` as the provider id and checks the token against `""` (`mod.rs:256-262`); the router then maps `""` to the default (`router.rs:33-35`). Reachable only with a `ProviderInfer { provider: "" }` token, which 17-11 admission no longer mints — latent, fixed with AC7 (CR-6). |
| Epic stories table / Kernel-Δ paragraph: 18-1 kernel-Δ 0 (`:32`, `:90`, `:122`) | Corrected in place to the measured, ratified figure (◆ FLAG-Winston). |
| Exit-line-1 provenance (`:22`): `env_contract.rs:265`/`:255`, `worker_spawn.rs:44-72`/`--once :55`, cassette `:4/:10/:13`, "zero consumers" | `MAOS_REPLAY_STRICT` `crates/maos-bin/src/env_contract.rs:280`, `MAOS_REPLAY_CASSETTE` `:275`, `MAOS_INFERENCE_MODE` `:270`; `parse_run_args` `crates/maos-bin/src/worker_spawn.rs:67-101`, `--once` `:78`; cassette `:3/:5/:11/:14`; four daemon tests export it (row above). Corrected in place. |
| R18-21 / Dependencies: 18-7 tracker row `sprint-status.yaml:282` | `:283` (`:282` is 18-6). Corrected in place (§R and Dependencies). |
| Holds | R18-1…R18-7, R18-9, R18-10 (`sprint-status.yaml:266` 17-5 done, `:273` 17-11 done), R18-22 (FR17 `:51`, FR58 `:92`, routing audit `:35`), R18-P3 (`manifest.toml:21` = `anthropic.claude-haiku-4-5-20251001`; Researcher `:49`), R18-P4; floors `spirits/butler/tests/corpus_halt.rs:298,303`; `proof-contract.md:26`. |

## Kernel pricing (E16-A7) — AC7

Method: each option was written in place at `c816c544`, `cargo fmt`'d, measured with `cargo run -p xtask -- check-kernel-baseline` / `find crates/maos-kernel-core/src -name '*.rs' | xargs cat | wc -l` (the pin's physical count) and `cargo run -p xtask -- kloc-check --json` (tokei 14.0.0, the production ceiling's count), compiled (`cargo test -p maos-kernel-core --lib inference`), then restored by copy-back; `git status` showed no crate file changed after each revert (`check-kernel-baseline`: 26361 / 95 files at HEAD).

| Prototype | Physical (pin 26361) | Tokei `maos-kernel-core` (20594) | Other crates | Compile / tests |
|---|---|---|---|---|
| **AC7 A (RULED)** — adapter: `.filter(\|id\| !id.is_empty())` on the resolved id; after `check_capability`, `if req.fallback_provider_ids.iter().any(\|id\| id != provider_id) { return Err(CapabilityDenied) }`; router: `dispatch` stops mapping `""` to the default; the existing unit test inverted (`unwrap_err` → `CapabilityDenied`) | **26363 (+2)** | **20595 (+1)** | 0 | `--lib inference` 14/14 green. **Plant** (remove the 3-line check): `inference::tests::fallback_503_routes_to_secondary` FAILED at `mod.rs:663`, 13/14, 1.22 s incremental. |
| AC7 B — per-hop tokens (`InferenceRequest.fallback_capability_tokens`, maos-domain) | 26381 (+20) | 20613 (+19) | maos-domain +6 | 14/14 green |
| AC7 C′ — delete `dispatch_with_fallback` + `is_retriable` + their 6 router tests and the adapter fallback test; adapter refuses a non-empty list | 26092 (−269 measured; −271 computed once the two now-unused imports go) | 20348 (−246 measured; −248 computed) | full C also drops the domain field and 19 call sites | 7/7 green |

Ruled figure: **+2 physical / +1 tokei** (production +2 tokei: +4 adapter, −2 router; the inverted in-`src` test −1). The AC2 IO prototype moved to `18-8-conformant-mcp-client.md` §Kernel pricing.

## Decisions — RULED 2026-10-25 by the operator (Lunarpulse)

**Ruling record.** D-18-1-01 **B** (split), 02 **A**, 03 **A**, 04 **A**, 05 **A**, 08 **A**, 09 **B** (the operator deviated from the recommendation: AC6 covers Butler AND the Digest class), 10 **A** (◆ FLAG-Winston grant +2 physical / +1 tokei ratified, ADR-069 envelope), 11 **A**. D-18-1-06/07/12 moved with the split and were ruled **B / B / B** for 18-8 (recorded there). D-18-1-13 is the standing measured-at-landing contract (2026-10-23), not an option. Review model: `anthropic/claude-opus-5-5` only. CR-n are creation defaults carried into the rulings; the operator may still overrule one.

| ID | Question | Options | Ruling and rationale | Status |
|---|---|---|---|---|
| **D-18-1-01** | One story, or split the MCP half off the critical path? | A — one story · **B — split AC2 + AC5 (+ D-06/07/12) into `18-8-conformant-mcp-client`** | **B.** The epic's 18-1 edges (`→ 18-5`, `→ 18-7`) ride on the inference seam only; the MCP half carries its own ruling and kernel figure. Recorded in the epic as a SPLIT against PRD amendment (b) (`prd/project-scoping-phased-development.md:406`), not new scope. | RULED 2026-10-25 — B |
| **D-18-1-02** | How does keyless explicit replay reach a provider Butler declares? | **A — the shared wrapping (`main.rs:3629-3647`) also registers the replay driver under every provider id the kernel interns (`anthropic`/`openai`/`ollama`, `inference/mod.rs:95-99`), default unchanged; Butler's token names its first declared `ProviderInfer` (17-11 rule, `main.rs:5550-5566`) and the request carries that id** · B — Butler declares the router pins · C — bind the cassette directly · D — CI sets `MAOS_OLLAMA_URL=skip` | **A.** Replay contacts no provider, so the extra keys widen nothing a token does not already gate; token and request name one declared provider — exactly what AC7 then enforces. | RULED 2026-10-25 — A |
| **D-18-1-03** | Completion contract, parse locus, `options` | **A — one JSON object `{"pattern","confidence","evidence","options":[…],"user_preference_drift"}` parsed by Butler with `serde_json` into a typed proposal; a missing/ill-typed field is a typed `ButlerError`; the deterministic path also emits `options` (the three picks of `handle_option_pick`, `lib.rs:561-597`); JB-5 flips to non-zero** · B — prose markers · C — fatal only when bound | **A.** The shape gate is a control (rule 11); a parse that can default is not one; the shipped Butler stops violating its own manifest. | RULED 2026-10-25 — A |
| **D-18-1-04** | What an inference/parse/shape failure does | **A — `--once`: typed non-zero exit; daemon: that `on_idle` pass is refused typed (stderr, no notification, no scalar write) and the daemon keeps serving; an unbound port keeps the deterministic path** · B — daemon exits · C — arithmetic fallback | **A.** No silent fallback (18-3 removes the Researcher's), and one exhausted cassette or provider 5xx does not kill a daemon. | RULED 2026-10-25 — A |
| **D-18-1-05** | Token lifetime for a daemon Butler | **A — mint per `on_idle` pass via a maos-bin minting closure (`issue_enterprise_governed_capability`, `crates/maos-bin/src/worker_spawn.rs:337`), 60 s TTL kept** · B — one 60 s mint, labelled · C — longer TTL | **A.** The only option under which the daemon path works at all; every mint stays audited, policy-checked and charged to the 17-9 window (`crates/maos-kernel-core/src/capability/mod.rs:206-229`) [INFERENCE: per-pass quota headroom not measured]. | RULED 2026-10-25 — A |
| D-18-1-06 | Which MCP revision is "conformant" | — | Moved to `18-8-conformant-mcp-client` with the split; ruled **B (dual-era)** there. | MOVED → 18-8 (RULED B) |
| D-18-1-07 | Where the MCP bearer comes from | — | Moved to 18-8; ruled **B (SecretStore)** there. | MOVED → 18-8 (RULED B) |
| **D-18-1-08** | What triggers the production digest (FR17 "within 30 s of the user's first session of the day") | **A — the Butler arm renders `morning_digest` at process start (the first session) and on the first `on_idle` after a local-day rollover; `--once` renders it after the pass; `{"event":"morning_digest", …}` on stdout plus a TL frame** · B — new verb · C — daily schedule | **A.** "First session" is a session event, not a cron tick; a verb is 19-4's render territory (D-E). | RULED 2026-10-25 — A |
| **D-18-1-09** | AC6 scope and proof route | A — Butler only · **B — Butler AND the Digest class** · C — 100 production runs | **B (operator; deviates from the creation recommendation A).** FR17's floors bind every digest-shipping class; the Digest class ships the J3 team digest. Chosen proof shape (minimal for the ruled scope): **one production witness per class**, plus **one new 100-digest library corpus for the Digest class** — the floor is a 100-digest-corpus property and none exists for it (Butler's exists, `hallucination.rs:187-254`). Field set per CR-8. | RULED 2026-10-25 — B |
| **D-18-1-10** | AC7: how fallback dispatch is authorized | **A — fail closed: a fallback id the token does not name is refused before any dispatch (`CapabilityDenied`); an empty resolved id is refused (`Unconfigured`); the router stops mapping `""` to the default** · B — per-hop tokens · C — delete the chain | **A.** Measured +2 physical / +1 tokei with a proven plant; no production request sets fallbacks. A future request-level fallback consumer owes B (CR-7). ◆ FLAG-Winston grant ratified. | RULED 2026-10-25 — A |
| **D-18-1-11** | Does 18-1 create the hermetic exit job? | **A — `epic-18-exit` (one image `ubuntu-24.04`), exit line 1 verbatim + planted-violation twin, `aggregate.needs` only (`epic-17-exit` precedent, `discipline.yml:1942-1949`, needs `:5593`)** · B — wait for 18-4 | **A.** The epic calls the exit command "the epic's first CI job, red until green" (`:11`); rule 11(b). | RULED 2026-10-25 — A |
| D-18-1-12 | The stdio hang beside AC5 | — | Moved to 18-8; ruled **B (bounded read, typed `Timeout`)** there. | MOVED → 18-8 (RULED B) |
| **D-18-1-13** | kloc / kernel governance | Exact measured requests at landing (after `cargo fmt --all`, `kloc-check --json`), returned before the landing commit, pinned with dated notes citing this key; `_aggregate_hardfail` = measured + 1; kernel pin + ceiling in one commit under FLAG-Winston with a HISTORY entry (`xtask/kernel-core-baseline.toml:506-512` template). | Standing contract 2026-10-23 (non-kernel crates measured at landing); E16-A7. | STANDING (not an option) |
| CR-1 | AC1 test home | New `crates/maos-bin/tests/butler_inference_18_1.rs`: process-level `maos run spirits/butler/manifest.toml --once`, fresh `HOME`/`MAOS_HOME`/`XDG_DATA_HOME`, no provider key, `MAOS_OLLAMA_URL` unset (the CI shape), `MAOS_INFERENCE_MODE=replay`, `MAOS_REPLAY_STRICT=1`; cases: good cassette (exit 0; notification fields equal the cassette's), missing `options` (non-zero, names the field), empty `entries` (exhaustion → non-zero), drifted prompt hash (non-zero), undeclared-provider mint refused typed. Parser unit tests in `spirits/butler/tests/`. | — |
| CR-5 | AC7 witness placement | Invert the in-`src` test `fallback_503_routes_to_secondary` (`inference/mod.rs:588-664`; rename `fallback_not_named_by_the_token_is_refused_before_dispatch`, assert the `secondary` driver is never called) and add the empty-id case in `crates/maos-kernel-core/tests/inference_fallback_authz_18_1.rs` (uncharged; public API as `benches/hello_spirit_p95.rs:89-141`). | — |
| CR-6 | Empty-id typing | An empty resolved provider id is `InferenceError::Unconfigured` (what an absent default returns, `mod.rs:260`); `router.dispatch(Some(""))` returns `RouterError::UnknownProvider("")`. | — |
| CR-7 | Deferred-work bookkeeping | `deferred-work.md:1213` gets `FIXED in tree by 18-1 (dev <date>); CLOSED on the landing run` and "a request-level fallback consumer owes per-hop authorization (D-18-1-10 B)". Rows are never deleted. | — |
| CR-8 | Digest-class field set (D-18-1-09 B) | FR17's (a)–(d) list is the *per-Spirit morning digest*'s and is asserted for Butler (`MorningDigest`, `spirits/butler/src/lib.rs:287-296`). The Digest class asserts its manifest-declared team-digest field set (`spirits/digest/manifest.toml:18`), already fatal in production (`main.rs:5862-5866`); the team digest has no trust-bar (FR17 (d)) and no anomaly-confidence field (FR17 (c)), and creation does not add them (that would be a `TeamDigest` + manifest schema change, the manifest focused-set widening of `proof-contract.md:45-51`, and new scope). The operator may overrule this default. | — |
| CR-9 | Digest-class proof homes | Library corpus: NEW `spirits/digest/tests/hallucination_18_1.rs` (the crate's first `tests/`), mirroring `spirits/butler/tests/hallucination.rs`: 100 seeded `RawDigestInputs` windows whose refs are real frames of a seeded TL; asserts 0/100 digests with an unresolved `clause_sources` ref and ≥ 95/100 whose `agents_halted` and halt-receipt refs equal the TL's receipt truth set; negative controls: a fabricated ref and a dropped receipt each red the checker. Production witness: extend `crates/maos-journey-test/tests/journey_j3.rs:18-77` (it already drives the real daemon) with wall-clock ≤ 30 s from spawn to the `team_digest` event and resolution of every `clause_sources` ref in the daemon's TL. | — |

## Acceptance Criteria

*(Epic §18-1 as ruled; every AC cites the code it changes. AC2 and AC5 moved to `18-8-conformant-mcp-client`; their numbers are kept so cites stay stable. One plant per control AC, `docs/testing/proof-contract.md:26`, except where named.)*

### AC1 — command: exit line 1 replays Butler's proposal through the Inference Port

1. `MAOS_INFERENCE_MODE=replay MAOS_REPLAY_STRICT=1 MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json maos run spirits/butler/manifest.toml --once` on a keyless runner exits 0; Butler's `on_idle` (`spirits/butler/src/lib.rs:390`) calls `InferencePort::complete` once through a NEW `Butler::with_deferred_inference_port` mirroring Researcher's (`spirits/researcher/src/lib.rs:528-535`), and the emitted notification's `pattern`/`confidence`/`evidence`/`options` equal the cassette completion's (D-18-1-03 A); `belief_variance` stays arithmetic (`assess()` `:623-667`) and `user_preference_drift` comes from the completion; the `belief_variance` halt fires as today (jb3).
2. Wiring (D-18-1-02 A): the Butler arm (`crates/maos-bin/src/main.rs:5083-5285`, constructor `:647-716`) binds an `InferencePortAdapter` over the shared router for `--live` and every explicit mode, reusing 15-6's wrapping (`:3625-3667`, extended to register the replay driver under the interned ids) — never a second copy; the binding backfill (`:5729-5747`) is generalized from Researcher-only to "the loaded Spirit's binding", minting the first declared `ProviderInfer` (`:5550-5566` rule), and the request names that provider. Butler's binding mints per pass (D-18-1-05 A); the Researcher's minted provider is unchanged (18-3 owns it).
3. Reds (D-18-1-04 A): with `--once`, a completion missing a required field, cassette exhaustion, strict drift, or any `InferenceError` exit non-zero with a typed message; the Butler `output_shape` check (`main.rs:5876-5888`) becomes fatal under `--once` (log-only today, `:5885`); JB-5 (`crates/maos-journey-test/tests/journey_butler.rs:168-202`) asserts the non-zero exit. The deterministic path (mode unset, no `--live`) emits `options` and stays green. In a daemon a failed pass is refused typed and the daemon keeps serving.
4. Capability: admission mints Butler's declared `anthropic` scope (`spirits/butler/manifest.toml:21`); an undeclared provider is refused typed at issuance (17-11, `cap_policy/mod.rs:159-166`). `on-idle-halt.json` is re-authored to the D-18-1-03 contract, `provenance: seed` kept.
5. CI (D-18-1-11 A): `epic-18-exit` runs line 1 verbatim and a twin with a planted-violation cassette that must exit non-zero.
6. **Proven red (1 plant):** restore the log-only shape check (`eprintln!` and continue) → CR-1's missing-`options` case exits 0 → red.

### AC2 — moved to `18-8-conformant-mcp-client` AC1 (D-18-1-01 B)

### AC3 — FR17 morning digest on the production path (D-18-1-08 A)

1. The Butler arm calls `Butler::morning_digest` (`spirits/butler/src/lib.rs:675-682`) — its first `maos-bin` caller — at process start and on the first `on_idle` after a local-day rollover; `--once` renders it after the pass; output `{"event":"morning_digest", …}` with the four FR17 fields (`MorningDigest`, `lib.rs:287-296`); `audit_db`/`journal_path` are the daemon's; `now_ns` is `SystemTime` (18-4's `Clock` replaces it).
2. **Proven red (1 plant):** remove the call → `crates/maos-bin/tests/butler_digest_18_1.rs` reds (no `morning_digest` event).

### AC4 — retired model pin (met by 17-5)

`spirits/butler/manifest.toml:21` = `anthropic.claude-haiku-4-5-20251001`, Researcher `spirits/researcher/manifest.toml:49`; `check-model-currency` (`discipline.yml:541`) green on every pin 18-1 adds (none under D-18-1-02 A). Existing gate → no plant.

### AC5 — moved to `18-8-conformant-mcp-client` AC3 (D-18-1-01 B)

### AC6 — FR17 floors for every digest-shipping class, and FR58 (routing audit; D-18-1-09 B)

1. **Butler.** The library floors stay green and are the floors of the production function: `spirits/butler/tests/hallucination.rs:187-254` (0/100 hallucinated, ≥ 95/100 open-halt inclusion) with its negative controls (`:256-288`). Production witness (`butler_digest_18_1.rs`): against a seeded TL, `maos run spirits/butler/manifest.toml --once` emits the digest **within 30 s of process start** (wall-clock from spawn; FR17 `functional-requirements.md:51`), every `source_log_ref` resolves to a real TL frame, the open-halt set equals the TL's truth set for the window, all four FR17 fields present.
2. **Digest class** (CR-8, CR-9). Library corpus: NEW `spirits/digest/tests/hallucination_18_1.rs` — over 100 seeded windows, 0/100 digests carry a `clause_sources` ref that does not resolve to a real TL frame, and ≥ 95/100 report every receipt-backed halt (`agents_halted` and its refs equal the TL truth set); two negative controls (fabricated ref; dropped receipt) red the checker. Production witness: `journey_j3.rs:18-77` extended — `maos run spirits/digest/manifest.toml --once` emits `team_digest` within 30 s of spawn, every `clause_sources` ref resolves in the daemon's TL, and the manifest field set (`spirits/digest/manifest.toml:18`) is present (already fatal, `main.rs:5862-5866`).
3. **FR58** (`functional-requirements.md:92`; NFR-Onb-2 `non-functional-requirements.md:126`): on a fresh home with no key and no inference env, `maos init` then `maos run spirits/butler/manifest.toml --once` exit 0 with a shape-valid Butler notification and digest, well inside 5 minutes.
4. **Proven red (2 plants, one per class — named under `proof-contract.md:26`'s exception: each class's witness is otherwise unfalsified):** (a) Butler — the production caller passes a truncated window (`now_ns − 1 h`) → the open-halt equality reds; (b) Digest — `render_j3_digest_scene` keeps the fixture's `source_log_ref` instead of the ingested frame id (`main.rs:471,484`) → the production witness's ref resolution reds.

### AC7 — every provider dispatch is capability-authorized (D-18-1-10 A; ◆ FLAG-Winston, grant ratified)

1. `InferencePortAdapter::complete` (`crates/maos-kernel-core/src/inference/mod.rs:256-262`) refuses an empty resolved provider id (`Unconfigured`, CR-6) and, after `check_capability`, refuses with `CapabilityDenied` **before any dispatch** a request whose `fallback_provider_ids` names a provider the token does not; `MultiProviderRouter::dispatch` (`router.rs:32-35`) no longer maps `""` to the default.
2. Witness (CR-5): the inverted `fallback_503_routes_to_secondary` (no driver is called), plus the empty-id case in `crates/maos-kernel-core/tests/inference_fallback_authz_18_1.rs`. Router-level fallback tests (`router.rs:236-354`, `tests/multi_provider_routing.rs:82-105`) stay green — the walker is unchanged.
3. Kernel: ratified +2 physical / +1 tokei; re-measured at landing; `check-kernel-baseline` and `kloc-check` re-pinned in one commit under FLAG-Winston.
4. **Proven red (1 plant):** remove the fallback check → the inverted unit test reds (measured at creation: FAILED at `mod.rs:663`, 13/14, 1.22 s).

## Tasks / Subtasks

- [ ] **T0 — Preflight at the dev HEAD** (all ACs)
  - [ ] Confirm §Decisions reads RULED (done 2026-10-25) and story + tracker read `ready-for-dev`; record Winston's FLAG sign-off on the AC7 kernel diff before landing.
  - [ ] `git diff --stat c816c544 -- crates/maos-kernel-core/src/inference crates/maos-bin/src/main.rs crates/maos-bin/src/cassette_replay.rs spirits/butler spirits/digest spirits/researcher/src/lib.rs crates/maos-journey-test .github/workflows/discipline.yml`; any drift → Debug Log, re-anchor. If 18-8 landed first, re-measure the kernel pin from its new value.
- [ ] **T1 — Butler seam** (AC1.1, AC1.4; D-18-1-03) — `spirits/butler/src/lib.rs`: inference field + `with_deferred_inference_port`; prompt built from the scenario (calendar, comms); typed proposal parse; `NotificationPayload.options`; `user_preference_drift` from the completion; unbound path unchanged except `options`. Re-author `on-idle-halt.json`.
- [ ] **T2 — Daemon wiring** (AC1.2; D-18-1-02/05) — `main.rs:3629-3647` replay registration; Butler arm binding for `--live`/explicit modes (extract a helper from `:5478-5534` rather than copying); backfill `:5729-5747` generalized; per-pass mint closure for Butler.
- [ ] **T3 — Reds** (AC1.3; D-18-1-04) — `--once` typed exits; `:5876-5888` fatal under `--once`; daemon pass refusal; JB-5 flip.
- [ ] **T4 — Butler digest caller** (AC3, AC6.1, AC6.3; D-18-1-08) — production call + event; `butler_digest_18_1.rs` (30 s, refs, open halts, fields; FR58 fresh-home case).
- [ ] **T5 — Digest class floors** (AC6.2; D-18-1-09 B, CR-8/9) — `spirits/digest/tests/hallucination_18_1.rs` (100-window corpus + two negative controls); `journey_j3.rs:18-77` extended (30 s, TL ref resolution).
- [ ] **T6 — Kernel** (AC7; D-18-1-10, CR-5/6) — `inference/mod.rs:256-262`, `router.rs:33-35`; inverted unit test; `tests/inference_fallback_authz_18_1.rs`; `cargo test -p maos-kernel-core --lib`.
- [ ] **T7 — Exit job** (AC1.5; D-18-1-11) — `epic-18-exit` in `.github/workflows/discipline.yml` (one image, line 1 verbatim + twin), `aggregate.needs`.
- [ ] **T8 — Plants and records** — plants for AC1, AC3, AC6 (a)+(b), AC7, recorded once in `18-1-evidence/plants.md` (command, output, wall-clock); `deferred-work.md` per CR-7; epic §18-1 if anything moved at dev HEAD.
- [ ] **T9 — Governance** (D-18-1-13) — `cargo fmt --all`; `kloc-check --json`; `check-kernel-baseline` (re-pin under FLAG-Winston); `check-model-currency`; `check-test-cost`; `check-dev-record-completeness`; measured requests to the operator **before** landing; `cargo nextest run -p xtask --locked --test d11_xtask_ceiling_ratchet --test recovery_lane_ceiling_rule` before the closing commit (`proof-contract.md:40-44`).

## Dev Notes

### What changes, by file

| Area | Files | Today → after |
|---|---|---|
| Butler | `spirits/butler/src/lib.rs` (struct `:337-369`, builders `:508-542`, `on_idle` `:390-486`, `NotificationPayload` `:91-100`, `assess` `:623-666`), `spirits/butler/tests/*`, `crates/maos-journey-test/cassettes/j-butler/on-idle-halt.json` | arithmetic seed → model proposal (bound) / `options` added (unbound) |
| Daemon | `crates/maos-bin/src/main.rs` (`:3629-3647`, `:5083-5285`, `:5478-5534` helper, `:5729-5747`, `:5801-5889`) | Butler never infers → seam bound under the 15-6 selector; shape check fatal under `--once`; digest event |
| Digest class | `spirits/digest/tests/hallucination_18_1.rs` (new; the crate's first `tests/`), `crates/maos-journey-test/tests/journey_j3.rs:18-77` | no 100-digest corpus, no 30 s/ref assertions → both |
| Kernel ◆ | `crates/maos-kernel-core/src/inference/mod.rs:256-262,588-664`, `router.rs:33-35`, new `crates/maos-kernel-core/tests/inference_fallback_authz_18_1.rs`; `xtask/kernel-core-baseline.toml:505`, `xtask/kloc.toml:264` | unchecked fallback → refused before dispatch |
| CI | `.github/workflows/discipline.yml` (new `epic-18-exit`; `aggregate.needs` near `:5593`) | no epic-18 job → line 1 + twin |
| Tests | `crates/maos-bin/tests/{butler_inference_18_1.rs,butler_digest_18_1.rs}` (new), `crates/maos-journey-test/tests/journey_butler.rs:168-202` | — |
| Records | `deferred-work.md:1213`, `18-1-evidence/`, `xtask/kloc.toml` | — |

### Current state that must be preserved

- **Unset mode = deterministic** (ADR-064, `docs/adr/ADR-064-one-inference-mode-selector.md`; `main.rs:5575-5579` for the Researcher): Butler with no `--live` and no `MAOS_INFERENCE_MODE` never touches the router; `corpus_halt.rs`, `on_idle_halt_8_10.rs`, `spirit_smoke.rs`, `journey_butler.rs` jb1–jb4 and `crates/maos-bench/src/harness/j0.rs` keep passing (only `options` is added).
- **`--live` MCP wiring** (`main.rs:5121-5270`) is untouched here (18-8 changes the transport it builds); the Researcher's three arms (`:5478-5579`) keep their behaviour — the backfill generalization leaves the Researcher's minted provider exactly as today.
- **Boot-loud epistemic port** (`requires_epistemic_halt_port`, `main.rs:512-527`; `user_preference_drift` `:524`): Butler still requires the scalar port.
- **Daemon idle loop** (`main.rs:4697-4777`, `fire_on_idle` `:4723`): a refused pass never cancels the loop.
- **Digest render** (`main.rs:445-504`, `:5850-5875`): the AC6 witness asserts on it; production code is unchanged (the plant is the only edit).
- **Router fallback walk** (`router.rs:46-84`) is unchanged; only the adapter decides whether it may run.

### Guardrails

- No second record/replay copy: extend `main.rs:3625-3667`, never wrap a driver elsewhere (epic `:129`).
- The token and the request name the same declared provider; never mint the router default for Butler (17-11 entitlement).
- Never default a missing completion field; never fall back to the arithmetic proposal after a bound-port error.
- Kernel edits only in `inference/mod.rs` and `router.rs`; FLAG-Winston sign-off before landing; in-`src` test lines count toward the pin and tokei.
- One plant per control AC (AC6 two, named), in place, restored by `git checkout -- <file>` (or copy-back + `cmp` where the file carries uncommitted 18-1 work, the 18-0 precedent); one target dir; local runs `env -u CI -u GITHUB_ACTIONS` (`proof-contract.md:22-27,84-87`).
- No `cargo` inside a `#[test]` (`proof-contract.md:68-76`); fixture binaries only via `build-test-fixtures`.
- If `intent-lineage-coverage-report.md` is modified by a kernel run, revert it (17-11 precedent).

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red |
|---|---|---|
| Butler shape/parse gate (AC1) | restore log-only `eprintln!` at `main.rs:5885` | `butler_inference_18_1` missing-`options` case exits 0 |
| Production digest (AC3) | remove the `morning_digest` call | `butler_digest_18_1`: no `morning_digest` event |
| Butler floors in production (AC6 a) | truncate the production window | open-halt equality reds |
| Digest-class floors in production (AC6 b) | keep the fixture `source_log_ref` (`main.rs:471,484`) | `journey_j3` extended witness: unresolved ref |
| Fallback authorization (AC7) | remove the fallback check | `inference::tests::fallback_…` reds (creation: 13/14) |

AC4 is an existing gate (`check-model-currency`), no plant. In CI: `epic-18-exit`'s twin is the line-1 red; the landing run of the whole matrix is the green proof.

### Tests and commands

```bash
# local (agent shell exports CI=true); one cargo process; CARGO_TARGET_DIR=/mnt/build/cargo
env -u CI -u GITHUB_ACTIONS cargo run --locked -p xtask -- build-test-fixtures
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p butler -p researcher -p maos-digest
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test butler_inference_18_1 --test butler_digest_18_1 \
  --test butler_8_14b --test inference_mode_15_6 --test researcher_8_14c \
  --test maosctl_load_16_6 --test one_daemon_one_door_16_1 --test root_shutdown_unload_16_3 --test wasm_form_admission_17_3b
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-journey-test   # journey_butler (JB-5 flip), journey_j3 (AC6 b), journey_researcher, journey_j0
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core && env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib
cargo run -p xtask -- check-kernel-baseline && cargo run -p xtask -- kloc-check --json
cargo run -p xtask -- check-model-currency && cargo run -p xtask -- check-test-cost
cargo fmt --all -- --check
```

`root_shutdown_unload_16_3` and some `one_daemon_one_door_16_1` scenes need the T3 substrate and print `SKIP:` locally (`tests/harness/ci.rs`); their proof is CI's. The Digest crate's package is `maos-digest` (`spirits/digest/Cargo.toml:2`).

### Review plan (proof contract `:31-57`)

- **Reviewer model:** `anthropic/claude-opus-5-5` only (operator ruling 2026-10-25; fable-5.1 declined, including the security layer).
- **Focused suites:** `-p butler`, `-p researcher` (shared binding), `-p maos-digest`, `-p maos-bin` (the binaries above), `-p maos-journey-test`, `-p maos-kernel-core` **plus** `cargo test -p maos-kernel-core --lib` (kernel touched). No manifest changes under D-18-1-02 A / CR-8, so no manifest focused-set widening.
- **Gates:** `check-kernel-baseline`, `kloc-check`, `check-model-currency`, `check-exit-commands` (exit line 1 verbs exist), `check-test-cost`; the workspace suite runs in CI.
- **Security-critical:** AC7 (authorization of every provider dispatch) and AC1.4 (issuance scope) — the Security review layer runs on opus-5-5 and is named in the review record. ◆ FLAG-Winston sign-off on the kernel diff.

### Previous-story intelligence

- **18-0 (done):** the lighter creation contract; prototypes measured then reverted; plants in place, one per control, recorded in `<story>-evidence/plants.md` with wall-clock; copy-back + `cmp` restore when the file carries uncommitted story work; kloc measured after `cargo fmt --all` and returned before landing.
- **17-11 (done):** issuance entitlement (`cap_policy/mod.rs:159-166`); the replay-arm rule "mint the Spirit's first declared provider" (`main.rs:5550-5566`); its review deferred this story's AC7 (`17-11-…md:157`) and 18-3's AC5 (`:158`).
- **15-6 (done):** one selector (ADR-064); explicit replay wraps every router driver (`main.rs:3629-3647`); exhaustion/drift surface as `MalformedResponse` through the provider path (`cassette_replay.rs:197-204`).
- **8.14b/8.10/8.11:** Butler's MCP port, epistemic scalar port and boot-loud halt port — the surfaces T1–T3 leave intact. **J3 (Digest class):** `journey_j3.rs` drives the real daemon render; `derive_team_digest` is the single derivation (`spirits/digest/src/lib.rs:217-219`).

### Git intelligence

`c816c544` 18-0 done; `f011ea3c` tracker hygiene; `b15eb503` 18-0 dev + review; `c8613658` 18-0 created, epic-18 opened. None touched this story's files except `deferred-work.md` (18-0 closures moved the AC7 row from `:1203` to `:1213`). Other agents' uncommitted planning edits (routing audit, PRD, epic-20, requirements inventory, index) were present at creation — not this story's.

### Project Structure Notes

- New test files follow `<topic>_18_1.rs`; evidence under `_bmad-output/implementation-artifacts/18-1-evidence/` (`plants.md`, `ci/`).

### References

- Epic `epic-18-spirits-that-think-w3.md`: §18-1 `:121-136` (creation corrections `:125`, rulings `:127`), §18-8 `:206-217`, exit block `:11-26`, Kernel-Δ `:32`, §R `:44-81`, stories table `:90,97`, rules 9–11 `:38-40`, Dependencies `:221`.
- `deferred-work.md:1212-1213`; `routing-audit-2026-10-22.md:35`; `prd/functional-requirements.md:51,92`; `prd/non-functional-requirements.md:126`; `prd/project-scoping-phased-development.md:406` (amendment (b)).
- `17-11-scope-selector-entitlement.md:33,148,157-158`; `docs/testing/proof-contract.md:10-18,22-27,31-57,68-80,84-92`; sibling `18-8-conformant-mcp-client.md`.

## Dev Agent Record

### Story context creation

Created 2026-10-25 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter creation contract) at `c816c544`: two read-only scouts (Butler/inference seam and exit-line provenance; MCP transport/fixture/kloc), direct reads of every cited range, the MCP 2026-07-28 changelog and versioning pages, and throwaway kernel prototypes (AC7 A, B, C′; the AC2 IO capture now priced in 18-8) measured with `check-kernel-baseline`, `wc -l` and `kloc-check --json` as the host's only cargo process (`/mnt/build/cargo`), each restored by copy-back with `git status` showing no crate change (pin 26361 / 95 files after the last revert). Rulings applied the same day (operator, Lunarpulse, via Main): D-18-1-01 B split → `18-8-conformant-mcp-client` (story created, epic §18-8, tracker row, `epics/index.md` 8 → 9); 02/03/04/05/08/10/11 A; 09 B (Digest class added to AC6 — the Digest code was measured for the re-scope: `spirits/digest/src/lib.rs:184-219,475-666`, `main.rs:445-504,5850-5875`, `journey_j3.rs:18-77`); kernel grant ratified; review opus-5-5 only. Status `ready-for-dev`.

### Agent Model Used

anthropic/claude-opus-5-5 (creation, 2026-10-25)

### Debug Log References

### Completion Notes List

### File List

### Change Log

- 2026-10-25 — Story created (backlog; D-18-1-01…13 PROPOSED) at `c816c544`: AC7 needs kernel code (◆ FLAG-Winston; A +2 physical / +1 tokei by reverted prototype, plant proven 13/14); keyless explicit replay cannot reach Butler's declared provider; Butler's notification lacks the manifest's `options` and JB-5 pins exit 0 on a violation; the daemon token expires after 60 s; MCP 2026-07-28 removed `initialize`/`Mcp-Session-Id` and the IO port drops response headers; the 100-digest floors exist at library level for Butler; epic §18-1/§R cites corrected in place.
- 2026-10-25 — RULED (operator, Lunarpulse): D-18-1-01 B split (old AC2/AC5 + D-06/07/12 → `18-8-conformant-mcp-client`); 02/03/04/05/08/10/11 A; 09 B (AC6 re-scoped to Butler + the Digest class: one production witness per class + a new Digest 100-digest corpus; CR-8/CR-9); kernel grant +2 / +1 ratified; review `anthropic/claude-opus-5-5` only. Status → `ready-for-dev`.
