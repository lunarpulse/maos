#![forbid(unsafe_code)]

//! `spirits/*/manifest*.toml` — the READER the shipped worker manifests never had
//! (story `j1-crosshost-2a`, AC3.3 + AC4.1).
//!
//! Before this file, **nothing in the repo parsed `spirits/*/manifest.toml`.** The
//! topology directory has had a blocking reader since 1a
//! (`topology_delegation_1a.rs::crosshost_1a_every_shipped_topology_parses_under_strict_keys`),
//! but the `[cli_wrapper]` manifests those topologies POINT AT were read by
//! exactly one thing: a live `maos run`. So committing `manifest-codex.toml` and
//! `manifest-claude.toml` would have been decoration — a file whose only validator
//! is an operator-local paid run is not a validated file.
//!
//! This is the sibling of the 1a topology reader, and it must be enrolled in
//! `.github/workflows/discipline.yml` or it is a suggestion rather than a control:
//! 24 of 28 `crates/maos-bin/tests/` targets are never invoked by any CI job.

use maos_bin::worker_cli::{
    refuse_missing_argv_flags, select_worker_cli, FIXTURE_CLI_NAME, SUPPORTED_WORKER_CLIS,
};
use maos_manifest::manifest::CliWrapperConfig;

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn spirits_dir() -> std::path::PathBuf {
    workspace_root().join("spirits")
}

/// Every `[cli_wrapper]`-bearing manifest under `spirits/`, as
/// `(path, root TOML, parsed [cli_wrapper])`.
fn shipped_worker_manifests() -> Vec<(std::path::PathBuf, toml::Value, CliWrapperConfig)> {
    // Review 2a-P10 — RECURSIVE: the helper's claim is "every [cli_wrapper]
    // manifest under spirits/", and a `spirits/<name>/profiles/manifest-*.toml`
    // would have escaped a flat read_dir while looking shipped.
    fn visit(
        dir: &std::path::Path,
        out: &mut Vec<(std::path::PathBuf, toml::Value, CliWrapperConfig)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.starts_with("manifest") {
                continue;
            }
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            let root: toml::Value = toml::from_str(&text)
                .unwrap_or_else(|e| panic!("{} is not valid TOML: {e}", path.display()));
            let Some(section) = root.get("cli_wrapper") else {
                continue;
            };
            // The composition root feeds `from_toml_str` the re-serialized
            // `[cli_wrapper]` section, so drive it the same way rather than a
            // shape the production path never sees.
            let section_toml = toml::to_string(section)
                .unwrap_or_else(|e| panic!("{} [cli_wrapper] re-serialize: {e}", path.display()));
            let cfg = CliWrapperConfig::from_toml_str(&section_toml)
                .unwrap_or_else(|e| panic!("{} fails [cli_wrapper] parsing: {e}", path.display()));
            out.push((path, root, cfg));
        }
    }
    let mut out = Vec::new();
    visit(&spirits_dir(), &mut out);
    out
}

/// AC3.3 — every shipped worker manifest parses AND resolves to a supported
/// adapter. A manifest naming an unsupported CLI fails closed at admission with
/// "unsupported cli_wrapper command", which is a fine runtime behaviour and a
/// terrible way to discover that a committed file was never loadable.
#[test]
fn crosshost_2a_every_shipped_worker_manifest_resolves_to_an_adapter() {
    let manifests = shipped_worker_manifests();
    let mut names: Vec<String> = Vec::new();
    for (path, root, cfg) in &manifests {
        let cli = select_worker_cli(&cfg.command).unwrap_or_else(|| {
            panic!(
                "{} names cli_wrapper command '{}', which resolves to NO adapter \
                 (supported: {SUPPORTED_WORKER_CLIS:?}) — it could never be admitted",
                path.display(),
                cfg.command
            )
        });
        // `[author] name` must be present: `resolve_cli_wrapper_tier` matches it
        // against the host grant's `signing_key_id`, and an absent author becomes
        // the literal "unknown", which matches no real grant.
        let author = root
            .get("author")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        assert!(
            !author.trim().is_empty(),
            "{} has no [author] name — admission would compare the literal \"unknown\" \
             against the host grant's signing_key_id and fail closed",
            path.display()
        );
        // `[sandbox] tier` must be T3: a CliWrapperSpirit below T3 is refused at
        // admission on BOTH the fixture and the real-CLI branch.
        assert_eq!(
            root.get("sandbox")
                .and_then(|s| s.get("tier"))
                .and_then(|t| t.as_str()),
            Some("T3"),
            "{} must declare [sandbox] tier = \"T3\" (a CliWrapperSpirit below T3 is \
             refused at admission)",
            path.display()
        );
        // AC1.3 — the adapter's oracle depends on argv flags the manifest carries.
        // `maos run` refuses at the composition root when they are missing; this
        // moves that refusal to CI, where it costs nothing.
        refuse_missing_argv_flags(cli.as_ref(), &cfg.argv_prefix).unwrap_or_else(|e| {
            panic!(
                "{} would be REFUSED at the composition root: {e}",
                path.display()
            )
        });
        names.push(cli.name().to_string());
    }
    // The heterogeneity claim, as a shipped fact rather than an aspiration: the
    // repo carries manifests for the fixture AND both real adapters, so two hosts
    // can run different worker CLIs under one protocol with no code change.
    for expected in [FIXTURE_CLI_NAME, "codex", "claude"] {
        assert!(
            names.iter().any(|n| n == expected),
            "expected a shipped worker manifest for `{expected}`; saw {names:?}"
        );
    }
}

/// AC4.1 — every REAL-adapter manifest must carry a non-empty ISOLATION
/// declaration in its hashed `argv_prefix`.
///
/// This is the assertion that stops the FS-jail posture from being a doc comment
/// in a different file. Before it, every occurrence of `--sandbox workspace-write`
/// in the tracked tree was a COMMENT: `CodexCli::argv` returned the bare task, the
/// only shipped `[cli_wrapper]` manifest was the fixture's, and no test anywhere
/// asserted that a sandbox flag was present in argv.
///
/// The declaration is argv-borne on purpose. `argv_prefix` is hashed into the
/// cap-token at issue and re-derived and asserted pre-spawn, so this is a
/// cap-token-bound guarantee rather than a TOCTOU stat — and it is what makes the
/// two adapters SYMMETRIC on this axis: codex's is a flag, claude's is a
/// `--settings` document, and both ride in the same hashed vector.
#[test]
fn crosshost_2a_real_adapter_manifests_declare_an_isolation_posture() {
    use maos_bin::worker_cli::refuse_unsafe_argv;

    let mut checked = 0usize;
    for (path, _root, cfg) in shipped_worker_manifests() {
        let cli = select_worker_cli(&cfg.command).expect("resolvable adapter");
        if cli.name() == FIXTURE_CLI_NAME {
            // The hermetic fixture — no jail to declare, and it must not acquire
            // a requirement (the journey + drain suites depend on it staying green).
            // Exemption is fixture-ONLY and asserted: a fourth adapter landing
            // here without a posture would otherwise skip silently.
            assert!(cli.refuse_missing_isolation(&cfg.argv_prefix).is_ok());
            continue;
        }
        // AC4.1 via the PRODUCTION seam (review 2a-P3): the isolation assertion
        // is the adapter's own, not a token scan in this test — codex must carry
        // the long-form `--sandbox workspace-write` pair, claude a `--settings`
        // document whose payload enables `sandbox`. A `{}` settings doc or a
        // missing declaration is a refusal, so the committed file can no longer
        // pass on the token's presence alone.
        cli.refuse_missing_isolation(&cfg.argv_prefix)
            .unwrap_or_else(|e| {
                panic!(
                    "{} drives adapter `{}` but declares no enforceable isolation posture: {e}. \
                 An argv posture nothing asserts is the same doc comment in a different file.",
                    path.display(),
                    cli.name()
                )
            });
        // Review 2a-P2 — the bypass refusal is the production seam's too, so a
        // committed manifest cannot declare a jail and then bypass it.
        refuse_unsafe_argv(cli.as_ref(), &cfg.argv_prefix)
            .unwrap_or_else(|e| panic!("{} carries a posture-bypassing argv: {e}", path.display()));
        // Review 2a-P9 — AC3.2's "explicit permission posture" made mechanical
        // for claude: `--permission-mode` or `--allowedTools` must be present,
        // or deleting it from the shipped manifest keeps every test green.
        if cli.name() == "claude" {
            assert!(
                cfg.argv_prefix.iter().any(|a| a == "--permission-mode")
                    || cfg.argv_prefix.iter().any(|a| a == "--allowedTools"),
                "{} carries no explicit permission posture (`--permission-mode`/`--allowedTools`) \
                 — without one a denied tool call makes the model explain itself in prose and \
                 exit 0, the exact mechanism of the ship-blocker (AC3.2)",
                path.display()
            );
        }
        checked += 1;
    }
    assert!(
        checked >= 2,
        "expected an isolation posture asserted for BOTH real adapters, checked {checked}"
    );
}

/// The codex manifest's `argv_prefix` must keep the LONG-FORM `--sandbox
/// workspace-write` spelling, in that order.
///
/// `-s` and `--sandbox` are identical to codex and DIFFERENT BYTES to
/// `argv_prefix_hash`. The Ed25519-signed T6 capture records
/// `codex exec --sandbox workspace-write …` in its `command_metadata`, so retyping
/// the short form would silently desynchronize the committed manifest from the
/// signed bundle — as a side effect of an edit about honesty.
#[test]
fn crosshost_2a_codex_manifest_keeps_the_attested_long_form_sandbox_spelling() {
    let (path, _root, cfg) = shipped_worker_manifests()
        .into_iter()
        .find(|(_, _, c)| c.command == "codex")
        .expect("spirits/worker/manifest-codex.toml must be shipped");
    let pos = cfg
        .argv_prefix
        .iter()
        .position(|a| a == "--sandbox")
        .unwrap_or_else(|| {
            panic!(
                "{} must spell the flag `--sandbox` (long form, as the signed T6 capture \
                 attests), not `-s`; got {:?}",
                path.display(),
                cfg.argv_prefix
            )
        });
    assert_eq!(
        cfg.argv_prefix.get(pos + 1).map(String::as_str),
        Some("workspace-write"),
        "{} must pass `--sandbox workspace-write`; got {:?}",
        path.display(),
        cfg.argv_prefix
    );
    assert!(
        !cfg.argv_prefix.iter().any(|a| a == "-s"),
        "{} must not carry the short `-s` spelling: {:?}",
        path.display(),
        cfg.argv_prefix
    );
}

/// j1-crosshost-2b Q3 — every shipped topology must be LOADABLE, judged by the real
/// production function and not by a re-implementation of it.
///
/// Loadable means: every named member manifest exists, and every entry survives
/// `validate_remote_topology_target` — which refuses `host` on a class Spirit. A
/// decorative `host` key therefore reds at the commit that adds it, instead of
/// sitting in a shipped file for two stories claiming a routing behaviour the parser
/// rejects (which is what happened to `bilateral-2-host-mira-nash.toml`).
fn validate_topology_is_loadable(
    path: &std::path::Path,
    entries: &[maos_bin::topology::TopologyEntry],
    base: &std::path::Path,
) {
    use maos_bin::topology::validate_remote_topology_target;
    for entry in entries {
        let raw = std::path::PathBuf::from(&entry.manifest);
        let child = if raw.is_absolute() {
            raw
        } else {
            base.join(raw)
        };
        assert!(
            child.exists(),
            "{} names topology member {} which does not exist",
            path.display(),
            child.display()
        );
        let child_root: toml::Value =
            toml::from_str(&std::fs::read_to_string(&child).unwrap()).unwrap();
        validate_remote_topology_target(entry, &child_root).unwrap_or_else(|err| {
            panic!(
                "{} is not loadable: {err}\n  Every shipped topology must load. If a `host` key \
                 here is decorative, DELETE it (1a's `priority_weight` precedent); do not extend \
                 remote routing to class Spirits to make a stale key parse.",
                path.display()
            )
        });
    }
}

/// Every topology this repo ships must point at member manifests that EXIST, and
/// the founder-loop topologies must be fully ROUTABLE.
///
/// The 1a topology reader proves the topology parses; it does not follow the
/// `manifest` paths. A topology naming a deleted or misspelled worker manifest
/// fails at `maos run` with a read error, which is the wrong place to learn it —
/// and `manifest-codex.toml` is reachable ONLY through `j1-founder-loop-codex.toml`,
/// so nothing else would notice if that link broke.
///
/// Routability is judged by the REAL production function
/// (`validate_remote_topology_target`), not by a re-implementation of it.
///
/// **j1-crosshost-2b Q3 — the stem filter is GONE and the measured fact it recorded
/// is REPAIRED.** `2a` recorded that `bilateral-2-host-mira-nash.toml` declares
/// `host` on CLASS Spirits, which `validate_remote_topology_target` refuses, so the
/// file parsed but could never load — and called it "a forward declaration of a
/// two-host scene that `j1-crosshost-2b` owns". Measured, it was neither: it was
/// 1a's own `priority_weight` defect, missed in 1a's own pass — a key claiming
/// behaviour the parser rejects. Q3 stripped the two keys on 1a's ratified
/// precedent (a key is consumed or deleted).
///
/// With the only unloadable file now loadable, routability is asserted for **every**
/// shipped topology, not just the `j1-founder-loop*` stem. That ordering matters:
/// strip first and this widening lands GREEN on day one, and the next decorative
/// `host` key reds at its ORIGIN commit instead of being discovered two stories
/// later. The `j1-founder-loop*` stem still carries the EXTRA obligation of
/// declaring exactly one delegation target; every other topology must simply be
/// routable-or-hostless.
#[test]
fn crosshost_2a_founder_loop_topologies_are_routable_end_to_end() {
    use maos_bin::topology::{topology_manifest_entries, validate_remote_topology_target};
    let dir = workspace_root().join("spirits/topologies");
    let mut routable_founder_loops: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .expect("spirits/topologies must exist")
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        let root: toml::Value = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let Some(entries) = topology_manifest_entries(&root).unwrap() else {
            continue;
        };
        let base = path.parent().unwrap();
        let is_founder_loop = name.starts_with("j1-founder-loop");
        // Widened by j1-crosshost-2b Q3: no file is exempt from loadability.
        validate_topology_is_loadable(&path, &entries, base);
        let mut delegation_targets = 0usize;
        for e in &entries {
            let child = {
                let raw = std::path::PathBuf::from(&e.manifest);
                if raw.is_absolute() {
                    raw
                } else {
                    base.join(raw)
                }
            };
            // Universal: a topology may never name a member that does not exist.
            assert!(
                child.exists(),
                "{} names topology member {} which does not exist",
                path.display(),
                child.display()
            );
            let child_root: toml::Value =
                toml::from_str(&std::fs::read_to_string(&child).unwrap()).unwrap();
            if !is_founder_loop {
                continue;
            }
            // The founder loops are this lane's, and they must load: a `host`-bearing
            // entry that is not a `[cli_wrapper]` worker is refused by production,
            // and a `host`-bearing entry that IS one is the delegation target the
            // completion enforcement depends on.
            validate_remote_topology_target(e, &child_root)
                .unwrap_or_else(|err| panic!("{} is not routable: {err}", path.display()));
            if e.host.is_some() {
                assert!(
                    child_root.get("cli_wrapper").is_some(),
                    "{} declares host `{}` for {}, which is not a [cli_wrapper] worker",
                    path.display(),
                    e.host.as_deref().unwrap_or(""),
                    child.display()
                );
                delegation_targets += 1;
            }
        }
        if is_founder_loop {
            // Without a `host` the delegated task is `None`, the frame-borne emit is
            // skipped and the completion enforcement is bypassed ENTIRELY — the
            // worker would be spawned with no task and the run would still exit 0.
            assert_eq!(
                delegation_targets,
                1,
                "{} must declare exactly one host-bearing [cli_wrapper] delegation \
                 target; without it the completion enforcement is bypassed",
                path.display()
            );
            routable_founder_loops.push(name);
        }
    }
    routable_founder_loops.sort();
    assert_eq!(
        routable_founder_loops,
        vec![
            "j1-founder-loop".to_string(),
            "j1-founder-loop-codex".to_string(),
            // j1-crosshost-2b AC2.2 — the cross-host scene. Deliberately edited here
            // rather than named outside the `j1-founder-loop*` prefix: the prefix is
            // what subjects a file to the routability + exactly-one-delegation-target
            // obligations above, so renaming around this list would have been the
            // WEAKER posture (it exempts the new file from the controls instead of
            // enrolling it).
            "j1-founder-loop-crosshost".to_string(),
        ],
        "the hermetic founder loop, the ported codex profile, AND the cross-host scene must all \
         be routable"
    );
}
