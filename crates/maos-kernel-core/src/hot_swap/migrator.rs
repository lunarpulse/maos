#![forbid(unsafe_code)]

//! Cross-major migration path — ADR-020 `migrate(predecessor_state)` entry point.
//!
//! When the state codec detects a cross-major version bump, the coordinator
//! invokes `run_migrator` to call the successor's `migrate()` hook, which
//! translates the predecessor's state schema to the successor's.

use std::sync::Arc;

use maos_domain::hot_swap::HotSwapError;
use maos_spirit_abi::lifecycle::MigratorError;

use crate::scheduler::control_block::{AnySpiritObj, SpiritControlBlock, SpiritManifestBundle};

/// Run the cross-major migration path.
///
/// 1. Verify the successor's `[migrates_from].versions` contains the predecessor version.
/// 2. Fire the `migrate()` hook on the successor's AnySpiritObj.
/// 3. Return the migrated state bytes.
pub async fn run_migrator(
    dispatcher: &crate::scheduler::hook_dispatch::HookDispatcher,
    scb: &SpiritControlBlock,
    successor_obj: &Arc<dyn AnySpiritObj>,
    predecessor_state: &[u8],
    successor_manifest: &SpiritManifestBundle,
    predecessor_version: &str,
) -> Result<Vec<u8>, HotSwapError> {
    // Derive predecessor + successor class names from the SCBs/manifests.
    // Story 5.2 review backfill: predecessor_class must come from predecessor SCB,
    // not from successor manifest (they may differ for a class-rename swap; today
    // same-class-only is enforced upstream but the error variant carries both
    // sides for diagnostic accuracy).
    let predecessor_class = scb
        .manifest
        .class
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "unknown".into());
    let successor_class = successor_manifest
        .class
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "unknown".into());
    let successor_version = successor_manifest
        .class
        .as_ref()
        .map(|c| c.version.clone())
        .unwrap_or_else(|| "unknown".into());

    // 1. Verify successor's [migrates_from].versions contains predecessor_version.
    let migrates_from =
        successor_manifest
            .migrates_from
            .as_ref()
            .ok_or_else(|| HotSwapError::EMigratorMissing {
                predecessor_class: predecessor_class.clone(),
                predecessor_version: predecessor_version.into(),
                successor_class: successor_class.clone(),
                successor_version: successor_version.clone(),
            })?;

    if !migrates_from
        .versions
        .iter()
        .any(|v| matches_version_pattern(v, predecessor_version))
    {
        return Err(HotSwapError::EMigratorMissing {
            predecessor_class: predecessor_class.clone(),
            predecessor_version: predecessor_version.into(),
            successor_class: successor_class.clone(),
            successor_version: successor_version.clone(),
        });
    }

    // 2. Fire the migrate hook on the SUCCESSOR object via a temporary SCB.
    let temp_scb = SpiritControlBlock::new(
        scb.pid,
        scb.spirit_id.clone(),
        successor_manifest.clone(),
        Arc::clone(successor_obj),
        scb.boot_nonce,
    );
    let result = dispatcher.fire_migrate(&temp_scb, predecessor_state).await;

    match result {
        Ok(migrated_bytes) => Ok(migrated_bytes),
        Err(e) => match e {
            MigratorError::NotImplemented => Err(HotSwapError::EMigratorMissing {
                predecessor_class,
                predecessor_version: predecessor_version.to_string(),
                successor_class,
                successor_version,
            }),
            other => Err(HotSwapError::MigratorFailed {
                error: other.to_string(),
            }),
        },
    }
}

/// Check if a predecessor version matches a version pattern.
///
/// Supports `"0.3.x"`-style wildcards where `x` matches any patch number.
/// Exact match `"0.3.1"` matches the exact version.
/// Ranges (e.g. `"0.2..0.3"`) are NOT supported at v0.3-β.
pub fn matches_version_pattern(pattern: &str, version: &str) -> bool {
    // Only allow 'x' wildcard in the patch (last) position.
    let parts: Vec<&str> = pattern.split('.').collect();
    let ver_parts: Vec<&str> = version.split('.').collect();
    if parts.len() != ver_parts.len() {
        return false;
    }
    for (i, part) in parts.iter().enumerate() {
        if *part == "x" {
            // Wildcard only permitted in the last position.
            if i != parts.len() - 1 {
                return false;
            }
            continue;
        }
        if *part != ver_parts[i] {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midas_version_pattern_exact_match() {
        assert!(matches_version_pattern("0.3.1", "0.3.1"));
    }

    #[test]
    fn wildcard_matches_any_patch() {
        assert!(matches_version_pattern("0.3.x", "0.3.1"));
        assert!(matches_version_pattern("0.3.x", "0.3.99"));
    }

    #[test]
    fn wildcard_major_mismatch_fails() {
        assert!(!matches_version_pattern("0.3.x", "0.4.1"));
    }

    #[test]
    fn wildcard_minor_mismatch_fails() {
        assert!(!matches_version_pattern("0.3.x", "0.2.1"));
    }

    #[test]
    fn exact_mismatch_fails() {
        assert!(!matches_version_pattern("0.3.1", "0.3.2"));
    }

    #[test]
    fn different_length_fails() {
        assert!(!matches_version_pattern("0.3.x", "0.3.1.1"));
        assert!(!matches_version_pattern("0.3", "0.3.1"));
    }
}
