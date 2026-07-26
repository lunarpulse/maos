//! Story 9.2 — GDPR Article 17 cascade corpus replay test.
//!
//! Replays the SHA-pinned `gdpr-cascade-v0.jsonl` corpus (50 scenarios) and
//! the independent `gdpr-cascade-probe-v0.jsonl` leakage probe (100 queries)
//! against the real kernel adapters in an isolated temp directory.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::Path;
use std::sync::Arc;

use maos_domain::distillation::{DigestPayload, DistillationRequest};
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::memory::{ForgetOutcome, MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::{DistillationPort, MemoryManagerPort};
use serde::Deserialize;
use tempfile::TempDir;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn corpus_path(name: &str) -> std::path::PathBuf {
    repo_root().join("tests").join("corpora").join(name)
}

#[derive(Debug, Deserialize)]
struct GdprCascadeScenario {
    scenario_id: String,
    principal: String,
    spirit_pid: u32,
    secondary_spirit_pid: Option<u32>,
    #[allow(dead_code)]
    boot_nonce: u64,
    schema: String,
    key: String,
    value: String,
    canary: Option<String>,
    distillate_embedded: bool,
    legal_hold_reason: Option<String>,
    expected_outcome: String,
    reused_pid: bool,
    reused_principal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GdprLeakageProbe {
    #[allow(dead_code)]
    probe_id: String,
    principal: String,
    spirit_pid: u32,
    #[allow(dead_code)]
    query_type: String,
    expected_subject_access_len: usize,
    #[allow(dead_code)]
    canary: Option<String>,
}

fn open_isolated_stores(
    dir: &TempDir,
) -> (
    Arc<maos_kernel_core::memory::PrivateMemoryStore>,
    Arc<maos_kernel_core::memory::SharedMemoryStore>,
    Arc<maos_kernel_core::memory::PrincipalNamespaceIndex>,
    Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    std::path::PathBuf,
) {
    let fs_root = dir.path().join("memory");
    std::fs::create_dir_all(&fs_root).unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let private = Arc::new(maos_kernel_core::memory::PrivateMemoryStore::new(
        fs_root,
        4 * 1024,
    ));
    let shared = Arc::new(maos_kernel_core::memory::SharedMemoryStore::open(&db_path).unwrap());
    let principal_index =
        Arc::new(maos_kernel_core::memory::PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open_with_global_legal_holds(
            &db_path, &db_path, 1,
        )
        .unwrap(),
    );
    (private, shared, principal_index, tl, db_path)
}

fn make_memory_adapter(
    private: Arc<maos_kernel_core::memory::PrivateMemoryStore>,
    shared: Arc<maos_kernel_core::memory::SharedMemoryStore>,
    principal_index: Arc<maos_kernel_core::memory::PrincipalNamespaceIndex>,
    tl: Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
) -> Arc<maos_kernel_core::memory::MemoryManagerAdapter> {
    Arc::new(maos_kernel_core::memory::MemoryManagerAdapter::new(
        private,
        shared,
        principal_index,
        tl,
    ))
}

fn write_principal_data(
    memory: &Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    spirit_pid: u32,
    principal: &str,
    schema: &str,
    key: &str,
    value: &str,
) {
    let ns = MemoryNamespace::Principal {
        principal_id: principal.into(),
        schema: schema.into(),
    };
    memory
        .write(
            spirit_pid,
            MemoryTier::Private,
            &ns,
            key,
            MemoryValue::Text(value.into()),
        )
        .unwrap();
}

fn insert_source_frame(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    spirit_pid: u32,
    payload: &str,
) -> [u8; 16] {
    tl.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete,
        spirit_pid,
        None,
        "task.complete",
        payload.as_bytes(),
        FrameOrigin::Kernel,
    );
    tl.last_frame_id()
}

fn write_distillate_with_canary(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    spirit_pid: u32,
    source_frame_id: [u8; 16],
    principal: &str,
    canary: &str,
) -> [u8; 16] {
    let writer = maos_kernel_core::iac::distillate::DistillateWriter::new(
        Arc::clone(tl),
        Arc::new(()) as Arc<dyn std::any::Any + Send + Sync>,
    );
    // Embed the principal_id in the distillate body so the forget cascade's
    // content-based filter (P3) can link this distillate to its subject and
    // scrub it.  A real distillate references its subject principal.
    let body = format!("{canary} principal={principal}");
    let request =
        DistillationRequest::new(vec![source_frame_id], 1, DigestPayload::Text(body), None)
            .unwrap();
    let receipt = writer.write_distillate(spirit_pid, request).unwrap();
    receipt.digest_frame_id
}

fn count_redaction_markers(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    principal: &str,
) -> usize {
    let entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete),
            ..Default::default()
        })
        .unwrap();
    entries
        .into_iter()
        .filter(|e| {
            e.intent == "distillate.redacted"
                && String::from_utf8_lossy(&e.payload_redacted)
                    .contains(&format!("\"principal_id\":\"{}\"", principal))
        })
        .count()
}

fn canary_survives_in_distillate_bodies(
    tl: &Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    canary: &str,
) -> bool {
    // P21: scans the RAW stored `payload_redacted` bytes of every Distillate
    // frame — the literal serialized distillate body, not a secondary index or
    // a redacted-on-read projection.  This is the on-disk reachable form for
    // the SQLite-backed TL; SQLite free-page ghosts (old page bytes surviving
    // an UPDATE until VACUUM/secure-delete) are a storage-layer concern outside
    // the cascade's reach.
    let entries = tl
        .query_frames(maos_kernel_core::iac::transparency_log::FrameFilter {
            kind: Some(maos_kernel_core::iac::transparency_log::FrameKind::Distillate),
            ..Default::default()
        })
        .unwrap();
    entries
        .iter()
        .any(|e| String::from_utf8_lossy(&e.payload_redacted).contains(canary))
}

#[test]
fn gdpr_cascade_v0_corpus_replay() {
    let dir = TempDir::new().unwrap();
    let (private, shared, principal_index, tl, db_path) = open_isolated_stores(&dir);
    let memory = make_memory_adapter(private, shared, principal_index, tl.clone());

    let corpus_file = std::fs::File::open(corpus_path("gdpr-cascade-v0.jsonl")).unwrap();
    let scenarios: Vec<GdprCascadeScenario> = std::io::BufReader::new(corpus_file)
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(scenarios.len(), 50, "corpus must contain 50 scenarios");
    let scenario_ids: BTreeSet<_> = scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect();
    let duplicate_ids: BTreeSet<_> = scenarios
        .iter()
        .filter(|scenario| {
            scenarios
                .iter()
                .filter(|candidate| candidate.scenario_id == scenario.scenario_id)
                .count()
                > 1
        })
        .map(|scenario| scenario.scenario_id.as_str())
        .collect();
    assert_eq!(
        scenario_ids.len(),
        scenarios.len(),
        "corpus scenario_id values must be unique; duplicate(s): {duplicate_ids:?}"
    );

    for scenario in &scenarios {
        // The terminal corpus includes a true empty subject and a deterministic
        // filesystem failure in addition to the ordinary/held paths.
        if scenario.expected_outcome == "failed" {
            let namespace = MemoryNamespace::Principal {
                principal_id: scenario.principal.clone(),
                schema: scenario.schema.clone(),
            };
            memory
                .write(
                    scenario.spirit_pid,
                    MemoryTier::Private,
                    &namespace,
                    &scenario.key,
                    MemoryValue::Blob(vec![0x5b; 8 * 1024]),
                )
                .unwrap();
        } else if scenario.expected_outcome != "not_found" {
            write_principal_data(
                &memory,
                scenario.spirit_pid,
                &scenario.principal,
                &scenario.schema,
                &scenario.key,
                &scenario.value,
            );
            if let Some(secondary) = scenario.secondary_spirit_pid {
                write_principal_data(
                    &memory,
                    secondary,
                    &scenario.principal,
                    &scenario.schema,
                    &format!("{}-secondary", scenario.key),
                    &format!("{}-secondary", scenario.value),
                );
            }
        }
        if scenario.expected_outcome == "failed" {
            let pid_dir = dir
                .path()
                .join("memory")
                .join(scenario.spirit_pid.to_string());
            let namespace_dir = std::fs::read_dir(&pid_dir)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            std::fs::remove_dir_all(&namespace_dir).unwrap();
            std::fs::write(&namespace_dir, b"simulated removal failure").unwrap();
        }

        // Distillate-embedded canary: plant a canary in a source frame and embed
        // it into a distillate body authored by the same Spirit.
        if let Some(canary) = &scenario.canary {
            assert!(scenario.distillate_embedded);
            let source = insert_source_frame(&tl, scenario.spirit_pid, canary);
            write_distillate_with_canary(
                &tl,
                scenario.spirit_pid,
                source,
                &scenario.principal,
                canary,
            );
        }

        let reason = scenario.legal_hold_reason.as_deref();
        let outcome = memory.forget_with_reason(&scenario.principal, reason);

        match scenario.expected_outcome.as_str() {
            "erased" => match outcome.unwrap() {
                ForgetOutcome::Erased { receipt, .. } => {
                    if scenario.reused_pid {
                        assert_eq!(
                            receipt.deleted_entries, 1,
                            "{}: pid-reuse erase should delete exactly the original row",
                            scenario.scenario_id
                        );
                    }
                }
                other => panic!("{}: expected Erased, got {:?}", scenario.scenario_id, other),
            },
            "held" => match outcome.unwrap() {
                ForgetOutcome::Suspended { hold } => {
                    assert!(hold.reason.starts_with("legal-hold"));
                    assert!(hold.status.contains("SUSPENDED"));
                }
                other => panic!("{}: expected Held, got {:?}", scenario.scenario_id, other),
            },
            "not_found" => match outcome.unwrap() {
                ForgetOutcome::Erased { receipt, .. } => {
                    assert_eq!(receipt.deleted_entries, 0);
                    assert_eq!(receipt.deleted_index_rows, 0);
                }
                other => panic!(
                    "{}: expected NotFound, got {:?}",
                    scenario.scenario_id, other
                ),
            },
            "failed" => {
                let error = outcome.expect_err("forced filesystem failure must propagate");
                assert!(
                    error.to_string().contains("directory")
                        || error.to_string().contains("Directory")
                );
            }
            other => panic!(
                "{}: unknown expected_outcome {}",
                scenario.scenario_id, other
            ),
        }

        if matches!(scenario.expected_outcome.as_str(), "erased" | "not_found") {
            let rows = maos_audit::subject_access_query(&db_path, &scenario.principal).unwrap();
            assert!(
                rows.is_empty(),
                "{}: forgotten principal must have empty subject access",
                scenario.scenario_id
            );
        } else {
            let rows = maos_audit::subject_access_query(&db_path, &scenario.principal).unwrap();
            assert!(
                !rows.is_empty(),
                "{}: legal-hold must retain subject-access rows",
                scenario.scenario_id
            );
        }

        if scenario.distillate_embedded {
            let canary = scenario.canary.as_ref().unwrap();
            assert!(
                !canary_survives_in_distillate_bodies(&tl, canary),
                "{}: canary survived in distillate body",
                scenario.scenario_id
            );
            assert_eq!(
                count_redaction_markers(&tl, &scenario.principal),
                1,
                "{}: expected exactly one redaction marker",
                scenario.scenario_id
            );
        }

        if scenario.reused_pid {
            let reused_principal = scenario.reused_principal.as_ref().unwrap();
            write_principal_data(
                &memory,
                scenario.spirit_pid,
                reused_principal,
                &scenario.schema,
                &scenario.key,
                &scenario.value,
            );
            let reused_rows = maos_audit::subject_access_query(&db_path, reused_principal).unwrap();
            assert_eq!(
                reused_rows.len(),
                1,
                "{}: reused principal must survive pid reuse",
                scenario.scenario_id
            );
            // P14: the ORIGINAL principal must remain unreachable after the pid
            // is reused for a new subject — the erased state survives reuse.
            // (Full boot-A/boot-B lifecycle modeling awaits a boot-nonce API on
            // forget; this asserts the cross-reuse invariant that is reachable
            // today.)
            let original_after =
                maos_audit::subject_access_query(&db_path, &scenario.principal).unwrap();
            assert!(
                original_after.is_empty(),
                "{}: erased principal must remain unreachable after pid reuse",
                scenario.scenario_id
            );
        }
    }
}
#[test]
fn gdpr_cascade_probe_v0_leakage_check() {
    let dir = TempDir::new().unwrap();
    let (private, shared, principal_index, tl, db_path) = open_isolated_stores(&dir);
    let memory = make_memory_adapter(private, shared, principal_index, tl.clone());

    let probe_file = std::fs::File::open(corpus_path("gdpr-cascade-probe-v0.jsonl")).unwrap();
    let probes: Vec<GdprLeakageProbe> = std::io::BufReader::new(probe_file)
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    assert_eq!(probes.len(), 100, "probe must contain 100 queries");

    // P13 (Murat anti-tautology): the probe must exercise the forget cascade,
    // not just verify write/read.  Each control principal is written, verified
    // at its expected pre-forget length, then FORGOTTEN — the probe then
    // asserts erasure from an angle the cascade actually touched.  Phantom
    // principals are never written and must remain empty.
    for probe in &probes {
        if probe.query_type == "control_principal" {
            write_principal_data(
                &memory,
                probe.spirit_pid,
                &probe.principal,
                "chat",
                "msg1",
                "control value",
            );
            // Pre-forget: subject access finds the row.
            let rows = maos_audit::subject_access_query(&db_path, &probe.principal).unwrap();
            assert_eq!(
                rows.len(),
                probe.expected_subject_access_len,
                "{}: pre-forget subject-access mismatch for {}",
                probe.probe_id,
                probe.principal
            );
            // Drive the cascade the original test skipped.
            let outcome = memory.forget_with_reason(&probe.principal, None).unwrap();
            assert!(
                matches!(outcome, ForgetOutcome::Erased { .. }),
                "{}: control principal must erase",
                probe.probe_id
            );
            let after = maos_audit::subject_access_query(&db_path, &probe.principal).unwrap();
            assert!(
                after.is_empty(),
                "{}: forgotten control principal must have empty subject access",
                probe.probe_id
            );
        } else {
            // Phantom principal: never written, must be empty.
            let rows = maos_audit::subject_access_query(&db_path, &probe.principal).unwrap();
            assert_eq!(
                rows.len(),
                probe.expected_subject_access_len,
                "{}: phantom subject-access mismatch for {}",
                probe.probe_id,
                probe.principal
            );
        }
    }

    // No probe canary token should appear in any distillate body after the
    // cascade.  For probes carrying a canary, plant a distillate embedding it,
    // forget, then assert the canary is gone (the non-tautological canary gate).
    for probe in &probes {
        if probe.query_type == "control_principal" {
            if let Some(canary) = &probe.canary {
                write_principal_data(
                    &memory,
                    probe.spirit_pid,
                    &probe.principal,
                    "chat",
                    "canary-msg",
                    "control value",
                );
                let source = insert_source_frame(&tl, probe.spirit_pid, canary);
                write_distillate_with_canary(
                    &tl,
                    probe.spirit_pid,
                    source,
                    &probe.principal,
                    canary,
                );
                memory.forget_with_reason(&probe.principal, None).unwrap();
                assert!(
                    !canary_survives_in_distillate_bodies(&tl, canary),
                    "{}: probe canary leaked into distillate body after forget",
                    probe.probe_id
                );
            }
        }
    }
}
