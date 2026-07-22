use std::sync::Arc;

use maos_domain::team::TeamId;
use maos_loom_lite::cross_team_consent::{CrossTeamConsentError, CrossTeamConsentPort};
use maos_loom_lite::replication::bundle::{
    apply_replication_bundle, build_replication_bundle_v2, BundleError, CrossTeamApplyContext,
};
use maos_loom_lite::replication::leaf::CollectiveKvLeaf;
use maos_loom_lite::store::{LoomLiteStore, StoreConfig};

const BASE_SEED: [u8; 32] = [0x42; 32];

struct FixedConsent(Result<bool, CrossTeamConsentError>);

impl CrossTeamConsentPort for FixedConsent {
    fn is_granted(
        &self,
        _from_team: &TeamId,
        _to_team: &TeamId,
        _intent: &str,
    ) -> Result<bool, CrossTeamConsentError> {
        self.0.clone()
    }
}

fn bundle_from(team: &TeamId) -> maos_loom_lite::replication::bundle::CrossRegionReplicationBundle {
    let leaf = CollectiveKvLeaf {
        source_region: "region-a".to_string(),
        source_ts: 7,
        spirit_pid: 11,
        namespace_kind: "default".to_string(),
        namespace_detail: String::new(),
        key: "shared".to_string(),
        value_kind: "text".to_string(),
        value_data: b"value".to_vec(),
        source_team: Some(team.clone()),
        distillation_depth: None,
        intent_lineage: None,
    };
    build_replication_bundle_v2(
        vec![leaf],
        &maos_domain::region::Region::canonicalize("region-a").unwrap(),
        team,
        &BASE_SEED,
    )
    .expect("leaf origin matches the envelope")
}

async fn store_for(team: &str, consent: FixedConsent) -> LoomLiteStore {
    LoomLiteStore::new(StoreConfig {
        connection_string: "host=127.0.0.1 port=1 dbname=unreachable".to_string(),
        home_region: "region-b".to_string(),
        home_team: team.to_string(),
        ..StoreConfig::default()
    })
    .await
    .unwrap()
    .with_cross_team_consent(Arc::new(consent))
}

/// 13.3 review (party-mode D1): a consented crossing whose claimed
/// `(region, team)` has no verifying key in the destination's
/// manifest-derived map must refuse at apply — the read path could never
/// serve those rows, so they must never land.
#[tokio::test]
async fn apply_refuses_crossing_without_claimed_pair_verifying_key() {
    let team_a = TeamId::new("team-a").unwrap();
    let team_b = TeamId::new("team-b").unwrap();
    let bundle = bundle_from(&team_a);

    // Consent GRANTED, destination team matches, leaves stamped — but the
    // store holds no verifying key for the claimed (region-a, team-a).
    let store = store_for("team-b", FixedConsent(Ok(true))).await;
    assert!(matches!(
        apply_replication_bundle(
            &bundle,
            &store,
            "region-b",
            Some(CrossTeamApplyContext::new(&team_b, "collective:share")),
            &BASE_SEED,
        )
        .await,
        Err(BundleError::TeamVerifyingKeyUnavailable { .. })
    ));
}

#[tokio::test]
async fn destination_and_consent_refusals_are_typed_before_store_access() {
    let team_a = TeamId::new("team-a").unwrap();
    let team_b = TeamId::new("team-b").unwrap();
    let team_c = TeamId::new("team-c").unwrap();
    let bundle = bundle_from(&team_a);

    let self_store = store_for("team-a", FixedConsent(Ok(true))).await;
    assert!(matches!(
        apply_replication_bundle(
            &bundle,
            &self_store,
            "region-b",
            Some(CrossTeamApplyContext::new(&team_a, "collective:share")),
            &BASE_SEED,
        )
        .await,
        Err(BundleError::SelfCrossing { .. })
    ));

    let mismatch_store = store_for("team-b", FixedConsent(Ok(true))).await;
    assert!(matches!(
        apply_replication_bundle(
            &bundle,
            &mismatch_store,
            "region-b",
            Some(CrossTeamApplyContext::new(&team_c, "collective:share")),
            &BASE_SEED,
        )
        .await,
        Err(BundleError::DestinationTeamMismatch { .. })
    ));

    let denied_store = store_for("team-b", FixedConsent(Ok(false))).await;
    assert!(matches!(
        apply_replication_bundle(
            &bundle,
            &denied_store,
            "region-b",
            Some(CrossTeamApplyContext::new(&team_b, "collective:share")),
            &BASE_SEED,
        )
        .await,
        Err(BundleError::ConsentDenied { .. })
    ));

    let stale_store = store_for(
        "team-b",
        FixedConsent(Err(CrossTeamConsentError::Stale {
            reason: "expired".to_string(),
        })),
    )
    .await;
    assert!(matches!(
        apply_replication_bundle(
            &bundle,
            &stale_store,
            "region-b",
            Some(CrossTeamApplyContext::new(&team_b, "collective:share")),
            &BASE_SEED,
        )
        .await,
        Err(BundleError::ConsentStateStale { .. })
    ));
}
