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
