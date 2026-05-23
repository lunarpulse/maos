#![forbid(unsafe_code)]

//! Gate A5 — `check-composition-root-completeness`.
//!
//! Per Epic 4 retro §A5: parses `crates/maos-kernel-core/src/api.rs` for `pub use`
//! re-exports naming `*Adapter` symbols, then parses
//! `crates/maos-bin/src/main.rs` for `Arc::new(<...>::<Adapter>::new(...))`
//! construction sites.  Fails if any `api.rs`-re-exported `*Adapter` lacks a
//! matching construction in `main.rs`, OR if two `Arc::new(<same>::new(...))`
//! instances exist.
//!
//! v0.3-β implementation: regex-based.  A whitelist file
//! `xtask/composition-root-whitelist.toml` may exempt adapters that legitimately
//! have multiple instances (starts empty).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

fn pub_use_adapter_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    // Capture only the final type name (e.g. SpiritSchedulerAdapter), not the full path.
    RE.get_or_init(|| {
        regex::Regex::new(r"pub\s+use\s+(?:[\w:]+::)*(\w+Adapter)(?:\s+as\s+\w+)?\s*;").unwrap()
    })
}

fn arc_new_adapter_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    // Match Arc::new(...Adapter::new/open/default(...)) OR ...Adapter::new/open/default(...)
    RE.get_or_init(|| {
        regex::Regex::new(
            r"(?:Arc::new\s*\(\s*)?(?:[\w:]+::)?(\w+Adapter)::(?:new|open|default)\s*\(",
        )
        .unwrap()
    })
}

fn load_whitelist(path: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    let p = PathBuf::from(path);
    if p.exists() {
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(toml) = content.parse::<toml::Value>() {
                if let Some(arr) = toml.get("whitelist").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            set.insert(s.to_string());
                        }
                    }
                }
            }
        }
    }
    set
}

pub fn run(api_rs: &str, main_rs: &str, whitelist_path: &str, json: bool) -> Result<(), String> {
    let api_content =
        fs::read_to_string(api_rs).map_err(|e| format!("failed to read {api_rs}: {e}"))?;
    let main_content =
        fs::read_to_string(main_rs).map_err(|e| format!("failed to read {main_rs}: {e}"))?;
    let whitelist = load_whitelist(whitelist_path);

    // 1. Collect all *Adapter symbols re-exported from api.rs.
    let mut adapters: HashSet<String> = HashSet::new();
    for cap in pub_use_adapter_re().captures_iter(&api_content) {
        adapters.insert(cap[1].to_string());
    }

    // 2. Collect all Arc::new(...Adapter::new(...)) constructions in main.rs.
    let mut constructions: HashMap<String, Vec<usize>> = HashMap::new();
    for (line_no, line) in main_content.lines().enumerate() {
        for cap in arc_new_adapter_re().captures_iter(line) {
            let name = cap[1].to_string();
            constructions.entry(name).or_default().push(line_no + 1);
        }
    }

    // 3. Check for missing constructions.
    let mut violations: Vec<String> = Vec::new();
    for adapter in &adapters {
        if !constructions.contains_key(adapter) && !whitelist.contains(adapter) {
            violations.push(format!(
                "error: adapter {adapter} is re-exported from api.rs but NOT constructed in main.rs"
            ));
        }
    }

    // 4. Check for duplicate constructions (same adapter constructed twice).
    for (adapter, lines) in &constructions {
        if lines.len() > 1 && !whitelist.contains(adapter) {
            violations.push(format!(
                "error: adapter {adapter} is constructed {} times in main.rs (lines: {:?}) — possible duplicate shared-state instance",
                lines.len(),
                lines
            ));
        }
    }

    if json {
        let payload = serde_json::json!({
            "passed": violations.is_empty(),
            "violation_count": violations.len(),
            "violations": violations,
            "adapters_found": adapters.len(),
            "constructions_found": constructions.len(),
        });
        println!("{}", payload);
    } else if !violations.is_empty() {
        for v in &violations {
            eprintln!("{v}");
        }
        eprintln!(
            "check-composition-root-completeness: FAIL ({} violation(s))",
            violations.len()
        );
    } else {
        eprintln!(
            "check-composition-root-completeness: PASS ({} adapter(s), {} construction(s))",
            adapters.len(),
            constructions.len()
        );
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err("composition-root-completeness violations found".to_string())
    }
}
