# Windows Release — Deferred Until Explicit Operator Command

**Status:** NOT SUPPORTED FOR RELEASE. On 2026-10-15, the operator explicitly deferred Windows release until a subsequent explicit command authorizes it. No version milestone or successful compile check automatically lifts this hold.

## Current release boundary

`.github/workflows/release.yml` builds, packages, signs and publishes only:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `aarch64-apple-darwin`

Windows targets are absent from the release matrix, signed manifest target list and published artifact list. Do not add Windows binaries, installers or package-manager publication without the operator's explicit release command.

## Implementation is not release support

Story 10.5 recorded Windows-specific implementation work. The previous version of this document incorrectly claimed that `x86_64-pc-windows-msvc` had been added to the release matrix; it has not. Existing Windows code or compile-only checks do not constitute a supported Windows release.

Windows compilation and packaging are not acceptance gates for Story 17-7's Linux T3 lifecycle work. Keep platform guards and unsupported-platform responses; do not claim Windows verification from Linux-only tests.

## Reopening the release

The owner is the operator. The sole reopening trigger is an explicit operator command to resume Windows release work. At that point, verify the native build and sandbox behavior, define the packaging/signing/publication path, and update the release matrix and this support statement together. There is no current MSI, winget or Scoop delivery commitment.

## References

- `.github/workflows/release.yml`
- Story 10.5 AC3: historical Windows implementation work
- Story 17-7: Linux T3 retained lifecycle and terminal facts
- ADR-004: hexagonal sandboxing with OS-native primitives
