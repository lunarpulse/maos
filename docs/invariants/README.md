# Invariant Register

This directory contains the canonical machine-readable register of MAOS invariants I1–I14.

## Files

- `I1.md` through `I14.md` — one file per invariant, with frontmatter enforcing the phase-cadence matrix from Architecture §3.2.1.
- `journal.jsonl` — append-only journal of every invariant-lock merge event.

## Claim ledger (20-3b)

Every register frontmatter declares `claim: guaranteed | asserted`. The existing
`invariant-lock` reads **all fourteen** entries on every invocation and includes
them in its JSON report, even when no invariant was changed.

- `guaranteed` requires `property_test: relative/path.rs#test_name`,
  `adversarial_corpus: relative/path`, and `ci_job`. The gate parses Rust
  (comments do not declare tests) and requires a non-empty top-level `#[test]`
  with no conditional/ignore attributes, directly loading the named corpus via
  `include_str!`/`include_bytes!` in its body. The normal Cargo integration target
  must belong to the explicitly selected package; custom/conditional target
  shapes are refused. The discipline job must be enabled and contain a direct
  `cargo test -p <package> --test <binary> [test_name]` run step, with the named
  corpus in that job/step's environment. Wrong selectors, build-only/ignored-only invocations and
  disabled jobs/steps do not earn a guarantee. Unsupported shell/conditional
  forms fail closed: use an asserted claim until execution is statically bound.
- `asserted` requires a non-empty `owner`. It is not a test failure or an
  assertion that implementation is absent: narrower unit/runtime anchors do
  not earn the full invariant-wide guarantee. Null evidence fields explicitly
  disclose that no complete property/adversarial pair has been established.
- The ledger owner owns the claim classification; it does not silently create
  a new implementation story. All broad invariants currently remain asserted.
  I5 separately records **FR13 asserted, owner 20-1** and **FR31 proven by
  20-3b** (cross-boot adversarial vectors in `principal_identity_20_3b.rs` and
  `butler_scene_18_4.rs`). Principal isolation does not prove quarantine isolation.
  The landing verifier must exercise these new vectors; metadata is not execution evidence.

### Routed AC5 witness ledger

These are integration-test **binaries**, not evidence that the full invariant is
guaranteed. New witnesses have not been executed by the editing agent; the
landing verifier must compile/run the green vectors and demonstrate the two
labelled red vectors with `--ignored --exact`. An ignored witness never earns
`claim: guaranteed`.

| Requirement | Claim and repair owner | Executing witness / disposition |
|---|---|---|
| FR34 | asserted; owner: none (trigger-owned; see deferred-work) | `maos-spirit-sdk --features spirit_test --test routed_sdk_20_3b`: `sdk_harness_covers_lifecycle_input_halt_manifest_and_class` covers the shipped surfaces; `sdk_harness_records_iac_frame_io_in_its_capture_report` is a labelled ignored red for absent SDK IAC capture. External-author validation remains `ops-external-cohort-and-pen-test`, not this harness witness. |
| FR53 / I14 | asserted; owner: 20-3c (member 12) | `maos-kernel-core --test routed_halt_swap_20_3b`: `active_halt_keeps_identity_and_replay_context_across_hot_swap` invokes the real hot-swap coordinator, comparing the original pending id and its owner, fire time, full replay payload and successor snapshot. Labelled ignored red: the current continuity gate drains the original halt and metadata; successor re-firing is not a substitute. |
| FR63 | asserted pending verifier execution; witness owner: 20-3b, no new repair owner claimed | `xtask --test routed_catalog_20_3b`: `every_kernel_catalog_variant_has_retryability_and_cause_chain` executes the real AST/registry/artifact gate and independently checks every published kernel row's retryability and cause-chain metadata. The catalog's E*-discovery contract remains the gate's documented scope; v1.0 coverage enrollment is separately dispositioned in deferred-work. |

All three binaries are named in the landing-2 hosted evidence workflow. Red
witnesses are expected failures with their reasons/owners retained, not CI passes.


The coverage spike selected NO-GO (43 explicitly advisory v1.0 empty gate
lists cannot satisfy strict generator completeness without false claims).
The matrix remains hand-maintained, in hard mode; its corpus-delta witness
therefore remains `tests/coverage-matrix.yaml`.


## Lock Protocol (ADR-037)

Any PR touching an invariant file (`I*.md`), the `lock.toml` mapping, or the `maos-domain` invariant types must pass the `invariant-lock` CI gate. The gate requires:

1. **Machine-checkable diff** against the invariant set (present by construction).
2. **Corpus delta** — touch to `tests/coverage-matrix.yaml` (file-touched check at v0.1-α; row-count comparison at v0.3+).
3. **Phase-commitment update** — at least one touched `I*.md` has its enforcement-cadence table modified.
4. **≥2 maintainer sign-offs** verified via platform API.
5. **Forward-only progression** — no demotion of enforcement cells (`runtime → CI` is forbidden).

On merge success, the gating CI job appends to `journal.jsonl`.
