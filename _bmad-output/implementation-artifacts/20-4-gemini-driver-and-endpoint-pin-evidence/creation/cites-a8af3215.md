# 20-4 creation — cites re-measured at `a8af3215` (rule 9)

Every range below was re-read at HEAD `a8af3215` (`git log -1 -- crates xtask` = `47631284`). "E20" =
`_bmad-output/planning-artifacts/epics/epic-20-ship-it-w5.md`; "S" = `sprint-status.yaml`; "PF" =
`epic-20-preflight-2026-10-30.md`; "D" = `deferred-work.md`; "R" = `routing-audit-2026-10-22.md`. The shared files
(E20, PF, D) are NOT edited by this creation except the one new D row in `records-owed.md` §B; the epic corrections
are owed through `records-owed.md` §A (orchestrator applies). The story cites the NEW values.

## C — epic §20-4 (E20:201-209), its stories row (E20:77), kloc row (E20:55), dependencies (E20:380)

| # | Where | OLD | NEW at `a8af3215` |
|---|---|---|---|
| C1 | E20:207 | `openai_round_trip_test.rs:1-30` (the `MockTransport` model) | file is 91 lines; `MockTransport` `:15-29`, request `:31-40`, three fixture tests `:42-91`; it returns a canned body and records neither URL nor headers — a Gemini test that must prove "no key in the URL" needs a recording transport |
| C2 | E20:207, :55 | `anthropic.rs` "307 lines" | 427 physical lines (`wc -l`); `complete` `:75-102`, `options.model_id` honoured `:79` |
| C3 | E20:207 | providers `src/` "1570 lines" | 1768 physical (`wc -l crates/maos-providers/src/*.rs`); governed tokei **1425** (`kloc-check`, pin `xtask/kloc.toml:427` = 2000) — PF:227 already had 1425 |
| C4 | E20:207, :208 | registration `main.rs:3081-3100`, adapter `:3136` | builder `crates/maos-bin/src/main.rs:3896-3963` (`default_id` first-registered `:3899,3910,3929,3953`; no-provider placeholder `:3959-3963`); replay interned ids `:4020-4024`; router `:4047-4049`; adapter `:4073-4085`; per-Spirit `port_builder` `:4089-4114`; shell default `:4374` |
| C5 | E20:205 (PF:228) | shared builder `:3818-3880,3992-4022` | `:3896-3963` (registration) and `:3993-4045` (15-6 replay/record wrapping) |
| C6 | E20:205, :208 | manifest primary/pin `manifest.rs:2423-2427,2454,2510-2522`; E20:208 `:2103`, `:2159-2171` | `crates/maos-manifest/src/manifest.rs`: `ALLOWED_PROVIDER_IDS = ["anthropic","openai","ollama"]` `:2413`; `ProvidersSection` `:2423-2428`; `ProviderConfig` `:2445-2455` (pin field `:2454`); id check `:2494-2501`; pin 64-hex check `:2510-2516`; test asserting `kimi` unsupported `:4325-4337` |
| C7 | E20:208 | ABI test `maos-spirit-abi/src/compliance.rs:507` | `:508` (PF:229 already moved it); struct `ProviderEndpointPin` `:258-267` |
| C8 | E20:208 | mismatch is a typed refusal "governed by `xtask/fr63-typed-errors.toml`" | the catalog has **no consumer**: `git grep` outside `_bmad-output` finds only its own header naming `check-fr63` (`xtask/fr63-typed-errors.toml:3`), which is not a subcommand (`xtask/src/main.rs` has none); its rows are registry errors only. A row there is documentation, not a control (rule 11). The refusal's real governance is its typed `AdmissionRefusal` code and the AC6 plant |
| C9 | E20:208 | "never routed through `inference/mod.rs`. kernel-Δ 0." | the pin itself needs no kernel line (admission, maos-bin). **But kernel-Δ 0 is false for the story:** `InferencePortAdapter::bucket_key` interns a closed set `"anthropic" | "openai" | "ollama"` and returns `Unconfigured` for any other id (`crates/maos-kernel-core/src/inference/mod.rs:91-109`, match `:96-101`); the limiter is installed in every mode except explicit replay (`crates/maos-bin/src/inference_mode.rs:95-97`, `main.rs:4079-4083`) — so a live or `record` Gemini/Kimi call is refused `Unconfigured` at HEAD. And D:1297/D:1298 assign two kernel IO repairs to this key (P3) |
| C10 | E20:209 | currency file + gate "created by the hotfix-lane row … `sprint-status.yaml:223`; `ls xtask/*.toml` = 24 files, none named so; until it lands the id is a literal in `gemini.rs` with a `// model-currency` marker" | closed by promotion: `17-5-model-pin-currency-gate: done` (S:266; S:223 `done`); `xtask/model-currency.toml` 137 lines (providers anthropic/openai/ollama only); gate `xtask/src/check_model_currency.rs` (subcommand `xtask/src/main.rs:438`, dispatch `:1395`); `xtask/*.toml` = 28 files. **The gate cannot see a Gemini or Kimi pin:** its `main.rs` scan matches only `(Anthropic|OpenAi|Ollama)Provider::(new|with_api_key)(` (`check_model_currency.rs:241-243`) and requires exactly those three hosts (`:287-293`). Measured: prototype `sel` (Gemini + Kimi constructors with ids absent from the data file) → gate rc 0, 16 pins, 0 findings — identical to HEAD (`measurements.md` §Gate blindness) |
| C11 | E20:209 | Bedrock/Vertex/Vault `sprint-status.yaml:294` | `later-bedrock-vertex-and-kms-backends` is S:344 |
| C12 | E20:207 | `provider-model-env-overrides` `sprint-status.yaml:224` | same line, still `backlog` (S:224; charter `tracker-notes-archive-2026-09-04.md:713-715`) |
| C13 | E20:207 | "`MAOS_DEFAULT_PROVIDER` does not exist in code (grep 0)" | still 0 (`git grep MAOS_DEFAULT_PROVIDER -- crates xtask docs`). **Measured at runtime:** set to `gemini`, the shell runs and the Transparency Log row reads `infer:anthropic->replay:default` (`runtime/selector-probe-a8af3215.txt` S2) |
| C14 | E20:207 | "the manifest `[providers].primary` route (`manifest.rs:2103`) is not used for selection" | stronger: **no production path reads `[providers]` at all** — `gate_manifest` (`crates/maos-bin/src/admission.rs:590-723`) parses no `providers` section, `GatedManifest` (`:440-465`) has no field, and the production admission passes `None` for `admit_spirit`'s `providers` argument (`admission.rs:903-919`, the `None` at `:917`). The kernel's `ProviderSwitched` journal (`crates/maos-kernel-core/src/security/mod.rs:417-448`) is therefore fed by tests only (`crates/maos-kernel-core/tests/provider_switched_journal.rs`). A pin written in a Spirit manifest today is silently ignored |
| C15 | E20:207 | cassette `crates/maos-journey-test/cassettes/providers/gemini-shell-intro.json` (new) | absent, as is the `providers/` directory; the epic line exits **1** at HEAD (`runtime/selector-probe-a8af3215.txt` S1). Cassette schema `maos.journey.cassette/v1`, provenance `seed|live-record` (`crates/maos-bin/src/cassette_replay.rs:45-52`); model `crates/maos-journey-test/cassettes/j0/shell-intro.json` |
| C16 | E20:207 | the shell line "replies from the cassette" proves the driver | replay replaces every driver with the cassette provider before the router is built (`main.rs:3998-4025`): the line proves selection, id routing and the minted scope, never the Gemini wire shape — that is AC1's round-trip test. The story labels this |
| C17 | E20:77, :203 | "kernel-Δ 0 · maos-providers +300–400 · maos-bin +40–80" | measured at creation (`measurements.md`): kernel R = KA + KIO1 + KIO2 **+13 physical / +9 tokei** (PROPOSED); maos-providers **+169 measured** (+10…+45 forecast); maos-bin **+145 measured** (+24…+54 forecast); maos-domain **+8 measured**; maos-manifest 0…+2, xtask +5…+20 forecast |
| C18 | E20:55 | kloc ask "maos-providers +300–400 … against +400"; "20-4 provider registration + pin check +40–80" | as C17. The providers numeric pin 2000 has 575 room and the recommended ceiling (1898) fits it; the aggregate has 1 line of room (191826 / 191827) |
| C19 | E20:205 | "Model currency completed by 17-5 (`xtask/model-currency.toml:1-25`, `xtask/src/main.rs:446-449`)" | data `:1-41` header, `[providers.*]` `:42-137`; subcommand `xtask/src/main.rs:438` |
| C20 | E20:205, :207 | `gemini-rs` denial `fr47-vendor-sdk-denylist.toml:17` | same (`:16` also denies `google-generative-ai`) |
| C21 | E20:49; S:342 | `ops-live-gemini-leg` records the cassette under `MAOS_INFERENCE_MODE=record` | record mode installs the rate limiter (`inference_mode.rs:95-97`) → `bucket_key` refuses `gemini` (C9): the leg needs this story's kernel landing (L2), not only L1 |
| C22 | E20:380 | dependencies: "hotfix-lane rows `provider-model-env-overrides` … and `model-pin-currency-gate-and-retired-pin-sweep` … before 20-4" | the second is closed (S:223, S:266); the first is D06 (open) |
| C23 | E20:205 | "no pin consumer found in scoped search" | same, and the ABI's other endpoint binding is vacuous too: registry admission's FR38 runtime drift check defaults `runtime_provider_endpoint` to the manifest's own value (`crates/maos-registry/src/admission.rs:138-160`) and both production callers pass `None` — `maosctl spirit install` (`crates/maos-registry/src/install.rs:50-59`) and `maosctl import` (`crates/maos-cli/src/subcommands.rs:1402-1410`); the `main.rs:8944,9013,14308,14564` callers are smoke arms → D-20-4-10 |
| C24 | E20:205 | "Include FR3 Kimi and per-Spirit provider selection" (D01 A) | Kimi's API is OpenAI-compatible at base `https://api.moonshot.ai`, path `/v1/chat/completions`, `Authorization: Bearer` (`https://platform.kimi.ai/docs/api/overview.md`, fetched 2026-11-02) — the existing driver's URL shape (`openai.rs:81`) fits unchanged; only attribution (`openai.rs:160`) and the secret key (`:38`) are OpenAI-fixed |

## T — tracker

| # | OLD | NEW |
|---|---|---|
| T1 | S:302 "provider_endpoint_pin enforced in adapter" | enforced at admission (D-20-4-05 A, maos-bin), not in the kernel inference adapter |
| T2 | S:342 `ops-live-gemini-leg` | needs L2 (C21); Kimi under D-20-4-09 |
| T3 | `docs/runbooks/provisioning-checklist.md:42` "no credential name exists in the tree" | true at HEAD; this story creates `MAOS_GEMINI_API_KEY` (and `MAOS_KIMI_API_KEY`) — operator row owed at landing |

## P — preflight

| # | OLD | NEW |
|---|---|---|
| P1 | PF:123 kernel "No; 0 at out-of-kernel construction/adapter boundary" | false (C9); measured row R +13 / +9 |
| P2 | PF:79 "+340–480 excludes Kimi, per-Spirit choice, override dependency; bin upper 80 > inherited 71 room" | measured/forecast providers +179…+214, bin +169…+199, domain +8, manifest 0…+2, xtask +5…+20, kernel +9 tokei (C17) |
| P3 | PF:228-231 (no kernel IO items) | D:1297 (ureq `BadHeader` echoes a provider credential) and D:1298 (non-2xx over 10 MiB loses its status) are **owned by this key** ("Owner: `20-4-gemini-driver-and-endpoint-pin`", D:1297, D:1298) and are kernel lines (`crates/maos-kernel-core/src/io/mod.rs:65-78,123-126`) — they were not in the epic section or the preflight |

## L — PRD / audit lines used

| # | Cite | Content at HEAD |
|---|---|---|
| L1 | `prd/functional-requirements.md:26` | FR3: "Operator can configure provider drivers (Anthropic, OpenAI, Gemini, Kimi, local-LLM via Ollama, air-gapped Bedrock) per Spirit, including locking provider endpoints for air-gapped deployment." |
| L2 | R:47 | `20-4` ← "FR3 Kimi + per-Spirit provider selection" |
| L3 | `tests/coverage-matrix.yaml:566-570` | FR3 `gates: []`, `corpora: []`, phase v0.5 |
| L4 | `docs/maos.dev/run-maos.md:75` | live keys named: Anthropic and OpenAI only |
| L5 | `spirits/hello-spirit/manifest.toml:14` | `provider.complete = ["anthropic.claude-haiku-4-5-20251001", "ollama.llama3.1:8b", "openai.gpt-4o-mini"]`; the shell mints `ProviderInfer { provider: router default }` (`crates/maos-bin/src/shell_host.rs:112-122`) and issuance refuses an undeclared provider (manifest comment `:12-13`) — a `gemini` default needs a `gemini.<id>` declaration |
