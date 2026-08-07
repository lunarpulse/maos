#![cfg(feature = "network")]

//! Story 13.6b — the crossing crosses, and the team that crossed it is the team
//! that signed it.
//!
//! Every leg here is registered `Blocking` on `check-multi-tenant-loom` with one
//! `#[test]` per `--exact` invocation: that gate's only anti-vacuity oracle is
//! `"running 1 test"` + `"1 passed"`, so a collapsed registry would let a null
//! assertion print green.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use maos_a2a_core::{
    A2APeerConfig, A2AProfile, ConsentAllowlists, CrossTeamCrossingPort, CrossingOutcome,
    CrossingRefusal, PeerCertFingerprint, PeerId,
};
use maos_bin::cross_team_crossing::{
    crossing_frame, reconcile_home_team_with_manifest, CrossTeamCrossingAdapter,
    CrossTeamCrossingControl, CrossTeamShareRequest,
};
use maos_cohort::{
    CohortAuthority, CohortClock, CohortManifest, CohortManifestState, CohortMember, ConsentMatrix,
    ConsentTuple, CrossTeamConsentGrant, InMemoryCohortAuditSink, ManifestSignature,
    PinnedAuthorityKeys, TeamEntry, COHORT_SCHEMA_V3, COHORT_SCHEMA_V4,
    RESERVED_INTENT_HALT_RECEIPT, RESERVED_INTENT_REISSUE,
};
use maos_domain::frame::FrameAddress;
use maos_domain::invariants::i8::A2AIntent;
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_loom_lite::replication::bundle::{
    build_replication_bundle_v2, verify_replication_bundle, CrossRegionReplicationBundle,
};
use maos_loom_lite::replication::leaf::CollectiveKvLeaf;
use maos_loom_lite::store::{CollectiveRow, LoomLiteStore, StoreConfig};
use maos_spirit_abi::identity::{HostId, SpiritId};
use tokio_postgres::NoTls;

// Story 13.6e (AC3) — the harness signs its own transcript record; the gate
// only verifies. Shared signer, no new crate and no new dependency.
#[path = "../../../tests/harness/evidence_record.rs"]
mod evidence_record;

const SHARE_ENV_KEYS: [&str; 7] = [
    "MAOS_CROSS_TEAM_SHARE_PEER",
    "MAOS_CROSS_TEAM_SHARE_TO_TEAM",
    "MAOS_CROSS_TEAM_SHARE_PID",
    "MAOS_CROSS_TEAM_SHARE_NAMESPACE",
    "MAOS_CROSS_TEAM_SHARE_KEY",
    "MAOS_CROSS_TEAM_SHARE_VALUE",
    "MAOS_CROSS_TEAM_BASE_SEED",
];
static SHARE_ENV_LOCK: Mutex<()> = Mutex::new(());

struct ShareEnvRestore(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl ShareEnvRestore {
    fn isolate() -> Self {
        let saved = SHARE_ENV_KEYS
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect();
        for key in SHARE_ENV_KEYS {
            std::env::remove_var(key);
        }
        Self(saved)
    }
}

impl Drop for ShareEnvRestore {
    fn drop(&mut self) {
        for (key, value) in self.0.drain(..) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn set_valid_share_env() {
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_PEER", "host-b");
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_TO_TEAM", "team-b");
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_PID", "7");
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_NAMESPACE", "default");
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_KEY", "crossing-key");
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_VALUE", "crossing-value");
}

#[cfg(unix)]
#[test]
fn crossing_request_rejects_non_utf8_peer_configuration() {
    use std::os::unix::ffi::OsStringExt;

    let _lock = SHARE_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let _restore = ShareEnvRestore::isolate();
    set_valid_share_env();
    std::env::set_var(
        "MAOS_CROSS_TEAM_SHARE_PEER",
        std::ffi::OsString::from_vec(vec![0xff]),
    );
    let error = CrossTeamShareRequest::from_env().expect_err("unreadable peer must fail closed");
    assert!(error.contains("PEER") && error.contains("UTF-8"));
}

#[cfg(unix)]
#[test]
fn crossing_request_rejects_non_utf8_namespace_configuration() {
    use std::os::unix::ffi::OsStringExt;

    let _lock = SHARE_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let _restore = ShareEnvRestore::isolate();
    set_valid_share_env();
    std::env::set_var(
        "MAOS_CROSS_TEAM_SHARE_NAMESPACE",
        std::ffi::OsString::from_vec(vec![0xff]),
    );
    let error =
        CrossTeamShareRequest::from_env().expect_err("unreadable namespace must fail closed");
    assert!(error.contains("NAMESPACE") && error.contains("UTF-8"));
}

#[test]
fn crossing_request_rejects_an_empty_key() {
    let _lock = SHARE_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let _restore = ShareEnvRestore::isolate();
    set_valid_share_env();
    std::env::set_var("MAOS_CROSS_TEAM_SHARE_KEY", "");
    let error = CrossTeamShareRequest::from_env().expect_err("an empty key must fail closed");
    assert!(error.contains("KEY") && error.contains("empty"));
}

// ─── Shared static-scan machinery (AC5) ─────────────────────────────────────
//
// The dead-wire negative this story INVERTS
// (`replication_crossing_has_no_production_initiator`) had a hole: it skipped
// `replication/bundle.rs` and never named `originate_team_row`, which lives in
// that file and calls `build_replication_bundle_v2` internally. A production
// caller of `originate_team_row` therefore inverted the crossing IN FACT while
// the negative stayed green (D-6b). Both the replacement positive leg and the
// hole-closure proof run through THIS one scanner, so the closure is a property
// of the shared code path rather than of one test's needle list.

/// Needles that mean "a production module reaches the crossing". Story 13.6b
/// adds `originate_team_row(` — the indirection the shipped scan was blind to.
const CROSSING_NEEDLES: [&str; 4] = [
    "apply_replication_bundle(",
    "build_replication_bundle(",
    "build_replication_bundle_v2(",
    "originate_team_row(",
];

/// The exact needle set the pre-13.6b gate used, retained ONLY so
/// [`crossing_scan_closes_the_originate_team_row_hole`] can prove the old set
/// was blind to the indirection and the new set is not.
const PRE_13_6B_NEEDLES: [&str; 3] = [
    "apply_replication_bundle(",
    "build_replication_bundle(",
    "build_replication_bundle_v2(",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// Walk production Rust sources under `roots` and report every
/// `path: needle xN` hit for `needles`.
///
/// "Production" excludes `tests`/`benches`/`examples`/`target` directories AND
/// everything from the first `#[cfg(test)]` marker onward, so an inline unit-test
/// module can never satisfy — or falsely red — a production-wiring assertion.
fn scan_production(roots: &[PathBuf], needles: &[&str], skip_defining_module: bool) -> Vec<String> {
    let mut hits = Vec::new();
    let mut stack: Vec<PathBuf> = roots.to_vec();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(name, "tests" | "benches" | "examples" | "target")
                    || name.starts_with('.')
                {
                    continue;
                }
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if skip_defining_module && path.ends_with("replication/bundle.rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).expect("read source");
                let production = text.split("#[cfg(test)]").next().unwrap_or_default();
                for needle in needles {
                    let count = production.matches(needle).count();
                    if count > 0 {
                        hits.push(format!("{}: {needle} x{count}", path.display()));
                    }
                }
            }
        }
    }
    hits.sort();
    hits
}

fn production_roots() -> Vec<PathBuf> {
    let root = workspace_root();
    vec![root.join("crates"), root.join("spirits")]
}

/// Extract the brace-balanced body of the item whose declaration contains
/// `signature`. Panics if the signature is absent — an assertion that silently
/// scoped itself to nothing is the failure mode 13.5g caught.
fn item_body<'a>(src: &'a str, signature: &str) -> &'a str {
    let start = src
        .find(signature)
        .unwrap_or_else(|| panic!("declaration `{signature}` not found — did it get renamed?"));
    let open = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("`{signature}` has no body"))
        + start;
    let mut depth = 0usize;
    for (offset, ch) in src[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open..open + offset + 1];
                }
            }
            _ => {}
        }
    }
    panic!("`{signature}` body is unbalanced");
}

fn read_source(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {}", path.display()))
}

// ─── AC1 / AC5 — the inverted dead-wire clause and its replacements ─────────

/// Story 13.6b / AC1+AC5 — replaces `replication-crossing-has-no-production-
/// initiator`, inverted in this same commit (11.3 D2 / 10.4c atomic cutover).
///
/// The retired negative asserted the crossing had NO production initiator. Its
/// replacement asserts the opposite at **both endpoints**, and — this is the part
/// 13.5g's open finding demands — it asserts **reachability**, not presence.
///
/// The first draft of this leg was a text scan for the crossing needles. The
/// serialized proven-red pass killed it: deleting the emitter CALL from
/// `run_cohort_a2a_daemon` left the leg green, because `emit_cross_team_share`
/// still contained the needle. That is exactly 13.5g's finding — *"legs green
/// while connecting to nothing"* — reproduced inside this story's own gate. So
/// the leg now walks the call chain hop by hop:
///
/// ```text
///   dispatch arm  →  run_cohort_a2a_daemon  →  emit_cross_team_share
///                                              →  originate_team_row (13.3b seam, D-6)
///                                              →  route_outbound      (the only outbound path, D-14)
///   handle_intake_verified (router.rs, D-8)  →  apply_crossing
///                                              →  apply_replication_bundle → is_granted (D-1)
/// ```
///
/// Breaking ANY hop reds this leg. AC5's composition-root test is therefore
/// satisfied by construction rather than by inspection.
#[test]
fn crossing_has_a_production_initiator_at_both_endpoints() {
    let main_rs = read_source("crates/maos-bin/src/main.rs");
    let crossing_rs = read_source("crates/maos-bin/src/cross_team_crossing.rs");
    let router_rs = read_source("crates/maos-a2a-core/src/router.rs");

    // ── EMITTER: dispatch → daemon → emit → 13.3b seam → the one outbound path.
    let dispatch = item_body(&main_rs, r#"if mode == "cohort-a2a-daemon""#);
    assert!(
        dispatch.contains("run_cohort_a2a_daemon("),
        "D-14: the daemon must still be reachable from the MAOS_ONE_SHOT dispatch"
    );
    let daemon = item_body(&main_rs, "async fn run_cohort_a2a_daemon(");
    assert!(
        daemon.contains("emit_cross_team_share("),
        "AC1/AC5 (13.5g): the emitter must be reachable FROM INSIDE the daemon runtime — a \
         crossing builder that merely exists in the file is dead wire"
    );
    assert!(
        daemon.contains("crossing_port"),
        "AC1: the daemon must install the applier port before its accept loop spawns"
    );
    let emitter = item_body(&main_rs, "async fn emit_cross_team_share(");
    assert!(
        emitter.contains("originate_team_row("),
        "D-6: the emitter must use the seam 13.3b left, not hand-roll leaf construction"
    );
    assert!(
        emitter.contains("route_outbound("),
        "D-14: the crossing must leave through the ONLY production outbound A2A path, so \
         `prepare_outbound` stamps cohort_source_team from the SIGNED declaration"
    );

    // ── APPLIER: the spoof-proof intake site → the port → apply → is_granted.
    let intake = item_body(&router_rs, "pub async fn handle_intake_verified(");
    assert!(
        intake.contains("apply_crossing("),
        "D-8: the applier must hang off handle_intake_verified (12.3 P5r), never handle_intake"
    );
    assert!(
        crossing_rs.contains("apply_replication_bundle("),
        "AC1: the applier must reach apply_replication_bundle — D-1's is_granted call site gets \
         its first non-test caller through it"
    );

    // ── The needle scan still runs, as the belt to that braces: no OTHER
    // production module may quietly acquire a crossing call site.
    let hits = scan_production(&production_roots(), &CROSSING_NEEDLES, true);
    let files: BTreeSet<&str> = hits
        .iter()
        .filter_map(|hit| hit.split(':').next())
        .collect();
    let expected = [
        "maos-bin/src/main.rs",
        "maos-bin/src/cross_team_crossing.rs",
    ];
    for file in &files {
        assert!(
            expected.iter().any(|allowed| file.ends_with(allowed)),
            "an unexpected production module reached the crossing: {file} — a crossing outside \
             the composition root cannot honour the D-5 one-store wall; hits={hits:?}"
        );
    }
    for allowed in expected {
        assert!(
            files.iter().any(|file| file.ends_with(allowed)),
            "AC1: {allowed} no longer reaches the crossing; hits={hits:?}"
        );
    }
}

/// Story 13.6b / AC5 — the D-6b hole is CLOSED, and the closure is proven, not
/// asserted.
///
/// A synthetic production module whose only reference to the crossing is
/// `originate_team_row(` is:
///   * **invisible** to the pre-13.6b needle set (that is the hole — routing the
///     call one file over kept `replication-crossing-has-no-production-initiator`
///     green while the crossing was live), and
///   * **caught** by this story's needle set.
///
/// This is the fixture AC5 demands: it reds the replacement leg's scanner.
#[test]
fn crossing_scan_closes_the_originate_team_row_hole() {
    let dir = tempfile::tempdir().expect("temp dir");
    let crate_src = dir.path().join("crates/fixture/src");
    std::fs::create_dir_all(&crate_src).expect("fixture tree");
    std::fs::write(
        crate_src.join("emitter.rs"),
        "pub async fn emit() {\n    let _ = originate_team_row(store, 1).await;\n}\n",
    )
    .expect("write fixture");
    let roots = vec![dir.path().join("crates")];

    let blind = scan_production(&roots, &PRE_13_6B_NEEDLES, true);
    assert!(
        blind.is_empty(),
        "the pre-13.6b needle set must be demonstrably BLIND to the originate_team_row \
         indirection — that blindness is D-6b; hits={blind:?}"
    );

    let closed = scan_production(&roots, &CROSSING_NEEDLES, true);
    assert_eq!(
        closed.len(),
        1,
        "AC5: the replacement scan must catch a production caller of originate_team_row; \
         hits={closed:?}"
    );
    assert!(closed[0].contains("originate_team_row("));
}

/// Story 13.6b / AC1 — the one-store wall is a CONTROL, not a sentence.
///
/// D-5: exactly one production `LoomLiteStore::new`, pinned to
/// `MAOS_LOOM_HOME_TEAM`, with `connection_assignment_guard` proving
/// `datname_for(home_team) == current_database()`. A second construction would
/// let one process straddle two tenants and "demonstrate" a crossing by
/// defeating ADR-055 in the act. Proven-red by adding a second production
/// construction.
#[test]
fn exactly_one_production_loom_lite_store_construction() {
    let hits = scan_production(&production_roots(), &["LoomLiteStore::new("], false);
    assert_eq!(
        hits.len(),
        1,
        "D-5: a crossing must be TWO daemons, never one process holding two stores; \
         production LoomLiteStore::new sites = {hits:?}"
    );
    assert!(
        hits[0].contains("maos-bin/src/main.rs"),
        "the single store construction must stay in the composition root; hit={:?}",
        hits[0]
    );
}

// ─── AC3 — the envelope/payload weld ────────────────────────────────────────

fn crossing_leaf(key: &str, value: &str) -> CollectiveKvLeaf {
    CollectiveKvLeaf {
        source_region: "region-a".to_string(),
        source_ts: 1,
        spirit_pid: 7,
        namespace_kind: "default".to_string(),
        namespace_detail: String::new(),
        key: key.to_string(),
        value_kind: "text".to_string(),
        value_data: value.as_bytes().to_vec(),
        source_team: None,
        distillation_depth: None,
        intent_lineage: None,
    }
}

/// A store whose Postgres host does not exist. `LoomLiteStore::new` succeeds
/// anyway (deadpool builds lazily — documented at `store.rs`), so ANY store
/// access from the applier surfaces as a pool error. That is what makes
/// "refused with `SourceTeamUnbound`" positive evidence that the weld ran
/// **before** `apply_replication_bundle` touched anything.
async fn dead_store(home_team: &str) -> Arc<LoomLiteStore> {
    Arc::new(
        LoomLiteStore::new(StoreConfig {
            connection_string:
                "host=127.0.0.1 port=1 user=nobody dbname=maos_absent connect_timeout=1"
                    .to_string(),
            home_region: "region-a".to_string(),
            home_team: home_team.to_string(),
            ..StoreConfig::default()
        })
        .await
        .expect("deadpool builds lazily, so construction against a dead host succeeds"),
    )
}

/// The **seed-holding** forger of D-10: a real, correctly-signed v2 bundle whose
/// `source_team` is a team the emitter does not speak for.
///
/// `derive_team_signing_seed` works for ANY `(region, team)`, so this bundle
/// verifies under the claimed pair. The shipped relabel negative
/// (`bundle.rs`'s seed-less forger) structurally cannot produce it, which is
/// exactly why AC5's derived-key leg may not be cited as covering this attacker.
fn seed_signed_bundle_claiming(team: &str, seed: &[u8; 32]) -> CrossRegionReplicationBundle {
    let region = Region::canonicalize("region-a").expect("canonical region");
    let claimed = TeamId::new(team).expect("canonical team");
    let bundle = build_replication_bundle_v2(
        vec![crossing_leaf("forged-crossing", "impersonated")],
        &region,
        &claimed,
        seed,
    )
    .expect("a seed holder can build a bundle under any team");
    verify_replication_bundle(&bundle, seed)
        .expect("D-10: the seed-holding forger's signature is VALID — that is the whole problem");
    bundle
}

fn control_from() -> FrameAddress {
    FrameAddress {
        spirit_id: SpiritId::from("cohort-control"),
        host_id: Some(HostId("host-a".to_string())),
        role: None,
    }
}

/// Story 13.6b / AC3 (D-13) — **the headline.**
///
/// 13.6a authenticates the ENVELOPE (`request.cohort_source_team`, refused under
/// `-32010`). The crossing decides consent from the PAYLOAD
/// (`bundle.source_team` → `is_granted`). Nothing bound them, so a host could
/// stamp a truthful envelope and sign a lying payload and land a row under an
/// impersonated team with transport ✓, `-32010` ✓, signature ✓, and consent ✓.
///
/// ⚠ 13.6a's impersonation leg may NOT be cited as evidence for this: it proves
/// the envelope binding against synthetic frames at a different site. This leg
/// runs the attack through the real applier with a real seed-signed payload.
#[tokio::test]
async fn crossing_weld_refuses_a_forged_payload_team_before_apply() {
    let seed = [0x42u8; 32];
    let forged = seed_signed_bundle_claiming("team-b", &seed);
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-c".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        forged,
    )
    .expect("crossing frame encodes");
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-c").await,
        TeamId::new("team-c").expect("canonical team"),
        seed,
    );

    // The envelope the mesh authenticated says team-a. The payload says team-b.
    match adapter.apply_crossing("team-a", &frame).await {
        CrossingOutcome::Refused(CrossingRefusal::SourceTeamUnbound {
            envelope_team,
            payload_team,
        }) => {
            assert_eq!(envelope_team, "team-a");
            assert_eq!(payload_team, "team-b");
        }
        other => panic!(
            "AC3: an authenticated envelope with a forged payload team must be refused under \
             its OWN cause before any apply; got {other:?}"
        ),
    }
}

/// Story 13.6b / AC3 — the weld is a BINDING, not a refuse-everything stub.
///
/// Without this control the leg above would pass against an applier that refuses
/// every crossing, which would satisfy the negative while silently deleting the
/// feature. Here the payload team EQUALS the authenticated envelope team, so the
/// weld must let the crossing through into `apply_replication_bundle`.
///
/// The expected outcome is `StateUnavailable` ("no cross-team consent port is
/// configured"), and that is a **precise passage witness**: inside
/// `apply_replication_bundle` the consent-port lookup sits downstream of the
/// signature verify, the principal-namespace refusal, the destination-region
/// binding, the self-crossing check, the destination-team match, AND the
/// leaf/envelope identity check. Observing it proves the crossing cleared every
/// one of those, so the weld admitted rather than short-circuited.
#[tokio::test]
async fn crossing_weld_admits_the_authenticated_team_and_proceeds_to_apply() {
    let seed = [0x42u8; 32];
    let bundle = seed_signed_bundle_claiming("team-b", &seed);
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-c".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        bundle,
    )
    .expect("crossing frame encodes");
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-c").await,
        TeamId::new("team-c").expect("canonical team"),
        seed,
    );

    match adapter.apply_crossing("team-b", &frame).await {
        CrossingOutcome::Refused(CrossingRefusal::SourceTeamUnbound { .. }) => panic!(
            "AC3: the weld refused a payload team that MATCHES the authenticated envelope — \
             this applier is a refuse-everything stub, not a binding"
        ),
        CrossingOutcome::Refused(CrossingRefusal::StateUnavailable { reason, .. }) => {
            assert!(
                reason.contains("consent port"),
                "the matched crossing must reach the consent-port lookup deep inside \
                 apply_replication_bundle; got {reason}"
            );
        }
        other => panic!(
            "expected the matched crossing to reach apply_replication_bundle's consent lookup; \
             got {other:?}"
        ),
    }
}

/// Story 13.6b / AC1 — a non-crossing frame is not the applier's business.
///
/// The port is consulted on the shared intake path, so a frame that is not a
/// crossing must classify as `NotCrossing` and leave the unchanged path alone.
/// Guards against a fail-closed applier breaking every other cohort intent.
#[tokio::test]
async fn crossing_applier_ignores_frames_that_are_not_crossings() {
    let seed = [0x42u8; 32];
    let mut frame = crossing_frame(
        &control_from(),
        &HostId("host-c".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        seed_signed_bundle_claiming("team-b", &seed),
    )
    .expect("crossing frame encodes");
    // The case the intent gate actually guards: a WELL-FORMED envelope carrying a
    // DIFFERENT intent. (The first draft blanked the envelope entirely, which
    // short-circuits one check earlier — the proven-red pass caught that the leg
    // survived deleting the intent gate.)
    frame.consent_envelope = Some(
        maos_domain::frame::ConsentEnvelope::with_fine_grained_intent(
            control_from(),
            maos_domain::invariants::i8::A2AIntent::new("cohort:digest-read"),
        ),
    );
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-c").await,
        TeamId::new("team-c").expect("canonical team"),
        seed,
    );
    assert_eq!(
        adapter.apply_crossing("team-b", &frame).await,
        CrossingOutcome::NotCrossing,
        "a frame carrying another cohort intent must fall through to the unchanged intake path"
    );
}

#[tokio::test]
async fn crossing_applier_rejects_a_mismatched_frame_kind() {
    let seed = [0x42u8; 32];
    let mut frame = crossing_frame(
        &control_from(),
        &HostId("host-c".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        seed_signed_bundle_claiming("team-b", &seed),
    )
    .expect("crossing frame encodes");
    frame.kind = maos_spirit_abi::identity::FrameKind::TaskAssign;
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-c").await,
        TeamId::new("team-c").expect("canonical team"),
        seed,
    );
    match adapter.apply_crossing("team-b", &frame).await {
        CrossingOutcome::Refused(CrossingRefusal::ApplyFailed { reason, .. }) => {
            assert!(
                reason.contains("frame kind"),
                "unexpected refusal: {reason}"
            );
        }
        other => panic!("a type-confused crossing frame must be refused; got {other:?}"),
    }
}

/// Story 13.6b / AC1 — the crossing body round-trips through the shipped
/// telemetry-control idiom, so no `FramePayload` or `FrameKind` variant is added
/// (`abi-diff` is a null control that cannot see an enum addition).
#[test]
fn crossing_control_round_trips_through_the_telemetry_idiom() {
    let seed = [0x42u8; 32];
    let bundle = seed_signed_bundle_claiming("team-b", &seed);
    let root = bundle.root;
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-c".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        bundle,
    )
    .expect("crossing frame encodes");
    assert_eq!(
        frame.kind,
        maos_spirit_abi::identity::FrameKind::TelemetryEvent,
        "the crossing must ride the existing FrameKind, never a new variant"
    );
    // GOLDEN, not self-referential: a round-trip through `CROSSING_EVENT_TYPE` on
    // both sides passes even when the constant drifts, which silently stops every
    // deployed applier from recognising a crossing. The proven-red pass caught
    // exactly that, so the literal is pinned here. Changing it is a wire-format
    // change and must be treated as one.
    let maos_domain::frame::FramePayload::TelemetryEvent(payload) = &frame.payload else {
        panic!("the crossing must ride FramePayload::TelemetryEvent");
    };
    assert_eq!(
        payload.event_type, "maos.cross-team-crossing.v1",
        "CROSSING_EVENT_TYPE is a pinned wire constant"
    );
    let decoded = CrossTeamCrossingControl::from_frame(&frame)
        .expect("a crossing frame is recognised")
        .expect("and decodes");
    let CrossTeamCrossingControl::Share { to_team, bundle } = decoded;
    assert_eq!(to_team, "team-c");
    assert_eq!(bundle.root, root, "the signed bytes must survive the wire");
    verify_replication_bundle(&bundle, &seed)
        .expect("the decoded bundle must still verify — the wire must not re-sign anything");
}

// ─── AC4 — one host, one team ───────────────────────────────────────────────

#[derive(Default)]
struct FixedClock;

impl CohortClock for FixedClock {
    fn now_secs(&self) -> u64 {
        0
    }
}

fn manifest_with(schema_version: u64, host_a_team: Option<&str>) -> CohortManifest {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    CohortManifest {
        schema_version,
        cohort_id: "cross-team-13-6b".to_string(),
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
                team: host_a_team.map(|team| TeamId::new(team).expect("canonical team")),
            },
            CohortMember {
                host_id: "host-b".to_string(),
                fingerprint: format!("sha256:{}", "cd".repeat(32)),
                roles: vec!["worker".to_string()],
                team: Some(TeamId::new("team-b").expect("canonical team")),
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
                team_id: TeamId::new("team-a").expect("canonical team"),
                region: Region::canonicalize("region-a").expect("canonical region"),
                datname: "maos_team_a".to_string(),
                members: vec![SpiritId::from("spirit-a")],
            },
            TeamEntry {
                team_id: TeamId::new("team-b").expect("canonical team"),
                region: Region::canonicalize("region-a").expect("canonical region"),
                datname: "maos_team_b".to_string(),
                members: vec![SpiritId::from("spirit-b")],
            },
        ]),
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: vec![CrossTeamConsentGrant {
            from_team: TeamId::new("team-a").expect("canonical team"),
            to_team: TeamId::new("team-b").expect("canonical team"),
            intent: "collective:share".to_string(),
        }],
    }
}

fn verified_manifest_state() -> Arc<CohortManifestState> {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    let manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a")).signed_with(&signing_key);
    let signed_toml = toml::to_string(&manifest).expect("signed manifest serializes");
    let pins = PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()])
        .expect("pinned authority key");
    Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host-b".to_string()),
            &signed_toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(FixedClock),
        )
        .expect("verified manifest state"),
    )
}

async fn dead_store_with_consent() -> Arc<LoomLiteStore> {
    let state = verified_manifest_state();
    Arc::new(
        LoomLiteStore::new(StoreConfig {
            connection_string:
                "host=127.0.0.1 port=1 user=nobody dbname=maos_absent connect_timeout=1"
                    .to_string(),
            home_region: "region-a".to_string(),
            home_team: "team-b".to_string(),
            ..StoreConfig::default()
        })
        .await
        .expect("deadpool builds lazily")
        .with_cross_team_consent(Arc::new(
            maos_bin::cross_team_consent::CrossTeamConsentAdapter::new(state),
        )),
    )
}

#[tokio::test]
async fn crossing_applier_binds_the_requested_destination_team() {
    let seed = [0x42u8; 32];
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-b".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        seed_signed_bundle_claiming("team-a", &seed),
    )
    .expect("crossing frame encodes");
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-b").await,
        TeamId::new("team-b").expect("canonical team"),
        seed,
    );
    match adapter.apply_crossing("team-a", &frame).await {
        CrossingOutcome::Refused(CrossingRefusal::ApplyFailed {
            reason,
            from_team,
            to_team,
            ..
        }) => {
            assert!(reason.contains("destination team mismatch"));
            assert_eq!(from_team, "team-a");
            assert_eq!(to_team, "team-c");
        }
        other => panic!("a request addressed to another team must be refused; got {other:?}"),
    }
}

#[tokio::test]
async fn unconsented_crossing_is_refused_at_the_destination_applier() {
    let seed = [0x42u8; 32];
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-b".to_string()),
        1,
        &TeamId::new("team-b").expect("canonical team"),
        seed_signed_bundle_claiming("team-c", &seed),
    )
    .expect("crossing frame encodes");
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store_with_consent().await,
        TeamId::new("team-b").expect("canonical team"),
        seed,
    );
    assert!(matches!(
        adapter.apply_crossing("team-c", &frame).await,
        CrossingOutcome::Refused(CrossingRefusal::ConsentDenied {
            ref from_team,
            ref to_team,
            ..
        }) if from_team == "team-c" && to_team == "team-b"
    ));
}

#[tokio::test]
async fn seedless_source_team_relabel_is_refused_at_the_destination_applier() {
    let seed = [0x42u8; 32];
    let mut relabelled = seed_signed_bundle_claiming("team-a", &seed);
    let forged_team = TeamId::new("team-c").expect("canonical team");
    relabelled.source_team = Some(forged_team.clone());
    for leaf in &mut relabelled.leaves {
        leaf.source_team = Some(forged_team.clone());
    }
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-b".to_string()),
        1,
        &TeamId::new("team-b").expect("canonical team"),
        relabelled,
    )
    .expect("crossing frame encodes");
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-b").await,
        TeamId::new("team-b").expect("canonical team"),
        seed,
    );
    match adapter.apply_crossing("team-c", &frame).await {
        CrossingOutcome::Refused(CrossingRefusal::ApplyFailed { reason, .. }) => {
            assert!(
                reason.contains("signature verification failed"),
                "seedless relabel must fail at derived-key verification; got {reason}"
            );
        }
        other => panic!("a seedless source-team relabel must be refused; got {other:?}"),
    }
}

/// Story 13.6b / AC4 — a host whose environment and signed manifest disagree
/// about its own team does not start.
///
/// This follows `reconcile_transport_identity_with_manifest`'s own shipped
/// doctrine — *"a config-time fact silently overrides a manifest-time fact …
/// Disagreement is a boot error, never a warning"* — which 13.6a's review wrote
/// for certificates and left unwritten for teams, one field over.
///
/// ⚠ This is the correctness control against misconfiguration. It is NOT the
/// security control against a peer: an attacker owns their own boot and will set
/// the environment correctly and lie in the payload. That attacker is refused by
/// the AC3 weld, and only by it.
#[test]
fn boot_refuses_a_home_team_that_disagrees_with_the_signed_manifest() {
    let manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    // Agreement boots.
    reconcile_home_team_with_manifest(&manifest, "host-a", "team-a")
        .expect("matching env and manifest must boot");
    // An unset/empty override is not a disagreement.
    reconcile_home_team_with_manifest(&manifest, "host-a", "   ")
        .expect("an empty override is not a disagreement");
    // Disagreement is a boot error, and it names both surfaces.
    let error = reconcile_home_team_with_manifest(&manifest, "host-a", "team-b")
        .expect_err("AC4: disagreement must refuse the boot");
    assert!(
        error.contains("team-b") && error.contains("team-a"),
        "the boot error must name the env value AND the signed value; got {error}"
    );
}

/// Story 13.6b / AC4 — absence never permits.
///
/// A pre-V4 manifest, or a V4 member declaring no team, gives `team_of_host`
/// `None`. `CohortManifest::team_of_host` is documented fail-closed by
/// construction, and this leg holds the boot path to that contract: a host that
/// asserts a team the signed manifest cannot corroborate must refuse to start
/// rather than emit crossings attributed to an unverifiable team.
#[test]
fn boot_refuses_a_home_team_the_manifest_cannot_corroborate() {
    for manifest in [
        manifest_with(COHORT_SCHEMA_V4, None),
        manifest_with(COHORT_SCHEMA_V3, Some("team-a")),
    ] {
        let error = reconcile_home_team_with_manifest(&manifest, "host-a", "team-a")
            .expect_err("AC4: an uncorroborated team claim must refuse the boot");
        assert!(
            error.contains("NO team"),
            "the refusal must say the manifest declares no team; got {error}"
        );
    }
}

// ─── Live two-datname, two-daemon integration (AC1/AC2/AC5) ─────────────────
//
// `AdvisorySubstrate`: these legs `.expect()` their own env vars rather than
// silently skipping (the 13.5g pattern — a skipped leg that prints green is the
// failure this project keeps catching). The gate controls execution.

static LIVE_LOCK: Mutex<()> = Mutex::new(());

fn pg_conn_team(team: &str) -> String {
    let var = match team {
        "team-a" => "MAOS_TEST_POSTGRES_TEAM_A",
        "team-b" => "MAOS_TEST_POSTGRES_TEAM_B",
        other => panic!("unknown team {other}"),
    };
    std::env::var(var)
        .unwrap_or_else(|_| panic!("{var} must be set for the live two-datname crossing legs"))
}

/// Raw client for one team's physical database. This oracle intentionally
/// bypasses the Spirit-facing store guard only after the two real daemons have
/// performed the crossing.
async fn raw_connect_team(team: &str) -> tokio_postgres::Client {
    let connection_string = pg_conn_team(team);
    let (client, connection) = tokio_postgres::connect(&connection_string, NoTls)
        .await
        .expect("raw Postgres connection");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    client
}

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

const DAEMON_LISTENING_MARKER: &str = "cohort-a2a-daemon listening on ";
const DAEMON_LISTEN_TIMEOUT: Duration = Duration::from_secs(90);

struct DaemonIdentity {
    cert: PathBuf,
    private_key: PathBuf,
    fingerprint: PeerCertFingerprint,
}

fn mint_daemon_identity(dir: &Path, host: &str) -> DaemonIdentity {
    let key = rcgen::KeyPair::generate().expect("rcgen keypair");
    let params =
        rcgen::CertificateParams::new(vec!["127.0.0.1".to_string()]).expect("certificate params");
    let cert = params.self_signed(&key).expect("self-signed certificate");
    let cert_path = dir.join(format!("{host}.cert.pem"));
    let key_path = dir.join(format!("{host}.key.pem"));
    std::fs::write(&cert_path, cert.pem()).expect("write certificate");
    std::fs::write(&key_path, key.serialize_pem()).expect("write private key");
    DaemonIdentity {
        cert: cert_path,
        private_key: key_path,
        fingerprint: PeerCertFingerprint::from_cert_der(cert.der().as_ref()),
    }
}

fn write_daemon_manifest(
    dir: &Path,
    identity_a: &DaemonIdentity,
    identity_b: &DaemonIdentity,
) -> (PathBuf, SigningKey) {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    let mut manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    manifest.members[0].fingerprint = identity_a.fingerprint.to_string();
    manifest.members[1].fingerprint = identity_b.fingerprint.to_string();
    manifest.consent = ConsentMatrix {
        send: vec![ConsentTuple {
            peer: "host-b".to_string(),
            role: "worker".to_string(),
            intent: maos_a2a_core::COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
        }],
        accept: vec![ConsentTuple {
            peer: "host-a".to_string(),
            role: "worker".to_string(),
            intent: maos_a2a_core::COHORT_INTENT_COLLECTIVE_SHARE.to_string(),
        }],
    };
    let signed = manifest.signed_with(&signing_key);
    let path = dir.join("manifest.toml");
    std::fs::write(
        &path,
        toml::to_string(&signed).expect("signed manifest serializes"),
    )
    .expect("write signed manifest");
    (path, signing_key)
}

#[derive(serde::Serialize)]
struct DaemonFileConfig {
    tcp: maos_a2a_tcp::TcpA2AConfig,
    peers: Vec<A2APeerConfig>,
    manifest_path: PathBuf,
    authority_keys: Vec<String>,
    local_host: String,
    control_spirit: String,
    digest_summary: maos_cohort::DigestSummary,
}

#[allow(clippy::too_many_arguments)]
fn write_daemon_file(
    dir: &Path,
    tag: &str,
    manifest_path: &Path,
    authority: &SigningKey,
    identity: &DaemonIdentity,
    local_host: &str,
    control_spirit: &str,
    peer_identity: &DaemonIdentity,
    peer_host: &str,
    peer_endpoint: String,
    peer_boot_nonce: u64,
) -> PathBuf {
    let intent = A2AIntent::new(maos_a2a_core::COHORT_INTENT_COLLECTIVE_SHARE);
    let file = DaemonFileConfig {
        tcp: maos_a2a_tcp::TcpA2AConfig {
            listen_addr: "127.0.0.1:0".parse().expect("loopback listen address"),
            own_cert_chain: identity.cert.clone(),
            own_private_key: identity.private_key.clone(),
            peer_pins: vec![maos_a2a_tcp::config::PinnedFingerprint {
                peer_id: PeerId::new(peer_host),
                fingerprint: peer_identity.fingerprint.clone(),
                boot_nonce: peer_boot_nonce,
            }],
            handshake_timeout: Duration::from_secs(30),
            ca_roots: None,
        },
        peers: vec![A2APeerConfig {
            peer_id: PeerId::new(peer_host),
            endpoint: peer_endpoint,
            cert_fingerprint: peer_identity.fingerprint.clone(),
            profile: A2AProfile::CrossHost,
            allowlists: ConsentAllowlists {
                send_allowlist: vec![intent.clone()],
                accept_allowlist: vec![intent],
            },
            partition_timeout_secs: 30,
            consent_ttl_secs: maos_a2a_core::config::DEFAULT_CONSENT_TTL_SECS,
        }],
        manifest_path: manifest_path.to_path_buf(),
        authority_keys: vec![hex::encode(authority.verifying_key().to_bytes())],
        local_host: local_host.to_string(),
        control_spirit: control_spirit.to_string(),
        digest_summary: maos_cohort::DigestSummary::default(),
    };
    let path = dir.join(format!("{tag}.daemon.toml"));
    std::fs::write(
        &path,
        toml::to_string(&file).expect("daemon config serializes"),
    )
    .expect("write daemon config");
    path
}

struct RunningDaemon(Child);

impl RunningDaemon {
    fn exited(&mut self) -> Option<std::process::ExitStatus> {
        self.0.try_wait().expect("inspect daemon")
    }
}

impl Drop for RunningDaemon {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn boot_daemon(mut command: Command) -> Result<(RunningDaemon, u16), String> {
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn daemon: {error}"))?;
    let stderr = child.stderr.take().ok_or("daemon stderr is not piped")?;
    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        let mut lines = BufReader::new(stderr);
        let mut line = String::new();
        while lines.read_line(&mut line).unwrap_or_default() != 0 {
            if tx.send(line.clone()).is_err() {
                break;
            }
            line.clear();
        }
    });
    let deadline = Instant::now() + DAEMON_LISTEN_TIMEOUT;
    let mut seen = String::new();
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                seen.push_str(&line);
                if let Some(port) = line
                    .split_once(DAEMON_LISTENING_MARKER)
                    .and_then(|(_, address)| address.trim().rsplit(':').next())
                    .and_then(|port| port.parse::<u16>().ok())
                {
                    return Ok((RunningDaemon(child), port));
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some(status) = child
                    .try_wait()
                    .map_err(|error| format!("inspect daemon: {error}"))?
                {
                    return Err(format!(
                        "daemon exited before listening ({status}); stderr:\n{seen}"
                    ));
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(format!("daemon stderr closed before listening:\n{seen}"));
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    Err(format!(
        "daemon did not listen within {DAEMON_LISTEN_TIMEOUT:?}; stderr:\n{seen}"
    ))
}

fn daemon_command(
    dir: &Path,
    tag: &str,
    config: &Path,
    postgres: &str,
    home_team: &str,
    boot_nonce: u64,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", config)
        .env("MAOS_LOOM_POSTGRES", postgres)
        .env("MAOS_LOOM_HOME_TEAM", home_team)
        .env("MAOS_REGION_HOME", "region-a")
        .env("MAOS_CROSS_TEAM_BASE_SEED", "42".repeat(32))
        .env("MAOS_TEST_BOOT_NONCE", boot_nonce.to_string())
        .env(
            "MAOS_AUDIT_DB",
            dir.join(format!("{tag}.transparency.sqlite")),
        )
        .env("MAOS_OLLAMA_URL", "skip")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
}

#[tokio::test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B (live Postgres)"]
async fn live_crossing_runs_through_two_daemon_processes() {
    let _evidence = evidence_record::attest("live_crossing_runs_through_two_daemon_processes");
    let _guard = LIVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let team_a_conn = pg_conn_team("team-a");
    let team_b_conn = pg_conn_team("team-b");
    assert_ne!(datname_of(&team_a_conn), datname_of(&team_b_conn));
    let raw_source = raw_connect_team("team-a").await;
    let raw_destination = raw_connect_team("team-b").await;

    let fixture = tempfile::tempdir().expect("two-daemon fixture");
    let identity_a = mint_daemon_identity(fixture.path(), "host-a");
    let identity_b = mint_daemon_identity(fixture.path(), "host-b");
    let (manifest_path, authority) =
        write_daemon_manifest(fixture.path(), &identity_a, &identity_b);
    const NONCE_A: u64 = 13_600_001;
    const NONCE_B: u64 = 13_600_002;

    let config_b = write_daemon_file(
        fixture.path(),
        "team-b",
        &manifest_path,
        &authority,
        &identity_b,
        "host-b",
        "spirit-b",
        &identity_a,
        "host-a",
        "tls://127.0.0.1:1".to_string(),
        NONCE_A,
    );
    let (mut daemon_b, port_b) = boot_daemon(daemon_command(
        fixture.path(),
        "team-b",
        &config_b,
        &team_b_conn,
        "team-b",
        NONCE_B,
    ))
    .unwrap_or_else(|error| panic!("team-b daemon failed: {error}"));

    let config_a = write_daemon_file(
        fixture.path(),
        "team-a",
        &manifest_path,
        &authority,
        &identity_a,
        "host-a",
        "spirit-a",
        &identity_b,
        "host-b",
        format!("tls://127.0.0.1:{port_b}"),
        NONCE_B,
    );
    let key_nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let key = format!("daemon-crossing-{}-{key_nonce}", std::process::id());
    let value = "crossed-through-both-daemons";
    let mut command_a = daemon_command(
        fixture.path(),
        "team-a",
        &config_a,
        &team_a_conn,
        "team-a",
        NONCE_A,
    );
    command_a
        .env("MAOS_CROSS_TEAM_SHARE_PEER", "host-b")
        .env("MAOS_CROSS_TEAM_SHARE_TO_TEAM", "team-b")
        .env("MAOS_CROSS_TEAM_SHARE_PID", "7")
        .env("MAOS_CROSS_TEAM_SHARE_NAMESPACE", "default")
        .env("MAOS_CROSS_TEAM_SHARE_KEY", &key)
        .env("MAOS_CROSS_TEAM_SHARE_VALUE", value);
    let (mut daemon_a, _port_a) =
        boot_daemon(command_a).unwrap_or_else(|error| panic!("team-a daemon failed: {error}"));

    let is_expected_row = |row: &CollectiveRow| {
        row.spirit_pid == 7
            && row.namespace_kind == "default"
            && row.namespace_detail == "xteam:team-a:"
            && row.key == key
            && row.value_kind == "text"
            && row.value_data == value.as_bytes()
            && row.source_region == "region-a"
            && row.source_team.as_ref().map(TeamId::as_str) == Some("team-a")
            && row.distillation_depth == Some(1)
            && row.intent_lineage.is_some()
    };
    let has_re_attestation = |row: &CollectiveRow| {
        let Ok(marker) = serde_json::from_str::<serde_json::Value>(&row.source_log_ref) else {
            return false;
        };
        marker.as_object().is_some_and(|object| object.len() == 2)
            && marker
                .get("source_region")
                .and_then(serde_json::Value::as_str)
                == Some("region-a")
            && marker
                .get("merkle_root")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|root| {
                    root.len() == 64 && root.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let destination_rows = LoomLiteStore::read_all_rows_from(&raw_destination)
            .await
            .expect("read physical destination rows");
        let destination_matches = destination_rows
            .iter()
            .filter(|row| is_expected_row(row) && has_re_attestation(row))
            .count();
        if destination_matches == 1 {
            break;
        }
        assert!(
            daemon_a.exited().is_none(),
            "the emitter daemon exited before the crossing landed"
        );
        assert!(
            daemon_b.exited().is_none(),
            "the applier daemon exited before the crossing landed"
        );
        assert!(
            Instant::now() < deadline,
            "the two-daemon crossing did not land within 30 seconds"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let destination_rows = LoomLiteStore::read_all_rows_from(&raw_destination)
        .await
        .expect("read physical destination rows");
    assert_eq!(
        destination_rows
            .iter()
            .filter(|row| is_expected_row(row) && has_re_attestation(row))
            .count(),
        1,
        "exactly the emitted value must land in team B's physical database"
    );
    let source_rows = LoomLiteStore::read_all_rows_from(&raw_source)
        .await
        .expect("read physical source rows");
    assert_eq!(
        source_rows
            .iter()
            .filter(|row| is_expected_row(row) && row.source_log_ref.is_empty())
            .count(),
        1,
        "originate_team_row must persist exactly the attested origin row before transport"
    );
}

/// Supplemental live-store coverage for the destination adapter's apply and
/// refusal branches. The separately-gated two-daemon witness above owns the
/// production emitter, transport, verified-intake, and composition-root claim;
/// this test deliberately isolates the adapter against two physical databases.
///
/// Asserts a granted apply, an applier-side consent denial, and the
/// envelope/payload weld against a seed-holding forger.
#[tokio::test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B (live Postgres)"]
async fn live_destination_adapter_applies_and_refuses_expected_shapes() {
    let _g = LIVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let seed = [0x42u8; 32];
    let clock = Arc::new(FixedClock);
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    let mut manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    manifest.teams = Some(vec![
        TeamEntry {
            team_id: TeamId::new("team-a").expect("canonical team"),
            region: Region::canonicalize("region-a").expect("canonical region"),
            datname: datname_of(&pg_conn_team("team-a")),
            members: vec![SpiritId::from("spirit-a")],
        },
        TeamEntry {
            team_id: TeamId::new("team-b").expect("canonical team"),
            region: Region::canonicalize("region-a").expect("canonical region"),
            datname: datname_of(&pg_conn_team("team-b")),
            members: vec![SpiritId::from("spirit-b")],
        },
    ]);
    let signed = manifest.signed_with(&signing_key);
    let signed_toml = toml::to_string(&signed).expect("signed manifest serializes");
    let pins = maos_cohort::PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()])
        .expect("pinned authority keys");
    let state = Arc::new(
        maos_cohort::CohortManifestState::load_with_clock(
            HostId("host-b".to_string()),
            &signed_toml,
            pins,
            Arc::new(maos_cohort::InMemoryCohortAuditSink::default()),
            clock,
        )
        .expect("verified manifest state"),
    );

    let verifying_keys =
        maos_bin::cross_team_consent::derive_team_verifying_keys(&state, &seed).expect("team keys");
    let store_b = Arc::new(
        LoomLiteStore::new(StoreConfig {
            connection_string: pg_conn_team("team-b"),
            home_region: "region-a".to_string(),
            home_team: "team-b".to_string(),
            ..StoreConfig::default()
        })
        .await
        .expect("team-b store")
        .with_cross_team_consent(Arc::new(
            maos_bin::cross_team_consent::CrossTeamConsentAdapter::new(Arc::clone(&state)),
        ))
        .with_team_verifying_keys(verifying_keys),
    );
    store_b.init_schema().await.expect("team-b schema");

    let team_b = TeamId::new("team-b").expect("canonical team");
    let applier = CrossTeamCrossingAdapter::new(Arc::clone(&store_b), team_b.clone(), seed);

    // (1) The granted A→B crossing lands.
    let bundle = build_replication_bundle_v2(
        vec![crossing_leaf("live-crossing", "granted")],
        &Region::canonicalize("region-a").expect("canonical region"),
        &TeamId::new("team-a").expect("canonical team"),
        &seed,
    )
    .expect("first-party promotion builds");
    let frame = crossing_frame(
        &control_from(),
        &HostId("host-b".to_string()),
        1,
        &team_b,
        bundle,
    )
    .expect("crossing frame encodes");
    match applier.apply_crossing("team-a", &frame).await {
        CrossingOutcome::Applied { applied_count } => assert_eq!(applied_count, 1),
        other => panic!("AC1: the granted crossing must land; got {other:?}"),
    }
    assert_eq!(
        store_b
            .read(
                7,
                &maos_domain::memory::MemoryNamespace::Default,
                "live-crossing"
            )
            .await
            .expect("read team-b"),
        Some(maos_domain::memory::MemoryValue::Text(
            "granted".to_string()
        )),
        "AC1: the row must be physically present in maos_team_b"
    );

    // Physical absence at the SOURCE datname — a shared table would fail this.
    let store_a = LoomLiteStore::new(StoreConfig {
        connection_string: pg_conn_team("team-a"),
        home_region: "region-a".to_string(),
        home_team: "team-a".to_string(),
        ..StoreConfig::default()
    })
    .await
    .expect("team-a store");
    store_a.init_schema().await.expect("team-a schema");
    assert_eq!(
        store_a
            .read(
                7,
                &maos_domain::memory::MemoryNamespace::Default,
                "live-crossing"
            )
            .await
            .expect("read team-a"),
        None,
        "AC1: the destination row must be physically ABSENT from maos_team_a"
    );

    // (2) The reverse crossing is refused AT THE APPLIER with the ordered pair.
    let reverse = build_replication_bundle_v2(
        vec![crossing_leaf("reverse-must-not-land", "denied")],
        &Region::canonicalize("region-a").expect("canonical region"),
        &TeamId::new("team-c").expect("canonical team"),
        &seed,
    )
    .expect("first-party promotion builds");
    let reverse_frame = crossing_frame(
        &control_from(),
        &HostId("host-b".to_string()),
        2,
        &team_b,
        reverse,
    )
    .expect("crossing frame encodes");
    match applier.apply_crossing("team-c", &reverse_frame).await {
        CrossingOutcome::Refused(CrossingRefusal::ConsentDenied {
            from_team,
            to_team,
            intent,
        }) => {
            assert_eq!(from_team, "team-c");
            assert_eq!(to_team, "team-b");
            assert_eq!(intent, "collective:share");
        }
        other => panic!(
            "AC2/AC5: an unconsented crossing must be refused AT THE DESTINATION APPLIER \
             carrying the ordered pair, not merely un-initiated at the source; got {other:?}"
        ),
    }

    // (3) The seed-holding forger (AC3) — refused even against a live store.
    let forged_frame = crossing_frame(
        &control_from(),
        &HostId("host-b".to_string()),
        3,
        &team_b,
        seed_signed_bundle_claiming("team-a", &seed),
    )
    .expect("crossing frame encodes");
    match applier.apply_crossing("team-c", &forged_frame).await {
        CrossingOutcome::Refused(CrossingRefusal::SourceTeamUnbound {
            envelope_team,
            payload_team,
        }) => {
            assert_eq!(envelope_team, "team-c");
            assert_eq!(payload_team, "team-a");
        }
        other => panic!("AC3: the seed-holding forger must be refused; got {other:?}"),
    }
    assert_eq!(
        store_b
            .read(
                7,
                &maos_domain::memory::MemoryNamespace::Default,
                "forged-crossing"
            )
            .await
            .expect("read team-b"),
        None,
        "AC3: an impersonated crossing must leave no row behind"
    );
}
