#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;

/// Parse `development_status:` from sprint-status.yaml into key → status.
///
/// ⚠ Strips the trailing `# …` comment before matching. Entries in this repo
/// carry long provenance comments after the value (`done  # dev_model_used:
/// …; SEALED 2026-…`). A parser that keeps the comment yields a status like
/// `done  # …` that equals no `TERMINAL_STATUS`, so the story is silently
/// skipped — 58 of 141 `done` stories (every one with a provenance comment,
/// all of Epic 9–13) escaped this gate that way until this fix. Mirrors the
/// same repair made to `check_dev_model_tier::load_sprint_status` in 14adad35.
/// This is the single-sourced sprint-status parser for xtask gates.
pub fn load_sprint_status(path: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };
    let mut in_status_section = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("development_status:") {
            in_status_section = true;
            continue;
        }
        if in_status_section {
            if !line.starts_with(' ') && !line.starts_with('\t') && !line.is_empty() {
                if !trimmed.starts_with('#') {
                    in_status_section = false;
                    continue;
                }
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().to_string();
                let value = v
                    .split('#')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches(|c| c == '\'' || c == '"')
                    .to_string();
                if !key.is_empty() && !value.is_empty() {
                    map.insert(key, value);
                }
            }
        }
    }
    map
}
