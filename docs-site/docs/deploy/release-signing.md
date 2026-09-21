---
title: Release Signing
sidebar_position: 4
description: Ed25519 release artifact signing, verification, and key rotation procedures.
---

# Release Signing

MAOS release artifacts are signed with Ed25519. The signing key is **distinct** from the audit-signing key and the capability-token signing key.

> Canonical runbook: `docs/runbooks/release-signing.md`

## Key provenance

| Key | Location | Format |
|-----|----------|--------|
| Release signing (private) | CI secret `RELEASE_SIGNING_KEY` | Hex-encoded 32-byte Ed25519 seed |
| Release signing (public) | `crates/maos-audit/src/release_verify.rs::RELEASE_PUBKEY` | `[u8; 32]` const |
| Bundled in binary | Every `maos` / `maosctl` binary | Compiled-in const |

## Initial key generation

```bash
# Generate a new release-signing key
maosctl audit keygen --output /tmp/release-signing.key

# The output file contains a hex-encoded 32-byte seed.
# Store the hex string as the CI secret RELEASE_SIGNING_KEY.
# Derive the public key and update RELEASE_PUBKEY in release_verify.rs.
```

## Signing flow (automated — CI)

1. Tag the pre-release: `git tag v0.1.0-alpha.1 && git push origin v0.1.0-alpha.1`
2. CI builds `maos` and `maosctl` for `linux-amd64`, `linux-arm64`, and `darwin-arm64` (six binaries total)
   (`maos-linux-amd64`, `maosctl-linux-amd64`, `maos-linux-arm64`, `maosctl-linux-arm64`, `maos-darwin-arm64`, `maosctl-darwin-arm64`)
3. CI runs `check-mock-not-in-release` on each `maos` binary
4. CI flattens the per-binary artifacts into `dist/`
5. CI generates `SHA256SUMS` for the explicit six-binary set via `xtask release-dry-run --manifest-only`
6. CI signs `SHA256SUMS` via `xtask release-verify --sign`
7. CI self-verifies via `xtask release-verify --verify`
8. CI publishes the six binaries, `SHA256SUMS`, and `SHA256SUMS.sig`

## Verification flow (operator)

```bash
# Download release artifacts to a local directory
mkdir maos-v0.1.0-alpha.1 && cd maos-v0.1.0-alpha.1
# Download all six binaries, SHA256SUMS, and SHA256SUMS.sig

# Verify with the bundled public key (offline-capable)
maosctl install --from-local . --verify-only

# Or via xtask (CI gate)
cargo run -p xtask -- release-verify --verify \
  --sha256sums SHA256SUMS \
  --sig SHA256SUMS.sig \
  --artifacts-dir .
```

## Key rotation

1. Generate a new key pair (see "Initial key generation" above)
2. Update `RELEASE_PUBKEY` in `crates/maos-audit/src/release_verify.rs`
3. Update the CI secret `RELEASE_SIGNING_KEY` in GitHub Settings
4. Tag a new release — the new key signs the new artifacts
5. Old artifacts remain verifiable with their original key (the pubkey is bundled in the binary built at that time)

### Emergency rotation (compromised key)

1. Revoke the CI secret immediately
2. Generate a new key pair
3. Re-sign and re-publish all affected release artifacts
4. Update `RELEASE_PUBKEY` and ship a point release
5. Publish a security advisory (SECURITY.md)

## Verification algorithm

```
signature = Ed25519(sha256(SHA256SUMS_bytes))
```

This is the same `sha256(content) -> Ed25519 sign` idiom used by `sealed_export::sign_bundle` (Story 9.1 FR44). The digest is computed over the raw bytes of the `SHA256SUMS` file (not the individual file hashes).
