# Epic 17 Context: Workers and the Third-Party Form

<!-- Compiled from planning artifacts. Edit freely. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Make isolation real for non-first-party execution forms: Workers execute behind a contained egress path, and third-party/polyglot Spirits execute as WASM components in a T2 runner rather than inside the daemon. Preserve kernel capability mediation, attributable audit records, resource containment, and the TypeScript authoring route. First-party compiled-in Rust Spirits remain trusted, in-process T0 code by design; this epic does not claim a process sandbox for them.

## Stories

- Story 17.1: Worker egress allowlist and scoped credential.
- Story 17.2: Worker cgroups applied.
- Story 17.3a: WASM recall, componentize-js, versioning, and egress spike.
- Story 17.3b: WASM form admission, contract, registry, and TypeScript toolchain.
- Story 17.3c: WASM Spirit on the IAC bus under T2.
- Story 17.3d: WASM log recall.
- Story 17.4: Hooks, Observer, and activity corpus.
- Story 17.5: Model-pin currency gate.
- Story 17.6: T2 sandbox repair and proven-red containment.

## Requirements & Constraints

Isolation must enforce the strictest manifest, trust-tier, and operator-policy floor. Capability tokens mediate external operations and bind authorization to trusted Spirit identity rather than guest declarations. Every accepted frame and authorization result must remain attributable and auditable; a preserved frame is not proof of enforcement. Configured enterprise identity or PDP failure must deny authorization rather than falling back to ungoverned execution.

Resource limits must bound CPU, memory, and descriptors; a serialized-frame bound alone does not bound guest execution or the host allocation used to lift guest values. Lifecycle records remain durable, and planned or unplanned termination must produce a halt receipt. A control is claimed only after production wiring, exercised behavior, and a planted defect proving rejection. CI skips and successful non-applicable cases are not containment evidence on declared-capable runners. Local inability to enforce a control must be named rather than silently called success.

Kernel changes require an isolated measured prototype and an explicit operator-approved numerical FLAG-Winston grant. Physical source baseline/content hashes and the production KLOC counter are separate instruments. Per-crate grants are measured after formatting; aggregate advisory alarms remain visible. A location charter or previously measured minimum seam is not a grant for a complete implementation.

## Technical Decisions

The first-party manifest form is `rust-inproc`; registry packages declaring it are refused at every trust tier. The third-party form is `wasm-component`, with manifest schema 5 and an artifact declaration. The `subprocess` compatibility token and mutually exclusive CLI-wrapper manifest shape are not silently interchangeable with the class form. WASM Spirit execution uses T2 plus WIT capability boundaries; T4 is reserved for the separate WASM tool sandbox.

Keep the ADR-031 runner process and ADR-032 length-delimited wire boundary. WASM engine support remains feature-gated and disabled by default until the release hold is cleared. An unavailable engine, malformed artifact, or incompatible component is a typed refusal, never bare-process fallback. Compilation and instantiation of untrusted components belong in the contained runner; daemon checks are metadata-only. Guest claims cannot select scheduler identity, admission sandbox, or trusted authorization state.

The current `maos:spirit@2.0.0` contract preserves intent, consent, lineage, and typed scopes. Do not introduce a WIT version change casually: contract changes require ratification and reissue of all four provenance-pinned components. TypeScript componentization uses the pinned toolchain and a release runner; debug compilation is not equivalent evidence for the shipped TypeScript artifact.

WASM recall is a separate capability surface and story. Its ratified transport is an inherited connected descriptor rather than a path socket; it is not part of the current runner launch/authority preparation. Worker vendor egress follows the ratified proxy decision, not general guest network access.

## Cross-Story Dependencies

The completed spike supplies measured premises and 17-6 supplies the real T2 route. Verified 17-3b contract/admission/refusal evidence enabled detached 17-3c implementation without premature 17-3b closure. Both are `done` (2026-10-06): the 17-3c admitted-session kernel-verdict gate and its compiled mutation closed 17-3b's final finding after independent verification.

17-3c must establish launch, trusted session identity, bus framing, safe conformance, authorization observation, and termination evidence before 17-3d adds recall. Registry publication depends on the admission/contract story, not recall. Reuse schema 5 for later egress/resource fields rather than making competing schema bumps.

The user accepts completed Ubuntu25.10 x86_64 testing as sufficient for 17-3c; no further Ubuntu24/26/ARM execution is required or claimed verified. Broader repository/other-story CI stays separate. 17-3c is reviewed and integrated (kernel pin 25293); see its specification and `17-3c-evidence/`. 17-3d stays backlog. Exit commands stay hermetic and secret-free; provisioned external demonstrations remain operator-lane evidence.
