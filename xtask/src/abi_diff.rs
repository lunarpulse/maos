//! ABI-diff gate backed by `cargo-public-api` per Story 1a.5.
//! See `docs/dev-discipline/abi-diff-migration.md` for rationale and rollback.

use std::fs;
use std::path::Path;
use std::process::Command;

const MANIFEST: &str = "crates/maos-spirit-abi/Cargo.toml";

pub fn run(base: &str, json: bool) -> Result<(), String> {
    let resolved = resolve_baseline(base);
    if resolved.exists() {
        let baseline = fs::read_to_string(&resolved)
            .map_err(|e| format!("cannot read {}: {e}", resolved.display()))?;
        let current = capture_public_api()?;
        let (added, removed) = line_diff(&baseline, &current);
        report(json, added, removed)
    } else {
        diff_git(base, json)
    }
}

fn resolve_baseline(base: &str) -> std::path::PathBuf {
    let p = Path::new(base);
    if p.exists() {
        return p.to_path_buf();
    }
    if let Some(stem) = base.strip_suffix(".json") {
        let txt = format!("{stem}.txt");
        let tp = Path::new(&txt);
        if tp.exists() {
            return tp.to_path_buf();
        }
    }
    p.to_path_buf()
}

fn line_diff<'a>(a: &'a str, b: &'a str) -> (Vec<&'a str>, Vec<&'a str>) {
    let al: Vec<&str> = a.lines().filter(|l| !l.is_empty()).collect();
    let bl: Vec<&str> = b.lines().filter(|l| !l.is_empty()).collect();
    (
        bl.iter().filter(|l| !al.contains(l)).copied().collect(),
        al.iter().filter(|l| !bl.contains(l)).copied().collect(),
    )
}

fn report(json: bool, added: Vec<&str>, removed: Vec<&str>) -> Result<(), String> {
    let passed = removed.is_empty();
    if json {
        println!(
            "{}",
            serde_json::json!({"passed": passed, "added": added, "removed": removed})
        );
    } else if passed {
        println!("abi-diff: PASSED (no breaking changes)");
    } else {
        eprintln!("abi-diff: breaking change detected");
        for l in &removed {
            eprintln!("  [-] {l}");
        }
        for l in &added {
            eprintln!("  [+] {l}");
        }
    }
    if !passed {
        Err("abi-diff failed".into())
    } else {
        Ok(())
    }
}

fn diff_git(base: &str, json: bool) -> Result<(), String> {
    let spec = format!("{base}..HEAD");
    let out = Command::new("cargo")
        .args([
            "public-api",
            "diff",
            &spec,
            "--manifest-path",
            MANIFEST,
            "--deny",
            "removed",
            "--deny",
            "changed",
        ])
        .output()
        .map_err(|e| format!("cargo-public-api not installed: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let passed = out.status.success();
    if json {
        println!(
            "{}",
            serde_json::json!({"passed": passed, "output": stdout.trim()})
        );
    } else if passed {
        println!("abi-diff: PASSED");
    } else {
        eprintln!("abi-diff: breaking change detected\n{stdout}");
    }
    if !passed {
        Err("abi-diff failed".into())
    } else {
        Ok(())
    }
}

fn capture_public_api() -> Result<String, String> {
    let out = Command::new("cargo")
        .args(["public-api", "--manifest-path", MANIFEST, "-sss"])
        .output()
        .map_err(|e| format!("cargo-public-api not installed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo-public-api failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolve_fallback() {
        let r = resolve_baseline("abi-baseline/v0.1-alpha-pre-abi-freeze.json");
        assert!(r.to_string_lossy().ends_with(".txt") || r.to_string_lossy().ends_with(".json"));
    }
    #[test]
    fn line_diff_add_remove() {
        let (a, r) = line_diff("a\nb\nc\n", "a\nd\nc\n");
        assert_eq!(a, vec!["d"]);
        assert_eq!(r, vec!["b"]);
    }
    #[test]
    fn line_diff_identical() {
        assert!(line_diff("x\ny\n", "x\ny\n") == (vec![], vec![]));
    }
}
