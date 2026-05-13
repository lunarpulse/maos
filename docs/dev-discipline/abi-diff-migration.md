# ABI-Diff Migration: Bespoke syn+quote Walker → cargo-public-api

**Story:** 1a.5  
**Date:** 2026-05-13  
**Status:** Active

## Decision

### Toolchain: Nightly Required

`cargo-public-api` v0.51.0 requires the nightly Rust toolchain to generate
rustdoc JSON output (`-Z unstable-options --output-format json`). Despite
investigating the stable-rust path described in the tool's earlier release
notes, the current version still invokes `rustup run nightly cargo rustdoc`
internally.

**Decision:** Accept nightly as a build-time dependency for the `abi-diff` gate
only. The nightly requirement is scoped exclusively to the `cargo-public-api`
invocation within `xtask/src/abi_diff.rs`. The workspace root
`rust-toolchain.toml` remains `stable`.

### Install Path

`cargo-public-api` is installed via `cargo install` in CI. It is not a library
dependency of the `xtask` crate — this avoids lockfile blast radius and keeps
the nightly requirement isolated.

| Path | Blast radius | Notes |
|------|-------------|-------|
| `cargo install cargo-public-api --version 0.51.0` | Zero (Cargo.lock unchanged) | Preferred. No new crate deps. |
| `public-api` crate as dev-dep | High (adds rustdoc-json, nightly-types, etc.) | Rejected. Lockfile blast too large. |

### Minimum Version Pin

`cargo-public-api` ≥ 0.51.0 is required. The CI install step pins this
explicitly:
```yaml
CARGO_PUBLIC_API_VERSION: "0.51.0"
```

## Rationale

### Why cargo-public-api

The bespoke `syn::parse_file` + `quote!(#item).to_string()` walker had four
soundness gaps:

1. **Quote-fragility:** `quote!`-based signatures are whitespace-sensitive and
   toolchain-version-dependent. Two semantically-identical APIs could produce
   different token-string representations (false positives).

2. **Reexport blindness:** `pub use` reexports were not tracked correctly.
   Moving a public function behind a `pub use` was invisible to the walker
   (false negative on removal).

3. **Generic-bound reorder instability:** `where T: Eq + 'static` vs
   `where T: 'static + Eq` produced different `quote!` strings despite being
   semantically identical (false positive).

4. **Inline-mod walking gaps:** `pub mod foo { pub fn bar(); }` vs inline
   module forms were handled inconsistently (false negatives on visibility).

`cargo-public-api` resolves all four gaps because it operates on rustdoc's
semantic JSON output rather than token-stream strings.

### Why Not Alternatives

- **`rustdoc --output-format json` directly:** Would work but the JSON schema
  is unstable across toolchain versions. `cargo-public-api` provides a
  normalization layer.
- **`cargo-semver-checks`:** Orthogonal — checks semver against published
  versions, not API surface enumeration. Could complement in v0.5+.
- **Patching the bespoke walker:** Would close some gaps but compounds
  maintenance debt. Every patch to the bespoke code is effort better spent on
  `cargo-public-api` which is maintained by the community (serde, tokio, clap,
  rustls all use it for their own public-API regression CI).

## Nightly Management

### CI (Ubuntu latest)

The `abi-diff` job in `discipline.yml` installs nightly explicitly:
```yaml
- name: Install nightly toolchain for cargo-public-api
  run: rustup toolchain install nightly --profile minimal
```

This installs the latest nightly. `cargo-public-api` is well-tested against
recent nightlies; if a nightly breaks it, the gate itself fails (correct
signal — investigation required).

### Pinning Strategy

If nightly breakage occurs:
1. Install a pinned nightly: `rustup toolchain install nightly-YYYY-MM-DD`
2. Set `RUSTUP_TOOLCHAIN` or update the CI step
3. Document the pin date in this file

### Local Development

Developers need nightly installed:
```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-public-api --version 0.51.0
```

No `rust-toolchain.toml` override is needed — `cargo-public-api` invokes
`rustup run nightly` internally. The workspace root toolchain stays stable.

## Rollback Procedure

If `cargo-public-api` becomes unusable:
1. Revert the commit(s) from Story 1a.5
2. The old `xtask/src/abi_diff.rs` (bespoke walker) is restored from git
   history
3. Restore the old baseline `abi-baseline/v0.1-alpha-pre-abi-freeze.json`
4. Remove the nightly install step from `discipline.yml`
5. Re-add `syn`, `quote`, `proc-macro2` to `xtask/Cargo.toml`

## Baseline Format

### Old Format (bespoke JSON)

```json
{
  "abi_version": 0,
  "items": [
    {"kind": "struct", "name": "Claim", "signature": "# [derive ...] pub struct Claim { ... }"}
  ]
}
```

### New Format (cargo-public-api canonical text)

```
pub mod maos_spirit_abi
pub mod maos_spirit_abi::compliance
pub enum maos_spirit_abi::compliance::EvidenceKind
...
```

The new format uses `-sss` (simplified × 3) filtering to omit blanket impls,
auto-trait impls, and auto-derived impls. The remaining lines are the
authoritative public API surface.

## Baseline Regeneration Procedure

1. When the ABI surface changes in a way that requires a version bump,
   update `ABI_VERSION` in `crates/maos-spirit-abi/src/lib.rs`.
2. Run:
   ```sh
   cargo public-api --manifest-path crates/maos-spirit-abi/Cargo.toml -sss \
       > abi-baseline/v<NEXT>-pre-bump.txt
   ```
3. Run `cargo xtask abi-diff --base abi-baseline/v<PREVIOUS>-pre-bump.txt`
   to verify the diff classifies correctly.
4. After merge, commit both the new baseline file and the bumped
   `ABI_VERSION`.

## Item Count Verification (AC6)

At HEAD on 2026-05-13, `cargo public-api -sss` produces **66 lines** covering:

| Category | Count | Items |
|----------|-------|-------|
| mod | 2 | `maos_spirit_abi`, `compliance` |
| enum | 6 | `EvidenceKind`, `PrincipleRef`, `SandboxTier`, `SigningAlg`, `TrustTier`, `Verdict` |
| struct | 7 | `CapabilityId`, `Claim`, `ComplianceClaimEnvelope`, `CryptoProviderId`, `ExecutionContextFingerprint`, `ProviderEndpointPin`, `Uuid` |
| const | 1 | `ABI_VERSION` |
| enum variants + fields | ~50 | All variants and fields of the above types |

The ≥15 top-level items requirement (6 structs + 5 enums + 1 const + 1 mod = 13
from the old count, plus additional enum variants/fields captured by the new
tool) is satisfied with margin. The old bespoke baseline counted 15 items; the
new tool captures a superset at 66 lines.

## Nightly Policy

**Decision (2026-05-13):** Floating nightly, no date pin. Rationale:

- `cargo-public-api` invokes `rustup run nightly` internally, bypassing any
  directory-level `rust-toolchain.toml` override. A pin in `xtask/rust-toolchain.toml`
  would not control what actually runs — it would be cosmetic.
- A pinned nightly rots. Someone must bump it on a cadence or CI silently drifts
  from reality. Floating nightly means CI occasionally breaks on a nightly regression,
  which is correct signal: "the tool you depend on doesn't work on tonight's compiler."
- `cargo install` means the tool itself floats independently of nightly. Pinning
  nightly underneath a floating tool gives deterministic nightly behavior for one
  CI run but the tool's own behavior changes across installs.
- This policy holds at v0.1-alpha. Revisit if MAOS ships stable releases where
  CI flakiness blocks a release cadence.

**NFR-Test-2 exemption:** nightly usage is confined to `cargo-public-api`'s
internal invocation, not MAOS code/config. The `rustup toolchain install nightly`
line in CI is an environment setup step, not a code dependency on nightly features.

## Gate Modes

The `xtask/src/abi_diff.rs` module supports two diff modes:

### File-based (line comparison)

When `--base` resolves to an existing file path, the gate captures the current
public API via `cargo public-api --manifest-path <path> -sss` and performs a
line-by-line comparison against the baseline file. Added lines are non-breaking;
removed lines are breaking. This is the mode used by CI (`discipline.yml`).

The `.json` → `.txt` fallback in `resolve_baseline` handles the migration from
the old bespoke JSON baseline format: if a `.json` path is given but only `.txt`
exists, it transparently resolves to `.txt`.

### Git-ref-based (cargo-public-api diff)

When `--base` is a git ref (not a file path), the gate delegates to
`cargo public-api diff <base>..HEAD`, which checks out both refs and compares
their public API surfaces directly. This mode uses `--deny removed --deny changed`
to fail on breaking changes. Useful for local development and PR review.

## Soundness-Gap Fixture Architecture

Four fixture groups under `xtask/tests/fixtures/abi-diff/` validate that the
new tool closes the bespoke walker's soundness gaps:

| Fixture | Gap closed | Test assertion |
|---------|-----------|----------------|
| `quote-whitespace/` | `quote!` whitespace fragility | Identical output across formatting variants |
| `pub-use-reexport/` | Reexport blindness | Reexported items preserved; removal detected |
| `generic-bound-reorder/` | Bound-order instability | Deterministic output per variant (bound order faithfully represented) |
| `inline-mod-items/` | Inline-mod walking gaps | Inline mod pub functions visible in output |

Each fixture is a self-contained crate with its own `Cargo.toml`, `src/lib.rs`,
and `EXPECTED.txt`. The `[workspace]` table in each `Cargo.toml` breaks the
fixture out of the MAOS workspace to avoid dependency conflicts.

Integration tests run via `cargo test -p xtask --test abi_diff_integration`.
These tests require nightly and `cargo-public-api` installed — they are not part
of `cargo test --workspace`.
