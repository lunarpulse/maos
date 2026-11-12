#![forbid(unsafe_code)]

//! Story 21-5 AC3.2 (D-21-5-09 A) — the FR20 buffer stays out of the kernel:
//! no `crates/maos-kernel-core/src/orchestrator/` directory, no
//! `pub mod orchestrator` in the kernel `lib.rs`, and no Rust file under
//! `crates/`, `spirits/` or `xtask/src` names the old path. The dependency
//! half of the control is `check-dependency-closure` (`FORBIDDEN_CRATES`).
//!
//! Proven red (story AC3.4): restoring `pub mod orchestrator;` with a stub
//! file reds the first two vectors; at HEAD before the move the directory
//! existed and 15 occurrences in 10 files named the path.

use std::error::Error;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

type TestResult = Result<(), Box<dyn Error>>;

/// The retired path, assembled so this file does not name it itself.
fn retired_path() -> String {
    ["maos_kernel_core", "orchestrator"].join("::")
}

fn root() -> Result<PathBuf, Box<dyn Error>> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask has no parent directory".into())
}

#[test]
fn the_kernel_has_no_orchestrator_module() -> TestResult {
    let root = root()?;
    let module_dir = root.join("crates/maos-kernel-core/src/orchestrator");
    assert!(
        !module_dir.exists(),
        "{} must not exist: the buffer lives in maos-orchestrator-buffer",
        module_dir.display()
    );
    let lib = std::fs::read_to_string(root.join("crates/maos-kernel-core/src/lib.rs"))?;
    let declares = lib.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with("pub mod orchestrator") || line.starts_with("mod orchestrator")
    });
    assert!(
        !declares,
        "kernel lib.rs must not declare an orchestrator module"
    );
    Ok(())
}

#[test]
fn no_rust_file_names_the_retired_kernel_path() -> TestResult {
    let root = root()?;
    let needle = retired_path();
    let mut hits = Vec::new();
    for dir in ["crates", "spirits", "xtask/src"] {
        for entry in WalkDir::new(root.join(dir)) {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs")
                && !path.components().any(|part| part.as_os_str() == "target")
                && std::fs::read_to_string(path)?.contains(&needle)
            {
                hits.push(path.display().to_string());
            }
        }
    }
    assert!(hits.is_empty(), "`{needle}` is named by: {hits:?}");
    Ok(())
}
