#![forbid(unsafe_code)]

use std::path::Path;

use maos_domain::invariants::i9::SandboxTier;
use maos_manifest::{ResourceCaps, SandboxConfig};

fn toml_block(path: &Path) -> String {
    let page = std::fs::read_to_string(path).expect("read cookbook page");
    page.split_once("```toml\n")
        .and_then(|(_, rest)| rest.split_once("\n```"))
        .map(|(block, _)| block.to_owned())
        .expect("cookbook page has a fenced TOML block")
}

fn section(block: &str, name: &str) -> String {
    let header = format!("[{name}]");
    let (_, rest) = block.split_once(&header).expect("section exists");
    rest.lines()
        .skip_while(|line| line.trim().is_empty())
        .take_while(|line| !line.starts_with('['))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn cookbook_sandbox_and_resource_sections_parse() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs-site/docs/cookbook");
    for page in ["hello-world-spirit.md", "manifest-fields.md"] {
        let block = toml_block(&root.join(page));
        let sandbox = SandboxConfig::from_toml_str(&section(&block, "sandbox"))
            .expect("cookbook sandbox section parses");
        ResourceCaps::from_toml_str(&section(&block, "resources"))
            .expect("cookbook resources section parses");
        assert_eq!(sandbox.tier, SandboxTier::T0, "{page} declares T0");
    }
}
