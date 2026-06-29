use std::fs;
use std::path::{Path, PathBuf};

// Retired smoke-arm payload values. These are literal topology/arm names that
// were formerly embedded in production code and later replaced by configurable
// topology files. This list does NOT track the `MAOS_ONE_SHOT` env-var
// mechanism — it only catches stale hard-coded arm-name strings that should
// have been removed when the topology was factored out.
const FORBIDDEN: &[&str] = &["smoke-founder-loop", "smoke-mira-nash"];

#[derive(Debug)]
struct Violation {
    path: PathBuf,
    line: usize,
    literal: &'static str,
}

pub fn run(path: &str, json: bool) -> Result<(), String> {
    let root = Path::new(path);
    let mut violations = Vec::new();
    scan_dir(root, &mut violations);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": "check-literal-reappearance",
                "passed": violations.is_empty(),
                "violations": violations.iter().map(|v| serde_json::json!({
                    "path": v.path.display().to_string(),
                    "line": v.line,
                    "literal": v.literal,
                })).collect::<Vec<_>>()
            })
        );
    } else if violations.is_empty() {
        println!("check-literal-reappearance: PASS");
    } else {
        eprintln!("check-literal-reappearance: FAIL");
        for v in &violations {
            eprintln!(
                "{}:{} contains forbidden literal {}",
                v.path.display(),
                v.line,
                v.literal
            );
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "check-literal-reappearance found {} forbidden literal(s)",
            violations.len()
        ))
    }
}

fn scan_dir(path: &Path, violations: &mut Vec<Violation>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if matches!(name, "target" | ".git") {
                continue;
            }
            scan_dir(&path, violations);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs")
            && !is_test_or_bench_file(&path)
        {
            scan_file(&path, violations);
        }
    }
}

/// Returns true for integration-test and benchmark files that should be
/// skipped: `*/tests/*.rs` and `*/benches/*.rs`. Production `src/*.rs`
/// files are always scanned even when they contain `#[cfg(test)]` blocks.
fn is_test_or_bench_file(path: &Path) -> bool {
    let components: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    for (i, comp) in components.iter().enumerate() {
        if (comp == "tests" || comp == "benches")
            && i + 1 < components.len()
            && components[i + 1].ends_with(".rs")
        {
            return true;
        }
    }
    false
}

fn scan_file(path: &Path, violations: &mut Vec<Violation>) {
    let Ok(src) = fs::read_to_string(path) else {
        return;
    };
    // Track whether we are inside a `#[cfg(test)]` module to avoid flagging
    // test-only references. This is a simple brace-depth heuristic, not a
    // full parser: it looks for `#[cfg(test)]` followed by `mod … {` and
    // counts braces until the module closes.
    let mut in_cfg_test: Option<usize> = None; // Some(depth) when inside
    let mut saw_cfg_test_attr = false;

    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();

        // Detect `#[cfg(test)]` attribute
        if trimmed == "#[cfg(test)]" {
            saw_cfg_test_attr = true;
            continue;
        }

        // If we just saw `#[cfg(test)]` and this line opens a mod block,
        // start tracking brace depth. We initialize to 0 and let the
        // tracking block below count this line's braces to avoid
        // double-counting.
        if saw_cfg_test_attr {
            saw_cfg_test_attr = false;
            if trimmed.starts_with("mod ") && trimmed.contains('{') {
                in_cfg_test = Some(0);
                // Fall through to the brace-depth tracking block below.
            }
        }

        // Track brace depth inside #[cfg(test)] module
        if let Some(depth) = in_cfg_test {
            let opens = trimmed.chars().filter(|&c| c == '{').count();
            let closes = trimmed.chars().filter(|&c| c == '}').count();
            let new_depth = (depth + opens).saturating_sub(closes);
            if new_depth == 0 {
                in_cfg_test = None;
            } else {
                in_cfg_test = Some(new_depth);
            }
            // Skip flagging lines inside #[cfg(test)] modules
            continue;
        }

        for literal in FORBIDDEN {
            if line.contains(literal) {
                violations.push(Violation {
                    path: path.to_path_buf(),
                    line: idx + 1,
                    literal,
                });
            }
        }
    }
}
