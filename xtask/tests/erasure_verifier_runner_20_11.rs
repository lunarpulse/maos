//! 20-11: both whole-package consumers provision the mandatory verifier backend.
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn both_whole_package_runners_provision_the_hash_locked_verifier() {
    let source =
        std::fs::read_to_string(root().join(".github/workflows/discipline.yml")).expect("workflow");
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(&source).unwrap_or_else(|error| panic!("workflow: {error}"));
    for name in ["workspace-test-suite", "check-multi-tenant-loom"] {
        let steps = workflow["jobs"][name]["steps"]
            .as_sequence()
            .unwrap_or_else(|| panic!("steps: {name}"));
        assert!(
            steps.iter().any(|step| step["uses"].as_str()
                == Some("./.github/actions/provision-erasure-verifier")),
            "{name} can execute the protected verifier tests without their declared backend"
        );
        assert!(
            !steps.iter().any(|step| step["run"]
                .as_str()
                .is_some_and(|script| script.contains("pip install --user cryptography"))),
            "{name} still uses an unpinned optional backend"
        );
    }
    let action = std::fs::read_to_string(
        root().join(".github/actions/provision-erasure-verifier/action.yml"),
    )
    .expect("provision action");
    for required in [
        "--require-hashes",
        "--only-binary=:all:",
        "requirements.txt",
        "Ed25519PrivateKey",
        "GITHUB_ENV",
    ] {
        assert!(
            action.contains(required),
            "missing real declared-backend control: {required}"
        );
    }
    let requirements =
        std::fs::read_to_string(root().join("tools/verify-erasure/requirements.txt"))
            .expect("locked backend");
    for dependency in ["cryptography==50.0.0", "cffi==2.1.1", "pycparser==3.12"] {
        assert!(
            requirements.contains(dependency),
            "unlocked verifier dependency: {dependency}"
        );
    }
    for line in requirements
        .lines()
        .filter(|line| line.trim_start().starts_with("--hash="))
    {
        let hash = line
            .trim()
            .trim_end_matches('\\')
            .trim()
            .strip_prefix("--hash=sha256:")
            .unwrap_or_else(|| panic!("SHA256 hash required: {line}"));
        assert!(
            hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "invalid wheel hash: {line}"
        );
    }
}

#[test]
fn declared_backend_path_survives_the_delegated_environment_filter() {
    let directory = tempfile::tempdir().expect("backend path transport fixture");
    std::fs::write(
        directory.path().join("maos_verifier_path_probe_20_11.py"),
        "VALUE = 'declared-path'\n",
    )
    .expect("transport sentinel, not a crypto substitute");
    let output = std::process::Command::new("python3").args(["-c", r#"
import importlib.util, os, subprocess, sys
spec = importlib.util.spec_from_file_location("ceiling", sys.argv[1])
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)
environment = module._environment("delegated")
assert environment.get("PYTHONPATH") == sys.argv[2], environment
subprocess.run([sys.executable, "-c", "import maos_verifier_path_probe_20_11 as probe; assert probe.VALUE == 'declared-path'"], env=environment, check=True)
"#]).arg(root().join("scripts/run_cgroup_ceiling_17_2.py")).arg(directory.path())
        .env_clear().env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", directory.path()).env("PYTHONPATH", directory.path())
        .output().expect("execute real environment filter and child import");
    assert!(
        output.status.success(),
        "declared backend path was lost at the cgroup boundary: {output:?}"
    );
}
