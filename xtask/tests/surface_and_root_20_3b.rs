use std::fs;
use std::path::Path;
use std::process::Command;
use xtask::check_error_catalog::workspace_root_from_cwd;
use xtask::check_service_boundary::snapshot_kernel_surface;

fn put(root: &Path, path: &str, text: &str) -> Result<(), String> {
    let path = root.join(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, text).map_err(|e| e.to_string())
}

#[test]
fn grouped_pub_use_adds_exactly_one_surface_change_in_both_walkers() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    for module in [false, true] {
        let render = |extra: &str| {
            let item = format!("pub use crate::source::{{A, B as C, nested::{{D, *}}{extra}}};");
            if module {
                format!("pub mod inline {{ {item} }}")
            } else {
                item
            }
        };
        put(dir.path(), "src/lib.rs", &render(""))?;
        let before = snapshot_kernel_surface(dir.path())?;
        put(dir.path(), "src/lib.rs", &render(", E"))?;
        let after = snapshot_kernel_surface(dir.path())?;
        assert_eq!(after.items.len(), before.items.len() + 1);
        for item in &before.items {
            assert!(after.items.contains(item), "old leaf changed: {item:?}");
        }
        assert_eq!(
            after
                .items
                .iter()
                .filter(|i| !before.items.contains(i))
                .count(),
            1
        );
    }
    Ok(())
}

#[test]
fn root_fallback_chooses_nearest_workspace_not_any_package_manifest() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    put(dir.path(), "Cargo.toml", "[workspace]\nmembers = []\n")?;
    fs::create_dir_all(dir.path().join("xtask")).map_err(|e| e.to_string())?;
    put(
        dir.path(),
        "nested/Cargo.toml",
        "[workspace]\nmembers = []\n",
    )?;
    put(
        dir.path(),
        "nested/xtask/Cargo.toml",
        "[package]\nname = 'fixture'\n",
    )?;
    fs::create_dir_all(dir.path().join("nested/xtask/src")).map_err(|e| e.to_string())?;
    assert_eq!(
        workspace_root_from_cwd(&dir.path().join("nested/xtask/src"))?,
        dir.path().join("nested")
    );
    assert!(workspace_root_from_cwd(Path::new("/")).is_err());
    Ok(())
}

#[test]
fn copied_binary_catalog_leg_works_without_runtime_manifest_environment() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    put(dir.path(), "Cargo.toml", "[workspace]\nmembers = []\n")?;
    put(dir.path(), "xtask/error-catalog.toml", "error = []\n[meta]\ncatalog_version = 'fixture'\nscan_dirs = ['src']\nmin_entry_count = 0\n")?;
    put(dir.path(), "src/lib.rs", "pub struct NoErrors;\n")?;
    let binary = dir.path().join("copied-xtask");
    fs::copy(env!("CARGO_BIN_EXE_xtask"), &binary).map_err(|e| e.to_string())?;
    let catalog = dir.path().join("xtask/error-catalog.toml");
    let output = dir.path().join("docs/errors/error-catalog.json");
    let generated = Command::new(&binary)
        .current_dir(dir.path().join("xtask"))
        .env_remove("CARGO_MANIFEST_DIR")
        .args(["error-catalog-generate", "--catalog"])
        .arg(&catalog)
        .arg("--output")
        .arg(&output)
        .output()
        .map_err(|e| e.to_string())?;
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let checked = Command::new(&binary)
        .current_dir(dir.path().join("xtask"))
        .env_remove("CARGO_MANIFEST_DIR")
        .args(["error-catalog-check", "--catalog"])
        .arg(&catalog)
        .arg("--json")
        .output()
        .map_err(|e| e.to_string())?;
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    Ok(())
}
