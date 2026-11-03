# Landing 1 plants and acceptance witnesses — 2026-11-03

The historical T0 stop was superseded by operator ruling **D-20-4-12 B** (`kimi-k3.1-pro`). The initial assignment HEAD was `16c35ef0d4b4de0f3969db15c9d7666fcdf9f2de`; the resumed HEAD is `8c809f9c5ce245f0bb51e94c20861e45c229d89f`. Kernel source was never changed.

## Six mutation controls

Exactly one named control per AC1–AC6 was exercised. Each row below is an observed test failure, not an inferred result. Raw outputs and timings are in [plant-results.json](plant-results.json); exact invocations and restoration commands are in [commands-resumed.json](commands-resumed.json).

| AC | Witness (`name:line`) | Single mutation | Red exit | Wall seconds |
|---|---|---|---:|---:|
| AC1 | `crates/maos-providers/tests/gemini_round_trip_test.rs::native_wire_header_usage_parts_and_stop_reasons:13` | Put the Gemini key in the URL query instead of keeping it exclusively in the header; assertion reports `credential in URL`. | 101 | 14.660 |
| AC2 | `crates/maos-providers/tests/kimi_round_trip_test.rs::compatible_wire_uses_kimi_key_and_attribution:12` | Attribute the compatible response to `openai` rather than `kimi`. | 101 | 11.144 |
| AC3 | `crates/maos-bin/tests/provider_selection_20_4.rs::epic_gemini_shell_command_selects_and_routes:9` | Ignore the selected default; the Transparency Log no longer contains `infer:gemini->`. | 100 | 53.184 |
| AC4 | `xtask/tests/model_currency_gate.rs::gemini_unknown_constructor_model_is_governed_20_4:534` | Remove Gemini from the host-constructor scanner; the unknown Gemini model loses its required finding. | 101 | 58.477 |
| AC5 | `crates/maos-bin/tests/provider_selection_20_4.rs::researcher_primary_overrides_declaration_order:80` | Remove the singleton-primary branch, allowing the old first declaration to win. | 100 | 31.947 |
| AC6 | `crates/maos-bin/tests/provider_endpoint_pin_20_4.rs::wrong_endpoint_pin_refuses_before_inference:10` | Skip the endpoint SHA-256 comparison; the bad pin is admitted. | 100 | 21.674 |

AC3's first invocation had an unquoted nextest expression and exited **2 before any test ran**. It was restored, then the same named mutation was re-applied with the correctly quoted expression. This invocation error is not counted as a red witness or a seventh control.

## Byte-preserving restoration

Before each control, the source was copied under `/tmp/204l1-proof/plants/<repo-relative-path>`. After every invocation, including the AC3 shell syntax error, this exact command form completed with exit **0**:

```sh
cp /tmp/204l1-proof/plants/<path> /home/lunarpulse/dev_ws/<path> && cmp /tmp/204l1-proof/plants/<path> /home/lunarpulse/dev_ws/<path> && touch /home/lunarpulse/dev_ws/<path>
```

The seven concrete copy-back + `cmp` + `touch` commands are retained in `commands-resumed.json`. `cmp` proves restoration bytes at that point; `touch` forces Cargo to observe restoration. Later intentional root cleanup reuses `GatedManifest` instead of reparsing its TOML sections and is measured separately in `kloc-final-formatted.json`.

## Before witnesses already observed

Independent resumed-HEAD API overlays produced exit **101** for each new provider test: Gemini's export and secret key were absent; Kimi's compatible constructor and secret key were absent. Every overlaid file was restored by copy-back + `cmp` + `touch`. Initial-HEAD runtime observations in `head-runtime-witnesses.json` show the ignored default selector, ignored per-Spirit primary, and admitted wrong pin. `head-currency-witness.json` records the old scanner accepting an unlisted Gemini and retired Kimi constructor. The exact outputs distinguish a missing-target red from the independent API compile reds; no Kimi result is inferred from Cargo stopping at the Gemini target.

The seed-shell witness proves provider selection, token binding and routing only. Native Gemini wire-format evidence comes from the recording-transport fixture test, not replay. Live paid-key wire legs and kernel AC7–AC8 remain Landing 2; no live/hosted pass is claimed here.

## Green-after on the final L1 source

After every control was restored and the canonical-manifest cleanup was formatted, the all-target workspace build completed with exit **0** (`artifact://11947`; the earlier 3000-second tool timeout is retained, not hidden).

| AC | Final exercised green witness | Observed result |
|---|---|---|
| AC1 | Gemini recording transport, including status-as-data; epic seed-shell command and TL/token query | Gemini's three tests passed within the eight-test provider run, exit 0 (`artifact://11956`). The literal shell pipeline exited 0; `epic-shell-command.json` records `infer:gemini->replay:default` and `{"ProviderInfer":{"provider":"gemini"}}`. |
| AC2 | Kimi compatible wire and old OpenAI regression; Kimi seed shell | Kimi's two and OpenAI's three tests passed in the same provider run; all 13 binary integration witnesses passed, exit 0 (`artifact://11952`). |
| AC3 | Selector, unknown/empty selector, bad credential bytes and registration/order | All nine selection tests passed in the 13-test binary run. |
| AC4 | Unknown Gemini model, deprecated Kimi K2, missing Kimi host | All 23 model-currency gate vectors passed, exit 0 (`artifact://11959`). |
| AC5 | Primary over declaration order, fallback refusal, undeclared primary | All selection vectors passed in the same final binary run. |
| AC6 | Wrong/matching SHA-256, URL/model assertions and Ollama air-gap endpoint change | All four endpoint tests passed in the same final binary run. |

`head-rebase-diff.txt` records an empty `git diff` between the initial and resumed HEADs for `main.rs`, `admission.rs`, provider sources, the model scanner and model policy. The initial runtime/gate before witnesses therefore have unchanged production proof surfaces across the 20-10 handoff; the independent API compile reds were additionally exercised against resumed HEAD. No second baseline build or extra mutation control is claimed.
