//! `maos init` tests — subprocess-isolated, parallel-safe.

use std::process::Command;

fn workspace_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{d}/../.."))
        .unwrap_or_else(|_| ".".into())
}

fn maos_bin() -> std::path::PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set");
    let workspace = std::path::Path::new(&dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let path = workspace.join("target").join(profile).join("maos");
    assert!(
        path.exists(),
        "maos binary not found at {path:?}; run `cargo build -p maos-bin` first"
    );
    path
}

/// A home that EXISTS. Every init test at Story 16-1's baseline used this
/// shape, which is exactly why `maos init` could exit 1 on every clean machine
/// for four stories without one red test: `run_init` opened `config.toml` with
/// `create_new(true)` BEFORE `create_dir_all(&home)`, so the failure needed a
/// fixture that does not pre-create the directory — see
/// [`absent_home`] and `init_creates_a_home_that_does_not_exist`.
fn isolated_home(label: &str) -> std::path::PathBuf {
    let path = absent_home(label);
    std::fs::create_dir_all(&path).unwrap();
    path
}

/// A home PATH whose directory deliberately does not exist — the clean-machine
/// vector of J0 exit line 1.
fn absent_home(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "maos-init-test-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn control_file(home: &std::path::Path) -> maos_domain::operator_door::ControlFile {
    maos_domain::operator_door::ControlFile::load(&maos_domain::operator_door::control_file_path(
        home,
    ))
    .expect("control.json must load and validate")
}

#[cfg(unix)]
fn mode_of(path: &std::path::Path) -> u32 {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).expect("metadata").mode() & 0o7777
}

/// Story 16-1 / AC2 — the clean-machine vector. RED at `f72b557b`:
/// `Error: Os { code: 2, kind: NotFound }`, EXIT=1.
#[test]
fn init_creates_a_home_that_does_not_exist() {
    let home = absent_home("absent");
    assert!(!home.exists(), "fixture must NOT pre-create the home");
    let out = Command::new(maos_bin())
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(
        out.status.success(),
        "init on an absent home must exit 0, got {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(home.is_dir(), "init must create the home directory");
    assert!(
        home.join("config.toml").exists(),
        "config.toml should exist"
    );
    for dir in ["skills", "audit", "journal"] {
        assert!(home.join(dir).is_dir(), "{dir} dir should exist");
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// Story 16-1 / AC2 — `maos init` is the ONE writer of `control.json`
/// (ADR-062): schema v1, `tcp://` loopback endpoint, 64 hex token, file `0600`,
/// home `0700`.
#[test]
fn init_mints_control_json_with_custody() {
    let home = absent_home("control");
    let out = Command::new(maos_bin())
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(
        out.status.success(),
        "init must exit 0: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let control = control_file(&home);
    assert_eq!(control.version, 1, "schema version is pinned at 1");
    assert!(
        control
            .endpoint
            .starts_with(maos_domain::operator_door::ENDPOINT_SCHEME),
        "endpoint must be scheme-qualified: {}",
        control.endpoint
    );
    let addr = control.endpoint_addr().expect("endpoint parses");
    assert!(addr.ip().is_loopback(), "endpoint must be loopback");
    assert_ne!(addr.port(), 0, "a probed port is never 0");
    assert_eq!(control.token.len(), 64, "32 bytes of entropy as hex");
    #[cfg(unix)]
    {
        assert_eq!(
            mode_of(&maos_domain::operator_door::control_file_path(&home)),
            0o600,
            "control.json carries the bearer token; 0600 or nothing"
        );
        assert_eq!(mode_of(&home), 0o700, "a created home is 0700 (D-16-1-D)");
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// Story 16-1 / AC2 — re-running `maos init` changes neither byte of
/// `control.json`. Rotation is `rm control.json && maos init`, never an
/// in-place rewrite: a rewritten token silently breaks every live root.
#[test]
fn init_does_not_rewrite_an_existing_control_json() {
    let home = absent_home("stable");
    let bin = maos_bin();
    for _ in 0..2 {
        assert!(Command::new(&bin)
            .arg("init")
            .env("MAOS_HOME", &home)
            .current_dir(workspace_root())
            .output()
            .expect("maos init")
            .status
            .success());
    }
    let path = maos_domain::operator_door::control_file_path(&home);
    let first = std::fs::read(&path).expect("control.json");
    assert!(Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("maos init")
        .status
        .success());
    assert_eq!(
        first,
        std::fs::read(&path).expect("control.json"),
        "re-running init must not rewrite control.json"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// Story 16-1 / AC2 — the upgrade path: a home initialised before 16-1 has a
/// complete `config.toml` and NO `control.json`, and `maos init` must mint one
/// instead of taking the "already initialized" shortcut.
#[test]
fn init_mints_control_json_on_an_already_initialised_home() {
    let home = absent_home("upgrade");
    let bin = maos_bin();
    assert!(Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("maos init")
        .status
        .success());
    let path = maos_domain::operator_door::control_file_path(&home);
    std::fs::remove_file(&path).expect("simulate a pre-16-1 home");
    let out = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("maos init");
    assert!(out.status.success(), "re-init must exit 0");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("already initialized"),
        "an initialised home still reports already initialized"
    );
    control_file(&home);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn init_creates_config_and_dirs() {
    let home = isolated_home("create");
    let bin = maos_bin();
    let out = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(
        out.status.success(),
        "init should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        home.join("config.toml").exists(),
        "config.toml should exist"
    );
    assert!(home.join("skills").is_dir(), "skills dir should exist");
    assert!(home.join("audit").is_dir(), "audit dir should exist");
    assert!(home.join("journal").is_dir(), "journal dir should exist");
    let cfg = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert!(cfg.contains("[slots]"), "config should declare slots");
    assert!(
        cfg.contains("[retention]"),
        "config should declare retention"
    );
    assert!(
        cfg.contains("default = \"persist\""),
        "retention default should be persist"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn init_is_idempotent() {
    let home = isolated_home("idempotent");
    let bin = maos_bin();
    // First init.
    let out1 = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init");
    assert!(
        out1.status.success(),
        "first init: {}",
        String::from_utf8_lossy(&out1.stderr)
    );
    let before = std::fs::read_to_string(home.join("config.toml")).unwrap();
    // Second init — should say already initialized and NOT clobber.
    let out2 = Command::new(&bin)
        .arg("init")
        .env("MAOS_HOME", &home)
        .current_dir(workspace_root())
        .output()
        .expect("failed to execute maos init again");
    assert!(out2.status.success(), "second init should succeed");
    let stdout = String::from_utf8_lossy(&out2.stdout);
    assert!(
        stdout.contains("already initialized"),
        "second init should say already initialized: {stdout}"
    );
    let after = std::fs::read_to_string(home.join("config.toml")).unwrap();
    assert_eq!(before, after, "config should not be clobbered");
    let _ = std::fs::remove_dir_all(&home);
}
