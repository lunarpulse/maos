# Landing 1 admission and binding sites

Static classification of current source after rustfmt (2026-11-03); kernel source is unchanged. These are coverage observations, not a claim that compilation or runtime verification has completed.

## Kernel SecurityManagerAdapter callers

| Caller | Classification | Provider treatment |
|---|---|---|
| `crates/maos-bin/src/admission.rs:991`, `load_admit_start` | Production shared admission for standalone `run`, topology, operator-door load/hot-swap successor and installed WASM components | `check_provider_pins` at function entry (`:932`), before pid allocation, storage binding or kernel admission. Receives `gated.providers.as_ref()`. Every production AdmissionDeps carries the immutable boot catalog; fixture-only callers explicitly carry None, which refuses any providers-bearing manifest. |
| `main.rs:4550`, shell hello-spirit | Production | Embedded manifest passes `gate_manifest`, declaration/fallback checks and catalog pin/model/URL assertions before scheduler load/admission. Kernel receives the section. The selected primary binds both the turn token and BoundInference requests. |
| `main.rs:9615`, one-shot hello-spirit | Production | Disk manifest passes the same gate/pin check before admission. Selected primary is retained for token issuance and BoundInference; absent primary retains router default. |
| `main.rs:11790`, `admit_daemon_control_spirit` | Production enterprise control identity | Full gate plus catalog assertion check, threaded from `build_enterprise_daemon_governance`, precedes kernel admission; section passed through. This identity is not an inference execution path. |
| `main.rs:8340`, fixed `smoke-spirit` | Diagnostic smoke arm | Parsed/synthetic smoke-only sections; passes providers=None, not an arbitrary providers-bearing production manifest. |
| `main.rs:14915`, ABI smoke closure | Diagnostic `smoke_abi_stability_7_5a` | Synthetic class and empty capabilities; providers=None; no provider manifest is accepted. |
| `main.rs:15376`, `live_mcp_fixture` | `#[cfg(test)]` unit-test root | Fixed MCP fixture and synthetic class; providers=None. |

The `maos_registry::admission::admit_spirit` calls in registry/air-gap diagnostic arms judge signed package metadata, not kernel Spirit admission or token issuance. The FR38 runtime drift-context gap remains assigned to 20-13, per D-20-4-10 B; it is not silently claimed closed here.

## Binding coverage

`main.rs::declared_providers` now returns only `[providers].primary.id` when present, otherwise the pre-existing ordered declaration list. Every Butler (`first_declared_provider`), Researcher and Orchestrator boot/topology/door/successor binding receives that function's result. Their existing resolver functions remain intentionally unchanged: given a singleton primary list, they cannot choose another registered declaration. Production root gate checks run before class construction so a missing primary is `provider_unavailable`, not a downstream generic class-build failure.

Shell and one-shot hello-spirit use `BoundInference` to name the exact provider their token names, rather than allowing hello's implicit request to select the router default. Installed components pass through shared admission before a session starts. The catalog contains all five configured endpoint/model pairs before replay/record wrapping; registration flags are captured from the actual post-wrapping provider map. Ollama's URL is read once. All URL assertions and hashes use that same boot value.

Shell, one-shot and control admission destructure the canonical `GatedManifest` instead of parsing the TOML sections a second time. Declaration degradation, posture, epistemic policy and class validation therefore have one implementation on these paths.

## Boundary

The pin locks the configured base URL bytes, not DNS or TLS identity. A missing credential does not erase the configured endpoint entry; explicit replay registers every provider id, so a keyless replay evaluates the same endpoint pin as a keyed deployment. A provider absent from the actual router is refused rather than silently falling back.
