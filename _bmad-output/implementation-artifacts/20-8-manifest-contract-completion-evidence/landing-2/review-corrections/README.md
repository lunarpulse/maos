# 20-8 landing 2 — review corrections (GPT P2 ×2, GLM F-1)

Dev208K, `anthropic/claude-sonnet-5-5`, base `a2bbd687`, 2026-11-11, uncommitted. The reviews are persisted verbatim in `../review/{gpt-review,security}.md`.
Kernel moves 27009 / 21178 → **27014 / 21181** (+5 / +3, Main-approved `/tmp` prototype, inside the ×2 ceiling 27043 / 21207 and the STOP 21250,
no new surface hash: `check-service-boundary` flags nothing); bin 38446 → **38475** (+29); aggregate hardfail 202719 → 202751.
Exact commands and exits: `commands.md`.

| Directory | Contents |
|---|---|
| `red-head/` | failing-before at HEAD `a2bbd687`: `red-head-kernel.log` (3 of 5 vectors red), `red-head-door-cold.log` (the cold swap of the dropping successor exits 0), the HEAD genuine build |
| `green/` | the same witnesses green (kernel 5/5, door 2/2) and the genuine build |
| `plants/` | five plants (diff, raw log, `plants.json`: exit, seconds, restore `cmp`); each restored by copy-back + `cmp` + `touch` |
| `gates/`, `final/` | gate JSON, clippy + ratchet, kernel `--lib`, focused suites, verb table, workspace nextest |

## Witness table

| Review item | Witness | Red before (HEAD) | Plant | Final |
|---|---|---|---|---|
| GPT P2-1: scalar `swap_invariants = false` is a parse refusal, not absence; zero hooks, runtime unchanged | `swap_invariants_20_8.rs::a_scalar_swap_invariants_declaration_is_a_parse_refusal_not_absence` | swap completes (`red-head-kernel.log`) | `scalar-error-swallowed` (`.ok()` restored ⇒ red) | green |
| GPT P2-2: a 32-hex and an `sk-queue` dropped name are recoverable from the persisted receipt; raw typed error intact; predecessor unchanged; zero hooks | `…::a_redactor_shaped_dropped_name_is_recoverable_from_the_receipt` (and the label check in `a_successor_that_drops_a_declared_invariant_…`) | label/receipt absent (`red-head-kernel.log`) | `raw-name-in-receipt`, and `raw-name-under-hex-label` (receipt redacted: hex decode `OddLength`) | green |
| GLM F-1: cold swap refuses a dropped invariant before the unload; superset passes | `one_daemon_one_door_16_1.rs::ac8_20_8_a_cold_swap_that_drops_a_swap_invariant_is_a_typed_409_before_unload` | exit 0 (`red-head-door-cold.log`) | `cold-swap-check-dropped` | green |

Retained caveats: the redactor's token rule is ≥ 32 consecutive hex characters (`redaction.rs` `TOKEN_HEX_MIN_LEN`), so the 32-hex witness name is genuinely
redaction-shaped; the first red of the receipt vectors at HEAD is the missing label, and the redaction itself is demonstrated by the `raw-name-under-hex-label`
plant. F-2 (precheck), F-3 and N-1 are deferred rows (`deferred-work.md` § "Deferred from: Story 20-8 landing 2"), not code.
