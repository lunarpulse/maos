#![no_std]
#![forbid(unsafe_code)]

//! `maos-spirit-abi` — wire-stable types ONLY (`#![no_std]`).
//!
//! `ABI_VERSION` was bumped to `1` in Story 1b.4 when the ComplianceClaim
//! schema was frozen under the joint Mary+Winston adversarial review (see
//! `_bmad-output/planning-artifacts/compliance-claim-schema-review.md`).
//!
//! Bumping `ABI_VERSION` here is the **ABI-bump trigger** per §8.5.
//! The abi-diff gate (`--deny removed --deny changed`) is baselined at
//! `abi-baseline/v1-pre-bump.txt` after the 1b.4 freeze.
//!
//! ## Story 2.1 additive surface (does NOT bump `ABI_VERSION`)
//!
//! Story 2.1 adds:
//! - `pub mod cancellation` — `CancellationSignal` trait + `NeverCancel`
//! - `pub mod lifecycle` — `Spirit` trait (11 hooks) + `SpiritVtable<T>` + payload types
//! - `pub mod ctx` — `Ctx` Spirit-author-facing context type
//!
//! All additions are ABI-additive per §8.5 rows 7+8. `ABI_VERSION` remains `1`.

extern crate alloc;

pub mod cancellation;
pub mod compliance;
pub mod ctx;
pub mod gateway;
pub mod identity;
pub mod lifecycle;

/// ABI version constant for the MAOS Spirit ABI.
/// Bumped according to the ABI Stability Triple rules (§8.5).
///
/// **Story 1b.4 froze this at `1`** at the ComplianceClaim envelope freeze.
pub const ABI_VERSION: u32 = 1;

/// Manifest schema version currently emitted by the kernel.
///
/// Bumped to `2` in Epic 6 §A4 (retro 2026-05-28) to track the four additive
/// sections landed across Epic 6 stories 6.2 / 6.4 / 6.5:
///
/// - `[[cli_wrapper]]` (Story 6.2 — `command`, `output_shape_version`,
///   `recovery_policy`, `posture`, `shutdown_signal`).
/// - `[[schedules]]` (Story 6.4 — `id`, `cadence`, `rate_limit_per_hour`,
///   `compliance_claim_ref_hex`, `side_effect_scopes`, `payload_b64`).
/// - `[gateways]` / `[[gateway]]` (Story 6.5 — `id`, `type`, `auth_secret_ref`,
///   `inbound_routing`, gateway-specific config blocks).
/// - `ConsentEnvelope.intent_class` + `ConsentEnvelope.valid_until_ns`
///   (Story 6.4 — additive on the consent envelope shape).
///
/// All four additions are wire-compatible at the TOML/serde layer
/// (`#[serde(default)]` + `#[serde(deny_unknown_fields)]`), so kernels at
/// `MANIFEST_SCHEMA_VERSION = 2` accept manifests authored against `= 1`
/// (the N-1 supported floor enforced by `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION`).
///
/// This constant is the single authoritative source consumed by
/// `maos-manifest::ClassSection` validation and by the `xtask
/// check-manifest-schema-version` gate. Story 7.5a's ABI Stability Triple
/// `(kernel_version, abi_version, manifest_schema_version)` consumes this
/// constant directly.
pub const MANIFEST_SCHEMA_VERSION: u32 = 2;

/// Lowest manifest schema version this kernel accepts at admission.
///
/// Story 7.5a will lift this floor on each ABI bump per the N-1 supported /
/// N-2 hard-refusal policy. At v0.5-α the floor remains at `1` — Epic 1b
/// baseline manifests load unchanged.
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Highest manifest schema version this kernel emits or accepts.
///
/// Currently equal to `MANIFEST_SCHEMA_VERSION`. The two constants stay
/// synonymous until Story 7.5a introduces an explicit N+1 acceptance window
/// for forward-compatibility experiments.
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = MANIFEST_SCHEMA_VERSION;
