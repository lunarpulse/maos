//! Story 7.5a (AC2) — ABI Stability Triple enforcement at the kernel admission
//! chokepoint (`SecurityManagerAdapter::admit_spirit`).
//!
//! Proves the load-time contract:
//!   * kernel below the declared `min_substrate_version` → `ESubstrateTooOld`
//!     (FR8 — the field is finally COMPARED, not just parsed);
//!   * kernel at/above the declared minimum → admits;
//!   * `manifest_schema_version == MIN_SUPPORTED` (N-1) → admits;
//!   * `manifest_schema_version < MIN_SUPPORTED` (N-2) → `EAbiTooOld`;
//!   * `manifest_schema_version > MAX_SUPPORTED` (forward) → `EAbiTooNew`
//!     (fail-closed per the §LOCKED Design Decision — NO warn-and-ignore window);
//!   * an unparseable `min_substrate_version` → fail-LOUD (`ESubstrateTooOld`),
//!     NEVER a silent admit (the discipline-floor invariant: no
//!     `unwrap_or_default()` on a version comparison).

use std::sync::Arc;

use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::{
    CapabilitiesRequired, ClassSection, EpistemicPolicySection, PostureSection, ResourceCaps,
    SandboxConfig, SecurityError, SecurityManagerAdapter,
};
use maos_spirit_abi::{
    MANIFEST_SCHEMA_VERSION, MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
    MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
};

fn make_adapter() -> (SecurityManagerAdapter, JournalAdapter, tempfile::TempDir) {
    cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    // Verified → T0: the positive cases admit to an admittable tier.
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    policy.update(inner);
    let adapter = SecurityManagerAdapter::new(policy);
    let tmpdir = tempfile::TempDir::new().unwrap();
    let journal = JournalAdapter::open(&tmpdir.path().join("journal.ndjson")).unwrap();
    (adapter, journal, tmpdir)
}

fn empty_caps_required() -> CapabilitiesRequired {
    CapabilitiesRequired {
        provider: maos_kernel_core::security::ProviderCapabilities { complete: vec![] },
        mcp: maos_kernel_core::security::manifest::McpCapabilities { servers: vec![] },
    }
}

fn posture() -> PostureSection {
    PostureSection::from_toml_str(
        r#"default = "assistive"
allowed_max = "assistive""#,
    )
    .unwrap()
}

/// Build a `[class]` section directly (bypassing `from_toml_str`'s shape-only
/// window check) so the out-of-window N-2/forward cases can be exercised at the
/// admit chokepoint — that is the security-relevant enforcement point.
fn class_section(min_substrate: &str, schema: u32) -> ClassSection {
    ClassSection {
        name: "test-spirit".into(),
        version: "0.1.0".into(),
        abi: "1.0".into(),
        manifest_schema_version: schema,
        min_substrate_version: min_substrate.into(),
        forms: vec!["rust-inproc".into()],
        trust_tier: "local".into(),
        description: "test".into(),
    }
}

/// Minimal valid `ClassSection` for tests that need an admit-spirit call but
/// are NOT testing ABI enforcement — the values are chosen to always pass.
fn default_test_class() -> ClassSection {
    class_section("0.0.1", MANIFEST_SCHEMA_VERSION)
}

fn admit(
    adapter: &SecurityManagerAdapter,
    journal: &JournalAdapter,
    pid: u32,
    class: &ClassSection,
) -> Result<(), SecurityError> {
    adapter
        .admit_spirit(
            pid,
            "test-spirit",
            &SandboxConfig {
                tier: SandboxTier::T0,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            journal,
            &posture(),
            Some(&EpistemicPolicySection::default_open_fail()),
            None,
            None,
            None,
            None,
            None,
            Some(class),
        )
        .map(|_| ())
}

#[test]
fn kernel_below_declared_min_substrate_rejected() {
    let (adapter, journal, _tmp) = make_adapter();
    // No released kernel will ever be >= 99.0.0 at this workspace version.
    let class = class_section("99.0.0", MANIFEST_SCHEMA_VERSION);
    let err = admit(&adapter, &journal, 1, &class).unwrap_err();
    match err {
        SecurityError::ESubstrateTooOld {
            declared_min,
            kernel_version,
            ..
        } => {
            assert_eq!(declared_min, "99.0.0");
            assert_eq!(kernel_version, env!("CARGO_PKG_VERSION"));
        }
        other => panic!("expected ESubstrateTooOld, got {other:?}"),
    }
}

#[test]
fn kernel_at_declared_min_admits() {
    let (adapter, journal, _tmp) = make_adapter();
    // The running kernel version itself — `>=KERNEL` is satisfied by equality.
    let class = class_section(env!("CARGO_PKG_VERSION"), MANIFEST_SCHEMA_VERSION);
    admit(&adapter, &journal, 2, &class).expect("kernel at declared min must admit");
}

#[test]
fn kernel_above_declared_min_admits() {
    let (adapter, journal, _tmp) = make_adapter();
    // A trivially-old minimum the current kernel exceeds.
    let class = class_section("0.0.1", MANIFEST_SCHEMA_VERSION);
    admit(&adapter, &journal, 3, &class).expect("kernel above declared min must admit");
}

#[test]
fn schema_n_minus_1_admits() {
    let (adapter, journal, _tmp) = make_adapter();
    let class = class_section("0.0.1", MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION);
    admit(&adapter, &journal, 4, &class).expect("N-1 schema (MIN_SUPPORTED) must admit");
}

#[test]
fn schema_below_min_rejected_eabitooold() {
    let (adapter, journal, _tmp) = make_adapter();
    // Schema 0 is the canonical below-window sentinel (u32, always < any MIN).
    let class = class_section("0.0.1", 0);
    let err = admit(&adapter, &journal, 5, &class).unwrap_err();
    match err {
        SecurityError::EAbiTooOld {
            declared_schema,
            min_supported,
        } => {
            assert_eq!(declared_schema, 0);
            assert_eq!(min_supported, MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION);
        }
        other => panic!("expected EAbiTooOld, got {other:?}"),
    }
}

#[test]
fn schema_above_max_rejected_eabitoonew_fail_closed() {
    let (adapter, journal, _tmp) = make_adapter();
    // Forward case — fail-CLOSED per the §LOCKED Design Decision (no
    // warn-and-ignore window; the future Spirit is told to upgrade the kernel).
    let above = MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION + 1;
    let class = class_section("0.0.1", above);
    let err = admit(&adapter, &journal, 6, &class).unwrap_err();
    match err {
        SecurityError::EAbiTooNew {
            declared_schema,
            max_supported,
        } => {
            assert_eq!(declared_schema, above);
            assert_eq!(max_supported, MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION);
        }
        other => panic!("expected EAbiTooNew, got {other:?}"),
    }
}

#[test]
fn unparseable_min_substrate_fails_loud_never_admits() {
    let (adapter, journal, _tmp) = make_adapter();
    // A version the hand-rolled comparator cannot parse MUST refuse (fail-loud),
    // NOT silently admit. Guards against a `semver_range_contains(...)
    // .unwrap_or_default()` regression (false-default would refuse, but the
    // discipline floor forbids the silent-default form entirely).
    let class = class_section("not-a-version!!", MANIFEST_SCHEMA_VERSION);
    let err = admit(&adapter, &journal, 7, &class).unwrap_err();
    assert!(
        matches!(err, SecurityError::ESubstrateTooOld { .. }),
        "unparseable min_substrate_version must fail loud with ESubstrateTooOld, got {err:?}"
    );
}

#[test]
fn no_class_section_rejected_e_class_required() {
    // A classless manifest is REJECTED at admission — `class: None` is no longer
    // a skip path but a hard refusal (SecurityError::EClassRequired). The
    // parse-time gate in `load_bundle_from_file` also requires `[class]`.
    let (adapter, journal, _tmp) = make_adapter();
    let err = adapter
        .admit_spirit(
            8,
            "classless",
            &SandboxConfig {
                tier: SandboxTier::T0,
                image_pin: None,
            },
            &ResourceCaps::default(),
            &empty_caps_required(),
            None,
            &journal,
            &posture(),
            Some(&EpistemicPolicySection::default_open_fail()),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
    match err {
        SecurityError::EClassRequired { spirit_id } => {
            assert_eq!(spirit_id, "classless");
        }
        other => panic!("expected EClassRequired, got {other:?}"),
    }
}

#[test]
fn combined_substrate_too_old_and_schema_n2_checks_substrate_first() {
    // When BOTH min_substrate_version is too high AND manifest_schema_version is
    // below MIN, the substrate check (leg 1) must fire first — documenting the
    // priority ordering in `admit_spirit`. If the checks were accidentally
    // swapped, this test would catch the regression.
    let (adapter, journal, _tmp) = make_adapter();
    let class = class_section("99.0.0", 0);
    let err = admit(&adapter, &journal, 9, &class).unwrap_err();
    match err {
        SecurityError::ESubstrateTooOld { .. } => {}
        other => panic!(
            "expected ESubstrateTooOld (substrate check before schema check), got {other:?}"
        ),
    }
}
