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
