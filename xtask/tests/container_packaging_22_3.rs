//! Story 22-3: the image, compose file and Helm chart are checked for the invariants the epic
//! names. Container and cluster execution is the hosted `container-build` job's evidence; these
//! tests read the files that job runs.
use serde_yaml::Value;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn text(path: &str) -> String {
    std::fs::read_to_string(root().join(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn yaml(path: &str) -> Value {
    serde_yaml::from_str(&text(path)).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn registered(name: &str) -> bool {
    text("crates/maos-bin/src/env_contract.rs").contains(&format!("name: \"{name}\""))
}

fn workspace_version() -> String {
    let manifest: toml::Value = toml::from_str(&text("Cargo.toml")).unwrap();
    manifest["workspace"]["package"]["version"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn image_holds_signed_maosctl_manifests_and_a_default_host_file() {
    let docker = text("Dockerfile");
    for exe in ["maos", "maosctl", "maos-wasm-runner"] {
        assert!(
            docker.contains(&format!(
                "COPY --from=verify /verify/{exe} /usr/local/bin/{exe}"
            )),
            "{exe}"
        );
        assert!(
            docker.contains(&format!("\"{exe}-linux-${{TARGETARCH}}\"")),
            "{exe} must be verified before it is copied"
        );
    }
    assert!(docker.contains("COPY spirits/ /usr/share/maos/spirits/"));
    assert!(docker.contains("COPY packaging/container/host.toml /usr/share/maos/host.toml"));
    assert!(docker.contains("ENV HOME=/var/lib/maos"));
    assert!(docker.contains("--chown=65532:65532"));
    let ignore = text(".dockerignore");
    for allowed in [
        "!dist/maosctl-linux-amd64",
        "!dist/maosctl-linux-arm64",
        "!packaging/container/host.toml",
        "!spirits/*/manifest.toml",
    ] {
        assert!(ignore.lines().any(|l| l == allowed), "{allowed}");
    }
    // The default cast is nash + observer, from manifests the image carries.
    for file in [
        "packaging/container/host.toml",
        "packaging/compose/host.toml",
    ] {
        let host: toml::Value = toml::from_str(&text(file)).unwrap();
        let cast: Vec<&str> = host["host"]["spirits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["manifest"].as_str().unwrap())
            .collect();
        assert_eq!(
            cast,
            [
                "/usr/share/maos/spirits/nash/manifest.toml",
                "/usr/share/maos/spirits/observer/manifest.toml"
            ],
            "{file}"
        );
        assert!(
            host["host"].get("door_bind").is_none(),
            "{file}: the door stays loopback"
        );
    }
    for name in ["nash", "observer"] {
        assert!(root()
            .join(format!("spirits/{name}/manifest.toml"))
            .is_file());
    }
}

#[test]
fn compose_has_init_dependency_volume_and_maosctl_healthcheck() {
    let mut compose = yaml("packaging/compose/compose.yaml");
    compose
        .apply_merge()
        .expect("resolve the x-maos-common anchor");
    let services = &compose["services"];
    assert_eq!(
        services["init"]["command"],
        serde_yaml::from_str::<Value>(r#"["init", "--plain"]"#).unwrap()
    );
    let maos = &services["maos"];
    assert_eq!(
        maos["depends_on"]["init"]["condition"].as_str(),
        Some("service_completed_successfully")
    );
    assert_eq!(maos["restart"].as_str(), Some("unless-stopped"));
    assert_eq!(maos["stop_grace_period"].as_str(), Some("30s"));
    assert_eq!(
        maos["healthcheck"]["test"],
        serde_yaml::from_str::<Value>(r#"["CMD", "/usr/local/bin/maosctl", "status"]"#).unwrap()
    );
    assert_eq!(
        maos["command"],
        serde_yaml::from_str::<Value>(r#"["serve", "--config", "/etc/maos/host.toml"]"#).unwrap()
    );
    assert!(
        compose["volumes"]["maos-home"].is_null() && compose["volumes"].get("maos-home").is_some()
    );
    for service in ["init", "maos"] {
        let mounts: Vec<&str> = services[service]["volumes"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(mounts.contains(&"maos-home:/var/lib/maos"), "{service}");
        assert!(
            services[service].get("ports").is_none(),
            "{service}: the door is loopback"
        );
        assert!(services[service].get("privileged").is_none());
    }
    // No container socket, and the T3 limit is stated.
    let raw = text("packaging/compose/compose.yaml");
    assert!(!raw.contains("docker.sock") && !raw.contains("podman.sock"));
    assert!(raw.contains("T3 Workers are NOT supported"));
    for name in maos["environment"].as_mapping().unwrap().keys() {
        let name = name.as_str().unwrap();
        assert!(
            name == "HOME" || registered(name),
            "{name} is not a registered env name"
        );
    }
}

#[test]
fn chart_version_and_values_match_the_contract() {
    let chart = yaml("packaging/helm/maos/Chart.yaml");
    assert_eq!(chart["apiVersion"].as_str(), Some("v2"));
    assert_eq!(
        chart["appVersion"].as_str(),
        Some(workspace_version().as_str())
    );
    assert!(chart["kubeVersion"].as_str().is_some());
    let values_text = text("packaging/helm/maos/values.yaml");
    let values = yaml("packaging/helm/maos/values.yaml");
    assert_eq!(
        values["image"]["repository"].as_str(),
        Some("ghcr.io/lunarpulse/maos")
    );
    assert_eq!(values["terminationGracePeriodSeconds"].as_u64(), Some(30));
    assert!(
        values["probes"]["liveness"]["failureThreshold"]
            .as_u64()
            .unwrap()
            >= 3
    );
    let host: toml::Value = toml::from_str(values["hostToml"].as_str().unwrap()).unwrap();
    assert_eq!(host["host"]["spirits"].as_array().unwrap().len(), 2);
    assert!(host["host"].get("door_bind").is_none());
    // Every secret name in the values file (live or the commented example) is registered.
    let mut seen = 0;
    for line in values_text.lines() {
        let body = line.trim_start_matches('#').trim();
        if let Some((name, _)) = body.split_once(':') {
            if name.starts_with("MAOS_") {
                seen += 1;
                assert!(registered(name), "{name}");
            }
        }
    }
    assert!(seen >= 1, "values.yaml carries an example secret name");
    assert!(values["secrets"].as_mapping().unwrap().is_empty());
    // T3 is stated in the three places AC4 names.
    for file in [
        "packaging/helm/maos/README.md",
        "packaging/helm/maos/templates/NOTES.txt",
        "packaging/helm/maos/values.yaml",
    ] {
        assert!(text(file).contains("T3 Workers"), "{file}");
    }
}

#[test]
fn statefulset_template_pins_probe_security_pvc_and_no_door_exposure() {
    let sts = text("packaging/helm/maos/templates/statefulset.yaml");
    assert!(sts.contains("kind: StatefulSet") && sts.contains("replicas: 1"));
    assert!(sts.contains("volumeClaimTemplates:"));
    assert!(sts.contains("mountPath: /var/lib/maos"));
    assert!(sts.contains("mountPath: /etc/maos") && sts.contains("readOnly: true"));
    assert!(sts.contains("fsGroup: 65532"));
    assert!(sts.contains("terminationGracePeriodSeconds:"));
    assert!(sts.contains("initContainers:") && sts.contains(r#"args: ["init", "--plain"]"#));
    for probe in ["readinessProbe", "livenessProbe"] {
        let at = sts.find(probe).unwrap_or_else(|| panic!("{probe}"));
        assert!(
            sts[at..]
                .lines()
                .nth(2)
                .unwrap()
                .contains(r#"command: ["/usr/local/bin/maosctl", "status"]"#),
            "{probe} must be an exec of maosctl status"
        );
    }
    assert!(!sts.contains("httpGet") && !sts.contains("tcpSocket"));
    // The door never leaves loopback: no Service, no port, no host networking, no socket, no privilege.
    assert!(!sts.contains("containerPort") && !sts.contains("hostPort"));
    assert!(!sts.contains("hostNetwork") && !sts.contains("hostPath"));
    assert!(!sts.contains("docker.sock") && !sts.contains("podman.sock"));
    assert!(!sts.contains("privileged: true"));
    let dir = root().join("packaging/helm/maos/templates");
    for entry in std::fs::read_dir(dir).unwrap() {
        let body = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(
            !body.contains("kind: Service\n"),
            "the chart renders no Service"
        );
    }
    let helpers = text("packaging/helm/maos/templates/_helpers.tpl");
    for line in [
        "runAsNonRoot: true",
        "runAsUser: 65532",
        "runAsGroup: 65532",
        "readOnlyRootFilesystem: true",
        "allowPrivilegeEscalation: false",
        r#"drop: ["ALL"]"#,
    ] {
        assert!(helpers.contains(line), "{line}");
    }
    let secret = text("packaging/helm/maos/templates/secret.yaml");
    assert!(secret.contains(r#"hasPrefix "MAOS_""#) && secret.contains("fail"));
    assert!(text("packaging/helm/maos/templates/configmap.yaml").contains(".Values.hostToml"));
}

#[test]
fn per_pr_job_proves_compose_and_helm_and_bounds_the_footprint() {
    let workflow = yaml(".github/workflows/discipline.yml");
    let job = &workflow["jobs"]["container-build"];
    let script: String = job["steps"]
        .as_sequence()
        .unwrap()
        .iter()
        .filter_map(|s| s["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "tests/integration/image-size-limit",
        "prove-compose.sh",
        "prove-helm.sh",
        "sha256sum -c",
        "maosctl",
    ] {
        assert!(script.contains(needle), "{needle}");
    }
    let compose = text("packaging/container/prove-compose.sh");
    for needle in [
        "tests/integration/idle-memory-limit",
        "memory.current",
        "restart maos",
        "maosctl status",
        "down\n",
    ] {
        assert!(compose.contains(needle), "{needle}");
    }
    assert_eq!(
        compose.matches("down -v").count(),
        1,
        "-v only in the cleanup trap"
    );
    let helm = text("packaging/container/prove-helm.sh");
    for needle in [
        "kind create cluster",
        "helm install",
        "delete pod",
        "@sha256:",
        "maosctl status",
    ] {
        assert!(helm.contains(needle), "{needle}");
    }
    // Every new `uses:` in the job stays a tag or a pinned commit; no remote script is piped.
    for piped in ["| sh\n", "| sh -", "| bash"] {
        assert!(
            !script.contains(piped),
            "no remote script is piped: {piped}"
        );
    }
    let required = workflow["jobs"]["aggregate"]["needs"]
        .as_sequence()
        .unwrap();
    assert!(required
        .iter()
        .any(|v| v.as_str() == Some("container-build")));
}

#[test]
fn footprint_limit_files_hold_one_integer_each() {
    for file in [
        "tests/integration/image-size-limit",
        "tests/integration/idle-memory-limit",
    ] {
        let body = text(file);
        let n: u64 = body
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{file}: one integer"));
        assert!(n > 1_000_000, "{file}");
        assert_eq!(body.lines().count(), 1, "{file}");
    }
}
