# Proven red (marker `E271-PLANT`; every plant removed afterwards)

Env: `CARGO_TARGET_DIR=/mnt/build/cargo-maos-e271`, one cargo process at a time, random 0600 hex `MAOS_AUDIT_KEY` file (deleted after).

## Run A — three plants together in `crates/maos-bin/src/admission.rs`
Command: `cargo test --locked -p maos-bin --test manifest_model_pin_s1 --test one_shot_hello --no-fail-fast` (exit 101).

| Plant | Reverts | Red tests |
|---|---|---|
| `E271-PLANT butler` | `first_declared_pin` back to main's `(provider, model)` from the first declaration | `a_butler_pin_listed_after_a_provider_only_entry_reaches_the_wire` |
| `E271-PLANT ac4` | `check_provider_pins` returns `ProviderUnavailable` again | `admission_admits_several_models_per_provider_and_still_refuses_unlisted_ones`, `shared_pin_validation_covers_the_shell_manifest_and_a_minimal_staged_successor` |
| `E271-PLANT s7a` | `.filter(|p| p.registered)` restored in `check_declared_pins` | `declared_pins_are_validated_for_a_known_but_unregistered_provider`, `keyless_one_shot_validates_an_unregistered_providers_declared_pin` (real binary) |

Totals under the plants: `manifest_model_pin_s1` 5 passed / 4 failed; `one_shot_hello` 2 passed / 1 failed.
The four failures in `manifest_model_pin_s1` are exactly the butler test, both AC4 tests and the s7a test; the single `one_shot_hello` failure is the s7a real-binary control. The three plants were applied together, so a red test is attributed by which plant its assertion exercises (each fix is isolated to its own lines).

## Run B — `E271-PLANT reread` in `prepare_spirit_from` (`crates/maos-bin/src/main.rs`)
The constructor is fed `std::fs::read_to_string(manifest)` instead of the admitted text.
Command: `cargo test --locked -p maos-bin --bin maos door_bind_27_1` (exit 101): 
`a_file_that_differs_after_admission_cannot_change_what_the_door_binds` FAILED at the "admitted text must bind" assertion (0 passed / 1 failed).

## Not planted
Item 5 (one-shot positive arm) is test hardening with no production change; it is red-capable only if the
admitted run fails to exit 0 or print an `introduction`, which a restored-source run does not do.
