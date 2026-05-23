#![forbid(unsafe_code)]

//! Runtime posture state per Spirit (Story 3.2, AC3).
//!
//! `PostureState` is held inside `PolicyTableInner` so CoW updates
//! propagate atomically. The posture-hash is the domain-separated
//! SHA-256 hash that drives TOCTOU capability-token rejection on shift.

use std::collections::{HashMap, HashSet};

use sha2::{Digest, Sha256};

use maos_domain::frame::HaltPolicyOverride;
use maos_domain::invariants::i4::ApprovalDecision;
pub use maos_domain::notification::ApprovalClass;

use super::manifest::{EpistemicAction, EpistemicPolicyRule, EpistemicPolicySection, Posture};

/// (posture, class) → requires-approval (Story 3.2, AC5).
/// Const so the table is one source of truth; tests verify against it.
pub const POSTURE_APPROVAL_MATRIX: &[(Posture, ApprovalClass, bool)] = &[
    // cautious row
    (Posture::Cautious, ApprovalClass::ReadonlyScoped, false),
    (Posture::Cautious, ApprovalClass::ReadonlySearch, false),
    (Posture::Cautious, ApprovalClass::Mutating, true),
    (Posture::Cautious, ApprovalClass::ExecCapable, true),
    (Posture::Cautious, ApprovalClass::ControlPlane, true),
    (Posture::Cautious, ApprovalClass::Interactive, true),
    // assistive row
    (Posture::Assistive, ApprovalClass::ReadonlyScoped, false),
    (Posture::Assistive, ApprovalClass::ReadonlySearch, false),
    (Posture::Assistive, ApprovalClass::Mutating, true),
    (Posture::Assistive, ApprovalClass::ExecCapable, true),
    (Posture::Assistive, ApprovalClass::ControlPlane, true),
    (Posture::Assistive, ApprovalClass::Interactive, true),
    // autonomous-with-halt row
    (
        Posture::AutonomousWithHalt,
        ApprovalClass::ReadonlyScoped,
        false,
    ),
    (
        Posture::AutonomousWithHalt,
        ApprovalClass::ReadonlySearch,
        false,
    ),
    (Posture::AutonomousWithHalt, ApprovalClass::Mutating, false),
    (
        Posture::AutonomousWithHalt,
        ApprovalClass::ExecCapable,
        false,
    ),
    (
        Posture::AutonomousWithHalt,
        ApprovalClass::ControlPlane,
        true,
    ),
    (
        Posture::AutonomousWithHalt,
        ApprovalClass::Interactive,
        false,
    ),
];

/// Look up whether the given (posture, class) pair requires approval.
pub fn posture_requires_approval(posture: Posture, class: ApprovalClass) -> bool {
    POSTURE_APPROVAL_MATRIX
        .iter()
        .find(|(p, c, _)| *p == posture && *c == class)
        .map(|(_, _, requires)| *requires)
        .unwrap_or(true) // fail-closed: unknown pair requires approval
}

/// Journal a posture shift into the Approval Decision Log per I4 (Story 3.2, AC4).
pub fn journal_posture_shift(
    log: &crate::iac::transparency_log::TransparencyLogAdapter,
    actor: &str,
    spirit_id: &str,
    from: Posture,
    to: Posture,
) -> Result<(), crate::iac::transparency_log::AuditError> {
    log.insert_approval_decision(ApprovalDecision {
        actor: actor.into(),
        target: spirit_id.into(),
        capability: "posture.shift".into(),
        intent: format!("{:?} -> {:?}", from, to),
        decision: true,
        reasoning: None,
    })
}

/// Effective runtime posture for a single Spirit.
/// Held inside PolicyTableInner so CoW updates propagate atomically.
#[derive(Debug, Clone, PartialEq)]
pub struct PostureState {
    pub current: Posture,
    pub allowed_max: Posture,
    pub epistemic_policy: EpistemicPolicySection,
}

impl PostureState {
    /// Deterministic hash bound into every capability token's
    /// `posture_snapshot_hash` field. Domain-separated with a fixed prefix.
    /// SHA-256 over a canonical encoding.
    pub fn posture_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        // Domain separation prefix (16 bytes)
        hasher.update(b"maos.posture.v1\0");

        // Posture variants encoded as u8
        hasher.update(&[posture_u8(self.current)]);
        hasher.update(&[posture_u8(self.allowed_max)]);

        // Rules: sorted by tag for determinism
        let mut sorted_rules: Vec<&EpistemicPolicyRule> =
            self.epistemic_policy.rules.iter().collect();
        sorted_rules.sort_by(|a, b| a.tag.cmp(&b.tag));

        // LEB128-encode rules count
        let count = sorted_rules.len() as u64;
        leb128_encode_u64(count, &mut hasher);

        for rule in &sorted_rules {
            // tag_bytes length (LEB128) + tag
            leb128_encode_u64(rule.tag.len() as u64, &mut hasher);
            hasher.update(rule.tag.as_bytes());

            // action_u8
            hasher.update(&[epistemic_action_u8(&rule.action)]);

            // threshold_bits: f32 -> u32 bits; 0x7F80_0001 sentinel for None
            // (avoids collision with NaN bit pattern 0xFFFF_FFFF)
            let threshold_bits = rule
                .on_confidence_below
                .map(|v| v.to_bits())
                .unwrap_or(0x7F80_0001);
            hasher.update(&threshold_bits.to_le_bytes());

            // conflict_flag: 0 or 1
            let conflict_flag = if rule.on_evidence_conflict.unwrap_or(false) {
                1u8
            } else {
                0u8
            };
            hasher.update(&[conflict_flag]);
        }

        // default_action_u8
        hasher.update(&[epistemic_action_u8(&self.epistemic_policy.default_action)]);

        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }

    /// Apply a director's `HaltPolicyOverride` list — clamps tilt to
    /// [-1.0, +1.0]; rejects NaN; produces a new EpistemicPolicySection
    /// with adjusted thresholds.
    pub fn apply_director_preferences(
        policy: EpistemicPolicySection,
        overrides: &[HaltPolicyOverride],
    ) -> Result<EpistemicPolicySection, PostureError> {
        let mut new_rules: Vec<EpistemicPolicyRule> = Vec::new();
        let mut override_map: HashMap<&str, f32> = HashMap::new();
        let mut seen_tags: HashSet<&str> = HashSet::new();

        for ov in overrides {
            if ov.recall_vs_precision.is_nan() {
                return Err(PostureError::InvalidOverride(format!(
                    "NaN recall_vs_precision for tag '{}'",
                    ov.tag
                )));
            }
            if !seen_tags.insert(ov.tag.as_str()) {
                return Err(PostureError::InvalidOverride(format!(
                    "duplicate override tag '{}'",
                    ov.tag
                )));
            }
            let clamped = ov.recall_vs_precision.clamp(-1.0, 1.0);
            override_map.insert(ov.tag.as_str(), clamped);
        }

        let mut applied_tags: HashSet<&str> = HashSet::new();
        for rule in &policy.rules {
            let mut new_rule = rule.clone();
            if let Some(tilt) = override_map.get(rule.tag.as_str()) {
                let threshold = rule.on_confidence_below.ok_or_else(|| {
                    PostureError::InvalidOverride(format!(
                        "override for tag '{}' has no on_confidence_below threshold to adjust",
                        rule.tag
                    ))
                })?;
                // +tilt raises threshold (more halts; higher recall)
                // -tilt lowers threshold (fewer halts; higher precision)
                let adjusted = (threshold + 0.1 * tilt).clamp(0.0, 1.0);
                new_rule.on_confidence_below = Some(adjusted);
                applied_tags.insert(rule.tag.as_str());
            }
            new_rules.push(new_rule);
        }

        let unresolvable: Vec<&str> = override_map
            .keys()
            .filter(|t| !applied_tags.contains(*t))
            .copied()
            .collect();
        if !unresolvable.is_empty() {
            return Err(PostureError::InvalidOverride(format!(
                "override tags not found in policy rules: [{}]",
                unresolvable.join(", ")
            )));
        }

        Ok(EpistemicPolicySection {
            rules: new_rules,
            default_action: policy.default_action,
        })
    }
}

fn posture_u8(p: Posture) -> u8 {
    match p {
        Posture::Cautious => 0,
        Posture::Assistive => 1,
        Posture::AutonomousWithHalt => 2,
        Posture::Autonomous => 3,
    }
}

fn epistemic_action_u8(a: &EpistemicAction) -> u8 {
    match a {
        EpistemicAction::VerbalizeOnly => 0,
        EpistemicAction::Flag => 1,
        EpistemicAction::Halt => 2,
    }
}

/// Simple LEB128 encoding for u64 into a hasher.
fn leb128_encode_u64(mut val: u64, hasher: &mut Sha256) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        hasher.update(&[byte]);
        if val == 0 {
            break;
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PostureError {
    #[error("requested posture {requested:?} exceeds allowed_max {allowed:?}")]
    AboveCeiling {
        requested: Posture,
        allowed: Posture,
    },
    #[error(
        "posture {0:?} is not a runtime posture at v0.3 — use cautious / assistive / autonomous-with-halt"
    )]
    NonRuntimePosture(Posture),
    #[error("invalid director override: {0}")]
    InvalidOverride(String),
    #[error("unknown spirit {0}")]
    UnknownSpirit(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(current: Posture, allowed_max: Posture) -> PostureState {
        PostureState {
            current,
            allowed_max,
            epistemic_policy: EpistemicPolicySection {
                rules: vec![],
                default_action: EpistemicAction::VerbalizeOnly,
            },
        }
    }

    fn make_state_with_policy(
        current: Posture,
        allowed_max: Posture,
        policy: EpistemicPolicySection,
    ) -> PostureState {
        PostureState {
            current,
            allowed_max,
            epistemic_policy: policy,
        }
    }

    #[test]
    fn posture_hash_is_deterministic() {
        let state = make_state(Posture::Assistive, Posture::AutonomousWithHalt);
        let h1 = state.posture_hash();
        let h2 = state.posture_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn posture_hash_changes_when_posture_variant_changes() {
        let s1 = make_state(Posture::Cautious, Posture::AutonomousWithHalt);
        let s2 = make_state(Posture::Assistive, Posture::AutonomousWithHalt);
        assert_ne!(s1.posture_hash(), s2.posture_hash());
    }

    #[test]
    fn posture_hash_changes_when_threshold_changes() {
        let policy1 = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                Some(0.5),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let policy2 = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                Some(0.8),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let s1 = make_state_with_policy(Posture::Assistive, Posture::AutonomousWithHalt, policy1);
        let s2 = make_state_with_policy(Posture::Assistive, Posture::AutonomousWithHalt, policy2);
        assert_ne!(s1.posture_hash(), s2.posture_hash());
    }

    #[test]
    fn posture_hash_changes_when_default_action_changes() {
        let policy1 = EpistemicPolicySection {
            rules: vec![],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let policy2 = EpistemicPolicySection {
            rules: vec![],
            default_action: EpistemicAction::Flag,
        };
        let s1 = make_state_with_policy(Posture::Assistive, Posture::AutonomousWithHalt, policy1);
        let s2 = make_state_with_policy(Posture::Assistive, Posture::AutonomousWithHalt, policy2);
        assert_ne!(s1.posture_hash(), s2.posture_hash());
    }

    #[test]
    fn posture_hash_stable_under_rule_reordering() {
        let policy1 = EpistemicPolicySection {
            rules: vec![
                EpistemicPolicyRule::new("b".into(), EpistemicAction::Flag, None, None, None),
                EpistemicPolicyRule::new(
                    "a".into(),
                    EpistemicAction::Halt,
                    Some(0.7),
                    Some(true),
                    None,
                ),
            ],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let policy2 = EpistemicPolicySection {
            rules: vec![
                EpistemicPolicyRule::new(
                    "a".into(),
                    EpistemicAction::Halt,
                    Some(0.7),
                    Some(true),
                    None,
                ),
                EpistemicPolicyRule::new("b".into(), EpistemicAction::Flag, None, None, None),
            ],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let s1 = make_state_with_policy(Posture::Assistive, Posture::AutonomousWithHalt, policy1);
        let s2 = make_state_with_policy(Posture::Assistive, Posture::AutonomousWithHalt, policy2);
        assert_eq!(
            s1.posture_hash(),
            s2.posture_hash(),
            "hash must be stable under rule reordering"
        );
    }

    #[test]
    fn apply_director_preferences_clamps_tilt() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                Some(0.5),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        // tilt > 1.0 should be clamped to 1.0
        let overrides = vec![HaltPolicyOverride {
            tag: "x".into(),
            recall_vs_precision: 2.0,
        }];
        let result = PostureState::apply_director_preferences(policy, &overrides).unwrap();
        // 0.5 + 0.1 * 1.0 = 0.6
        assert_eq!(result.rules[0].on_confidence_below, Some(0.6));
    }

    #[test]
    fn apply_director_preferences_rejects_nan() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                Some(0.5),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let overrides = vec![HaltPolicyOverride {
            tag: "x".into(),
            recall_vs_precision: f32::NAN,
        }];
        let err = PostureState::apply_director_preferences(policy, &overrides).unwrap_err();
        assert!(matches!(err, PostureError::InvalidOverride(_)));
    }

    #[test]
    fn apply_director_preferences_idempotent() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Flag,
                Some(0.3),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let result = PostureState::apply_director_preferences(policy.clone(), &[]).unwrap();
        assert_eq!(result.rules[0].tag, policy.rules[0].tag);
        assert_eq!(
            result.rules[0].on_confidence_below,
            policy.rules[0].on_confidence_below
        );
        assert_eq!(result.default_action, policy.default_action);
    }

    #[test]
    fn apply_director_preferences_negative_tilt_lowers_threshold() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                Some(0.8),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let overrides = vec![HaltPolicyOverride {
            tag: "x".into(),
            recall_vs_precision: -0.5, // precision tilt: lower threshold
        }];
        let result = PostureState::apply_director_preferences(policy, &overrides).unwrap();
        // 0.8 - 0.1 * 0.5 = 0.75
        assert_eq!(result.rules[0].on_confidence_below, Some(0.75));
    }

    #[test]
    fn apply_director_preferences_rejects_unknown_tag() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "a".into(),
                EpistemicAction::Halt,
                Some(0.5),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let overrides = vec![HaltPolicyOverride {
            tag: "nonexistent".into(),
            recall_vs_precision: 0.5,
        }];
        let err = PostureState::apply_director_preferences(policy, &overrides).unwrap_err();
        assert!(
            matches!(err, PostureError::InvalidOverride(ref msg) if msg.contains("not found")),
            "expected unknown-tag error, got: {err:?}"
        );
    }

    #[test]
    fn apply_director_preferences_rejects_duplicate_override_tag() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                Some(0.5),
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let overrides = vec![
            HaltPolicyOverride {
                tag: "x".into(),
                recall_vs_precision: 0.5,
            },
            HaltPolicyOverride {
                tag: "x".into(),
                recall_vs_precision: -0.5,
            },
        ];
        let err = PostureState::apply_director_preferences(policy, &overrides).unwrap_err();
        assert!(
            matches!(err, PostureError::InvalidOverride(ref msg) if msg.contains("duplicate")),
            "expected duplicate-tag error, got: {err:?}"
        );
    }

    #[test]
    fn apply_director_preferences_rejects_override_for_rule_without_threshold() {
        let policy = EpistemicPolicySection {
            rules: vec![EpistemicPolicyRule::new(
                "x".into(),
                EpistemicAction::Halt,
                None,
                None,
                None,
            )],
            default_action: EpistemicAction::VerbalizeOnly,
        };
        let overrides = vec![HaltPolicyOverride {
            tag: "x".into(),
            recall_vs_precision: 0.5,
        }];
        let err = PostureState::apply_director_preferences(policy, &overrides).unwrap_err();
        assert!(
            matches!(err, PostureError::InvalidOverride(ref msg) if msg.contains("no on_confidence_below")),
            "expected no-threshold error, got: {err:?}"
        );
    }

    #[test]
    fn posture_matrix_covers_all_combinations() {
        let runtime_postures = [
            Posture::Cautious,
            Posture::Assistive,
            Posture::AutonomousWithHalt,
        ];
        let classes = [
            ApprovalClass::ReadonlyScoped,
            ApprovalClass::ReadonlySearch,
            ApprovalClass::Mutating,
            ApprovalClass::ExecCapable,
            ApprovalClass::ControlPlane,
            ApprovalClass::Interactive,
        ];

        assert_eq!(POSTURE_APPROVAL_MATRIX.len(), 18);

        for posture in &runtime_postures {
            for class in &classes {
                let count = POSTURE_APPROVAL_MATRIX
                    .iter()
                    .filter(|(p, c, _)| p == posture && c == class)
                    .count();
                assert_eq!(
                    count, 1,
                    "(posture={:?}, class={:?}) must appear once",
                    posture, class
                );
            }
        }
    }

    #[test]
    fn posture_matrix_matches_epic_3_acs() {
        let expected: &[(Posture, ApprovalClass, bool)] = &[
            (Posture::Cautious, ApprovalClass::ReadonlyScoped, false),
            (Posture::Cautious, ApprovalClass::ReadonlySearch, false),
            (Posture::Cautious, ApprovalClass::Mutating, true),
            (Posture::Cautious, ApprovalClass::ExecCapable, true),
            (Posture::Cautious, ApprovalClass::ControlPlane, true),
            (Posture::Cautious, ApprovalClass::Interactive, true),
            (Posture::Assistive, ApprovalClass::ReadonlyScoped, false),
            (Posture::Assistive, ApprovalClass::ReadonlySearch, false),
            (Posture::Assistive, ApprovalClass::Mutating, true),
            (Posture::Assistive, ApprovalClass::ExecCapable, true),
            (Posture::Assistive, ApprovalClass::ControlPlane, true),
            (Posture::Assistive, ApprovalClass::Interactive, true),
            (
                Posture::AutonomousWithHalt,
                ApprovalClass::ReadonlyScoped,
                false,
            ),
            (
                Posture::AutonomousWithHalt,
                ApprovalClass::ReadonlySearch,
                false,
            ),
            (Posture::AutonomousWithHalt, ApprovalClass::Mutating, false),
            (
                Posture::AutonomousWithHalt,
                ApprovalClass::ExecCapable,
                false,
            ),
            (
                Posture::AutonomousWithHalt,
                ApprovalClass::ControlPlane,
                true,
            ),
            (
                Posture::AutonomousWithHalt,
                ApprovalClass::Interactive,
                false,
            ),
        ];
        assert_eq!(POSTURE_APPROVAL_MATRIX, expected);
    }

    #[test]
    fn posture_requires_approval_returns_true_for_unknown_pairs() {
        assert!(posture_requires_approval(
            Posture::Autonomous,
            ApprovalClass::ReadonlyScoped
        ));
    }
}
