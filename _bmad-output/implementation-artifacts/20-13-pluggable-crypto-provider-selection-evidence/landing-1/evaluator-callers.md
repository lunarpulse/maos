# Ring-direct evaluator caller analysis and D-20-13-17

Compared the `be3b9763` HEAD sources using `git show HEAD:<path>` with the L1
working tree and searched non-test Rust call sites. The previous shared-core
implementation allowed an empty attestation to mask a nonempty runtime value,
which could change a ring-direct admission refusal to an admit. The orchestrator
adjudicated D-20-13-06 A as D-20-13-17: ring-direct evaluation preserves HEAD
semantics; only the new provider-aware boot evaluator treats an empty attested
value as unpinned. Nonempty attestations compare exactly on both paths.

| Caller / path | HEAD runtime values | L1 runtime values and disposition |
|---|---|---|
| `maos-registry/src/admission.rs:148-172`, reached by `install.rs:86-94` and `maos-cli/src/subcommands.rs:1408-1409` | Optional operator overrides or, for those real callers, `None` selects manifest-derived endpoint and crypto. The manifest parser accepts nonempty values. | Same ring-direct input; preserve HEAD comparison even for empty claims. |
| `maos-registry/src/compliance_verify.rs:52-66`, public structural shim | Both manifest-derived; potentially nonempty. No non-test in-repository caller found. | Same ring-direct input; preserve HEAD comparison. |
| `maos-bin/src/main.rs:15235-15301`, compliance smoke | Manifest-derived `ring` and Anthropic endpoint; drift vector explicitly passes `fips-module`. | Same smoke inputs and ring-direct comparison. |
| `maos-compliance/src/evaluator.rs` public `evaluate_envelope` → `evaluate_envelope_at` | Caller-supplied context; comparison of empty and nonempty values is exact. | Same for both public ring-direct functions. |
| `maos-registry/src/install.rs:295-334` `verify_installed` | No selected-provider evaluator; ring-direct, manifest-derived admission. | New daemon boot-only `evaluate_envelope_with`, selected name and catalog endpoint; empty attested fields unpinned per D-20-13-17. |

The FKCS `AdmissionHarness` constructs nonempty `fkcs`/`local://fkcs`/`none`
values, but its `Local` unsigned package bypasses public-untrusted claim
verification; it is an in-memory harness, not evidence that the production
ring-direct callers are empty. The source-level `evaluate_envelope_at` calls in
the binary's CCAC smoke use `reference_context`, likewise intentionally carrying
real corpus values. The earlier runtime-context synthetic witness confirmed the
shared core admits an empty claim against a nonempty runtime (`artifact://15752`);
the stricter regression witness must reject this vector after the correction.
