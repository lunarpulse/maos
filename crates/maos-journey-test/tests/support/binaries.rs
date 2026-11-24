//! Journey executables must come from the selected target directory and current sources.
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const REBUILD: &str = "cargo build -p maos-bin -p maos-cli";

pub fn resolve(name: &str, cargo_binary: Option<&str>) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    resolve_at(name, cargo_binary, &root)
}

pub fn resolve_at(name: &str, cargo_binary: Option<&str>, root: &Path) -> String {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let target = if target.is_absolute() {
        target
    } else {
        root.join(target)
    };
    let binary = cargo_binary.map(PathBuf::from).unwrap_or_else(|| {
        target
            .join("debug")
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
    });
    let built = std::fs::metadata(&binary)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|error| {
            panic!(
                "journey binary {} unavailable: {error}; run `{REBUILD}`",
                binary.display()
            )
        });
    for source in ["crates/maos-bin/src", "crates/maos-cli/src"] {
        require_fresh(&binary, built, &root.join(source));
    }
    binary.to_string_lossy().into_owned()
}

fn require_fresh(binary: &Path, built: SystemTime, directory: &Path) {
    let entries = std::fs::read_dir(directory).unwrap_or_else(|error| {
        panic!(
            "cannot inspect journey sources {}: {error}; run `{REBUILD}`",
            directory.display()
        )
    });
    for entry in entries {
        let entry = entry.expect("read journey source entry");
        let path = entry.path();
        let metadata = entry.metadata().expect("stat journey source");
        if metadata.is_dir() {
            require_fresh(binary, built, &path);
        } else if metadata.is_file() {
            let modified = metadata.modified().expect("read journey source mtime");
            assert!(
                modified <= built,
                "stale journey binary {}: source {} is newer; run `{REBUILD}`",
                binary.display(),
                path.display()
            );
        }
    }
}

/// Poll `probe` every 200 ms until it returns `true` or `limit_secs` pass (a liveness bound, not
/// a timing assertion). The JB-7 wall-clock guard scans journey sources, not this helper.
#[allow(dead_code)]
pub fn poll_until(limit_secs: u64, mut probe: impl FnMut() -> bool) -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(limit_secs);
    loop {
        if probe() {
            return true;
        }
        if std::time::Instant::now() >= until {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
