#![cfg(feature = "network")]
#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use ed25519_dalek::SigningKey;
use maos_bin::tenant_map::{tenant_map_for_store, TenantMapAdapter, TenantMapBootError};
use maos_cohort::{
    CohortAuditEvent, CohortClock, CohortError, CohortManifest, CohortManifestForkReason,
    CohortManifestState, InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys,
    TeamEntry, COHORT_SCHEMA_V1, COHORT_SCHEMA_V2, RESERVED_INTENT_HALT_RECEIPT,
    RESERVED_INTENT_REISSUE,
};
use maos_domain::ports::registry::SpiritId;
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_loom_lite::tenant::{TenantMapError, TenantMapPort};
use maos_spirit_abi::identity::HostId;

#[derive(Default)]
struct TestClock(AtomicU64);

impl TestClock {
    fn set(&self, now_secs: u64) {
        self.0.store(now_secs, Ordering::SeqCst);
    }
}

impl CohortClock for TestClock {
    fn now_secs(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

struct Fixture {
    state: Arc<CohortManifestState>,
    clock: Arc<TestClock>,
    signing_key: SigningKey,
    audit: Arc<InMemoryCohortAuditSink>,
}

fn signed_manifest(
    signing_key: &SigningKey,
    schema_version: u64,
    version: u64,
    hosts: &[&str],
) -> String {
    let authority_key = hex::encode(signing_key.verifying_key().to_bytes());
    let members = hosts
        .iter()
        .map(|host| maos_cohort::CohortMember {
            host_id: (*host).to_string(),
            fingerprint: format!("sha256:{}", "11".repeat(32)),
            roles: vec!["worker".to_string()],
        })
        .collect();
    let teams = (schema_version == COHORT_SCHEMA_V2).then(|| {
        vec![
            TeamEntry {
                team_id: TeamId::new("team-a").unwrap(),
                region: Region::canonicalize("region-a").unwrap(),
                datname: "maos_team_a".to_string(),
                members: vec![SpiritId::from("spirit-a")],
            },
            TeamEntry {
                team_id: TeamId::new("team-b").unwrap(),
                region: Region::canonicalize("region-b").unwrap(),
                datname: "maos_team_b".to_string(),
                members: vec![SpiritId::from("spirit-b")],
            },
        ]
    });
    let manifest = CohortManifest {
        schema_version,
        cohort_id: "cohort-tenant".to_string(),
        version,
        authority: maos_cohort::CohortAuthority {
            threshold: 1,
            keys: vec![authority_key],
        },
        members,
        consent: maos_cohort::ConsentMatrix::default(),
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams,
        signature: ManifestSignature { sig: String::new() },
    }
    .signed_with(signing_key);
    toml::to_string(&manifest).unwrap()
}

fn fixture() -> Fixture {
    let signing_key = SigningKey::from_bytes(&[13; 32]);
    let pinned = PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()]).unwrap();
    let clock = Arc::new(TestClock::default());
    clock.set(100);
    let audit = Arc::new(InMemoryCohortAuditSink::default());
    let state = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host-a".to_string()),
            &signed_manifest(&signing_key, COHORT_SCHEMA_V2, 1, &["host-a", "host-b"]),
            pinned,
            audit.clone(),
            clock.clone(),
        )
        .unwrap(),
    );
    Fixture {
        state,
        clock,
        signing_key,
        audit,
    }
}

#[test]
fn same_team_binding_admits_and_resolves_datname() {
    let fixture = fixture();
    let adapter = TenantMapAdapter::new(fixture.state, "host-a", true).unwrap();
    adapter.register_spirit(7, SpiritId::from("spirit-a"));

    let team = adapter.team_of(7).unwrap();
    assert_eq!(team.as_str(), "team-a");
    assert_eq!(adapter.datname_for(&team).unwrap(), "maos_team_a");
}

#[test]
fn unregistered_spirit_is_typed_refusal() {
    let fixture = fixture();
    let adapter = TenantMapAdapter::new(fixture.state, "host-a", true).unwrap();

    assert!(matches!(
        adapter.team_of(999),
        Err(TenantMapError::SpiritUnmapped { spirit_pid: 999 })
    ));
}

#[test]
fn expired_lease_is_tenant_map_stale() {
    let fixture = fixture();
    let adapter = TenantMapAdapter::new(fixture.state, "host-a", true).unwrap();
    adapter.register_spirit(7, SpiritId::from("spirit-a"));
    fixture.clock.set(221);

    assert!(matches!(
        adapter.team_of(7),
        Err(TenantMapError::Stale { .. })
    ));
}

#[test]
fn evicted_local_host_stops_serving() {
    let fixture = fixture();
    let adapter = TenantMapAdapter::new(fixture.state.clone(), "host-a", true).unwrap();
    adapter.register_spirit(7, SpiritId::from("spirit-a"));
    fixture
        .state
        .apply_reissue(&signed_manifest(
            &fixture.signing_key,
            COHORT_SCHEMA_V2,
            2,
            &["host-b", "host-c"],
        ))
        .unwrap();

    assert!(matches!(
        adapter.team_of(7),
        Err(TenantMapError::Stale { .. })
    ));
}

#[test]
fn tenant_boot_refuses_absent_or_unrefreshable_source() {
    assert!(matches!(
        tenant_map_for_store("team-a", None),
        Err(TenantMapBootError::SourceUnavailable)
    ));

    let fixture = fixture();
    assert!(matches!(
        TenantMapAdapter::new(fixture.state, "host-a", false),
        Err(TenantMapBootError::SourceUnrefreshable)
    ));
}

#[test]
fn tenant_boot_refuses_peerless_cohort() {
    let signing_key = SigningKey::from_bytes(&[17; 32]);
    let pinned = PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()]).unwrap();
    let state = Arc::new(
        CohortManifestState::load(
            HostId("host-a".to_string()),
            &signed_manifest(&signing_key, COHORT_SCHEMA_V2, 1, &["host-a"]),
            pinned,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap(),
    );

    assert!(matches!(
        TenantMapAdapter::new(state, "host-a", true),
        Err(TenantMapBootError::SourceUnrefreshable)
    ));
}

#[test]
fn tenant_boot_refuses_verified_v1_manifest() {
    let signing_key = SigningKey::from_bytes(&[19; 32]);
    let pinned = PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()]).unwrap();
    let state = Arc::new(
        CohortManifestState::load(
            HostId("host-a".to_string()),
            &signed_manifest(&signing_key, COHORT_SCHEMA_V1, 1, &["host-a", "host-b"]),
            pinned,
            Arc::new(InMemoryCohortAuditSink::default()),
        )
        .unwrap(),
    );

    assert!(matches!(
        TenantMapAdapter::new(state, "host-a", true),
        Err(TenantMapBootError::SchemaV2FloorRequired)
    ));
}

#[test]
fn non_tenant_boot_quietly_disables_map() {
    assert!(tenant_map_for_store("", None).unwrap().is_none());
}

#[test]
fn v2_adoption_is_locally_audited() {
    let fixture = fixture();
    fixture
        .state
        .apply_reissue(&signed_manifest(
            &fixture.signing_key,
            COHORT_SCHEMA_V2,
            2,
            &["host-a", "host-b"],
        ))
        .unwrap();
    assert!(fixture.audit.events().iter().any(|event| matches!(
        event,
        CohortAuditEvent::MemberReissueAccepted { version: 2, .. }
    )));
}

#[test]
fn runtime_v2_to_v1_downgrade_is_rejected_and_audited() {
    let fixture = fixture();
    let error = fixture
        .state
        .apply_reissue(&signed_manifest(
            &fixture.signing_key,
            COHORT_SCHEMA_V1,
            2,
            &["host-a", "host-b"],
        ))
        .unwrap_err();
    assert!(matches!(
        error,
        CohortError::ECohortManifestFork {
            reason: CohortManifestForkReason::SchemaDowngrade,
            ..
        }
    ));
    assert!(fixture.audit.events().iter().any(|event| matches!(
        event,
        CohortAuditEvent::ReissueRejected { reason, .. }
            if reason.contains("schema_downgrade")
    )));
}

#[test]
fn tenant_map_rejects_local_host_spoof() {
    let fixture = fixture();
    assert!(matches!(
        TenantMapAdapter::new(fixture.state, "host-b", true),
        Err(TenantMapBootError::LocalHostMismatch { .. })
    ));
}
#[test]
fn tenant_map_13_1_gate_matrix() {
    same_team_binding_admits_and_resolves_datname();
    unregistered_spirit_is_typed_refusal();
    expired_lease_is_tenant_map_stale();
    evicted_local_host_stops_serving();
    tenant_boot_refuses_absent_or_unrefreshable_source();
    tenant_boot_refuses_peerless_cohort();
    tenant_boot_refuses_verified_v1_manifest();
    non_tenant_boot_quietly_disables_map();
    v2_adoption_is_locally_audited();
    runtime_v2_to_v1_downgrade_is_rejected_and_audited();
    tenant_map_rejects_local_host_spoof();
}
