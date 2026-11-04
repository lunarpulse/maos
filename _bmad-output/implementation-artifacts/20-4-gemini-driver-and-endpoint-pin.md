---
story_id: '20.4'
story_key: 20-4-gemini-driver-and-endpoint-pin
created: 2026-11-02
context_commit: a8af3215
baseline_commit: a8af3215
security_sensitive: true
kernel_grant: "RULED 2026-11-02 (ask s204; D-20-4-06 A + D-20-4-07 A; R = KA + KIO1 + KIO2): measured at a8af3215 from 26841 physical / 21042 tokei, +13 / +9. Ceilings (STOP above): kernel ≤ 26867 physical / 21060 tokei; L2 kernel last. The second or later kernel landing re-bases; cumulative STOP above 21209 tokei, ADR-069 alarm 22000; expected cumulative with 20-1b +68 and 20-11 +6 is 21125."
kloc_grant: "RULED 2026-11-02 (ask s204; D-20-4-11 A; D02 A ×2 measured / ×3 unmeasured), L1 rebased by parent handoff 2026-11-03 at 8c809f9c. L1 ceilings (STOP above; exact pins; aggregate hardfail = measured + 1): maos-providers ≤ 1898; maos-bin ≤ 34643 = 34191 + 145×2 + 54×3; maos-domain ≤ 10115; maos-manifest ≤ 4610; xtask ≤ 44466; aggregate ≤ 193526 = 192519 + 1007. Kernel delta 0 in L1 (26841 physical / 21042 tokei); L2 retains kernel_grant and re-bases at its start. Creation-wide pricing is retained below as historical evidence. Every other crate 0."
review: "bmad-code-review under docs/testing/proof-contract.md. DIFFICULT tier (`proof-contract.md:67-71`): a kernel FLAG-Winston row (D-20-4-06/07), secrets handling (two new credentials, header redaction, AC3.4, AC8.1) and a refusing control (the endpoint pin, AC6). Blind and Edge on opus-5.5, Acceptance on opus-5.5, a Security reading on AC1.1 (key placement), AC6 and AC8, Test Infrastructure on glm-5.3 (standing policy; any session substitution recorded at review). Creation anthropic/claude-opus-5-5."
---

# Story 20.4: Gemini driver and endpoint pin enforcement

**Shared kernel STOP supersession — 2026-11-04 (ask `halt_grant`).** Historical internal STOP
21209 / 21,209 and the 20-9-only 21230 / 21,230 are superseded by **21250 / 21,250 for the whole
cumulative measured kernel ledger at each landing**. D-20-14-12 C's joint 21250 now equals that shared STOP,
not a separate exception. With ruled 20-11 O4, cumulative measured tokei is **21203 + 12 = 21215**
(physical **27032 + 16 = 27048**). ADR-069 alarm **22000** and envelope **23000** are unchanged.
Older arithmetic/rulings below remain historical; later kernel landers must re-measure and re-base.


Status: ready-for-dev

<!-- Created 2026-11-02 at a8af3215 (main, one docs commit ahead of origin/main 47631284; `git log -1 -- crates xtask` = 47631284) by bmad-create-story under the lighter measured creation contract. Operator rulings in force: D01 A, D02 A, D11 A (2026-10-30). Preflight D06 is OPEN; D-20-4-01…11 are PROPOSED, not ruled — the operator rules; a different ruling rewrites only the clauses marked with that row. No cargo ran (parent rule): cites re-read at HEAD (`creation/cites-a8af3215.md`); kernel and kloc by the prebuilt gate binary on `git archive` copies (`creation/measurements.md`); the epic's AC1 shell line run on the prebuilt binary in an isolated home (`creation/runtime/`). Stale E20 §20-4 cites are corrected through `creation/records-owed.md` §A. -->

**Status rationale — ready-for-dev, not blocked.** Every AC is hermetic (mock transport, seeded cassettes, loopback
listeners). Two inputs gate parts of it: D06 decides who creates `MAOS_DEFAULT_PROVIDER` (AC1.5, AC3.3 wait for the
`provider-model-env-overrides` row only under D06 A), and the model ids are a policy edit (D-20-4-03, owner Winston).
Live Gemini/Kimi calls are operator legs and need L2's kernel row first (C21).

> **The capability:** *an operator can run any Spirit on Anthropic, OpenAI, Gemini, Kimi or a local Ollama — the Spirit's own `[providers]` choice where it makes one — and a Spirit that pins its provider endpoint is refused, before any token is minted, by a root whose configured endpoint is not the one it pinned.*

**Closes:**
- Epic-20 §20-4 (E20@HEAD:201-209) as amended 2026-10-30 (D01 A; preflight §3.4): epic AC1 → AC1 + AC3, AC2 → AC6, AC3 → AC4; plus the D01 A widening (R:47): Kimi → AC2, per-Spirit selection → AC5.
- FR3 (`prd/functional-requirements.md:26`) except Bedrock (parked, S:344): Gemini, Kimi, per-Spirit selection, endpoint locking.
- `deferred-work.md:1297` (ureq `BadHeader` echoes a provider credential) → AC3.4 + AC8.1; `:1298` (non-2xx over 10 MiB loses its status) → AC8.2. Both name this key as owner.
- `tests/coverage-matrix.yaml:566-570` (FR3 `gates: []`) → AC9.

**Does NOT close, and says so:**
- Live Gemini/Kimi round trips with a paid key (`ops-live-gemini-leg`, S:342; Kimi per D-20-4-09). This story's replay lines prove selection and routing; the round-trip tests prove the wire shape against fixtures; neither is a vendor call.
- The three `MAOS_*_MODEL` overrides of `provider-model-env-overrides` (S:224) unless D06 rules B; per-Spirit *model* or *endpoint* reconfiguration (D-20-4-04 D); Bedrock/Vertex/KMS (S:344).
- Real HTTP status for the existing drivers (`provider-error-surfacing`, S:225: `http_post` flattens non-2xx to `Transport`, `io/mod.rs:97-108`); the Gemini driver uses `http_post_exchange` and gets real status for itself only.
- The FR38 runtime provider-endpoint drift check in registry admission (vacuous in production, C23) — D-20-4-10.
- Worker CLI proxies (`worker_cli.rs:749,815`) and their egress grants; crypto-provider selection (20-13); cost-attribution completeness (20-14).

## Story

As **an operator who chooses inference providers per deployment and per Spirit — including air-gapped hosts on a local model**,
I want **Gemini and Kimi drivers beside the existing three, a default I can name, a Spirit's own `[providers]` choice honoured, and its endpoint pin enforced**,
so that **a Spirit runs on the provider I configured, and a pinned Spirit never talks to an endpoint it did not pin.**

**Contract:**
- Epic §20-4 (E20@HEAD:201-209) as amended 2026-10-30 (D01 A, preflight `epic-20-preflight-2026-10-30.md` §3.4, §2 row 20-4, D06) — create-story may tighten, not add uncited ACs: AC2/AC5 are the received R:47 charter (E20:205), AC7/AC8 the received D:1297/D:1298 rows, AC4 the epic's AC3 corrected to the closed 17-5 gate.
- Tracker `sprint-status.yaml:302`; rules 9–11 (E20 confidence rules, E20:59-61); proof contract `docs/testing/proof-contract.md:10-31` (one final-tree run + one plant per AC), `:33-71` (review, tiers; a manifest change widens the focused set, `:48-53`).

## Inventory at HEAD — measured, not assumed

Method: every cite re-read at `a8af3215`; witnesses located by `grep` over `crates/*/tests` and inline test modules;
the epic's AC1 line run on the prebuilt HEAD binary (`creation/runtime/selector-probe-a8af3215.txt`); no cargo.

| # | Charter item | Disposition | Code at HEAD | Witness / measurement | AC |
|---|---|---|---|---|---|
| 1 | Gemini driver | **missing** | providers `src/` = anthropic/openai/ollama/fixture_replay/rate_limit/provider/lib (1425 tokei); FR47 denies `gemini-rs`, `google-generative-ai` (`xtask/fr47-vendor-sdk-denylist.toml:16-17`) | — | AC1 |
| 2 | Kimi driver | **missing**; OpenAI driver fits the wire | `openai.rs:81` `{endpoint}/v1/chat/completions` + Bearer; attribution fixed `"openai"` `:160`, key fixed `OpenAiApiKey` `:38`; Kimi = `https://api.moonshot.ai` + same path (vendor overview, C24) | `openai_round_trip_test.rs` (3) | AC2 |
| 3 | Credentials | **missing** | `SecretKey` = Anthropic, OpenAI, 8 MCP bearers (`crates/maos-domain/src/ports/secret_store.rs:10-15,51-78`) | `provisioning-checklist.md:42` "no credential name exists" | AC3 |
| 4 | Registration / default | literals, first-registered default | `main.rs:3896-3963`; replay interns `anthropic/openai/ollama` `:4020-4024`; shell mints the router default `:4374`, `shell_host.rs:112-122` | — | AC3 |
| 5 | `MAOS_DEFAULT_PROVIDER` | **missing** | `git grep` 0 | measured: set to `gemini`, TL row `infer:anthropic->replay:default` (S2) | AC1, AC3 |
| 6 | Epic AC1 shell line | **red** | cassette `cassettes/providers/gemini-shell-intro.json` absent | measured **rc 1** (S1) | AC1 |
| 7 | Model currency | **exists, blind to new providers** | `xtask/model-currency.toml` (anthropic/openai/ollama); scan regex `check_model_currency.rs:241-243`, hosts `:287-293` | measured: Gemini+Kimi constructors with unlisted ids → rc 0, 16 pins (= HEAD) | AC4 |
| 8 | `[providers]` per-Spirit section | **parse-only, never read** | schema `manifest.rs:2413-2536` (ids anthropic/openai/ollama `:2413`); `gate_manifest` ignores it (`admission.rs:590-723`); `admit_spirit(.., providers = None, ..)` `admission.rs:917` | `ProvidersSection` unit tests `manifest.rs:4281-4398`; kernel `ProviderSwitched` fed by tests only (`crates/maos-kernel-core/tests/provider_switched_journal.rs`) | AC5 |
| 9 | Per-Spirit binding today | declaration order | `declared_providers` `main.rs:890-904`; Researcher `spirits/researcher/src/lib.rs:487`; Orchestrator `orchestrator_root.rs:221-237`; shell = router default | `researcher_inference_18_3.rs:301` (unregistered refused) | AC5 |
| 10 | Endpoint pin | **false** (silently ignored) | field `manifest.rs:2454`, 64-hex check `:2510-2516`; no consumer | — | AC6 |
| 11 | Kernel provider-id set | **refuses Gemini/Kimi** under the limiter | `inference/mod.rs:91-109` (match `:96-101`); limiter in every mode but explicit replay (`inference_mode.rs:95-97`) | `inference_fallback_authz_18_1.rs` (adapter harness to reuse) | AC7 ◆ |
| 12 | Kernel IO credential echo | **open** (D:1297) | `io/mod.rs:123-126` `Err(e) => Transport(format!("ureq POST {url}: {e}"))` | MCP half closed by 18-8 (`maos-mcp/src/transport/streamable_http.rs:85`) | AC3.4, AC8 ◆ |
| 13 | Kernel IO status loss | **open** (D:1298) | `read_capped` `io/mod.rs:65-78`, POST `:129` | — | AC8 ◆ |

## Current-tree corrections — read before implementing

Full OLD → NEW: `creation/cites-a8af3215.md` (C1–C24, T1–T3, P1–P3, L1–L5).

| Premise (epic / preflight / tracker) | Fact at `a8af3215` and consequence |
|---|---|
| "kernel-Δ 0" (E20:77,203,208; PF:123) | The kernel interns a closed provider-id set and refuses `gemini`/`kimi` whenever the rate limiter is installed (live, record, deterministic); D:1297/D:1298 are kernel IO rows owned here. Measured R = +13 / +9 (C9, P3). |
| "model id is a literal with a `// model-currency` marker until the lane lands" (E20:209) | Closed by 17-5; the gate exists but cannot see a Gemini/Kimi constructor (measured, C10). AC4 extends it. |
| "governed by `xtask/fr63-typed-errors.toml`" (E20:208) | No gate reads that file (C8). The refusal's contract is its `AdmissionRefusal` code and the AC6 plant. |
| "enforced at provider construction / `InferencePortAdapter`" (E20:208; S:302) | Endpoints are fixed at boot; the pin lives in a Spirit manifest that admission never reads (C14). Enforcement point: admission (D-20-4-05). |
| "the shell line replies from the cassette" proves the driver (E20:207) | Replay swaps every driver for the cassette provider (`main.rs:3998-4025`): it proves selection/routing/scope, never the wire (C16). |
| `[providers].primary` "not used for selection" (E20:207) | Not read at all; `fallback` ids are also refused by the kernel at call time unless equal to the primary (`inference/mod.rs:291-293`). |
| Anchors `main.rs:3081-3100,3136`, `manifest.rs:2103,2159-2171`, `anthropic.rs` 307, providers 1570, `sprint-status.yaml:294` | `main.rs:3896-3963,4073-4085`; `manifest.rs:2454,2510-2516`; 427; 1425 tokei; S:344 (C2–C6, C11). |

## Kernel and kloc pricing — measured at creation

**Instrument:** prebuilt `/mnt/build/cargo-maos/debug/xtask` (`check-kernel-baseline --json`, `kloc-check --json`) run
from a `git archive a8af3215` copy root per prototype; the untouched copy reproduces the tree exactly (26841 / 95 files
/ set hash `db39bc4a…`; kernel 21042 tokei; aggregate 191826). Detail and diffs: `creation/measurements.md`,
`creation/kloc-kernel-a8af3215-and-prototypes.json`.

**Kernel (◆ FLAG-Winston, PROPOSED):**

| Shape | Kernel physical | Kernel tokei | Other | Diff |
|---|---:|---:|---|---|
| KA `bucket_key` interns `gemini`, `kimi` (D-06 A) | +2 (26843) | +2 (21044) | — | `ka-intern-gemini-kimi.diff` |
| KB closed set moved to `maos_providers::intern_provider_id` (D-06 B) | −4 (26837) | −4 (21038) | maos-providers +3 | `kb-intern-set-in-providers.diff` |
| KIO1 `BadHeader` → "value withheld" (D:1297; D-07 A) | +7 (26848) | +5 (21047) | — | `kio1-badheader-redaction.diff` |
| KIO2 non-2xx over the cap truncated, status kept (D:1298; D-07 A) | +4 (26845) | +2 (21044) | — | `kio2-non2xx-truncate-keep-status.diff` |
| **R = KA + KIO1 + KIO2 (recommended)** | **+13 (26854)** | **+9 (21051)** | — | `r-ka-kio1-kio2-bundle.diff` |
| R′ = KB + KIO1 + KIO2 | +7 (26848) | +3 (21045) | maos-providers +3 | `rprime-kb-kio1-kio2-bundle.diff` |
| Rmin = KA only (D-07 C) | +2 | +2 | D:1297/1298 re-routed | `ka-intern-gemini-kimi.diff` |
| none (D-06 C) | 0 | 0 | live/record Gemini/Kimi refused `Unconfigured` | — |

Both pins move with any kernel shape (content hashes of `inference/mod.rs`, `io/mod.rs`). Not compiled ([INFERENCE];
measurement owed 1–2).

**kloc forecast** (tokei; HEAD = every pin, 1 line aggregate room; recommended options):

| Crate · component | Measured / forecast | Basis |
|---|---:|---|
| maos-kernel-core · R (AC7, AC8) | **+9** (physical +13) | prototype `r` |
| maos-providers · `gemini.rs` (AC1) | **+142** | prototype `g1` |
| maos-providers · `OpenAiProvider::compatible` for Kimi (AC2) | **+26** | prototype `k1` |
| maos-providers · `PROVIDER_IDS` (AC3) | **+1** | prototype `sel` |
| maos-providers · credential byte check for four keyed drivers (AC3.4), optional `gemini`/`kimi` quota defaults | +10…+45 [INFERENCE] | 18-8 bearer check `streamable_http.rs:81-87`; `rate_limit.rs:276-293` |
| maos-domain · two `SecretKey` variants (AC3.1) | **+8** | prototype `sec` |
| maos-bin · env rows + purge labels (AC3.1) | **+17** | prototype `sec` |
| maos-bin · registration, replay ids, `MAOS_DEFAULT_PROVIDER` (AC3.2–3.3) | **+51** | prototype `sel` |
| maos-bin · `[providers]` parse, declared check, pin check, two refusals, deps plumbing (AC5.1, AC6) | **+77** | prototype `pin` |
| maos-bin · binding sites honour `primary`; `fallback` refusal; endpoint entries for Gemini/Kimi (AC5.3) | +24…+54 [INFERENCE] | `main.rs:890-904,4374`; `orchestrator_root.rs:221-237` |
| maos-manifest · `ALLOWED_PROVIDER_IDS` (AC5.2) | 0…+2 [INFERENCE] | `manifest.rs:2413` |
| xtask · currency scan of Gemini + `compatible` constructors, five hosts (AC4) | +5…+20 [INFERENCE] | `check_model_currency.rs:239-295` |
| spirits (`spirits/researcher/src`), tests, fixtures, cassettes, toml data, docs | uncharged | `kloc_check.rs:300-317` |
| **aggregate** | **+331 measured; +331…+452 total** | sum |

**Envelope (D-20-4-11 A, PROPOSED; D02 A ×2 measured / ×3 unmeasured upper; ceilings, not reservations):**

| Crate | HEAD | Forecast | Ceiling with allowance | L1 ceiling | Inherited room | Operator ruling needed |
|---|---:|---:|---:|---:|---:|---|
| maos-kernel-core (physical / tokei) | 26841 / 21042 | +13 / +9 | **26867 / 21060** | — (L2) | 0 | **yes — FLAG-Winston** |
| maos-providers | 1425 | +169 meas, +10…+45 | **1898** (pin 2000 kept) | 1898 | 575 numeric | yes (aggregate) |
| maos-bin | 33498 | +145 meas, +24…+54 | **33950** | 33950 | 0 | yes |
| maos-domain | 10099 | +8 meas | **10115** | 10115 | 0 | yes |
| maos-manifest | 4604 | 0…+2 | **4610** | 4610 | 0 | yes |
| xtask | 44406 | +5…+20 | **44466** | 44466 | 0 | yes |
| aggregate | 191826 | +331…+452 | **192851** (+662 + 363); hardfail = measured + 1 | 192833 | 1 | yes |

maos-bin interaction: 20-1b's Part-B maos-bin ceiling was computed from 33195 (S:297, 20-1a frontmatter); every
story re-measures against the exact landed pins of the one before it. Pins land exact after `cargo fmt --all`; a lower
measurement lowers the pin.

**Operator-ruling list (every draw above the inherited room; PROPOSED):** (1) FLAG-Winston kernel row R (D-20-4-06 +
D-20-4-07; R′/Rmin/none priced); (2) maos-providers ≤ 1898; (3) maos-bin ≤ 33950; (4) maos-domain ≤ 10115;
(5) maos-manifest ≤ 4610; (6) xtask ≤ 44466; (7) aggregate ≤ 192851, hardfail measured + 1; (8) D06 and
D-20-4-01…11. Each figure lands exact, with a HISTORY row citing this key with `->` (D11 A).

## Decisions — RULED 2026-11-02 (ask s204)

**Rulings 2026-11-02 (ask s204):** The operator ruled D06 D and every D-20-4-01…11 row exactly as recommended. 20-4 creates `MAOS_DEFAULT_PROVIDER`; the existing lane retains the three `MAOS_*_MODEL` overrides. Kernel row R and the ceilings are granted as recorded in frontmatter.

**Superseding ruling 2026-11-03 (ask d204_12): D-20-4-12 B.** D-20-4-03's Kimi default is now `kimi-k3.1-pro`, not `kimi-k2.6`; Gemini remains `gemini-3.5-flash-lite` (re-checked Stable on Google's models page 2026-11-03). K2 ids are refused as vendor-deprecated. The successor's context is 1,048,576 tokens; access needs a successful $1 top-up; account-tier limits apply (Tier0: concurrency 1, RPM 3, TPM 500,000, TPD 1,500,000). Standard uncached input/output prices are $2/$10 per million tokens. Sources, fetched 2026-11-03: [guide](https://platform.kimi.ai/docs/guide/kimi-k3-1-pro-quickstart.md), [pricing](https://platform.kimi.ai/docs/pricing/chat), [limits](https://platform.kimi.ai/docs/pricing/limits). Rendered price/limit evidence is saved under `landing-1/` because the vendor's markdown omits table cells. This supersession applies to every D-20-4-03 reference below.

| ID | Question | Options | Recommendation and evidence | Status |
|---|---|---|---|---|
| **D06** (preflight) | Who creates `MAOS_DEFAULT_PROVIDER` (AC1's selector) | **A** author `provider-model-env-overrides` (S:224) first, then 20-4 · **B** 20-4 absorbs the whole four-variable charter (`tracker-notes-archive-2026-09-04.md:713-715`) and the lane row retires by explicit migration · **C** hard-coded/global default only · **D** *(added at creation)* 20-4 creates `MAOS_DEFAULT_PROVIDER` only (prototype `sel`, inside maos-bin +51); the lane keeps the three `MAOS_*_MODEL` overrides, its note re-scoped | **D.** The selector is the only part of that charter AC1 needs and it sits in the exact builder block this story rewrites (`main.rs:3896-4049`) — A serialises two stories over the same 60 lines with no story file yet for the lane. The model overrides are not a drive-by: an env-supplied model id is a pin the 17-5 gate cannot scan (it reads literals, `check_model_currency.rs:24-26`), which is why the lane carries "currency verified in-story" (OD3); B pulls that design in. C fails the epic's own AC1 command. | RULED 2026-11-02: **D** (ask s204) |
| **D-20-4-01** | Gemini wire shape | **A** native `generateContent` (`POST {endpoint}/v1beta/models/{model}:generateContent`), key in `x-goog-api-key`, through `http_post_exchange` (status as data) — prototype `g1`, +142 · **B** Gemini's OpenAI-compatible endpoint through the OpenAI driver with a path parameter · **C** native through `http_post` (non-2xx flattened to `Transport`, like the siblings) | **A.** The epic names a hand-rolled native driver (E20:207); `generateContent` is the documented REST method (`https://ai.google.dev/api/generate-content`, read 2026-11-02). Status as data lets a Gemini 429 reach the kernel's existing `RateLimited` mapping (`inference/mod.rs:338-354`), which C cannot. B couples Gemini to a compatibility layer and still needs a path change in `openai.rs:81`. The key never goes in the URL: the URL is echoed in transport errors (`io/mod.rs:106,125`) and journaled (`:117-118`) — the vendor's own curl sample uses `?key=` (AC1.4 plants exactly that). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-02** | Kimi driver | **A** `OpenAiProvider::compatible(provider_id, secret_key, …)`; Kimi = id `kimi`, `KimiApiKey`, `https://api.moonshot.ai` — prototype `k1`, +26 · **B** a separate `kimi.rs` (copy of `openai.rs`, ≈ +120 [INFERENCE]) · **C** Kimi's Anthropic-compatible endpoint (`/anthropic/v1/messages`) through a generalised `AnthropicProvider` | **A.** Kimi documents OpenAI Chat Completions compatibility at base `https://api.moonshot.ai/v1` (`https://platform.kimi.ai/docs/api/overview.md`); the driver's URL shape already matches (`openai.rs:81`), only attribution (`:160`) and key (`:38`) are fixed. B duplicates 200 lines that drift; C is a second compatibility layer with no advantage. | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-03** | Default model pins (policy, Winston) | **A** `gemini-3.5-flash-lite` + `kimi-k2.6` (the cost tier, matching the `claude-haiku-4-5-20251001`/`gpt-4o-mini` defaults) · **B** `gemini-3.8-flash` + `kimi-k3.1-pro` (capability tier) · **C** the operator names them at T0 | **A, re-verified on the vendor pages at T0.** Google lists `gemini-3.5-flash-lite` as Stable and recommends 3.5 Flash-Lite or 3.8 Flash for new projects (`https://ai.google.dev/gemini-api/docs/models`, read 2026-11-02); Kimi's K2.6 quickstart uses `kimi-k2.6` (`https://platform.kimi.ai/docs/guide/kimi-k2-6-quickstart.md`). Either way every chosen id enters `xtask/model-currency.toml` `allowed` and the price book before it appears in `main.rs` (AC4). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-04** | Per-Spirit provider selection (R:47) | **A** the Spirit manifest's `[providers]` (existing Story 5.5b schema): read at admission; `primary.id` must be declared in `provider.complete` and registered, and becomes the Spirit's bound provider at every binding site; `endpoint_url`/`model_id` are assertions (must equal the registered driver's); a non-empty `fallback` is refused typed; `admit_spirit` receives the section (kernel `ProviderSwitched` journal, no kernel line) · **B** an operator per-Spirit table in `~/.maos/config.toml` (`[spirits.<name>] provider = …`), manifests untouched · **C** A + B (author declares and pins, operator re-selects among declared providers) · **D** per-Spirit driver instances honouring a manifest `endpoint_url`/`model_id` (a router per Spirit) | **A.** Lands the schema the epic's AC2 already binds the pin to, with no new config surface; the kernel journal for a provider switch exists and is unfed (`security/mod.rs:417-448`). `fallback` is refused rather than silently unused: the kernel refuses any fallback id the token does not name (`inference/mod.rs:291-293`). Boundary, stated: an operator cannot re-select the provider of a signed third-party Spirit without editing its manifest (breaking its signature) — that is C's reason to exist, at ≈ 2× cost and a config schema; D needs per-Spirit routers through `port_builder` (`main.rs:4089-4114`) and the shell path. | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-05** | Where the endpoint pin is enforced (epic AC2) | **A** admission (maos-bin): `check_provider_pins` before `admit_spirit` at every production admission site, against the per-id configured-endpoint table (every `PROVIDER_IDS` id, key or not; captured before replay/record wrapping); `sha256` of the exact base-URL bytes; typed `endpoint_pin_mismatch`; prototype `pin`, +77 with AC5.1 · **B** per request, a pinned wrapper around each Spirit's port in `port_builder` · **C** A + B | **A.** Endpoints are fixed once per boot (literals; `MAOS_OLLAMA_URL` read once, `main.rs:3940`), so admission sees the only value a call can use and refuses before a token exists. B misses the shell path (it uses the shared adapter, `main.rs:4073-4085`, not `port_builder`) and re-checks an immutable value per call; C pays both. Never in `inference/mod.rs` (E20:208). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-06** | Kernel provider-id set (FLAG-Winston) | **A** KA: two arms (+2 / +2) · **B** KB: the set moves to `maos_providers::intern_provider_id` (kernel −4 / −4, providers +3) · **C** no kernel: Gemini/Kimi in explicit replay only; live/record/deterministic refused `Unconfigured` | **A.** Smallest change that makes FR3 live true; the fail-closed list stays under kernel review (the Story 6.4 review fix, `inference/mod.rs:86-90`). B shrinks the kernel but moves a fail-closed security list to a crate no FLAG reviews — the next provider would widen it unreviewed. C leaves `ops-live-gemini-leg` unrunnable (record mode installs the limiter, C21). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-07** | D:1297 / D:1298 (kernel IO, owned here) | **A** KIO1 + KIO2 in the kernel (+11 / +7) **and** a provider-side credential byte check (0x21–0x7E) at construction for the four keyed drivers · **B** provider-side check only for D:1297 (the kernel echo stays reachable by any future header-carrying caller, labelled) + KIO2 · **C** re-route both (rule 10) and keep the kernel to KA | **A.** The kernel adapter is the one choke point every header-carrying caller crosses (providers, MCP); 18-8 closed only MCP's half (`streamable_http.rs:85`). The provider-side check gives a typed refusal before any request — the 18-8 precedent. C re-routes rows the ledger already assigned here with the same FLAG review (D:1297-1298). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-08** | Landing shape | **A** one key, two landings: L1 kernel-free (AC1–AC6, AC9 partial) · L2 kernel last (AC7, AC8, AC9 close); `done` after L2 · **B** one landing · **C** split keys `20-4a`/`20-4b` | **A.** The creation contract puts kernel last; L1 alone makes every replay line and the pin true, L2 makes live true. C adds keys (and the doc-gate churn 20-7 recorded on a rename) for a +13-line kernel row. Interim state, stated: between L1 and L2 a box whose only key is Gemini or Kimi defaults to it and its live/record/deterministic calls fail closed `Unconfigured` (typed, never silent); no release tag between the landings (20-5 runs last, PF:124). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-09** | Kimi live proof (PF:123 "live Kimi proof unmeasured") | **A** widen `ops-live-gemini-leg`'s note to record a Kimi cassette too (key unchanged) · **B** a new `ops-live-kimi-leg` row · **C** fixture-only, labelled | **A.** Same operator, same mode, same leg shape; renaming or adding keys touches the doc gates (20-7 incident, `20-7-…md:374`). C leaves FR3's Kimi claim fixture-only forever. | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-10** | FR38 runtime provider-endpoint drift check is vacuous (found at creation, C23) | **A** fix here: the two production registry admission callers (`maos-registry/src/install.rs:50-59`, `maos-cli/src/subcommands.rs:1402-1410`, both in `maosctl`, where no provider is built) receive `runtime_provider_endpoint` from a provider-endpoint table shared through maos-providers · **B** route to `20-13-pluggable-crypto-provider-selection`, which owns the twin `runtime_crypto_provider` default (`maos-registry/src/admission.rs:138-145`) and names the runtime context (E20:312) · **C** ownerless, labelled | **B.** Rule 10(b): 20-13 already touches the same struct and the runtime context; this story does not touch registry admission, and A needs the daemon's endpoint table outside the daemon. Filed as a `deferred-work.md` row with PROPOSED owner 20-13 (records-owed X4); a different ruling re-points it. | RULED 2026-11-02: **B** (ask s204) |
| **D-20-4-11** | Per-crate envelope (D02 A applied) | **A** the envelope above (×3 unmeasured / ×2 measured) · **B** exact measured grants only, every review correction re-asks · **C** allowance ×1.3 (rule 6) | **A.** 20-3b and 20-3c landed at 2–3× creation forecasts; 20-7 L1 review corrections added +387 xtask inside its ceiling (`xtask/kloc.toml:339` history). | RULED 2026-11-02: **A** (ask s204) |
| **D-20-4-12** | T0 re-verification of the Kimi default after K2 access restriction and announced retirement | **A** retain K2 with an explicit exception · **B** use the successor | **B: `kimi-k3.1-pro`**, superseding D-20-4-03's Kimi default only. K2 ids refused as vendor-deprecated; Gemini unchanged. T0 price/context/limits re-checked above. | **RULED B 2026-11-03 (ask d204_12); implementation resumed** |

## Acceptance Criteria

*(Epic §20-4 as amended 2026-10-30; clauses marked (D-…) carry the PROPOSED recommendation and are rewritten by a
different ruling. Every AC names a failing-before witness and a plant. **local** = hermetic on the dev host;
**hosted** = the discipline run.)*

### L1 — kernel-free

#### AC1 — Gemini answers: the driver round-trips and the shell selects it (epic AC1; D-20-4-01 A) — the command

1. `crates/maos-providers/src/gemini.rs` (`GeminiProvider::{new, with_api_key}`, prototype `g1`): `POST {endpoint}/v1beta/models/{model}:generateContent` with `x-goog-api-key` and `content-type: application/json`; body `contents: [{role: "user", parts: [{text}]}]`, `generationConfig: {maxOutputTokens, temperature?}`; `options.model_id`, when present, must equal the configured/admitted model; both configured and requested ids must be nonempty ASCII `[A-Za-z0-9._:-]` (operator-adjudicated security review F1 supersedes the original override allowance); sent through `http_post_exchange` so a non-2xx is `ProviderError::ProviderRejected { status, body }`; the response text is every `candidates[0].content.parts[*].text` concatenated; `finishReason` `STOP` → `StopSequence`, `MAX_TOKENS` → `MaxTokens`, any other → `ProviderStop(<value>)`; usage from `usageMetadata.promptTokenCount` / `candidatesTokenCount`; attribution `gemini`; `credential_fingerprint` overridden. No vendor SDK (FR47). The key never appears in a URL.
2. **Witness (local):** `cargo test -p maos-providers --test gemini_round_trip_test` (new) over a *recording* `MockTransport` (captures URL, headers, body; returns a fixture status + body) and fixtures `crates/maos-providers/tests/fixtures/gemini_{success,max_tokens,multi_part,safety,error_429}_response.json`: asserts path and model, the header, that the key is absent from the URL and body, every parsed field, and `429` → `ProviderRejected { status: 429, .. }`.
3. **Failing-before (local):** the test target and `gemini.rs` do not exist at `a8af3215` (C3).
4. **Plant (local, one):** send the key as a `?key=` query parameter (the vendor's documented curl form) ⇒ the no-key-in-URL assertion reds.
5. **The command (local; after AC3):** `printf '@hello-spirit hello\n' | MAOS_DEFAULT_PROVIDER=gemini MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/providers/gemini-shell-intro.json maos shell` exits 0, prints the cassette reply, and the Transparency Log carries `infer:gemini->replay:…` for hello-spirit's turn under a `ProviderInfer { provider: "gemini" }` token. The seed cassette (`provenance: "seed"`, attribution `provider_id: "replay"`, as `cassettes/j0/shell-intro.json`) is new; `ops-live-gemini-leg` replaces it with `live-record`. Measured at HEAD: **rc 1** (cassette absent) and, with the J0 cassette, `infer:anthropic->replay:default` (`creation/runtime/selector-probe-a8af3215.txt` S1, S2). Witness: `crates/maos-bin/tests/provider_selection_20_4.rs` runs the built binary in an isolated home and reads the TL row. **This line proves selection, routing and the minted scope; it never exercises the Gemini wire (C16) — AC1.2 does.**

#### AC2 — Kimi through the OpenAI-compatible driver (R:47; D-20-4-02 A)

1. `OpenAiProvider::compatible(provider_id, secret_key, transport, endpoint, model, store)` (prototype `k1`); `OpenAiProvider::new` delegates with `("openai", SecretKey::OpenAiApiKey)`; attribution names `provider_id`. Kimi = `compatible("kimi", SecretKey::KimiApiKey, …, "https://api.moonshot.ai", <D-20-4-03 model>)`.
2. **Witness (local):** `cargo test -p maos-providers --test kimi_round_trip_test` (new; recording transport; `tests/fixtures/kimi_{success,max_tokens}_response.json`): URL `https://api.moonshot.ai/v1/chat/completions`, `Authorization: Bearer <key>`, attribution `kimi`. `openai_round_trip_test` unchanged and green. The replay line of AC1.5 with `MAOS_DEFAULT_PROVIDER=kimi` and `cassettes/providers/kimi-shell-intro.json` gives `infer:kimi->…` (same witness file).
3. **Failing-before (local):** attribution is the literal `"openai"` (`openai.rs:160`) and no Kimi key exists (`secret_store.rs:10-15`).
4. **Plant (local):** restore the `"openai"` literal in `parse_openai_response` ⇒ the Kimi attribution assertion reds.

#### AC3 — credentials, registration and the named default (D06 per ruling; D-20-4-07 A provider half)

1. `SecretKey::{GeminiApiKey, KimiApiKey}` → `gemini-api-key`/`MAOS_GEMINI_API_KEY`, `kimi-api-key`/`MAOS_KIMI_API_KEY`; `ALL` 10 → 12 (keyring migration `main.rs:3827-3858` and purge `purge.rs:333-340` follow `ALL`); purge labels; `env_contract.rs` UserFacing rows (`check-env-contract` green). Prototype `sec`.
2. The builder registers Gemini (`https://generativelanguage.googleapis.com`) and Kimi (`https://api.moonshot.ai`) after OpenAI and before Ollama, so the default of every box without the new keys is unchanged; replay answers every id in `maos_providers::PROVIDER_IDS` (`main.rs:4020-4024`). Prototype `sel`.
3. (D06 D/B) `MAOS_DEFAULT_PROVIDER=<id>` names the router default; empty = unset; an id that is not registered refuses the boot with a message naming the registered ids (never a silent fallback). Under D06 A this clause is the lane's and AC1.5 waits for it.
4. (D-20-4-07 A) Each keyed driver (anthropic, openai, gemini, kimi) refuses at construction a credential with any byte outside 0x21–0x7E, with a message naming the key, never its value; the root refuses the boot (exit 78, `main.rs:3914-3917` pattern). No new `ProviderError` variant: the kernel matches it exhaustively (`inference/mod.rs:357-365`) — adding one is a kernel line outside the ruled row (STOP).
5. **Failing-before (local):** measured S2 (selector ignored); `MAOS_GEMINI_API_KEY` registers nothing (no `SecretKey`). Witnesses in `provider_selection_20_4.rs`: selector honoured; unknown id (`bedrock`) refused naming the registered ids; a `MAOS_GEMINI_API_KEY` carrying `\u{1}` refuses the boot and the stderr does not contain the key.
6. **Plant (local):** ignore `MAOS_DEFAULT_PROVIDER` ⇒ the `infer:gemini->` assertion reds.

#### AC4 — new model pins are governed (epic AC3, corrected to the closed 17-5 gate; D-20-4-03)

1. `xtask/model-currency.toml` gains `[providers.gemini]` and `[providers.kimi]` (`allowed` per D-20-4-03, vendor pages cited and dated in its header, policy owner Winston `:7-9`); the `main.rs` defaults and the hello-spirit declarations `gemini.<id>`, `kimi.<id>` (`spirits/hello-spirit/manifest.toml:14`; the shell mints the router default and issuance refuses an undeclared provider, L5) are allowed pins.
2. `xtask/src/check_model_currency.rs` scans `GeminiProvider::{new, with_api_key}` (model = third argument) and `OpenAiProvider::compatible` (model = fifth argument) and requires all five hosts; literal arguments and `crate::NAME.1.into()` references to attribute-free root tuple constants are governed with declaration-line diagnostics and one pin per source definition (security review F6). Shared definitions are parsed as Rust, never inferred from comments, conditional declarations or lexical shadows; module documentation states this closed scope.
3. `xtask/provider-pricing.toml` gains a row per default model from the vendors' pricing pages (`cost-reconcile` matches `(provider, model)` exactly, header `:1-7`).
4. **Failing-before (measured):** with Gemini and Kimi constructors naming ids absent from the data file the gate passes — rc 0, 16 pins, identical to HEAD (`creation/measurements.md` §Gate blindness). Witness vectors in `xtask/tests/model_currency_gate.rs`: an unknown `gemini` id in a `main.rs` fixture reds; a retired `kimi` id reds; a fixture without a Kimi constructor reds.
5. **Plant (local):** drop `Gemini` from the constructor regex ⇒ the Gemini vector reds.

#### AC5 — a Spirit's own provider choice is honoured (R:47; D-20-4-04 A)

1. `gate_manifest` (`admission.rs:590-723`) reads `[providers]` into `GatedManifest.providers` (prototype `pin`); every `primary`/`fallback` id must be declared in `provider.complete` (else typed `provider_unavailable`); a non-empty `fallback` is refused typed (`fallback_unsupported`: the kernel refuses fallback ids the token does not name, `inference/mod.rs:291-293`).
2. `crates/maos-manifest/src/manifest.rs:2413` allows `gemini` and `kimi`; the two tests asserting `kimi` unsupported (`:4325-4337` and the fallback case after it) re-target an id that stays unsupported (`bedrock`).
3. The Spirit's bound provider is `[providers].primary.id` when present (it must be a router id at boot — explicit replay answers every `PROVIDER_IDS` id, `main.rs:4020-4024` — else `provider_unavailable` at admission), otherwise today's rule — at every binding site: shell (`main.rs:4374`), Butler (`first_declared_provider`, `:890-892`), Researcher (`spirits/researcher/src/lib.rs:487`), Orchestrator (`orchestrator_root.rs:221-237`).
4. `admit_spirit` receives `gated.providers.as_ref()` (`admission.rs:917`), feeding the kernel's existing `ProviderSwitched` journal (`security/mod.rs:417-448`) — no kernel line.
5. **Failing-before (local):** a Researcher manifest declaring `anthropic` and `openai` with `[providers] primary.id = "openai"` binds `anthropic` at HEAD (section ignored, C14). Witness in `provider_selection_20_4.rs` (replay registers every id; the TL intent names the bound provider).
6. **Plant (local):** the binding ignores `primary` ⇒ the openai-binding vector reds.

#### AC6 — the endpoint pin refuses (epic AC2; D-20-4-05 A) ◆ control (rule 11)

1. Every production admission path runs `check_provider_pins` before `admit_spirit` mints anything. The endpoint table holds, for every id in `maos_providers::PROVIDER_IDS`, the endpoint this boot is configured with (the literal, or `MAOS_OLLAMA_URL`) whether or not its credential is present — so the check means the same in live, record and keyless replay, and is captured before replay/record wrapping (prototype `pin` recorded only registered drivers; the dev widens it). Each `[providers]` entry's id must be in that table, `hex(sha256(<endpoint bytes>)) == provider_endpoint_pin` — the exact base URL the driver was built with, no normalisation (`printf %s <url> | sha256sum`) — and `endpoint_url`, when given, equals it; otherwise the typed refusal `endpoint_pin_mismatch` and the admission rolls back.
2. T0 classifies every `admit_spirit` caller (`admission.rs:903`; `main.rs:4461` shell; `:8238,9560,11734,14861`) as production or smoke/test; each production site checks or names in the record why it cannot carry `[providers]`.
3. Air-gap vector: a Spirit declaring `ollama` and pinned to `sha256("http://127.0.0.1:11434")` is admitted with `MAOS_OLLAMA_URL=http://127.0.0.1:11434` and refused with `MAOS_OLLAMA_URL=http://10.0.0.5:11434`.
4. **Failing-before (local):** at HEAD a manifest carrying a wrong 64-hex pin is admitted (C14). Witness `crates/maos-bin/tests/provider_endpoint_pin_20_4.rs` (door/run admission of a temp manifest; replay mode; no network).
5. **Plant (local, one):** skip the hash comparison ⇒ the wrong-pin vector admits ⇒ red.
6. Security reading at review: no path admits a `[providers]`-bearing Spirit without the check; the pin locks configuration, not network identity (TLS/DNS trust stays with ureq/webpki) — the operator doc says so. The refusal's contract is its code, not an FR63 catalog row (no gate reads that file, C8).

### L2 — kernel last

#### AC7 — Gemini and Kimi pass the rate-limit id set (D-20-4-06 A) ◆ FLAG-Winston

1. `bucket_key` (`inference/mod.rs:96-101`) interns `gemini` and `kimi` (`ka-intern-gemini-kimi.diff`); any other id still `Unconfigured` (fail-closed kept). Quotas default to 1000 rpm (`rate_limit.rs:314-319`) unless explicit `gemini`/`kimi` defaults are added in maos-providers.
2. **Failing-before (local):** `crates/maos-kernel-core/tests/inference_provider_ids_20_4.rs` (the `inference_fallback_authz_18_1.rs` harness shape): adapter with a rate limiter, a router holding `gemini` and a `ProviderInfer { provider: "gemini" }` token ⇒ `Err(Unconfigured)` at `a8af3215`; after: `Ok`, bucket key `gemini`; `bedrock` stays `Unconfigured`; every id in `maos_providers::PROVIDER_IDS` passes `bucket_key` (the two lists cannot drift).
3. **Plant (local):** drop the `gemini` arm ⇒ red.

#### AC8 — the kernel IO adapter never echoes a credential and never loses a status (D:1297, D:1298; D-20-4-07 A) ◆ FLAG-Winston

1. KIO1 (`kio1-badheader-redaction.diff`): a `BadHeader` transport error becomes `ureq POST <url>: invalid request header (value withheld)`. **Failing-before (local):** a kernel-core test sends a header value `sk-secret\u{1}tail` through `IoSubsystemAdapter::http_post_exchange` (to a loopback listener unless ureq validates before connecting, measurement 2) ⇒ at HEAD the error contains `sk-secret`; after, it does not.
2. KIO2 (`kio2-non2xx-truncate-keep-status.diff`): a non-2xx body over 10 MiB is cut at the cap and keeps its status; a 2xx over the cap is still `Decode`. **Failing-before (local):** a loopback listener answering `401` with 10 MiB + 1 bytes ⇒ HEAD `Err(IoError::Decode)`; after `Ok(HttpExchange { status: 401, body.len() == 10 MiB })`.
3. **Plants (local, two, named — each row has no other falsifier):** revert KIO1 ⇒ AC8.1 red; revert KIO2 ⇒ AC8.2 red.

#### AC9 — records

1. `tests/coverage-matrix.yaml:566-570` FR3 names AC1/AC2/AC5/AC6 witnesses; `docs/maos.dev/run-maos.md:75` names the new keys, `MAOS_DEFAULT_PROVIDER`, `[providers]` and the pin's hash definition and boundary.
2. At `done`: D:1297, D:1298 `CLOSED <date> by 20-4 @<commit>` with witness; `check-dev-record-completeness` passes; `ops-live-gemini-leg` note says L2 landed (it can record); epic corrections via `creation/records-owed.md` §A.

## Tasks / Subtasks

- [ ] **T0 — Preflight.** Operator rulings on D06 and D-20-4-01…11 recorded; tracker → `in-progress`. Run `creation/measurements.md` "Not measured here" 1–7 (item 6 classifies the admission sites; item 7 the vendor ids/prices). Re-measure every cite this story touches (rule 9). A measured figure above the ruled envelope, or a T0 result contradicting a ruled option, is STOP + D-row.
- [x] **T1 — Gemini driver (AC1.1–1.4).** `gemini.rs`, recording transport, five fixtures, round-trip test, plant.
- [x] **T2 — Kimi (AC2).** `compatible`, fixtures, test, plant.
- [x] **T3 — Keys, registration, default, credential check (AC3, AC1.5).** `SecretKey`, env rows, purge labels, builder, replay ids, selector, hello-spirit declarations, the two seed cassettes, `provider_selection_20_4.rs`, plant.
- [x] **T4 — Currency (AC4).** Data, price rows, scan extension, vectors, plant.
- [x] **T5 — Per-Spirit selection (AC5).** `[providers]` read, declared check, fallback refusal, binding sites, `admit_spirit` argument, manifest ids/tests, witness, plant.
- [x] **T6 — Endpoint pin (AC6).** Endpoint table, `check_provider_pins` at each production site, refusal codes, air-gap vector, witness, plant, operator doc.
- [ ] **T7 — L1 landing (D-20-4-08 A).** Exact pins inside the ruled envelope; `check-env-contract`, `check-model-currency`, `kloc-check`, `check-kernel-baseline` (unchanged), `check-service-boundary`, `check-corpus`, `check-dev-record-completeness`, `check-epic-close-coherence`, `prepush`; one hosted discipline run on the final tree + plants. Revert `intent-lineage-coverage-report.md` if a run modifies it. No commit or push without the operator.
- [ ] **T8 — Kernel id set (AC7)** under the ruled row: red-before test, KA, plant.
- [ ] **T9 — Kernel IO (AC8)** under the ruled row: two red-before tests, KIO1, KIO2, two plants.
- [ ] **T10 — L2 landing + records (AC9).** Both kernel pins + content hashes `inference/mod.rs`, `io/mod.rs` (re-base on 20-1b if it landed first); FLAG-Winston decision record (precedent `20-7-flag-winston-decision.md`); `cargo test -p maos-kernel-core --lib`; hosted run; ledger closures; coverage; docs.

## Dev Notes

### What changes, by file

| Area | Files | AC |
|---|---|---|
| Providers | NEW `crates/maos-providers/src/gemini.rs`; `src/openai.rs:18-165` (`compatible`, attribution); `src/lib.rs:8-28` (module, exports, `PROVIDER_IDS`); `src/anthropic.rs:25-51`, `src/openai.rs:26-52` (credential byte check); optional `src/rate_limit.rs:275-293`; NEW `tests/{gemini,kimi}_round_trip_test.rs`, `tests/fixtures/{gemini,kimi}_*.json` | AC1–AC3 |
| Domain | `crates/maos-domain/src/ports/secret_store.rs:10-78` | AC3 |
| Bin | `crates/maos-bin/src/main.rs:3896-4049` (registration, replay ids, selector, endpoint table), `:4374` (shell binding), `:890-904` (Butler), the four `AdmissionDeps` sites `:4334,5817,6701,10254`; `src/admission.rs:234-394` (refusals), `:440-465` (`GatedManifest`), `:590-723` (`gate_manifest`), `:778-790` (`AdmissionDeps`), `:903-919` (check + `providers` argument); `src/orchestrator_root.rs:221-237`; `src/env_contract.rs:84-93`; `src/purge.rs:333-340`; NEW `tests/provider_selection_20_4.rs`, `tests/provider_endpoint_pin_20_4.rs` | AC1, AC3, AC5, AC6 |
| Researcher (uncharged) | `spirits/researcher/src/lib.rs:487` | AC5 |
| Manifest | `crates/maos-manifest/src/manifest.rs:2413`, tests `:4325-4398` | AC5 |
| Kernel ◆ (L2) | `crates/maos-kernel-core/src/inference/mod.rs:96-101`; `src/io/mod.rs:65-78,86-136`; NEW `tests/inference_provider_ids_20_4.rs`, `tests/io_redaction_and_status_20_4.rs`; pins `xtask/kernel-core-baseline.toml:505` + EOF HISTORY, `xtask/kloc.toml:264` | AC7, AC8 |
| xtask / data | `xtask/src/check_model_currency.rs:18-26,239-295`; `xtask/model-currency.toml`; `xtask/provider-pricing.toml`; `xtask/tests/model_currency_gate.rs`; `xtask/kloc.toml` rows | AC4 |
| Manifests / cassettes | `spirits/hello-spirit/manifest.toml:14`; NEW `crates/maos-journey-test/cassettes/providers/{gemini,kimi}-shell-intro.json` | AC1, AC2, AC4 |
| Docs / records | `docs/maos.dev/run-maos.md:65-103`; `tests/coverage-matrix.yaml:566-570`; `deferred-work.md:1297-1298` | AC9 |

### Overlap (serialize; re-measure)

- **`maos-bin` `main.rs` builder and `admission.rs`** — 20-1b (yank policy in `TlYankObserver`, `main.rs:276-347`; admission identity), 20-8 (manifest fields at `manifest.rs:228-237,2423-2454` — the same `[providers]` region, E20:258), 20-13 (crypto selection at the same composition root). One integration owner per file per landing.
- **Kernel pins** — 20-1b's row (re-base rule above); 20-8/9/13/14 forecast kernel touches.
- **`provider-error-surfacing`** (S:225) owns real status for the existing drivers; AC1 gives Gemini real status without touching the others.
- **`provider-model-env-overrides`** (S:224) — D06.

### Current state that must be preserved

- Default provider of every existing box (registration order; Gemini/Kimi register only with their keys).
- 15-6 inference-mode lattice: explicit replay contacts no provider and answers every interned id (`main.rs:3998-4025`); a present-but-empty cassette stays present (`:3979-3982`).
- Fail-closed rate limiting for unknown ids (`inference/mod.rs:86-101`); capability check per call (`:250-272`); the fallback-id refusal (`:291-293`).
- 17-11: hello-spirit declares every provider the router can default to (`spirits/hello-spirit/manifest.toml:12-14`).
- 17-5: every model literal the gate scans stays scanned; `findings.is_empty()` never passes a gate that scanned nothing (`check_model_currency.rs:33-39`).
- 16-4: purge removes every MAOS credential via `SecretKey::ALL` (`purge.rs:333-340`); the keyring migration covers `ALL` (`main.rs:3827-3858`).

### Guardrails

- No kernel line without the ruled FLAG-Winston row; any `crates/maos-kernel-core/src` change outside AC7/AC8's shapes is STOP — including a new `ProviderError` variant (exhaustive kernel match `inference/mod.rs:357-365`).
- No vendor SDK (FR47 denylist); no network in any test (loopback listeners only); no paid key, no live call outside the operator leg.
- A credential never reaches a URL, an error string, a log line or the Transparency Log (`ProviderAttribution.endpoint_url` is journaled and enters cost frames, `inference/mod.rs:437-461`).
- Seed cassettes say `provenance: "seed"` and attribute `replay`; nothing in the tree claims a Gemini/Kimi live round trip until the operator leg records one.
- One cargo process per checkout; `env -u CI -u GITHUB_ACTIONS`; inherit the target directory, never override or clean it; plants byte-restored in the real tree (`proof-contract.md:24-31`). A manifest change widens the focused set (`proof-contract.md:48-53`): every reader of `spirits/hello-spirit/manifest.toml`, `gate_manifest`, `from_toml_str`, plus `spirit_boundary_invariants` and `check-corpus`.

### Proven-red controls (rule 11; proof contract)

| Control | Plant | Expected red | Leg |
|---|---|---|---|
| Key placement (AC1) | `?key=` in the URL | no-key-in-URL assertion | local |
| Kimi attribution (AC2) | `"openai"` literal restored | Kimi attribution assertion | local |
| Default selector (AC3) | env ignored | `infer:gemini->` assertion | local |
| Currency scan (AC4) | `Gemini` dropped from the regex | Gemini vector | local |
| Per-Spirit binding (AC5) | `primary` ignored | openai-binding vector | local |
| Endpoint pin (AC6) | hash comparison skipped | wrong-pin vector admits | local |
| Kernel id set (AC7) | `gemini` arm dropped | `Unconfigured` | local |
| Header redaction (AC8.1) | KIO1 reverted | secret in error | local |
| Status kept (AC8.2) | KIO2 reverted | `Decode` | local |

### Tests and commands

```bash
# local; one cargo process per checkout; inherit the target directory
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-providers -p maos-domain -p maos-manifest -p xtask
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-providers --test gemini_round_trip_test --test kimi_round_trip_test
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-bin --test provider_selection_20_4 --test provider_endpoint_pin_20_4
env -u CI -u GITHUB_ACTIONS cargo nextest run --locked -p maos-kernel-core --test inference_provider_ids_20_4 --test io_redaction_and_status_20_4   # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --lib                                                                     # L2
env -u CI -u GITHUB_ACTIONS cargo test --locked -p maos-kernel-core --test spirit_boundary_invariants                                         # manifest change
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-model-currency
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-env-contract
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-corpus
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-kernel-baseline
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- kloc-check --json
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-dev-record-completeness
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- check-epic-close-coherence
env -u CI -u GITHUB_ACTIONS cargo run -q --locked -p xtask -- prepush
```

### Review plan (`proof-contract.md:33-71`)

DIFFICULT: Blind + Edge (opus-5.5), Acceptance (opus-5.5), Security reading on AC1.1/AC1.4 (key placement), AC3.4,
AC6 (the pin's coverage and boundary) and AC8 (credential echo), Test Infrastructure (glm-5.3). Focused suites per
touched crate + the manifest-reader widening + `cargo test -p maos-kernel-core --lib` at L2; the reviewer reads the
plant record, never re-runs plants except for a disputed finding.

### Previous-story intelligence (20-1a, 20-7, 18-8, 18-1, 17-11, 17-5, 15-6)

- 20-1a: landed installed-Spirit dispatch and boot re-verification through `admission.rs`/`main.rs` (`850e4c00`, `8143326b`, `47631284`); its hosted run found faults local runs did not — run the hosted discipline once on the final tree before calling L1 done.
- 20-7: kernel landed at exactly the prototype's figure (26841 / 21042); the re-base rule with 20-1b is binding; a key rename broke two doc gates prepush does not run (`20-7-…md:374`) — this story renames no key.
- 18-8: closed D:1297's MCP half by refusing non-visible-ASCII bearers before the port (`maos-mcp/src/transport/mod.rs:60-63`, `streamable_http.rs:85`) — AC3.4's precedent; it named this key for the kernel half.
- 18-1: replay answers every interned id so a Spirit's own declared provider works keyless (`main.rs:4016-4024`) — extend, do not fork, that list.
- 17-11: adding `openai` to a manifest widened the focused set and the landing run found two reds the focused set missed (`proof-contract.md:48-53`).
- 17-5: the currency gate fails closed on a missing host constructor (`check_model_currency.rs:287-293`) — extend the host list, never relax it.
- 15-6: `MAOS_INFERENCE_MODE` resolves once after provider eligibility (`main.rs:3965-3991`); a live key marks a provider eligible (`env_contract.rs:84-93` purposes).

### Git intelligence

`a8af3215` (20-7 D-20-7-09/10/11 records), `47631284` (20-1a hosted-found fixes; aggregate 191826), `0836ff40`,
`03ee4a4e`, `8143326b` (20-1a review corrections). No commit since `1fd4de31` touched `crates/maos-providers`,
`inference/mod.rs` or `io/mod.rs`; `main.rs` is unchanged since `8143326b`.

### Library / framework (read 2026-11-02)

No new crate. Gemini REST: `POST https://generativelanguage.googleapis.com/v1beta/{model=models/*}:generateContent`,
request `contents[]`/`generationConfig`, response `candidates[].content.parts[].text`, `finishReason`
(`STOP`, `MAX_TOKENS`, `SAFETY`, …), `usageMetadata.promptTokenCount`/`candidatesTokenCount`
(`https://ai.google.dev/api/generate-content`); key header `x-goog-api-key`; current stable ids
(`https://ai.google.dev/gemini-api/docs/models`). Kimi: base `https://api.moonshot.ai/v1`, `POST /chat/completions`,
`Authorization: Bearer`, keys bound to their platform (`https://platform.kimi.ai/docs/api/overview.md`); ids via
`GET /v1/models` (`https://platform.kimi.ai/docs/api/list-models.md`). `ureq` 2.12.1 (locked), `sha2`/`hex`
already maos-bin dependencies (`crates/maos-bin/Cargo.toml:100-101`).

### References

E20@HEAD:7,49,55,59-61,77,201-209,380; preflight `epic-20-preflight-2026-10-30.md:63,79,123,139,227-231,285-286,290,295`;
tracker `sprint-status.yaml:224-225,302,342,344`; `deferred-work.md:1297-1298`; `routing-audit-2026-10-22.md:47`;
PRD `functional-requirements.md:26`; `tests/coverage-matrix.yaml:566-570`; `docs/runbooks/provisioning-checklist.md:42`;
ADR-005 (provider drivers), ADR-064 (replay selector), ADR-069 (`:35-46`).

## Dev Agent Record

### Landing 1 — resumed implementation, kernel-free (2026-11-03)

Agent: `openai-codex/gpt-6.1-sol` (`Dev204L1`). Initial baseline `16c35ef0`;
parked at the parent's request under `/tmp/204l1-wip`, with the tracked patch
checked against a `git archive HEAD` copy (no build). Resumed at **`8c809f9c`**
after 20-10's hosted corrections. The parked patch applied with only the stale
`xtask/kloc.toml` hunks excluded; those pins were re-cut against the new HISTORY,
preserving the 20-10 segments. No story/tracker status flip, commit or push.

**Rebased L1 grant (parent handoff):** maos-bin ceiling **34643** =
34191 + 145 × 2 + 54 × 3; aggregate ceiling **193526** = 192519 + 1007.
Other granted ceilings unchanged; kernel must remain 26841 physical / 21042 tokei.
Exact measurement after `cargo fmt --all`, via prebuilt `kloc-check --json`:

| Surface | Baseline -> formatted | L1 ceiling |
|---|---|---:|
| maos-providers | 1425 -> 1608 | 1898 |
| maos-bin | 34191 -> 34363 | 34643 |
| maos-domain | 10099 -> 10107 | 10115 |
| maos-manifest | 4604 -> 4604 | 4610 |
| xtask | 44406 -> 44420 | 44466 |
| aggregate | 192519 -> 192896 (hardfail 192897) | 193526 |
| kernel tokei | 21042 -> 21042 | 21042 |

Measurements: [final formatted JSON](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/kloc-final-formatted.json).
Before parking, `check-kernel-baseline --json` passed at 26841 physical lines,
95 files, unchanged content hashes and set hash `db39bc4a…`; kernel source was
never edited. The initial stop below is historical, superseded by D-20-4-12 B.
The new default is `kimi-k3.1-pro`; standard vendor prices, context and limits
were re-checked and recorded in [preflight](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/preflight.md).

Admission coverage is classified in [admission-sites.md](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/admission-sites.md).
The existing Researcher/Orchestrator resolver functions intentionally remain
unchanged: every production binding receives the selected singleton primary
from the root's `declared_providers` helper, or the previous declaration order.
Hello's token and implicit requests are bound together through `BoundInference`.

The hosted/live workflow is cut, **not pushed or executed**, at
[`ci/ci-20-4-verify.yml`](20-4-gemini-driver-and-endpoint-pin-evidence/ci/ci-20-4-verify.yml),
SHA-256 `8b6a3ab4a6e25cf0e22a81dc67ebf30ec6015b11f8da3afccb25082d360187f5`.
Live jobs require explicit operator opt-in after L2 and paid keys; no live call
is claimed. Local seed replay proves routing, not the vendor wire.

Execution evidence is append-only in [commands-resumed.json](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/commands-resumed.json);
earlier baseline/static-gate evidence remains in [commands.json](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/commands.json).

#### L1 observed proof

Per-AC `name:line`, independent before witnesses, all six mutation failures,
wall times, copy-back + `cmp` + `touch` restoration and final focused greens:
[plants.md](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/plants.md).
The runtime/scanner proof surfaces are byte-identical between the initial and
resumed HEADs (`head-rebase-diff.txt`); API compile reds were additionally
exercised against resumed HEAD. The literal epic shell pipeline exited 0;
its saved TL query shows `infer:gemini->replay:default` and a
`ProviderInfer { provider: "gemini" }` issuance (`epic-shell-command.json`).

| Exercised verification | Exit | Observed result |
|---|---:|---|
| Workspace `build --all-targets --locked`, final after affected test migrations | 0 | 5.33 s; `logs/all-target-build-final.log` |
| Provider wire tests (Gemini, Kimi, old OpenAI) | 0 | 8 passed; `logs/focused-provider-wire.log` |
| Selection / endpoint integration tests | 0 | 13 passed; `logs/focused-provider-selection.log` |
| Model-currency gate vectors | 0 | 23 passed; `logs/focused-model-currency.log` |
| Prescribed `spirit_boundary_invariants` | 0 | 1 passed; `logs/spirit-boundary-invariants.log` |
| Widened manifest-reader binaries and Spirit packages | 0 | 517 passed, 4 skipped; `logs/widened-manifest-readers.log` |
| Affected enterprise source-seam and purge receipt contracts | 0 | 24 passed; `logs/focused-affected-assertions.log` |
| Final serial workspace nextest, `--profile ci --test-threads 1 --no-fail-fast` | 0 | 5594 passed, 138 skipped, 642.290 s test span; `logs/workspace-serial-final.log` |
| Fresh serial `check-test-budget` | 0 | 598 binaries, 642.145 s sum, no findings; `workspace-serial-final-junit.xml`, `gates/final-check-test-budget.json` |
| Default-profile/default-concurrency workspace nextest | 0 | 5594 passed, 138 skipped, 43.100 s; `logs/workspace-default.log` |
| Non-denying all-target clippy JSON stream + advisory ratchet | 0 / 0 | 1331 occurrences / 871 source-distinct vs hosted baseline 1383 / 897; `clippy.jsonl`, `logs/clippy-ratchet.log` |
| `cargo deny --manifest-path /home/lunarpulse/dev_ws/Cargo.toml check` | 0 | Advisories, bans, licenses and sources OK; existing allowance/skip warnings retained in `logs/cargo-deny.log` |
| Structural/source and record gates (20 commands), plus fresh serial budget above | 0 | All requested gates and `check-corpus` passed; `gates/final-*.json` and exact commands in the audit |
| First prescribed prepush | 1 | 82 pass (including workspace-nextest), 8 conditional/CI skips, one advisory and one blocking tracked-tree red; `logs/prepush.json` |
| Corrective frozen-tree prescribed prepush | 1 | `tracked-tree` passed; workspace-nextest exited 100 (5584 passed, 9 failed + 1 timeout); 25 timing advisories; `logs/prepush-frozen.json`, `workspace-prepush-frozen-junit.xml` |

The first serial run found two genuinely affected assertions, not flakes:
the borrowed journal moved into `ProviderAdmissionContext<'a>`, and the purge
receipt now covers 12 slots (four provider keys, eight MCP bearers). Those
assertions were migrated, retaining the shared-journal/no-second-open invariant
and checking `SecretKey::ALL.len()`. The first migration also searched for a
function signature inside a body-only source slice; its focused failure is
retained in the command audit and corrected to inspect the context destructure.
No production behavior was weakened. The original 5592/5594 serial result and
JUnit remain as `logs/workspace-serial-initial.log` and
`workspace-serial-initial-junit.xml`; the final full serial run above is green.
An earlier all-target job hit the tool's 3000 s deadline; no numeric process
exit was supplied. Its unbounded continuation passed; no timeout is relabelled
as a source failure or a green result.

Observed host load: initially 110.84/112.68/88.24; before the first serial run
5.60/11.40/38.78; before default concurrency 3.01/3.64/11.28 (`uptime`).
No timing budget was widened. The independently prescribed default run passed;
the later frozen-tree prepush encountered the loaded failures recorded below.

The clippy stream contains 60 occurrences in assigned files, **none intersecting
added/changed lines and none in the new provider/test files**. The scoped
inventory and changed-line review are saved in `clippy-owned-diagnostics.json`
and `clippy-added-line-review.json`. The requested hosted-sensitive classes
(`result_large_err`, `collapsible_if`, `too_many_arguments`, `unused_mut`,
`trim_split_whitespace`) have no new-line finding locally; the compatible
constructor has six arguments and the governance context keeps its builder at
seven. Hosted compilation itself is unexecuted, not inferred green.

The first prescribed prepush took 4517.94 s. Its only blocking red was
`tracked-tree`: my tracked Dev Record/provisioning edits occurred while it was
running, violating the preserved-tree condition. This is retained as a failed
aggregate, not renamed green. `cmp` found the generated lineage report still
byte-identical to its committed baseline, so no report restoration was needed.
The separate test-budget advisory has six binary overruns and a 1157.060 s
suite sum against 1100 s; original JUnit is retained as
`workspace-prepush-initial-junit.xml`. Same-line rule-10 disposition is in
`deferred-work.md`'s 20-4 L1 section, owned by `20-7-stable-runner-perf-gates`;
budgets are unchanged and these advisories remain open regardless of a later
tracked-tree correction. After this run, `uptime` was 28.13/26.99/106.16.
Signed T3 substrate, `strace` network tracing and the Python Ed25519 backend
were unavailable for the named conditional legs; the prepush's
`output_diagnostics` preserves every SKIP rather than claiming those legs ran.

The corrective prepush was run with owned tracked writes frozen and took
523.95 s. It **passed the tracked-tree guard**; its only blocking red was the
workspace leg: 5594 cases, 5584 passed, nine failed and one timed out. Eight
belong to the named hook-pump, observer, one-shot-drain and shell-clarification
timing families; two are unattributed CLI fixture observations (503-busy saw
two round trips; a dead-door fixture got busy exit 75 instead of 69). The latter
are not asserted to be the older 411 family or a diagnosed production defect.
All ten have same-line rule-10 citations in the 20-4 section of
`deferred-work.md`, with `20-7-stable-runner-perf-gates` ownership; **no further
retry-to-green** was performed. Its 25 timing advisories (24 binaries plus
2909.207 s suite sum) include the 20-4 selection/pin test binaries; full values
are in `prepush-frozen-budget-findings.json`, with budgets unchanged.
After this run `uptime` was 114.19/107.29/97.97. The final prepush is
**not green**, despite the independently prescribed serial/default greens and
all focused AC witnesses passing. L1 implementation is present; the recorded
loaded-run failures remain verification limitations, not suppressed results.

#### L1 implementation file groups

- `maos-providers/src/{gemini,provider,anthropic,openai,lib}.rs`: native Gemini,
  reusable compatible attribution and shared non-echoing credential validation.
- `maos-domain/src/ports/secret_store.rs`, `maos-bin/src/{main,admission,env_contract,purge}.rs`:
  two logical keys, registration/default selection, immutable endpoint catalog,
  canonical admission and exact selected-provider request/token binding.
- `maos-manifest/src/manifest.rs`: five permitted provider ids; unsupported-id
  vectors use `bedrock`.
- Provider wire/regression tests, seven JSON fixtures, `tests/support/wire204.rs`;
  bin selection/pin tests and `tests/support/provider204.rs`; eight existing
  admission fixtures now explicitly pass no catalog, plus the enterprise seam,
  purge receipt and sealed-secret cardinality regressions.
- `spirits/hello-spirit/manifest.toml` and the two provider seed cassettes.
- `xtask/{kloc.toml,model-currency.toml,provider-pricing.toml}`,
  `xtask/src/check_model_currency.rs`, `xtask/tests/model_currency_gate.rs`.
- `docs/maos.dev/run-maos.md`, `docs/runbooks/provisioning-checklist.md`,
  `tests/coverage-matrix.yaml`, this L1 record and its evidence/CI recipe.
  The provisioning row now names both real key slots while retaining `absent`
  for paid live credentials. Kernel files and baseline pins are unchanged.
  The shared deferral ledger receives only the new 20-4 timing-advisory section;
  concurrent stories' rows are preserved.

#### L1 intermediary cleanup — standing operator scope

After all cargo work and final record gates, `/usr/bin/ps -eo comm=` (from
`/proc`) showed no cargo/rustc/nextest/clippy/xtask process before removal.
Pruned **2321** stale real `debug/incremental/<crate>/s-*` directories;
**5465** sessions were kept. Newest directory/content sessions and every
session with any entry touched within two hours were retained. Strict
pre-removal recency rechecks found no newly recent candidate and no errors.
No `deps`, `build` or fingerprint directories, other-agent scratch, whole-target
clean, or target-directory override was used.

| Allocated bytes (`du -s -B1`, both commands exited 0) | Before | After | Observed net freed |
|---|---:|---:|---:|
| `/mnt/build/cargo-maos` | 231836020736 | 213276917760 | **18559102976** (17.285 GiB) |
| `/tmp` | 1776513024 | 1408479232 | **368033792** (0.343 GiB) |

Five explicitly owned scratch paths were removed; six recorded preflight
homes/fixture roots were already absent. The parked git-archive copy and plant
backups were removed only after their wanted evidence was preserved here.
`/tmp/204l1-wip/{tracked.patch,new-files.tar.gz,SHA256SUMS}` remains intact
until the parent confirms the commit is on HEAD. A scoped `.orig`/`.rej` scan of
owned file neighborhoods found none. New logs/JUnit are retained, referenced
evidence; registered fixture outputs remain available. Unattributed T3 roots were not
deleted. Full paths, cutoff and errors are in `cleanup-plan.json`,
`cleanup-result.json` and `cleanup-proc-scan.json`; unrelated concurrent `/tmp`
activity can also affect its observed net delta.

T0's tracker/status action and T7's hosted run remain unchecked: status changes
and pushes are forbidden in this assignment. T8–T10 and ledger closures remain
L2 work. This is not a story-done, live-vendor, hosted-run or review-pass claim.


### Initial T0 stop — historical, superseded by D-20-4-12 B (2026-11-03)

Agent: `openai-codex/gpt-6.1-sol` (`Dev204L1`), baseline `16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de`.
**At this initial stop, no implementation was retained.** T0 requires STOP on a result contradicting a ruled option (task T0 above).
D-20-4-12 records the vendor notice and the unresolved policy interpretation; no replacement model, invented
price, release claim, or vendor round-trip has been substituted. The parent was notified.

**Clean boundary:** initial source edits were surgically reversed in full, including removal of the newly
created Gemini source. The edit tool returned each tracked file's original snapshot tag after reversal.
No kernel, CI branch, `.devcontainer` file, fixture corpus, pin, story status, or sprint status was changed.
The checked-in story still says `ready-for-dev`; this pass made no status flip, as assigned.

**Observed baseline, not final-tree proof:** the prebuilt `xtask kloc-check --json` exited 0:

| Surface | Measured at HEAD | Rebased L1 ceiling |
|---|---:|---:|
| maos-providers | 1425 | 1898 |
| maos-bin | 34188 | **34640** = 34188 + 145 × 2 + 54 × 3 |
| maos-domain | 10099 | 10115 |
| maos-manifest | 4604 | 4610 |
| xtask | 44406 | 44466 |
| aggregate | 192516 | **193523** = 192516 + (192833 − 191826) |
| kernel tokei | 21042 | 21042 (no L1 delta) |

Kernel physical baseline 26841 is inherited from the assigned context, **not re-measured in this pass**.
The only cargo invocation was the before witness:
`env -u CI -u GITHUB_ACTIONS cargo test --manifest-path /home/lunarpulse/dev_ws/Cargo.toml --locked -p maos-providers --test gemini_round_trip_test --test kimi_round_trip_test`
exited **101**, naming the missing Gemini test target; it compiled nothing.
This is AC1's missing-target red, **not** a Kimi attribution failure or a green-after result.
No plants ran (count 0, no plant wall-clock); no AC is claimed complete.

**Evidence:** [preflight and stop rationale](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/preflight.md),
[all shell commands and observed exits](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/commands.json),
[raw baseline](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/kloc-head.json),
[plant/witness disposition](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/plants.md).
`uptime` reported load averages 15.28 / 44.69 / 31.94.

**Unverified / remaining:** AC1–AC6 implementation and green-after witnesses, all six plants, AC9 L1 records,
live/hosted workflow diff and digest, fmt, final kloc measurement and exact repins, all-target build, focused
tests and widened manifest readers, serial/default workspace nextest, all requested gates, clippy counts,
cargo deny and prescribed prepush. Paid live legs and L2 remain outside this pass's delivered proof.
No push, commit, extra target directory or worktree was used.


### Landing 1 — Review corrections (2026-11-03)

Review base `ef134444`; operator-adjudicated P1–P3 and F1/F2/F4–F7 are
implemented. F3 is covered by P2. F8 and F9 are dismissed with the adjudicated
limiter-isolation and exact-base-URL/DNS/TLS reasons, not hidden or patched
outside scope. No kernel source, dependency, status, CI branch or hosted/live
cut changed; no commit or push was made.

**Behavior:** explicit blocked Gemini candidates may omit generated content
while preserving stop/usage/attribution; missing/empty finish reasons and
missing/invalid/overflowing usage are serde refusals. Gemini model ids are
ASCII `[A-Za-z0-9._:-]`, nonempty, and a request cannot change the admitted
model. AC1's old override allowance is explicitly superseded by F1.
Gemini/OpenAI/Kimi vendor stop labels are bounded at 64 UTF-8 bytes; provider
error text uses a control-free 512-byte source prefix, including Display.
Gemini's non-2xx status stays typed data; kernel error mapping is unchanged.

**Registration proof:** supported live-mode EOF boots, with no cassette or
inference, prove the respective new key actually registers its native id and
the absent id is refused. Anthropic/OpenAI precedence has separate conflicts.
Replay is explicitly selected and labelled routing-only. Both final
map-insertion omission plants yield native-registration red / replay green;
all restores used copy-back + `cmp` + `touch`, rc 0. Original two-red replay
observations and the invalid-mode draft are retained, not relabelled.

**Pin/currency maintenance:** five shared endpoint/model tuples feed real root
constructors and the admission catalog; Ollama's configured URL is read once.
The capturing-transport unit covers all five driver endpoints. Currency
scanning resolves crate-qualified, attribute-free root tuple constants through
existing `syn`, preserves declaration-line diagnostics and deduplicates shared
pins. Missing/unqualified/foreign references, commented definitions and
inactive cfg definitions have fail-closed witnesses. No new dependency was
needed. Operator docs now explicitly state that endpoint pins are opt-in.

**Observed verification:** provider package serial 64 passed / 3 existing
ignored, rc 0; final currency serial 25/25, rc 0; five affected root binaries
serial 46/46, rc 0; final complete workspace default inventory **5613 passed /
138 skipped**, rc 0. The final workspace used supported nextest build-metadata
reuse after genuine CLI fixture builds: ordinary workspace builds had
restored an older cached `maosctl` alias and triggered nine journey freshness
failures. A second non-green run also retained an old embedded KLOC pin.
Those prerequisite failures, their real repairs and rejected CLI flag drafts
are preserved; no timing flake was observed or retried to green.

The final 18 structural gates all returned rc 0, including kernel/service,
environment, serde, composition, J1, records/coherence, test-cost, invariant,
coverage, exits/decisions, performance policy and model currency (22 pins /
17 files). Final non-denying all-target workspace clippy and its ratchet
returned rc 0: **1331 occurrences / 871 source-distinct**, unchanged from prior
local debt and 52/26 below hosted baseline1383/897. No new correction-hunk
warning or suppression was introduced. `cargo deny` returned rc 0 with existing
allowance/duplicate warnings. Recorded host load at 18:28:47 UTC: 2.11/2.53/2.77.

| Formatted measure | Review base | Final | Ceiling |
|---|---:|---:|---:|
| maos-providers | 1608 | 1679 | 1898 |
| maos-bin | 34363 | 34504 | 34643 |
| xtask | 44420 | 44452 | 44466 |
| maos-domain | 10107 | 10107 | 10115 |
| maos-manifest | 4604 | 4604 | 4610 |
| aggregate | 192896 | 193140 | 193526 |

Exact pins and first-segment `20-4 landing 1 review corrections` HISTORY are
updated; aggregate hardfail193141 = measured193140 + 1. Kernel remains
26841 physical /21042 tokei, all 95 hashes unchanged. No envelope was breached.

**Evidence:** [dispositions and named red/green witnesses](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/review/corrections/disposition.md),
[plants/restores](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/review/corrections/plants.md),
[verification and build-once protocol](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/review/corrections/verification.md),
[exact commands/exits](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/review/corrections/commands.md),
[changed paths](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/review/corrections/files-changed.txt).
Raw logs, both final JUnits, clippy events and hashed metadata inputs are retained
in that correction directory.

**Cleanup:** after the final gates, a `/proc` executable scan found no
Cargo/rustc/nextest/clippy process and no unreadable live pid. Removed 87 stale
incremental `s-*` sessions, preserving the newest per crate and all descendant
mtime/ctime activity within two hours, plus only owned `/tmp/204l1-review`.
Observed `du -s -B1` decreased cargo-maos222472871936 →221931663360
(**541208576 bytes**) and tmp1428324352 →1418465280 (**9859072 bytes**).
No deps/build/fingerprint directory was touched. The three protected
`/tmp/204l1-wip` archive files remain; [cleanup.json](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/review/corrections/cleanup.json)
records the exact removed sessions and process/recency proof.

**Not re-run in this review scope:** full serial workspace nextest, full
prescribed prepush, paid vendor calls, hosted execution/hosted-only clippy,
and all Landing 2 kernel work. Earlier non-green prepush records remain intact.

### Landing 1 — Hosted-1 regressions (2026-11-04)

Baseline supplied by Main: clean HEAD `76efc19d`. Discipline run `41815429165`
identified two regressions introduced by correction commit `73e7afbc`, not
record-only failures or the registered timing-flake families.

**Network boundary (job126524848874):** the required
`cargo test -p maos-bin --no-default-features --test verb_table_15_3 -- --test-threads=1`
was exercised before edits: rc101, E0433 at the catalog helper's return and
constructed admission types. `main.rs:16802` now gates that helper on `network`;
`:16825` gates its provider-only unit module on `all(test, network)`.
Shared tuple constants remain attribute-free, and lib.rs/CI controls are
unchanged. The same negative-feature control is now **7/7, rc0**.
Default-feature verb control plus the catalog unit are **8/8 serial, rc0**.

The full L1/correction diff was read and grep-audited for other optional
provider/admission symbols. Native all-target checks with `network`,
`fixture_replay`, and `network,wasm-host` each returned rc0.
The additionally requested no-default all-target check returned **rc101** in
untouched `eval_halt_18_2.rs:26,353,357` (ungated cassette/provider test imports,
outside both 20-4 diffs). Main was notified; that older test was not modified or
excluded. The aarch64-apple-darwin workflow target was not cross-compiled here.

**Model-currency cost (job126540412403):** hosted evidence was
30.601 s over 25 tests, exceeding the unchanged 30.000 s sum budget.
`check_model_currency.rs:238` now lexes root items, discards fn/impl/mod brace
groups before syn sees their bodies, and prefix-checks candidates before
constructing only `ItemConst` ASTs. Root visibility, original declaration spans,
attribute refusal, crate-qualified binding, comment/cfg/shadow controls and
fresh per-audit inputs are retained; no cache or budget exception was added.

The new `shared_default_scan_skips_unrelated_item_bodies_hosted_1:672` witness
was observed red before this cutover (rc101); the final serial suite is
**26/26, rc0**, retaining every original test name. Fresh JUnit sums were
13.640 s /25 before and **15.879 s /26 after**, below the unchanged 30 s budget.
Local wall sums did not improve and are not presented as a speedup.
Five controlled audit profiles measured mean user CPU
0.5305912 →0.4310398 s (**18.762% lower**), mean wall
0.5437248 →0.4401365 s. This confirms a real AST cost, not that it was the
majority of audit cost or that a hosted rerun has passed. A premature stale
measurement draft and intermediate attempts are explicitly disqualified/
retained rather than substituted for final evidence.

**Verification:** fmt completed; final 19 structural/record gates returned rc0.
Non-denying full workspace/all-target clippy and the unchanged ratchet returned
rc0: **1331 occurrences /871 source-distinct**, unchanged from prior local debt
and below hosted1383/897. No new diagnostic was emitted on the hotfix code.
No full workspace nextest/prepush, paid vendor leg or hosted rerun was claimed.

**Exact formatted pins:** bin34551 →34552 (+1, ceiling34643);
xtask44452 →44466 (+14, **at ceiling44466**); aggregate193187 →193202 (+15,
ceiling193526), hardfail193203 = measured +1. Kernel remains26841/21042 with
all95 hashes unchanged. No envelope was exceeded.

**Cleanup:** after all gates, `/proc` verified no Cargo/rustc/nextest/clippy
process. Only28 stale incremental sessions were removed; newest5929 sessions
and625 additional recent sessions were retained. Target bytes
222600966144 →221768646656 (**832319488 freed**); `/tmp`
1453056000 →1453056000 (0 freed; no new owned scratch remained).
The protected WIP archive trio remains. No deps/build/fingerprint directory
was deleted. Final host load was89.62/97.98/100.20; exact cleanup metadata is
in `landing-1/hosted-1/cleanup.json`.

Evidence: [network audit](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/network-audit.md),
[measured performance](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/performance.md),
[commands/exits](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/hosted-1/commands.md),
raw logs, both fresh currency JUnits, timing/profile JSON and the clippy stream
under `landing-1/hosted-1/`. No commit, push, kernel change or status flip.

### Story context creation

Created 2026-11-02 by `anthropic/claude-opus-5-5` (bmad-create-story, lighter measured contract) at `a8af3215`,
without cargo (parent rule).

**Inputs:** E20 §20-4 at HEAD (amended 2026-10-30, PF §3.4) — stale cites corrected here and owed to E20
(`creation/records-owed.md` §A); preflight D06 and §2/§3.4 rows; tracker `:224-225,302,342,344`; D:1297-1298; FR3;
R:47; direct reads of every cited range (`creation/cites-a8af3215.md`); kloc and kernel by the prebuilt gate binary on
`/tmp/proto204` `git archive` copies (`creation/kloc-kernel-a8af3215-and-prototypes.json`); prototypes KA, KB, KIO1,
KIO2, R, R′, G1, K1, SEC, SEL, PIN saved as diffs (`git apply --check` clean), `rustfmt`'d, never applied to the tree;
the AC1 shell line run on the prebuilt binary (`creation/runtime/`); vendor docs read 2026-11-02.

**Measurements owed:** `creation/measurements.md` "Not measured here" 1–7.

**Records owed:** `creation/records-owed.md` (§A epic corrections; §B ledger; §O operator prerequisites; rulings; landing).

**Revert:** no crate source modified (prototypes only under `/tmp/proto204`, deleted); no cargo/test ran;
`intent-lineage-coverage-report.md` untouched.

**Gates after the record edits** (prebuilt `/mnt/build/cargo-maos/debug/xtask`, `env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=…/xtask`, `--json`, in-tree; `creation/gates/*.json`): `check-dev-record-completeness` rc 0 (197 done stories, 167 deferred-owner assertions, 0 violations); `check-epic-close-coherence` rc 0 (21 epics, pin 26841, 0 violations); `check-decision-register` rc 0 (23 rows, 0 findings); `check-exit-commands` rc 0 (2 blocks, 10 tokens, 0 findings); `check-kernel-baseline` rc 0 (26841 / 95 files, set hash `db39bc4a…`). `kloc-check` in-tree rc 1 — `maos-wasm-host 2420 > 2412`, aggregate 191834: an uncommitted sibling edit to `crates/maos-wasm-host/src/runner.rs`, not this creation (no crate source touched here); the HEAD `git archive` copy measured rc 0, aggregate 191826 (`creation/measurements.md`).

### Agent Model Used

anthropic/claude-opus-5-5 (creation).

### Debug Log References

Exact commands/exits: [command-results.md](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/command-results.md).
Per-AC red/green and restoration: [plants.md](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/plants.md).
Full outputs and JUnit are retained under the adjacent `logs/` and landing directory.

### Completion Notes List

AC1–AC6 are implemented and their focused witnesses are green. Kernel/L2,
paid vendor calls and hosted execution are not claimed. The L1 section above
records both green full matrices and the non-green prescribed prepush; no
status flip, commit or push was made.

### File List

[Owned paths only](20-4-gemini-driver-and-endpoint-pin-evidence/landing-1/files-changed.txt);
concurrent stories' files are excluded and the shared-ledger hunk is identified.

## Change Log

- 2026-11-02: Story created at `a8af3215` (ready-for-dev; D06 open, D-20-4-01…11 PROPOSED). Kernel-Δ 0 disproved: the kernel's provider-id set refuses Gemini/Kimi under the rate limiter and D:1297/D:1298 are kernel rows owned here — R = KA + KIO1 + KIO2 priced +13 / +9 by reverted prototype. Measured: the epic's AC1 line exits 1 and the selector is ignored (`infer:anthropic->replay:default`); the currency gate passes with unscanned Gemini/Kimi pins; `[providers]` and its pin are never read.
- 2026-11-02: Operator rulings recorded (ask s204): D06 D and D-20-4-01…11 as recommended; kernel R = KA + KIO1 + KIO2 and ceilings per frontmatter. Status remains ready-for-dev.
- 2026-11-03: L1 T0 STOP recorded as D-20-4-12 after the official Kimi notice restricted K2 access and announced retirement; incomplete source edits reversed, baseline envelope and missing-target witness preserved. No status flip or L1 completion claim.
- 2026-11-03: D-20-4-12 B selected `kimi-k3.1-pro`; L1 AC1–AC6 implemented, six restored plants and final local matrices recorded, exact rebased pins landed with kernel unchanged. Hosted/live recipe cut with SHA-256 but not pushed. Loaded prepush failures and unchanged-budget advisories retained under rule 10; no story status flip or L2 closure.
