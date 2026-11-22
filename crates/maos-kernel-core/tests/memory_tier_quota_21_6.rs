//! Story 21.6 (FR28) — per-identity memory-tier quota on the private and shared
//! tiers. The kernel refuses with `MemoryError::QuotaExceeded`; compaction stays
//! the Spirit's own job. Every boundary is exact (`limit`, `limit + 1`,
//! update-at-limit); nothing here depends on timing. Collective is not charged
//! and not tested (parked under `v25-reza-postgres-legs`).
//!
//! Charge unit: `key.len()` + `MemoryValue::approximate_len()`, so a one-byte
//! key with an eight-byte `Text` value costs 9.

use std::sync::{Arc, Barrier};

use maos_domain::memory::{
    ForgetOutcome, MemoryError, MemoryNamespace, MemoryTier, MemoryValue, QuotaResource,
};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore, TierLimit,
};
use tempfile::TempDir;

const BOTH: [MemoryTier; 2] = [MemoryTier::Private, MemoryTier::Shared];

fn limit(bytes: u64, entries: u64) -> TierLimit {
    TierLimit { bytes, entries }
}

fn build(tmp: &TempDir, limit: TierLimit) -> MemoryManagerAdapter {
    let db = tmp.path().join("audit.db");
    let private = PrivateMemoryStore::new(tmp.path().join("memory"), 4 * 1024).with_quota(limit);
    let shared = SharedMemoryStore::open(&db).unwrap().with_quota(limit);
    let principal = PrincipalNamespaceIndex::open(&db).unwrap();
    MemoryManagerAdapter::new(
        Arc::new(private),
        Arc::new(shared),
        Arc::new(principal),
        Arc::new(TransparencyLogAdapter::open_in_memory(0xC216)),
    )
}

fn open(tmp: &TempDir, limit: TierLimit) -> Arc<MemoryManagerAdapter> {
    Arc::new(build(tmp, limit))
}

fn bound(adapter: &MemoryManagerAdapter, pid: u32, identity: &str) {
    adapter.bind_storage_identity(pid, identity).unwrap();
}

fn ns(tier: MemoryTier) -> MemoryNamespace {
    match tier {
        MemoryTier::Shared => MemoryNamespace::Coordination,
        _ => MemoryNamespace::Default,
    }
}

fn text(len: usize) -> MemoryValue {
    MemoryValue::Text("x".repeat(len))
}

/// `(tier, resource, used, requested, limit)` of a typed quota refusal.
fn refused(result: Result<(), MemoryError>) -> (MemoryTier, QuotaResource, u64, u64, u64) {
    match result {
        Err(MemoryError::QuotaExceeded {
            tier,
            resource,
            used,
            requested,
            limit,
        }) => (tier, resource, used, requested, limit),
        other => panic!("expected QuotaExceeded, got {other:?}"),
    }
}

fn past_quota(tier: MemoryTier, other: MemoryTier) {
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    bound(&adapter, 2, "spirit-b");
    let (view, ns, other_ns) = (adapter.for_spirit(1), ns(tier), ns(other));
    view.write(tier, &ns, "a", text(8)).unwrap(); // 9
    view.write(tier, &ns, "b", text(8)).unwrap(); // 18
    view.write(tier, &ns, "c", text(1)).unwrap(); // 20 == limit: accepted
    assert_eq!(
        refused(view.write(tier, &ns, "d", text(1))),
        (tier, QuotaResource::Bytes, 20, 2, 20),
        "limit + 1 is refused and names the tier and the exhausted resource"
    );

    // Everything else still works for the full Spirit.
    assert_eq!(view.read(tier, &ns, "a").unwrap(), Some(text(8)));
    assert_eq!(view.scan(tier, &ns, "", 16).unwrap().len(), 3);
    view.write(other, &other_ns, "other-tier", text(8)).unwrap();
    adapter
        .for_spirit(2)
        .write(tier, &ns, "theirs", text(8))
        .unwrap();
    assert!(view.delete(tier, &ns, "c").unwrap());
    assert_eq!(
        refused(view.write(tier, &ns, "d", text(2))),
        (tier, QuotaResource::Bytes, 18, 3, 20),
        "limit + 1 after a delete is still refused"
    );
    view.write(tier, &ns, "d", text(1)).unwrap();

    // The entry cap is a separate, equally exact resource.
    let capped_tmp = TempDir::new().unwrap();
    let capped = open(&capped_tmp, limit(1_000, 2));
    bound(&capped, 1, "spirit-a");
    let view = capped.for_spirit(1);
    view.write(tier, &ns, "a", text(1)).unwrap();
    view.write(tier, &ns, "b", text(1)).unwrap();
    assert_eq!(
        refused(view.write(tier, &ns, "c", text(1))),
        (tier, QuotaResource::Entries, 2, 1, 2)
    );
    view.write(tier, &ns, "a", text(500)).unwrap(); // an update adds no entry
}

#[test]
fn private_write_past_quota_is_refused_typed_and_other_ops_succeed() {
    past_quota(MemoryTier::Private, MemoryTier::Shared);
}

#[test]
fn shared_write_past_quota_is_refused_typed_and_other_ops_succeed() {
    past_quota(MemoryTier::Shared, MemoryTier::Private);
}

#[test]
fn update_charges_only_the_delta_and_shrinking_write_succeeds_when_full() {
    for tier in BOTH {
        let tmp = TempDir::new().unwrap();
        let adapter = open(&tmp, limit(20, 100));
        bound(&adapter, 1, "spirit-a");
        let (view, ns) = (adapter.for_spirit(1), ns(tier));
        view.write(tier, &ns, "a", text(19)).unwrap(); // 20 == limit
        let same_size = MemoryValue::Text("y".repeat(19));
        view.write(tier, &ns, "a", same_size.clone()).unwrap(); // update at limit
        assert_eq!(view.read(tier, &ns, "a").unwrap(), Some(same_size));
        view.write(tier, &ns, "a", text(5)).unwrap(); // shrink: charged 6, not 26
        view.write(tier, &ns, "b", text(13)).unwrap(); // 6 + 14 == limit
        view.write(tier, &ns, "a", text(5)).unwrap(); // update at limit, no growth
        assert_eq!(
            refused(view.write(tier, &ns, "a", text(6))),
            (tier, QuotaResource::Bytes, 20, 1, 20),
            "only the one-byte growth is requested"
        );
        view.write(tier, &ns, "b", text(2)).unwrap(); // shrinking while full succeeds

        // At the entry cap an update still succeeds; a new key does not.
        let capped_tmp = TempDir::new().unwrap();
        let capped = open(&capped_tmp, limit(1_000, 1));
        bound(&capped, 1, "spirit-a");
        let view = capped.for_spirit(1);
        view.write(tier, &ns, "a", text(1)).unwrap();
        view.write(tier, &ns, "a", text(9)).unwrap();
        assert!(matches!(
            view.write(tier, &ns, "b", text(1)),
            Err(MemoryError::QuotaExceeded {
                resource: QuotaResource::Entries,
                ..
            })
        ));
    }
}

#[test]
fn refused_and_failed_writes_charge_nothing() {
    let tmp = TempDir::new().unwrap();
    let adapter = build(&tmp, limit(20, 100)).with_principal_write_enforcement();
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    let (private, shared) = (MemoryTier::Private, MemoryTier::Shared);
    let principal = MemoryNamespace::principal("subject-1", "profile").unwrap();
    let default = MemoryNamespace::Default;

    // Earlier refusals (principal authorization, namespace partition, key).
    assert!(matches!(
        view.write(private, &principal, "p", text(8)),
        Err(MemoryError::PrincipalWriteUnauthorized { spirit_pid: 1 })
    ));
    adapter.authorize_principal_writes(1);
    assert!(matches!(
        view.write(shared, &principal, "p", text(8)),
        Err(MemoryError::NamespaceViolation(_))
    ));
    assert!(matches!(
        view.write(private, &default, "bad/key", text(8)),
        Err(MemoryError::InvalidKey { .. })
    ));
    assert!(matches!(
        view.write(private, &default, &"k".repeat(1025), text(8)),
        Err(MemoryError::KeyTooLong { .. })
    ));

    // A quota refusal charges nothing, on either tier.
    for tier in BOTH {
        assert_eq!(
            refused(view.write(tier, &ns(tier), "huge", text(21))),
            (tier, QuotaResource::Bytes, 0, 25, 20)
        );
    }

    // A failed spill with rollback charges nothing and keeps the prior entry.
    #[cfg(unix)]
    {
        use maos_kernel_core::memory::spill_test_faults::{arm_failure, disarm, FailurePoint};
        view.write(
            private,
            &default,
            "m",
            MemoryValue::Markdown("# 8 byte".into()),
        )
        .unwrap(); // 9
        arm_failure(FailurePoint::TempFileSync);
        assert!(view
            .write(
                private,
                &default,
                "m",
                MemoryValue::Markdown("# twelve bytes".into())
            )
            .is_err());
        disarm();
        assert_eq!(
            view.read(private, &default, "m").unwrap(),
            Some(MemoryValue::Markdown("# 8 byte".into()))
        );
        // A principal-index spill closure that fails charges nothing either.
        arm_failure(FailurePoint::TempFileSync);
        assert!(view.write(private, &principal, "p", text(8)).is_err());
        disarm();
        assert_eq!(view.read(private, &principal, "p").unwrap(), None);
    }
    #[cfg(not(unix))]
    view.write(private, &default, "m", text(8)).unwrap();

    // Exactly the limit remains: "m" (9) + "b" (9) + "c" (2) == 20.
    view.write(private, &default, "b", text(8)).unwrap();
    view.write(private, &default, "c", text(1)).unwrap();
    assert!(matches!(
        view.write(private, &default, "d", text(1)),
        Err(MemoryError::QuotaExceeded { .. })
    ));
    view.write(shared, &ns(shared), "a", text(8)).unwrap();
    view.write(shared, &ns(shared), "b", text(8)).unwrap();
    view.write(shared, &ns(shared), "c", text(1)).unwrap();
}

#[test]
fn forget_credits_each_affected_owner_and_held_forget_credits_nothing() {
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    bound(&adapter, 2, "spirit-b");
    bound(&adapter, 3, "spirit-c");
    let principal = MemoryNamespace::principal("subject-1", "profile").unwrap();
    let default = MemoryNamespace::Default;
    let private = MemoryTier::Private;
    let (a, b, c) = (
        adapter.for_spirit(1),
        adapter.for_spirit(2),
        adapter.for_spirit(3),
    );
    a.write(private, &principal, "k", text(8)).unwrap(); // 9
    a.write(private, &default, "z", text(10)).unwrap(); // 11 -> 20, full
    b.write(private, &principal, "k", text(8)).unwrap(); // 9
    b.write(private, &default, "z", text(9)).unwrap(); // 10 -> 19
    c.write(private, &default, "bystander", text(10)).unwrap();
    assert!(a.write(private, &default, "y", text(1)).is_err());

    // A legal hold suspends the erase: nothing is credited.
    assert!(matches!(
        adapter.forget_with_reason("subject-1", Some("legal-hold: case-1")),
        Ok(ForgetOutcome::Suspended { .. })
    ));
    assert!(matches!(
        a.write(private, &default, "y", text(1)),
        Err(MemoryError::QuotaExceeded { .. })
    ));
    assert!(adapter.release_legal_hold("subject-1").unwrap());

    // The erase credits both owners the principal can span.
    match adapter.forget_with_reason("subject-1", None).unwrap() {
        ForgetOutcome::Erased { receipt, .. } => assert_eq!(receipt.deleted_entries, 2),
        other => panic!("expected an erase, got {other:?}"),
    }
    a.write(private, &default, "y", text(8)).unwrap(); // 11 + 9 == 20
    b.write(private, &default, "w", text(8)).unwrap(); // 10 + 9 == 19 (+1 spare)
    assert_eq!(
        c.read(private, &default, "bystander").unwrap(),
        Some(text(10))
    );
    assert!(a.write(private, &default, "v", text(1)).is_err());
}

#[test]
fn delete_credits_usage() {
    for tier in BOTH {
        let tmp = TempDir::new().unwrap();
        let adapter = open(&tmp, limit(20, 100));
        bound(&adapter, 1, "spirit-a");
        bound(&adapter, 2, "spirit-b");
        let (a, b, ns) = (adapter.for_spirit(1), adapter.for_spirit(2), ns(tier));
        a.write(tier, &ns, "a", text(8)).unwrap();
        a.write(tier, &ns, "b", text(8)).unwrap();
        a.write(tier, &ns, "c", text(1)).unwrap();
        assert!(a.write(tier, &ns, "d", text(8)).is_err());
        assert!(a.delete(tier, &ns, "a").unwrap());
        assert!(!a.delete(tier, &ns, "a").unwrap(), "already gone");
        assert_eq!(a.read(tier, &ns, "a").unwrap(), None);
        a.write(tier, &ns, "d", text(8)).unwrap(); // the freed 9 bytes + the entry
        assert!(!b.delete(tier, &ns, "b").unwrap(), "another Spirit's entry");
        assert_eq!(a.read(tier, &ns, "b").unwrap(), Some(text(8)));
    }

    // A spilled value and a principal entry: the first is deleted with its
    // spill, the second is erased only by the forget cascade.
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    let (private, default) = (MemoryTier::Private, MemoryNamespace::Default);
    view.write(
        private,
        &default,
        "m",
        MemoryValue::Markdown("# 8 byte".into()),
    )
    .unwrap();
    assert!(view.delete(private, &default, "m").unwrap());
    assert_eq!(view.read(private, &default, "m").unwrap(), None);
    let principal = MemoryNamespace::principal("subject-1", "profile").unwrap();
    view.write(private, &principal, "k", text(8)).unwrap();
    assert!(matches!(
        view.delete(private, &principal, "k"),
        Err(MemoryError::NamespaceViolation(_))
    ));
    assert_eq!(view.read(private, &principal, "k").unwrap(), Some(text(8)));
    assert!(matches!(
        view.delete(MemoryTier::Collective, &default, "k"),
        Err(MemoryError::CollectiveNotYetAvailable { .. })
    ));
}

#[test]
fn usage_after_restart_follows_the_ruled_attribution() {
    let tmp = TempDir::new().unwrap();
    let md = |n: &str| MemoryValue::Markdown(n.repeat(8));
    let default = MemoryNamespace::Default;
    let (private, shared) = (MemoryTier::Private, MemoryTier::Shared);
    {
        let adapter = open(&tmp, limit(20, 100));
        bound(&adapter, 1, "spirit-a");
        let view = adapter.for_spirit(1);
        view.write(private, &default, "m", md("m")).unwrap(); // spilled, durable: 9
        view.write(private, &default, "s", text(8)).unwrap(); // memory-only: 9
        view.write(shared, &MemoryNamespace::Coordination, "a", text(8))
            .unwrap();
        view.write(shared, &MemoryNamespace::Coordination, "b", text(8))
            .unwrap();
    }

    // "Restart": new stores over the same files, the same identity under a new pid.
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 77, "spirit-a");
    bound(&adapter, 78, "spirit-b");
    let view = adapter.for_spirit(77);
    // Durable private usage (9) is rebuilt; the lost memory-only entry is not charged.
    view.write(private, &default, "n", md("n")).unwrap(); // 18
    assert_eq!(
        refused(view.write(private, &default, "o", md("o"))),
        (private, QuotaResource::Bytes, 18, 9, 20)
    );
    // Shared rows follow the identity, not the pid: usage (18) survives and a
    // rewrite from the new pid replaces the row instead of adding one.
    let coordination = MemoryNamespace::Coordination;
    assert_eq!(
        refused(view.write(shared, &coordination, "c", text(8))),
        (shared, QuotaResource::Bytes, 18, 9, 20)
    );
    view.write(shared, &coordination, "a", text(8)).unwrap();
    view.write(shared, &coordination, "c", text(1)).unwrap(); // 20
    let rows = view.scan(shared, &coordination, "a", 16).unwrap();
    assert_eq!(rows.len(), 1, "the restarted pid replaced the row");
    // Another identity has its own quota.
    let other = adapter.for_spirit(78);
    other.write(private, &default, "o", md("o")).unwrap();
    other.write(shared, &coordination, "c", text(8)).unwrap();
}

#[test]
fn legacy_ownerless_shared_rows_stay_readable_and_uncharged() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("audit.db");
    let coordination = MemoryNamespace::Coordination;
    // A pre-21.6 table: no `writer_owner` column, rows inserted with an explicit
    // column list like the external fixtures do.
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch(
        "CREATE TABLE shared_memory (
            writer_spirit_pid INTEGER NOT NULL, namespace TEXT NOT NULL, key TEXT NOT NULL,
            value BLOB NOT NULL, kind TEXT NOT NULL, timestamp_ns INTEGER NOT NULL,
            PRIMARY KEY (writer_spirit_pid, namespace, key));",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO shared_memory (writer_spirit_pid, namespace, key, value, kind, timestamp_ns)
         VALUES (5, ?1, 'legacy', X'68656C6C6F', 'text', 1)",
        [serde_json::to_string(&coordination).unwrap()],
    )
    .unwrap();
    drop(conn);

    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    let shared = MemoryTier::Shared;
    assert_eq!(
        view.read(shared, &coordination, "legacy").unwrap(),
        Some(MemoryValue::Text("hello".into()))
    );
    view.write(shared, &coordination, "a", text(8)).unwrap();
    view.write(shared, &coordination, "b", text(8)).unwrap();
    view.write(shared, &coordination, "c", text(1)).unwrap(); // 20: legacy row not charged
    assert!(view.write(shared, &coordination, "d", text(1)).is_err());
    assert!(!view.delete(shared, &coordination, "legacy").unwrap());
    assert!(view
        .read(shared, &coordination, "legacy")
        .unwrap()
        .is_some());
}

#[test]
fn refusal_never_evicts_or_truncates_prior_entries() {
    for tier in BOTH {
        let tmp = TempDir::new().unwrap();
        let adapter = open(&tmp, limit(30, 100));
        bound(&adapter, 1, "spirit-a");
        let (view, ns) = (adapter.for_spirit(1), ns(tier));
        let values = [
            ("a", MemoryValue::Text("alpha".into())),
            ("b", MemoryValue::Json(serde_json::json!({"n": [1, 2]}))),
            ("c", MemoryValue::Blob(vec![0, 1, 2, 255])),
            ("m", MemoryValue::Markdown("# k\0".into())),
        ];
        for (key, value) in &values {
            view.write(tier, &ns, key, value.clone()).unwrap();
        }
        // Refused: a new key and a growing update of an existing one.
        assert!(view.write(tier, &ns, "new", text(18)).is_err());
        assert!(view.write(tier, &ns, "a", text(19)).is_err());
        for (key, value) in &values {
            assert_eq!(view.read(tier, &ns, key).unwrap().as_ref(), Some(value));
        }
        assert_eq!(view.scan(tier, &ns, "", 16).unwrap().len(), values.len());
    }
}

#[test]
fn concurrent_writers_cannot_overshoot_the_limit() {
    const WRITERS: usize = 8;
    for tier in BOTH {
        let tmp = TempDir::new().unwrap();
        let adapter = open(&tmp, limit(18, 100));
        bound(&adapter, 1, "spirit-a");
        let (start, ns) = (Barrier::new(WRITERS), ns(tier));
        let outcomes: Vec<bool> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..WRITERS)
                .map(|i| {
                    let (adapter, start, ns) = (&adapter, &start, &ns);
                    scope.spawn(move || {
                        start.wait();
                        match adapter.write(1, tier, ns, &format!("k{i}"), text(7)) {
                            Ok(()) => true,
                            Err(MemoryError::QuotaExceeded { .. }) => false,
                            Err(other) => panic!("unexpected error: {other:?}"),
                        }
                    })
                })
                .collect();
            threads.into_iter().map(|t| t.join().unwrap()).collect()
        });
        // Each entry costs 9 bytes: exactly two fit under 18, whatever the interleaving.
        assert_eq!(outcomes.iter().filter(|ok| **ok).count(), 2, "{tier:?}");
        assert_eq!(adapter.scan(1, tier, &ns, "", 64).unwrap().len(), 2);
    }
}

#[test]
fn shared_rows_are_keyed_by_owner_so_a_reused_pid_cannot_erase_another_spirit() {
    let tmp = TempDir::new().unwrap();
    let (shared, co) = (MemoryTier::Shared, MemoryNamespace::Coordination);
    {
        let adapter = open(&tmp, limit(20, 100));
        bound(&adapter, 1, "spirit-a");
        let a = adapter.for_spirit(1);
        a.write(shared, &co, "k", text(8)).unwrap(); // 9
    }
    // Restart: the pid counter restarts and pid 1 is now a different Spirit.
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-b");
    bound(&adapter, 2, "spirit-a");
    let b = adapter.for_spirit(1);
    b.write(shared, &co, "k", text(3)).unwrap();
    assert_eq!(b.scan(shared, &co, "k", 16).unwrap().len(), 2, "both rows");
    // spirit-a keeps its charge: 9 + 11 == 20, then refused.
    let a = adapter.for_spirit(2);
    a.write(shared, &co, "n", text(10)).unwrap();
    assert!(a.write(shared, &co, "o", text(1)).is_err());

    // An owned write never replaces an ownerless legacy row of the same pid.
    let conn = rusqlite::Connection::open(tmp.path().join("audit.db")).unwrap();
    conn.execute(
        "INSERT INTO shared_memory (writer_spirit_pid, namespace, key, value, kind, timestamp_ns)
         VALUES (5, ?1, 'legacy', X'6869', 'text', 1)",
        [serde_json::to_string(&co).unwrap()],
    )
    .unwrap();
    drop(conn);
    bound(&adapter, 5, "spirit-c");
    let c = adapter.for_spirit(5);
    c.write(shared, &co, "legacy", text(2)).unwrap();
    assert_eq!(c.scan(shared, &co, "legacy", 16).unwrap().len(), 2);
}

#[test]
fn two_pids_bound_to_one_owner_share_one_size_source() {
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(30, 100));
    bound(&adapter, 1, "spirit-a");
    bound(&adapter, 2, "spirit-a");
    let (a, b) = (adapter.for_spirit(1), adapter.for_spirit(2));
    let (private, default) = (MemoryTier::Private, MemoryNamespace::Default);
    a.write(private, &default, "k", text(19)).unwrap(); // 20
    assert_eq!(b.read(private, &default, "k").unwrap(), Some(text(19)));
    a.write(private, &default, "k", text(1)).unwrap(); // shrinks to 2
    a.write(private, &default, "z", text(17)).unwrap(); // 18 -> owner holds 20
                                                        // B must not restore the stale 20-byte value over the bytes A reused.
    assert_eq!(
        refused(b.write(private, &default, "k", text(19))),
        (private, QuotaResource::Bytes, 20, 18, 30)
    );
    // A deletes it; B's delete finds nothing and must not credit it again.
    assert!(a.delete(private, &default, "k").unwrap());
    assert!(!b.delete(private, &default, "k").unwrap());
    b.write(private, &default, "w", text(10)).unwrap(); // 18 + 11 == 29
    assert_eq!(
        refused(a.write(private, &default, "v", text(1))),
        (private, QuotaResource::Bytes, 29, 2, 30)
    );
}

#[cfg(unix)]
#[test]
fn partial_forget_keeps_inline_charges_and_credits_only_what_was_removed() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    let (private, default) = (MemoryTier::Private, MemoryNamespace::Default);
    let principal = MemoryNamespace::principal("subject-1", "profile").unwrap();
    view.write(private, &principal, "k", text(8)).unwrap(); // spilled: 9
    view.write(private, &default, "s", text(8)).unwrap(); // memory-only: 9
    let first_child = |dir: &std::path::Path| {
        std::fs::read_dir(dir)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
    };
    let ns_dir = first_child(&first_child(&tmp.path().join("memory")));
    let mode = |bits| std::fs::set_permissions(&ns_dir, PermissionsExt::from_mode(bits)).unwrap();
    mode(0o500); // the unlink inside it now fails
    if std::fs::File::create(ns_dir.join("probe")).is_ok() {
        return; // privileged: the permission cannot force a failure here
    }
    assert!(adapter.forget_with_reason("subject-1", None).is_err());
    mode(0o700);
    // The memory-only entry still counts: 18 + 2 == 20, then refused.
    view.write(private, &default, "y", text(1)).unwrap();
    assert!(view.write(private, &default, "x", text(1)).is_err());
    // The retry completes and credits the principal entry exactly once.
    adapter.forget_with_reason("subject-1", None).unwrap();
    view.write(private, &default, "w", text(8)).unwrap(); // 11 + 9 == 20
    assert!(view.write(private, &default, "x", text(1)).is_err());
}

#[cfg(unix)]
#[test]
fn injected_commit_faults_roll_back_the_stored_value_and_the_charge() {
    use maos_kernel_core::memory::spill_test_faults::{arm_failure, disarm, FailurePoint};
    let (private, default) = (MemoryTier::Private, MemoryNamespace::Default);
    let old = MemoryValue::Markdown("# 8 byte".into());
    let faults = [
        FailurePoint::TempFileSync,
        FailurePoint::Rename,
        FailurePoint::CommitDirectorySync,
    ];
    for point in faults {
        let tmp = TempDir::new().unwrap();
        let adapter = open(&tmp, limit(20, 100));
        bound(&adapter, 1, "spirit-a");
        let view = adapter.for_spirit(1);
        view.write(private, &default, "m", old.clone()).unwrap(); // 9
        let bigger = MemoryValue::Markdown("# twelve bytes".into());
        arm_failure(point);
        assert!(
            view.write(private, &default, "m", bigger).is_err(),
            "{point:?}"
        );
        disarm();
        let kept = view.read(private, &default, "m").unwrap();
        assert_eq!(kept.as_ref(), Some(&old), "{point:?} kept the old value");
        // A failed new key leaves neither a value nor a charge behind.
        arm_failure(point);
        let fresh = MemoryValue::Markdown("# new".into());
        assert!(
            view.write(private, &default, "n", fresh).is_err(),
            "{point:?}"
        );
        disarm();
        assert_eq!(
            view.read(private, &default, "n").unwrap(),
            None,
            "{point:?}"
        );
        view.write(private, &default, "b", text(10)).unwrap(); // 9 + 11 == 20
        assert!(
            view.write(private, &default, "c", text(1)).is_err(),
            "{point:?}"
        );
    }

    // A delete whose commit fails restores the value and credits nothing.
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    view.write(private, &default, "m", old.clone()).unwrap();
    arm_failure(FailurePoint::CommitDirectorySync);
    assert!(view.delete(private, &default, "m").is_err());
    disarm();
    assert_eq!(
        view.read(private, &default, "m").unwrap().as_ref(),
        Some(&old)
    );
    view.write(private, &default, "b", text(10)).unwrap(); // 9 + 11 == 20
    assert!(view.write(private, &default, "c", text(1)).is_err());
    assert!(view.delete(private, &default, "m").unwrap());
    view.write(private, &default, "c", text(8)).unwrap(); // 11 + 9 == 20
}

#[test]
fn principal_index_refresh_failure_does_not_fail_a_committed_write() {
    let tmp = TempDir::new().unwrap();
    let adapter = open(&tmp, limit(20, 100));
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    let (private, default) = (MemoryTier::Private, MemoryNamespace::Default);
    let principal = MemoryNamespace::principal("subject-1", "profile").unwrap();
    // Make the final index refresh fail after the spill has committed.
    let conn = rusqlite::Connection::open(tmp.path().join("audit.db")).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER refuse_refresh BEFORE UPDATE ON principal_index
         BEGIN SELECT RAISE(ABORT, 'refresh blocked'); END;",
    )
    .unwrap();
    view.write(private, &principal, "p", text(8)).unwrap(); // 9, committed
    assert_eq!(view.read(private, &principal, "p").unwrap(), Some(text(8)));
    assert_eq!(adapter.subject_access("subject-1").unwrap().len(), 1);
    // Charged exactly once: 9 + 11 == 20, then refused.
    view.write(private, &default, "b", text(10)).unwrap();
    assert!(view.write(private, &default, "c", text(1)).is_err());
}

#[test]
fn growth_is_refused_while_any_total_is_over_a_limit() {
    let tmp = TempDir::new().unwrap();
    let (private, default) = (MemoryTier::Private, MemoryNamespace::Default);
    let md = |n: usize| MemoryValue::Markdown("m".repeat(n));
    {
        let adapter = open(&tmp, limit(1_000, 5));
        bound(&adapter, 1, "spirit-a");
        for key in ["a", "b", "c"] {
            adapter
                .for_spirit(1)
                .write(private, &default, key, md(8))
                .unwrap();
        }
    }
    // The operator lowers the entry cap below what the identity already holds.
    let adapter = open(&tmp, limit(1_000, 2));
    bound(&adapter, 1, "spirit-a");
    let view = adapter.for_spirit(1);
    assert_eq!(
        refused(view.write(private, &default, "a", md(9))),
        (private, QuotaResource::Entries, 3, 0, 2),
        "a byte-growing update is refused while entries are over the cap"
    );
    view.write(private, &default, "a", md(8)).unwrap(); // same size: not growth
    view.write(private, &default, "a", md(1)).unwrap(); // shrink
    assert!(view.delete(private, &default, "c").unwrap()); // back at the cap
    view.write(private, &default, "a", md(9)).unwrap();
}
