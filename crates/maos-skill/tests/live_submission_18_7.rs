//! Story 18.7 D-03..06/16: real payloads, typed refusal, distinct identity.
use maos_domain::self_telemetry::SelfTelemetryReport;
use maos_skill::*;
use std::collections::BTreeSet;
struct Author {
    identity: AuthorIdentity,
    actions: BTreeSet<String>,
    principals: Vec<String>,
}
impl Author {
    fn new(class: &str, pid: u32) -> Self {
        Self {
            identity: AuthorIdentity {
                class: class.into(),
                pid,
                boot_nonce: "boot-one".into(),
            },
            actions: BTreeSet::from(["skill.author.self".into()]),
            principals: vec!["alice@example.test".into()],
        }
    }
    fn ctx(&self) -> SubmissionContext<'_> {
        SubmissionContext {
            author: &self.identity,
            declared_actions: &self.actions,
            principal_ids: &self.principals,
        }
    }
}
fn document(id: &str, version: &str, body: &str) -> String {
    format!("---\nid='{id}'\nversion='{version}'\nname='Research'\ndescription='d'\nrequired_capabilities=['skill.author.self']\n---\n{body}\n")
}
fn proposal(pid: u32, version: &str, diff: &str) -> SkillRevisionProposal {
    build_proposal(
        SkillId::from("researcher.core"),
        SkillVersion::from(version),
        diff.into(),
        SelfTelemetryReport::new(pid, 10, 20, 3, 1, 0, 0, 0, vec![], vec![], 21).unwrap(),
    )
    .unwrap()
}
fn admitted(q: &mut SkillAdmissionQueue, author: &Author, directory: &std::path::Path) -> String {
    let (entry, _) = q
        .prepare_author(
            document("researcher.core", "1.0.0", "Original body"),
            &author.ctx(),
            &[],
        )
        .unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    q.prepare_decision(&key, true, "operator".into(), Some(directory))
        .unwrap();
    q.apply_decision(&key, true, Some(directory)).unwrap();
    key
}
fn refused(
    q: &SkillAdmissionQueue,
    result: Result<(QueueEntry, maos_domain::invariants::i4::ApprovalDecision), ESkillStore>,
    expected: ESkillQueue,
) {
    let before = q.entries().len();
    assert!(matches!(&result, Err(ESkillStore::InvalidEntry(error)) if error == &expected));
    assert_eq!(q.entries().len(), before);
}
#[test]
fn vetting_refuses_foreign_namespace_capability_escalation_and_malformed_document() {
    let q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    refused(
        &q,
        q.prepare_author(document("butler.core", "1.0.0", "Body"), &author.ctx(), &[]),
        ESkillQueue::ForeignNamespace,
    );
    let escalated =
        document("researcher.core", "1.0.0", "Body").replace("skill.author.self", "memory.write");
    refused(
        &q,
        q.prepare_author(escalated, &author.ctx(), &[]),
        ESkillQueue::CapabilityEscalation,
    );
    assert!(matches!(
        q.prepare_author("not a document".into(), &author.ctx(), &[]),
        Err(ESkillStore::InvalidEntry(ESkillQueue::Schema(
            ESkillSchema::MissingFence
        )))
    ));
    assert!(q.entries().is_empty());
}
#[test]
fn principal_matching_is_token_bounded_in_both_document_and_diff() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    refused(
        &q,
        q.prepare_author(
            document("researcher.core", "1.0.0", "Cite alice@example.test."),
            &author.ctx(),
            &[],
        ),
        ESkillQueue::PrincipalDataRefused,
    );
    assert!(q
        .prepare_author(
            document(
                "researcher.core",
                "1.0.0",
                "alice@example.test.other is a different token"
            ),
            &author.ctx(),
            &[],
        )
        .is_ok());
    let dir = tempfile::tempdir().unwrap();
    admitted(&mut q, &author, dir.path());
    refused(
        &q,
        q.prepare_proposal(proposal(7, "1.0.0", "+alice@example.test\n"), &author.ctx()),
        ESkillQueue::PrincipalDataRefused,
    );
    let (entry, row) = q
        .prepare_proposal(proposal(7, "1.0.0", "+generic advice"), &author.ctx())
        .unwrap();
    assert!(!format!("{row:?}").contains("alice@example.test"));
    assert!(!format!("{row:?}").contains("generic advice"));
    assert_eq!(entry.author.as_ref().unwrap(), &author.identity);
}
#[test]
fn proposal_requires_owned_current_admitted_author_revision_and_nonempty_diff() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    refused(
        &q,
        q.prepare_proposal(proposal(7, "1.0.0", "diff"), &author.ctx()),
        ESkillQueue::TargetNotOwned,
    );
    let dir = tempfile::tempdir().unwrap();
    admitted(&mut q, &author, dir.path());
    let foreign = Author::new("butler", 8);
    refused(
        &q,
        q.prepare_proposal(proposal(8, "1.0.0", "diff"), &foreign.ctx()),
        ESkillQueue::TargetNotOwned,
    );
    refused(
        &q,
        q.prepare_proposal(proposal(7, "0.9.0", "diff"), &author.ctx()),
        ESkillQueue::StaleTargetVersion,
    );
    let mut empty = proposal(7, "1.0.0", "diff");
    empty.proposed_diff = " \n".into();
    refused(
        &q,
        q.prepare_proposal(empty, &author.ctx()),
        ESkillQueue::Proposal(ESkillProposal::EmptyDiff),
    );
    refused(
        &q,
        q.prepare_proposal(proposal(99, "1.0.0", "diff"), &author.ctx()),
        ESkillQueue::EvidenceMismatch,
    );
    assert_eq!(q.entries().len(), 1);
}
#[test]
fn prepare_is_side_effect_free_and_enqueue_receipt_addresses_exact_payload() {
    use sha2::{Digest, Sha256};
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let (entry, row) = q
        .prepare_author(
            document("researcher.core", "1.0.0", "Body"),
            &author.ctx(),
            &[],
        )
        .unwrap();
    assert_eq!(q.pending_count(), 0);
    let expected = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&(&entry.author, &entry.payload)).unwrap())
    );
    assert_eq!(entry.payload_sha256, expected);
    assert_eq!(entry.entry_id, expected[..16]);
    assert_eq!(row.actor, "spirit:researcher@7");
    assert_eq!(row.target, "researcher.core@1.0.0");
    assert!(row
        .reasoning
        .as_ref()
        .unwrap()
        .contains(&entry.payload_sha256));
    assert!(!row.reasoning.as_ref().unwrap().contains("Body"));
    q.commit_prepared(entry.clone()).unwrap();
    assert_eq!(
        q.show(&entry.entry_id).unwrap().state,
        SkillAdmissionState::Pending
    );
    assert_eq!(q.own_entries("researcher").len(), 1);
    assert!(q.own_entries("butler").is_empty());
}
#[test]
fn two_revision_authors_coexist_and_one_decision_changes_neither_base_nor_sibling() {
    let mut q = SkillAdmissionQueue::new();
    let first = Author::new("researcher", 7);
    let second = Author::new("researcher", 8);
    let dir = tempfile::tempdir().unwrap();
    let base = admitted(&mut q, &first, dir.path());
    let (one, _) = q
        .prepare_proposal(proposal(7, "1.0.0", "diff one"), &first.ctx())
        .unwrap();
    let one_key = one.entry_id.clone();
    q.commit_prepared(one).unwrap();
    assert!(matches!(
        q.prepare_proposal(proposal(7, "1.0.0", "another diff"), &first.ctx()),
        Err(ESkillStore::InvalidEntry(ESkillQueue::DuplicateSkillId(_)))
    ));
    let (two, _) = q
        .prepare_proposal(proposal(8, "1.0.0", "diff two"), &second.ctx())
        .unwrap();
    let two_key = two.entry_id.clone();
    q.commit_prepared(two).unwrap();
    assert_ne!(one_key, two_key);
    assert!(matches!(
        q.show("researcher.core"),
        Err(ESkillQueue::AmbiguousEntry)
    ));
    let row = q
        .prepare_decision(&one_key, true, "operator".into(), None)
        .unwrap();
    assert_eq!(row.target, format!("researcher.core@1.0.0#{one_key}"));
    q.apply_decision(&one_key, true, None).unwrap();
    assert_eq!(q.show(&base).unwrap().state, SkillAdmissionState::Admitted);
    assert_eq!(
        q.show(&two_key).unwrap().state,
        SkillAdmissionState::Pending
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("researcher.core.md")).unwrap(),
        document("researcher.core", "1.0.0", "Original body")
    );
}
#[test]
fn full_document_diff_evidence_and_provenance_survive_restart() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let dir = tempfile::tempdir().unwrap();
    admitted(&mut q, &author, dir.path());
    let payload = proposal(7, "1.0.0", "--- old\n+++ new\n@@\n-old\n+new\n");
    let (entry, _) = q.prepare_proposal(payload.clone(), &author.ctx()).unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    let store = LocalFsSkillQueueStore::at_path(dir.path().join("queue.json"));
    store.save(&q.to_stored()).unwrap();
    let restored = SkillAdmissionQueue::from_stored(store.load().unwrap()).unwrap();
    assert_eq!(restored.entries(), q.entries());
    assert_eq!(
        restored.show(&key).unwrap().payload,
        Some(SkillPayload::RevisionProposal(payload))
    );
    let mut tampered = q.to_stored();
    tampered[1].payload = None;
    assert!(matches!(
        SkillAdmissionQueue::from_stored(tampered),
        Err(ESkillStore::InvalidEntry(ESkillQueue::InvalidStoredEntry(
            _
        )))
    ));
}
#[test]
fn admission_materializes_only_after_decision_and_foreign_files_never_overwrite() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("researcher.core.md");
    let text = document("researcher.core", "1.0.0", "Body");
    let (entry, _) = q.prepare_author(text.clone(), &author.ctx(), &[]).unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    q.prepare_decision(&key, true, "operator".into(), Some(dir.path()))
        .unwrap();
    assert!(!dest.exists());
    std::fs::write(&dest, &text).unwrap(); // Even byte-equal but unowned is foreign.
    assert!(matches!(
        q.prepare_decision(&key, true, "operator".into(), Some(dir.path())),
        Err(ESkillStore::MaterializeCollision(_))
    ));
    assert!(q.audit_trail().is_empty());
    assert!(q
        .prepare_decision(&key, false, "operator".into(), None)
        .is_ok());
    assert!(matches!(
        q.apply_decision(&key, true, Some(dir.path())),
        Err(ESkillStore::MaterializeCollision(_))
    ));
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), text);
    assert_eq!(q.show(&key).unwrap().state, SkillAdmissionState::Pending);
    std::fs::remove_file(&dest).unwrap();
    q.prepare_decision(&key, true, "operator".into(), Some(dir.path()))
        .unwrap();
    q.apply_decision(&key, true, Some(dir.path())).unwrap();
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), text);
}
#[test]
fn authored_next_version_replaces_only_known_owned_bytes_and_advances_target() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let dir = tempfile::tempdir().unwrap();
    admitted(&mut q, &author, dir.path());
    refused(
        &q,
        q.prepare_author(
            document("researcher.core", "1.0.0", "Body"),
            &author.ctx(),
            &[],
        ),
        ESkillQueue::StaleTargetVersion,
    );
    let next = document("researcher.core", "2.0.0", "New body");
    let (entry, _) = q.prepare_author(next.clone(), &author.ctx(), &[]).unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    q.prepare_decision(&key, true, "operator".into(), Some(dir.path()))
        .unwrap();
    q.apply_decision(&key, true, Some(dir.path())).unwrap();
    refused(
        &q,
        q.prepare_proposal(proposal(7, "1.0.0", "diff"), &author.ctx()),
        ESkillQueue::StaleTargetVersion,
    );
    assert!(q
        .prepare_proposal(proposal(7, "2.0.0", "diff"), &author.ctx())
        .is_ok());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("researcher.core.md")).unwrap(),
        next
    );
}

#[test]
fn refusal_codes_never_include_dynamic_error_details() {
    let principal = "alice@example.test";
    let failures = [
        ESkillQueue::Schema(ESkillSchema::TomlParse(principal.into())),
        ESkillQueue::UnknownEntryPath(principal.into()),
        ESkillQueue::InvalidStoredEntry(principal.into()),
        ESkillQueue::DuplicateSkillId(principal.into()),
        ESkillQueue::PrincipalDataRefused,
    ];
    for failure in failures {
        assert!(!failure.code().contains(principal));
    }
    assert_eq!(
        ESkillQueue::PrincipalDataRefused.code(),
        "principal-data-refused"
    );
}

#[test]
fn authored_version_must_exceed_every_non_revision_history_in_every_state() {
    let author = Author::new("researcher", 7);
    for state in [
        SkillAdmissionState::Pending,
        SkillAdmissionState::Admitted,
        SkillAdmissionState::Rejected,
    ] {
        for source in ["author", "package", "legacy"] {
            let q = SkillAdmissionQueue::new();
            let mut earlier = if source != "author" {
                QueueEntry::package_shipped("researcher.core".into(), "3.0.0".into()).unwrap()
            } else {
                q.prepare_author(
                    document("researcher.core", "3.0.0", "Earlier body"),
                    &author.ctx(),
                    &[],
                )
                .unwrap()
                .0
            };
            if source == "legacy" {
                earlier.entry_path = "author_self".into();
                earlier.legacy = true;
                earlier.seal().unwrap();
            }
            earlier.state = state;
            let q = SkillAdmissionQueue::from_stored(vec![earlier]).unwrap();
            for version in ["2.0.0", "3.0.0", "3.0.0+different-build"] {
                refused(
                    &q,
                    q.prepare_author(
                        document("researcher.core", version, "Different body"),
                        &author.ctx(),
                        &[],
                    ),
                    ESkillQueue::StaleTargetVersion,
                );
            }
            if state != SkillAdmissionState::Pending {
                assert!(q
                    .prepare_author(
                        document("researcher.core", "3.0.1", "Advancing body"),
                        &author.ctx(),
                        &[],
                    )
                    .is_ok());
            }
        }
    }
}

#[test]
fn discovered_package_versions_are_checked_even_without_queue_entries() {
    let root = tempfile::tempdir().unwrap();
    for (file, id, version) in [
        ("low.md", "researcher.core", "1.0.0"),
        ("high.md", "researcher.core", "4.0.0"),
        ("other.md", "researcher.other", "99.0.0"),
    ] {
        std::fs::write(root.path().join(file), document(id, version, "Package")).unwrap();
    }
    let discovered = discover_skills(&[root.path().to_path_buf()]);
    assert_eq!(discovered.len(), 3);
    let q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    for version in ["0.9.0", "2.0.0", "4.0.0"] {
        refused(
            &q,
            q.prepare_author(
                document("researcher.core", version, "Authored"),
                &author.ctx(),
                &discovered,
            ),
            ESkillQueue::StaleTargetVersion,
        );
    }
    assert!(q
        .prepare_author(
            document("researcher.core", "4.0.1", "Authored"),
            &author.ctx(),
            &discovered,
        )
        .is_ok());
    assert!(q.entries().is_empty());
}

#[test]
fn rejected_authored_text_never_becomes_a_revision_base_or_reusable_version() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let root = tempfile::tempdir().unwrap();
    admitted(&mut q, &author, root.path());
    let rejected = document("researcher.core", "2.0.0", "Rejected instructions");
    let (entry, _) = q.prepare_author(rejected, &author.ctx(), &[]).unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    q.prepare_decision(&key, false, "operator".into(), None)
        .unwrap();
    q.apply_decision(&key, false, None).unwrap();
    refused(
        &q,
        q.prepare_author(
            document("researcher.core", "2.0.0", "Changed rejected instructions"),
            &author.ctx(),
            &[],
        ),
        ESkillQueue::StaleTargetVersion,
    );
    refused(
        &q,
        q.prepare_author(
            document("researcher.core", "1.5.0", "Lower"),
            &author.ctx(),
            &[],
        ),
        ESkillQueue::StaleTargetVersion,
    );
    refused(
        &q,
        q.prepare_proposal(proposal(7, "2.0.0", "diff"), &author.ctx()),
        ESkillQueue::StaleTargetVersion,
    );
    assert!(q
        .prepare_proposal(proposal(7, "1.0.0", "diff"), &author.ctx())
        .is_ok());
    assert_eq!(
        std::fs::read_to_string(root.path().join("researcher.core.md")).unwrap(),
        document("researcher.core", "1.0.0", "Original body"),
    );
    let only_rejected =
        SkillAdmissionQueue::from_stored(vec![q.show(&key).unwrap().clone()]).unwrap();
    refused(
        &only_rejected,
        only_rejected.prepare_proposal(proposal(7, "2.0.0", "diff"), &author.ctx()),
        ESkillQueue::TargetNotOwned,
    );
}

#[test]
fn authored_ids_have_the_same_128_byte_boundary_as_the_operator_door() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let id = format!("researcher.{}", "a".repeat(128 - "researcher.".len()));
    let (entry, _) = q
        .prepare_author(document(&id, "1.0.0", "Body"), &author.ctx(), &[])
        .unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry.clone()).unwrap();
    let root = tempfile::tempdir().unwrap();
    q.prepare_decision(&key, true, "operator".into(), Some(root.path()))
        .unwrap();
    assert!(!root.path().join(format!("{id}.md")).exists());
    let too_long = format!("{id}a");
    refused(
        &q,
        q.prepare_author(document(&too_long, "1.0.0", "Body"), &author.ctx(), &[]),
        ESkillQueue::AuthoredIdTooLong,
    );
    let mut stored = entry;
    stored.id = SkillId(too_long.clone());
    stored.payload = Some(SkillPayload::AuthorSelf(document(
        &too_long, "1.0.0", "Body",
    )));
    stored.seal().unwrap();
    assert!(matches!(
        SkillAdmissionQueue::from_stored(vec![stored]),
        Err(ESkillStore::InvalidEntry(ESkillQueue::AuthoredIdTooLong))
    ));
}

#[test]
fn decision_preflight_refuses_missing_home_and_non_directory_destinations_without_rows() {
    let mut q = SkillAdmissionQueue::new();
    let author = Author::new("researcher", 7);
    let (entry, _) = q
        .prepare_author(
            document("researcher.core", "1.0.0", "Body"),
            &author.ctx(),
            &[],
        )
        .unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    assert!(matches!(
        q.prepare_decision(&key, true, "operator".into(), None),
        Err(ESkillStore::NoDataHome)
    ));
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("not-a-directory");
    std::fs::write(&file, "foreign").unwrap();
    for directory in [file.clone(), file.join("nested")] {
        assert!(matches!(
            q.prepare_decision(&key, true, "operator".into(), Some(&directory)),
            Err(ESkillStore::Io(_))
        ));
    }
    let dest = root.path().join("researcher.core.md");
    std::fs::create_dir(&dest).unwrap();
    assert!(matches!(
        q.prepare_decision(&key, true, "operator".into(), Some(root.path())),
        Err(ESkillStore::Io(_))
    ));
    let absent = root.path().join("absent/nested");
    q.prepare_decision(&key, true, "operator".into(), Some(&absent))
        .unwrap();
    assert!(!absent.exists());
    assert!(q.audit_trail().is_empty());
    assert_eq!(q.show(&key).unwrap().state, SkillAdmissionState::Pending);
}

#[test]
fn decision_preflight_requires_exact_admitted_bytes_and_class_ownership() {
    let root = tempfile::tempdir().unwrap();
    let first = Author::new("researcher", 7);
    let nested = Author::new("researcher.lab", 8);
    let mut q = SkillAdmissionQueue::new();
    let original = document("researcher.lab.core", "1.0.0", "Original");
    let (entry, _) = q
        .prepare_author(original.clone(), &first.ctx(), &[])
        .unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    q.apply_decision(&key, true, Some(root.path())).unwrap();
    let (entry, _) = q
        .prepare_author(
            document("researcher.lab.core", "2.0.0", "New"),
            &nested.ctx(),
            &[],
        )
        .unwrap();
    let key = entry.entry_id.clone();
    q.commit_prepared(entry).unwrap();
    assert!(matches!(
        q.prepare_decision(&key, true, "operator".into(), Some(root.path())),
        Err(ESkillStore::MaterializeCollision(_))
    ));
    let dest = root.path().join("researcher.lab.core.md");
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), original);
    let mut owned = SkillAdmissionQueue::new();
    admitted(&mut owned, &first, root.path());
    let (entry, _) = owned
        .prepare_author(
            document("researcher.core", "2.0.0", "New"),
            &first.ctx(),
            &[],
        )
        .unwrap();
    let key = entry.entry_id.clone();
    owned.commit_prepared(entry).unwrap();
    std::fs::write(root.path().join("researcher.core.md"), "modified bytes").unwrap();
    assert!(matches!(
        owned.prepare_decision(&key, true, "operator".into(), Some(root.path())),
        Err(ESkillStore::MaterializeCollision(_))
    ));
}

#[test]
fn nonfinite_telemetry_values_and_thresholds_are_typed_submission_refusals() {
    use maos_domain::self_telemetry::HaltTelemetryEntry;
    let root = tempfile::tempdir().unwrap();
    let author = Author::new("researcher", 7);
    let mut q = SkillAdmissionQueue::new();
    admitted(&mut q, &author, root.path());
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for threshold in [false, true] {
            let mut p = proposal(7, "1.0.0", "diff");
            p.telemetry_evidence.halt_events.push(
                HaltTelemetryEntry::new(
                    "halt",
                    "tag",
                    "predicate",
                    if threshold { 0.5 } else { bad },
                    if threshold { Some(bad) } else { Some(1.0) },
                    15,
                    None,
                )
                .unwrap(),
            );
            refused(
                &q,
                q.prepare_proposal(p, &author.ctx()),
                ESkillQueue::NonFiniteEvidence,
            );
        }
    }
    let mut finite = proposal(7, "1.0.0", "diff");
    finite.telemetry_evidence.halt_events.push(
        HaltTelemetryEntry::new("halt", "tag", "predicate", 0.5, Some(1.0), 15, None).unwrap(),
    );
    let (entry, _) = q.prepare_proposal(finite, &author.ctx()).unwrap();
    let mut corrupt = entry.clone();
    if let Some(SkillPayload::RevisionProposal(p)) = &mut corrupt.payload {
        p.telemetry_evidence.halt_events[0].value = f32::INFINITY;
    }
    corrupt.seal().unwrap();
    assert!(matches!(
        SkillAdmissionQueue::from_stored(vec![corrupt]),
        Err(ESkillStore::InvalidEntry(ESkillQueue::NonFiniteEvidence))
    ));
    q.commit_prepared(entry).unwrap();
    let restored = SkillAdmissionQueue::from_stored(q.to_stored()).unwrap();
    assert_eq!(restored.entries(), q.entries());
    assert_eq!(ESkillQueue::NonFiniteEvidence.code(), "non-finite-evidence");
    assert_eq!(
        ESkillQueue::AuthoredIdTooLong.code(),
        "authored-id-too-long"
    );
}
