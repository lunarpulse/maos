# Story 27-1 pin closure — evidence (branch e27-1-pin-closure-2026-11-25, base main ceb3cc88)

Operator ruling 2026-11-25 "as recommended": S-7a A (validate declared pins regardless of
provider registration), S-7b B (accept the four existing reviews). Source: GPT-6.2 review of
main (agent `Rev271`).

## What changed

| # | Item | Files |
|---|---|---|
| 1 | Butler first-declaration: provider from the first entry, its model through `pin_for` (port of the 2026-11-20 correction) | `crates/maos-bin/src/admission.rs` `first_declared_pin`; test `a_butler_pin_listed_after_a_provider_only_entry_reaches_the_wire` |
| 2 | AC4: an invalid registered-provider `[providers]` assertion returns `ModelNotAdmitted` (actual model named), not `ProviderUnavailable` (port of 2026-11-21); the separate registration check still returns `provider_unavailable` for an absent provider | `admission.rs` `check_provider_pins`; two assertions in `crates/maos-bin/tests/manifest_model_pin_s1.rs` |
| 3 | S-7a A: `check_declared_pins` looks the provider up in the boot catalog without the `registered` filter (Anthropic, OpenAI, Gemini, Kimi, Ollama all carry defaults and currency rows; no fallback B needed) | `admission.rs`; tests `declared_pins_are_validated_for_a_known_but_unregistered_provider` (gated + `check_staged_pins` negative) and, real binary, `keyless_one_shot_validates_an_unregistered_providers_declared_pin` in `one_shot_hello.rs` |
| 4 | Door load: the native constructor takes the already-gated manifest TEXT, never a pathname reread. `build_spirit_obj`, `gate_staged`, `read_butler_epistemic_policy` take text; the hot-swap factory reads the staged file once, checks and builds from the same bytes; the door gate-then-build step is the free function `prepare_spirit_from` | `crates/maos-bin/src/main.rs`; test `crates/maos-bin/tests/support/door_bind_27_1.rs` (module in `main.rs`) |
| 5 | One-shot Hello positive arm now requires a successful exit and a parsed JSON `introduction` | `crates/maos-bin/tests/one_shot_hello.rs` |

## Documented limits (recorded, not implemented; remaining LOW gap per S-7b B)

1. **Shell-entry control gap.** The shell entry's pin check (`crates/maos-bin/src/main.rs:4821`,
   `check_gated_pins(&gated, provider_catalog.get())` over the embedded Hello manifest) has no
   removal-sensitive test: `shared_pin_validation_covers_the_shell_manifest_and_a_minimal_staged_successor`
   exercises the helper, so it would still pass if this call were deleted. The check is present; the
   gap is test coverage only.
2. **Registry install and migration-plan creation do not run the runtime model-currency check.**
   `maos_registry::install::install` (`crates/maos-registry/src/install.rs:47`, admission at `:133-135`,
   storage `:155` ff.; called from `crates/maos-cli/src/subcommands.rs:2745-2758`) performs package
   admission and persists it without `check_gated_pins`/`check_staged_pins`; and
   `migration_plan::create_plan` (`crates/maos-bin/src/main.rs:10980`) only writes a plan file. No Spirit
   runs in either; the runtime check runs at the first load/boot (`installed_spirits.rs`,
   `load_admit_start`) and at hot-swap staging (`main.rs:3414`).

## Plants (marker `E271-PLANT`, all removed; grep over `crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` empty)

See `proven-red/`. Summary: reverting each fix turns exactly the named tests red; restored, they are green.
