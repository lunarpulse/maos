//! Story 11.4a (F2, FLAG-Winston) — integration tests for the bounded
//! `OperatorPolicyConfig.per_capability_deny` restriction layer + the
//! deny-wins arm in `PolicyTable::evaluate`.
//!
//! These tests live in `tests/` (NOT counted by the kernel-core src-line
//! baseline) so the re-pin stays confined to the named functional surface
//! (the field + the arm). They exercise the kernel-core behavior directly;
//! the live-Cedar end-to-end deny through `issue_with_mediation` is covered
//! by the `maos-pdp` adapter tests (AC2/AC3/AC4).
//!
//! F2 rulings exercised here:
//! - Cedar `forbid`-beats-`permit`: an org deny returns `Deny` BEFORE any
//!   grant check, even when the Spirit's manifest grants the capability.
//! - The kernel keeps the ceiling (I1): `per_capability_deny` can only
//!   subtract (deny); it can never grant a capability beyond the manifest.
//! - Empty `per_capability_deny` ⇒ the arm is a no-op (AC1 byte-identical).

use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;

use maos_kernel_core::capability::cap_policy::decision::{
    Capability, Intent, PolicyDecision, TrustTier,
};
use maos_kernel_core::capability::cap_policy::{
    ManifestCapabilityScope, OperatorPolicyConfig, PolicyTable, PolicyTableInner,
};

/// Build a table where spirit `pid` has `FsRead { /tmp }` in its manifest,
/// verified trust, T0 floors — i.e. the capability would otherwise `Allow`.
fn table_with_manifest_grant(pid: u32) -> PolicyTable {
    let table = PolicyTable::new();
    let mut inner = PolicyTableInner::default();
    inner.manifest_scopes.insert(
        pid,
        ManifestCapabilityScope {
            scopes: vec![Scope::FsRead {
                subtree: "/tmp".into(),
            }],
            declared_tier: SandboxTier(0),
            trust_tier: TrustTier::Verified,
        },
    );
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier(0));
    table.update(inner);
    table
}

fn fsread_cap() -> Capability {
    Capability {
        scope: Scope::FsRead {
            subtree: "/tmp".into(),
        },
    }
}

fn fsread_intent() -> Intent {
    Intent::FsRead {
        subtree: "/tmp".into(),
    }
}

#[test]
fn baseline_manifest_grants_fsread() {
    // Sanity: without any deny, the manifest grants FsRead → Allow.
    let table = table_with_manifest_grant(42);
    let decision = table.evaluate(42, &fsread_cap(), &fsread_intent());
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn per_capability_deny_beats_manifest_grant_via_scope_key() {
    // F2: an org forbid on the FsRead stable action key returns Deny EVEN
    // THOUGH the manifest grants FsRead. Cedar forbid-beats-permit.
    let table = table_with_manifest_grant(42);
    let forbid_key = maos_domain::ports::scope_action_key(&Scope::FsRead {
        subtree: "/tmp".into(),
    })
    .to_string();
    {
        let mut inner = (*table.inner().load_full()).clone();
        inner.operator_policy.per_capability_deny.insert(forbid_key);
        table.update(inner);
    }
    let decision = table.evaluate(42, &fsread_cap(), &fsread_intent());
    assert_eq!(
        decision,
        PolicyDecision::Deny,
        "org forbid (scope key) must beat manifest grant"
    );
}

#[test]
fn per_capability_deny_beats_manifest_grant_via_intent_key() {
    // F2: forbid may also be keyed by stable intent key (same vocabulary as
    // per_capability_approval). The deny-wins arm checks BOTH.
    let table = table_with_manifest_grant(42);
    let forbid_key = "fs.read".to_string();
    {
        let mut inner = (*table.inner().load_full()).clone();
        inner.operator_policy.per_capability_deny.insert(forbid_key);
        table.update(inner);
    }
    let decision = table.evaluate(42, &fsread_cap(), &fsread_intent());
    assert_eq!(
        decision,
        PolicyDecision::Deny,
        "org forbid (intent key) must beat manifest grant"
    );
}

#[test]
fn per_capability_deny_absent_is_noop_allow() {
    // AC1 byte-identical: empty per_capability_deny ⇒ the arm is a no-op;
    // the manifest-grant Allow path is unchanged.
    let table = table_with_manifest_grant(7);
    // OperatorPolicyConfig::default() has an empty per_capability_deny.
    let _ = OperatorPolicyConfig::default();
    let decision = table.evaluate(7, &fsread_cap(), &fsread_intent());
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn per_capability_deny_cannot_grant_beyond_manifest() {
    // I1 — the kernel keeps the ceiling. per_capability_deny can ONLY
    // subtract. A capability NOT in the manifest is denied by scope-match
    // regardless of the deny set; populating an unrelated deny key does
    // not relax the manifest ceiling.
    let table = table_with_manifest_grant(42);
    {
        let mut inner = (*table.inner().load_full()).clone();
        // Forbid a DIFFERENT capability (NetHttps) — must not affect FsRead,
        // and must not grant FsWrite (which is absent from the manifest).
        let net_key = maos_domain::ports::scope_action_key(&Scope::NetHttps {
            domain: "example.com".into(),
        })
        .to_string();
        inner.operator_policy.per_capability_deny.insert(net_key);
        table.update(inner);
    }
    // FsWrite is NOT in spirit 42's manifest → scope-match denies it. The
    // deny set (which only contains NetHttps) neither grants nor relaxes.
    let fswrite = Capability {
        scope: Scope::FsWrite {
            subtree: "/etc".into(),
        },
    };
    let fswrite_intent = Intent::FsWrite {
        subtree: "/etc".into(),
    };
    let decision = table.evaluate(42, &fswrite, &fswrite_intent);
    assert_eq!(
        decision,
        PolicyDecision::Deny,
        "ceiling preserved: manifest-absent capability stays denied"
    );
    // And FsRead (in manifest, not denied) still Allows.
    let decision = table.evaluate(42, &fsread_cap(), &fsread_intent());
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn per_spirit_capability_deny_only_targets_one_spirit() {
    let target = table_with_manifest_grant(42);
    {
        let mut inner = (*target.inner().load_full()).clone();
        inner
            .manifest_scopes
            .insert(7, inner.manifest_scopes[&42].clone());
        inner
            .operator_policy
            .per_spirit_capability_deny
            .entry(42)
            .or_default()
            .insert("fs.read".to_string());
        target.update(inner);
    }
    assert_eq!(
        target.evaluate(42, &fsread_cap(), &fsread_intent()),
        PolicyDecision::Deny,
        "subject-scoped deny must block the targeted spirit"
    );
    assert_eq!(
        target.evaluate(7, &fsread_cap(), &fsread_intent()),
        PolicyDecision::Allow,
        "subject-scoped deny must not become an org-wide deny"
    );
}
#[test]
fn per_capability_deny_does_not_override_self_telemetry() {
    // FR56 invariant: SelfTelemetryRead is a kernel always-allow evaluated
    // BEFORE the inner load / deny-wins arm. An operator forbid on the
    // SelfTelemetryRead stable action key does NOT override it (the Spirit
    // reads its own telemetry — a fundamental kernel invariant, not an
    // operator-admission gate).
    let table = table_with_manifest_grant(42);
    {
        let mut inner = (*table.inner().load_full()).clone();
        let tel_key = maos_domain::ports::scope_action_key(&Scope::SelfTelemetryRead).to_string();
        inner.operator_policy.per_capability_deny.insert(tel_key);
        table.update(inner);
    }
    let cap = Capability {
        scope: Scope::SelfTelemetryRead,
    };
    let decision = table.evaluate(42, &cap, &Intent::SelfTelemetryRead);
    assert_eq!(
        decision,
        PolicyDecision::Allow,
        "FR56 self-telemetry always-allow is a kernel invariant, not PDP-overridable"
    );
}
