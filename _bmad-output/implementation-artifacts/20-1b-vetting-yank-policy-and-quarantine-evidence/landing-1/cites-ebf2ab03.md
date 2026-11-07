# 20-1b L1 — citations re-measured from ebf2ab03

Baseline `ebf2ab03ac64de5b828d995f72ae8b24199fa9a0`; resumed/current HEAD `bdfc143041613473e7a5f564de8fe30af7c2b0f7` plus the uncommitted L1 tree. Historical a4c00897/26801 kernel cites are not reused as current measurements. `cites-ebf2ab03.json` records exact git-show commands, exit codes, source SHA-256 and physical counts. Positions below are measured declaration/branch locations, not inferred runtime results. New test definitions and observed red/final witnesses are in the story Dev Agent Record.

| Source | Declaration / branch | ebf2ab03 line | Current line |
|---|---|---:|---:|
| `crates/maos-compliance/src/vetting/attestation.rs` | `pub struct VettingClaim` | 105 | 105 |
| `crates/maos-compliance/src/vetting/attestation.rs` | `pub artifact_sha256:` | absent | 110 |
| `crates/maos-compliance/src/vetting/attestation.rs` | `pub publisher_pubkey:` | absent | 112 |
| `crates/maos-compliance/src/vetting/attestation.rs` | `pub fn issue_attestation` | 150 | 154 |
| `crates/maos-compliance/src/vetting/attestation.rs` | `pub fn verify_attestation_signature` | 167 | 171 |
| `crates/maos-compliance/src/vetting/mod.rs` | `pub struct VettingSubject` | absent | 197 |
| `crates/maos-compliance/src/vetting/mod.rs` | `pub fn verify_attestation` | 197 | 220 |
| `crates/maos-compliance/src/vetting/mod.rs` | `pub fn evaluate_upgrade_precondition` | 280 | 309 |
| `crates/maos-compliance/src/vetting/keyring.rs` | `pub fn push_attestation_revocation` | 157 | 157 |
| `crates/maos-compliance/src/vetting/keyring.rs` | `pub fn verify_enrollment` | 190 | 190 |
| `crates/maos-compliance/src/vetting/keyring.rs` | `pub fn verify_attestation_not_revoked` | 288 | 288 |
| `crates/maos-compliance/src/vetting/keyring.rs` | `pub fn is_revoked` | 334 | 334 |
| `crates/maos-compliance/src/vetting/terminal.rs` | `pub struct RunningSpiritObservation` | 99 | 99 |
| `crates/maos-compliance/src/vetting/terminal.rs` | `pub fn observe_running_spirit` | 152 | 152 |
| `crates/maos-registry/src/admission.rs` | `pub fn admit_spirit_with_attestation` | 354 | 354 |
| `crates/maos-registry/src/admission.rs` | `pub fn verify_publisher_sig` | 549 | 549 |
| `crates/maos-registry/src/storage.rs` | `pub fn at_path` | 235 | 235 |
| `crates/maos-registry/src/storage.rs` | `fn check_publisher` | absent | 373 |
| `crates/maos-registry/src/storage.rs` | `pub(crate) fn install_package` | 356 | 418 |
| `crates/maos-registry/src/storage.rs` | `fn publish_with_origin(` | 72, 528 | 72, 632 |
| `crates/maos-registry/src/storage.rs` | `fn yanks_since(` | 69, 674 | 69, 808 |
| `crates/maos-domain/src/ports/registry.rs` | `pub struct YankEntry` | 334 | 348 |
| `crates/maos-domain/src/ports/registry.rs` | `pub enum YankOrigin` | absent | 341 |
| `crates/maos-domain/src/ports/registry.rs` | `pub enum RegistryError` | 378 | 404 |
| `crates/maos-bin/src/admission.rs` | `pub fn bind_registry_publisher` | absent | 483 |
| `crates/maos-bin/src/admission.rs` | `pub fn bind_registry_package_at` | absent | 498 |
| `crates/maos-bin/src/admission.rs` | `pub async fn load_admit_start` | 923 | 974 |
| `crates/maos-bin/src/main.rs` | `fn enforce_vetted_upgrade_precondition` | 130 | 130 |
| `crates/maos-kernel-core/src/revocation/applier.rs` | `pub async fn apply_crl` | 73 | 73 |
| `crates/maos-kernel-core/src/revocation/applier.rs` | `RevocationAction::DrainThenTerminate \| RevocationAction::Quarantine =>` | 160 | 160 |
| `crates/maos-kernel-core/src/memory/private.rs` | `pub fn bind_storage_identity` | 81 | 81 |

The measured kernel source hashes match. check-kernel-baseline independently reports the unchanged 95-file set hash and 26841 physical lines. The R forecast is carried from the reverted historical prototype (+78 physical/+68 tokei), NOT a new prototype run: current26841/21042 → forecast26919/21110; D02 A ×2 ceilings26997/21178 stay below shared21250 tokei STOP. L1 changes no kernel source.
