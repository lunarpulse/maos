use std::collections::HashSet;
use std::fs;
use std::path::Path;
use syn::visit::Visit;

use crate::corpus_types::{load_toml, JudgeConfig, JudgeDirectCallIdentifiers};
use crate::fs_walk;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub kind: String,
    pub judge: String,
    pub detail: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.kind == "direct_call" {
            let file = self.file.as_deref().unwrap_or("?");
            let line = self.line.map(|l| l.to_string()).unwrap_or_else(|| "?".into());
            return write!(f, "NFR-Test-1 violation: direct judge-LLM call at {file}:{line}: route via JudgeRunner trait + tests/judge-config.toml (epic-0 / Story 0.3 BDD2)");
        }
        let (field, req) = match self.kind.as_str() {
            "temperature" => ("temperature", "temperature=0.0"),
            "top_p" => ("top_p", "top_p=1.0"),
            "seed" => return write!(f, "NFR-Test-1 violation: judge '{}' missing seed; pinned-judge contract requires seed: u64 (epic-0 / Story 0.3 BDD2)", self.judge),
            "retry_budget" => ("retry_budget", "retry_budget=1"),
            "prompt_version_hash" => ("prompt_version_hash", "64-hex SHA-256"),
            "model" => ("model", "provider:model_id@version format"),
            _ => return write!(f, "NFR-Test-1 violation: {} — {}: {}", self.kind, self.judge, self.detail),
        };
        write!(f, "NFR-Test-1 violation: judge '{}' has {}={}; pinned-judge contract requires {} (epic-0 / Story 0.3 BDD2)",
            self.judge, field, self.detail, req)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
    pub checked: usize,
}

pub fn run(config_path: &str, identifiers_path: &str, json: bool) -> Result<(), String> {
    let report = check_judge_config(Path::new(config_path), Path::new(identifiers_path))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_else(|e| format!("{{\"error\":\"json serialize failed: {e}\"}}")));
    } else if report.passed {
        println!("check-judge-config: PASSED ({} entries checked)", report.checked);
    } else {
        for v in &report.violations { eprintln!("{v}"); }
    }
    if !report.passed { return Err("check-judge-config failed".into()); }
    Ok(())
}

fn check_judge_config(config_path: &Path, identifiers_path: &Path) -> Result<Report, String> {
    let config: JudgeConfig = load_toml(config_path)?;
    let identifiers: JudgeDirectCallIdentifiers = if identifiers_path.exists() {
        load_toml(identifiers_path)?
    } else {
        JudgeDirectCallIdentifiers { direct_calls: Vec::new() }
    };

    let mut violations = Vec::new();
    let checked = config.judge.len();

    for (name, entry) in &config.judge {
        let mut push = |kind: &str, detail: &str| {
            violations.push(Violation { kind: kind.into(), judge: name.clone(), detail: detail.into(), file: None, line: None });
        };
        if entry.seed.is_none() { push("seed", "missing"); }
        if entry.temperature != 0.0 { push("temperature", &entry.temperature.to_string()); }
        if entry.top_p != 1.0 { push("top_p", &entry.top_p.to_string()); }
        if entry.retry_budget != 1 { push("retry_budget", &entry.retry_budget.to_string()); }
        if !is_hex_64(&entry.prompt_version_hash) { push("prompt_version_hash", &entry.prompt_version_hash); }
        if !is_model_format(&entry.model) { push("model", &entry.model); }
    }

    let call_set: HashSet<String> = identifiers.direct_calls.into_iter().collect();
    if !call_set.is_empty() {
        let mut test_files = Vec::new();
        let tests_dir = Path::new("tests");
        if tests_dir.exists() { fs_walk::collect_rs_files(tests_dir, &mut test_files); }
        let crates_tests = Path::new("crates");
        if crates_tests.exists() {
            for entry in fs::read_dir(crates_tests).map_err(|e| format!("cannot read crates dir: {e}"))? {
                let entry = entry.map_err(|e| format!("cannot iterate crates dir entry: {e}"))?;
                let tests_dir = entry.path().join("tests");
                if tests_dir.exists() { fs_walk::collect_rs_files(&tests_dir, &mut test_files); }
            }
        }
        for file in &test_files {
            let src = fs::read_to_string(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
            let ast = syn::parse_file(&src).map_err(|e| format!("parse error in {}: {e}", file.display()))?;
            let mut visitor = DirectCallVisitor { file: file.display().to_string(), call_set: &call_set, violations: &mut violations, in_cfg_test: false, skip_xtask: file.components().next().map(|c| c.as_os_str() == "xtask").unwrap_or(false) };
            visitor.visit_file(&ast);
        }
    }

    let passed = violations.is_empty();
    Ok(Report { passed, violations, checked })
}

fn is_hex_64(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

fn is_model_format(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() < 2 { return false; }
    let provider = parts[0];
    let rest = &s[provider.len() + 1..];
    if provider.is_empty() || !provider.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') { return false; }
    if rest.is_empty() { return false; }
    rest.chars().all(|c| c.is_ascii_alphanumeric() || "._:/@-".contains(c))
}

struct DirectCallVisitor<'a> {
    file: String,
    call_set: &'a HashSet<String>,
    violations: &'a mut Vec<Violation>,
    in_cfg_test: bool,
    skip_xtask: bool,
}

impl<'a> DirectCallVisitor<'a> {
    fn check(&mut self, name: &str, line: usize) {
        if self.call_set.contains(name) && !(self.skip_xtask && self.in_cfg_test) {
            self.violations.push(Violation { kind: "direct_call".into(), judge: name.into(), detail: String::new(), file: Some(self.file.clone()), line: Some(line) });
        }
    }
}

impl<'a> Visit<'_> for DirectCallVisitor<'a> {
    fn visit_item_mod(&mut self, node: &syn::ItemMod) {
        let was = self.in_cfg_test;
        if node.attrs.iter().any(|attr| attr.path().is_ident("cfg") && attr.meta.require_list().map(|m| m.tokens.to_string().contains("test")).unwrap_or(false)) {
            self.in_cfg_test = true;
        }
        syn::visit::visit_item_mod(self, node);
        self.in_cfg_test = was;
    }

    fn visit_expr_call(&mut self, node: &syn::ExprCall) {
        if let syn::Expr::Path(expr_path) = &*node.func {
            if let Some(seg) = expr_path.path.segments.last() {
                self.check(&seg.ident.to_string(), seg.ident.span().start().line);
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_path(&mut self, node: &syn::ExprPath) {
        if let Some(seg) = node.path.segments.last() {
            self.check(&seg.ident.to_string(), seg.ident.span().start().line);
        }
        syn::visit::visit_expr_path(self, node);
    }
}

#[cfg(test)]
mod tests { include!("tests/check_judge_config_tests.rs"); }
