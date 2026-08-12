<!-- AUTO-GENERATED from maos-spirit-abi rustdoc — do not edit; regenerate via: cargo run -p xtask -- gen-abi-docs -->

# `deprecation` Module {#abi-deprecation-module}

## Related {#abi-deprecation-related}

- [lifecycle Module](./lifecycle) — hooks observe warnings via `Ctx::deprecation_warnings()`
- [ctx Module](./ctx) — `Ctx::deprecation_warnings()`
- [STABILITY.md](https://github.com/lunarpulse/maos/blob/main/STABILITY.md) — deprecation lifecycle tracking


*ABI_VERSION = 1 · MANIFEST_SCHEMA_VERSION = 4*

Story 7.1 v0.5 binding — deprecation warning channel surface.

Spirit code that uses a deprecated ABI surface receives a tagged warning
observable via `Ctx::deprecation_warnings()`. The `spirit-test` SDK
surfaces these warnings in test output; Story 7.5a's ABI compatibility
matrix gate (NFR-Maint-3) consumes them at v1.0 to assert every deprecated
surface has a matching `STABILITY.md` entry.

At v0.5 the ABI has ZERO deprecations to surface — the channel ships
EMPTY-PRESENT. The `Ctx::mock_with_deprecation_warnings(vec![...])`
test helper lets `spirit-test` verify the surfacing WORKS even though
no real deprecations exist at v0.5.
