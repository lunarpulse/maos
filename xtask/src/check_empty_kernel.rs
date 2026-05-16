use std::fs;
use std::path::Path;
use syn::visit::Visit;

use crate::fs_walk;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub struct_name: String,
    pub field_name: String,
    pub field_type: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "I9 violation: persistent struct {} not in I9 whitelist at {}:{} (field: {}: {})",
            self.struct_name, self.file, self.line, self.field_name, self.field_type
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExemptionViolation {
    pub file: String,
    pub line: usize,
    pub struct_name: String,
}

impl std::fmt::Display for ExemptionViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "I9 violation: #[i9_exempt] at {}:{} not documented in docs/invariants/i9-exemptions.md",
            self.file, self.line
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub exemption_violations: Vec<ExemptionViolation>,
}

#[derive(Debug, serde::Deserialize)]
struct Whitelist {
    paths: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Denylist {
    types: Vec<String>,
}

pub fn run(
    path: &str,
    whitelist_path: &str,
    denylist_path: &str,
    exemptions_path: &str,
    json: bool,
) -> Result<(), String> {
    let report = run_silent(
        Path::new(path),
        Path::new(whitelist_path),
        Path::new(denylist_path),
        Path::new(exemptions_path),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("check-empty-kernel: PASSED (0 violations)");
        } else {
            for v in &report.violations {
                eprintln!("{v}");
            }
            for ev in &report.exemption_violations {
                eprintln!("{ev}");
            }
        }
    }

    if !report.passed {
        return Err("check-empty-kernel failed".into());
    }

    Ok(())
}

pub(crate) fn run_silent(
    kernel_path: &Path,
    whitelist_path: &Path,
    denylist_path: &Path,
    exemptions_path: &Path,
) -> Result<Report, String> {
    let whitelist: Whitelist = load_toml(whitelist_path)?;
    let denylist: Denylist = load_toml(denylist_path)?;

    let mut violations = Vec::new();
    let mut exemption_sites = Vec::new();

    let mut rs_files = Vec::new();
    fs_walk::collect_rs_files(kernel_path, &mut rs_files);

    for file in &rs_files {
        let src = fs::read_to_string(file)
            .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        let ast = syn::parse_file(&src)
            .map_err(|e| format!("parse error in {}: {e}", file.display()))?;

        let file_str = file.display().to_string();
        let in_whitelist = whitelist.paths.iter().any(|p| {
            // Tolerate non-existent whitelist entries by checking path-prefix match.
            file_str == p.as_str() || file_str.starts_with(&format!("{p}/")) || (Path::new(p).is_dir() && file_str.starts_with(p))
        });

        let mut visitor = EmptyKernelVisitor {
            file: file_str.clone(),
            in_whitelist,
            denylist: &denylist.types,
            violations: &mut violations,
            exemption_sites: &mut exemption_sites,
        };
        visitor.visit_file(&ast);
    }

    // Cross-check exemptions against docs/invariants/i9-exemptions.md
    let mut exemption_violations = Vec::new();
    if exemptions_path.exists() {
        let exemptions_src = fs::read_to_string(exemptions_path)
            .map_err(|e| format!("cannot read {}: {e}", exemptions_path.display()))?;
        for site in &exemption_sites {
            let key = format!("{}::{}::{}::", site.crate_name, site.module_path, site.struct_name);
            let trimmed_key = key.trim_end_matches(':').to_string();
            if !exemptions_src.lines().any(|line| line.contains(&trimmed_key))
                && !exemptions_src.lines().any(|line| line.contains(&site.struct_name))
            {
                exemption_violations.push(ExemptionViolation {
                    file: site.file.clone(),
                    line: site.line,
                    struct_name: site.struct_name.clone(),
                });
            }
        }
    } else if !exemption_sites.is_empty() {
        for site in &exemption_sites {
            exemption_violations.push(ExemptionViolation {
                file: site.file.clone(),
                line: site.line,
                struct_name: site.struct_name.clone(),
            });
        }
    }

    let passed = violations.is_empty() && exemption_violations.is_empty();
    Ok(Report {
        passed,
        violations,
        exemption_violations,
    })
}

fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let src = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&src).map_err(|e| format!("toml parse error in {}: {e}", path.display()))
}

struct ExemptionSite {
    file: String,
    line: usize,
    crate_name: String,
    module_path: String,
    struct_name: String,
}

struct EmptyKernelVisitor<'a> {
    file: String,
    in_whitelist: bool,
    denylist: &'a [String],
    violations: &'a mut Vec<Violation>,
    exemption_sites: &'a mut Vec<ExemptionSite>,
}

impl<'a> Visit<'_> for EmptyKernelVisitor<'a> {
    fn visit_item_struct(&mut self, node: &syn::ItemStruct) {
        let struct_name = node.ident.to_string();
        let line = node.ident.span().start().line;

        let is_exempt = has_i9_exempt(&node.attrs);
        if is_exempt {
            self.exemption_sites.push(ExemptionSite {
                file: self.file.clone(),
                line,
                crate_name: "maos_kernel_core".into(),
                module_path: infer_module_path(&self.file),
                struct_name: struct_name.clone(),
            });
            // Exempt structs are skipped for field checks, but nested items still visited.
            syn::visit::visit_item_struct(self, node);
            return;
        }

        if self.in_whitelist {
            // Struct is in a whitelisted path; skip denylist check.
            return;
        }

        for field in &node.fields {
            if let Some(ident) = &field.ident {
                let field_name = ident.to_string();
                let type_str = normalize_type(&field.ty);
                if is_denylisted(&type_str, self.denylist) {
                    self.violations.push(Violation {
                        file: self.file.clone(),
                        line,
                        struct_name: struct_name.clone(),
                        field_name,
                        field_type: type_str,
                    });
                }
            }
        }

        syn::visit::visit_item_struct(self, node);
    }
}

fn has_i9_exempt(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let segments = &attr.path().segments;
        let is_i9_exempt = attr.path().is_ident("i9_exempt")
            || segments.last().map(|s| s.ident == "i9_exempt").unwrap_or(false);
        if is_i9_exempt {
            if let Ok(meta) = attr.meta.require_list() {
                let tokens = meta.tokens.to_string();
                return tokens.contains("reason");
            }
        }
        false
    })
}

fn normalize_type(ty: &syn::Type) -> String {
    let rendered = quote::quote!(#ty).to_string();
    // Normalize whitespace.
    rendered.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_denylisted(type_str: &str, denylist: &[String]) -> bool {
    // Compact form (no spaces) for robust matching.
    let compact = type_str.replace(' ', "");

    // Check Vec<T> carve-out first.
    if let Some(inner) = extract_vec_inner(&compact) {
        return !is_primitive_type(inner);
    }

    for denied in denylist {
        let d = denied.replace(' ', "");
        if compact == d
            || compact.starts_with(&format!("{d}<"))
            || compact.starts_with(&format!("std::sync::{d}"))
            || compact.starts_with(&format!("std::cell::{d}"))
            || compact.starts_with(&format!("std::collections::{d}"))
            || compact.starts_with(&format!("alloc::sync::{d}"))
            || compact.starts_with(&format!("alloc::collections::{d}"))
            || compact.starts_with(&format!("core::sync::atomic::{d}"))
            || compact.starts_with(&format!("std::sync::atomic::{d}"))
        {
            return true;
        }
    }
    false
}

fn extract_vec_inner(compact: &str) -> Option<&str> {
    // Handle path-qualified Vec (including bare Vec).
    if let Some(pos) = compact.find("Vec<") {
        let after = &compact[pos + 4..];
        // Bracket-depth matching for nested generics.
        let mut depth = 1u32;
        let mut end = 0;
        for (i, ch) in after.char_indices() {
            match ch {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth == 0 {
            return Some(after[..end].trim());
        }
    }
    None
}

fn is_primitive_type(type_str: &str) -> bool {
    matches!(
        type_str,
        "u8" | "u16" | "u32" | "u64"
            | "i8" | "i16" | "i32" | "i64"
            | "usize" | "isize"
            | "f32" | "f64"
            | "bool"
    )
}

fn infer_module_path(file_path: &str) -> String {
    // Extract module path from file path, e.g.
    // crates/maos-kernel-core/src/capability/cap_policy/mod.rs -> capability::cap_policy
    if let Some(src_pos) = file_path.find("/src/") {
        let after_src = &file_path[src_pos + 5..];
        let without_ext = after_src.strip_suffix(".rs").unwrap_or(after_src);
        let parts: Vec<_> = without_ext.split('/').collect();
        if parts.last() == Some(&"mod") {
            let mod_parts = &parts[..parts.len() - 1];
            return mod_parts.join("::");
        }
        parts.join("::")
    } else {
        // Fallback: strip known prefixes and .rs extension, convert slashes.
        let without_ext = file_path.strip_suffix(".rs").unwrap_or(file_path);
        without_ext.replace('/', "::").replace('\\', "::")
    }
}

#[cfg(test)]
mod tests {
    include!("tests/check_empty_kernel_tests.rs");
}
