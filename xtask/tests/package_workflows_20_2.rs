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
    for step in workflow["jobs"]
        .as_mapping()
        .into_iter()
        .flat_map(|jobs| jobs.values())
        .flat_map(|job| job["steps"].as_sequence().into_iter().flatten())
    {
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
    for name in [
        "deb",
        "netns",
        "container",
        "recipes",
        "cargo-and-symbols",
        "native-packaging",
        "packaging-assembly",
    ] {
        assert!(workflow["jobs"][name].is_mapping(), "missing {name}");
    }
}

fn steps(job: &Value) -> String {
    job["steps"]
        .as_sequence()
        .into_iter()
        .flatten()
        .filter_map(|step| step["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn review_p1_ubuntu_rpm_has_explicit_checked_dependency_mode() {
    let workflow = load(".github/workflows/release.yml");
    assert!(steps(&workflow["jobs"]["sign-and-publish"]).contains("dist rpm-ubuntu dist/packages"));
}

#[test]
fn review_p2_runner_owns_output_directory_before_root_containers() {
    let workflow = load(".github/workflows/release.yml");
    let script = steps(&workflow["jobs"]["sign-and-publish"]);
    assert!(script
        .find("mkdir -p dist/packages")
        .zip(script.find("for arch in amd64"))
        .is_some_and(|(mkdir, containers)| mkdir < containers));
}

#[test]
fn review_p4_python_controls_are_blocking_native_release_steps() {
    let workflow = load(".github/workflows/discipline.yml");
    assert!(steps(&workflow["jobs"]["release-dry-run"])
        .contains("python3 packaging/test_release_20_2.py"));
}

#[test]
fn review_p5_all_recipe_routes_require_wrong_key_rejection() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let script = std::fs::read_to_string(root.join("packaging/witness-recipe.sh")).unwrap();
    assert!(
        script.contains("forge-recipe-signature.py")
            && script.contains("InvalidSignature")
            && script.contains("accepted forged signature")
    );
}

#[test]
fn review_f1_packaging_manifest_authenticates_verifier_and_recipes() {
    let workflow = load(".github/workflows/release.yml");
    let script = steps(&workflow["jobs"]["sign-and-publish"]);
    assert!(script.contains("assemble-release-packaging.py"));
}

#[test]
fn review_f2_operator_prepare_cannot_build_tag_code_with_publish_permissions() {
    let workflow = load("_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml");
    let prepare = &workflow["jobs"]["operator-prepare"];
    assert_eq!(prepare["permissions"]["contents"].as_str(), Some("read"));
    assert!(
        prepare["permissions"]["packages"].is_null()
            && prepare["permissions"]["id-token"].is_null()
    );
    assert_eq!(
        workflow["jobs"]["operator-publication"]["environment"].as_str(),
        Some("release")
    );
    assert!(steps(prepare).contains("release-verify --verify"));
    assert!(!steps(&workflow["jobs"]["operator-publication"]).contains("cargo build"));
}

#[test]
fn review_f3_release_signing_requires_environment_reference() {
    let workflow = load(".github/workflows/release.yml");
    assert_eq!(
        workflow["jobs"]["sign-and-publish"]["environment"].as_str(),
        Some("release")
    );
}

#[test]
fn review_f4_critical_release_actions_are_full_commit_pinned() {
    for path in [".github/workflows/release.yml", ".github/workflows/container.yml",
        "_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml"] {
        let workflow = load(path);
        for job in workflow["jobs"].as_mapping().into_iter().flat_map(|jobs| jobs.values()) {
            for step in job["steps"].as_sequence().into_iter().flatten() {
                if let Some(uses) = step["uses"].as_str() {
                    if ["softprops/action-gh-release@", "docker/build-push-action@", "docker/login-action@", "sigstore/cosign-installer@"]
                        .iter().any(|prefix| uses.starts_with(prefix)) {
                        assert!(uses.rsplit_once('@').is_some_and(|(_, pin)| pin.len() == 40
                            && pin.bytes().all(|b| b.is_ascii_hexdigit())), "{path}: {uses}");
                    }
                }
            }
        }
    }
}

#[test]
fn review_f5_and_f10_container_bases_and_owned_state_are_explicit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let docker = std::fs::read_to_string(root.join("Dockerfile")).unwrap();
    assert!(docker
        .lines()
        .filter(|line| line.starts_with("FROM "))
        .all(|line| line.contains("@sha256:")));
    assert!(docker.contains("--chown=65532:65532") && docker.contains("/var/lib/maos"));
}

#[test]
fn review_f8_netns_artifacts_are_verified_before_execution() {
    let workflow = load(".github/workflows/discipline.yml");
    let script = steps(&workflow["jobs"]["air-gap-netns-witness"]);
    assert!(script
        .find("packaging/verify-release.py")
        .zip(script.find("tests/air-gap-netns-corroborate.sh"))
        .is_some_and(|(verify, execute)| verify < execute));
}

#[test]
fn review_f9_container_wait_binds_release_id_and_attempt() {
    let workflow = load(".github/workflows/container.yml");
    assert!(steps(&workflow["jobs"]["build-and-push"]).contains("packaging/wait-release.sh"));
}

#[test]
fn hosted_1_rpm_artifact_macro_does_not_redefine_reserved_suffix() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let spec = std::fs::read_to_string(root.join("packaging/rpm/maos.spec")).unwrap();
    assert!(!spec.lines().any(|line| line.starts_with("%global suffix ")));
    assert!(!spec.contains("%{suffix}"));
    assert!(spec.contains("%global maos_artifact_suffix linux-amd64"));
    assert!(spec.contains("%global maos_artifact_suffix linux-arm64"));
    assert_eq!(spec.matches("%{maos_artifact_suffix}").count(), 9);
}

#[test]
fn hosted_1_every_debian_builder_provisions_build_essential() {
    for (path, jobs) in [
        (".github/workflows/discipline.yml", &["release-dry-run"][..]),
        (".github/workflows/release.yml", &["sign-and-publish"][..]),
        ("_bmad-output/implementation-artifacts/20-2-deb-airgap-docker-and-formula-corrections-evidence/ci/ci-20-2-verify.yml",
            &["native-packaging", "deb"][..]),
    ] {
        let workflow = load(path);
        for job in jobs {
            let script = steps(&workflow["jobs"][*job]);
            let provisioning: Vec<_> = script.lines().filter(|line|
                line.contains("apt-get install") && line.split_whitespace().any(|word| word == "debhelper")).collect();
            assert!(!provisioning.is_empty(), "{path}:{job}: missing prerequisite step");
            for line in provisioning {
                assert!(line.split_whitespace().any(|word| word == "build-essential"),
                    "{path}:{job}: {line}");
            }
            assert!(!script.contains("dpkg-buildpackage -d"), "{path}:{job}: dependency bypass");
        }
    }
}
