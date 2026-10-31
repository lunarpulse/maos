use std::fs;

#[test]
fn private_tier_count_finds_principal_data_when_shared_tier_is_empty() {
    let root = tempfile::tempdir().expect("temp root");
    let principal_namespace =
        "7b225072696e636970616c223a7b227072696e636970616c5f6964223a22616c696365222c22736368656d61223a2270726f66696c65227d7d";
    let principal_dir = root.path().join("42").join(principal_namespace);
    fs::create_dir_all(&principal_dir).expect("principal namespace");
    fs::write(principal_dir.join("note.txt"), b"private data").expect("private value");

    let spirit_namespace = root
        .path()
        .join("42")
        .join("7b22537069726974223a226275746c6572227d");
    fs::create_dir_all(&spirit_namespace).expect("spirit namespace");
    fs::write(spirit_namespace.join("other.txt"), b"not principal data").expect("other value");

    assert_eq!(
        maos_audit::private_tier_principal_row_count(root.path()).expect("count private tier"),
        1
    );
}

#[test]
fn empty_logical_keys_are_values_not_metadata() {
    let root = tempfile::tempdir().unwrap();
    let ns = maos_domain::memory::MemoryNamespace::principal("empty-key", "profile").unwrap();
    let directory = root
        .path()
        .join("42")
        .join(hex::encode(serde_json::to_vec(&ns).unwrap()));
    fs::create_dir_all(&directory).unwrap();
    for name in [
        ".md",
        ".json",
        ".bin",
        ".txt",
        ".identity",
        ".key-h-metadata",
        ".spill.tmp",
    ] {
        fs::write(directory.join(name), b"data").unwrap();
    }
    assert_eq!(
        maos_audit::private_tier_principal_row_count(root.path()).unwrap(),
        4
    );
}

#[test]
fn missing_identity_counts_debris_as_zero_and_refuses_unattributed_values() {
    for owner in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let path = if owner {
            root.path().join(format!("h-{}", "a".repeat(64)))
        } else {
            root.path().join("42").join(format!("h-{}", "b".repeat(64)))
        };
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(".identity-tmp.1.0"), "partial").unwrap();
        assert_eq!(
            maos_audit::private_tier_principal_row_count(root.path()).unwrap(),
            0
        );
        fs::write(path.join("6b.txt"), "unattributed").unwrap();
        let error = maos_audit::private_tier_principal_row_count(root.path()).unwrap_err();
        assert!(error.to_string().contains(path.to_str().unwrap()));
    }
}
