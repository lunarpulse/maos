//! Deterministic expansion for the red-team generator.
//!
//! Expands 80 canonical scenarios across 8 §8.1 attack classes using per-class
//! parameter axes.  8 expansion variants per seed produce ≥640 items.
//! A per-seed minimum-emit pass guarantees every seed contributes at least one
//! item post-dedup.

use sha2::Sha256;
use std::collections::BTreeMap;

use super::{RedTeamItem, RedTeamSeed};

/// Expand `n` items deterministically.  The `n` parameter sets the target
/// count; the implementation guarantees at least `n` items (may overshoot
/// slightly due to the per-seed minimum-emit pass).
pub fn expand_deterministic(seeds: &[RedTeamSeed], n: usize) -> Vec<RedTeamItem> {
    if seeds.is_empty() || n == 0 {
        return vec![];
    }

    let variants_per_seed = 8;

    let mut items: Vec<RedTeamItem> = Vec::with_capacity(n);
    let mut idx = 0;

    // Cycle through seeds × variants to produce at least n items.
    let cycles = (n + seeds.len() * variants_per_seed - 1) / (seeds.len() * variants_per_seed);
    let cycles = cycles.max(1);

    for _cycle in 0..cycles {
        for seed in seeds.iter() {
            let axes = if seed.parameter_axes.is_empty() {
                vec!["variant".to_string()]
            } else {
                seed.parameter_axes.clone()
            };

            for variant_idx in 0..variants_per_seed {
                if idx >= n {
                    break;
                }
                let mut params: BTreeMap<String, String> = BTreeMap::new();
                for (axis_i, axis_name) in axes.iter().enumerate() {
                    let value = axis_value(seed, axis_name, axis_i, variant_idx);
                    params.insert(axis_name.clone(), value);
                }
                let scenario = build_scenario(seed, &params, variant_idx);
                let id = format!("red-team-{:03}", idx + 1);
                items.push(RedTeamItem {
                    id,
                    class: seed.class.clone(),
                    scenario_description: scenario,
                    parameters: params,
                    expected_kernel_response: expected_response(seed, variant_idx),
                    expected_audit_signal: expected_signal(seed, variant_idx),
                    seed_id: seed.id.clone(),
                    canonical_assertion: seed.canonical_assertion.clone(),
                });
                idx += 1;
            }
            if idx >= n {
                break;
            }
        }
        if idx >= n {
            break;
        }
    }

    // Sort by id for stability
    items.sort_by(|a, b| a.id.cmp(&b.id));

    // --- Deduplication ---
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut deduped: Vec<RedTeamItem> = Vec::with_capacity(items.len());
    for item in items {
        let key = dedup_key(&item);
        if seen.insert(key) {
            deduped.push(item);
        }
    }

    // --- Per-seed minimum-emit pass ---
    // If any seed has zero items post-dedup, add at least 1 item per seed
    // by widening variant axes.
    let mut seed_ids_present: std::collections::BTreeSet<String> =
        deduped.iter().map(|i| i.seed_id.clone()).collect();
    let mut re_idx = deduped.len();

    for seed in seeds.iter() {
        if !seed_ids_present.contains(&seed.id) {
            // All 8 variants collided — add a widened variant
            let axes = if seed.parameter_axes.is_empty() {
                vec!["variant".to_string()]
            } else {
                seed.parameter_axes.clone()
            };
            let mut params: BTreeMap<String, String> = BTreeMap::new();
            for (axis_i, axis_name) in axes.iter().enumerate() {
                let value = axis_value(seed, axis_name, axis_i, 99); // distinct variant index
                params.insert(axis_name.clone(), value);
            }
            let scenario = build_scenario(seed, &params, 99);
            let item = RedTeamItem {
                id: format!("red-team-{:04}", re_idx + 1),
                class: seed.class.clone(),
                scenario_description: scenario,
                parameters: params,
                expected_kernel_response: expected_response(seed, 99),
                expected_audit_signal: expected_signal(seed, 99),
                seed_id: seed.id.clone(),
                canonical_assertion: seed.canonical_assertion.clone(),
            };
            deduped.push(item);
            seed_ids_present.insert(seed.id.clone());
            re_idx += 1;
        }
    }

    // Re-sort
    deduped.sort_by(|a, b| a.id.cmp(&b.id));
    // Re-number ids sequentially
    for (i, item) in deduped.iter_mut().enumerate() {
        item.id = format!("red-team-{:03}", i + 1);
    }

    deduped
}

/// Deduplication key — class + canonical_assertion + scenario hash.
fn dedup_key(item: &RedTeamItem) -> String {
    use sha2::Digest;
    let mut h = Sha256::new();
    h.update(item.class.as_bytes());
    h.update(item.canonical_assertion.as_bytes());
    h.update(item.scenario_description.as_bytes());
    format!("{:x}", h.finalize())
}

/// Deterministic parameter-axis value.
fn axis_value(seed: &RedTeamSeed, axis_name: &str, axis_idx: usize, variant_idx: usize) -> String {
    use sha2::Digest;
    let mut h = Sha256::new();
    h.update(seed.id.as_bytes());
    h.update(axis_name.as_bytes());
    h.update(&axis_idx.to_le_bytes());
    h.update(&variant_idx.to_le_bytes());
    let hex = h.finalize();
    let tag = hex.iter().take(4).map(|b| format!("{:02x}", b)).collect::<String>();

    // Map to meaningful axis values per class
    match seed.class.as_str() {
        "capability_confusion" => match axis_name {
            "target_capability_class" => format!("cap-class-{}", variant_idx % 4),
            "spoofed_caller_identity" => format!("spirit-id-{}", variant_idx % 10),
            "TTL_boundary" => format!("ttl-{}s", (variant_idx % 5 + 1) * 60),
            "frame_ordering" => format!("order-{}", variant_idx % 3),
            _ => format!("{}-{}", axis_name, tag),
        },
        "iac_frame_injection" => match axis_name {
            "injection_point" => format!("iac-frame-{}", variant_idx % 5),
            "payload_type" => format!("payload-{}", variant_idx % 4),
            "frame_size" => format!("{}_bytes", (variant_idx % 4 + 1) * 512),
            "timing_context" => format!("t-{}", variant_idx % 3),
            _ => format!("{}-{}", axis_name, tag),
        },
        _ => format!("{}-v{}-{}", axis_name, variant_idx, tag),
    }
}

/// Build a scenario description from seed + parameters.
fn build_scenario(seed: &RedTeamSeed, params: &BTreeMap<String, String>, variant_idx: usize) -> String {
    let params_str = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[{} v{}] {} — Attacker varies: {}. Kernel defense: {}. Expected detection at: {}.",
        seed.class,
        variant_idx,
        seed.attack_summary,
        params_str,
        seed.kernel_defense_mechanism,
        seed.expected_detection_surface,
    )
}

/// Map seed + variant to expected kernel response.
fn expected_response(seed: &RedTeamSeed, variant_idx: usize) -> String {
    match seed.class.as_str() {
        "capability_confusion" => "ECapabilityScopeViolation".to_string(),
        "iac_frame_injection" => "EIACFrameRejected".to_string(),
        "distillation_poisoning" => "EDistillationIntegrityViolation".to_string(),
        "ledger_tampering" => "ELedgerIntegrityViolation".to_string(),
        "cross_spirit_privilege_escalation" => "ECrossSpiritPrivilegeViolation".to_string(),
        "resource_exhaustion" => "EResourceCapExceeded".to_string(),
        "side_channel_timing" => "ESideChannelAnomalyDetected".to_string(),
        "kernel_syscall_abuse" => "ESyscallPolicyViolation".to_string(),
        _ => {
            if variant_idx % 3 == 0 {
                "RejectedWithAudit".to_string()
            } else {
                "StructuralAlarmRung".to_string()
            }
        }
    }
}

/// Map seed + variant to expected audit signal.
fn expected_signal(seed: &RedTeamSeed, variant_idx: usize) -> String {
    match seed.class.as_str() {
        "capability_confusion" => "EAuditCapabilityScopeViolation".to_string(),
        "iac_frame_injection" => "EAuditFrameInjection".to_string(),
        "distillation_poisoning" => "EAuditDistillationPoisoning".to_string(),
        "ledger_tampering" => "EAuditLedgerIntegrity".to_string(),
        "cross_spirit_privilege_escalation" => "EAuditCrossSpiritPrivilege".to_string(),
        "resource_exhaustion" => "EAuditResourceCap".to_string(),
        "side_channel_timing" => "EAuditSideChannel".to_string(),
        "kernel_syscall_abuse" => "EAuditSyscallPolicy".to_string(),
        _ => format!("EAuditSecurityEvent_{}", variant_idx % 10),
    }
}
