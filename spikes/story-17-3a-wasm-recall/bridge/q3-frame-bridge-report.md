# Q3 frame_bridge

## Verdict — NO-GO for additive frame growth

The extended bridge itself round-trips a fully populated domain frame through
canonical CBOR byte-for-byte, while the same target through HEAD's bridge loses
all four requested surfaces. But adding the fields changes the component-model
record: the committed `@1.0.0` component fails against the extended host with
`expected record of 11 fields, found 8 fields`. Record growth is therefore **not
additive** and cannot be part of a compatibility-preserving recall-only `@1.1`
cutover.

### Evidence and commands

Full transcripts:

- `evidence/q3-frame-bridge.log`
- `evidence/q3-wit-surface.log`
- Q2 tax/compile transcript: `evidence/q2-frame-growth-tax.log`

The bridge was built and run with:

```bash
cargo run --offline --release --manifest-path spikes/story-17-3a-wasm-recall/bridge/Cargo.toml --bin frame-bridge-probe
```

It reads the frame WIT, real domain/codec crates, and committed fixture; each
transcript records that it does not merely inspect data it set itself.

Excerpt (10 lines):

```text
extended_cbor_equal=true bytes=1025
head_control_cbor_equal=false head_bytes=476
head_control_defaults=readonly/none/empty-lineage/empty-scope
frame_growth_skew=type-mismatch
frame_growth_error=type-checking export func `handle-frame`

Caused by:
    expected record of 11 fields, found 8 fields

[exit=0]
```

### Measured result and price

- **Lossless bridge.** `bridge/src/bridge.rs.txt` is a spike copy of
  `lower`/`lift`, extended for `IntentClass`, `ConsentEnvelope`,
  `IntentLineage(Vec<A2AIntent>)`, and typed `Scope`. The probe populated all
  four, including five scope variants (filesystem, MCP, CLI hash, gateway,
  Loom), and observed equal canonical CBOR (1,025 bytes). Its same-target HEAD
  control observed unequal CBOR (476 bytes) and exactly
  `readonly/none/empty-lineage/empty-scope`.
- **WIT surface.** The executed surface parser observed **25** records:
  baseline 16 → recall-only 21 → frame WIT 25. `iac-frame` grows **8 → 11**
  fields (`intent`, `consent-envelope`, `intent-lineage`); `task-assign-body`
  remains 5 fields but its `scope` changes from `list<string>` to
  `list<scope>`. New record field counts are `consent-envelope=5`,
  `scope-mcp-call=2`, `scope-cli-subprocess-spawn=3`, and
  `scope-gateway-send=2`.
- **Production compile tax.** In the scratch HEAD host, this WIT causes the
  three measured errors at `frame_bridge.rs:216` (cannot collect `String` as
  `frames::Scope`), `:305` (a `Scope` passed to `scope_from_debug_string`), and
  `:404` (missing the three new `IacFrame` fields). Consequently both live
  equivalence legs are vacuous/red until the real bridge migration lands.
- **Version skew.** After satisfying the fixture's WASI imports with the real
  `HostState`/`add_to_linker_sync`, `bindings::Spirit::instantiate` reached the
  frame export and rejected its record shape (11 expected, 8 found). This is
  the requested component-model compatibility measurement, not a source-level
  inference.
- **FrameKindLabel choice.** Keep a *separate* `recall.frame-kind-label` for
  the current contract: the parser measured **30** label cases versus only
  **17** `frames.frame-kind` cases, so reuse would erase 13 current audit
  labels. This is GO-WITH-CONDITIONS, not an open-ended compatibility claim:
  the source `FrameKindLabel` is non-exhaustive but a WIT enum is closed, so a
  future label is a WIT type change and must trigger the same fixture/version
  treatment. The existing corpus has no enum-case count gate for this label;
  its measured current cost is record-total +9 from baseline and the fixture
  reissue described by Q2.
- **Lines/dependencies/kernel.** The WIT copy is 383 lines; the spike bridge
  is 575 `.rs.txt` lines and the probe is 150 `.rs.txt` lines. Kernel-Δ is 0.
  Spike-only direct dependencies are path `maos-domain`, `maos-spirit-abi`,
  `maos-wasm-host`, plus `wasmtime` and `wasmtime-wasi` pinned to 46.0.3; no
  workspace member receives a dependency.
- **CI.** No CI run was made. The existing `check-wasm-form-equiv` job has a
  30-minute timeout; it needs no new step, but Q3 makes its native-twin build,
  both oracle legs, and provenance prerequisites red until production code and
  all four components are cut over.

### Consequence (OLD → NEW)

- **17-1:** OLD: no frame WIT involvement. NEW: unchanged; this result adds no
  egress or kernel work.
- **17-3b AC1 / R17-28:** OLD: a `@1.1` WIT addition is treated as compatible
  with the pinned fixtures. NEW: “the recall-only import may be isolated for
  compatibility; any frame field growth requires a separate component release
  and all four provenance-pinned components to be rebuilt/re-pinned.”
- **17-3b AC2 / R17-30:** OLD: add recall and frame fields under one additive
  claim. NEW: “separate the recall-only WIT compatibility gate from the
  breaking frame record revision; add the nine total record-table rows and
  remove the ABI-ratification claim for domain `LogRecall*` per Q2’s measured
  `maos-spirit-abi` public-API result.”
- **17-3b AC5 / R17-29:** OLD: leave the equivalence tier wording unchanged.
  NEW: “run the real bridge migration and component reissue before requiring
  the two equivalence legs; type-check failure must remain red rather than
  being treated as advisory.”
- **R17-31:** OLD: TS component tooling may target the same WIT without an
  explicit record-revision plan. NEW: the TS release path must target the
  recall-only compatible world or deliberately publish/retest the new
  frame-record component version; do not mix those claims.
- **17-6:** OLD: no Q3 dependency. NEW: unchanged; Q3 measured no sandbox
  policy, process launch, or kernel line.

### Findings

- **17-3b:** WIT record field additions are a type replacement, not an
  additive import; retain the recall-only and frame-growth WIT variants as
  separate decisions until a full component-versioning plan is approved.
- **17-3b:** `Scope` is `#[non_exhaustive]`; the spike rejects an unknown
  future scope rather than silently lowering it. A production WIT projection
  needs an explicit future-variant/version policy.
- **20-3a:** `check-wasm-form-equiv` reports its legs as vacuous when bridge
  compilation fails; its red disposition is correct, but the error should be
  surfaced as a compile-precondition failure in story evidence.

### Reproduce

From the repository root:

```bash
cargo run --locked --offline --release --manifest-path spikes/story-17-3a-wasm-recall/bridge/Cargo.toml --bin frame-bridge-probe
python3 -c 'import pathlib; assert pathlib.Path("spikes/story-17-3a-wasm-recall/wit-1.1-frame/spirit.wit").exists()'
```

The first command proves the CBOR control and `11 fields` / `8 fields` skew
verdict. The WIT cardinality values are in `evidence/q3-wit-surface.log`.
