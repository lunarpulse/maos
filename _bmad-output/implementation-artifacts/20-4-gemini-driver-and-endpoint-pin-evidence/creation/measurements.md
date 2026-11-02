# 20-4 creation — measurements (2026-11-02, HEAD `a8af3215`)

No cargo ran (parent rule: another agent owns cargo; inherited `CARGO_TARGET_DIR=/mnt/build/cargo-maos`, never set,
no build, no test). No crate source in the tree was modified: every prototype was made on a `git archive HEAD` copy
under `/tmp/proto204/<name>/` (deleted after measurement), every changed file `rustfmt --edition 2021` (rustfmt
1.9.0; the repo has no `rustfmt.toml`), then measured with the prebuilt gate binary **from that copy's root**:
`env -u CI -u GITHUB_ACTIONS CARGO_MANIFEST_DIR=/tmp/proto204/<name>/xtask /mnt/build/cargo-maos/debug/xtask {check-kernel-baseline,kloc-check} --json`
(tokei inside `kloc-check`). Every `.diff` here passes `git apply --check` against the working tree. Raw results:
`kloc-kernel-a8af3215-and-prototypes.json`. Prototypes were not compiled ([INFERENCE] that they build).

## Instrument replication

| Figure | Method | In-tree and `/tmp` copy `base` | Pin |
|---|---|---|---|
| Kernel physical | `check-kernel-baseline --json` | **26841**, 95 files, set hash `db39bc4a…`, changed/added/removed `[]`, rc 0 | `xtask/kernel-core-baseline.toml:505` |
| Kernel tokei | `kloc-check --json` | **21042** | `xtask/kloc.toml:264` |
| Per-crate tokei | `kloc-check --json` (`passed=true`, `alarm=true`, `over_budget=[]`, rc 0) | maos-providers **1425** · maos-bin **33498** · maos-domain **10099** · maos-manifest **4604** · maos-secrets **536** · xtask **44406** · aggregate **191826** | `:427` (2000) · `:517` (exact) · `:425` (exact) · `:415` (exact) · `:480` (1000) · `:339` (exact) · `_aggregate_hardfail` `:718` = 191827 |

Aggregate room: **1 line**. maos-providers (575) and maos-secrets (464) have numeric pin room, but under D02 A every
added line needs the aggregate grant.

## Kernel-touching options, priced by throwaway prototype

| # | Prototype (D-row) | Diff | Kernel physical | Kernel tokei | Also |
|---|---|---|---:|---:|---|
| KA | `bucket_key` interns `"gemini"` and `"kimi"` (two match arms) (D-20-4-06 A) | `ka-intern-gemini-kimi.diff` | **+2** (26843) | **+2** (21044) | — |
| KB | the closed set moves to `maos_providers::PROVIDER_IDS` / `intern_provider_id`; the kernel calls it (D-20-4-06 B) | `kb-intern-set-in-providers.diff` | **−4** (26837) | **−4** (21038) | maos-providers +3 |
| KIO1 | `http_post_exchange` maps `ureq::Error::Transport` of kind `BadHeader` to "invalid request header (value withheld)" (D:1297; D-20-4-07 A) | `kio1-badheader-redaction.diff` | **+7** (26848) | **+5** (21047) | — |
| KIO2 | `read_capped(.., truncate)`: a non-2xx body over 10 MiB is cut at the cap and keeps its status; a 2xx over the cap still errors (D:1298; D-20-4-07 A) | `kio2-non2xx-truncate-keep-status.diff` | **+4** (26845) | **+2** (21044) | — |
| **R = KA + KIO1 + KIO2 (recommended)** | | `r-ka-kio1-kio2-bundle.diff` | **+13 (26854)** | **+9 (21051)** | changed files `inference/mod.rs`, `io/mod.rs` |
| R′ = KB + KIO1 + KIO2 | | `rprime-kb-kio1-kio2-bundle.diff` | +7 (26848) | +3 (21045) | maos-providers +3; aggregate +6 |
| Rmin = KA only (D-20-4-07 C) | | `ka-intern-gemini-kimi.diff` | +2 (26843) | +2 (21044) | D:1297/1298 re-routed |
| none (D-20-4-06 C) | Gemini/Kimi reachable in explicit replay only; live and `record` refused `Unconfigured` | — | 0 | 0 | FR3 live false; `ops-live-gemini-leg` cannot run |

Every kernel shape moves both pins (`src_lines` at `kernel-core-baseline.toml:505`, the content hashes of the changed
files, `kloc.toml:264`). ADR-069 (`docs/adr/ADR-069-kernel-core-envelope-through-v1.md:35-46`): R reaches 21051
tokei, 949 under the 22000 alarm; its ×2 ceiling (21060) is 940 under. **Interaction with 20-1b** (R = KQ1 + P,
+78 / +68, granted from 26801 / 21011, ceilings 26957 / 21147, S:297): both remain unlanded against today's
26841 / 21042 (20-7's +40 / +31 landed). Whichever of 20-1b and 20-4 lands second re-bases: expected cumulative
21042 + 68 + 9 = **21119** tokei; cumulative ceilings 21042 + 136 (20-1b allowance) + 18 = **21196**, inside the
21209 cap ruled with 20-7/20-1b (791 under 22000); physical 26841 + 78 + 13 = 26932 expected.

## Out-of-kernel, priced by prototype

| # | Prototype | Diff | Measured |
|---|---|---|---:|
| G1 | `crates/maos-providers/src/gemini.rs`: native `generateContent`, key in `x-goog-api-key` (never the URL), `http_post_exchange` with non-2xx → `ProviderRejected { status, body }`, multi-part text concatenation, `finishReason`/`usageMetadata` mapping, `options.model_id` honoured; lib exports. No inline tests (fixtures in `tests/`, uncharged) | `g1-gemini-driver.diff` | **maos-providers +142** |
| K1 | `OpenAiProvider::compatible(provider_id, secret_key, …)`: Kimi = OpenAI-compatible vendor under its own id and credential; attribution no longer hard-coded `openai` (four inline test call sites updated) | `k1-kimi-openai-compatible.diff` | **maos-providers +26** |
| SEC | `SecretKey::{GeminiApiKey, KimiApiKey}` (`gemini-api-key` / `MAOS_GEMINI_API_KEY`, `kimi-api-key` / `MAOS_KIMI_API_KEY`; `ALL` 10 → 12, so keyring migration and purge pick them up); purge labels; `env_contract.rs` rows for both keys and `MAOS_DEFAULT_PROVIDER` | `sec-secret-keys-env-purge.diff` | **maos-domain +8, maos-bin +17** |
| SEL | builder registers Gemini (`https://generativelanguage.googleapis.com`) and Kimi (`https://api.moonshot.ai`); replay answers every id in `maos_providers::PROVIDER_IDS`; `MAOS_DEFAULT_PROVIDER` selects the router default, an unregistered id refuses boot typed (D06 D) | `sel-registration-and-default-selector.diff` | **maos-bin +51, maos-providers +1** |
| PIN | `[providers]` parsed by `gate_manifest` into `GatedManifest.providers`; every entry must be declared in `provider.complete`; `check_provider_pins` (sha256 of the registered base URL's bytes vs the pin, `endpoint_url` equality); two typed refusals (`provider_unavailable`, `endpoint_pin_mismatch`); `AdmissionDeps.endpoints` at the four call sites; endpoint table captured at registration; `admit_spirit` receives `gated.providers` (feeds the kernel's existing `ProviderSwitched` journal, no kernel line) | `pin-providers-section-and-endpoint-pin.diff` | **maos-bin +77** |
| **Measured total (recommended, out-of-kernel)** | G1 + K1 + SEC + SEL + PIN | — | maos-providers **+169**, maos-bin **+145**, maos-domain **+8** |

## Gate blindness (rule 11 failing-before, measured)

`check-model-currency --json` on the `sel` copy (Gemini `gemini-3.5-flash-lite` and Kimi `kimi-k2.6` constructors in
`main.rs`; neither provider exists in `xtask/model-currency.toml`) → **rc 0, `passed: true`, 17 files, 16 pins, 0
findings** — byte-identical counts to the in-tree HEAD run (`/tmp/c204/mc-head.json`: rc 0, 16 pins). The gate's
regex (`check_model_currency.rs:241-243`) does not match `GeminiProvider::new` or `OpenAiProvider::compatible`, and its
required-host list (`:287-293`) is the three existing providers.

## Runtime measurement at HEAD (prebuilt binary, no cargo)

`runtime/selector-probe.sh` (isolated `HOME`/`XDG_*` under `/tmp/c204/probe`, deleted; `MAOS_OLLAMA_URL=skip`) →
`runtime/selector-probe-a8af3215.txt`. Binary `/mnt/build/cargo-maos/debug/maos`, mtime 2026-11-02 11:15 UTC; `main.rs`
is unchanged between `8143326b` (09:28) and HEAD (`git diff 8143326b 47631284 -- crates/maos-bin/src/main.rs` empty),
so the provider builder and shell path the probe exercised are HEAD's [INFERENCE: the binary was built from that
tree; not reproduced by cargo].

| Run | Command (abridged) | Observed |
|---|---|---|
| S1 | the epic's AC1 line: `MAOS_DEFAULT_PROVIDER=gemini MAOS_INFERENCE_MODE=replay MAOS_REPLAY_CASSETTE=crates/maos-journey-test/cassettes/providers/gemini-shell-intro.json maos shell` | **rc 1**: `cassette read: …gemini-shell-intro.json: No such file or directory` |
| S2 | same selector with the J0 cassette, input `@hello-spirit hello` | rc 0; reply from the cassette; Transparency Log row **`infer:anthropic->replay:default`** — the selector is ignored |
| S3 | control, no selector | rc 0; same row `infer:anthropic->replay:default` |

## Not measured here (owed to the dev pass; a measured figure above the ruled envelope is STOP + D-row)

1. Compilation of every prototype (`cargo check --locked -p maos-kernel-core -p maos-providers -p maos-domain -p maos-bin -p maos-manifest`), in particular `ureq::Error::Transport(t)` / `t.kind() == ureq::ErrorKind::BadHeader` against the locked ureq 2.12.1 (KIO1).
2. Whether ureq 2.12.1 validates a header before connecting (KIO1's hermetic witness needs no listener if so; else a loopback listener).
3. The live Gemini/Kimi wire against the vendor (operator leg; `ops-live-gemini-leg`): request accepted, `usageMetadata` present, `finishReason` values seen.
4. The per-Spirit binding sites (shell default, Butler `first_declared_provider` `main.rs:890-892`, Researcher `spirits/researcher/src/lib.rs:487`, Orchestrator `orchestrator_root.rs:221-237`) honouring `[providers].primary` — forecast only (+20…+50 maos-bin).
5. `check-model-currency` extension (regex + `compatible` 5th-argument read + five required hosts) — forecast xtask +5…+20.
6. Every production admission site that must run the pin check: `admission.rs:903` (door/run) is prototyped; the shell's own `admit_spirit` (`main.rs:4461`) and the others at `main.rs:8238,9560,11734,14861` are to be classified (production vs smoke/test) at T0.
7. Vendor model ids and prices on the day (D-20-4-03): `https://ai.google.dev/gemini-api/docs/models` and `https://platform.kimi.ai/docs/api/list-models.md` were read 2026-11-02; the price book rows need the vendors' pricing pages.
