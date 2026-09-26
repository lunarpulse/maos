# Q2 additive WIT

## Verdict — GO-WITH-CONDITIONS

The recall-only `maos:spirit@1.1.0` WIT parses and lets the existing HEAD
`maos-wasm-runner` compile, and the live `check-wasm-form-equiv` oracle stayed
green. It is **not free/additive at the repository level**: it needs five
`wit_corpus.rs` record rows, all four provenance source hashes change, and the
current HEAD rebuild cannot reproduce any pinned component hash. This is a
measured conditional GO for a recall-only revision, not a GO for Q3 record
growth (see the Q3 report).

### Evidence and commands

Full transcripts:

- `evidence/q2-recall-only-tax.log`
- `evidence/q2-frame-growth-tax.log`
- `evidence/q2-recall-wit-surface.log`
- `evidence/q2-fixture-rebuild.log`
- `evidence/q2-abi-diff-scope.log`

The scratch probe ran `cargo test --locked -p maos-wasm-host --test
wit_corpus`, `cargo run --locked -p xtask -- check-equiv-fixture-provenance
--json`, the text and CI-JSON forms of `check-wasm-form-equiv`, its native-twin
build, the rest of `cargo test --locked -p maos-wasm-host`, and
`cargo build --locked -p maos-wasm-host --bin maos-wasm-runner`. Each log starts
with UTC/HEAD and identifies whether it read the tree/runtime rather than only
probe-set data.

Excerpt (recall-only; 13 lines):

```text
$ cargo test --locked -p maos-wasm-host --test wit_corpus
thread 'corpus_covers_all_record_types' ...
assertion `left == right` failed: must cover all 16 record types ...
left: 21
right: 16
[exit=101]
$ cargo run --locked -p xtask -- check-equiv-fixture-provenance --json
... FAIL — 4 fixture drift/mismatch(es):
- echo_spirit_component.wasm: source_sha256 drift ... actual=4fec...55e49
- equiv_identity_spirit_component.wasm: source_sha256 drift ... actual=329c...703d5
- equiv_divergent_spirit_component.wasm: source_sha256 drift ... actual=33ee...e8e62
- equiv_cosmetic_spirit_component.wasm: source_sha256 drift ... actual=95e4...2e43f9
[exit=1]
check-wasm-form-equiv: PASSED — oracle green (base: 20 passed/0 failed, anti-canned: 23 passed/0 failed)
```

### Measured price

- **WIT/corpus.** The recall-only WIT is 331 lines. The six current corpus
  gates are unchanged at `frame-kind=17`, `frame-origin=4`,
  `posture-hint=3`, `rupture-reason=5`, and `frame-payload=11`; only the
  record-total gate at `wit_corpus.rs:156` moves **16 → 21**. Add the exact
  field-table rows: `recall-cursor=2`, `recall-filter=6`, `recall-entry=6`,
  `recall-page=2`, and `fetch-response=7`. The parser probe observed all 21
  records and those field cardinalities.
- **Provenance.** All four and only four pinned rows change. The recall-only
  source SHA-256 values observed in the scratch worktree are: echo
  `4fec8fdd00aecdbb993cf6963e317f445e9e7b17c9d0ac747676eefe1dd55e49`, identity
  `329c95e16df2e9e43f87d502d8bfc923595645364dab516ab42918e587b703d5`, divergent
  `33ee50b9843fedf8ba4c8aaee58c40c2decdc8be507e2c3876aaf994db7e8e62`, cosmetic
  `95e44482d5bab42844fa775897ac0197e22bba8986a89e757f7fe9605b2e43f9`.
- **Fixture rebuild.** With Rust/Cargo `1.98.1`, wasm target `wasm32-wasip2`,
  `wit-bindgen 0.44.0`, and `wasm-tools 1.259.0`, all four builds succeeded but
  none matched `wasm_sha256`: echo `a41ad244…aba2832` (69,735 bytes), identity
  `c72ac4c6…5ca9ef` (69,794), divergent `9a095976…bcf120` (69,794), cosmetic
  `15aebcf6…7f171b` (69,833). `wasm-tools validate` accepted pinned and rebuilt
  echo components. **Why they differ is NOT-MEASURED:** the committed
  provenance records source and blob hashes, not the original compiler/toolchain;
  producer-marker inspection found none in either component.
- **P4 compile answer.** Recall-only did **not** force a host trait
  implementation at compile time: `cargo build --locked -p maos-wasm-host
  --bin maos-wasm-runner` exited 0 (`Finished dev profile ... [exit=0]`), so
  its exact compile-error text is **none**. The runner/old fixture invocation
  was not used to infer support for a new importing guest. Frame growth did
  fail with these exact error headlines:

  ```text
  error[E0277]: a value of type `Vec<frames::Scope>` cannot be built from an iterator over elements of type `std::string::String`
  error[E0308]: mismatched types
  expected reference `&str`
     found reference `&frames::Scope`
  error[E0063]: missing fields `consent_envelope`, `intent` and `intent_lineage` in initializer of `frames::IacFrame`
  error: could not compile `maos-wasm-host` (lib) due to 3 previous errors
  ```

  They are respectively at `frame_bridge.rs:216`, `:305`, and `:404`; the
  frame form gate was red/vacuous (`ran=false`, `passed=0`, `failed=0`).
- **Code/dependencies/kernel.** This spike adds no production kernel lines
  (kernel-Δ 0) and no production dependency. The Q3-only spike bridge has
  575 `.rs.txt` conversion lines plus 150 probe lines; its direct spike-only
  dependencies are path `maos-domain`, `maos-spirit-abi`, `maos-wasm-host` and
  `wasmtime`/`wasmtime-wasi =46.0.3`.
- **CI.** No CI run was made. The existing `check-wasm-form-equiv` job has a
  30-minute timeout and five relevant steps (native twin, oracle, anti-canned,
  release-graph, provenance); a WIT edit adds no step but makes its provenance
  step red until every affected fixture is reissued.
- **P6 / abi-ratification.** **Measured moot for `LogRecall*`:** after
  installing `cargo-public-api 0.52.0` and its required
  `rustc 1.100.0-nightly (f7575a9da 2026-09-24)`, the complete public API of
  `crates/maos-spirit-abi/Cargo.toml` was emitted (155,998 bytes) and contains
  neither `LogRecall` nor `LogFetch`. `xtask abi-diff --base HEAD --json` then
  exited 0 with no changed public API. The 17-3b AC2 ratification clause cannot
  cover domain `LogRecall*`; `wit_corpus`/provenance are the applicable WIT
  controls.

### Consequence (OLD → NEW)

- **17-1:** OLD: no WIT/provenance obligation. NEW: unchanged; Q2 adds no
  17-1 code or kernel cost.
- **17-3b AC1 / R17-28:** OLD: “8 tracked guests rebuilt.” NEW: “reissue the
  four provenance-pinned components (echo, identity, divergent, cosmetic),
  record a reproducible build toolchain, and bind the caller channel; the other
  four core-WASM fixtures are outside provenance.” This changes the premise
  corrected by §R5 R17-28.
- **17-3b AC2 / R17-30:** OLD: add recall and an ABI-ratification claim. NEW:
  “add the five corpus rows and update four provenance rows; remove the
  `abi-ratifications.toml` clause for domain `LogRecall*`, because the measured
  `maos-spirit-abi` public API has neither recall type. Keep WIT completeness
  and provenance as the controls.”
- **17-3b AC5 / R17-29:** OLD: advisory equivalence wording is enough. NEW:
  “keep both live oracle legs non-vacuous and reissue components before the
  existing provenance step; a frame-shape edit also requires the bridge fix
  before the oracle can run.”
- **17-6:** OLD: no Q2 dependency. NEW: unchanged; Q2 measured no sandbox
  path or kernel lines.

### Findings

- **17-3b:** a “scoped-nightly byte-rebuild” is documented by
  `check_equiv_fixture_provenance.rs:15-17`, but the workflow search found no
  wasm32-wasip2/component rebuild job—only a native-twin build and provenance
  check. The rebuild mechanism must be created or the documentation corrected.
- **17-3b:** frame record growth is not additive; see the Q3 report and its
  actual `expected record of 11 fields, found 8 fields` result.
- **20-3a:** before this probe installed the tool, `abi-diff` reduced a missing
  `cargo-public-api` dependency to `{\"passed\":false,\"output\":\"\"}`. It should
  report that prerequisite explicitly rather than emitting an empty oracle.

### Reproduce

From the repository root (use private scratch targets):

```bash
git worktree add --detach /tmp/17-3a-q2 HEAD
cp spikes/story-17-3a-wasm-recall/wit-1.1/spirit.wit /tmp/17-3a-q2/wit/spirit.wit
cd /tmp/17-3a-q2
CARGO_TARGET_DIR=/tmp/17-3a-q2-target cargo test --locked -p maos-wasm-host --test wit_corpus
CARGO_TARGET_DIR=/tmp/17-3a-q2-target cargo run --locked -p xtask -- check-equiv-fixture-provenance --json
CARGO_TARGET_DIR=/tmp/17-3a-q2-target cargo build --locked --release --manifest-path crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml
CARGO_TARGET_DIR=/tmp/17-3a-q2-target cargo run --locked -p xtask -- check-wasm-form-equiv --json
CARGO_TARGET_DIR=/tmp/17-3a-q2-target cargo build --locked -p maos-wasm-host --bin maos-wasm-runner
cd -
CARGO_TARGET_DIR=/tmp/17-3a-q2-fixtures-target cargo build --locked --offline --release --target wasm32-wasip2 --manifest-path crates/maos-wasm-host/guests/echo-spirit/Cargo.toml
CARGO_TARGET_DIR=/tmp/17-3a-q2-fixtures-target cargo build --locked --offline --release --target wasm32-wasip2 --manifest-path crates/maos-wasm-host/guests/equiv-fixture/wasm-guest/Cargo.toml
CARGO_TARGET_DIR=/tmp/17-3a-q2-fixtures-target cargo build --locked --offline --release --target wasm32-wasip2 --manifest-path crates/maos-wasm-host/guests/equiv-fixture/divergent-guest/Cargo.toml
CARGO_TARGET_DIR=/tmp/17-3a-q2-fixtures-target cargo build --locked --offline --release --target wasm32-wasip2 --manifest-path crates/maos-wasm-host/guests/equiv-fixture/cosmetic-guest/Cargo.toml
sha256sum /tmp/17-3a-q2-fixtures-target/wasm32-wasip2/release/{echo_spirit,equiv_identity_spirit,equiv_divergent_spirit,equiv_cosmetic_spirit}.wasm
rustup toolchain install nightly --profile minimal
CARGO_TARGET_DIR=/tmp/17-3a-public-api-target cargo install --locked --root /tmp/17-3a-public-api cargo-public-api
PATH=/tmp/17-3a-public-api/bin:$PATH RUSTUP_TOOLCHAIN=nightly cargo public-api --manifest-path crates/maos-spirit-abi/Cargo.toml
PATH=/tmp/17-3a-public-api/bin:$PATH RUSTUP_TOOLCHAIN=nightly CARGO_TARGET_DIR=/tmp/17-3a-q2-target cargo run --locked -p xtask -- abi-diff --base HEAD --json
git worktree remove --force /tmp/17-3a-q2
```
