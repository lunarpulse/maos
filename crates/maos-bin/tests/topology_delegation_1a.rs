#![forbid(unsafe_code)]

//! `[topology]` parser + frame-borne-delegation contracts (story
//! `j1-crosshost-1a`, AC1.1-1.2 / AC3.3).
//!
//! These legs previously lived in `main.rs`'s in-`src` `#[cfg(test)] mod tests`.
//! That module is charged to `maos-bin`'s KLOC budget and is **never executed by
//! CI** — the "budget-charged code with no execution path" class this story filed
//! as evidence against decision D11. Moving them here costs zero budget
//! (`xtask/src/kloc_check.rs` excludes `tests/`) and makes them executable.

use maos_bin::delegation;
use maos_bin::topology::{
    topology_manifest_entries, validate_remote_topology_target, TopologyEntry,
};

fn parse(src: &str) -> toml::Value {
    toml::from_str(src).expect("fixture must be valid TOML")
}

fn topologies_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spirits/topologies")
}

// ── Story 9.6 legs, relocated verbatim in substance ─────────────────────────

#[test]
fn story_9_6_parses_topology_manifest_entries() {
    let root = parse(
        r#"
        [topology]
        name = "founder-loop"

        [[topology.spirits]]
        manifest = "spirits/orchestrator/manifest.toml"

        [[topology.spirits]]
        manifest = "spirits/architect/manifest.toml"
        "#,
    );
    assert_eq!(
        topology_manifest_entries(&root).unwrap().unwrap(),
        vec![
            TopologyEntry {
                manifest: "spirits/orchestrator/manifest.toml".to_string(),
                host: None,
            },
            TopologyEntry {
                manifest: "spirits/architect/manifest.toml".to_string(),
                host: None,
            },
        ]
    );
}

#[test]
fn story_9_6_topology_empty_spirits_array_is_error() {
    let root = parse("[topology]\nname = \"empty\"\nspirits = []\n");
    let err = topology_manifest_entries(&root).unwrap_err();
    assert!(err.contains("no spirits"), "got: {err}");
}

#[test]
fn story_9_6_topology_missing_manifest_key_is_error() {
    let root = parse("[topology]\nname = \"bad\"\n\n[[topology.spirits]]\nrole = \"worker\"\n");
    let err = topology_manifest_entries(&root).unwrap_err();
    assert!(
        err.contains("unknown key") || err.contains("must declare manifest"),
        "an entry with no manifest must be rejected; got: {err}"
    );
}

#[test]
fn story_9_6_non_topology_manifest_returns_none() {
    let root = parse("[class]\nname = \"butler\"\n");
    assert!(topology_manifest_entries(&root).unwrap().is_none());
}

#[test]
fn story_9_6_topology_table_without_spirits_key_is_error() {
    let root = parse("[topology]\nname = \"incomplete\"\n");
    let err = topology_manifest_entries(&root).unwrap_err();
    assert!(
        err.contains("must declare [[topology.spirits]]"),
        "got: {err}"
    );
}

#[test]
fn story_9_6_topology_entry_count_matches_declaration() {
    let root = parse(
        r#"
        [topology]
        name = "test"

        [[topology.spirits]]
        manifest = "a/manifest.toml"

        [[topology.spirits]]
        manifest = "b/manifest.toml"
        "#,
    );
    assert_eq!(topology_manifest_entries(&root).unwrap().unwrap().len(), 2);
}

// ── j1-crosshost-1a AC1.1-1.2 ───────────────────────────────────────────────

#[test]
fn crosshost_1a_topology_parses_the_existing_host_key() {
    let root = parse(
        r#"
        [topology]
        name = "delegating"

        [[topology.spirits]]
        manifest = "../orchestrator/manifest.toml"

        [[topology.spirits]]
        manifest = "../worker/manifest.toml"
        host = "developer-remote-host"
        "#,
    );
    let entries = topology_manifest_entries(&root).unwrap().unwrap();
    assert_eq!(entries[0].host, None, "a local member carries no host");
    assert_eq!(
        entries[1].host.as_deref(),
        Some("developer-remote-host"),
        "the delegation target carries the EXISTING `host` key — not a new `host_id` spelling"
    );
}

#[test]
fn crosshost_1a_host_bearing_class_entry_is_refused_not_loaded_locally() {
    let entry = TopologyEntry {
        manifest: "../mira/manifest.toml".to_string(),
        host: Some("host-a-prod-edge".to_string()),
    };
    let class_manifest = parse("[class]\nname = \"mira\"\n");
    let err = validate_remote_topology_target(&entry, &class_manifest)
        .expect_err("an unsupported remote class Spirit must fail loud");
    assert!(
        err.contains("remote routing is only implemented for [cli_wrapper]")
            && err.contains("refusing to load the entry locally"),
        "the refusal must name both the unsupported target and the prohibited local fallback: {err}"
    );

    let local_entry = TopologyEntry {
        manifest: "../mira/manifest.toml".to_string(),
        host: None,
    };
    validate_remote_topology_target(&local_entry, &class_manifest)
        .expect("a class Spirit with no remote host remains a valid local entry");
}

#[test]
fn crosshost_1a_unknown_topology_key_is_rejected_not_ignored() {
    let root = parse(
        r#"
        [topology]
        name = "stale"

        [[topology.spirits]]
        manifest = "../orchestrator/manifest.toml"
        priority_weight = 3
        "#,
    );
    let err = topology_manifest_entries(&root).unwrap_err();
    assert!(
        err.contains("unknown key") && err.contains("priority_weight"),
        "an unread key must fail the run loudly — it is a false claim about behavior; got: {err}"
    );
}

#[test]
fn crosshost_1a_host_id_is_not_a_second_spelling_for_host() {
    let root = parse(
        r#"
        [topology]
        name = "wrong-spelling"

        [[topology.spirits]]
        manifest = "../worker/manifest.toml"
        host_id = "developer-remote-host"
        "#,
    );
    let err = topology_manifest_entries(&root).unwrap_err();
    assert!(
        err.contains("host_id"),
        "`host_id` must be rejected as unknown — `host` is the shipped key; got: {err}"
    );
}

/// The control that would have caught the dead-key class in the first place:
/// every topology manifest this repo SHIPS must parse under the strict policy.
/// A re-added unread key reds here, not inside a signed demo.
#[test]
fn crosshost_1a_every_shipped_topology_parses_under_strict_keys() {
    let dir = topologies_dir();
    let mut checked = 0usize;
    for entry in std::fs::read_dir(&dir)
        .expect("spirits/topologies must exist")
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let root = parse(&std::fs::read_to_string(&path).unwrap());
        let parsed = topology_manifest_entries(&root)
            .unwrap_or_else(|e| panic!("{} fails the strict parser: {e}", path.display()));
        assert!(
            parsed.is_some(),
            "{} declares no [topology] section",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "expected at least the three shipped topologies, saw {checked}"
    );
}

/// AC1.1 / AC3.3 — the J1 topology's worker entry IS the delegation target. If
/// this reds, the founder loop silently fell back to loading the worker locally
/// and the frame-borne leg this story exists to build is not exercised.
#[test]
fn crosshost_1a_j1_topology_declares_the_delegation_host() {
    let root =
        parse(&std::fs::read_to_string(topologies_dir().join("j1-founder-loop.toml")).unwrap());
    let entries = topology_manifest_entries(&root).unwrap().unwrap();
    let hosts: Vec<&str> = entries.iter().filter_map(|e| e.host.as_deref()).collect();
    assert_eq!(
        hosts,
        vec![delegation::TO_HOST],
        "exactly one `host`-bearing entry, and it must be the developer-remote host"
    );
}

/// AC1.6 — the env shortcut is DELETED, not bypassed: the read, the const and its
/// doc, the `env_contract.rs` registry row, and the topology-manifest mention.
///
/// A grep control over the two governed source trees, scoped to **live code**.
/// Comment prose that RECORDS the deletion (this test's own message, the
/// `run_cli_wrapper_manifest` doc explaining why the parameter replaced the env
/// var) is the audit trail, not a violation — the AC is about the mechanism. A
/// control that also banned the word would pressure the next author to delete the
/// explanation, which is the opposite of what this story is for.
#[test]
fn crosshost_1a_maos_worker_task_is_gone_from_live_code() {
    const NEEDLE: &str = "MAOS_WORKER_TASK";
    let self_path = std::path::Path::new(file!())
        .file_name()
        .map(|n| n.to_os_string());

    fn is_comment(line: &str) -> bool {
        let t = line.trim_start();
        t.starts_with("//") || t.starts_with('#') || t.starts_with('*')
    }

    fn scan(dir: &std::path::Path, skip: &Option<std::ffi::OsString>, hits: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                    continue;
                }
                scan(&path, skip, hits);
                continue;
            }
            if !matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("rs") | Some("toml")
            ) {
                continue;
            }
            if path.file_name().map(|n| n.to_os_string()) == *skip {
                continue;
            }
            let Ok(body) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (n, line) in body.lines().enumerate() {
                if line.contains(NEEDLE) && !is_comment(line) {
                    hits.push(format!("{}:{}", path.display(), n + 1));
                }
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut hits = Vec::new();
    scan(&root.join("crates"), &self_path, &mut hits);
    scan(&root.join("spirits"), &self_path, &mut hits);
    assert!(
        hits.is_empty(),
        "{NEEDLE} must be DELETED from live code in crates/ and spirits/ (read, const, \
         env_contract row, topology mention) — found at {hits:?}"
    );
}

/// AC3.3 — the five identity strings, and `peer_id == HostId`. A drift here is
/// silent: a mismatched sender host resolves to a different peer config at intake
/// and the frame is denied for the wrong reason.
#[test]
fn crosshost_1a_delegation_identities_are_pinned() {
    assert_eq!(delegation::RECIPIENT_SPIRIT, "developer-remote");
    assert_eq!(delegation::FROM_SPIRIT, "orchestrator");
    assert_eq!(delegation::TO_HOST, "developer-remote-host");
    assert_eq!(delegation::FROM_HOST, "founder-loop-host");
    assert_eq!(
        orchestrator::DELEGATION_CONSENT_INTENT,
        "development-task:write-workspace"
    );
}
