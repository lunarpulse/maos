//! Integration test: memory.md opaque write + filesystem-canonical + no parser crates.
//! AC3 — Story 4.3.

use std::sync::Arc;

use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue, ValueKind};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};
use maos_kernel_core::iac::transparency_log::TransparencyLogAdapter;
use tempfile::TempDir;

fn make_adapter() -> (Arc<MemoryManagerAdapter>, TempDir) {
    let tmp = TempDir::new().unwrap();
    let memory_root = tmp.path().join("memory");
    let db_path = tmp.path().join("audit.db");

    let private = Arc::new(PrivateMemoryStore::new(memory_root.clone(), 4 * 1024));
    let shared = Arc::new(SharedMemoryStore::open(&db_path).unwrap());
    let principal = Arc::new(PrincipalNamespaceIndex::open(&db_path).unwrap());
    let tl = Arc::new(TransparencyLogAdapter::open_in_memory(0xBEE4));
    let adapter = Arc::new(MemoryManagerAdapter::new(private, shared, principal, tl));
    (adapter, tmp)
}

#[test]
fn markdown_spills_to_disk_and_reads_back_byte_identical() {
    let (adapter, tmp) = make_adapter();
    let payload = "# Memory\n\n---\nkey: value\n\nRaw \0 control chars".to_string();
    let val = MemoryValue::Markdown(payload.clone());
    adapter
        .write(7, MemoryTier::Private, &MemoryNamespace::Default, "memory.md", val.clone())
        .unwrap();

    // Verify via read (namespace_to_dirname hex-encodes the namespace).
    let got = adapter
        .read(7, MemoryTier::Private, &MemoryNamespace::Default, "memory.md")
        .unwrap();
    assert_eq!(got, Some(MemoryValue::Markdown(payload)));
}

#[test]
fn operator_hand_edit_survives_on_next_read() {
    let (adapter, tmp) = make_adapter();
    let original = "# Original\n".to_string();
    adapter
        .write(8, MemoryTier::Private, &MemoryNamespace::Default, "notes", MemoryValue::Markdown(original.clone()))
        .unwrap();

    // Simulate operator editing the file on disk.
    let memory_root = tmp.path().join("memory");
    let ns_hex = {
        let json = serde_json::to_string(&MemoryNamespace::Default).unwrap();
        let mut hex = String::new();
        for b in json.as_bytes() {
            hex.push_str(&format!("{b:02x}"));
        }
        hex
    };
    // fs_path_for appends .md for Markdown values, so the on-disk name is notes.md.
    let path = memory_root.join("8").join(&ns_hex).join("notes.md");
    let edited = "# Edited by operator\n\nNew content.\n";
    std::fs::write(&path, edited).unwrap();

    // Read should return the edited content, not the original.
    let got = adapter
        .read(8, MemoryTier::Private, &MemoryNamespace::Default, "notes")
        .unwrap();
    assert_eq!(got, Some(MemoryValue::Markdown(edited.to_string())));
}

#[test]
fn no_markdown_parser_in_kernel_core_deps() {
    // Run cargo tree to assert no markdown/YAML parser crates.
    let output = std::process::Command::new("cargo")
        .args(["tree", "-p", "maos-kernel-core"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let forbidden = ["pulldown-cmark", "comrak", "serde_yaml"];
    for crate_name in &forbidden {
        assert!(
            !stdout.contains(crate_name),
            "forbidden parser crate {crate_name} found in maos-kernel-core dep graph"
        );
    }
}
