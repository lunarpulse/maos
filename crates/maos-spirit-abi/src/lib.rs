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
//!
//! # Version history
//!
//! | Module / constant | Introduced | Notes |
//! |---|---|---|
//! | `ABI_VERSION` | Story 1b.4 | Frozen at `1` at ComplianceClaim envelope freeze |
//! | `cancellation`, `lifecycle`, `ctx` | Story 2.1 | Additive ABI surface, no version bump |
//! | `identity` | Story 1b.1 | Wire-stable since v0.1-β |
//! | `compliance` | Story 1b.4 | Frozen schema, ABI_VERSION bump trigger |
//! | `gateway` | Story 6.5 | ADR-029 binding-v1.0 |
//! | `deprecation` | Story 7.1 | Empty-present deprecation channel |
//! | `MANIFEST_SCHEMA_VERSION = 4` | Story 13.5d | `[capabilities.required.loom]` section |
//!
//! ## Modules
//! | Module | Description | Introduced |
//! |---|---|---|
//! | [`cancellation`] | `CancellationSignal` trait + `NeverCancel` — runtime-agnostic cancellation | Story 2.1 |
//! | [`compliance`] | `ComplianceClaim` envelope — Ed25519-signed attestation schema | Story 1b.4 (frozen) |
//! | [`ctx`] | `Ctx` type — Spirit-author-facing context for hook invocations | Story 2.1 |
//! | [`deprecation`] | `DeprecationWarning` — deprecation channel for ABI surface evolution | Story 7.1 |
//! | [`gateway`] | `GatewaySubmodule` trait + `GatewayCtx` — external messaging gateway contract | Story 6.5 (ADR-029) |
//! | [`identity`] | `SpiritId`, `HostId`, `FrameKind` — wire-stable identity and frame discrimination | Story 1b.1 |
//! | [`lifecycle`] | `Spirit` trait + `SpiritVtable` + payload types | Story 2.1 |

extern crate alloc;

pub mod cancellation;
pub mod compliance;
pub mod ctx;
pub mod deprecation;
pub mod gateway;
pub mod identity;
pub mod lifecycle;

pub use deprecation::DeprecationWarning;

/// ABI version constant for the MAOS Spirit ABI.
///
/// Bumped according to the ABI Stability Triple rules (§8.5).
///
/// **Story 1b.4 froze this at `1`** at the ComplianceClaim envelope freeze.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::ABI_VERSION;
///
/// assert_eq!(ABI_VERSION, 1);
/// ```
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
/// Bumped to `3` in Story 9.4b AC-6 (2026-06-15) to track the additive
/// `[model_provenance]` section (`covered_model_id`, `training_data_lineage`
/// — reverse-DNS-constrained, NOT free-text — `last_eval_timestamp`). The
/// section is wire-compatible at the TOML/serde layer: it is OPTIONAL on read
/// (`from_manifest_toml` returns `None` when absent), so kernels at
/// `MANIFEST_SCHEMA_VERSION = 3` still admit manifests authored at `= 2`
/// (the N-1 supported floor) — AC-11 append-only compat. Recorded as one
/// ratified `[[ratification]]` entry in `xtask/abi-ratifications.toml`.
///
/// This constant is the single authoritative source consumed by
/// `maos-manifest::ClassSection` validation and by the `xtask
/// check-manifest-schema-version` gate. Story 7.5a's ABI Stability Triple
/// `(kernel_version, abi_version, manifest_schema_version)` consumes this
/// constant directly.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::MANIFEST_SCHEMA_VERSION;
///
/// assert_eq!(MANIFEST_SCHEMA_VERSION, 4);
/// ```
pub const MANIFEST_SCHEMA_VERSION: u32 = 4;

/// Lowest manifest schema version this kernel accepts at admission.
///
/// Story 7.5a will lift this floor on each ABI bump per the N-1 supported /
/// N-2 hard-refusal policy. At v0.5-α the floor remains at `1` — Epic 1b
/// baseline manifests load unchanged.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;
///
/// fn check_manifest_version(declared: u32) -> bool {
///     declared >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
/// }
///
/// assert!(check_manifest_version(1));
/// assert!(check_manifest_version(3));
/// assert!(!check_manifest_version(0));
/// ```
pub const MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = 1;

/// Highest manifest schema version this kernel emits or accepts.
///
/// Currently equal to `MANIFEST_SCHEMA_VERSION`. The two constants stay
/// synonymous until Story 7.5a introduces an explicit N+1 acceptance window
/// for forward-compatibility experiments.
///
/// # Example
///
/// ```rust
/// use maos_spirit_abi::{
///     MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
///     MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
/// };
///
/// fn is_version_supported(v: u32) -> bool {
///     v >= MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION
///         && v <= MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION
/// }
///
/// assert!(is_version_supported(1));  // N-1 — supported
/// assert!(is_version_supported(2));  // N-1 — supported
/// assert!(is_version_supported(3));  // Current — supported
/// assert!(is_version_supported(4)); // Current — supported
/// assert!(!is_version_supported(0)); // Below floor — EAbiTooOld
/// ```
pub const MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION: u32 = MANIFEST_SCHEMA_VERSION;
