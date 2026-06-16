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

1. Tag a release: `git tag v0.5.0 && git push --tags`
2. CI builds `maos-linux-amd64`, `maos-linux-arm64`, `maos-darwin-arm64`
3. CI runs `check-mock-not-in-release` on each (native) binary
4. CI generates `SHA256SUMS` via `sha256sum maos-*`
5. CI signs `SHA256SUMS` via `xtask release-verify --sign`
6. CI publishes to GitHub Releases with `.sig` attached
7. CI self-verifies via `xtask release-verify --verify`

## Verification flow (operator)

```bash
# Download release artifacts to a local directory
mkdir maos-v0.5.0 && cd maos-v0.5.0
# Download: maos-linux-amd64, SHA256SUMS, SHA256SUMS.sig

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
