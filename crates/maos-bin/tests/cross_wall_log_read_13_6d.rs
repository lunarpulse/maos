use std::sync::{Arc, LazyLock, Mutex};

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{LogRecallFilter, LogRecallPage};
use maos_domain::ports::{
    CrossWallLogReadPort, CrossWallRecallConsentDecision, CrossWallRecallConsentError,
    CrossWallRecallConsentPort, LogRecallPort,
};
use maos_domain::team::TeamId;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};

use maos_bin::cross_wall_log_read::CrossWallLogReadAdapter;

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
    LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

struct RestoreMaosHome(Option<std::ffi::OsString>);

impl Drop for RestoreMaosHome {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("MAOS_HOME", value),
            None => std::env::remove_var("MAOS_HOME"),
        }
    }
}

fn seed_remote_artifact(home: &std::path::Path, path_team: &TeamId, bound_team: &TeamId) {
    std::env::set_var("MAOS_HOME", home);
    let path =
        maos_audit::transparency_log_path_for_tenant_mode(true, Some(path_team.as_str())).unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let log = TransparencyLogAdapter::open(&path, 0x136D).unwrap();
    for marker in ["remote-a", "remote-b"] {
        log.insert_frame_event(
            FrameKind::TaskAssign,
            42,
            None,
            marker,
            marker.as_bytes(),
            FrameOrigin::HumanAuthored,
        );
    }
    drop(log);
    maos_audit::write_tenant_binding(&path, bound_team, Some("maos_team_b")).unwrap();
}

struct GrantedConsent;

impl CrossWallRecallConsentPort for GrantedConsent {
    fn decide(
        &self,
        _remote_team: &TeamId,
        _intent: &str,
    ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError> {
        Ok(CrossWallRecallConsentDecision::Granted)
    }
}

#[test]
fn consented_cross_wall_page_contains_remote_frames_and_no_local_frames() {
    let _lock = env_lock();
    let restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    let remote = TeamId::new("team-b").unwrap();
    seed_remote_artifact(home.path(), &remote, &remote);

    let local = Arc::new(TransparencyLogAdapter::open_in_memory(0x136E));
    for marker in ["local-a", "local-b"] {
        local.insert_frame_event(
            FrameKind::TaskAssign,
            42,
            None,
            marker,
            marker.as_bytes(),
            FrameOrigin::HumanAuthored,
        );
    }
    let local_ids: std::collections::HashSet<_> = local
        .query_frames(FrameFilter {
            spirit_pid: Some(42),
            kind: Some(FrameKind::TaskAssign),
            ..Default::default()
        })
        .unwrap()
        .into_iter()
        .map(|entry| entry.frame_id)
        .collect();
    let adapter = LogRecallAdapter::new(local)
        .with_cross_wall_consent(Arc::new(GrantedConsent))
        .with_cross_wall_read(Arc::new(CrossWallLogReadAdapter::new(true)));

    let page = adapter
        .recall_cross_wall(42, &remote, LogRecallFilter::default())
        .unwrap();
    let returned_ids: std::collections::HashSet<_> =
        page.entries.iter().map(|entry| entry.frame_id).collect();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.intent.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["remote-a", "remote-b"])
    );
    assert!(returned_ids.is_disjoint(&local_ids));
    drop(restore);
}

#[test]
fn ordinary_boot_path_still_refuses_a_foreign_bound_artifact() {
    let _lock = env_lock();
    let restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    let local = TeamId::new("team-a").unwrap();
    let remote = TeamId::new("team-b").unwrap();
    seed_remote_artifact(home.path(), &remote, &remote);
    let path =
        maos_audit::transparency_log_path_for_tenant_mode(true, Some(remote.as_str())).unwrap();

    assert!(matches!(
        maos_bin::tenant_map::phase_a_preflight(&path, &local).unwrap(),
        maos_audit::TenantBindingPhaseADecision::Refuse(
            maos_audit::TenantBindingPhaseARefusal::BoundToForeignTeam { bound, env }
        ) if bound == "team-b" && env == local
    ));
    drop(restore);
}

#[test]
fn cross_wall_reader_returns_rows_from_the_named_bound_artifact() {
    let _lock = env_lock();
    let restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    let remote = TeamId::new("team-b").unwrap();
    seed_remote_artifact(home.path(), &remote, &remote);

    let page: LogRecallPage = CrossWallLogReadAdapter::new(true)
        .read_remote(42, &remote, LogRecallFilter::default())
        .unwrap();

    assert_eq!(page.entries.len(), 2);
    assert!(page.entries.iter().all(|entry| entry.peer_spirit_pid == 42));
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.intent.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["remote-a", "remote-b"])
    );
    drop(restore);
}

#[test]
fn cross_wall_reader_refuses_a_path_whose_artifact_binding_names_another_team() {
    let _lock = env_lock();
    let restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    let requested = TeamId::new("team-b").unwrap();
    let bound = TeamId::new("team-a").unwrap();
    seed_remote_artifact(home.path(), &requested, &bound);

    let error = CrossWallLogReadAdapter::new(true)
        .read_remote(42, &requested, LogRecallFilter::default())
        .unwrap_err();

    assert!(error.to_string().contains("bound to team-a"));
    assert!(error.to_string().contains("requested team-b"));
    drop(restore);
}

#[test]
fn cross_wall_reader_open_is_read_only_nofollow_and_non_migrating() {
    let source = include_str!("../src/../../maos-iac/src/adapter/transparency_log.rs");
    let method = source
        .split("pub fn open_read_only")
        .nth(1)
        .and_then(|tail| tail.split("pub fn open_with_global_legal_holds").next())
        .expect("open_read_only method source");
    assert!(method.contains("SQLITE_OPEN_READ_ONLY"));
    assert!(method.contains("SQLITE_OPEN_NOFOLLOW"));
    assert!(!method.contains("execute_batch"));
    assert!(!method.contains("SCHEMA_SQL"));
}

#[test]
fn single_connection_foreign_read_is_immune_to_an_artifact_swap_between_binding_and_query() {
    // P2 regression (Story 13.6d): `read_remote` verifies the binding AND serves
    // the rows on ONE read-only NOFOLLOW connection. A regression to two separate
    // opens re-resolves the path for the second open, so a file swap between them
    // would attest one artifact while disclosing another. This replicates that
    // exact sequence with the swap injected, proving the held connection serves
    // the *verified* artifact's rows.
    let dir = tempfile::tempdir().unwrap();
    let good_path = dir.path().join("good.db");
    let swap_path = dir.path().join("swap.db");
    let team_a = TeamId::new("team-a").unwrap();

    // good.db: bound to team-a, frames {good-1}.
    let good = TransparencyLogAdapter::open(&good_path, 0x136D).unwrap();
    good.insert_frame_event(
        FrameKind::TaskAssign,
        42,
        None,
        "good-1",
        b"good-1",
        FrameOrigin::HumanAuthored,
    );
    drop(good);
    maos_audit::write_tenant_binding(&good_path, &team_a, Some("maos_team_a")).unwrap();

    // swap.db: ALSO bound to team-a (so a naive second-open binding check would
    // still pass) but carrying DIFFERENT frames {swap-1}.
    let swap = TransparencyLogAdapter::open(&swap_path, 0x136D).unwrap();
    swap.insert_frame_event(
        FrameKind::TaskAssign,
        42,
        None,
        "swap-1",
        b"swap-1",
        FrameOrigin::HumanAuthored,
    );
    drop(swap);
    maos_audit::write_tenant_binding(&swap_path, &team_a, Some("maos_team_a")).unwrap();

    // The fixed sequence: ONE connection, held through the swap.
    let conn = maos_audit::open_tenant_artifact_readonly(&good_path)
        .expect("open good.db")
        .expect("good.db exists");
    let artifact = maos_audit::read_tenant_artifact_on(&conn).expect("read binding on held conn");
    assert_eq!(artifact.binding_team.as_deref(), Some("team-a"));

    // TOCTOU trigger: replace the path with swap.db AFTER the binding was verified.
    std::fs::rename(&swap_path, &good_path).unwrap();

    // Query on the SAME held connection (what `read_remote` does after P2).
    let reader = TransparencyLogAdapter::from_read_only_connection(conn);
    let page: LogRecallPage = LogRecallAdapter::query_page(&reader, 42, LogRecallFilter::default())
        .expect("query on held connection");

    // Served rows are good.db's {good-1}, NOT swap.db's {swap-1}: the held
    // connection still points at good.db's inode. Two opens would return swap-1.
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].intent.as_str(), "good-1");
}
