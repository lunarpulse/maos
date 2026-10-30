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
        dir.path().join("tests/property.rs"),
        "#[test] fn preserves_property() {}\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join("tests/adversary.jsonl"),
        "{\"malicious\":true}\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        dir.path().join(".github/workflows/discipline.yml"),
        "jobs:\n  fixture:\n    steps:\n    - run: cargo test --test property\n",
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
