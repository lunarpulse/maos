//! Story 17-3b AC5 — authored TypeScript projects receive the exact host WIT.

#[test]
fn root_template_and_example_wit_are_byte_identical() {
    let root = include_bytes!("../../../wit/spirit.wit");
    let template = include_bytes!("../../../templates/spirit-ts/wit/spirit.wit");
    let example = include_bytes!("../../../examples/example-spirit-ts/wit/spirit.wit");

    assert_eq!(
        root, template,
        "template WIT drifted from the host contract"
    );
    assert_eq!(root, example, "example WIT drifted from the host contract");
}

#[test]
fn typescript_authoring_imports_match_the_current_wit_package() {
    let wit = include_str!("../../../wit/spirit.wit");
    let package = wit
        .lines()
        .find_map(|line| line.strip_prefix("package maos:spirit@"))
        .and_then(|version| version.strip_suffix(';'))
        .expect("root WIT must declare the maos:spirit package version");
    let spirit = format!("\"maos:spirit/spirit@{package}\"");
    let recall = format!("\"maos:spirit/recall@{package}\"");
    let readme_package = format!("`maos:spirit@{package}`");
    let readme_recall = format!("`maos:spirit/recall@{package}`");

    for (name, source, tests, readme) in [
        (
            "template",
            include_str!("../../../templates/spirit-ts/src/index.ts"),
            include_str!("../../../templates/spirit-ts/tests/spirit.test.ts"),
            include_str!("../../../templates/spirit-ts/README.md"),
        ),
        (
            "example",
            include_str!("../../../examples/example-spirit-ts/src/index.ts"),
            include_str!("../../../examples/example-spirit-ts/tests/spirit.test.ts"),
            include_str!("../../../examples/example-spirit-ts/README.md"),
        ),
    ] {
        assert!(
            source.contains(&format!("from {spirit}")),
            "{name} frame types must use the current generated world declaration"
        );
        assert!(
            source.contains(&format!("from {recall}")),
            "{name} recall imports must use the current WIT version"
        );
        assert!(
            tests.contains(&format!("vi.mock({recall},")),
            "{name} recall mock must target the current WIT version"
        );
        assert!(
            tests.contains(&format!("from {recall}")),
            "{name} test imports must use the current WIT version"
        );
        assert!(
            readme.contains(&readme_package) && readme.contains(&readme_recall),
            "{name} README must describe the current WIT version"
        );
    }
}
