// TODO(story-0.1-review): Migrate from custom syn-based ABI parser to cargo-public-api
// by Story 1a.1's ABI freeze. The custom syn+quote approach is adequate for v0.1-alpha's
// 3-item ABI surface but quote!-based signatures are fragile across toolchain versions.
// See: https://github.com/..../cargo-public-api for --diff-git-checkouts usage.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiItem {
    pub kind: String,
    pub name: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AbiSnapshot {
    pub abi_version: u32,
    pub items: Vec<ApiItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffReport {
    pub passed: bool,
    pub added: Vec<ApiItem>,
    pub removed: Vec<ApiItem>,
    pub changed: Vec<(ApiItem, ApiItem)>,
    pub abi_version_current: u32,
    pub abi_version_baseline: u32,
}

pub fn run(base: &str, json: bool) -> Result<(), String> {
    let report = abi_diff(base)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("abi-diff: PASSED (no breaking changes)");
        } else {
            eprintln!("ABI-diff violation: breaking change detected without abi_version bump");
            eprintln!("  baseline abi_version: {}", report.abi_version_baseline);
            eprintln!("  current abi_version:  {}", report.abi_version_current);
            for item in &report.removed {
                eprintln!("  [-] {} {}", item.kind, item.name);
            }
            for (old, new) in &report.changed {
                eprintln!("  [~] {} {} (was: {}, now: {})", old.kind, old.name, old.signature, new.signature);
            }
        }
    }

    if !report.passed {
        return Err("abi-diff failed".into());
    }

    Ok(())
}

fn abi_diff(base: &str) -> Result<DiffReport, String> {
    // --base can be a path to a baseline JSON file, a git ref, or "HEAD~1" (default).
    // When a file path is provided, use it directly. Otherwise, use the canonical baseline.
    let baseline_path = if Path::new(base).exists() && base.ends_with(".json") {
        base.to_string()
    } else {
        "abi-baseline/v0.1-alpha-pre-abi-freeze.json".to_string()
    };
    let baseline: AbiSnapshot = load_snapshot(&baseline_path)?;
    let current = snapshot_abi("crates/maos-spirit-abi")?;

    let baseline_set: BTreeMap<String, &ApiItem> = baseline
        .items
        .iter()
        .map(|i| (format!("{}::{}", i.kind, i.name), i))
        .collect();
    let current_set: BTreeMap<String, &ApiItem> = current
        .items
        .iter()
        .map(|i| (format!("{}::{}", i.kind, i.name), i))
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (key, item) in &current_set {
        if let Some(&old) = baseline_set.get(key) {
            if old.signature != item.signature {
                changed.push((old.clone(), (*item).clone()));
            }
        } else {
            added.push((*item).clone());
        }
    }

    for (key, item) in &baseline_set {
        if !current_set.contains_key(key) {
            removed.push((*item).clone());
        }
    }

    let has_breaking = !removed.is_empty() || !changed.is_empty();
    let version_bumped = current.abi_version > baseline.abi_version;
    let passed = !has_breaking || version_bumped;

    Ok(DiffReport {
        passed,
        added: added.into_iter().collect(),
        removed: removed.into_iter().collect(),
        changed,
        abi_version_current: current.abi_version,
        abi_version_baseline: baseline.abi_version,
    })
}

fn load_snapshot(path: &str) -> Result<AbiSnapshot, String> {
    let src = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    serde_json::from_str(&src).map_err(|e| format!("cannot parse {path}: {e}"))
}

pub fn snapshot_abi(crate_path: &str) -> Result<AbiSnapshot, String> {
    let mut items = Vec::new();
    let src_dir = Path::new(crate_path).join("src");
    collect_pub_items(&src_dir, &mut items)?;

    let version_file = Path::new(crate_path).join("src/version.rs");
    let abi_version = if version_file.exists() {
        extract_abi_version(&version_file)?
    } else {
        extract_abi_version_from_lib(&src_dir.join("lib.rs"))?
    };

    items.sort();
    Ok(AbiSnapshot { abi_version, items })
}

fn collect_pub_items(dir: &Path, items: &mut Vec<ApiItem>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| format!("read_dir error: {e}"))? {
        let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_pub_items(&path, items)?;
        } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
            let src = fs::read_to_string(&path)
                .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
            let ast = syn::parse_file(&src)
                .map_err(|e| format!("parse error in {}: {e}", path.display()))?;
            extract_items_from_file(&ast, items);
        }
    }
    Ok(())
}

fn extract_items_from_file(ast: &syn::File, items: &mut Vec<ApiItem>) {
    for item in &ast.items {
        match item {
            syn::Item::Fn(f) if is_pub(&f.vis) => {
                items.push(ApiItem {
                    kind: "fn".into(),
                    name: f.sig.ident.to_string(),
                    signature: quote::quote!(#f).to_string(),
                });
            }
            syn::Item::Struct(s) if is_pub(&s.vis) => {
                items.push(ApiItem {
                    kind: "struct".into(),
                    name: s.ident.to_string(),
                    signature: quote::quote!(#s).to_string(),
                });
            }
            syn::Item::Enum(e) if is_pub(&e.vis) => {
                items.push(ApiItem {
                    kind: "enum".into(),
                    name: e.ident.to_string(),
                    signature: quote::quote!(#e).to_string(),
                });
            }
            syn::Item::Trait(t) if is_pub(&t.vis) => {
                items.push(ApiItem {
                    kind: "trait".into(),
                    name: t.ident.to_string(),
                    signature: quote::quote!(#t).to_string(),
                });
            }
            syn::Item::Type(t) if is_pub(&t.vis) => {
                items.push(ApiItem {
                    kind: "type".into(),
                    name: t.ident.to_string(),
                    signature: quote::quote!(#t).to_string(),
                });
            }
            syn::Item::Const(c) if is_pub(&c.vis) => {
                items.push(ApiItem {
                    kind: "const".into(),
                    name: c.ident.to_string(),
                    signature: quote::quote!(#c).to_string(),
                });
            }
            syn::Item::Static(s) if is_pub(&s.vis) => {
                items.push(ApiItem {
                    kind: "static".into(),
                    name: s.ident.to_string(),
                    signature: quote::quote!(#s).to_string(),
                });
            }
            syn::Item::Use(u) if is_pub(&u.vis) => {
                // Flatten use tree into individual items.
                flatten_use_tree(&u.tree, items);
            }
            syn::Item::Mod(m) if is_pub(&m.vis) => {
                items.push(ApiItem {
                    kind: "mod".into(),
                    name: m.ident.to_string(),
                    signature: format!("pub mod {}", m.ident),
                });
            }
            _ => {}
        }
    }
}

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn flatten_use_tree(tree: &syn::UseTree, items: &mut Vec<ApiItem>) {
    match tree {
        syn::UseTree::Path(_p) => {
            // Skip flattening for now; record the full path.
            items.push(ApiItem {
                kind: "use".into(),
                name: quote::quote!(#tree).to_string(),
                signature: quote::quote!(#tree).to_string(),
            });
        }
        syn::UseTree::Name(n) => {
            items.push(ApiItem {
                kind: "use".into(),
                name: n.ident.to_string(),
                signature: quote::quote!(#tree).to_string(),
            });
        }
        syn::UseTree::Rename(r) => {
            items.push(ApiItem {
                kind: "use".into(),
                name: format!("{} as {}", r.ident, r.rename),
                signature: quote::quote!(#tree).to_string(),
            });
        }
        syn::UseTree::Glob(_) => {
            items.push(ApiItem {
                kind: "use".into(),
                name: "*".into(),
                signature: quote::quote!(#tree).to_string(),
            });
        }
        syn::UseTree::Group(g) => {
            for item in &g.items {
                flatten_use_tree(item, items);
            }
        }
    }
}

fn extract_abi_version(path: &Path) -> Result<u32, String> {
    let src = fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let ast = syn::parse_file(&src)
        .map_err(|e| format!("parse error in {}: {e}", path.display()))?;

    // Walk top-level items looking for `pub const ABI_VERSION: u32 = <literal>;`
    for item in &ast.items {
        if let syn::Item::Const(c) = item {
            if c.ident == "ABI_VERSION" && is_pub(&c.vis) {
                if let syn::Expr::Lit(expr_lit) = &*c.expr {
                    if let syn::Lit::Int(lit_int) = &expr_lit.lit {
                        return lit_int.base10_parse::<u32>()
                            .map_err(|e| format!("cannot parse ABI_VERSION value: {e}"));
                    }
                }
            }
        }
    }
    Err(format!("ABI_VERSION const not found in {}", path.display()))
}

fn extract_abi_version_from_lib(path: &Path) -> Result<u32, String> {
    extract_abi_version(path)
}

#[cfg(test)]
mod tests {
    include!("tests/abi_diff_tests.rs");
}
