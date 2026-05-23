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
        other => Err(format!(
            "unknown corpus name; supported: secret-redaction-1e4, red-team-640\n  got: {}",
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
        other => Err(format!(
            "unknown corpus name; supported: secret-redaction-1e4, red-team-640\n  got: {}",
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
        other => Err(format!(
            "unknown corpus name; supported: secret-redaction-1e4, red-team-640\n  got: {}",
            other
        )),
    }
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
