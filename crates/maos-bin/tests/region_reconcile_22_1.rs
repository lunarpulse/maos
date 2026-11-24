//! Story 22-1 AC4 (21-3 AC6, D4a) — `MAOS_REGION_HOME` is reconciled against the signed
//! `TeamEntry.region` at cohort-daemon boot.
//!
//! The comparison is a library function reachable without TLS (like
//! `reconcile_home_team_with_manifest`); a source-structure leg holds the daemon boot to
//! calling it with the value that stamps rows.
#![cfg(feature = "network")]

use maos_bin::cross_team_crossing::reconcile_home_region_with_manifest;
use maos_cohort::{
    CohortAuthority, CohortManifest, CohortMember, ConsentMatrix, ManifestSignature, TeamEntry,
    COHORT_SCHEMA_V3, COHORT_SCHEMA_V4, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_spirit_abi::identity::SpiritId;

fn manifest_with(schema_version: u64, host_a_team: Option<&str>) -> CohortManifest {
    let team = |name: &str| TeamId::new(name).expect("canonical team");
    CohortManifest {
        schema_version,
        cohort_id: "region-22-1".to_string(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec!["00".repeat(32)],
        },
        members: vec![CohortMember {
            host_id: "host-a".to_string(),
            fingerprint: format!("sha256:{}", "ab".repeat(32)),
            roles: vec!["worker".to_string()],
            team: host_a_team.map(team),
        }],
        consent: ConsentMatrix::default(),
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams: Some(vec![TeamEntry {
            team_id: team("team-a"),
            region: Region::canonicalize("region-a").expect("canonical region"),
            datname: "maos_team_a".to_string(),
            members: vec![SpiritId::from("spirit-a")],
        }]),
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: vec![],
    }
}

#[test]
fn boot_refuses_a_region_that_disagrees_with_the_signed_manifest() {
    let manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    reconcile_home_region_with_manifest(&manifest, "host-a", Some("region-a"))
        .expect("a matching region must boot");
    let error = reconcile_home_region_with_manifest(&manifest, "host-a", Some("region-b"))
        .expect_err("AC4: a mismatching region must refuse the boot");
    assert!(
        error.contains("region-b") && error.contains("region-a") && error.contains("team-a"),
        "the refusal must name the configured and the signed region and the team; got {error}"
    );
}

#[test]
fn pinning_off_is_not_a_disagreement() {
    let manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    for off in [None, Some(""), Some("   ")] {
        reconcile_home_region_with_manifest(&manifest, "host-a", off)
            .expect("no resolved region behaves like the team axis: pass");
    }
}

#[test]
fn a_region_the_manifest_cannot_corroborate_refuses() {
    for manifest in [
        manifest_with(COHORT_SCHEMA_V4, None),
        manifest_with(COHORT_SCHEMA_V3, Some("team-a")),
    ] {
        let error = reconcile_home_region_with_manifest(&manifest, "host-a", Some("region-a"))
            .expect_err("a set region with no signed team refuses");
        assert!(error.contains("NO team"), "got {error}");
    }
    // A team the manifest carries no TeamEntry for corroborates nothing either.
    let mut manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    manifest.teams = Some(Vec::new());
    let error = reconcile_home_region_with_manifest(&manifest, "host-a", Some("region-a"))
        .expect_err("no TeamEntry refuses");
    assert!(error.contains("no team entry"), "got {error}");
}

/// The daemon boot calls the comparison with the value that stamps rows. Removing the call
/// from `reconcile_transport_identity_with_manifest` (or feeding it another reader) fails here.
#[test]
fn the_cohort_boot_reconciles_the_value_that_stamps_rows() {
    let main = include_str!("../src/main.rs");
    let start = main
        .find("fn reconcile_transport_identity_with_manifest(")
        .expect("the identity reconcile exists");
    let body = &main[start..];
    let end = body.find("\n}\n").expect("the function ends");
    let body = &body[..end];
    assert!(
        body.contains("reconcile_home_region_with_manifest(")
            && body.contains("host_config::stamped_home_region()"),
        "AC4: reconcile_transport_identity_with_manifest must call the region axis with \
         stamped_home_region()"
    );
    assert!(
        main.contains(".with_home_region(maos_bin::host_config::stamped_home_region())"),
        "the stamp and the reconcile must read the same value"
    );
}
