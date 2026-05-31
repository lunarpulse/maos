use std::collections::BTreeMap;
use std::path::Path;

pub fn run(report: &Path, threshold: f64, strict: bool, json: bool) -> i32 {
    if !report.exists() {
        eprintln!(
            "check-multi-provider-drift: report not found: {}",
            report.display()
        );
        return 1;
    }

    let content = match std::fs::read_to_string(report) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("check-multi-provider-drift: failed to read report: {e}");
            return 1;
        }
    };

    let rows: Vec<serde_json::Value> = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("check-multi-provider-drift: failed to parse report JSON: {e}");
            return 1;
        }
    };

    let mut by_fixture: BTreeMap<String, Vec<&serde_json::Value>> = BTreeMap::new();
    for row in &rows {
        let fixture_id = row["fixture_id"].as_str().unwrap_or("unknown").to_string();
        by_fixture.entry(fixture_id).or_default().push(row);
    }

    let mut outliers: Vec<serde_json::Value> = Vec::new();

    for (fixture_id, provider_rows) in &by_fixture {
        let provider_ids: Vec<&str> = provider_rows
            .iter()
            .map(|r| r["provider"].as_str().unwrap_or("unknown"))
            .collect();

        if provider_ids.len() < 2 {
            outliers.push(serde_json::json!({
                "fixture_id": fixture_id,
                "reason": "missing_provider",
                "detail": format!("only {} provider(s) found", provider_ids.len())
            }));
            continue;
        }

        for metric in &["response_text_len", "input_tokens", "output_tokens"] {
            let values: Vec<f64> = provider_rows
                .iter()
                .filter_map(|r| r[*metric].as_f64())
                .collect();
            if values.is_empty() {
                continue;
            }
            let sorted = {
                let mut v = values.clone();
                // Use total_cmp to handle NaN deterministically — provider
                // reports may carry NaN tokens/latency in pathological cases
                // and `partial_cmp(...).unwrap()` would panic. total_cmp
                // gives a total order where NaNs sort to one end.
                v.sort_by(|a, b| a.total_cmp(b));
                v
            };
            let median = if sorted.len() % 2 == 0 {
                (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
            } else {
                sorted[sorted.len() / 2]
            };

            if median == 0.0 {
                continue;
            }

            for row in provider_rows {
                let val = row[*metric].as_f64().unwrap_or(0.0);
                let delta_pct = ((val - median) / median).abs() * 100.0;
                if delta_pct >= threshold {
                    outliers.push(serde_json::json!({
                        "fixture_id": fixture_id,
                        "provider": row["provider"],
                        "metric": metric,
                        "value": val,
                        "median": median,
                        "delta_pct": delta_pct,
                        "threshold": threshold,
                    }));
                }
            }
        }

        let stop_reasons: Vec<String> = provider_rows
            .iter()
            .filter_map(|r| r["stop_reason"].as_str().map(String::from))
            .collect();
        let unique_reasons: Vec<&str> = {
            let mut seen: Vec<&str> = Vec::new();
            for s in &stop_reasons {
                if !seen.contains(&s.as_str()) {
                    seen.push(s.as_str());
                }
            }
            seen
        };
        if unique_reasons.len() > 1 {
            outliers.push(serde_json::json!({
                "fixture_id": fixture_id,
                "reason": "stop_reason_disagreement",
                "values": unique_reasons,
            }));
        }
    }

    if json {
        let output = serde_json::json!({
            "outliers": outliers,
            "total_rows": rows.len(),
            "fixtures_checked": by_fixture.len(),
            "threshold": threshold,
        });
        // Story 5.5b backfill review: propagate serde error rather than
        // panicking — discipline §1373 (no `.unwrap()` on serde paths).
        match serde_json::to_string_pretty(&output) {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("check-multi-provider-drift: failed to serialize JSON: {e}");
                return 1;
            }
        }
    } else if outliers.is_empty() {
        println!(
            "check-multi-provider-drift: no outliers detected ({} fixtures, threshold={}%)",
            by_fixture.len(),
            threshold
        );
    } else {
        println!(
            "check-multi-provider-drift: {} outlier(s) detected:",
            outliers.len()
        );
        for outlier in &outliers {
            match serde_json::to_string(outlier) {
                Ok(s) => println!("  - {s}"),
                Err(e) => eprintln!("  - <serialization error: {e}>"),
            }
            // GitHub Actions annotation for PR-visible drift markers
            // (story narrative line 511; original code skipped this).
            if let Some(fixture) = outlier.get("fixture_id").and_then(|v| v.as_str()) {
                println!("::notice file=tests/reports/multi-provider.json::Drift outlier in fixture {fixture}");
            }
        }
    }

    if strict && !outliers.is_empty() {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    include!("tests/check_multi_provider_drift_tests.rs");
}
