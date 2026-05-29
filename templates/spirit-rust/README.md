# {{crate_name}}

A MAOS Spirit scaffolded from `templates/spirit-rust/` (Story 2.3 v0.3 NFR-Onb-1 prerequisite).

## Build your first Spirit in 30 minutes

This template scaffolds a minimal, compilable MAOS Spirit using the
`#[spirit]` proc-macro. By default it contains a single `on_idle` hook.
You should be able to build and test your first Spirit within 30 minutes
of opening this scaffold — the goal of the NFR-Onb-1 30-Min First Spirit
Validation Gate (N=12 stratified; median ≤45 min, p95 ≤90 min, ≥10/12
succeed).

## How to run

```bash
# If you generated this crate from templates/spirit-rust/:
cargo test --features maos-spirit-sdk/local_runner
```

The smoke test fires `on_idle` through the `local_runner` and asserts the
hook fired exactly once.

## What `#[spirit]` derives for you

The `#[spirit]` proc-macro (shipped in Story 2.1) generates:

- A `SpiritVtable<YourStruct>` type (`#[repr(C)]`) with function pointers
  for each lifecycle hook that you implement.
- A `__maos_spirit_vtable_YourStruct()` function that returns a static
  reference to the vtable — the vtable is the dispatch surface the MAOS
  kernel uses to invoke your Spirit's hooks.
- Hook-budget metadata if you annotate hooks with `#[hook(budget = "...")]`
  (not used by this template; available for advanced Spirits).

The macro imposes a contract on your `impl` block: hooks must be regular
`fn` methods with the exact signatures shown in the `on_idle` example.
Wrong signatures produce confusing errors in generated code.

## How to extend

Read the Spirit lifecycle trait at `crates/maos-spirit-abi/src/lifecycle.rs`
to see all 11 available hooks:

- `on_load`, `on_start`, `on_idle`, `on_frame`, `on_telemetry_event`,
  `on_schedule`, `on_swap_in`, `on_pause`, `on_resume`, `on_consolidate`,
  `on_unload`

Add any hook you need to your `impl {{class_name}}` block with the exact
signature from the trait definition and the `#[spirit]` macro will wire
it into the vtable automatically.

## What ships at v0.3 vs. later

| Milestone | What ships |
|-----------|------------|
| Story 2.3 (v0.3 prerequisite) | This template (Rust only), `local_runner` SDK seed, example Spirit with CI |
| Story 2.4 | Full spirit-test SDK seed: assertion macros, halt resolution, manifest self-check, LCAS 70-bucket |
| Story 7.1 (v0.5+) | Per-language templates (TypeScript / Python / Go), full per-language SDK |
| Story 7.5b | NFR-Onb-1 30-Min First Spirit Validation Gate execution (against Butler from Story 8.1) |

## Author your first Spirit — v0.5 path

```bash
# Scaffold a new Rust Spirit from this template:
cargo generate --git https://github.com/your-org/maos templates/spirit-rust --name my-spirit

# The v0.5 assertion macros are the canonical author-facing shape:
#   spirit_test::assert!(condition, "diagnostic")
#   spirit_test::expect_frame!(report, kind = ..., bytes_matches = ...)
#   spirit_test::expect_halt!(report, halt_id = ..., kind_matches = ...)
# See `crates/maos-spirit-sdk/src/spirit_test/assert.rs` for full reference.
```

## Status of this template

**v0.5 binding per Story 7.1.** Rust + TypeScript templates ship at v0.5.
Python deferred to v1.0; Go deferred to v1.5. The NFR-Onb-1 30-Min First
Spirit Validation Gate executes at Story 7.5b against the Butler reference
Spirit shipped by Story 8.1. This template is the SUBSTRATE for that gate.
