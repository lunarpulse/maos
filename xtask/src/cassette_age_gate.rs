use std::path::Path;

const TIER2_STAMP_FILE: &str = ".tier2-first-success";
const MAX_AGE_DAYS: u64 = 14;

pub fn run(cassette_dir: &str, json: bool, stamp_dir: Option<&str>) -> Result<(), String> {
    let dir = Path::new(cassette_dir);
    if !dir.exists() {
        return Err(format!("cassette directory not found: {cassette_dir}"));
    }

    let mut files = Vec::new();
    collect_json_files(dir, &mut files);

    if files.is_empty() {
        if json {
            println!("{{\"cassettes\":0,\"stale\":0}}");
        } else {
            println!("cassette-age-gate: no cassettes found in {cassette_dir}");
        }
        return Ok(());
    }

    let mut stale = Vec::new();
    for path in &files {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

        let recorded_at = extract_recorded_at(&content);
        if let Some(date_str) = recorded_at {
            if is_stale(&date_str, MAX_AGE_DAYS) {
                stale.push((path.display().to_string(), date_str));
            }
        }
    }

    if json {
        println!(
            "{{\"cassettes\":{},\"stale\":{}}}",
            files.len(),
            stale.len()
        );
    }

    if stale.is_empty() {
        if !json {
            println!(
                "cassette-age-gate: {} cassettes checked, all within {MAX_AGE_DAYS}-day window",
                files.len()
            );
        }
        Ok(())
    } else {
        for (path, date) in &stale {
            eprintln!("  STALE: {path} (recorded_at: {date})");
        }
        let stamp_path = Path::new(stamp_dir.unwrap_or(cassette_dir)).join(TIER2_STAMP_FILE);

        if stamp_path.exists() {
            Err(format!(
                "cassette-age-gate: {}/{} cassettes exceed {MAX_AGE_DAYS}-day age limit \
                 (Tier-2 has previously succeeded — hard-fail)",
                stale.len(),
                files.len()
            ))
        } else {
            eprintln!(
                "cassette-age-gate: {}/{} cassettes exceed {MAX_AGE_DAYS}-day age limit \
                 (WARN — run Tier-2 nightly to refresh)",
                stale.len(),
                files.len()
            );
            Ok(())
        }
    }
}

fn collect_json_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

fn extract_recorded_at(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"recorded_at\"") {
            if let Some(val) = trimmed.split('"').nth(3) {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Convert a Gregorian calendar date to a monotonic day count using the civil-calendar algorithm
/// that correctly handles all leap years including century exceptions.
/// Formula (with Jan/Feb treated as months 13/14 of the prior year):
/// `365*y + y/4 - y/100 + y/400 + (153*m+8)/5 + day`
fn civil_date_to_days(year: i64, month: i64, day: i64) -> i64 {
    let (y, m) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    365 * y + y / 4 - y / 100 + y / 400 + (153 * m + 8) / 5 + day
}

fn is_stale(date_str: &str, max_age_days: u64) -> bool {
    let date_part = date_str.split('T').next().unwrap_or(date_str);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    let year: i64 = parts[0].parse().unwrap_or(0);
    let month: i64 = parts[1].parse().unwrap_or(0);
    let day: i64 = parts[2].parse().unwrap_or(0);

    let recorded = civil_date_to_days(year, month, day);

    let now_unix_days = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400) as i64;
    // Unix epoch (1970-01-01) in the same civil-day numbering
    let epoch = civil_date_to_days(1970, 1, 1);
    let now = epoch + now_unix_days;

    (now - recorded) > max_age_days as i64
}
