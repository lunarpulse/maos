use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::io::BufRead;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub occurrences: usize,
    pub source_distinct: usize,
}

/// T0 methodology: count warning compiler-message events, retaining duplicate
/// lib/lib-test emissions; deduplicate by package, level, code, message and
/// primary source ranges (never by target, rendered text or secondary spans).
pub fn count(reader: impl BufRead) -> Result<Counts, String> {
    let mut occurrences = 0;
    let mut distinct = BTreeSet::new();
    let mut finished = false;
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        let event: Value = serde_json::from_str(&line)
            .map_err(|e| format!("clippy JSON line {}: {e}", index + 1))?;
        if event["reason"] == "build-finished" {
            if event["success"] != true {
                return Err("clippy build did not succeed".into());
            }
            finished = true;
        }
        if event["reason"] == "compiler-message"
            && (!event["message"].is_object() || !event["message"]["level"].is_string())
        {
            return Err("compiler-message lacks message object or level".into());
        }
        if event["reason"] != "compiler-message" || event["message"]["level"] != "warning" {
            continue;
        }
        let message = &event["message"];
        let spans = message["spans"].as_array().ok_or("warning lacks spans")?;
        let primary: Vec<_> = spans
            .iter()
            .filter(|s| s["is_primary"] == true)
            .map(|s| {
                serde_json::json!([
                    s["file_name"],
                    s["line_start"],
                    s["column_start"],
                    s["line_end"],
                    s["column_end"]
                ])
            })
            .collect();
        if !event["package_id"].is_string() || !message["message"].is_string() {
            return Err("warning lacks package or message".into());
        }
        distinct.insert(
            serde_json::json!([
                event["package_id"],
                message["level"],
                message["code"]["code"],
                message["message"],
                primary
            ])
            .to_string(),
        );
        occurrences += 1;
    }
    if !finished {
        return Err("clippy JSON lacks successful build-finished event".into());
    }
    Ok(Counts {
        occurrences,
        source_distinct: distinct.len(),
    })
}

pub fn compare(actual: &Counts, baseline: &Counts) -> Result<(), String> {
    if actual.occurrences > baseline.occurrences
        || actual.source_distinct > baseline.source_distinct
    {
        return Err(format!(
            "clippy warning debt grew: {}/{} occurrences, {}/{} source-distinct",
            actual.occurrences,
            baseline.occurrences,
            actual.source_distinct,
            baseline.source_distinct
        ));
    }
    Ok(())
}

pub fn run(input: &str, baseline: &str) -> Result<(), String> {
    let baseline: Counts =
        toml::from_str(&std::fs::read_to_string(baseline).map_err(|e| format!("{baseline}: {e}"))?)
            .map_err(|e| format!("{baseline}: {e}"))?;
    let file = std::fs::File::open(input).map_err(|e| format!("{input}: {e}"))?;
    let actual = count(std::io::BufReader::new(file))?;
    println!(
        "{}",
        serde_json::to_string(&actual).map_err(|e| e.to_string())?
    );
    compare(&actual, &baseline)
}
