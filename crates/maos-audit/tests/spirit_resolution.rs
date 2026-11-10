use maos_audit::resolve_spirit_name;

fn create_log(path: &std::path::Path) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open(path).expect("open fixture log");
    connection
        .execute_batch(
            "CREATE TABLE transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );",
        )
        .expect("create transparency log");
    connection
}

fn insert_lifecycle_load(
    connection: &rusqlite::Connection,
    frame_byte: u8,
    timestamp_ns: i64,
    boot_nonce: i64,
    spirit_pid: i64,
    intent: &str,
    payload_name: &str,
) {
    let payload = serde_json::to_vec(&serde_json::json!({"spirit_id": payload_name}))
        .expect("serialize payload");
    connection
        .execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token,
              kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, 7, ?6, ?7, 0)",
            rusqlite::params![
                &[frame_byte; 16] as &[u8],
                timestamp_ns,
                spirit_pid,
                boot_nonce,
                rusqlite::types::Null,
                intent,
                &payload as &[u8],
            ],
        )
        .expect("insert lifecycle load");
}

#[test]
fn payload_identity_overrides_legacy_intent_and_incarnations_are_deduplicated() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let path = temp.path().join("audit.sqlite");
    let connection = create_log(&path);
    insert_lifecycle_load(&connection, 1, 10, 7, 42, "legacy-name", "researcher");
    insert_lifecycle_load(&connection, 2, 20, 7, 42, "legacy-name", "researcher");
    insert_lifecycle_load(&connection, 3, 30, 8, 43, "legacy-name", "researcher");
    drop(connection);

    assert_eq!(
        resolve_spirit_name(&path, "researcher", true).expect("resolve payload identity"),
        vec![(7, 42), (8, 43)],
        "dropping timestamps must not return duplicate incarnation pairs"
    );
    assert!(
        resolve_spirit_name(&path, "legacy-name", true).is_err(),
        "an authoritative payload identity must override a contradictory legacy intent"
    );
    assert_eq!(
        resolve_spirit_name(&path, "researcher", false).expect("latest incarnation"),
        vec![(8, 43)]
    );
}

#[test]
fn scheduler_journal_echo_rows_are_not_spirit_identities() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let path = temp.path().join("audit.sqlite");
    let connection = create_log(&path);
    insert_lifecycle_load(&connection, 1, 10, 9, 42, "lifecycle.load", "worker");
    // SpiritSchedulerAdapter::journal_lifecycle echoes a T3 SandboxApplied
    // transition at pid 0 with the Spirit's id in its kind-7 payload.
    let echo = serde_json::to_vec(&serde_json::json!({
        "lifecycle_event": "SandboxApplied",
        "spirit_id": "worker",
    }))
    .expect("serialize echo payload");
    connection
        .execute(
            "INSERT INTO transparency_log
             (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token,
              kind, intent, payload_redacted, origin)
             VALUES (?1, 20, 0, 9, NULL, 7, 'lifecycle.journal', ?2, 0)",
            rusqlite::params![&[2u8; 16] as &[u8], &echo as &[u8]],
        )
        .expect("insert scheduler journal echo");
    drop(connection);

    assert_eq!(
        resolve_spirit_name(&path, "worker", false).expect("resolve the loaded Worker"),
        vec![(9, 42)],
        "a pid-0 journal echo must not displace the Worker's real pid"
    );
}

#[test]
fn cost_spirit_names_distinguish_boots_and_skip_echoes() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let path = temp.path().join("audit.sqlite");
    let connection = create_log(&path);
    insert_lifecycle_load(&connection, 1, 10, 7, 42, "lifecycle.load", "alpha");
    insert_lifecycle_load(&connection, 2, 20, 8, 42, "lifecycle.load", "beta");
    insert_lifecycle_load(&connection, 3, 30, 7, 42, "lifecycle.journal", "wrong");
    insert_lifecycle_load(&connection, 5, 50, 7, 42, "cap.revoke", "forged");
    insert_lifecycle_load(&connection, 6, 60, 7, 99, "cap.verify.ok", "forged");
    insert_lifecycle_load(&connection, 7, 70, 7, 42, "lifecycle.load", "later");
    connection
        .execute(
            "INSERT INTO transparency_log VALUES (?1, 80, 42, 7, ?2, 7, 'lifecycle.load', ?3, 0)",
            rusqlite::params![
                &[8u8; 16] as &[u8],
                &[9u8; 32] as &[u8],
                br#"{"spirit_id":"spoof"}"# as &[u8]
            ],
        )
        .expect("insert token-bearing fake load");
    connection
        .execute(
            "INSERT INTO transparency_log VALUES (?1, 40, 43, 8, NULL, 19, 'gamma', ?2, 0)",
            rusqlite::params![&[4u8; 16] as &[u8], b"{}" as &[u8]],
        )
        .expect("insert admission");
    drop(connection);
    let names = maos_audit::spirit_names(&path).expect("names");
    assert_eq!(
        names.get(&(7, 42)).unwrap(),
        &vec![(10, "alpha".into()), (70, "later".into())]
    );
    assert_eq!(names.get(&(8, 42)).unwrap(), &vec![(20, "beta".into())]);
    assert_eq!(names.get(&(8, 43)).unwrap(), &vec![(40, "gamma".into())]);
    assert!(!names.contains_key(&(7, 99)));
}

#[test]
fn cost_query_origin_filter_keeps_only_kernel_frames() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let path = temp.path().join("audit.sqlite");
    let connection = create_log(&path);
    for (byte, origin) in [(1u8, 3i64), (2, 1), (3, 0)] {
        connection
            .execute(
                "INSERT INTO transparency_log VALUES (?1, 10, 7, 1, NULL, 29, 'cost', X'7B7D', ?2)",
                rusqlite::params![&[byte; 16] as &[u8], origin],
            )
            .expect("insert cost frame");
    }
    drop(connection);
    let filter = maos_audit::AuditFilter {
        kind: Some("cost".into()),
        origin: Some(maos_domain::invariants::i3::FrameOrigin::Kernel),
        ..Default::default()
    };
    let entries = maos_audit::query(&path, filter).expect("query");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].frame_id_hex, "01".repeat(16));
}
