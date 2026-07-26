//! Story 13.5b — legal-hold authority binding must fail closed.

#![forbid(unsafe_code)]

use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use tempfile::TempDir;

#[test]
fn independently_opened_team_shard_cannot_answer_hold_absent() {
    let dir = TempDir::new().expect("tempdir");
    let global_path = dir.path().join("global.sqlite");
    let shard_path = dir.path().join("team-a.sqlite");
    let principal = "held@example.org";

    let global =
        TransparencyLogAdapter::open_with_global_legal_holds(&global_path, &global_path, 1)
            .expect("open global TL");
    global
        .place_legal_hold(principal, "litigation", Some("case-13-5b"), 42)
        .expect("place global hold");

    let shard = TransparencyLogAdapter::open(&shard_path, 2).expect("open unbound shard TL");
    assert!(
        shard.is_under_legal_hold(principal).is_err(),
        "an unbound shard must be indeterminate/fail-closed, never false"
    );
}

#[test]
fn attached_shard_lists_and_releases_host_global_hold() {
    let dir = TempDir::new().expect("tempdir");
    let global_path = dir.path().join("global.sqlite");
    let shard_path = dir.path().join("team-a.sqlite");
    let principal = "held@example.org";
    let global =
        TransparencyLogAdapter::open_with_global_legal_holds(&global_path, &global_path, 1)
            .expect("open global TL");
    global
        .place_legal_hold(principal, "litigation", Some("case-13-5b"), 42)
        .expect("place global hold");

    let shard = TransparencyLogAdapter::open_with_global_legal_holds(&shard_path, &global_path, 2)
        .expect("open bound shard");
    assert!(shard.is_under_legal_hold(principal).unwrap());
    let holds = shard.list_legal_holds().expect("list global holds");
    assert_eq!(holds.len(), 1);
    assert_eq!(holds[0].principal_id, principal);
    assert_eq!(holds[0].case_ref.as_deref(), Some("case-13-5b"));
    assert!(shard.release_legal_hold(principal).unwrap());
    assert!(!global.is_under_legal_hold(principal).unwrap());
}
