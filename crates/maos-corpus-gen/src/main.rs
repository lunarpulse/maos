//! `maos-corpus-gen` CLI — corpus generation and coverage reporting.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "maos-corpus-gen",
    about = "MAOS parameterized corpus generators"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a corpus and write it to a JSONL file.
    Generate {
        /// Corpus name: "secret-redaction-1e4" or "red-team-640".
        #[arg(long)]
        corpus: String,

        /// Generation mode: per-commit, quarterly, or canary.
        #[arg(long, default_value = "per-commit")]
        mode: String,

        /// Output path for the JSONL file.
        #[arg(long)]
        out: String,

        /// RNG seed for canary mode (required when --mode canary).
        #[arg(long)]
        rng_seed: Option<u64>,

        /// Marker namespace for canary mode (required when --mode canary).
        #[arg(long)]
        marker_namespace: Option<String>,
    },

    /// Print a coverage report (text or JSON) for a corpus.
    Coverage {
        /// Corpus name: "secret-redaction-1e4" or "red-team-640".
        #[arg(long)]
        corpus: String,

        /// Emit JSON to stdout instead of a human-readable table.
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Optional path to a fixture seed TOML (overrides bundled seeds).
        #[arg(long)]
        seeds_fixture: Option<String>,
    },

    /// Story 7.4 — extend the LCAS corpus 70 → 210 IN-PLACE: preserve the
    /// existing clearly-decidable lines verbatim, append the 140 generated
    /// genuinely-ambiguous + adversarially-misleading items, and write the
    /// merged corpus sorted by id (deterministic; re-running is byte-stable).
    LcasExtend {
        /// Path to the existing 70-item `lcas-v0.3.jsonl` (lines preserved verbatim).
        #[arg(long)]
        existing: String,
        /// Output path for the merged 210-item corpus.
        #[arg(long)]
        out: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            corpus,
            mode,
            out,
            rng_seed,
            marker_namespace,
        } => {
            if let Err(e) = run_generate(&corpus, &mode, &out, rng_seed, marker_namespace) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::LcasExtend { existing, out } => {
            if let Err(e) = run_lcas_extend(&existing, &out) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Coverage {
            corpus,
            json,
            seeds_fixture,
        } => {
            if let Err(e) = run_coverage(&corpus, json, seeds_fixture.as_deref()) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn run_generate(
    corpus: &str,
    mode: &str,
    out_path: &str,
    rng_seed: Option<u64>,
    marker_namespace: Option<String>,
) -> Result<(), String> {
    use maos_corpus_gen::CorpusGenerator;

    match corpus {
        "secret-redaction-1e4" => {
            let gen = maos_corpus_gen::secret_redaction::SecretRedactionGenerator::default();
            match mode {
                "per-commit" => {
                    let items = gen.expand(10_000);
                    write_jsonl(out_path, &items)
                }
                "quarterly" => {
                    let items = gen.expand(100_000);
                    write_jsonl(out_path, &items)
                }
                "canary" => {
                    let seed = rng_seed.ok_or("canary mode requires --rng-seed <u64>")?;
                    let ns = marker_namespace
                        .ok_or("canary mode requires --marker-namespace <string>")?;
                    let items = gen.generate_canary_batch(1000, seed, &ns);
                    write_jsonl(out_path, &items)
                }
                other => Err(format!("unknown mode: {}", other)),
            }
        }
        "red-team-640" => {
            let gen = maos_corpus_gen::red_team::RedTeamGenerator::default();
            let items = gen.expand(640);
            write_jsonl(out_path, &items)
        }
        "ccac-600" => {
            let gen = maos_corpus_gen::ccac::CcacGenerator::new();
            let items = gen.expand(maos_corpus_gen::ccac::CORPUS_SIZE);
            write_jsonl(out_path, &items)
        }
        other => Err(format!(
            "unknown corpus name; supported: secret-redaction-1e4, red-team-640, ccac-600\n  got: {}",
            other
        )),
    }
}

fn run_coverage(corpus: &str, json: bool, seeds_fixture: Option<&str>) -> Result<(), String> {
    if let Some(fixture_path) = seeds_fixture {
        let path = std::path::Path::new(fixture_path);
        return run_coverage_with_fixture(corpus, json, path);
    }

    match corpus {
        "secret-redaction-1e4" => {
            maos_corpus_gen::secret_redaction::run_coverage("secret-redaction-1e4", json)
        }
        "red-team-640" => maos_corpus_gen::red_team::run_coverage("red-team-640", json),
        "ccac-600" => maos_corpus_gen::ccac::run_coverage("ccac-600", json),
        other => Err(format!(
            "unknown corpus name; supported: secret-redaction-1e4, red-team-640, ccac-600\n  got: {}",
            other
        )),
    }
}

fn run_coverage_with_fixture(
    corpus: &str,
    json: bool,
    fixture_path: &std::path::Path,
) -> Result<(), String> {
    use maos_corpus_gen::CorpusGenerator;
    match corpus {
        "secret-redaction-1e4" => {
            let gen =
                maos_corpus_gen::secret_redaction::SecretRedactionGenerator::with_fixture_seeds(
                    fixture_path,
                )
                .map_err(|e| format!("failed to load fixture: {}", e))?;
            let report = gen.coverage_report();
            let ac5_floor = 1000;
            for (class, cc) in &report.classes {
                if cc.expanded_count < ac5_floor {
                    eprintln!(
                        "NFR-Sec-4 floor violation: class {} has {} items, floor is {}",
                        class, cc.expanded_count, ac5_floor
                    );
                    return Err(format!("NFR-Sec-4 floor violation: class {}", class));
                }
            }
            if json {
                let out = serde_json::to_string_pretty(&report)
                    .map_err(|e| format!("JSON serialization error: {}", e))?;
                println!("{}", out);
            } else {
                maos_corpus_gen::secret_redaction::print_text_report(&report);
            }
            Ok(())
        }
        "red-team-640" => {
            let gen = maos_corpus_gen::red_team::RedTeamGenerator::with_fixture_seeds(fixture_path)
                .map_err(|e| format!("failed to load fixture: {}", e))?;
            let report = gen.coverage_report();
            let floor = 80;
            for (class, cc) in &report.classes {
                if cc.expanded_count < floor {
                    eprintln!(
                        "NFR-Sec-10 floor violation: class {} has {} items, floor is {}",
                        class, cc.expanded_count, floor
                    );
                    return Err(format!("NFR-Sec-10 floor violation: class {}", class));
                }
            }
            if json {
                let out = serde_json::to_string_pretty(&report)
                    .map_err(|e| format!("JSON serialization error: {}", e))?;
                println!("{}", out);
            } else {
                maos_corpus_gen::red_team::print_text_report(&report);
            }
            Ok(())
        }
        "ccac-600" => {
            maos_corpus_gen::ccac::run_coverage_with_fixture("ccac-600", json, fixture_path)
        }
        other => Err(format!(
            "unknown corpus name; supported: secret-redaction-1e4, red-team-640\n  got: {}",
            other
        )),
    }
}

/// Story 7.4 — merge the existing clearly-decidable LCAS lines (verbatim) with
/// the 140 generated genuinely-ambiguous + adversarially-misleading items,
/// sorted by id, written `\n`-terminated. Deterministic: same inputs → same
/// bytes (the SHA-pin discipline).
fn run_lcas_extend(existing_path: &str, out_path: &str) -> Result<(), String> {
    // (id, json_line) pairs. Existing lines are preserved BYTE-FOR-BYTE.
    let mut rows: Vec<(String, String)> = Vec::new();

    let existing = std::fs::read_to_string(existing_path)
        .map_err(|e| format!("cannot read existing corpus {existing_path}: {e}"))?;
    for line in existing.lines() {
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| format!("existing corpus line is not valid JSON: {e}"))?;
        let id = v
            .get("id")
            .and_then(|i| i.as_str())
            .ok_or_else(|| "existing corpus line missing string `id`".to_string())?
            .to_string();
        rows.push((id, line.to_string()));
    }

    for item in maos_corpus_gen::lcas::generate_extension() {
        let line = serde_json::to_string(&item)
            .map_err(|e| format!("serialization error: {e}"))?;
        rows.push((item.id.clone(), line));
    }

    // Sort by id ascending; ids are unique across the three buckets.
    rows.sort_by(|a, b| a.0.cmp(&b.0));

    let mut buf = String::new();
    for (_, line) in &rows {
        buf.push_str(line);
        buf.push('\n');
    }
    std::fs::write(out_path, buf).map_err(|e| format!("I/O error writing {out_path}: {e}"))?;
    eprintln!(
        "lcas-extend: wrote {} items ({} existing + 140 generated) to {}",
        rows.len(),
        rows.len() - 140,
        out_path
    );
    Ok(())
}

fn write_jsonl<I: serde::Serialize>(path: &str, items: &[I]) -> Result<(), String> {
    let mut buf = String::new();
    for item in items {
        let line =
            serde_json::to_string(item).map_err(|e| format!("serialization error: {}", e))?;
        buf.push_str(&line);
        buf.push('\n');
    }
    std::fs::write(path, buf).map_err(|e| format!("I/O error writing {}: {}", path, e))?;
    Ok(())
}
