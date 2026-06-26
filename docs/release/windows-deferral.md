# Windows Binary Support — v1.5 (Story 10.5 AC3)

**Status:** IMPLEMENTED at v1.5 (Story 10.5 AC3, 2026-06-25).

## History

Windows binary support was deferred from v1.0 to v1.5 per the phased roadmap.
The deferral rationale (pre-v1.5):

1. Target audience at v0.5/v1.0 was Linux/macOS operators only.
2. CI cost of Windows runners.
3. Unix-specific APIs needed `#[cfg]` gating.
4. T3 container isolation assumes Linux runtime.

## v1.5 Implementation (Story 10.5 AC3)

- `platform_binary_name()` returns `maos-windows-amd64.exe` on Windows x86_64
- `windows.rs` sandbox body: T2 restricted-token via `CreateRestrictedToken` +
  per-Spirit resource caps via `win32job` Job Object
- `mod.rs` extended: `Cleanup::JobObject` drop path + `classify_exit` Windows arm
- Unix-specific APIs already `#[cfg(unix)]` gated (no changes needed)
- Kernel-core baseline re-pinned: 22574 → 22726 (+152 lines, FLAG-Winston)
- `x86_64-pc-windows-msvc` added to release/CI matrix
- Sandbox tests are `#[cfg(target_os = "windows")]` gated (won't run in Linux CI)

## Packaging (v1.5+)

- `.msi` installer + `winget` manifest: release-time CI job
- Scoop bucket: community contribution
- Ed25519 signature verification pipeline is OS-agnostic (same as Linux/macOS)

## Reference

- Story 10.5 AC3: Windows binary (v1.5)
- Story 1b.3 §AC4: per-Spirit resource caps cross-reference
- ADR-004: hexagonal sandboxing with OS-native primitives
