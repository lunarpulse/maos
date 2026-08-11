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
    crossing_frame, crossing_frame_with_binding, erase_frame_with_binding,
    reconcile_home_team_with_manifest, CrossTeamCrossingAdapter, CrossTeamCrossingControl,
    CrossTeamShareRequest,
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
use maos_loom_lite::tenant::TenantMapPort;
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
/// `item_body`, but a MISSING declaration is a finding rather than a panic:
/// the per-limb falsification below unwires a site by renaming its needle, and
/// a renamed declaration must red the site it belongs to, not abort the test.
fn item_body_opt<'a>(src: &'a str, signature: &str) -> Option<&'a str> {
    src.contains(signature).then(|| item_body(src, signature))
}

/// Story 13.6 (AC2/T3) — the SEVEN wiring sites, as a pure oracle.
///
/// Extracted from the leg below so each site can be falsified per-limb against
/// an in-memory clone: delete one needle, assert the problem that names that
/// exact site appears, and let the clone go out of scope. Restore is by
/// construction — nothing on disk is mutated, so there is no restore step to
/// get wrong (the drift gate's idiom).
///
/// The previous draft of this leg listed six sites. The seventh — the applier
/// PORT CONSTRUCTION at `main.rs:9460-9481` — was omitted: delete it and the
/// runtime receives `None`, `handle_intake_verified` classifies every crossing
/// frame as `CrossingOutcome::NotCrossing`, and `router.rs:1639-1646` NACKs
/// `StateUnavailable`. The wire is dead and every text scan stays green.
fn crossing_chain_problems(main_rs: &str, crossing_rs: &str, router_rs: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let mut require = |present: bool, site: &str, why: &str| {
        if !present {
            problems.push(format!("site {site}: {why}"));
        }
    };

    // ── EMITTER: dispatch → daemon → emit → 13.3b seam → the one outbound path.
    let dispatch = item_body_opt(main_rs, r#"if mode == "cohort-a2a-daemon""#).unwrap_or("");
    require(
        dispatch.contains("run_cohort_a2a_daemon("),
        "1/dispatch-arm",
        "D-14: the daemon must still be reachable from the MAOS_ONE_SHOT dispatch",
    );
    let daemon = item_body_opt(main_rs, "async fn run_cohort_a2a_daemon(").unwrap_or("");
    require(
        daemon.contains("emit_cross_team_share("),
        "2/daemon-calls-emitter",
        "AC1/AC5 (13.5g): the emitter must be reachable FROM INSIDE the daemon runtime — a \
         crossing builder that merely exists in the file is dead wire",
    );
    let emitter = item_body_opt(main_rs, "async fn emit_cross_team_share(").unwrap_or("");
    require(
        emitter.contains("originate_team_row("),
        "3/emitter-uses-13-3b-seam",
        "D-6: the emitter must use the seam 13.3b left, not hand-roll leaf construction",
    );
    require(
        emitter.contains("route_outbound("),
        "4/emitter-uses-only-outbound-path",
        "D-14: the crossing must leave through the ONLY production outbound A2A path, so \
         `prepare_outbound` stamps cohort_source_team from the SIGNED declaration",
    );

    // ── APPLIER: the spoof-proof intake site → the port → apply → is_granted.
    let intake = item_body_opt(router_rs, "pub async fn handle_intake_verified(").unwrap_or("");
    require(
        intake.contains("apply_crossing("),
        "5/intake-calls-applier",
        "D-8: the applier must hang off handle_intake_verified (12.3 P5r), never handle_intake",
    );
    require(
        crossing_rs.contains("apply_replication_bundle("),
        "6/applier-reaches-apply",
        "AC1: the applier must reach apply_replication_bundle — D-1's is_granted call site gets \
         its first non-test caller through it",
    );

    // ── SITE 7: the applier PORT, constructed from this process's one store and
    // handed to the runtime before the accept loop spawns.
    require(
        daemon.contains("CrossTeamCrossingAdapter::new("),
        "7a/port-constructed",
        "AC2: the applier port must be CONSTRUCTED inside the daemon runtime — without it the \
         runtime keeps the legacy port and every crossing frame NACKs StateUnavailable",
    );
    // Paren-balanced: the port must appear in the runtime builder's ARGUMENT
    // LIST, not merely somewhere in the same function — dropping the hand-off
    // while keeping `let crossing_port = ...` is exactly the refactor shape
    // that leaves the wire dead.
    let runtime_call_carries_port = daemon
        .find("build_cohort_a2a_daemon_runtime(")
        .map(|start| {
            let tail = &daemon[start..];
            let mut depth = 0usize;
            let mut end = tail.len();
            for (index, character) in tail.char_indices() {
                match character {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = index;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            tail[..end].contains("crossing_port")
        })
        .unwrap_or(false);
    require(
        runtime_call_carries_port,
        "7b/port-installed",
        "AC2: the constructed applier port must be HANDED to the runtime builder before the \
         accept loop spawns, so no inbound connection observes a legacy-applier window",
    );
    problems
}

/// Rename one call inside one item's body, leaving declarations and sibling
/// wiring sites untouched. The meta-test must plant exactly one defect.
fn unwire_in_item(source: &str, signature: &str, needle: &str) -> String {
    let signature_start = source
        .find(signature)
        .unwrap_or_else(|| panic!("declaration `{signature}` not found"));
    let body_start = source[signature_start..]
        .find('{')
        .map(|offset| signature_start + offset)
        .unwrap_or_else(|| panic!("`{signature}` has no body"));
    let body = item_body(source, signature);
    let needle_start = body
        .find(needle)
        .map(|offset| body_start + offset)
        .unwrap_or_else(|| panic!("`{needle}` not found inside `{signature}`"));
    let mut unwired = source.to_string();
    unwired.replace_range(needle_start..needle_start + needle.len(), "__unwired__(");
    unwired
}

#[test]
fn crossing_has_a_production_initiator_at_both_endpoints() {
    let main_rs = read_source("crates/maos-bin/src/main.rs");
    let crossing_rs = read_source("crates/maos-bin/src/cross_team_crossing.rs");
    let router_rs = read_source("crates/maos-a2a-core/src/router.rs");

    let problems = crossing_chain_problems(&main_rs, &crossing_rs, &router_rs);
    assert!(
        problems.is_empty(),
        "the crossing call chain is broken: {problems:?}"
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

/// Story 13.6 (AC2/T3) — per-limb dead-wire falsification, serialized, seven
/// sites, byte-identical restore by construction.
#[test]
fn every_crossing_wiring_site_is_individually_falsifiable() {
    let main_rs = read_source("crates/maos-bin/src/main.rs");
    let crossing_rs = read_source("crates/maos-bin/src/cross_team_crossing.rs");
    let router_rs = read_source("crates/maos-a2a-core/src/router.rs");
    assert!(
        crossing_chain_problems(&main_rs, &crossing_rs, &router_rs).is_empty(),
        "the real tree must be green before any limb is falsified"
    );

    // (site, file under test, containing item, call whose deletion unwires it)
    let sites: [(&str, char, &str, &str); 7] = [
        (
            "1/dispatch-arm",
            'm',
            r#"if mode == "cohort-a2a-daemon""#,
            "run_cohort_a2a_daemon(",
        ),
        (
            "2/daemon-calls-emitter",
            'm',
            "async fn run_cohort_a2a_daemon(",
            "emit_cross_team_share(",
        ),
        (
            "3/emitter-uses-13-3b-seam",
            'm',
            "async fn emit_cross_team_share(",
            "originate_team_row(",
        ),
        (
            "4/emitter-uses-only-outbound-path",
            'm',
            "async fn emit_cross_team_share(",
            "route_outbound(",
        ),
        (
            "5/intake-calls-applier",
            'r',
            "pub async fn handle_intake_verified(",
            "apply_crossing(",
        ),
        (
            "6/applier-reaches-apply",
            'c',
            "async fn apply_crossing(",
            "apply_replication_bundle(",
        ),
        (
            "7a/port-constructed",
            'm',
            "async fn run_cohort_a2a_daemon(",
            "CrossTeamCrossingAdapter::new(",
        ),
    ];
    for (site, file, scope, needle) in sites {
        let (mutated_main, mutated_crossing, mutated_router) = match file {
            'm' => (
                unwire_in_item(&main_rs, scope, needle),
                crossing_rs.clone(),
                router_rs.clone(),
            ),
            'c' => (
                main_rs.clone(),
                unwire_in_item(&crossing_rs, scope, needle),
                router_rs.clone(),
            ),
            _ => (
                main_rs.clone(),
                crossing_rs.clone(),
                unwire_in_item(&router_rs, scope, needle),
            ),
        };
        let problems = crossing_chain_problems(&mutated_main, &mutated_crossing, &mutated_router);
        assert_eq!(
            problems.len(),
            1,
            "unwiring `{needle}` inside `{scope}` must plant exactly one defect: {problems:?}"
        );
        assert!(
            problems[0].starts_with(&format!("site {site}")),
            "deleting `{needle}` must red exactly {site}; got {problems:?}"
        );
    }

    // Site 7b is the INSTALLATION, falsified separately: keep the construction
    // and drop only the hand-off from the daemon runtime builder call.
    let unwired = unwire_in_item(
        &main_rs,
        "async fn run_cohort_a2a_daemon(",
        "        crossing_port,\n",
    );
    let problems = crossing_chain_problems(&unwired, &crossing_rs, &router_rs);
    assert_eq!(
        problems.len(),
        1,
        "dropping the runtime hand-off must plant exactly one defect: {problems:?}"
    );
    assert!(
        problems[0].starts_with("site 7b/"),
        "dropping the runtime hand-off must red 7b; got {problems:?}"
    );

    // Restore is by construction: every mutation lived in a local `String`.
    assert_eq!(main_rs, read_source("crates/maos-bin/src/main.rs"));
    assert_eq!(
        crossing_rs,
        read_source("crates/maos-bin/src/cross_team_crossing.rs")
    );
    assert_eq!(router_rs, read_source("crates/maos-a2a-core/src/router.rs"));
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
#[tokio::test]
async fn crossing_refuses_emitter_host_that_is_not_the_authenticated_frame_host() {
    let seed = [0x42u8; 32];
    let frame = crossing_frame_with_binding(
        &control_from(),
        &HostId("host-c".to_string()),
        1,
        &TeamId::new("team-c").expect("canonical team"),
        "0123456789abcdef0123456789abcdef".to_string(),
        "forged-host".to_string(),
        seed_signed_bundle_claiming("team-a", &seed),
    )
    .expect("crossing frame encodes");
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-c").await,
        TeamId::new("team-c").expect("canonical team"),
        seed,
    );

    assert!(matches!(
        adapter.apply_crossing("team-a", &frame).await,
        CrossingOutcome::Refused(CrossingRefusal::EmitterHostUnbound {
            emitter_host,
            authenticated_host,
            ..
        }) if emitter_host == "forged-host" && authenticated_host == "host-a"
    ));
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
    let CrossTeamCrossingControl::Share { to_team, bundle, .. } = decoded else {
        panic!("a share frame must decode as CrossTeamCrossingControl::Share");
    };
    assert_eq!(to_team, "team-c");
    assert_eq!(bundle.root, root, "the signed bytes must survive the wire");
    verify_replication_bundle(&bundle, &seed)
        .expect("the decoded bundle must still verify — the wire must not re-sign anything");
}

#[test]
fn erase_control_carries_only_the_reconciliation_locator() {
    let from = FrameAddress {
        spirit_id: SpiritId::from("spirit-b"),
        host_id: Some(HostId("host-b".to_string())),
        role: None,
    };
    let peer = HostId("host-a".to_string());
    let frame = erase_frame_with_binding(
        &from,
        &peer,
        1,
        &TeamId::new("team-a").expect("canonical team"),
        7,
        &maos_domain::memory::MemoryNamespace::Default,
        "crossed-key".to_string(),
        "0123456789abcdef0123456789abcdef".to_string(),
        maos_bin::cross_team_crossing::erase_locator_digest(
            "team-a",
            "team-b",
            7,
            "default",
            "crossed-key",
        ),
    )
    .expect("erase control encodes");
    assert_eq!(
        frame.intent,
        maos_domain::invariants::i1::IntentClass::Standard,
        "erase controls must not inherit the share frame's read-only class"
    );
    assert_eq!(
        frame
            .consent_envelope
            .as_ref()
            .and_then(|envelope| envelope.intent_class.as_ref())
            .map(|intent| intent.as_str()),
        Some("collective:erase"),
        "erase controls must carry the destructive fine-grained route intent"
    );
    let decoded = CrossTeamCrossingControl::from_frame(&frame)
        .expect("erase control is recognized")
        .expect("erase control decodes");
    assert!(matches!(
        decoded,
        CrossTeamCrossingControl::Erase {
            to_team,
            spirit_pid: 7,
            namespace,
            key,
            ..
        } if to_team == "team-a" && namespace == "default" && key == "crossed-key"
    ));
}

/// A consent grant authorizes the relationship, not an arbitrary peer-chosen
/// locator. The dead store makes any attempted deletion surface as a pool error;
/// the distinct provenance refusal proves the handler stopped before touching it.
#[tokio::test]
async fn erase_control_without_share_provenance_is_refused_before_store_access() {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    let mut manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
    manifest.cross_team_consent.push(CrossTeamConsentGrant {
        from_team: TeamId::new("team-b").expect("canonical team"),
        to_team: TeamId::new("team-a").expect("canonical team"),
        intent: "collective:erase".to_string(),
    });
    let signed = manifest.signed_with(&signing_key);
    let signed_toml = toml::to_string(&signed).expect("signed manifest serializes");
    let state = Arc::new(
        CohortManifestState::load_with_clock(
            HostId("host-a".to_string()),
            &signed_toml,
            PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()])
                .expect("pinned authority key"),
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(FixedClock),
        )
        .expect("verified manifest state"),
    );
    let tenant_map = Arc::new(
        maos_bin::tenant_map::TenantMapAdapter::new(Arc::clone(&state), "host-a", true)
            .expect("refreshable tenant map"),
    );
    let adapter = CrossTeamCrossingAdapter::new(
        dead_store("team-a").await,
        TeamId::new("team-a").expect("canonical team"),
        [0x42; 32],
    )
    .with_erase_reconciliation(
        Arc::new(maos_bin::cross_team_consent::CrossTeamConsentAdapter::new(state)),
        tenant_map,
        SpiritId::from("spirit-a"),
        Arc::new(maos_iac::TransparencyLogAdapter::open_in_memory(13_600)),
    );
    let frame = erase_frame_with_binding(
        &FrameAddress {
            spirit_id: SpiritId::from("spirit-b"),
            host_id: Some(HostId("host-b".to_string())),
            role: None,
        },
        &HostId("host-a".to_string()),
        1,
        &TeamId::new("team-a").expect("canonical team"),
        7,
        &maos_domain::memory::MemoryNamespace::Default,
        "never-shared".to_string(),
        "0123456789abcdef0123456789abcdef".to_string(),
        maos_bin::cross_team_crossing::erase_locator_digest(
            "team-a",
            "team-b",
            7,
            "default",
            "never-shared",
        ),
    )
    .expect("erase control encodes");
    match adapter.apply_crossing("team-b", &frame).await {
        CrossingOutcome::Refused(CrossingRefusal::ApplyFailed { reason, .. }) => {
            assert_eq!(reason, "collective erase provenance not found");
        }
        other => panic!("unshared locator must receive the typed provenance refusal, got {other:?}"),
    }
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

/// Story 13.6 (AC2) — the composed scene adds `team-c`, so the third
/// provisioned database finally has a reader on this harness too.
fn pg_conn_team(team: &str) -> String {
    let var = match team {
        "team-a" => "MAOS_TEST_POSTGRES_TEAM_A",
        "team-b" => "MAOS_TEST_POSTGRES_TEAM_B",
        "team-c" => "MAOS_TEST_POSTGRES_TEAM_C",
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

/// Sign one cohort manifest, stamping each member's real certificate
/// fingerprint from `identities` (positional, member order).
///
/// Takes the manifest and a SLICE rather than two fixed identities: Story
/// 13.6's composed scene runs THREE daemons, and hard-coding an arity here
/// would force a second copy of the signing/serialization step.
fn write_daemon_manifest(
    dir: &Path,
    mut manifest: CohortManifest,
    identities: &[&DaemonIdentity],
) -> (PathBuf, SigningKey) {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    assert_eq!(
        manifest.members.len(),
        identities.len(),
        "every cohort member needs a minted identity"
    );
    for (member, identity) in manifest.members.iter_mut().zip(identities) {
        member.fingerprint = identity.fingerprint.to_string();
    }
    let signed = manifest.signed_with(&signing_key);
    let path = dir.join("manifest.toml");
    std::fs::write(
        &path,
        toml::to_string(&signed).expect("signed manifest serializes"),
    )
    .expect("write signed manifest");
    (path, signing_key)
}

/// The 13.6b two-daemon crossing manifest: host-a (team-a) → host-b (team-b).
fn two_team_crossing_manifest() -> CohortManifest {
    let mut manifest = manifest_with(COHORT_SCHEMA_V4, Some("team-a"));
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
    manifest
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

/// One configured peer of a daemon: who it is, where it listens, and the boot
/// nonce its pin is bound to.
///
/// Story 13.6 (AC2): the middle daemon of the composed chain both ACCEPTS from
/// host-a and SENDS to host-c, so `peers` and `peer_pins` — already `Vec` in
/// the production config types — finally carry more than one entry.
struct DaemonPeer<'a> {
    identity: &'a DaemonIdentity,
    host: &'a str,
    endpoint: String,
    boot_nonce: u64,
}

fn write_daemon_file(
    dir: &Path,
    tag: &str,
    manifest_path: &Path,
    authority: &SigningKey,
    identity: &DaemonIdentity,
    local_host: &str,
    control_spirit: &str,
    peers: &[DaemonPeer<'_>],
) -> PathBuf {
    let intent = A2AIntent::new(maos_a2a_core::COHORT_INTENT_COLLECTIVE_SHARE);
    let erase_intent = A2AIntent::new(maos_a2a_core::CROSS_TEAM_COLLECTIVE_ERASE_INTENT);
    let file = DaemonFileConfig {
        tcp: maos_a2a_tcp::TcpA2AConfig {
            listen_addr: "127.0.0.1:0".parse().expect("loopback listen address"),
            own_cert_chain: identity.cert.clone(),
            own_private_key: identity.private_key.clone(),
            peer_pins: peers
                .iter()
                .map(|peer| maos_a2a_tcp::config::PinnedFingerprint {
                    peer_id: PeerId::new(peer.host),
                    fingerprint: peer.identity.fingerprint.clone(),
                    boot_nonce: peer.boot_nonce,
                })
                .collect(),
            handshake_timeout: Duration::from_secs(30),
            ca_roots: None,
        },
        peers: peers
            .iter()
            .map(|peer| A2APeerConfig {
                peer_id: PeerId::new(peer.host),
                endpoint: peer.endpoint.clone(),
                cert_fingerprint: peer.identity.fingerprint.clone(),
                profile: A2AProfile::CrossHost,
                allowlists: ConsentAllowlists {
                    send_allowlist: vec![intent.clone(), erase_intent.clone()],
                    accept_allowlist: vec![intent.clone(), erase_intent.clone()],
                },
                partition_timeout_secs: 30,
                consent_ttl_secs: maos_a2a_core::config::DEFAULT_CONSENT_TTL_SECS,
            })
            .collect(),
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
        let mut readiness_open = true;
        while lines.read_line(&mut line).unwrap_or_default() != 0 {
            if readiness_open {
                readiness_open = tx.send(std::mem::take(&mut line)).is_ok();
            } else {
                // Keep draining after readiness so a later daemon diagnostic
                // cannot fill or close the child's stderr pipe.
                line.clear();
            }
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

/// Story 13.6 (AC2) — `region` is now a parameter, not a hard-coded
/// `region-a`.
///
/// ⚠ The composed journey passes each daemon the region carried by its OWN
/// signed `TeamEntry`, so the scene DERIVES the value that production merely
/// assumes. Nothing in production reconciles `MAOS_REGION_HOME` against
/// `TeamEntry.region` — that is a recorded finding, not something this story
/// fixes (trap 1: 13.6 judges, it does not build).
fn daemon_command(
    dir: &Path,
    tag: &str,
    config: &Path,
    postgres: &str,
    home_team: &str,
    region: &str,
    boot_nonce: u64,
) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env("MAOS_ONE_SHOT", "cohort-a2a-daemon")
        .env("MAOS_COHORT_DAEMON_CONFIG", config)
        .env("MAOS_LOOM_POSTGRES", postgres)
        .env("MAOS_LOOM_HOME_TEAM", home_team)
        .env("MAOS_REGION_HOME", region)
        .env("MAOS_CROSS_TEAM_BASE_SEED", "42".repeat(32))
        .env("MAOS_TEST_BOOT_NONCE", boot_nonce.to_string())
        .env(
            "MAOS_AUDIT_DB",
            dir.join(format!("{tag}.transparency.sqlite")),
        )
        .env("MAOS_OLLAMA_URL", "skip")
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    // Child commands inherit the test runner's environment. Start every daemon
    // with no share request; sender call sites opt in field by field.
    for key in [
        "MAOS_CROSS_TEAM_SHARE_PEER",
        "MAOS_CROSS_TEAM_SHARE_TO_TEAM",
        "MAOS_CROSS_TEAM_SHARE_PID",
        "MAOS_CROSS_TEAM_SHARE_NAMESPACE",
        "MAOS_CROSS_TEAM_SHARE_KEY",
        "MAOS_CROSS_TEAM_SHARE_VALUE",
    ] {
        command.env_remove(key);
    }
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
    let (manifest_path, authority) = write_daemon_manifest(
        fixture.path(),
        two_team_crossing_manifest(),
        &[&identity_a, &identity_b],
    );
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
        &[DaemonPeer {
            identity: &identity_a,
            host: "host-a",
            endpoint: "tls://127.0.0.1:1".to_string(),
            boot_nonce: NONCE_A,
        }],
    );
    let (mut daemon_b, port_b) = boot_daemon(daemon_command(
        fixture.path(),
        "team-b",
        &config_b,
        &team_b_conn,
        "team-b",
        "region-a",
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
        &[DaemonPeer {
            identity: &identity_b,
            host: "host-b",
            endpoint: format!("tls://127.0.0.1:{port_b}"),
            boot_nonce: NONCE_B,
        }],
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
        "region-a",
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
            && row.namespace_detail.starts_with("xteam:team-a:")
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
            .filter(|row| {
                is_expected_row(row)
                    && row.cross_emitter_host.is_some()
                    && row.cross_op_id.is_some()
                    && row.cross_source_ts.is_some()
                    && row.cross_source_region.is_some()
                    && has_re_attestation(row)
            })
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
            .filter(|row| {
                row.spirit_pid == 7
                    && row.namespace_kind == "default"
                    && row.namespace_detail.is_empty()
                    && row.key == key
                    && row.value_kind == "text"
                    && row.value_data == value.as_bytes()
                    && row.source_region == "region-a"
                    && row.source_team.as_ref().map(TeamId::as_str) == Some("team-a")
                    && row.distillation_depth == Some(1)
                    && row.intent_lineage.is_some()
                    && row.cross_emitter_host.is_none()
                    && row.cross_op_id.is_none()
                    && row.cross_source_ts.is_none()
                    && row.cross_source_region.is_none()
                    && row.source_log_ref.is_empty()
            })
            .count(),
        1,
        "originate_team_row must persist one native origin row before transport"
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
        // Story 13.6: this leg was RED at HEAD on any live substrate — the
        // store declared `home_team` but carried no tenant map, so
        // `init_schema`'s `connection_assignment_guard` failed
        // `TenantMapStale { reason: "tenant map is not configured" }` before a
        // single assertion ran. A leg that could never be green is a null
        // control; the repair is the production wiring, from the same signed
        // manifest state the consent adapter already uses.
        .with_tenant_map({
            let map = Arc::new(
                maos_bin::tenant_map::TenantMapAdapter::new(Arc::clone(&state), "host-b", true)
                    .expect("tenant map from the signed manifest"),
            );
            // `team_guard(spirit_pid)` runs on every read/erase, so the pid the
            // adapter legs use must be mapped to a Spirit team B's SIGNED entry
            // lists — the registration the daemon performs for its control
            // Spirit, done explicitly here because this leg drives the store
            // directly rather than through a daemon boot.
            map.register_spirit(7, SpiritId::from("spirit-b"));
            map as Arc<dyn maos_loom_lite::tenant::TenantMapPort>
        })
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
    .expect("team-a store")
    .with_tenant_map({
        // Same repair as the team-B store above: a `home_team` store with no
        // tenant map cannot pass `connection_assignment_guard`, so this half of
        // the physical-absence control could never have run.
        let state_a = Arc::new(
            maos_cohort::CohortManifestState::load_with_clock(
                HostId("host-a".to_string()),
                &signed_toml,
                maos_cohort::PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()])
                    .expect("pinned authority keys"),
                Arc::new(maos_cohort::InMemoryCohortAuditSink::default()),
                Arc::new(FixedClock),
            )
            .expect("verified manifest state for host-a"),
        );
        let map = Arc::new(
            maos_bin::tenant_map::TenantMapAdapter::new(state_a, "host-a", true)
                .expect("tenant map from the signed manifest"),
        );
        map.register_spirit(7, SpiritId::from("spirit-a"));
        map as Arc<dyn maos_loom_lite::tenant::TenantMapPort>
    });
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

    // A reconciliation path must erase the physical crossed row it resolved,
    // even when a native row has the same logical address. Generic `erase`
    // prefers native rows, so using it here would leave the crossed copy behind.
    store_b
        .write(
            7,
            &maos_domain::memory::MemoryNamespace::Default,
            "live-crossing",
            maos_domain::memory::MemoryValue::Text("native-wins-normal-read".to_string()),
        )
        .await
        .expect("seed same-address native row");
    let receipt = store_b
        .erase_crossed_row(
            7,
            &maos_domain::memory::MemoryNamespace::Default,
            "live-crossing",
            &TeamId::new("team-a").expect("canonical source team"),
            1,
            "region-a",
        )
        .await
        .expect("erase the selected crossed physical row");
    assert_eq!(receipt.deleted_rows, 1);
    assert_eq!(
        store_b
            .read(
                7,
                &maos_domain::memory::MemoryNamespace::Default,
                "live-crossing"
            )
            .await
            .expect("read native survivor"),
        Some(maos_domain::memory::MemoryValue::Text(
            "native-wins-normal-read".to_string()
        )),
        "the same-address native row must survive crossed-row reconciliation"
    );
    let client = store_b.pool().get().await.expect("borrow team-b client");
    let physical_rows = LoomLiteStore::read_all_rows_from(&**client)
        .await
        .expect("read physical rows");
    assert!(
        physical_rows.iter().all(|row| !(row.key == "live-crossing"
            && row.namespace_detail.starts_with("xteam:team-a:"))),
        "the crossed physical copy must be gone, never silently left behind"
    );
}

// ─── Story 13.6 (AC2/AC3) — the composed Reza journey ───────────────────────
//
// "One run" is structurally impossible: `CrossTeamShareRequest::from_env`
// returns AT MOST ONE request and `run_cohort_a2a_daemon` calls the emitter
// once, then parks on `ctrl_c()`. So the scene is ONE COMPOSED TOPOLOGY,
// written down: **3 daemon processes + 3 CLI one-shot processes = 6**, under
// one signed manifest, one authority key, one base seed, three distinct
// datnames, and one SHARED `MAOS_HOME`.
//
// ⚠ The chain is A→B, B→C — TWO INDEPENDENT ORIGINATIONS, not a transitive
// flow. `originate_team_row` mints a NEW row from B's own store stamped
// `source_team=team-b`; nothing of team-a's row travels onward. The chain
// shape is chosen precisely because it needs ZERO production lines: a true
// hub from one host would need `from_env` to return more than one request,
// which trap 1 forbids this story from building.
//
// The scene is genuinely THREE-REGION: each `TeamEntry.region` is distinct
// `region-a` because `apply_replication_bundle` binds the destination region:
// a genuinely cross-region crossing is REFUSED by the region axis, by design
// (the 13.3 same-region reflex). The three-region half of the substrate lives
// on the region-axis databases the SLO/consensus gates read, not on the
// crossing. `daemon_command` now takes the region from each team's SIGNED
// entry rather than a literal — production reconciles nothing here, and that
// is a recorded finding.

const JOURNEY_NONCE_A: u64 = 13_600_101;
const JOURNEY_NONCE_B: u64 = 13_600_102;
const JOURNEY_NONCE_C: u64 = 13_600_103;
/// Each team's SIGNED home region. `TeamEntry.region` is authoritative and is
/// what every daemon receives as `MAOS_REGION_HOME`, so the composed scene is
/// genuinely three-region rather than three copies of one.
fn journey_region(team: &str) -> &'static str {
    match team {
        "team-a" => "region-a",
        "team-b" => "region-b",
        "team-c" => "region-c",
        other => panic!("unknown journey team {other}"),
    }
}
const RESEARCHER_ROUTE_KEY: &str = "researcher/collective-route-ready";

/// A THREE-team cohort manifest for the composed journey.
///
/// A NEW builder, not a widened `manifest_with`: that one has seven call sites
/// across eighteen tests which all assert two-member / two-team shapes, so
/// mutating it would rewrite unrelated legs to buy nothing.
///
/// Datnames are DERIVED from the live connection strings rather than written
/// as literals — `connection_assignment_guard` proves
/// `datname_for(home_team) == current_database()` at boot, so a literal that
/// drifted from the substrate would surface as a confusing tenant refusal
/// instead of a topology error.
fn three_team_journey_manifest() -> CohortManifest {
    let signing_key = SigningKey::from_bytes(&[23; 32]);
    let team = |name: &str| TeamId::new(name).expect("canonical team");
    let member = |host: &str, name: &str| CohortMember {
        host_id: host.to_string(),
        fingerprint: format!("sha256:{}", "ab".repeat(32)),
        roles: vec!["worker".to_string()],
        team: Some(team(name)),
    };
    let entry = |name: &str, members: Vec<SpiritId>| TeamEntry {
        team_id: team(name),
        region: Region::canonicalize(journey_region(name)).expect("canonical region"),
        datname: datname_of(&pg_conn_team(name)),
        members,
    };
    let share = maos_a2a_core::COHORT_INTENT_COLLECTIVE_SHARE.to_string();
    let tuple = |peer: &str| ConsentTuple {
        peer: peer.to_string(),
        role: "worker".to_string(),
        intent: share.clone(),
    };
    // Transport-matrix entries for the erase control: team-b's one-shot SENDS
    // `collective:erase`, team-a's daemon ACCEPTS it. The wire intent is
    // distinct from share (Story 13.6 closure review F4), so the matrix must
    // name it explicitly — a share-only route must not carry a destructive
    // control.
    let erase_tuple = |peer: &str| ConsentTuple {
        peer: peer.to_string(),
        role: "worker".to_string(),
        intent: maos_a2a_core::CROSS_TEAM_COLLECTIVE_ERASE_INTENT.to_string(),
    };
    let grant = |from: &str, to: &str, intent: &str| CrossTeamConsentGrant {
        from_team: team(from),
        to_team: team(to),
        intent: intent.to_string(),
    };
    CohortManifest {
        schema_version: COHORT_SCHEMA_V4,
        cohort_id: "reza-journey-13-6".to_string(),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signing_key.verifying_key().to_bytes())],
        },
        members: vec![
            member("host-a", "team-a"),
            member("host-b", "team-b"),
            member("host-c", "team-c"),
        ],
        consent: ConsentMatrix {
            send: vec![
                tuple("host-a"),
                tuple("host-b"),
                tuple("host-c"),
                // send tuples key the RECEIVER: team-b's erase one-shot sends
                // `collective:erase` TO host-a.
                erase_tuple("host-a"),
            ],
            accept: vec![
                tuple("host-a"),
                tuple("host-b"),
                // accept tuples key the SENDER: host-a's daemon admits the
                // erase control FROM host-b.
                erase_tuple("host-b"),
            ],
        },
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams: Some(vec![
            // `researcher` is a declared member of team-a: the Spirit→
            // collective route registers the loaded Spirit against its signed
            // team, and an unlisted Spirit is refused `TenantSpiritUnmapped`.
            entry(
                "team-a",
                vec![SpiritId::from("spirit-a"), SpiritId::from("researcher")],
            ),
            entry("team-b", vec![SpiritId::from("spirit-b")]),
            entry("team-c", vec![SpiritId::from("spirit-c")]),
        ]),
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: vec![
            grant("team-b", "team-a", "collective:erase"),
            grant("team-a", "team-b", "collective:share"),
            grant("team-b", "team-c", "collective:share"),
            // The read side: `maos traceback --team team-b` runs from a
            // home-team-a process, and `CrossWallRecallConsentAdapter` grants
            // only on `cross_team_admits(home_team, remote_team, "log:recall")`.
            grant("team-a", "team-b", "log:recall"),
        ],
}
}

/// A CLI one-shot in the composed scene: same signed manifest, same shared
/// `MAOS_HOME`, same base seed — only the team, its signed region, and the
/// mode change. Callers that need cohort-backed dispatch explicitly install
/// their validated daemon config.
fn journey_cli(home: &Path, postgres: &str, home_team: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maos"));
    command
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .env_remove("MAOS_ONE_SHOT")
        .env("MAOS_HOME", home)
        .env_remove("MAOS_AUDIT_DB")
        .env("MAOS_LOOM_POSTGRES", postgres)
        .env("MAOS_LOOM_HOME_TEAM", home_team)
        .env("MAOS_REGION_HOME", journey_region(home_team))
        .env("MAOS_CROSS_TEAM_BASE_SEED", "42".repeat(32))
        .env("MAOS_OLLAMA_URL", "skip");
    command
}

/// The provenance a crossed row must carry, and nothing more (AC3).
fn crossed_row_matches(
    row: &CollectiveRow,
    pid: i64,
    source_team: &str,
    key: &str,
    value: &str,
) -> bool {
    row.spirit_pid == pid
        && row.namespace_kind == "default"
        && row
            .namespace_detail
            .starts_with(&format!("xteam:{source_team}:"))
        && row.key == key
        && row.value_kind == "text"
        && row.value_data == value.as_bytes()
        && row.source_region == journey_region(source_team)
        && row.source_team.as_ref().map(TeamId::as_str) == Some(source_team)
        && row.cross_emitter_host.is_some()
        && row.cross_op_id.is_some()
        && row.cross_source_ts == Some(row.source_ts)
        && row.cross_source_region.as_deref() == Some(row.source_region.as_str())
        && row.distillation_depth == Some(1)
        && row.intent_lineage.as_ref().is_some_and(|lineage| {
            let intents = lineage.as_slice();
            intents.len() == 1
                && intents[0].as_str() == maos_a2a_core::COHORT_INTENT_COLLECTIVE_SHARE
        })
}

/// The re-attestation marker is exactly `{source_region, merkle_root}` — two
/// keys, no payload, no transparency-log reference. AC3's minimum-disclosure
/// negative is that a THIRD key would fail this.
fn re_attestation_is_minimal(row: &CollectiveRow, source_region: &str) -> bool {
    let Ok(marker) = serde_json::from_str::<serde_json::Value>(&row.source_log_ref) else {
        return false;
    };
    marker.as_object().is_some_and(|object| object.len() == 2)
        && marker
            .get("source_region")
            .and_then(serde_json::Value::as_str)
            == Some(source_region)
        && marker
            .get("merkle_root")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|root| root.len() == 64 && root.bytes().all(|b| b.is_ascii_hexdigit()))
}

async fn all_rows(client: &tokio_postgres::Client) -> Vec<CollectiveRow> {
    LoomLiteStore::read_all_rows_from(client)
        .await
        .expect("read physical rows")
}

fn erase_reconciliation_problem(
    source_present: bool,
    destination_present: bool,
) -> Option<&'static str> {
    match (source_present, destination_present) {
        (false, false) => None,
        (true, false) => Some("one-sided erase: source row survived after destination erase"),
        (false, true) => Some("erase left the destination row without its source row"),
        (true, true) => Some("erase left both source and destination rows"),
    }
}

#[test]
fn one_sided_erase_is_red_on_reconciliation() {
    let problem = erase_reconciliation_problem(true, false)
        .expect("a one-sided erase must be a reconciliation failure");
    assert!(
        problem.contains("one-sided erase") && problem.contains("source row"),
        "the reconciliation must name the one-sided defect: {problem}"
    );
    assert!(
        erase_reconciliation_problem(false, false).is_none(),
        "only a fully reconciled erase is green"
    );
}

#[tokio::test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B/_C (live 3-team Postgres)"]
async fn reza_three_team_three_region_production_journey() {
    let _evidence = evidence_record::attest("reza_three_team_three_region_production_journey");
    // probes into proof of the complete journey.
    let _guard = LIVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());

    let conn_a = pg_conn_team("team-a");
    let conn_b = pg_conn_team("team-b");
    let conn_c = pg_conn_team("team-c");
    let datnames = [
        datname_of(&conn_a),
        datname_of(&conn_b),
        datname_of(&conn_c),
    ];
    for (index, left) in datnames.iter().enumerate() {
        for right in &datnames[index + 1..] {
            assert_ne!(
                left, right,
                "the composed journey needs three PHYSICALLY distinct databases"
            );
        }
    }
    let raw_a = raw_connect_team("team-a").await;
    let raw_b = raw_connect_team("team-b").await;
    let raw_c = raw_connect_team("team-c").await;

    let fixture = tempfile::tempdir().expect("journey fixture");
    // ONE shared MAOS_HOME across every process. Without it each process
    // derives its own audit root and `maos traceback --team team-b` reads a
    // path no daemon ever wrote (`transparency_log_path_for_team`).
    let home = fixture.path().join("maos-home");
    std::fs::create_dir_all(&home).expect("shared MAOS_HOME");
    let team_b_tl = home
        .join("audit")
        .join("teams")
        .join("team-b")
        .join("transparency.sqlite");
    assert!(
        !team_b_tl.exists(),
        "team B's tenant log must not exist before a daemon writes it"
    );

    let identity_a = mint_daemon_identity(fixture.path(), "host-a");
    let identity_b = mint_daemon_identity(fixture.path(), "host-b");
    let identity_c = mint_daemon_identity(fixture.path(), "host-c");
    let (manifest_path, authority) = write_daemon_manifest(
        fixture.path(),
        three_team_journey_manifest(),
        &[&identity_a, &identity_b, &identity_c],
    );

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let key_ab = format!("reza-a-to-b-{}-{nonce}", std::process::id());
    let key_bc = format!("reza-b-to-c-{}-{nonce}", std::process::id());
    let value_ab = "team-a-distillate";
    let value_bc = "team-b-distillate";

    // ── PROCESS 1: the tail of the chain. host-c only ACCEPTS.
    let config_c = write_daemon_file(
        fixture.path(),
        "journey-c",
        &manifest_path,
        &authority,
        &identity_c,
        "host-c",
        "spirit-c",
        &[DaemonPeer {
            identity: &identity_b,
            host: "host-b",
            endpoint: "tls://127.0.0.1:1".to_string(),
            boot_nonce: JOURNEY_NONCE_B,
        }],
    );
    let mut command_c = daemon_command(
        fixture.path(),
        "journey-c",
        &config_c,
        &conn_c,
        "team-c",
        journey_region("team-c"),
        JOURNEY_NONCE_C,
    );
    command_c
        .env("MAOS_HOME", &home)
        .env_remove("MAOS_AUDIT_DB");
    let (mut daemon_c, port_c) =
        boot_daemon(command_c).unwrap_or_else(|error| panic!("team-c daemon failed: {error}"));

    // ── PROCESS 2: the middle. host-b ACCEPTS from host-a AND SENDS to host-c
    // — the first configuration in this repo where `peers`/`peer_pins` carry
    // two entries.
    let config_b = write_daemon_file(
        fixture.path(),
        "journey-b",
        &manifest_path,
        &authority,
        &identity_b,
        "host-b",
        "spirit-b",
        &[
            DaemonPeer {
                identity: &identity_a,
                host: "host-a",
                endpoint: "tls://127.0.0.1:1".to_string(),
                boot_nonce: JOURNEY_NONCE_A,
            },
            DaemonPeer {
                identity: &identity_c,
                host: "host-c",
                endpoint: format!("tls://127.0.0.1:{port_c}"),
                boot_nonce: JOURNEY_NONCE_C,
            },
        ],
    );
    let mut command_b = daemon_command(
        fixture.path(),
        "journey-b",
        &config_b,
        &conn_b,
        "team-b",
        journey_region("team-b"),
        JOURNEY_NONCE_B,
    );
    command_b
        .env("MAOS_HOME", &home)
        .env_remove("MAOS_AUDIT_DB")
        .env("MAOS_CROSS_TEAM_SHARE_PEER", "host-c")
        .env("MAOS_CROSS_TEAM_SHARE_TO_TEAM", "team-c")
        .env("MAOS_CROSS_TEAM_SHARE_PID", "8")
        .env("MAOS_CROSS_TEAM_SHARE_NAMESPACE", "default")
        .env("MAOS_CROSS_TEAM_SHARE_KEY", &key_bc)
        .env("MAOS_CROSS_TEAM_SHARE_VALUE", value_bc);
    let (mut daemon_b, port_b) =
        boot_daemon(command_b).unwrap_or_else(|error| panic!("team-b daemon failed: {error}"));

    // ── PROCESS 3: the head. host-a only SENDS.
    let config_a = write_daemon_file(
        fixture.path(),
        "journey-a",
        &manifest_path,
        &authority,
        &identity_a,
        "host-a",
        "spirit-a",
        &[DaemonPeer {
            identity: &identity_b,
            host: "host-b",
            endpoint: format!("tls://127.0.0.1:{port_b}"),
            boot_nonce: JOURNEY_NONCE_B,
        }],
    );
    let mut command_a = daemon_command(
        fixture.path(),
        "journey-a",
        &config_a,
        &conn_a,
        "team-a",
        journey_region("team-a"),
        JOURNEY_NONCE_A,
    );
    command_a
        .env("MAOS_HOME", &home)
        .env_remove("MAOS_AUDIT_DB")
        .env("MAOS_CROSS_TEAM_SHARE_PEER", "host-b")
        .env("MAOS_CROSS_TEAM_SHARE_TO_TEAM", "team-b")
        .env("MAOS_CROSS_TEAM_SHARE_PID", "7")
        .env("MAOS_CROSS_TEAM_SHARE_NAMESPACE", "default")
        .env("MAOS_CROSS_TEAM_SHARE_KEY", &key_ab)
        .env("MAOS_CROSS_TEAM_SHARE_VALUE", value_ab);
    let (mut daemon_a, port_a) =
        boot_daemon(command_a).unwrap_or_else(|error| panic!("team-a daemon failed: {error}"));

    // Both crossings land, each in the consumer team's OWN physical database.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let landed_b = all_rows(&raw_b)
            .await
            .iter()
            .filter(|row| {
                crossed_row_matches(row, 7, "team-a", &key_ab, value_ab)
                    && re_attestation_is_minimal(row, journey_region("team-a"))
            })
            .count();
        let landed_c = all_rows(&raw_c)
            .await
            .iter()
            .filter(|row| {
                crossed_row_matches(row, 8, "team-b", &key_bc, value_bc)
                    && re_attestation_is_minimal(row, journey_region("team-b"))
            })
            .count();
        if landed_b == 1 && landed_c == 1 {
            break;
        }
        for (label, daemon) in [
            ("team-a", &mut daemon_a),
            ("team-b", &mut daemon_b),
            ("team-c", &mut daemon_c),
        ] {
            assert!(
                daemon.exited().is_none(),
                "the {label} daemon exited before both crossings landed"
            );
        }
        assert!(
            Instant::now() < deadline,
            "the composed chain did not land within 60s (A→B {landed_b}, B→C {landed_c})"
        );
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    // AC3 — minimum disclosure, judged on the rows that actually crossed.
    for (label, rows, source_team, key, value) in [
        (
            "team-b",
            all_rows(&raw_b).await,
            "team-a",
            &key_ab,
            value_ab,
        ),
        (
            "team-c",
            all_rows(&raw_c).await,
            "team-b",
            &key_bc,
            value_bc,
        ),
    ] {
        let crossed: Vec<&CollectiveRow> = rows
            .iter()
            .filter(|row| row.key == *key && row.source_team.is_some())
            .collect();
        assert_eq!(crossed.len(), 1, "{label}: exactly one crossed row");
        let row = crossed[0];
        assert!(
            crossed_row_matches(
                row,
                if source_team == "team-a" { 7 } else { 8 },
                source_team,
                key,
                value
            ),
            "{label}: the crossed row must carry exactly the five policy-allowed \
             provenance fields"
        );
        assert!(
            re_attestation_is_minimal(row, journey_region(source_team)),
            "{label}: the re-attestation marker must be exactly \
             {{source_region, merkle_root}} — a third key is a disclosure leak"
        );
        assert!(
            !row.source_log_ref.contains(value),
            "{label}: the raw payload must never appear in the provenance marker"
        );
        assert!(
            !row.source_log_ref.contains("42424242"),
            "{label}: the cross-team base seed must never ride the marker"
        );
    }

    // The chain is TWO ORIGINATIONS: team-b's outbound row is stamped
    // `source_team=team-b` from team-b's OWN store, and carries nothing of
    // team-a's row. Say it in an assertion, not only in prose.
    let team_b_rows = all_rows(&raw_b).await;
    let originated_in_b = team_b_rows
        .iter()
        .filter(|row| row.key == key_bc && row.source_log_ref.is_empty())
        .count();
    assert_eq!(
        originated_in_b, 1,
        "team-b must persist its own attested origin row before transport"
    );
    assert!(
        !team_b_rows
            .iter()
            .any(|row| row.key == key_bc && row.value_data == value_ab.as_bytes()),
        "the B→C origination must not carry team-a's payload — the chain is not \
         a transitive flow"
    );

    // ── PROCESS 4: `maos run <researcher-manifest>`.
    raw_a
        .execute("DELETE FROM collective_memory WHERE key = $1", &[&RESEARCHER_ROUTE_KEY])
        .await
        .expect("clear researcher route");
    let researcher = journey_cli(&home, &conn_a, "team-a")
        .env("MAOS_COHORT_DAEMON_CONFIG", &config_a)
        .args(["run", "spirits/researcher/manifest.toml", "--once"])
        .output()
        .expect("maos run researcher");
    assert!(researcher.status.success(), "{}", String::from_utf8_lossy(&researcher.stderr));
    let route_rows: i64 = raw_a
        .query_one("SELECT count(*) FROM collective_memory WHERE key = $1", &[&RESEARCHER_ROUTE_KEY])
        .await
        .expect("count researcher route")
        .get(0);
    assert_eq!(route_rows, 1);

    // ── PROCESS 5: cohort-backed destination erase reconciles the source.
    let erase_config_b = write_daemon_file(
        fixture.path(), "journey-erase-b", &manifest_path, &authority, &identity_b,
        "host-b", "spirit-b",
        &[DaemonPeer {
            identity: &identity_a,
            host: "host-a",
            endpoint: format!("tls://127.0.0.1:{port_a}"),
            boot_nonce: JOURNEY_NONCE_A,
        }],
    );
    let erase = journey_cli(&home, &conn_b, "team-b")
        .env("MAOS_COHORT_DAEMON_CONFIG", &erase_config_b)
        .env("MAOS_TEST_BOOT_NONCE", JOURNEY_NONCE_B.to_string())
        .env("MAOS_ONE_SHOT", "collective-erase")
        .env("MAOS_COLLECTIVE_ERASE_PID", "7")
        .env("MAOS_COLLECTIVE_ERASE_NAMESPACE", "default")
        .env("MAOS_COLLECTIVE_ERASE_KEY", &key_ab)
        .output()
        .expect("maos collective-erase");
    assert!(erase.status.success(), "{}", String::from_utf8_lossy(&erase.stderr));
    let erase_json: serde_json::Value =
        serde_json::from_slice(&erase.stdout).expect("collective erase JSON");
    assert_eq!(erase_json["reconciliation"]["status"], "erase_reconciled");
    let destination_present = all_rows(&raw_b).await.iter().any(|row| row.key == key_ab);
    let source_present = all_rows(&raw_a).await.iter().any(|row| row.key == key_ab);
    assert!(erase_reconciliation_problem(source_present, destination_present).is_none());

    // Close the serving daemons before reading the tenant artifact.
    drop(daemon_a);
    drop(daemon_b);
    drop(daemon_c);
    assert!(team_b_tl.exists());

    // ── PROCESS 6: consented production traceback.
    let traceback = journey_cli(&home, &conn_a, "team-a")
        .env("MAOS_COHORT_DAEMON_CONFIG", &config_a)
        .args(["traceback", "--team", "team-b", "--spirit-pid", "0"])
        .output()
        .expect("maos traceback");
    assert!(traceback.status.success(), "{}", String::from_utf8_lossy(&traceback.stderr));
    let traceback_json: serde_json::Value =
        serde_json::from_slice(&traceback.stdout).expect("traceback JSON");
    assert_eq!(traceback_json["outcome"], "ok");

    // The direct adapter proof is deliberately retained: remote entries expose
    // exactly the six-field disclosure DTO and never payload bytes.
    let _restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
    std::env::set_var("MAOS_HOME", &home);
    let page = {
        use maos_domain::ports::CrossWallLogReadPort;
        maos_bin::cross_wall_log_read::CrossWallLogReadAdapter::new(true)
            .read_remote(
                0,
                &TeamId::new("team-b").expect("canonical team"),
                maos_domain::log_recall::LogRecallFilter::default(),
            )
            .expect("read daemon-written tenant artifact")
    };
    assert!(!page.entries.is_empty());
    for entry in &page.entries {
        let object = serde_json::to_value(entry).expect("entry serializes");
        let fields: BTreeSet<&str> = object
            .as_object()
            .expect("entry object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            fields,
            BTreeSet::from([
                "frame_id", "timestamp_ns", "kind", "intent", "peer_spirit_pid",
                "payload_available",
            ])
        );
        assert!(object["payload_available"].is_boolean());
    }
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

// ─── Story 13.6 (AC4) — the refused-crossing operator tail, and retry ───────
//
// The two slices measurement showed were genuinely uncovered:
//   * NOTHING outside `main.rs` read `crossing_outcome_label` or the emitter's
//     TL `status` field for a REFUSED crossing (`grep` → zero hits). The
//     two-daemon live test exercised only the happy path.
//   * `grep -rn "retry\|recover\|repair"` across all three crossing test files
//     returned ZERO. "Retry succeeds only after a valid consent repair" had no
//     coverage anywhere.

/// Every `status` the emitter journaled to the tenant Transparency Log, oldest
/// first. This is the operator's durable view of a crossing outcome — the
/// field nothing has ever read back for a refusal.
fn crossing_tl_statuses(db: &Path) -> Vec<String> {
    let Ok(connection) = rusqlite::Connection::open(db) else {
        return Vec::new();
    };
    let Ok(mut statement) = connection.prepare(
        "SELECT payload_redacted FROM transparency_log \
         WHERE intent = 'collective.host.cross-team-share' ORDER BY timestamp_ns ASC",
    ) else {
        return Vec::new();
    };
    let Ok(rows) = statement.query_map([], |row| row.get::<_, Vec<u8>>(0)) else {
        return Vec::new();
    };
    rows.filter_map(Result::ok)
        .filter_map(|payload| serde_json::from_slice::<serde_json::Value>(&payload).ok())
        .filter_map(|payload| {
            payload
                .get("status")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect()
}

#[tokio::test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A/_B (live Postgres)"]
async fn refused_crossing_is_operator_visible_and_retry_needs_a_consent_repair() {
    let _evidence = evidence_record::attest(
        "refused_crossing_is_operator_visible_and_retry_needs_a_consent_repair",
    );
    let _guard = LIVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let conn_a = pg_conn_team("team-a");
    let conn_b = pg_conn_team("team-b");
    assert_ne!(datname_of(&conn_a), datname_of(&conn_b));
    let raw_b = raw_connect_team("team-b").await;

    let fixture = tempfile::tempdir().expect("refusal fixture");
    let home = fixture.path().join("maos-home");
    std::fs::create_dir_all(&home).expect("shared MAOS_HOME");
    let team_a_tl = home
        .join("audit")
        .join("teams")
        .join("team-a")
        .join("transparency.sqlite");
    let identity_a = mint_daemon_identity(fixture.path(), "host-a");
    let identity_b = mint_daemon_identity(fixture.path(), "host-b");
    const NONCE_A: u64 = 13_600_201;
    const NONCE_B: u64 = 13_600_202;

    // The ONE defect: the signed manifest carries no `team-a → team-b`
    // collective:share grant. Everything else — identities, pins, allowlists,
    // teams — is the same configuration the happy-path leg uses.
    let mut refusing = two_team_crossing_manifest();
    refusing.cross_team_consent.clear();
    let (manifest_path, authority) =
        write_daemon_manifest(fixture.path(), refusing, &[&identity_a, &identity_b]);

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let key = format!("refused-crossing-{}-{nonce}", std::process::id());

    let boot_pair = |manifest_path: &Path, authority: &SigningKey| {
        let config_b = write_daemon_file(
            fixture.path(),
            "refusal-b",
            manifest_path,
            authority,
            &identity_b,
            "host-b",
            "spirit-b",
            &[DaemonPeer {
                identity: &identity_a,
                host: "host-a",
                endpoint: "tls://127.0.0.1:1".to_string(),
                boot_nonce: NONCE_A,
            }],
        );
        let mut command_b = daemon_command(
            fixture.path(),
            "refusal-b",
            &config_b,
            &conn_b,
            "team-b",
            "region-a",
            NONCE_B,
        );
        command_b
            .env("MAOS_HOME", &home)
            .env_remove("MAOS_AUDIT_DB");
        let (daemon_b, port_b) =
            boot_daemon(command_b).unwrap_or_else(|error| panic!("team-b daemon: {error}"));

        let config_a = write_daemon_file(
            fixture.path(),
            "refusal-a",
            manifest_path,
            authority,
            &identity_a,
            "host-a",
            "spirit-a",
            &[DaemonPeer {
                identity: &identity_b,
                host: "host-b",
                endpoint: format!("tls://127.0.0.1:{port_b}"),
                boot_nonce: NONCE_B,
            }],
        );
        let mut command_a = daemon_command(
            fixture.path(),
            "refusal-a",
            &config_a,
            &conn_a,
            "team-a",
            "region-a",
            NONCE_A,
        );
        command_a
            .env("MAOS_HOME", &home)
            .env_remove("MAOS_AUDIT_DB")
            .env("MAOS_CROSS_TEAM_SHARE_PEER", "host-b")
            .env("MAOS_CROSS_TEAM_SHARE_TO_TEAM", "team-b")
            .env("MAOS_CROSS_TEAM_SHARE_PID", "7")
            .env("MAOS_CROSS_TEAM_SHARE_NAMESPACE", "default")
            .env("MAOS_CROSS_TEAM_SHARE_KEY", &key)
            .env("MAOS_CROSS_TEAM_SHARE_VALUE", "refused-then-repaired");
        let (daemon_a, _port_a) =
            boot_daemon(command_a).unwrap_or_else(|error| panic!("team-a daemon: {error}"));
        (daemon_a, daemon_b)
    };

    async fn wait_for_statuses(db: &Path, count: usize) -> Vec<String> {
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            let statuses = crossing_tl_statuses(db);
            if statuses.len() >= count {
                return statuses;
            }
            assert!(
                Instant::now() < deadline,
                "the emitter journaled {} of {count} expected crossing outcomes",
                statuses.len()
            );
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
    }

    // ── 1. The refusal, and the operator tail nothing has ever read.
    let pair = boot_pair(&manifest_path, &authority);
    let statuses = wait_for_statuses(&team_a_tl, 1).await;
    drop(pair);
    assert_eq!(
        statuses[0], "crossing_consent_denied",
        "the TL `status` field must carry the TYPED cause of a refusal, not a \
         generic failure token"
    );
    assert!(
        !all_rows(&raw_b).await.iter().any(|row| row.key == key),
        "a refused crossing must leave no row in the destination team"
    );

    // ── 2. Retry WITHOUT a repair. The same request, a fresh process, the same
    // refusal — a retry is not a remedy.
    let pair = boot_pair(&manifest_path, &authority);
    let statuses = wait_for_statuses(&team_a_tl, 2).await;
    drop(pair);
    assert_eq!(
        statuses[1], "crossing_consent_denied",
        "an unrepaired retry must reproduce the SAME typed refusal"
    );
    assert!(
        !all_rows(&raw_b).await.iter().any(|row| row.key == key),
        "an unrepaired retry must still leave no row in the destination team"
    );

    // ── 3. The repair: re-sign the manifest WITH the grant, at a new version,
    // and retry. Only now may the crossing land.
    let mut repaired = two_team_crossing_manifest();
    repaired.version = 2;
    let (repaired_path, repaired_authority) =
        write_daemon_manifest(fixture.path(), repaired, &[&identity_a, &identity_b]);
    let pair = boot_pair(&repaired_path, &repaired_authority);
    let statuses = wait_for_statuses(&team_a_tl, 3).await;
    drop(pair);
    assert_eq!(
        statuses[2], "crossing_applied",
        "a retry AFTER a valid consent repair must succeed"
    );
    assert!(
        all_rows(&raw_b)
            .await
            .iter()
            .any(|row| row.key == key
                && row.source_team.as_ref().map(TeamId::as_str) == Some("team-a")),
        "the repaired retry must land the row in team B"
    );

    // The operator tail is TYPED across the whole sequence: the same request
    // produced two distinguishable outcomes, and the distinction survived into
    // the durable audit surface rather than only into a process's stdout.
    let distinct: BTreeSet<&str> = statuses.iter().map(String::as_str).collect();
    assert_eq!(
        distinct,
        BTreeSet::from(["crossing_consent_denied", "crossing_applied"]),
        "refusal and success must stay distinguishable on the operator surface"
    );
}

// ─── Story 13.6 (AC6) — the fourteen-institution Cortex axis ───────────────
//
// Reza's three teams are one institution. This leg deliberately models the
// other axis as fourteen independent signed cohorts: no shared authority,
// identity, host binding, team, or physical datname. It creates the fourteen
// ephemeral datnames on the configured local Postgres server so the signed
// topology is checked against a real substrate, then removes them before exit.

struct InstitutionWitness {
    authority: [u8; 32],
    datname: String,
    host: String,
    team: TeamId,
    state: Arc<CohortManifestState>,
}

/// Removes any successfully provisioned live-test databases during unwinding.
///
/// `Drop` cannot await, so it delegates the asynchronous Postgres work to a
/// dedicated current-thread runtime. Cleanup errors are deliberately ignored
/// here: a destructor must not replace the test's original failure.
struct InstitutionDatabaseDropGuard {
    base_connection: String,
    datnames: Vec<String>,
    armed: bool,
}

impl InstitutionDatabaseDropGuard {
    fn new(base_connection: String) -> Self {
        Self {
            base_connection,
            datnames: Vec::new(),
            armed: true,
        }
    }

    fn record_created(&mut self, datname: String) {
        self.datnames.push(datname);
    }

    async fn remove_all(&self) -> Result<(), tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(&self.base_connection, NoTls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        for datname in &self.datnames {
            client
                .execute(&format!("DROP DATABASE {datname} WITH (FORCE)"), &[])
                .await?;
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for InstitutionDatabaseDropGuard {
    fn drop(&mut self) {
        if !self.armed || self.datnames.is_empty() {
            return;
        }

        let base_connection = self.base_connection.clone();
        let datnames = std::mem::take(&mut self.datnames);
        if let Ok(thread) = std::thread::Builder::new()
            .name("institution-database-cleanup".to_string())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    return;
                };
                runtime.block_on(async move {
                    let Ok((client, connection)) =
                        tokio_postgres::connect(&base_connection, NoTls).await
                    else {
                        return;
                    };
                    tokio::spawn(async move {
                        let _ = connection.await;
                    });
                    for datname in datnames {
                        let _ = client
                            .execute(&format!("DROP DATABASE {datname} WITH (FORCE)"), &[])
                            .await;
                    }
                });
            })
        {
            let _ = thread.join();
        }
    }
}

fn institution_connection(base: &str, datname: &str) -> String {
    if let Some(offset) = base.find("dbname=") {
        let value_start = offset + "dbname=".len();
        let value_end = base[value_start..]
            .find(char::is_whitespace)
            .map(|end| value_start + end)
            .unwrap_or(base.len());
        return format!("{}{}{}", &base[..value_start], datname, &base[value_end..]);
    }
    let (prefix, current_and_query) = base
        .rsplit_once('/')
        .unwrap_or_else(|| panic!("live Postgres connection has no dbname: {base}"));
    let suffix = current_and_query
        .find('?')
        .map(|offset| &current_and_query[offset..])
        .unwrap_or("");
    format!("{prefix}/{datname}{suffix}")
}

async fn raw_connect_institution(connection_string: &str) -> tokio_postgres::Client {
    let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
        .await
        .expect("institution live Postgres connection");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    client
}

fn institution_witness(index: u8, datname: String, fingerprint: String) -> InstitutionWitness {
    let authority = [index; 32];
    let signing_key = SigningKey::from_bytes(&authority);
    let host = format!("institution-{index:02}-host");
    let team = TeamId::new(&format!("institution-{index:02}-team"))
        .expect("canonical institution team");
    let manifest = CohortManifest {
        schema_version: COHORT_SCHEMA_V4,
        cohort_id: format!("cortex-institution-{index:02}"),
        version: 1,
        authority: CohortAuthority {
            threshold: 1,
            keys: vec![hex::encode(signing_key.verifying_key().to_bytes())],
        },
        members: vec![CohortMember {
            host_id: host.clone(),
            fingerprint,
            roles: vec!["worker".to_string()],
            team: Some(team.clone()),
        }],
        consent: ConsentMatrix::default(),
        reserved_intents: vec![
            RESERVED_INTENT_REISSUE.to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ],
        t_stale_secs: 120,
        teams: Some(vec![TeamEntry {
            team_id: team.clone(),
            region: Region::canonicalize("region-a").expect("canonical region"),
            datname: datname.clone(),
            members: vec![SpiritId::from(format!("institution-{index:02}-spirit"))],
        }]),
        signature: ManifestSignature { sig: String::new() },
        cross_team_consent: Vec::new(),
    }
    .signed_with(&signing_key);
    let signed_toml = toml::to_string(&manifest).expect("institution manifest serializes");
    let pins = PinnedAuthorityKeys::from_keys(vec![signing_key.verifying_key()])
        .expect("one operator-pinned institution authority");
    let state = Arc::new(
        CohortManifestState::load_with_clock(
            HostId(host.clone()),
            &signed_toml,
            pins,
            Arc::new(InMemoryCohortAuditSink::default()),
            Arc::new(FixedClock),
        )
        .expect("institution manifest verifies only under its pinned authority"),
    );
    InstitutionWitness {
        authority,
        datname,
        host,
        team,
        state,
    }
}

#[tokio::test]
#[ignore = "AdvisorySubstrate: requires MAOS_TEST_POSTGRES_TEAM_A and a Postgres role permitted to CREATE/DROP DATABASE"]
async fn cortex_fourteen_institution_isolation_live() {
    let _evidence = evidence_record::attest("cortex_fourteen_institution_isolation_live");
    let _guard = LIVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let base_connection = pg_conn_team("team-a");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let prefix = format!("maos_i136_{}_{nonce:x}", std::process::id());
    let datnames: Vec<String> = (1_u8..=14)
        .map(|index| format!("{prefix}_{index:02}"))
        .collect();
    assert!(
        datnames.iter().all(|datname| datname.len() <= 63),
        "the ephemeral institution datnames must remain valid PostgreSQL identifiers"
    );
    let mut database_guard = InstitutionDatabaseDropGuard::new(base_connection.clone());

    let admin = raw_connect_institution(&base_connection).await;
    for datname in &datnames {
        admin
            .execute(&format!("CREATE DATABASE {datname}"), &[])
            .await
            .unwrap_or_else(|error| panic!("provision institution datname {datname}: {error}"));
        database_guard.record_created(datname.clone());
    }

    let identity_fixture = tempfile::tempdir().expect("institution identities");
    let identities: Vec<DaemonIdentity> = (1_u8..=14)
        .map(|index| mint_daemon_identity(identity_fixture.path(), &format!("institution-{index:02}-host")))
        .collect();
    let mut witnesses: Vec<InstitutionWitness> = datnames
        .iter()
        .zip(&identities)
        .enumerate()
        .map(|(offset, (datname, identity))| {
            institution_witness(
                (offset + 1) as u8,
                datname.clone(),
                identity.fingerprint.to_string(),
            )
        })
        .collect();

    // The fourteen signed topology entries must bind to fourteen independently
    // reachable physical databases, not fourteen hosts under one authority.
    for witness in &witnesses {
        let connection = institution_connection(&base_connection, &witness.datname);
        let client = raw_connect_institution(&connection).await;
        let actual: String = client
            .query_one("SELECT current_database()", &[])
            .await
            .expect("read institution current_database")
            .get(0);
        assert_eq!(actual, witness.datname);
        let manifest = witness.state.manifest().expect("verified institution manifest");
        assert_eq!(
            manifest.team_of_host(&witness.host),
            Some(&witness.team),
            "the signed host→team binding must remain inside its institution"
        );
        assert_eq!(
            manifest.teams.as_ref().expect("institution team topology")[0].datname,
            witness.datname,
            "the signed team→datname binding must name the live institution database"
        );
    }

    let expected_authorities: BTreeSet<String> = witnesses
        .iter()
        .map(|witness| {
            hex::encode(SigningKey::from_bytes(&witness.authority).verifying_key().to_bytes())
        })
        .collect();
    let observed_authorities: BTreeSet<String> = witnesses
        .iter()
        .map(|witness| {
            witness
                .state
                .manifest()
                .expect("verified institution manifest")
                .authority
                .keys
                .into_iter()
                .next()
                .expect("one declared authority")
        })
        .collect();
    let identity_witnesses: BTreeSet<String> = witnesses
        .iter()
        .map(|witness| {
            witness
                .state
                .manifest()
                .expect("verified institution manifest")
                .members[0]
                .fingerprint
                .clone()
        })
        .collect();
    assert_eq!(
        identity_witnesses.len(),
        14,
        "each independent institution must retain a distinct certificate identity witness"
    );
    assert_eq!(expected_authorities.len(), 14, "fixture authorities must be unique");
    assert_eq!(
        observed_authorities, expected_authorities,
        "all fourteen and only fourteen operator-pinned authority witnesses must reconcile"
    );

    // A manifest is institution-local. A real crossing aimed at institution 1's
    // live database but authenticated as institution 2 reaches the production
    // applier and receives its typed consent refusal before any database write.
    let target = &witnesses[0];
    let foreign = &witnesses[1];
    let target_store = Arc::new(
        LoomLiteStore::new(StoreConfig {
            connection_string: institution_connection(&base_connection, &target.datname),
            home_region: "region-a".to_string(),
            home_team: target.team.as_str().to_string(),
            ..StoreConfig::default()
        })
        .await
        .expect("target institution live store")
        .with_cross_team_consent(Arc::new(
            maos_bin::cross_team_consent::CrossTeamConsentAdapter::new(Arc::clone(&target.state)),
        )),
    );
    let seed = [0x13; 32];
    let foreign_bundle = build_replication_bundle_v2(
        vec![crossing_leaf("cross-institution-must-refuse", "isolated")],
        &Region::canonicalize("region-a").expect("canonical region"),
        &foreign.team,
        &seed,
    )
    .expect("foreign institution can form a correctly signed bundle");
    let foreign_frame = crossing_frame(
        &control_from(),
        &HostId(target.host.clone()),
        14,
        &target.team,
        foreign_bundle,
    )
    .expect("cross-institution frame encodes");
    let target_applier =
        CrossTeamCrossingAdapter::new(target_store, target.team.clone(), seed);
    match target_applier.apply_crossing(foreign.team.as_str(), &foreign_frame).await {
        CrossingOutcome::Refused(CrossingRefusal::ConsentDenied {
            from_team,
            to_team,
            intent,
        }) => {
            assert_eq!(from_team, foreign.team.as_str());
            assert_eq!(to_team, target.team.as_str());
            assert_eq!(intent, "collective:share");
        }
        other => panic!(
            "a foreign institution's manifest must not authorize its crossing; got {other:?}"
        ),
    }

    // Proven-red clone control: transplant institution 1's authority into
    // institution 2's otherwise valid manifest and sign it with the donor.
    // Institution 2's original operator pin rejects the forged topology before
    // it can replace the independently reconciled witness.
    let mut cross_authority_clone = witnesses[1]
        .state
        .manifest()
        .expect("clone source manifest");
    let donor_key = SigningKey::from_bytes(&witnesses[0].authority);
    cross_authority_clone.authority.keys =
        vec![hex::encode(donor_key.verifying_key().to_bytes())];
    let clone_toml = toml::to_string(&cross_authority_clone.signed_with(&donor_key))
        .expect("cross-authority clone serializes");
    let recipient_key = SigningKey::from_bytes(&witnesses[1].authority);
    let recipient_pins = PinnedAuthorityKeys::from_keys(vec![recipient_key.verifying_key()])
        .expect("recipient's original authority pin");
    assert!(
        matches!(
            CohortManifestState::load_with_clock(
                HostId(witnesses[1].host.clone()),
                &clone_toml,
                recipient_pins,
                Arc::new(InMemoryCohortAuditSink::default()),
                Arc::new(FixedClock),
            ),
            Err(maos_cohort::CohortError::ECohortAuthorityUnpinned {
                unpinned_count: 1,
                ..
            })
        ),
        "proven-red: a donor authority swapped into another institution's manifest \
         must be rejected by that institution's original pin"
    );

    // Removing one independent institution releases only its own state. The
    // authority retires its original host identity by rotating to a fresh
    // retirement sentinel member and revoking every cross-team consent grant;
    // V4 manifests may not carry an empty member or team set, so retirement
    // is expressed as identity rotation, not truncation. The remaining
    // thirteen retain their original pin witness and deny the same
    // foreign-team consent decision, proving revocation/removal does not
    // mutate a shared authority or consent table.
    let removed = witnesses.remove(0);
    let removed_authority = hex::encode(
        SigningKey::from_bytes(&removed.authority)
            .verifying_key()
            .to_bytes(),
    );
    let original_host = removed.host.clone();
    let retired_identity = mint_daemon_identity(identity_fixture.path(), "institution-01-retired");
    let mut tombstone = removed.state.manifest().expect("institution to revoke");
    tombstone.version = 2;
    tombstone.members.clear();
    tombstone
        .members
        .push(maos_cohort::manifest::CohortMember {
            host_id: "institution-01-retired".to_string(),
            fingerprint: retired_identity.fingerprint.to_string(),
            roles: vec!["worker".to_string()],
            team: None,
        });
    tombstone.cross_team_consent.clear();
    let removal_key = SigningKey::from_bytes(&removed.authority);
    let tombstone_toml = toml::to_string(&tombstone.signed_with(&removal_key))
        .expect("institution removal manifest serializes");
    removed
        .state
        .issue_reissue(&tombstone_toml)
        .expect("institution authority accepts its own removal reissue");
    let retired_manifest = removed
        .state
        .manifest()
        .expect("removed institution manifest");
    assert!(
        retired_manifest.team_of_host(&original_host).is_none(),
        "the removed institution must no longer expose its original host identity"
    );
    assert!(
        retired_manifest.cross_team_consent.is_empty(),
        "the removed institution must no longer grant cross-team consent"
    );
    drop(removed);
    assert_eq!(witnesses.len(), 13);
    for witness in &witnesses {
        let manifest = witness.state.manifest().expect("remaining manifest");
        assert_eq!(
            manifest.authority.keys,
            vec![hex::encode(
                SigningKey::from_bytes(&witness.authority)
                    .verifying_key()
                    .to_bytes()
            )],
            "removing a peer institution must not alter another institution's pin"
        );
        assert_ne!(
            manifest.authority.keys[0], removed_authority,
            "a remaining institution must retain its own authority witness"
        );
        assert!(
            !manifest.cross_team_admits(
                &witness.team,
                &TeamId::new("institution-01-team").expect("canonical removed team"),
                "collective:share",
            ),
            "removing an institution must not create consent in a remaining institution"
        );
    }

    database_guard
        .remove_all()
        .await
        .unwrap_or_else(|error| panic!("remove institution datnames: {error}"));
    database_guard.disarm();
}
