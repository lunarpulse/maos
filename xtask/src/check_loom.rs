use std::collections::HashSet;
use std::fs;
use std::path::Path;
use syn::visit::Visit;

use crate::fs_walk;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub identifier: String,
    pub kind: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NFR-Test-9 violation: Loom-not-in-kernel grep matched '{}' at {}:{} (kind: {})",
            self.identifier, self.file, self.line, self.kind
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
}

#[derive(Debug, serde::Deserialize)]
struct KernelCrates {
    crates: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Blocklist {
    blocklist: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Allowlist {
    allowed: Vec<AllowedEntry>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct AllowedEntry {
    file: String,
    identifier: String,
}

pub fn run(
    path: Option<&str>,
    crates_path: &str,
    blocklist_path: &str,
    allowlist_path: &str,
    json: bool,
) -> Result<(), String> {
    let report = check_loom(
        path.map(Path::new),
        Path::new(crates_path),
        Path::new(blocklist_path),
        Path::new(allowlist_path),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("check-loom: PASSED (0 violations)");
        } else {
            for v in &report.violations {
                eprintln!("{v}");
            }
        }
    }

    if !report.passed {
        return Err("check-loom failed".into());
    }

    Ok(())
}

fn check_loom(
    path: Option<&Path>,
    crates_path: &Path,
    blocklist_path: &Path,
    allowlist_path: &Path,
) -> Result<Report, String> {
    let blocklist: Blocklist = load_toml(blocklist_path)?;
    let allowlist: Allowlist = if allowlist_path.exists() {
        load_toml(allowlist_path)?
    } else {
        Allowlist {
            allowed: Vec::new(),
        }
    };

    let blockset: HashSet<String> = blocklist.blocklist.into_iter().collect();
    let allowset: HashSet<(String, String)> = allowlist
        .allowed
        .into_iter()
        .map(|a| (a.file, a.identifier))
        .collect();

    let mut violations = Vec::new();

    if let Some(scan_path) = path {
        // Direct path scan (integration-test mode).
        let mut rs_files = Vec::new();
        fs_walk::collect_rs_files(scan_path, &mut rs_files);
        for file in &rs_files {
            let src = fs::read_to_string(file)
                .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            let ast = syn::parse_file(&src)
                .map_err(|e| format!("parse error in {}: {e}", file.display()))?;
            let mut visitor = LoomVisitor {
                file: file.display().to_string(),
                blockset: &blockset,
                allowset: &allowset,
                violations: &mut violations,
            };
            visitor.visit_file(&ast);
        }
    } else {
        let kernel_crates: KernelCrates = load_toml(crates_path)?;
        for crate_name in &kernel_crates.crates {
            let crate_src = Path::new("crates").join(crate_name).join("src");
            if !crate_src.exists() {
                continue;
            }
            let mut rs_files = Vec::new();
            fs_walk::collect_rs_files(&crate_src, &mut rs_files);

            for file in &rs_files {
                let src = fs::read_to_string(file)
                    .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                let ast = syn::parse_file(&src)
                    .map_err(|e| format!("parse error in {}: {e}", file.display()))?;

                let mut visitor = LoomVisitor {
                    file: file.display().to_string(),
                    blockset: &blockset,
                    allowset: &allowset,
                    violations: &mut violations,
                };
                visitor.visit_file(&ast);
            }
        }
    }

    let passed = violations.is_empty();
    Ok(Report { passed, violations })
}

fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let src =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&src).map_err(|e| format!("toml parse error in {}: {e}", path.display()))
}

struct LoomVisitor<'a> {
    file: String,
    blockset: &'a HashSet<String>,
    allowset: &'a HashSet<(String, String)>,
    violations: &'a mut Vec<Violation>,
}

impl<'a> Visit<'_> for LoomVisitor<'a> {
    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        self.check_ident(&node.ident, "ItemStruct");
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &syn::ItemEnum) {
        self.check_ident(&node.ident, "ItemEnum");
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_trait(&mut self, node: &syn::ItemTrait) {
        self.check_ident(&node.ident, "ItemTrait");
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_fn(&mut self, node: &syn::ItemFn) {
        self.check_ident(&node.sig.ident, "ItemFn");
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_item_type(&mut self, node: &syn::ItemType) {
        self.check_ident(&node.ident, "ItemType");
        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        // Skip #[cfg(test)] modules and modules named `tests`.
        let is_test_mod = node.ident == "tests"
            || node.attrs.iter().any(|attr| {
                if attr.path().is_ident("cfg") {
                    if let Ok(meta) = attr.meta.require_list() {
                        return meta.tokens.to_string().contains("test");
                    }
                }
                false
            });

        if is_test_mod {
            // Do NOT recurse into test modules.
            return;
        }

        self.check_ident(&node.ident, "ItemMod");
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_const(&mut self, node: &syn::ItemConst) {
        self.check_ident(&node.ident, "ItemConst");
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_static(&mut self, node: &syn::ItemStatic) {
        self.check_ident(&node.ident, "ItemStatic");
        syn::visit::visit_item_static(self, node);
    }

    fn visit_item_use(&mut self, node: &syn::ItemUse) {
        // Collect the rightmost path segment of each use tree.
        let names = collect_use_names(&node.tree, &mut |glob_path| {
            eprintln!(
                "NFR-Test-9 warning: glob import '{}' at {}:{} — cannot verify blocklisted symbols; review manually",
                glob_path, self.file, node.use_token.span.start().line
            );
        });
        for name in names {
            if self.blockset.contains(&name) && !self.is_allowed(&name) {
                self.violations.push(Violation {
                    file: self.file.clone(),
                    line: node.use_token.span.start().line,
                    identifier: name,
                    kind: "ItemUse".into(),
                });
            }
        }
        syn::visit::visit_item_use(self, node);
    }
}

impl<'a> LoomVisitor<'a> {
    fn check_ident(&mut self, ident: &proc_macro2::Ident, kind: &str) {
        let name = ident.to_string();
        if self.blockset.contains(&name) && !self.is_allowed(&name) {
            self.violations.push(Violation {
                file: self.file.clone(),
                line: ident.span().start().line,
                identifier: name,
                kind: kind.into(),
            });
        }
    }

    fn is_allowed(&self, ident: &str) -> bool {
        self.allowset
            .iter()
            .any(|(f, i)| self.file.ends_with(f) && i == ident)
    }
}

fn collect_use_names(tree: &syn::UseTree, on_glob: &mut dyn FnMut(&str)) -> Vec<String> {
    match tree {
        syn::UseTree::Name(name) => vec![name.ident.to_string()],
        syn::UseTree::Rename(rename) => vec![rename.rename.to_string()],
        syn::UseTree::Glob(_glob) => {
            let path = reconstruct_use_path(tree);
            on_glob(&path);
            Vec::new()
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .flat_map(|t| collect_use_names(t, on_glob))
            .collect(),
        syn::UseTree::Path(path) => collect_use_names(&path.tree, on_glob),
    }
}

fn reconstruct_use_path(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => {
            let rest = reconstruct_use_path(&p.tree);
            if rest.is_empty() {
                p.ident.to_string()
            } else {
                format!("{}::{}", p.ident, rest)
            }
        }
        syn::UseTree::Glob(_) => String::new(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    include!("tests/check_loom_tests.rs");
}
