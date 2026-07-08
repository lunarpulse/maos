# SBOM + Signing-Chain Verification Runbook (Story 10.2, F6→C)

This runbook describes the **operational** verification a CI bot (or operator)
performs for each participant Spirit after the trial: re-loading the binary on a
clean VM, checking SBOM completeness against the lockfile, and verifying the
signing chain against the trusted root.

> **v1.0/v1.5 scope (F6→C):** these checks are **operational
> verification, not automated gate logic.** They are recorded per participant
> as `sbom_verified` and `signing_chain_verified` in `trial-results.toml`, but
> `check-third-party-trial` does not assert them as blocking before v2.0. They
> surface supply-chain health; the v1.0/v1.5 ship decision is carried by the
> count floors and per-participant success conjunction.

## 1. Re-load the signed Spirit binary on a clean VM

1. Provision a fresh Host VM with no prior MAOS state.
2. Install the substrate and verify the control plane is runnable:

   ```sh
   maosctl install
   maosctl --version
   ```

3. Load the participant's Spirit binary from local artifacts and confirm the
   kernel accepts it (offline-capable, same idiom as release verification):

   ```sh
   maosctl install --from-local . --verify-only
   ```

4. Run the Spirit and confirm it loads against the kernel (`binary_loads`) and
   reaches the ≥1000-frame bar before recording the result.

A binary that fails to load or verify here is recorded `binary_loads = false`
and fails the per-participant success conjunction — it does **not** advance to
the SBOM/signing steps below.

## 2. Verify SBOM completeness

1. Locate the SBOM emitted alongside the Spirit binary (CycloneDX/SPDX document
   for the Spirit's dependency closure).
2. List every declared runtime + build dependency in the SBOM.
3. Compare the declared set against the Spirit's resolved dependency graph:

   ```sh
   cargo tree -p <spirit-crate> --locked
   ```

   and against the workspace lockfile (`Cargo.lock`) for the transitive closure.
4. The SBOM is **complete** when every package in `cargo tree` / `Cargo.lock`
   appears in the SBOM with a matching name and version. Record
   `sbom_verified = true`; otherwise record `false` and file the gap as a public
   issue (the friction becomes part of the durable record).

## 3. Verify the signing chain against the trusted root

1. Identify the signature attached to the Spirit binary (or the `SHA256SUMS` +
   `.sig` pair, per the release-signing idiom — `sha256(content) → Ed25519`).
2. Verify against the trusted root public key bundled in the binary
   (`crates/maos-audit/src/release_verify.rs::RELEASE_PUBKEY`):

   ```sh
   cargo run -p xtask -- release-verify --verify \
     --sha256sums SHA256SUMS \
     --sig SHA256SUMS.sig \
     --artifacts-dir .
   ```

3. Confirm the signing key traces to the trusted root (no untrusted
   intermediates; no expired or revoked keys per the key-rotation policy in
   `docs/runbooks/release-signing.md`).
4. Record `signing_chain_verified = true`; a break in the chain records `false`
   and triggers the emergency-rotation steps in the release-signing runbook.

> **Reminder for v1.0/v1.5:** steps 2 and 3 populate advisory booleans only.
> At those tiers the trust boundary is PR review (F4→A), not cryptographic
> signing — see `README.md` §5. v2.0 changes this via ADR-053.

## 4. v2.0: CI-bot-derived attestation is asserted, not advisory

ADR-053 graduates this runbook from human-path evidence to a machine-derived
attestation seam at v2.0:

1. A fresh CI runner starts with no prior MAOS state.
2. The producer gate re-loads the candidate and derives `binary_loads` and
   `frames_run` from execution, not from the participant file.
3. SBOM completeness is derived by reconciling the candidate-shipped
   `Cargo.lock` against an independently recomputed `cargo tree --locked`
   closure, plus the existing dependency-closure deny policy. CycloneDX/SPDX
   emission remains deferred to v2.5; the v2.0 bill of materials is the
   reconciled closure.
4. Signing-chain truth is derived by `maos_audit::release_verify` against
   `RELEASE_PUBKEY`.
5. Halt recall is derived through `maos-eval` scoring over the
   class-appropriate subset and records the corpus SHA/provisional stamp.
6. The producer emits participant records with
   `derivation_provenance = "maos-trial-attestation-v2"`.

At `MAOS_SHIP_PHASE=v2_0`, `check-third-party-trial` default-denies any
present participant record lacking that provenance stamp and folds
`sbom_verified` + `signing_chain_verified` into the participant success
conjunction. Hand-authored v2.0 TOML is therefore rejected; v1.0/v1.5 TOML stays
valid for its original advisory SBOM/signing boundary.
