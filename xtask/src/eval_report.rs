//! Story 28-1 — `xtask eval-report`: the offline half of the Spirit
//! evaluation protocol (`docs/research/spirit-evaluation-protocol.md`).
//!
//! Three modes, one JSON document on stdout (and `--out`):
//! - aggregate `--results <class>.results.jsonl`: k `maos eval halt|distill`
//!   lines → `maos.eval.live/v1` (mean/min/max over the repeats, Wilson 95 %
//!   over the per-row majority decision, confusion by family, flip rate,
//!   tokens and cost from the price book);
//! - compare `--reference a.json --candidate b.json` → `maos.eval.compare/v1`;
//! - calibrate `--calibrate --spec <min> --metric <m> --result a.json`.
//!
//! The report is printed before the verdict: exit 1 names every missed floor,
//! an overspent comparison tolerance or a measurement below its spec minimum.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use maos_eval::distill_replay::{
    FAITHFULNESS_FLOOR, HEDGE_PRESERVATION_FLOOR, RECALL_FLOOR, SECRET_LEAKAGE_CEILING,
    TRACEABILITY_FLOOR,
};
use maos_eval::halt_replay::round_down_hundredth;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const LIVE_SCHEMA: &str = "maos.eval.live/v1";
const COMPARE_SCHEMA: &str = "maos.eval.compare/v1";
const CALIBRATION_SCHEMA: &str = "maos.eval.calibration/v1";
const PRICING: &str = "xtask/provider-pricing.toml";
const WILSON_Z: f64 = 1.96;
const EPS: f64 = 1e-9;
/// The NFR-Aud-7 metrics of `maos eval distill`: name, bound, is a ceiling.
const DISTILL: [(&str, f64, bool); 5] = [
    ("recall", RECALL_FLOOR, false),
    ("faithfulness", FAITHFULNESS_FLOOR, false),
    ("hedge_preservation", HEDGE_PRESERVATION_FLOOR, false),
    ("traceability", TRACEABILITY_FLOOR, false),
    ("secret_leakage", SECRET_LEAKAGE_CEILING, true),
];

#[derive(clap::Args, Debug)]
pub struct Args {
    /// k result lines, one `maos eval halt|distill` stdout line each.
    #[arg(long)]
    results: Option<PathBuf>,
    /// Workspace root: the corpus, the cassettes and the price book resolve against it.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// Directory holding the cassettes by file name (default: each line's `cassette` under `--root`).
    #[arg(long)]
    cassettes: Option<PathBuf>,
    /// Price book (default `<root>/xtask/provider-pricing.toml`).
    #[arg(long)]
    pricing: Option<PathBuf>,
    /// The repeats the run was asked for; fewer result lines is an error.
    #[arg(long)]
    repeats: Option<usize>,
    /// The workflow run id recorded in `runs[]`.
    #[arg(long)]
    run_id: Option<String>,
    /// Also write the report to this file.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Compare: the reference `maos.eval.live/v1` file.
    #[arg(long, requires = "candidate")]
    reference: Option<PathBuf>,
    /// Compare: the candidate `maos.eval.live/v1` file.
    #[arg(long, requires = "reference")]
    candidate: Option<PathBuf>,
    /// Calibrate a floor from the `maos.eval.live/v1` file `--result` (D-28-1-03 A).
    #[arg(long, requires_all = ["spec", "metric", "result"])]
    calibrate: bool,
    /// Calibrate: the spec minimum of the metric.
    #[arg(long)]
    spec: Option<f64>,
    /// Calibrate: the metric name.
    #[arg(long)]
    metric: Option<String>,
    /// Calibrate: the `maos.eval.live/v1` file.
    #[arg(long)]
    result: Option<PathBuf>,
}

pub fn run(args: Args) -> Result<(), String> {
    let (report, failures) = if args.calibrate {
        calibrate(&args)?
    } else if let (Some(reference), Some(candidate)) = (&args.reference, &args.candidate) {
        compare(reference, candidate)?
    } else {
        aggregate(&args)?
    };
    let text = serde_json::to_string_pretty(&report).map_err(|e| format!("eval-report: {e}"))?;
    if let Some(out) = &args.out {
        std::fs::write(out, format!("{text}\n"))
            .map_err(|e| format!("eval-report: {}: {e}", out.display()))?;
    }
    println!("{text}");
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("eval-report: {}", failures.join("; ")))
    }
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("eval-report: {}: {e}", path.display()))
}

fn json_lines(bytes: &[u8], what: &str) -> Result<Vec<Value>, String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(|e| format!("eval-report: {what}: {e}")))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn field<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    v.get(key)
        .ok_or_else(|| format!("eval-report: missing `{key}`"))
}

fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    field(v, key)?
        .as_str()
        .ok_or_else(|| format!("eval-report: `{key}` is not a string"))
}

fn num(v: &Value, key: &str) -> Result<f64, String> {
    field(v, key)?
        .as_f64()
        .ok_or_else(|| format!("eval-report: `{key}` is not a number"))
}

fn object<'a>(v: &'a Value, key: &str) -> Result<&'a Map<String, Value>, String> {
    field(v, key)?
        .as_object()
        .ok_or_else(|| format!("eval-report: `{key}` is not an object"))
}

/// Wilson 95 % score interval of `successes` of `n`; `null` for an empty sample.
fn wilson(successes: usize, n: usize) -> Value {
    if n == 0 {
        return Value::Null;
    }
    let (p, n) = (successes as f64 / n as f64, n as f64);
    let d = 1.0 + WILSON_Z * WILSON_Z / n;
    let centre = (p + WILSON_Z * WILSON_Z / (2.0 * n)) / d;
    let half = WILSON_Z * (p * (1.0 - p) / n + WILSON_Z * WILSON_Z / (4.0 * n * n)).sqrt() / d;
    json!([(centre - half).max(0.0), (centre + half).min(1.0)])
}

/// One metric over the k repeats: the mean decides (D-28-1-01 B); `n` is its
/// denominator (the compare tolerance is 1 / `n`); the Wilson interval is over
/// the per-row majority decision, `majority` successes of `n`, reported, not
/// gated. A distill metric has no per-row decisions: `majority` is `None`.
fn metric(values: &[f64], bound: f64, ceiling: bool, n: usize, majority: Option<usize>) -> Value {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pass = if ceiling {
        mean <= bound + EPS
    } else {
        mean + EPS >= bound
    };
    json!({
        "mean": mean, "min": min, "max": max,
        "majority": majority.map(|s| if n == 0 { 1.0 } else { s as f64 / n as f64 }),
        "n": n,
        "wilson95": majority.map_or(Value::Null, |s| wilson(s, n)),
        "floor": bound,
        "direction": if ceiling { "ceiling" } else { "floor" },
        "pass": pass,
    })
}

/// A corpus row: its family (D-28-1-09 A), its label and calendar-conflict flag.
struct Row {
    id: String,
    family: String,
    expected: bool,
    conflict: bool,
}

/// D-28-1-09 A: a halt corpus' family is the scenario id's leading letters;
/// the distill corpus' is the row's `control`, `clean` when null.
fn rows(kind: &str, corpus: &[Value]) -> Result<Vec<Row>, String> {
    let flag = |r: &Value, key: &str| r.get(key).and_then(Value::as_bool).unwrap_or(false);
    corpus
        .iter()
        .map(|r| -> Result<Row, String> {
            let id = text(r, "scenario_id")?;
            let family: String = if kind == "halt" {
                id.chars().take_while(char::is_ascii_alphabetic).collect()
            } else {
                r.get("control")
                    .and_then(Value::as_str)
                    .unwrap_or("clean")
                    .to_owned()
            };
            Ok(Row {
                id: id.to_owned(),
                family,
                expected: flag(r, "expected_halt"),
                conflict: flag(r, "calendar_conflict"),
            })
        })
        .collect()
}

#[derive(Default, Clone, Copy)]
struct Counts {
    tp: usize,
    fp: usize,
    fn_: usize,
    cc_tp: usize,
    cc_expected: usize,
}

fn count(rows: &[Row], halted: &BTreeMap<String, bool>) -> Counts {
    let mut c = Counts::default();
    for r in rows {
        match (r.expected, halted[&r.id]) {
            (true, true) => c.tp += 1,
            (false, true) => c.fp += 1,
            (true, false) => c.fn_ += 1,
            (false, false) => {}
        }
        if r.conflict && r.expected {
            c.cc_expected += 1;
            c.cc_tp += usize::from(halted[&r.id]);
        }
    }
    c
}

/// The four halt measures as `(name, successes, n)`; `errors` rows of `rows`.
fn measures(c: Counts, errors: usize, rows: usize) -> [(&'static str, usize, usize); 4] {
    [
        ("halt_recall_calendar_conflict", c.cc_tp, c.cc_expected),
        ("halt_recall_overall", c.tp, c.tp + c.fn_),
        ("halt_precision_overall", c.tp, c.tp + c.fp),
        ("format_error_rate", errors, rows),
    ]
}

fn ratio((_, s, n): (&str, usize, usize)) -> f64 {
    if n == 0 {
        1.0
    } else {
        s as f64 / n as f64
    }
}

/// A flag per corpus row: the line's `verdicts`, or whether it names the row
/// in `format_errors` (`key`).
fn per_row(line: &Value, rows: &[Row], key: &str) -> Result<BTreeMap<String, bool>, String> {
    let ids = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|i| i.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let flags: BTreeMap<String, bool> = if key == "verdicts" {
        let mut out = BTreeMap::new();
        for (id, v) in object(line, key)? {
            let halted = v
                .as_bool()
                .ok_or_else(|| format!("eval-report: verdict of {id} is not a boolean"))?;
            out.insert(id.clone(), halted);
        }
        out
    } else {
        let named = ids(field(line, key)?);
        rows.iter()
            .map(|r| (r.id.clone(), named.contains(&r.id)))
            .collect()
    };
    if flags.len() != rows.len() || rows.iter().any(|r| !flags.contains_key(&r.id)) {
        return Err(format!("eval-report: `{key}` is not the corpus' rows"));
    }
    if key != "verdicts" && ids(field(line, key)?).len() != flags.values().filter(|f| **f).count() {
        return Err("eval-report: `format_errors` names a row the corpus lacks".into());
    }
    Ok(flags)
}

/// More than half of the repeats said true (a tie is false).
fn majority(runs: &[BTreeMap<String, bool>]) -> BTreeMap<String, bool> {
    runs[0]
        .keys()
        .map(|id| {
            let yes = runs.iter().filter(|r| r[id]).count();
            (id.clone(), yes * 2 > runs.len())
        })
        .collect()
}

/// What the halt or distill leg contributes to the report.
struct Body {
    metrics: Map<String, Value>,
    confusion: Value,
    majority: Value,
    flip_rows: Vec<String>,
    flip_rate: Value,
    format_errors: (usize, Vec<String>),
    failures: Vec<String>,
}

fn halt_body(lines: &[Value], rows: &[Row]) -> Result<Body, String> {
    let floors = object(&lines[0], "floors")?;
    let mut verdicts = Vec::new();
    let mut errors = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let (v, e) = (
            per_row(line, rows, "verdicts")?,
            per_row(line, rows, "format_errors")?,
        );
        let c = count(rows, &v);
        let score = field(line, "score")?;
        for (key, want) in [
            ("true_positives", c.tp),
            ("false_positives", c.fp),
            ("false_negatives", c.fn_),
        ] {
            if score.get(key).and_then(Value::as_u64) != Some(want as u64) {
                return Err(format!(
                    "eval-report: run {}: score.{key} disagrees with its verdicts",
                    i + 1
                ));
            }
        }
        verdicts.push(v);
        errors.push(e);
    }
    let (decided, errored) = (majority(&verdicts), majority(&errors));
    let flip_rows: Vec<String> = rows
        .iter()
        .filter(|r| verdicts.iter().any(|v| v[&r.id] != verdicts[0][&r.id]))
        .map(|r| r.id.clone())
        .collect();
    let per_run: Vec<_> = verdicts
        .iter()
        .zip(&errors)
        .map(|(v, e)| {
            measures(
                count(rows, v),
                e.values().filter(|f| **f).count(),
                rows.len(),
            )
        })
        .collect();
    let at_majority = measures(
        count(rows, &decided),
        errored.values().filter(|f| **f).count(),
        rows.len(),
    );

    let mut metrics = Map::new();
    let mut failures = Vec::new();
    for (i, &(name, s, n)) in at_majority.iter().enumerate() {
        let ceiling = name == "format_error_rate";
        let bound = if ceiling {
            Some(0.0)
        } else {
            floors.get(name).and_then(Value::as_f64)
        };
        let Some(bound) = bound else { continue };
        let values: Vec<f64> = per_run.iter().map(|m| ratio(m[i])).collect();
        let m = metric(&values, bound, ceiling, n, Some(s));
        if m["pass"] == false {
            let side = if ceiling {
                "above its ceiling"
            } else {
                "below its floor"
            };
            failures.push(format!(
                "{name} mean {:.3} {side} {bound}",
                m["mean"].as_f64().unwrap_or(0.0)
            ));
        }
        metrics.insert(name.to_owned(), m);
    }

    let mut confusion: BTreeMap<&str, [usize; 4]> = BTreeMap::new();
    for r in rows {
        let slot = match (r.expected, decided[&r.id]) {
            (true, true) => 0,
            (false, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        };
        confusion.entry(&r.family).or_default()[slot] += 1;
    }
    let confusion = confusion
        .into_iter()
        .map(|(f, [tp, fp, tn, fn_])| {
            (
                f.to_owned(),
                json!({"tp": tp, "fp": fp, "tn": tn, "fn": fn_}),
            )
        })
        .collect::<Map<_, _>>();
    let error_rows: BTreeSet<&String> = errors
        .iter()
        .flat_map(|e| e.iter().filter(|(_, f)| **f).map(|(id, _)| id))
        .collect();
    Ok(Body {
        metrics,
        confusion: Value::Object(confusion),
        majority: json!(decided),
        flip_rate: json!(flip_rows.len() as f64 / rows.len() as f64),
        flip_rows,
        format_errors: (
            errors
                .iter()
                .map(|e| e.values().filter(|f| **f).count())
                .sum(),
            error_rows.into_iter().cloned().collect(),
        ),
        failures,
    })
}

/// `"{metric} {value:.3} misses its floor {bound}"` (metric from [`DISTILL`], both
/// numbers parsing) or `"negative control … not detected"`, anchored at both ends.
fn is_gate_failure(f: &str) -> bool {
    let number = |s: &str| s.parse::<f64>().is_ok();
    DISTILL.iter().any(|(name, ..)| {
        f.strip_prefix(*name)
            .and_then(|r| r.strip_prefix(' '))
            .and_then(|r| r.split_once(" misses its floor "))
            .is_some_and(|(value, bound)| number(value) && number(bound))
    }) || (f.starts_with("negative control ") && f.ends_with(" not detected"))
}

/// `maos eval distill` lines carry the five metric values, the control
/// detections and the run's failures, no per-row decisions: mean/min/max
/// against the NFR-Aud-7 constants, no interval, no confusion matrix. The
/// denominators come from the corpus: the clean rows' claims (recall; the
/// findings are one per claim, so also faithfulness, traceability and secret
/// leakage) and their hedges (hedge preservation). A numeric miss may average
/// out over the repeats; a structural failure of any repeat (a stale judge
/// prompt, a changed frozen survey, a writer/cite disagreement) or an
/// undetected control never does.
fn distill_body(lines: &[Value], corpus: &[Value]) -> Result<Body, String> {
    let claims: Vec<&Value> = corpus
        .iter()
        .filter(|r| r["control"].is_null())
        .flat_map(|r| r["claims"].as_array().into_iter().flatten())
        .collect();
    let hedges: usize = claims
        .iter()
        .map(|c| c["hedges"].as_array().map_or(0, Vec::len))
        .sum();
    let mut metrics = Map::new();
    let mut failures = Vec::new();
    for (name, bound, ceiling) in DISTILL {
        let values = lines
            .iter()
            .map(|l| num(field(field(l, "score")?, "metrics")?, name))
            .collect::<Result<Vec<_>, String>>()?;
        let n = if name == "hedge_preservation" {
            hedges
        } else {
            claims.len()
        };
        let m = metric(&values, bound, ceiling, n, None);
        if m["pass"] == false {
            failures.push(format!(
                "{name} mean {:.3} misses its bound {bound}",
                m["mean"].as_f64().unwrap_or(0.0)
            ));
        }
        metrics.insert(name.to_owned(), m);
    }
    let mut undetected = BTreeSet::new();
    for l in lines {
        for c in field(field(l, "score")?, "controls")?
            .as_array()
            .into_iter()
            .flatten()
        {
            if c["detected"] != true {
                undetected.insert(text(c, "scenario_id")?.to_owned());
            }
        }
    }
    // Every other line failure is structural. Only the COMPLETE shapes of
    // `distill_replay::missed_floors` / `gate_failures` are numeric: a survey
    // message embeds model text and must never match by substring.
    let structural: BTreeSet<&str> = lines
        .iter()
        .flat_map(|l| l["missed_floors"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .filter(|f| !is_gate_failure(f))
        .collect();
    failures.extend(structural.into_iter().map(str::to_owned));
    if !undetected.is_empty() {
        failures.push(format!(
            "controls not detected: {}",
            undetected.iter().cloned().collect::<Vec<_>>().join(",")
        ));
    }
    Ok(Body {
        metrics,
        confusion: json!({}),
        majority: Value::Null,
        flip_rows: Vec::new(),
        flip_rate: Value::Null,
        format_errors: (0, Vec::new()),
        failures,
    })
}

/// The cassette of `line`, refused unless its bytes are the line's `cassette_sha256`.
fn cassette(line: &Value, args: &Args) -> Result<Value, String> {
    let rel = text(line, "cassette")?;
    let path = match &args.cassettes {
        Some(dir) => dir.join(
            Path::new(rel)
                .file_name()
                .ok_or("eval-report: cassette has no file name")?,
        ),
        None => args.root.join(rel),
    };
    let bytes = read(&path)?;
    if hex(&bytes) != text(line, "cassette_sha256")? {
        return Err(format!(
            "eval-report: {} is not the cassette its result line names (cassette_sha256)",
            path.display()
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("eval-report: {}: {e}", path.display()))
}

/// Input/output tokens per (provider, model) over every cassette entry, priced
/// from the book; a model the book lacks is an error, never 0 (AC8).
/// Returns `(provider, model_id, input, output, cost_usd)`.
fn spend(cassettes: &[Value], book: &Path) -> Result<(String, String, u64, u64, f64), String> {
    let mut by_model: BTreeMap<(String, String), (u64, u64)> = BTreeMap::new();
    for c in cassettes {
        for entry in field(c, "entries")?.as_array().into_iter().flatten() {
            let response = field(entry, "response")?;
            let who = field(response, "provider_attribution")?;
            let usage = field(response, "usage")?;
            let slot = by_model
                .entry((
                    text(who, "provider_id")?.to_owned(),
                    text(who, "model_id")?.to_owned(),
                ))
                .or_default();
            slot.0 += num(usage, "input_tokens")? as u64;
            slot.1 += num(usage, "output_tokens")? as u64;
        }
    }
    if by_model.is_empty() {
        return Err("eval-report: the cassettes hold no completions".into());
    }
    let book_text = String::from_utf8(read(book)?).map_err(|e| format!("eval-report: {e}"))?;
    let book_toml: toml::Value =
        toml::from_str(&book_text).map_err(|e| format!("eval-report: {}: {e}", book.display()))?;
    let entries = book_toml
        .get("entries")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| format!("eval-report: {} has no entries", book.display()))?;
    let mut micro = 0.0;
    for ((provider, model), (input, output)) in &by_model {
        let row = entries
            .iter()
            .find(|e| {
                e.get("provider").and_then(toml::Value::as_str) == Some(provider.as_str())
                    && e.get("model").and_then(toml::Value::as_str) == Some(model.as_str())
            })
            .ok_or_else(|| {
                format!(
                    "eval-report: {provider}/{model} is not in the price book {} — an unpriced model is an error, not 0",
                    book.display()
                )
            })?;
        let price = |key: &str| {
            row.get(key)
                .and_then(toml::Value::as_integer)
                .ok_or_else(|| format!("eval-report: price row {provider}/{model} has no `{key}`"))
        };
        micro += (*input as f64 * price("input_price_micro_per_1k")? as f64
            + *output as f64 * price("output_price_micro_per_1k")? as f64)
            / 1000.0;
    }
    let names = |side: usize| {
        let set: BTreeSet<&String> = by_model
            .keys()
            .map(|k| if side == 0 { &k.0 } else { &k.1 })
            .collect();
        set.into_iter().cloned().collect::<Vec<_>>().join("+")
    };
    Ok((
        names(0),
        names(1),
        by_model.values().map(|t| t.0).sum(),
        by_model.values().map(|t| t.1).sum(),
        micro / 1e6,
    ))
}

fn aggregate(args: &Args) -> Result<(Value, Vec<String>), String> {
    let results = args.results.as_deref().ok_or(
        "eval-report: one of --results <file.jsonl>, --reference/--candidate or --calibrate is required",
    )?;
    let lines = json_lines(&read(results)?, "result line")?;
    let first = lines
        .first()
        .ok_or("eval-report: the results file has no lines")?;
    if let Some(k) = args.repeats {
        if lines.len() != k {
            return Err(format!(
                "eval-report: {} result lines for {k} repeats — a run produced no numbers",
                lines.len()
            ));
        }
    }
    let kind = match text(first, "event")? {
        "eval_halt" => "halt",
        "eval_distill" => "distill",
        other => return Err(format!("eval-report: unknown event `{other}`")),
    };
    for key in ["event", "class", "corpus", "corpus_sha256", "floors"] {
        if lines.iter().any(|l| l.get(key) != first.get(key)) {
            return Err(format!("eval-report: the result lines disagree on `{key}`"));
        }
    }
    let corpus_bytes = read(&args.root.join(text(first, "corpus")?))?;
    if hex(&corpus_bytes) != text(first, "corpus_sha256")? {
        return Err(
            "eval-report: the corpus is not the one the result lines scored (corpus_sha256)".into(),
        );
    }
    let corpus_json = json_lines(&corpus_bytes, "corpus row")?;
    let corpus_rows = rows(kind, &corpus_json)?;
    if corpus_rows.is_empty() {
        return Err("eval-report: the corpus has no rows".into());
    }
    let mut corpus_families: BTreeMap<&str, usize> = BTreeMap::new();
    for r in &corpus_rows {
        *corpus_families.entry(&r.family).or_default() += 1;
    }

    let cassettes = lines
        .iter()
        .map(|l| cassette(l, args))
        .collect::<Result<Vec<_>, _>>()?;
    let prompts = |c: &Value| -> Result<Vec<String>, String> {
        field(c, "entries")?
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| text(e, "prompt_sha256").map(str::to_owned))
            .collect()
    };
    let prompt_hashes = prompts(&cassettes[0])?;
    if kind == "halt" {
        for c in &cassettes[1..] {
            if prompts(c)? != prompt_hashes {
                return Err(
                    "eval-report: the repeats were recorded under different prompts".into(),
                );
            }
        }
    }
    let book = args
        .pricing
        .clone()
        .unwrap_or_else(|| args.root.join(PRICING));
    let (provider, model_id, input, output, cost) = spend(&cassettes, &book)?;
    let book_hash = hex(&read(&book)?);

    let body = if kind == "halt" {
        halt_body(&lines, &corpus_rows)?
    } else {
        distill_body(&lines, &corpus_json)?
    };
    let runs: Vec<Value> = lines
        .iter()
        .enumerate()
        .map(|(i, l)| {
            json!({
                "run": i + 1, "cassette": l["cassette"], "cassette_sha256": l["cassette_sha256"],
                "provenance": l["provenance"], "workflow_run_id": args.run_id,
            })
        })
        .collect();
    let report = json!({
        "schema": LIVE_SCHEMA,
        "class": first["class"], "eval": kind,
        "provider": provider, "model_id": model_id,
        "prompt_version": first.get("spirit_version"),
        "prompt_sha256": hex(prompt_hashes.join("\n").as_bytes()),
        "corpus": first["corpus"], "corpus_sha256": first["corpus_sha256"],
        "corpus_families": corpus_families,
        "k": lines.len(), "runs": runs,
        "metrics": body.metrics,
        "confusion_by_family": body.confusion,
        "majority_verdicts": body.majority,
        "flip_rows": body.flip_rows, "flip_rate": body.flip_rate,
        "format_errors": {"count": body.format_errors.0, "rows": body.format_errors.1},
        "tokens": {"input": input, "output": output},
        "cost_usd": cost,
        "price_book": {"path": book.display().to_string(), "sha256": book_hash},
        "failures": body.failures,
        "pass": body.failures.is_empty(),
    });
    Ok((report, body.failures))
}

fn live(path: &Path) -> Result<Value, String> {
    let v: Value = serde_json::from_slice(&read(path)?)
        .map_err(|e| format!("eval-report: {}: {e}", path.display()))?;
    if text(&v, "schema")? != LIVE_SCHEMA {
        return Err(format!(
            "eval-report: {} is not a {LIVE_SCHEMA} file",
            path.display()
        ));
    }
    Ok(v)
}

/// D-28-1-06 B: the candidate passes when every floor holds (a malformed
/// completion is the `format_error_rate` ceiling) and no metric is worse than
/// the reference by more than one decision, 1 / the reference's `n`.
fn compare(reference: &Path, candidate: &Path) -> Result<(Value, Vec<String>), String> {
    let (r, c) = (live(reference)?, live(candidate)?);
    for key in ["class", "eval", "corpus_sha256"] {
        if r.get(key) != c.get(key) {
            return Err(format!(
                "eval-report: the reference and the candidate differ in `{key}`"
            ));
        }
    }
    let reference_metrics = object(&r, "metrics")?;
    let mut metrics = Map::new();
    let mut reasons = Vec::new();
    for (name, cand) in object(&c, "metrics")? {
        let refm = reference_metrics
            .get(name)
            .ok_or_else(|| format!("eval-report: the reference has no metric `{name}`"))?;
        let (rv, cv) = (num(refm, "mean")?, num(cand, "mean")?);
        let ceiling = cand["direction"] == "ceiling";
        let worse = if ceiling { cv - rv } else { rv - cv };
        let tolerance = refm["n"].as_u64().map_or(0.0, |n| 1.0 / n.max(1) as f64);
        if worse > tolerance + EPS {
            reasons.push(format!(
                "{name} is worse than the reference by {worse:.3}, more than one decision ({tolerance:.3})"
            ));
        }
        metrics.insert(
            name.clone(),
            json!({"reference": rv, "candidate": cv, "delta": cv - rv, "tolerance": tolerance, "bound": cand["floor"]}),
        );
    }
    reasons.extend(
        c["failures"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|f| format!("the candidate fails: {f}")),
    );
    let decisions = |v: &Value| {
        v["majority_verdicts"]
            .as_object()
            .cloned()
            .unwrap_or_default()
    };
    let (before, after) = (decisions(&r), decisions(&c));
    let changed: Vec<&String> = after
        .iter()
        .filter(|(id, d)| before.get(*id) != Some(*d))
        .map(|(id, _)| id)
        .collect();
    let side = |v: &Value| json!({"model_id": v["model_id"], "k": v["k"], "prompt_sha256": v["prompt_sha256"]});
    let report = json!({
        "schema": COMPARE_SCHEMA, "class": c["class"], "eval": c["eval"],
        "reference": side(&r), "candidate": side(&c),
        "metrics": metrics, "changed_rows": changed,
        "verdict": if reasons.is_empty() { "pass" } else { "fail" },
        "reasons": reasons,
    });
    Ok((report, reasons))
}

/// D-28-1-03 A: floor = max(spec minimum, Wilson lower bound of the measured
/// baseline rounded down to 0.01). A mean below the spec minimum never lowers
/// the floor: it fails and asks for an operator spec-change D-row.
fn calibrate(args: &Args) -> Result<(Value, Vec<String>), String> {
    let (Some(spec), Some(name), Some(path)) = (args.spec, &args.metric, &args.result) else {
        return Err("eval-report: --calibrate needs --spec, --metric and --result".into());
    };
    let live = live(path)?;
    let m = object(&live, "metrics")?
        .get(name)
        .ok_or_else(|| format!("eval-report: the result has no metric `{name}`"))?;
    if m["direction"] != "floor" {
        return Err(format!(
            "eval-report: `{name}` is a ceiling; only floors are calibrated"
        ));
    }
    let lower = m["wilson95"][0]
        .as_f64()
        .ok_or_else(|| format!("eval-report: `{name}` has no Wilson interval"))?;
    let mean = num(m, "mean")?;
    let floor = spec.max(round_down_hundredth(lower));
    let below = mean + EPS < spec;
    let failures = if below {
        vec![format!(
            "{name} mean {mean:.3} is below the spec minimum {spec}: open an operator spec-change D-row; the floor stays {spec}"
        )]
    } else {
        Vec::new()
    };
    let report = json!({
        "schema": CALIBRATION_SCHEMA,
        "metric": name, "class": live["class"], "k": live["k"],
        "model_id": live["model_id"], "prompt_sha256": live["prompt_sha256"],
        "spec_minimum": spec, "measured_mean": mean, "wilson95": m["wilson95"],
        "floor": floor,
        "rule": "D-28-1-03 A: max(spec minimum, Wilson lower bound of the measured baseline rounded down to 0.01)",
        "spec_change_required": below,
    });
    Ok((report, failures))
}
