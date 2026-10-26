#![cfg(feature = "network")]
#![forbid(unsafe_code)]

use std::sync::Arc;

use maos_bin::admission::{gate_manifest, GatedManifest};
use maos_domain::invariants::i1::Scope;
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::cap_policy::{
    decision::TrustTier, PolicyTable, PolicyTableInner,
};
use maos_kernel_core::capability::cap_tokens;
use maos_kernel_core::journal::JournalAdapter;
use maos_kernel_core::security::SecurityManagerAdapter;
use maos_manifest::{capabilities_required_to_scopes, LogCapabilities};

const MANIFEST: &str = include_str!("../../../spirits/researcher/manifest.toml");

fn admit(gated: &GatedManifest) -> Vec<Scope> {
    cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    inner
        .trust_tier_floor
        .insert(TrustTier::Verified, SandboxTier::T0);
    policy.update(inner);
    let security = SecurityManagerAdapter::new(Arc::clone(&policy));
    let scratch = tempfile::TempDir::new().expect("admission scratch directory");
    let journal = JournalAdapter::open(&scratch.path().join("journal.ndjson"))
        .expect("real admission journal");
    let sandbox = security
        .admit_spirit(
            173,
            &gated.spirit_id,
            &gated.sandbox_cfg,
            &gated.resource_caps,
            &gated.caps_required,
            Some(&gated.output_shape),
            &journal,
            &gated.posture_section,
            gated.epistemic_policy.as_ref(),
            Some(&gated.scheduling),
            Some(&gated.lifecycle),
            None,
            None,
            None,
            Some(&gated.class_section),
        )
        .expect("checked-in researcher passes kernel admission");
    let snapshot = policy.inner().load_full();
    assert_eq!(
        snapshot.manifest_scopes[&173].scopes,
        sandbox.declared_scopes
    );
    sandbox.declared_scopes
}

#[test]
fn checked_in_researcher_admission_preserves_both_schema_five_log_scopes() {
    let gated = gate_manifest(MANIFEST, &|class| class == "researcher")
        .expect("checked-in researcher passes production structural admission");
    assert_eq!(gated.class_section.manifest_schema_version, 6);
    assert_eq!(
        gated.caps_required.log,
        LogCapabilities {
            recall: true,
            fetch: true
        }
    );
    let degraded = gated.caps_required.clone().degrade_for_schema_version(6);
    assert_eq!(
        degraded, gated.caps_required,
        "schema-six degradation is lossless"
    );
    let scopes = admit(&gated);
    assert_eq!(scopes, capabilities_required_to_scopes(&degraded));
    assert!(scopes.contains(&Scope::LogRecall));
    assert!(scopes.contains(&Scope::LogFetch));
    assert!(scopes.contains(&Scope::SkillAuthorSelf));
    assert!(scopes.contains(&Scope::ProviderInfer {
        provider: "anthropic".into()
    }));
    // 17-11 review (operator ruling 2026-10-24): the Researcher also declares an
    // `openai.<model>` provider so an OpenAI-only router default is issuable.
    assert!(scopes.contains(&Scope::ProviderInfer {
        provider: "openai".into()
    }));
    for (server, tools) in [
        ("web", ["search", "fetch"]),
        ("arxiv", ["search", "get_paper"]),
        ("github", ["search_code", "get_repo"]),
        ("citation-graph", ["traverse", "get_citations"]),
    ] {
        for tool in tools {
            assert!(scopes.contains(&Scope::McpCall {
                server: server.into(),
                tool: tool.into()
            }));
        }
    }
    assert!(scopes.contains(&Scope::LoomRead));
    assert!(scopes.contains(&Scope::LoomWrite));
    assert!(!scopes.contains(&Scope::LoomScan));
    assert!(!scopes.contains(&Scope::DistillateWrite));
    assert!(!scopes
        .iter()
        .any(|scope| matches!(scope, Scope::IacSend { .. })));
    assert_eq!(
        scopes.len(),
        15,
        "existing grants plus recall/fetch, the 17-11 openai provider and 18-7 self-authoring"
    );
}

#[test]
fn researcher_schema_four_degradation_drops_log_but_preserves_existing_capabilities() {
    let current = gate_manifest(MANIFEST, &|class| class == "researcher").unwrap();
    let legacy_text =
        MANIFEST.replace("manifest_schema_version = 6", "manifest_schema_version = 4");
    assert_ne!(legacy_text, MANIFEST, "exercise the actual previous schema");
    let legacy = gate_manifest(&legacy_text, &|class| class == "researcher")
        .expect("previous schema remains structurally admissible");
    assert_eq!(legacy.caps_required.log, LogCapabilities::default());
    assert_eq!(
        legacy.caps_required.provider,
        current.caps_required.provider
    );
    assert_eq!(legacy.caps_required.mcp, current.caps_required.mcp);
    assert_eq!(legacy.caps_required.loom, current.caps_required.loom);
    let scopes = admit(&legacy);
    assert!(!scopes.contains(&Scope::LogRecall));
    assert!(!scopes.contains(&Scope::LogFetch));
    let existing: Vec<_> = capabilities_required_to_scopes(&current.caps_required)
        .into_iter()
        .filter(|scope| {
            !matches!(
                scope,
                Scope::LogRecall | Scope::LogFetch | Scope::SkillAuthorSelf
            )
        })
        .collect();
    assert_eq!(scopes, existing);
}
