//! Story 7.5a precursor — v1 manifest loads on v2 kernel (cross-crate path).
//!
//! Companion to `crates/maos-spirit-abi/tests/manifest_n_minus_1_test.rs`
//! (which pins the kernel-side constant invariants). This file exercises the
//! manifest *validation* path:
//!
//! - A `[class]` section authored at `manifest_schema_version = 1` (the Epic
//!   1b baseline) MUST load successfully on a kernel at
//!   `MANIFEST_SCHEMA_VERSION = 2` (Epic 6 §A4 bump).
//! - A `[class]` section at the kernel's current `MANIFEST_SCHEMA_VERSION`
//!   MUST load successfully.
//! - A `[class]` section above `MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION` MUST
//!   be rejected with a `ManifestError::Toml` mentioning
//!   `class.manifest_schema_version`.
//! - A `[class]` section below `MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION` MUST
//!   be rejected with the same error class.

use maos_manifest::{ClassSection, ManifestError};
use maos_spirit_abi::{
    MANIFEST_SCHEMA_VERSION, MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION,
    MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION,
};

fn class_toml_with_schema_version(v: u32) -> String {
    format!(
        r#"
name = "hello-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = {v}
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "MAOS reference Spirit"
"#,
    )
}

#[test]
fn v1_manifest_loads_on_current_kernel() {
    // The Epic 1b baseline manifest (manifest_schema_version = 1) MUST load
    // on every kernel released after Epic 1b — this is the Story 7.5a N-1
    // supported floor commitment in operational form.
    let toml = class_toml_with_schema_version(MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION);
    let c = ClassSection::from_toml_str(&toml).expect(
        "Story 7.5a N-1 supported floor violated — v_MIN manifest rejected by current kernel",
    );
    assert_eq!(c.manifest_schema_version, MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION);
}

#[test]
fn current_schema_version_manifest_loads() {
    let toml = class_toml_with_schema_version(MANIFEST_SCHEMA_VERSION);
    let c = ClassSection::from_toml_str(&toml)
        .expect("kernel refused the manifest_schema_version it itself emits");
    assert_eq!(c.manifest_schema_version, MANIFEST_SCHEMA_VERSION);
}

#[test]
fn above_max_schema_version_rejected() {
    let above = MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION + 1;
    let toml = class_toml_with_schema_version(above);
    let err = ClassSection::from_toml_str(&toml).unwrap_err();
    assert!(
        matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.manifest_schema_version")),
        "expected class.manifest_schema_version validation error, got {err:?}",
    );
}

#[test]
fn zero_schema_version_rejected() {
    // Schema version 0 is the canonical TOML-default sentinel — must always
    // fail validation regardless of how MIN_SUPPORTED evolves over time
    // (MIN_SUPPORTED is u32-typed; 0 is always strictly below any future MIN).
    let toml = class_toml_with_schema_version(0);
    let err = ClassSection::from_toml_str(&toml).unwrap_err();
    assert!(
        matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.manifest_schema_version")),
        "expected class.manifest_schema_version validation error, got {err:?}",
    );
}

#[test]
fn v2_manifest_loads_on_v2_kernel() {
    // Direct guard for the Epic 6 §A4 bump: at v2, manifests authored against
    // the four new sections ([[cli_wrapper]], [[schedules]], [gateways],
    // ConsentEnvelope.intent_class/valid_until_ns) must load.
    // This test pins the v2 contract independently of MANIFEST_SCHEMA_VERSION
    // continuing to climb; when v3 lands, this test still verifies v2 loads.
    let toml = class_toml_with_schema_version(2);
    let result = ClassSection::from_toml_str(&toml);
    if MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION <= 2 && 2 <= MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION {
        let c = result.expect("v2 manifest rejected within supported window");
        assert_eq!(c.manifest_schema_version, 2);
    } else {
        // v2 is outside the window — must fail.
        assert!(result.is_err());
    }
}
