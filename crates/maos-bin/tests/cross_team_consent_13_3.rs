#![cfg(feature = "network")]

use std::sync::{Arc, Mutex};

use ed25519_dalek::SigningKey;
use maos_bin::cross_team_consent::{
    derive_team_verifying_keys, CrossTeamConsentAdapter, CrossWallRecallConsentAdapter,
};
use maos_bin::cross_wall_log_read::CrossWallLogReadAdapter;
use maos_cohort::{
    CohortAuthority, CohortClock, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    CrossTeamConsentGrant, InMemoryCohortAuditSink, ManifestSignature, PinnedAuthorityKeys,
    TeamEntry, COHORT_SCHEMA_V3, RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::log_recall::{LogRecallError, LogRecallFilter, LogRecallPage};
use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::ports::{
    CrossWallLogReadPort, CrossWallRecallConsentDecision, CrossWallRecallConsentError,
    CrossWallRecallConsentPort, LogRecallPort,
};
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_iac::adapter::log_recall::LogRecallAdapter;
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_loom_lite::cross_team_consent::{CrossTeamConsentError, CrossTeamConsentPort};
use maos_loom_lite::replication::bundle::{
    apply_replication_bundle, build_replication_bundle_v2, BundleError, CrossTeamApplyContext,
};
use maos_loom_lite::replication::leaf::CollectiveKvLeaf;
use maos_loom_lite::store::{LoomLiteStore, StoreConfig};
use maos_loom_lite::tenant::{TenantMapError, TenantMapPort};
use maos_spirit_abi::identity::{HostId, SpiritId};

// Story 13.6e (AC3) — the harness signs its own transcript record; the gate
// only verifies. Shared signer, no new crate and no new dependency.
#[path = "../../../tests/harness/evidence_record.rs"]
mod evidence_record;

struct RestoreMaosHome(Option<std::ffi::OsString>);

impl Drop for RestoreMaosHome {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("MAOS_HOME", value),
            None => std::env::remove_var("MAOS_HOME"),
        }
    }
}

#[derive(Default)]
struct TestClock(Mutex<u64>);

impl TestClock {
    fn advance(&self, seconds: u64) {
        *self.0.lock().unwrap() += seconds;
    }
}

impl CohortClock for TestClock {
    fn now_secs(&self) -> u64 {
        *self.0.lock().unwrap()
    }
}

fn consent_state(clock: Arc<TestClock>) -> Arc<CohortManifestState> {
    consent_state_for(
        clock,
        "cross-team-test",
        "region-a",
        "region-b",
        "maos_team_a".to_string(),
        "maos_team_b".to_string(),
        "collective:share",
    )
}

fn consent_state_for(
    clock: Arc<TestClock>,
    cohort_id: &str,
    team_a_region: &str,
    team_b_region: &str,
    team_a_datname: String,
    team_b_datname: String,
    grant_intent: &str,
) -> Arc<CohortManifestState> {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V3,
        cohort_id: cohort_id.to_string(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signing_key.verifying_key().to_bytes())],
        },
        members: vec![
            CohortMember {
                host_id: "host-a".to_string(),
                fingerprint: format!("sha256:{}", "ab".repeat(32)),
                roles: vec!["worker".to_string()],
                team: None,
            },
            CohortMember {
                host_id: "host-b".to_string(),
                fingerprint: format!("sha256:{}", "cd".repeat(32)),
                roles: vec!["worker".to_string()],
                team: None,
            },
        ],
        consent: ConsentMatrix::default(),
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams: Some(vec![
            TeamEntry {
                team_id: TeamId::new("team-a").unwrap(),
                region: Region::canonicalize(team_a_region).unwrap(),
                datname: team_a_datname,
                members: vec![SpiritId::from("spirit-a")],
            },
            TeamEntry {
                team_id: TeamId::new("team-b").unwrap(),
                region: Region::canonicalize(team_b_region).unwrap(),
                datname: team_b_datname,
                members: vec![SpiritId::from("spirit-b")],
            },
        ]),
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: vec![CrossTeamConsentGrant {
            from_team: TeamId::new("team-a").unwrap(),
            to_team: TeamId::new("team-b").unwrap(),
            intent: grant_intent.to_string(),
        }],
    }
    .signed_with(&signing_key);
    let signed_toml = toml::to_string(&manifest).unwrap();
    let pins = PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()]).unwrap();
    Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host-a".to_string()),
            &signed_toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
            clock,
        )
        .unwrap(),
    )
}

#[test]
fn directional_grant_and_stale_state_are_distinguishable() {
    let clock = Arc::new(TestClock::default());
    let state = consent_state(Arc::clone(&clock));
    let team_a_region = Region::canonicalize("region-a").unwrap();
    let team_a_id = TeamId::new("team-a").unwrap();
    let keys = derive_team_verifying_keys(&state, &[0x42; 32]).unwrap();
    assert_eq!(
        keys.get(&(team_a_region.clone(), team_a_id.clone())),
        Some(&maos_audit::sealed_export::derive_team_pubkey(
            &[0x42; 32],
            &team_a_region,
            &team_a_id,
        ))
    );
    let adapter = CrossTeamConsentAdapter::new(state);
    let team_a = TeamId::new("team-a").unwrap();
    let team_b = TeamId::new("team-b").unwrap();

    assert!(adapter
        .is_granted(&team_a, &team_b, "collective:share")
        .unwrap());
    assert!(!adapter
        .is_granted(&team_b, &team_a, "collective:share")
        .unwrap());

    clock.advance(121);
    assert!(matches!(
        adapter.is_granted(&team_a, &team_b, "collective:share"),
        Err(CrossTeamConsentError::Stale { .. })
    ));
}

// ─── Story 13.6b — clause (f-i) is INVERTED, not deleted ────────────────────
//
// `replication_crossing_has_no_production_initiator` lived here and asserted the
// crossing had NO production initiator. Story 13.6b wired one, so the negative
// was inverted and REPLACED in the same commit (11.3 D2 / 10.4c atomic cutover —
// never left half-deleted). Its replacements live in
// `crates/maos-bin/tests/cross_team_crossing_13_6b.rs`:
//
//   * `crossing_has_a_production_initiator_at_both_endpoints` — the positive,
//     naming the emitter AND the destination applier;
//   * `crossing_scan_closes_the_originate_team_row_hole` — closes D-6b, the hole
//     the retired scan had: it skipped `replication/bundle.rs` and never named
//     `originate_team_row(`, so routing the call one file over kept it green.
//     Measured at cutover: the retired needle set saw only the applier's
//     `apply_replication_bundle(` and was BLIND to the emitter;
//   * `exactly_one_production_loom_lite_store_construction` — the D-5 one-store
//     wall as a control rather than a sentence;
//   * `crossing_weld_refuses_a_forged_payload_team_before_apply` — the envelope/
//     payload weld (AC3), with the seed-holding forger the shipped relabel
//     negative structurally cannot reach.

#[test]
fn cross_wall_recall_manifest_direction_and_staleness_are_typed() {
    let clock = Arc::new(TestClock::default());
    let state = consent_state_for(
        Arc::clone(&clock),
        "cross-wall-recall-test",
        "region-a",
        "region-b",
        "maos_team_a".to_string(),
        "maos_team_b".to_string(),
        "log:recall",
    );
    let team_a = TeamId::new("team-a").unwrap();
    let team_b = TeamId::new("team-b").unwrap();
    let team_c = TeamId::new("team-c").unwrap();
    let source = CrossWallRecallConsentAdapter::new(Arc::clone(&state), team_a.clone());
    assert_eq!(
        source.decide(&team_b, "log:recall").unwrap(),
        CrossWallRecallConsentDecision::Granted
    );
    let destination = CrossWallRecallConsentAdapter::new(Arc::clone(&state), team_b);
    assert_eq!(
        destination.decide(&team_a, "log:recall").unwrap(),
        CrossWallRecallConsentDecision::WrongDirection
    );
    assert_eq!(
        source.decide(&team_c, "log:recall").unwrap(),
        CrossWallRecallConsentDecision::NoGrant
    );
    clock.advance(121);
    assert!(matches!(
        source.decide(&team_c, "log:recall"),
        Err(CrossWallRecallConsentError::Stale { .. })
    ));
}

#[test]
fn cross_wall_recall_live_path_uses_verified_state_and_home_team() {
    let _lock = LIVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let _restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("MAOS_HOME", home.path());
    let state = consent_state_for(
        Arc::new(TestClock::default()),
        "cross-wall-live-path",
        "region-a",
        "region-b",
        "maos_team_a".to_string(),
        "maos_team_b".to_string(),
        "log:recall",
    );
    let home_team = TeamId::new("team-a").unwrap();
    let remote_team = TeamId::new("team-b").unwrap();
    let remote_path =
        maos_audit::transparency_log_path_for_tenant_mode(true, Some(remote_team.as_str()))
            .unwrap();
    std::fs::create_dir_all(remote_path.parent().unwrap()).unwrap();
    let remote_log = TransparencyLogAdapter::open(&remote_path, 0x136D).unwrap();
    remote_log.insert_frame_event(
        FrameKind::TaskAssign,
        42,
        None,
        "remote-live-path",
        b"remote",
        FrameOrigin::HumanAuthored,
    );
    drop(remote_log);
    maos_audit::write_tenant_binding(&remote_path, &remote_team, Some("maos_team_b")).unwrap();

    let adapter = LogRecallAdapter::new(Arc::new(TransparencyLogAdapter::open_in_memory(0x136E)))
        .with_cross_wall_consent(Arc::new(CrossWallRecallConsentAdapter::new(
            state, home_team,
        )))
        .with_cross_wall_read(Arc::new(CrossWallLogReadAdapter::new(true)));
    let page = adapter
        .recall_cross_wall(42, &remote_team, LogRecallFilter::default())
        .unwrap();
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].intent, "remote-live-path");
}

#[test]
fn log_recall_filter_has_no_team_dimension() {
    let source = include_str!("../src/../../maos-domain/src/log_recall.rs");
    let fields = source
        .split("pub struct LogRecallFilter {")
        .nth(1)
        .and_then(|tail| tail.split("\n}").next())
        .expect("LogRecallFilter fields");
    let constructor = source
        .split("impl LogRecallFilter {")
        .nth(1)
        .and_then(|tail| tail.split("impl Default for LogRecallFilter").next())
        .expect("LogRecallFilter constructor");
    assert!(
        !fields.contains("TeamId")
            && !fields
                .lines()
                .any(|line| line.trim_start().starts_with("team:"))
            && !constructor.contains("TeamId")
            && !constructor.contains("remote_team"),
        "Story 13.6d security property: LLM-built LogRecallFilter must not name a team"
    );
    assert!(
        source.contains("pub struct CrossWallRecallRequest")
            && source.contains("remote_team: TeamId")
            && source.contains("remote_team: TeamId::new(remote_team)?"),
        "the operator-only sibling request must own the validated team dimension"
    );
}

#[test]
fn researcher_recall_surface_cannot_import_unscoped_ranged_recall() {
    let cargo = include_str!("../../../spirits/researcher/Cargo.toml");
    let source = include_str!("../../../spirits/researcher/src/lib.rs");
    assert!(
        !cargo.contains("maos-audit"),
        "Researcher must not gain the crate that exposes ranged_recall"
    );
    assert!(
        !source.contains("ranged_recall("),
        "Researcher production code must stay compile-pinned to LogRecallPort"
    );
}

#[test]
fn cross_wall_recall_has_production_caller_and_live_preconditions() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("workspace root");
    let mut callers = Vec::new();
    let mut stack = vec![
        workspace_root.join("crates"),
        workspace_root.join("spirits"),
    ];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if matches!(name, "tests" | "benches" | "examples" | "target")
                    || name.starts_with('.')
                {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                let text = std::fs::read_to_string(&path).expect("read source");
                let production = text.split("#[cfg(test)]").next().unwrap_or(&text);
                if production.contains(".recall_cross_wall(") {
                    callers.push(path);
                }
            }
        }
    }
    assert_eq!(
        callers,
        vec![workspace_root.join("crates/maos-bin/src/main.rs")],
        "Story 13.6d owns exactly one production caller in the composition root"
    );
    let main = include_str!("../src/main.rs");
    let wiring = main
        .split("LogRecallAdapter + directional manifest-consent seam")
        .nth(1)
        .and_then(|tail| tail.split("let log_recall_adapter").next())
        .expect("cross-wall composition-root wiring");
    // All four preconditions must live inside this single conditional branch,
    // in source order — relocating any token outside the guard reds the leg
    // (independent substring checks could not catch that).
    let mut cursor = 0usize;
    for token in [
        "cohort_daemon.as_ref()",
        "MAOS_LOOM_HOME_TEAM",
        "with_cross_wall_consent",
        "with_cross_wall_read",
    ] {
        cursor = wiring[cursor..]
            .find(token)
            .unwrap_or_else(|| panic!("cross-wall wiring missing token in order: {token}"))
            + cursor;
    }
    // The composition-root dispatch must be wired, not merely defined: a dead
    // `run_cross_wall_traceback` helper with the dispatch deleted would keep a
    // substring scanner green, so assert the dispatch site itself.
    let dispatch = main
        .split("if let Some(request) = cross_wall_traceback")
        .nth(1)
        .and_then(|tail| tail.split("let memory_any").next())
        .expect("cross-wall traceback dispatch block");
    assert!(
        dispatch.contains("run_cross_wall_traceback"),
        "the composition root must dispatch to run_cross_wall_traceback"
    );
}

struct GrantedCrossWallConsent;

impl CrossWallRecallConsentPort for GrantedCrossWallConsent {
    fn decide(
        &self,
        _team: &TeamId,
        _intent: &str,
    ) -> Result<CrossWallRecallConsentDecision, CrossWallRecallConsentError> {
        Ok(CrossWallRecallConsentDecision::Granted)
    }
}

struct SucceedingCrossWallRead;

impl CrossWallLogReadPort for SucceedingCrossWallRead {
    fn read_remote(
        &self,
        _spirit_pid: u32,
        _remote_team: &TeamId,
        _filter: LogRecallFilter,
    ) -> Result<LogRecallPage, LogRecallError> {
        Ok(LogRecallPage::new(Vec::new(), None))
    }
}

fn cross_wall_capability_rows(
    tl: &TransparencyLogAdapter,
) -> Vec<maos_iac::adapter::transparency_log::TransparencyLogEntry> {
    tl.query_frames(FrameFilter {
        kind: Some(FrameKind::CapabilityInvocation),
        ..Default::default()
    })
    .expect("query cross-wall audit rows")
}

#[test]
fn cross_wall_recall_refusals_and_disclosures_are_journaled() {
    // Behavioral (not a source-scan): drive a refusal and a disclosure through
    // a real LogRecallAdapter and assert the audit rows. AC5 / ADR-049 §7: a
    // refusal and a disclosure each journal a CapabilityInvocation row, never
    // folded into an empty page; a disclosure claims crossed_team_boundary only
    // once the read succeeded.
    let team = TeamId::new("team-a").unwrap();

    // Refusal: no consent provider → NoConsentProvider, journaled "refused".
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x13_6D));
    let error = LogRecallAdapter::new(Arc::clone(&tl))
        .recall_cross_wall(7, &team, LogRecallFilter::default())
        .unwrap_err();
    assert!(matches!(
        error,
        LogRecallError::ECrossWallRecallDenied { .. }
    ));
    let rows = cross_wall_capability_rows(&tl);
    assert_eq!(rows.len(), 1, "a refused recall journals one audit row");
    let payload: serde_json::Value = serde_json::from_slice(&rows[0].payload_redacted).unwrap();
    assert_eq!(payload["outcome"], "refused");
    assert_eq!(payload["crossed_team_boundary"], false);

    // Disclosure: consent granted + succeeding read → "disclosing" then
    // "disclosed"; crossed_team_boundary is true only on the outcome row.
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0x13_6E));
    LogRecallAdapter::new(Arc::clone(&tl))
        .with_cross_wall_consent(Arc::new(GrantedCrossWallConsent))
        .with_cross_wall_read(Arc::new(SucceedingCrossWallRead))
        .recall_cross_wall(7, &team, LogRecallFilter::default())
        .unwrap();
    let rows = cross_wall_capability_rows(&tl);
    assert_eq!(
        rows.len(),
        2,
        "a disclosed recall journals intent then outcome"
    );
    let outcomes: Vec<String> = rows
        .iter()
        .map(|row| {
            serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                .unwrap()
                .get("outcome")
                .and_then(|value| value.as_str())
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(outcomes, vec!["disclosing", "disclosed"]);
}

#[test]
fn cross_wall_traceback_refuses_without_cohort_preconditions() {
    // Composition-root guard (AC3 / 13.5g): `maos traceback` must reach
    // recall_cross_wall and fail closed (NoConsentProvider) when the cohort
    // preconditions are absent. Deleting the dispatch would make the binary not
    // recognize the request and fail differently → this leg reds. Mirrors the
    // dev T8 smoke; BOOTS ≠ SERVES.
    let _restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("MAOS_HOME", home.path());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_maos"))
        .args([
            "traceback",
            "--team",
            "team-b",
            "--spirit-pid",
            "42",
            "--limit",
            "2",
        ])
        .output()
        .expect("maos binary runs");
    assert!(
        !output.status.success(),
        "cross-wall traceback without cohort state must fail closed, not exit 0"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cross_wall_traceback") && stderr.contains("refused"),
        "cross-wall traceback must surface a typed refusal; got stderr: {stderr}"
    );
}

// ─── Live composition-level crossing (13.3 review) ──────────────────────────
//
// The headline asymmetric negative must be observed through the PRODUCTION
// consent surface — a signed V3 manifest feeding `CrossTeamConsentAdapter` —
// over two physical databases, not a hard-coded consent stub on one database.

static LIVE_LOCK: Mutex<()> = Mutex::new(());

fn pg_conn_team(team: &str) -> String {
    let var = match team {
        "team-a" => "MAOS_TEST_POSTGRES_TEAM_A",
        "team-b" => "MAOS_TEST_POSTGRES_TEAM_B",
        other => panic!("unknown team {other}"),
    };
    std::env::var(var).unwrap_or_else(|_| panic!("{var} must be set for the live crossing test"))
}

/// Parse the database name from a Postgres connection string (URL or
/// key-value form) so manifest datnames match the physical fixture.
fn datname_of(conn: &str) -> String {
    if let Some((_, rest)) = conn.split_once("dbname=") {
        return rest.split_whitespace().next().unwrap_or(rest).to_string();
    }
    conn.rsplit('/')
        .next()
        .unwrap_or(conn)
        .split('?')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn live_consent_state(clock: Arc<TestClock>) -> Arc<CohortManifestState> {
    // Both teams in ONE canonical region (the §A7 same-region reflex —
    // cross-region is already refused by the region axis), datnames matching
    // the physical fixture databases.
    consent_state_for(
        clock,
        "cross-team-live",
        "region-a",
        "region-a",
        datname_of(&pg_conn_team("team-a")),
        datname_of(&pg_conn_team("team-b")),
        "collective:share",
    )
}

struct LiveTeamMap {
    home_team: TeamId,
    home_datname: String,
}

impl TenantMapPort for LiveTeamMap {
    fn team_of(&self, spirit_pid: u32) -> Result<TeamId, TenantMapError> {
        if spirit_pid == 7 {
            Ok(self.home_team.clone())
        } else {
            Err(TenantMapError::SpiritUnmapped { spirit_pid })
        }
    }

    fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError> {
        if team == &self.home_team {
            Ok(self.home_datname.clone())
        } else {
            Err(TenantMapError::TeamUnknown {
                team_id: team.clone(),
            })
        }
    }

    fn register_spirit(&self, _spirit_pid: u32, _spirit_id: SpiritId) {}
}

async fn live_team_store(
    team: &str,
    state: &Arc<CohortManifestState>,
    seed: &[u8; 32],
) -> LoomLiteStore {
    let verifying_keys = derive_team_verifying_keys(state, seed).expect("manifest-derived keys");
    let store = LoomLiteStore::new(StoreConfig {
        connection_string: pg_conn_team(team),
        home_region: "region-a".to_string(),
        home_team: team.to_string(),
        ..StoreConfig::default()
    })
    .await
    .expect("store creation must succeed")
    .with_cross_team_consent(Arc::new(CrossTeamConsentAdapter::new(Arc::clone(state))))
    .with_tenant_map(Arc::new(LiveTeamMap {
        home_team: TeamId::new(team).unwrap(),
        home_datname: datname_of(&pg_conn_team(team)),
    }))
    .with_team_verifying_keys(verifying_keys);
    store.init_schema().await.expect("schema init must succeed");
    store
}

fn live_leaf(key: &str, value: &str) -> CollectiveKvLeaf {
    CollectiveKvLeaf {
        source_region: "region-a".to_string(),
        source_ts: 1,
        spirit_pid: 7,
        namespace_kind: "default".to_string(),
        namespace_detail: String::new(),
        key: key.to_string(),
        value_kind: "text".to_string(),
        value_data: value.as_bytes().to_vec(),
        // Deliberately UNSTAMPED: a leaf with NO origin is the first-party
        // PROMOTION case, which the builder still performs (13.3 review F1
        // regression cover — a first-party leaf must not land as a
        // permanently unreadable row). A leaf that already carried a foreign
        // origin would be REFUSED instead of relabelled (13.3b rework).
        source_team: None,
        distillation_depth: None,
        intent_lineage: None,
    }
}

#[tokio::test]
#[ignore = "requires live Postgres (MAOS_TEST_POSTGRES_TEAM_A/_B)"]
async fn asymmetric_consent_reverse_share_refused() {
    let _evidence = evidence_record::attest("asymmetric_consent_reverse_share_refused");
    let _g = LIVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let clock = Arc::new(TestClock::default());
    let state = live_consent_state(Arc::clone(&clock));
    let seed = [0x42u8; 32];
    let region = Region::canonicalize("region-a").unwrap();
    let team_a = TeamId::new("team-a").unwrap();
    let team_b = TeamId::new("team-b").unwrap();

    let store_b = live_team_store("team-b", &state, &seed).await;

    // A→B is granted by the SIGNED V3 manifest: the crossing lands and the
    // row reads back through the attestation guard with manifest-derived
    // keys — the full production consent surface, not a stub.
    let bundle = build_replication_bundle_v2(
        vec![live_leaf("composition-row", "signed-and-consented")],
        &region,
        &team_a,
        &seed,
    )
    .expect("first-party promotion builds");
    let result = apply_replication_bundle(
        &bundle,
        &store_b,
        "region-a",
        Some(CrossTeamApplyContext::new(&team_b, "collective:share")),
        &seed,
    )
    .await
    .unwrap();
    assert_eq!(result.applied_count, 1);
    assert_eq!(
        store_b
            .read(7, &MemoryNamespace::Default, "composition-row")
            .await
            .unwrap(),
        Some(MemoryValue::Text("signed-and-consented".to_string()))
    );

    // B→A: the signed manifest grants only A→B — ConsentDenied, and nothing
    // is served from team A's own store. Read-None is a valid absence
    // witness HERE: a wrongly landed row would carry full attestation and
    // this store holds both teams' verifying keys, so it WOULD be served.
    let store_a = live_team_store("team-a", &state, &seed).await;
    let reverse = build_replication_bundle_v2(
        vec![live_leaf("reverse-must-not-land", "denied")],
        &region,
        &team_b,
        &seed,
    )
    .expect("first-party promotion builds");
    assert!(matches!(
        apply_replication_bundle(
            &reverse,
            &store_a,
            "region-a",
            Some(CrossTeamApplyContext::new(&team_a, "collective:share")),
            &seed,
        )
        .await,
        Err(BundleError::ConsentDenied { .. })
    ));
    assert_eq!(
        store_a
            .read(7, &MemoryNamespace::Default, "reverse-must-not-land")
            .await
            .unwrap(),
        None,
        "denied reverse crossing must not be served from team A's database"
    );
    assert!(
        store_a
            .scan(7, &MemoryNamespace::Default, "reverse", 10)
            .await
            .unwrap()
            .is_empty(),
        "denied reverse crossing must not appear in team A's scan"
    );

    // A stale lease is distinguishable from no-grant (AC4): advance the
    // clock past t_stale_secs and the SAME granted crossing refuses typed.
    clock.advance(121);
    assert!(matches!(
        apply_replication_bundle(
            &bundle,
            &store_b,
            "region-a",
            Some(CrossTeamApplyContext::new(&team_b, "collective:share")),
            &seed,
        )
        .await,
        Err(BundleError::ConsentStateStale { .. })
    ));
}

/// Story 13.6 closure — the falsifier behind a composition-root exemption.
///
/// `CrossTeamConsentAdapter` is constructed twice in `main.rs`: once in the
/// collective-store setup and once in the cohort A2A daemon's crossing-port
/// construction, where `with_erase_reconciliation` needs the consent
/// projection to authorize a two-sided erase. `check-composition-root-
/// completeness` exempts it in `xtask/composition-root-whitelist.toml`, and
/// that exemption rests on ONE claim: the adapter owns no state of its own, so
/// two instances over the same `bootstrap.state` Arc cannot diverge.
///
/// This test IS that claim. Add a second field — a cache, a clock, a counter —
/// and the premise is false, the two instances become genuinely independent,
/// and this reds so the exemption gets re-argued instead of rotting.
#[test]
fn cross_team_consent_adapter_is_a_pure_shared_state_projection() {
    const ADAPTER_SRC: &str = include_str!("../src/cross_team_consent.rs");

    let body = ADAPTER_SRC
        .split_once("pub struct CrossTeamConsentAdapter {")
        .expect("adapter struct is declared with a brace body")
        .1
        .split_once('}')
        .expect("adapter struct body is terminated")
        .0;

    let fields: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
        .collect();

    assert_eq!(
        fields,
        vec!["state: Arc<CohortManifestState>,"],
        "CrossTeamConsentAdapter must stay a pure projection over the shared \
         manifest state — its composition-root duplicate-construction exemption \
         (xtask/composition-root-whitelist.toml) is only sound while the adapter \
         owns nothing that two instances could diverge on"
    );
}
