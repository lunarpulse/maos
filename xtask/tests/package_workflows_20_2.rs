//! Story 20-2: wiring is checked separately from hosted execution evidence.
use serde_yaml::Value;
use std::path::Path;

fn load(path: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let text = std::fs::read_to_string(root.join(path)).unwrap();
    match serde_yaml::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("{path}: {error}"),
    }
}

#[test]
fn ac1_native_deb_matrix_and_all_consumers_are_required() {
    let workflow = load(".github/workflows/discipline.yml");
    let jobs = &workflow["jobs"];
    let native = jobs["release-dry-run"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .unwrap();
    assert!(native
        .iter()
        .any(|r| r["os"].as_str() == Some("ubuntu-24.04")));
    assert!(native
        .iter()
        .any(|r| r["os"].as_str() == Some("ubuntu-24.04-arm")));
    let required = jobs["aggregate"]["needs"].as_sequence().unwrap();
    for name in [
        "air-gap-netns-witness",
        "container-build",
        "package-recipes",
        "cargo-install",
    ] {
        assert!(jobs[name].is_mapping(), "missing {name}");
        assert!(
            required.iter().any(|v| v.as_str() == Some(name)),
            "unbound {name}"
        );
    }
    let ship = jobs["v1-0-ship-gate"]["needs"].as_sequence().unwrap();
    assert!(ship
        .iter()
        .any(|v| v.as_str() == Some("air-gap-netns-witness")));
    assert!(
        !ship.iter().any(|v| v.as_str() == Some("check-air-gap")),
        "D-02 C advisory"
    );
}

#[test]
fn ac5_login_push_and_sign_are_all_credential_gated() {
    let workflow = load(".github/workflows/container.yml");
    assert!(workflow["on"]["push"]["tags"].is_sequence());
    assert!(workflow["on"]["pull_request_target"].is_null());
    let job = &workflow["jobs"]["build-and-push"];
    assert!(job["env"]["PUBLISH"]
        .as_str()
        .unwrap()
        .contains("DOCKERHUB_TOKEN != ''"));
    for step in job["steps"].as_sequence().unwrap() {
        let uses = step["uses"].as_str().unwrap_or("");
        let run = step["run"].as_str().unwrap_or("");
        if uses.starts_with("docker/login-action")
            || uses.starts_with("docker/build-push-action")
            || uses.starts_with("sigstore/")
            || run.contains("cosign sign")
        {
            assert_eq!(step["if"].as_str(), Some("env.PUBLISH == 'true'"));
        }
        if step["name"].as_str() == Some("Always build and smoke without logging in") {
            assert!(step["if"].is_null());
            assert!(run.contains("--load") && !run.contains("--push"));
        }
    }
}

#[test]
fn ac3_hosted_recipe_stays_branch_only_and_exercises_real_controls() {
    let path = "_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml";
    let workflow = load(path);
    assert_eq!(
        workflow["on"]["push"]["branches"][0].as_str(),
        Some("ci/20-2-verify")
    );
    for name in ["deb", "netns", "container", "recipes", "cargo-and-symbols"] {
        assert!(workflow["jobs"][name].is_mapping(), "missing {name}");
    }
}
