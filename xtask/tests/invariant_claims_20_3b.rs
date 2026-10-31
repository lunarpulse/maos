use std::fs;
use std::path::Path;
use xtask::invariant_lock::{read_claims, validate_claim, ClaimState};

fn guaranteed() -> &'static str {
    "---\nid: I1\nclaim: guaranteed\nproperty_test: tests/property.rs#preserves_property\nadversarial_corpus: tests/adversary.jsonl\nci_job: fixture\n---\n"
}

#[test]
fn guaranteed_requires_property_corpus_and_ci_job_together() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.path().join("tests")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.path().join(".github/workflows")).map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = 'fixture'\nversion = '0.0.0'\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join("tests/property.rs"),
        "#[test] fn preserves_property() { let corpus = include_str!(\"adversary.jsonl\"); assert!(corpus.contains(\"malicious\")); }\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join("tests/adversary.jsonl"),
        "{\"malicious\":true}\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join(".github/workflows/discipline.yml"),
        "jobs:\n  fixture:\n    steps:\n    - env:\n        CORPUS: tests/adversary.jsonl\n      run: cargo test -p fixture --test property preserves_property -- --exact\n",
    )
    .map_err(|e| e.to_string())?;
    assert!(matches!(
        validate_claim(dir.path(), guaranteed())?.claim,
        ClaimState::Guaranteed
    ));
    for (old, new) in [
        ("property_test: tests/property.rs#preserves_property\n", ""),
        ("adversarial_corpus: tests/adversary.jsonl\n", ""),
        ("ci_job: fixture\n", ""),
        ("#preserves_property", "#nonexistent_test"),
        ("adversary.jsonl", "missing.jsonl"),
        ("ci_job: fixture", "ci_job: missing"),
        ("claim: guaranteed", "claim: imaginary"),
    ] {
        assert!(
            validate_claim(dir.path(), &guaranteed().replace(old, new)).is_err(),
            "{old}"
        );
    }
    Ok(())
}

#[test]
fn guaranteed_refuses_nonexecuting_or_unbound_witnesses() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.path().join("tests")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.path().join(".github/workflows")).map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = 'fixture'\nversion = '0.0.0'\n",
    )
    .map_err(|e| e.to_string())?;
    let source = "#[test]\nfn preserves_property() { let corpus = include_str!(\"adversary.jsonl\"); assert!(corpus.contains(\"malicious\")); }\n";
    let workflow = "jobs:\n  fixture:\n    steps:\n    - env:\n        CORPUS: tests/adversary.jsonl\n      run: cargo test -p fixture --test property preserves_property -- --exact\n";
    fs::write(
        dir.path().join("tests/adversary.jsonl"),
        "{\"malicious\":true}\n",
    )
    .map_err(|e| e.to_string())?;
    for (label, test, job) in [
        ("commented test", format!("/* {source} */"), workflow.to_owned()),
        ("ignored test", format!("#[ignore]\n{source}"), workflow.to_owned()),
        ("not a test", source.replace("#[test]", ""), workflow.to_owned()),
        ("cfg-disabled test", format!("#[cfg(any())]\n{source}"), workflow.to_owned()),
        ("cfg-disabled file", format!("#![cfg(any())]\n{source}"), workflow.to_owned()),
        ("corpus in comment only", source.replace("include_str!(\"adversary.jsonl\")", "\"unrelated\" /* adversary.jsonl */"), workflow.to_owned()),
        ("wrong package", source.to_owned(), workflow.replace("-p fixture", "-p unrelated")),
        ("unrelated job", source.to_owned(), workflow.replace("--test property", "--test unrelated")),
        ("wrong selector", source.to_owned(), workflow.replace("property preserves_property", "property another")),
        ("disabled job", source.to_owned(), workflow.replace("  fixture:\n", "  fixture:\n    if: false\n")),
        ("disabled step", source.to_owned(), workflow.replace("    - env:", "    - if: false\n      env:")),
        ("corpus not in job", source.to_owned(), workflow.replace("tests/adversary.jsonl", "tests/unrelated.jsonl")),
        ("corpus not in test", source.replace("adversary.jsonl", "unrelated.jsonl"), workflow.to_owned()),
        ("build only", source.to_owned(), workflow.replace("cargo test", "cargo test --no-run")),
        ("ignored only", source.to_owned(), workflow.replace("-- --exact", "-- --ignored")),
        ("echoed invocation", source.to_owned(), workflow.replace("run: cargo", "run: echo cargo")),
        ("conditional shell", source.to_owned(), workflow.replace("run: cargo test -p fixture --test property preserves_property -- --exact", "run: |\n        if false; then\n          cargo test -p fixture --test property preserves_property -- --exact\n        fi")),
        ("nonblocking step boolean", source.to_owned(), workflow.replace("    - env:", "    - continue-on-error: true\n      env:")),
        ("nonblocking step expression", source.to_owned(), workflow.replace("    - env:", "    - continue-on-error: ${{ true }}\n      env:")),
        ("nonblocking job expression", source.to_owned(), workflow.replace("  fixture:\n", "  fixture:\n    continue-on-error: ${{ true }}\n")),
        ("nonblocking string", source.to_owned(), workflow.replace("    - env:", "    - continue-on-error: 'false'\n      env:")),
        ("unreachable if corpus", "#[test] fn preserves_property() { if false { let corpus = include_str!(\"adversary.jsonl\"); assert!(corpus.contains(\"malicious\")); } }".into(), workflow.to_owned()),
        ("match-nested corpus", "#[test] fn preserves_property() { match false { true => { let corpus = include_str!(\"adversary.jsonl\"); assert!(corpus.contains(\"malicious\")); }, _ => {} } }".into(), workflow.to_owned()),
        ("uncalled closure corpus", "#[test] fn preserves_property() { let unused = || { let corpus = include_bytes!(\"adversary.jsonl\"); check(corpus); }; }".into(), workflow.to_owned()),
        ("cfg-excluded corpus", "#[test] fn preserves_property() { #[cfg(any())] { let corpus = include_str!(\"adversary.jsonl\"); check(corpus); } }".into(), workflow.to_owned()),
        ("cfg-excluded binding", "#[test] fn preserves_property() { #[cfg(any())] let corpus = include_str!(\"adversary.jsonl\"); check(corpus); }".into(), workflow.to_owned()),
        ("discarded corpus", "#[test] fn preserves_property() { let _ = include_bytes!(\"adversary.jsonl\"); check(\"unrelated\"); }".into(), workflow.to_owned()),
        ("unused corpus binding", "#[test] fn preserves_property() { let corpus = include_str!(\"adversary.jsonl\"); check(\"unrelated\"); }".into(), workflow.to_owned()),
        ("return before corpus", "#[test] fn preserves_property() { return; let corpus = include_str!(\"adversary.jsonl\"); check(corpus); }".into(), workflow.to_owned()),
        ("shadowed corpus binding", "#[test] fn preserves_property() { let corpus = include_str!(\"adversary.jsonl\"); let corpus = \"unrelated\"; check(corpus); }".into(), workflow.to_owned()),
        ("cfg-excluded call", "#[test] fn preserves_property() { let corpus = include_str!(\"adversary.jsonl\"); #[cfg(any())] check(corpus); }".into(), workflow.to_owned()),
        ("cfg-excluded assert", "#[test] fn preserves_property() { let corpus = include_str!(\"adversary.jsonl\"); #[cfg(any())] assert!(corpus.contains(\"malicious\")); }".into(), workflow.to_owned()),
        ("discarded call result", source.replace("assert!(corpus.contains(\"malicious\"))", "corpus.len()"), workflow.to_owned()),
        ("exit before use", source.replace("let corpus", "std::process::exit(0); let corpus"), workflow.to_owned()),
        ("imported exit", source.replace("let corpus", "exit(0); let corpus"), workflow.to_owned()),
        ("shadow assert in test", source.replace("let corpus", "macro_rules! assert { ($($t:tt)*) => {}; } let corpus"), workflow.to_owned()),
        ("shadow assert in file", format!("macro_rules! assert {{ ($($t:tt)*) => {{}}; }} {source}"), workflow.to_owned()),
        ("runner override", source.to_owned(), workflow.replace("CORPUS:", "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER: 'true'\n        CORPUS:")),
        ("build runner override", source.to_owned(), workflow.replace("CORPUS:", "CARGO_BUILD_RUNNER: 'true'\n        CORPUS:")),
        ("rustc wrapper override", source.to_owned(), workflow.replace("CORPUS:", "RUSTC_WRAPPER: 'true'\n        CORPUS:")),
        ("custom shell", source.to_owned(), workflow.replace("    - env:", "    - shell: 'true {0}'\n      env:")),
        ("custom directory", source.to_owned(), workflow.replace("    - env:", "    - working-directory: elsewhere\n      env:")),
        ("job shell default", source.to_owned(), workflow.replace("    steps:", "    defaults:\n      run:\n        shell: 'true {0}'\n    steps:")),
    ] {
        fs::write(dir.path().join("tests/property.rs"), test).map_err(|e| e.to_string())?;
        fs::write(dir.path().join(".github/workflows/discipline.yml"), job)
            .map_err(|e| e.to_string())?;
        assert!(validate_claim(dir.path(), guaranteed()).is_err(), "{label}");
    }
    Ok(())
}

#[test]
fn guaranteed_accepts_asserted_dataflow_and_question_mark_tests(
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    fs::create_dir_all(dir.path().join("tests"))?;
    fs::create_dir_all(dir.path().join(".github/workflows"))?;
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = 'fixture'\nversion = '0.0.0'\n",
    )?;
    fs::write(dir.path().join("tests/adversary.jsonl"), "malicious")?;
    fs::write(dir.path().join(".github/workflows/discipline.yml"), "jobs:\n  fixture:\n    steps:\n    - env:\n        CORPUS: tests/adversary.jsonl\n      run: cargo test -p fixture --test property preserves_property -- --exact\n")?;
    for body in [
        "let corpus = corpus.trim(); assert!(corpus.contains(\"malicious\"));",
        "let cases = parse((&corpus))?; assert_eq!(cases.len(), 1);",
        "prepare()?; check(&corpus)?;",
        "let result = check(&corpus); assert_eq!(result, true);",
    ] {
        fs::write(dir.path().join("tests/property.rs"), format!(
            "#[test] fn preserves_property() -> Result<(), Error> {{ let corpus = include_str!(\"adversary.jsonl\"); {body} Ok(()) }}"))?;
        validate_claim(dir.path(), guaranteed())?;
    }
    Ok(())
}

#[test]
fn asserted_claim_without_owner_is_not_an_honest_ledger_entry() {
    for claim in [
        "---\nid: I1\nclaim: asserted\n---\n",
        "---\nid: I1\nclaim: asserted\nowner: ' '\n---\n",
    ] {
        assert!(validate_claim(Path::new("."), claim).is_err());
    }
    assert!(validate_claim(
        Path::new("."),
        "---\nid: I1\nclaim: asserted\nowner: 20-3b\n---\n"
    )
    .is_ok());
}

#[test]
fn committed_ledger_covers_all_fourteen_and_keeps_requirement_ownership() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("workspace parent missing")?;
    let claims = read_claims(root)?;
    assert_eq!(claims.len(), 14);
    for n in 1..=14 {
        assert!(claims.contains_key(&format!("I{n}")));
    }
    let memory = claims.get("I5").ok_or("I5 missing")?;
    assert_eq!(
        memory.requirement_claims.get("FR13").map(String::as_str),
        Some("asserted, owner 20-1")
    );
    assert_eq!(
        memory.requirement_claims.get("FR31").map(String::as_str),
        Some("proven by 20-3b")
    );
    assert!(matches!(memory.claim, ClaimState::Asserted));
    Ok(())
}

#[test]
fn forged_id_or_missing_register_cannot_shrink_the_ledger() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.path().join("docs/invariants")).map_err(|e| e.to_string())?;
    for n in 1..=14 {
        fs::write(
            dir.path().join(format!("docs/invariants/I{n}.md")),
            format!("---\nid: I{n}\nclaim: asserted\nowner: 20-3b\n---\n"),
        )
        .map_err(|e| e.to_string())?;
    }
    assert_eq!(read_claims(dir.path())?.len(), 14);
    fs::write(
        dir.path().join("docs/invariants/I14.md"),
        "---\nid: I1\nclaim: asserted\nowner: 20-3b\n---\n",
    )
    .map_err(|e| e.to_string())?;
    assert!(read_claims(dir.path()).is_err());
    fs::remove_file(dir.path().join("docs/invariants/I14.md")).map_err(|e| e.to_string())?;
    assert!(read_claims(dir.path()).is_err());
    Ok(())
}

#[test]
fn routed_witness_binaries_and_labelled_red_owners_remain_enrolled() -> Result<(), String> {
    let workflow: serde_yaml::Value = serde_yaml::from_str(include_str!(
        "../../_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/ci/ci-20-3b-verify.yml"
    )).map_err(|e| e.to_string())?;
    let steps = workflow["jobs"]["landing-2-proof"]["steps"]
        .as_sequence()
        .ok_or("hosted steps")?;
    let runs = steps
        .iter()
        .filter_map(|s| s["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for command in [
        "-p maos-spirit-sdk --features spirit_test --test routed_sdk_20_3b",
        "-p maos-kernel-core --test routed_halt_swap_20_3b",
        "--test routed_catalog_20_3b",
        "sdk_harness_records_iac_frame_io_in_its_capture_report",
        "active_halt_keeps_identity_and_replay_context_across_hot_swap",
        "-- --ignored --exact --nocapture",
    ] {
        assert!(runs.contains(command), "{command}");
    }
    for (source, test, reason) in [
        (
            include_str!("../../crates/maos-spirit-sdk/tests/routed_sdk_20_3b.rs"),
            "sdk_harness_records_iac_frame_io_in_its_capture_report",
            "ownerless and open",
        ),
        (
            include_str!("../../crates/maos-kernel-core/tests/routed_halt_swap_20_3b.rs"),
            "active_halt_keeps_identity_and_replay_context_across_hot_swap",
            "owned by 20-3c",
        ),
    ] {
        let source = syn::parse_file(source).map_err(|e| e.to_string())?;
        let witness = source
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(f) if f.sig.ident == test => Some(f),
                _ => None,
            })
            .ok_or("routed red test missing")?;
        assert!(
            witness.attrs.iter().any(|attr| match &attr.meta {
                syn::Meta::NameValue(value) if value.path.is_ident("ignore") =>
                    matches!(&value.value, syn::Expr::Lit(lit)
                    if matches!(&lit.lit, syn::Lit::Str(text) if text.value().contains(reason))),
                _ => false,
            }),
            "{test}: missing explicit red owner/disposition"
        );
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("workspace")?;
    let claims = read_claims(root)?;
    let continuity = claims.get("I14").ok_or("I14")?;
    assert!(matches!(continuity.claim, ClaimState::Asserted));
    assert_eq!(continuity.owner.as_deref(), Some("20-3c"));
    assert!(continuity.requirement_claims["FR53"].contains("asserted, owner 20-3c"));
    let ledger = include_str!("../../docs/invariants/README.md");
    assert!(ledger.contains("FR34 | asserted; owner: none (trigger-owned; see deferred-work)"));
    Ok(())
}
