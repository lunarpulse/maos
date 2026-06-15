# Windows Binary Deferral — v1.5 (E10 Story 10.5)

**Decision:** Windows binary support is explicitly deferred to v1.5 (Epic 10 Story 10.5).

## Rationale

1. **Target audience at v0.5/v1.0:** Enterprise operators deploying MAOS run
   Linux (amd64/arm64) and macOS (arm64). Windows deployments are not part
   of the v1.0 launch cohort identified in NFR-Onb-1.

2. **CI cost:** Windows CI runners are more expensive and slower. Adding a
   Windows target to the release matrix doubles the build time without
   serving any v1.0 customer.

3. **Dependency surface:** Several workspace crates (`maos-iac`, `maos-audit`)
   use Unix-specific APIs (`std::os::unix::fs::PermissionsExt`) that would
   require `#[cfg(unix)]` / `#[cfg(windows)]` gates.

4. **Sandbox tier T3** (container isolation via Docker/Podman, Story 5.5a)
   assumes a Linux container runtime. Windows container support is a
   separate effort.

## v1.5 plan (Story 10.5)

- Add `x86_64-pc-windows-msvc` target to the release matrix
- Gate Unix-specific APIs with `#[cfg(unix)]`
- Test on Windows CI (GitHub Actions `windows-latest`)
- Ship `.msi` installer + `winget` manifest
- Scoop bucket as community contribution

## Reference

- AC-2: "Windows binary explicitly deferred to v1.5 (E10 Story 10.5) with a recorded rationale"
- Story 10.5: `10-5-mature-v1-5-skill-format-conformance-jetbrains-windows-2-year-lts-japanese-cn-s-i18n`
