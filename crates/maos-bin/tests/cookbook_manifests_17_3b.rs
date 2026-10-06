#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use maos_bin::admission::{gate_manifest, AdmissionRefusal};
use maos_manifest::{ClassSection, ModelProvenanceSection};

const COOKBOOK_PAGES: &[&str] = &[
    "hello-world-spirit.md",
    "manifest-fields.md",
    "compliance-claim.md",
    "wasm-component-spirit.md",
];

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

fn page_path(locale: Option<&str>, page: &str) -> PathBuf {
    let root = repository_root();
    match locale {
        Some("ko") => root
            .join("docs-site/i18n/ko/docusaurus-plugin-content-docs/current/cookbook")
            .join(page),
        Some(locale) => panic!("cookbook path: unsupported locale {locale}"),
        None => root.join("docs-site/docs/cookbook").join(page),
    }
}

fn read_page(locale: Option<&str>, page: &str) -> String {
    std::fs::read_to_string(page_path(locale, page))
        .unwrap_or_else(|error| panic!("{page}: read cookbook page: {error}"))
}

fn toml_blocks(page: &str, content: &str) -> Vec<String> {
    content
        .split("```toml\n")
        .skip(1)
        .map(|rest| {
            rest.split_once("\n```")
                .map(|(block, _)| block.to_owned())
                .unwrap_or_else(|| panic!("{page}: TOML block has a closing fence"))
        })
        .collect()
}

fn one_manifest(page: &str) -> String {
    let content = read_page(None, page);
    let blocks = toml_blocks(page, &content);
    assert_eq!(
        blocks.len(),
        1,
        "{page}: whole-manifest TOML fenced section count must be one"
    );
    blocks.into_iter().next().unwrap_or_else(|| {
        panic!("{page}: whole-manifest TOML fenced section exists after count assertion")
    })
}

fn table_body(page: &str, manifest: &str, section: &str) -> String {
    let header = format!("[{section}]");
    let (_, after_header) = manifest
        .split_once(&header)
        .unwrap_or_else(|| panic!("{page}: [{section}] table exists"));
    after_header
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.starts_with('['))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn cookbook_toml_blocks_match_korean_twins_byte_for_byte() {
    for page in COOKBOOK_PAGES {
        let english = toml_blocks(page, &read_page(None, page));
        let korean = toml_blocks(page, &read_page(Some("ko"), page));
        assert_eq!(
            english, korean,
            "{page}: TOML fenced section blocks must be byte-identical between English and Korean"
        );
    }
}

#[test]
fn cookbook_whole_manifests_pass_structural_gates_before_class_admission() {
    for page in ["hello-world-spirit.md", "manifest-fields.md"] {
        let manifest = one_manifest(page);
        let outcome = gate_manifest(&manifest, &|_| true);
        assert!(
            outcome.is_ok(),
            "{page}: whole manifest must pass every structural gate with a permissive class predicate; production admission additionally requires a daemon-known class, got {outcome:?}"
        );
    }

    let page = "wasm-component-spirit.md";
    let manifest = one_manifest(page);
    let outcome = gate_manifest(&manifest, &|_| true);
    #[cfg(not(feature = "wasm-host"))]
    assert!(
        matches!(outcome, Err(AdmissionRefusal::WasmEngineOff)),
        "{page}: whole manifest must pass structural gates with a permissive class predicate and reach wasm_engine_off, got {outcome:?}"
    );
    #[cfg(feature = "wasm-host")]
    {
        let gated = outcome.expect("component cookbook must pass structural admission");
        assert!(matches!(
            gated.require_in_process("operator door"),
            Err(AdmissionRefusal::SpawnedSurfaceUnsupported { .. })
        ));
    }
}

#[test]
fn compliance_claim_fragment_tables_parse() {
    let page = "compliance-claim.md";
    let fragment = one_manifest(page);

    let class = ClassSection::from_toml_str(&table_body(page, &fragment, "class"))
        .unwrap_or_else(|error| panic!("{page}: [class] section parses: {error}"));
    assert_eq!(
        class.trust_tier, "public-vetted",
        "{page}: [class] section declares a valid hyphenated trust tier"
    );

    let model_provenance = format!(
        "[model_provenance]\n{}",
        table_body(page, &fragment, "model_provenance")
    );
    let provenance = ModelProvenanceSection::from_manifest_toml(&model_provenance)
        .unwrap_or_else(|error| panic!("{page}: [model_provenance] section parses: {error}"));
    assert!(
        provenance.is_some(),
        "{page}: [model_provenance] section is present"
    );
}
