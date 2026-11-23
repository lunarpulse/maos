use crate::fs_walk;
use std::path::Path;
use syn::visit::Visit;
use syn::{
    Expr, ExprCall, ExprGroup, ExprLit, ExprParen, ItemConst, ItemStatic, Lit, Path as SynPath,
};

#[derive(Debug)]
struct EnvRead {
    line: usize,
    shape: &'static str,
    name: String,
    is_prefix_scan: bool,
}

#[derive(Default)]
struct EnvReadVisitor {
    reads: Vec<EnvRead>,
}

fn path_matches(path: &SynPath, expected: &[&str]) -> bool {
    path.segments.len() == expected.len()
        && path
            .segments
            .iter()
            .zip(expected)
            .all(|(segment, expected_segment)| segment.ident == *expected_segment)
}

fn literal_string(expr: &Expr) -> Option<&syn::LitStr> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(value),
            ..
        }) => Some(value),
        Expr::Paren(ExprParen { expr, .. }) | Expr::Group(ExprGroup { expr, .. }) => {
            literal_string(expr)
        }
        _ => None,
    }
}

fn read_shape(function: &Expr) -> Option<(&'static str, bool)> {
    let Expr::Path(path) = function else {
        return None;
    };

    if path_matches(&path.path, &["env", "var"]) || path_matches(&path.path, &["std", "env", "var"])
    {
        Some(("env::var", false))
    } else if path_matches(&path.path, &["env", "var_os"])
        || path_matches(&path.path, &["std", "env", "var_os"])
    {
        Some(("env::var_os", false))
    } else if path_matches(&path.path, &["duration_ms_from_env"]) {
        Some(("duration_ms_from_env", false))
    } else if path_matches(&path.path, &["any_env_with_prefix"]) {
        Some(("any_env_with_prefix", true))
    } else {
        None
    }
}

impl<'ast> Visit<'ast> for EnvReadVisitor {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Some((shape, is_prefix_scan)) = read_shape(&node.func) {
            if let Some(value) = node.args.first().and_then(literal_string) {
                let name = value.value();
                if name.starts_with("MAOS_") {
                    self.reads.push(EnvRead {
                        line: value.span().start().line,
                        shape,
                        name,
                        is_prefix_scan,
                    });
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    // 22-1 AC2: a name held in a `const`/`static` `&str` is read through the
    // constant (`read_override(ENV_ENDPOINT)`), which no literal-argument match sees.
    fn visit_item_const(&mut self, node: &'ast ItemConst) {
        self.push_const(&node.ty, &node.expr);
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_static(&mut self, node: &'ast ItemStatic) {
        self.push_const(&node.ty, &node.expr);
        syn::visit::visit_item_static(self, node);
    }
}

impl EnvReadVisitor {
    fn push_const(&mut self, ty: &syn::Type, expr: &Expr) {
        let syn::Type::Reference(reference) = ty else {
            return;
        };
        let is_str = matches!(&*reference.elem, syn::Type::Path(p) if p.path.is_ident("str"));
        let Some(value) = literal_string(expr).filter(|_| is_str) else {
            return;
        };
        let name = value.value();
        let shaped = name.len() > "MAOS_".len()
            && name.starts_with("MAOS_")
            && name
                .bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
        if shaped {
            self.reads.push(EnvRead {
                line: value.span().start().line,
                shape: "const &str",
                is_prefix_scan: name.ends_with('_'),
                name,
            });
        }
    }
}

fn detect_env_reads(source: &str) -> Result<Vec<EnvRead>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut visitor = EnvReadVisitor::default();
    visitor.visit_file(&file);
    Ok(visitor.reads)
}

pub fn run(maos_bin_dir: &str, json: bool) -> Result<(), String> {
    let env_contract_path = Path::new(maos_bin_dir).join("src/env_contract.rs");
    if !env_contract_path.exists() {
        return Err(format!(
            "check-env-contract: env_contract.rs not found at {}",
            env_contract_path.display()
        ));
    }

    let contract_src = std::fs::read_to_string(&env_contract_path).map_err(|e| {
        format!(
            "check-env-contract: read {}: {e}",
            env_contract_path.display()
        )
    })?;
    let registered: Vec<&str> = contract_src
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("name: \"MAOS_") {
                let start = trimmed.find('"')? + 1;
                let end = trimmed[start..].find('"')? + start;
                Some(&trimmed[start..end])
            } else {
                None
            }
        })
        .collect();

    let mut violations = Vec::new();

    // 22-1 D-22-1-08 B: the product crates and spirits, rooted from `--maos-bin-dir`
    // (`<root>/crates/maos-bin`), so the CI invocation is unchanged. xtask is harness.
    let mut rs_files = Vec::new();
    fs_walk::collect_rs_files(&Path::new(maos_bin_dir).join("src"), &mut rs_files);
    if let Some(root) = Path::new(maos_bin_dir).parent().and_then(Path::parent) {
        for group in ["crates", "spirits"] {
            let Ok(members) = std::fs::read_dir(root.join(group)) else {
                continue;
            };
            let mut members: Vec<_> = members.flatten().map(|m| m.path()).collect();
            members.sort();
            for member in members {
                if member != Path::new(maos_bin_dir) {
                    fs_walk::collect_rs_files(&member.join("src"), &mut rs_files);
                }
            }
        }
    }

    for file_path in &rs_files {
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "env_contract.rs" {
            continue;
        }
        let contents = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let reads = detect_env_reads(&contents).map_err(|error| {
            format!(
                "check-env-contract: parse {} for env reads: {error}",
                file_path.display()
            )
        })?;

        for read in reads {
            if read.is_prefix_scan {
                if !read.name.ends_with('_')
                    || !registered.iter().any(|name| name.starts_with(&read.name))
                {
                    violations.push(format!(
                        "{}:{}: {}(\"{}\") requires an underscore-terminated prefix with a registered env_contract.rs member",
                        file_path.display(),
                        read.line,
                        read.shape,
                        read.name
                    ));
                }
            } else if !registered.contains(&read.name.as_str()) {
                violations.push(format!(
                    "{}:{}: {}(\"{}\") not in env_contract.rs registry",
                    file_path.display(),
                    read.line,
                    read.shape,
                    read.name
                ));
            }
        }
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "passed": violations.is_empty(),
                "registered_count": registered.len(),
                "violations": violations,
            })
        );
    }

    if violations.is_empty() {
        eprintln!(
            "check-env-contract: PASS ({} MAOS_* vars registered, 0 violations across maos-bin, the product crates and the spirits; xtask env stays harness)",
            registered.len()
        );
        Ok(())
    } else {
        for v in &violations {
            eprintln!("  VIOLATION: {v}");
        }
        Err(format!(
            "check-env-contract: FAIL — {} unregistered MAOS_* env reads (maos-bin, the product crates and the spirits)",
            violations.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::run;
    use std::fs;

    fn write_contract(root: &std::path::Path, registered: &[&str]) {
        let entries = registered
            .iter()
            .map(|name| format!("    EnvVar {{\n        name: \"{name}\",\n    }},"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("src/env_contract.rs"),
            format!("pub const MAOS_ENV_REGISTRY: &[EnvVar] = &[\n{entries}\n];\n"),
        )
        .unwrap();
    }

    #[test]
    fn unregistered_multiline_raw_helper_read_fails() {
        let temp = tempfile::tempdir().unwrap();
        write_contract(temp.path(), &[]);
        fs::write(
            temp.path().join("src/runtime.rs"),
            r##"
fn fixture() {
let _ = duration_ms_from_env(
    r#"MAOS_UNREGISTERED_HELPER"#,
    DEFAULT_REFRESH_INTERVAL,
);
}
"##,
        )
        .unwrap();

        assert!(run(temp.path().to_str().unwrap(), false).is_err());
    }

    #[test]
    fn registered_multiline_raw_and_parenthesized_reads_stay_green() {
        let temp = tempfile::tempdir().unwrap();
        write_contract(
            temp.path(),
            &["MAOS_DIRECT", "MAOS_OS", "MAOS_HELPER", "MAOS_SSO_JWKS"],
        );
        fs::write(
            temp.path().join("src/runtime.rs"),
            r##"
fn fixture() {
let _ = std::env::var(
    ("MAOS_DIRECT"),
);
let _ = std::env::var_os(
    r#"MAOS_OS"#,
);
let _ = duration_ms_from_env(
    r#"MAOS_HELPER"#,
    DEFAULT_REFRESH_INTERVAL,
);
let _ = any_env_with_prefix(
    ("MAOS_SSO_"),
);
}
"##,
        )
        .unwrap();

        assert!(run(temp.path().to_str().unwrap(), false).is_ok());
    }

    #[test]
    fn non_read_literals_and_registered_prefix_scans_stay_green() {
        let temp = tempfile::tempdir().unwrap();
        write_contract(
            temp.path(),
            &["MAOS_SSO_JWKS", "MAOS_KMS_MASTER_KEY", "MAOS_SIEM_FILE"],
        );
        fs::write(
            temp.path().join("src/runtime.rs"),
            r##"
fn fixture() {
std::env::set_var("MAOS_SUPERVISION_FAST", "1");
std::env::set_var("MAOS_SCHEDULE_FAST", "1");
Command::new("worker").env("MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT", "1");
Command::new("worker").env("MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT", "0");
// std::env::var("MAOS_COMMENT");
/* std::env::var("MAOS_BLOCK_COMMENT"); */
let documentation = r#"std::env::var("MAOS_STRING_CONTENT")"#;
testenv::var("MAOS_SUFFIX_MODULE");
let _ = any_env_with_prefix("MAOS_SSO_");
let _ = any_env_with_prefix("MAOS_KMS_");
let _ = any_env_with_prefix("MAOS_SIEM_");
}
"##,
        )
        .unwrap();

        assert!(run(temp.path().to_str().unwrap(), false).is_ok());
    }

    #[test]
    fn partial_prefix_scan_fails() {
        let temp = tempfile::tempdir().unwrap();
        write_contract(temp.path(), &["MAOS_SSO_JWKS"]);
        fs::write(
            temp.path().join("src/runtime.rs"),
            "fn fixture() { let _ = any_env_with_prefix(\"MAOS_SSO_JW\"); }\n",
        )
        .unwrap();

        assert!(run(temp.path().to_str().unwrap(), false).is_err());
    }

    /// `<root>/crates/maos-bin` with a contract, plus a sibling crate's `src/<file>`.
    fn workspace(
        registered: &[&str],
        crate_name: &str,
        file: &str,
        source: &str,
    ) -> (tempfile::TempDir, String) {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("crates/maos-bin");
        write_contract(&bin, registered);
        let dir = temp.path().join("crates").join(crate_name).join("src");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(file), source).unwrap();
        let bin = bin.to_str().unwrap().to_string();
        (temp, bin)
    }

    #[test]
    fn unregistered_read_in_a_sibling_crate_fails_and_a_registered_one_passes() {
        for crate_name in ["maos-domain", "maos-kernel-core"] {
            let plant = "fn f() { let _ = std::env::var(\"MAOS_X_PLANT\"); }\n";
            let (_t, bin) = workspace(&[], crate_name, "plant.rs", plant);
            assert!(
                run(&bin, false).is_err(),
                "a planted read in {crate_name}/src must red"
            );
            let registered = "fn f() { let _ = std::env::var(\"MAOS_REGION_HOME\"); }\n";
            let (_t, bin) = workspace(&["MAOS_REGION_HOME"], crate_name, "ok.rs", registered);
            assert!(
                run(&bin, false).is_ok(),
                "{crate_name}: a registered read stays green"
            );
        }
    }

    #[test]
    fn const_held_names_are_detected_in_sibling_crates() {
        let source = "pub const ENV_ENDPOINT: &str = \"MAOS_OPERATOR_HTTP_ENDPOINT\";\n\
                      static OTHER: &str = \"MAOS_OTHER_STATIC\";\n\
                      const NOT_A_NAME: &str = \"maos_lower\";\n";
        let (_t, bin) = workspace(&[], "maos-cli", "door_client.rs", source);
        assert!(
            run(&bin, false).is_err(),
            "an unregistered const name must red"
        );
        let (_t, bin) = workspace(
            &["MAOS_OPERATOR_HTTP_ENDPOINT", "MAOS_OTHER_STATIC"],
            "maos-cli",
            "door_client.rs",
            source,
        );
        assert!(
            run(&bin, false).is_ok(),
            "registered const names stay green"
        );
    }
}
