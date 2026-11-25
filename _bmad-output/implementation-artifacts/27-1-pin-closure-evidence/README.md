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
| 4 | Door load: the native constructor takes the already-gated manifest TEXT, never a pathname reread. `build_spirit_obj`, `gate_staged`, `read_butler_epistemic_policy` take text; the in-process hot-swap factory reads the staged file once, checks and builds from the same bytes (spawned successor and kernel bundle: see "Scope of the item-4 guarantee"); the door gate-then-build step is the free function `prepare_spirit_from`, and `door_manifest_text` picks the door's pre-read text | `crates/maos-bin/src/main.rs`; tests `crates/maos-bin/tests/support/door_bind_27_1.rs` (module in `main.rs`) |
| 5 | One-shot Hello positive arm now requires a successful exit and a parsed JSON `introduction` | `crates/maos-bin/tests/one_shot_hello.rs` |

## Documented limits (recorded, not implemented; remaining LOW gap per S-7b B)

1. **Shell-entry control gap.** The shell entry's pin check (`crates/maos-bin/src/main.rs:4836`,
   `check_gated_pins(&gated, provider_catalog.get())` over the embedded Hello manifest) has no
   removal-sensitive test: `shared_pin_validation_covers_the_shell_manifest_and_a_minimal_staged_successor`
   exercises the helper, so it would still pass if this call were deleted. The check is present; the
   gap is test coverage only.
2. **Registry install and migration-plan creation do not run the runtime model-currency check.**
   `maos_registry::install::install` (`crates/maos-registry/src/install.rs:47`, admission at `:133-135`,
   storage `:155` ff.; called from `crates/maos-cli/src/subcommands.rs:2745-2758`) performs package
   admission and persists it without `check_gated_pins`/`check_staged_pins`; and
   `migration_plan::create_plan` (`crates/maos-bin/src/main.rs:10995`) only writes a plan file. No Spirit
   runs in either; the runtime check runs at the first load/boot (`installed_spirits.rs`,
   `load_admit_start`) and at hot-swap staging (`main.rs:3429`).

## Plants (marker `E271-PLANT`, all removed; grep over `crates .github docs RELEASE-HOLDS.md Cargo.lock xtask` empty)

See `proven-red/`. Summary: reverting each fix turns exactly the named tests red; restored, they are green.

## Scope of the item-4 guarantee (narrowed after review)

"Reads the staged file once, checks and builds from the same bytes" holds for the **in-process**
successor only (`build_spirit_obj`). Two reads remain on a hot swap, both of a daemon-owned,
mode-0600, create-new, random-named snapshot that no other local user can write:

- the **spawned** successor rereads the snapshot (`crates/maos-bin/src/spawned_lifecycle.rs:1201-1203`,
  `spawned_successor`, which also needs the path for its launch directory; its caller and
  `tests/support/spawned_world_20_9.rs:426` would change signature, so it was not widened here);
  spawned guests make no provider calls, so no model binding rides on that read;
- the kernel `SpiritManifestBundle` is built from a separate read inside
  `orchestrator.upgrade(snapshot)` (`crates/maos-bin/src/migration_plan.rs:236` neighbourhood).

The ordinary door `load` path is exact: `door_manifest_text` (`main.rs`) returns the door's pre-read
text, which `prepare_spirit_from` gates, pin-checks and builds from. The `load_spirit` call site is one
line; `door_bind_27_1::the_door_loads_from_its_pre_read_text_not_the_pathname` witnesses the choice,
not the async method, so a reread inserted at the call site rather than in `door_manifest_text` would
not be caught (recorded limit).

AC4 refusal text: the echoed `[providers].primary.model_id` is uncapped in the manifest, so
`ModelNotAdmitted`'s message now echoes at most 128 characters (an ellipsis marks a cut) with
control characters escaped (`admission.rs`, `echoed`); test
`a_hostile_model_id_yields_a_bounded_printable_refusal`.

## Open (pre-existing; recorded, NOT fixed here)

1. **Cold-swap preflight DoS via a shared-temp sidecar (P2).** `ManifestSnapshot::new` writes the
   retained predecessor bytes into the shared temp directory (`crates/maos-bin/src/operator_door.rs:487`,
   `std::env::temp_dir().join("maos-restore-<ulid>.toml")`). `bind_registry_package_at` then calls
   `signed_package_at_with`, which reads `<dir>/signed_package.json` beside that snapshot
   (`crates/maos-registry/src/install.rs:274-279`), contradicting its own comment at `:290` ("A temporary
   manifest snapshot is never a package-identity lookup location"). Any local user can create
   `/tmp/signed_package.json`; the lookup then parses it (or fails on its content, or on a mismatched
   `spirit_id`) and every cold-swap restore preflight fails with `cold_swap_unrestorable`: a local denial
   of service. Whether a planted file could also influence identity binding was not checked.
   Suggested fix: skip the sidecar lookup for snapshots (the preflight needs only the gated manifest) or
   bind the publisher from the recorded admission record instead of a path-adjacent file.
2. **Door can reach pin validation with `catalog=None` during startup.** The HTTP listener starts before
   the boot provider catalog is published, so a door request that arrives in that window runs
   `check_gated_pins`/`check_staged_pins` with no catalog and the declared-pin check is vacuous
   (`catalog.and_then(..)` is `None`). The runtime driver gate (`AdmittedModels`) still refuses an
   unadmitted model on the wire; the gap is the typed admission refusal during startup only.
