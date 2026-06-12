use crate::fs_walk;
use std::path::Path;

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

    let src_dir = Path::new(maos_bin_dir).join("src");
    let mut rs_files = Vec::new();
    fs_walk::collect_rs_files(&src_dir, &mut rs_files);

    for file_path in &rs_files {
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name == "env_contract.rs" {
            continue;
        }
        let contents = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (line_no, line) in contents.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let patterns = [
                ("env::var(\"MAOS_", "MAOS_"),
                ("env::var_os(\"MAOS_", "MAOS_"),
            ];
            for (pat, prefix) in &patterns {
                if let Some(pos) = line.find(pat) {
                    let after = &line[pos + pat.len()..];
                    if let Some(end) = after.find('"') {
                        let var_name = format!("{prefix}{}", &after[..end]);
                        if !registered.contains(&var_name.as_str()) {
                            violations.push(format!(
                                "{}:{}: env::var(\"{}\") not in env_contract.rs registry",
                                file_path.display(),
                                line_no + 1,
                                var_name
                            ));
                        }
                    }
                }
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
            "check-env-contract: PASS ({} MAOS_* vars registered, 0 violations)",
            registered.len()
        );
        Ok(())
    } else {
        for v in &violations {
            eprintln!("  VIOLATION: {v}");
        }
        Err(format!(
            "check-env-contract: FAIL — {} unregistered MAOS_* env::var reads",
            violations.len()
        ))
    }
}
