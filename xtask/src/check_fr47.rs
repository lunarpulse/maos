use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::fs_walk;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Violation {
    pub crate_name: String,
    pub dependency: String,
    pub manifest_path: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FR47 violation: Spirit must obtain inference via kernel Inference Port — crate '{}' depends on '{}' (manifest: {})",
            self.crate_name, self.dependency, self.manifest_path
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub passed: bool,
    pub violations: Vec<Violation>,
}

#[derive(Debug, serde::Deserialize)]
struct Denylist {
    #[serde(rename = "vendor-sdk-denylist")]
    vendor_sdk_denylist: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Allowlist {
    #[serde(rename = "allowed")]
    allowed: Vec<AllowedEntry>,
}

#[derive(Debug, serde::Deserialize, Clone)]
struct AllowedEntry {
    crate_name: String,
    dependency: String,
}

pub fn run(
    path: Option<&str>,
    denylist_path: &str,
    allowlist_path: &str,
    json: bool,
) -> Result<(), String> {
    let report = check_fr47(
        path.map(Path::new),
        Path::new(denylist_path),
        Path::new(allowlist_path),
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.passed {
            println!("check-fr47: PASSED (0 violations)");
        } else {
            for v in &report.violations {
                eprintln!("{v}");
            }
        }
    }

    if !report.passed {
        return Err("check-fr47 failed".into());
    }
    Ok(())
}

pub fn check_fr47(
    path: Option<&Path>,
    denylist_path: &Path,
    allowlist_path: &Path,
) -> Result<Report, String> {
    let denylist: Denylist = load_toml(denylist_path)?;
    let allowlist: Allowlist = if allowlist_path.exists() {
        load_toml(allowlist_path)?
    } else {
        Allowlist {
            allowed: Vec::new(),
        }
    };

    let denyset: HashSet<String> = denylist.vendor_sdk_denylist.into_iter().collect();
    let allowset: HashSet<(String, String)> = allowlist
        .allowed
        .into_iter()
        .map(|a| (a.crate_name, a.dependency))
        .collect();

    let mut violations = Vec::new();

    let scan_dirs: Vec<std::path::PathBuf> = if let Some(scan_path) = path {
        vec![scan_path.to_path_buf()]
    } else {
        // Default: scan all workspace crates except xtask and target
        let mut dirs = Vec::new();
        let crates_dir = Path::new("crates");
        if crates_dir.exists() {
            for entry in fs::read_dir(crates_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str == "xtask" {
                    continue;
                }
                let cargo_toml = entry.path().join("Cargo.toml");
                if cargo_toml.exists() {
                    dirs.push(entry.path());
                }
            }
        }
        dirs
    };

    for dir in &scan_dirs {
        let manifest_path = dir.join("Cargo.toml");
        if !manifest_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
        let doc: toml::Value = content
            .parse()
            .map_err(|e| format!("toml parse error in {}: {e}", manifest_path.display()))?;
        let crate_name = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(deps) = doc.get(section).and_then(|v| v.as_table()) {
                for (local_name, value) in deps {
                    let resolved_name = value
                        .as_table()
                        .and_then(|t| t.get("package"))
                        .and_then(|p| p.as_str())
                        .unwrap_or(local_name)
                        .to_string();
                    if denyset.contains(&resolved_name)
                        && !allowset.contains(&(crate_name.clone(), resolved_name.clone()))
                    {
                        violations.push(Violation {
                            crate_name: crate_name.clone(),
                            dependency: resolved_name,
                            manifest_path: manifest_path.display().to_string(),
                        });
                    }
                }
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

#[cfg(test)]
mod tests {
    include!("tests/check_fr47_tests.rs");
}
