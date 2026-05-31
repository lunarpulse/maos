use clap::{Parser, Subcommand};
use std::path::Path;
use std::process;

mod abi_diff;
mod calibrate;
mod check_adr_040_accepted;
mod check_bare_review_findings;
mod check_composition_root_completeness;
mod check_corpus;
mod check_deprecations_declared;
mod check_dev_model_used_populated;
mod check_dev_record_completeness;
mod check_empty_kernel;
mod check_epic_6_bridge;
mod check_fr47;
mod check_judge_config;
mod check_loom;
mod check_manifest_schema_version;
pub mod check_mock_not_in_release;
mod check_multi_provider_drift;
mod check_pub_field_constructors;
mod check_review_findings_resolved;
mod check_security_md;
mod check_serde_error_handling;
mod check_service_boundary;
mod check_unsafe;
mod check_workspace_count;
mod corpus_staleness;
mod corpus_types;
mod coverage_matrix;
mod coverage_matrix_nfr_test_3;
mod example_spirit_regen;
mod fs_walk;
mod gen_isolation_corpus;
mod gen_termination_corpus;
mod invariant_lock;
mod kloc_check;
mod rebaseline_check;
mod templates_regen;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "MAOS workspace automation — CI gates, linting, checks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// AC2 — Zero-unsafe gate for the capability-validation path.
    CheckUnsafe {
        #[arg(long, default_value = "crates/maos-kernel-core/capability")]
        path: String,
        #[arg(long)]
        json: bool,
    },
    /// AC3 — KLOC budget check with alarm and hard-fail thresholds.
    KlocCheck {
        #[arg(long, default_value = "xtask/kloc.toml")]
        config: String,
        #[arg(long)]
        json: bool,
    },
    /// AC4 — ABI diff against the previous tagged baseline.
    AbiDiff {
        #[arg(long, default_value = "HEAD~1")]
        base: String,
        #[arg(long)]
        json: bool,
    },
    /// AC6 — I9 structural-state lint for empty-kernel invariant.
    CheckEmptyKernel {
        #[arg(long, default_value = "crates/maos-kernel-core")]
        path: String,
        #[arg(long, default_value = "xtask/i9-whitelist.toml")]
        whitelist: String,
        #[arg(long, default_value = "xtask/i9-denylist.toml")]
        denylist: String,
        #[arg(long, default_value = "docs/invariants/i9-exemptions.md")]
        exemptions: String,
        #[arg(long)]
        json: bool,
    },
    /// AC7 — NFR-Test-9 Loom-not-in-kernel structural grep.
    CheckLoom {
        #[arg(long)]
        path: Option<String>,
        #[arg(long, default_value = "xtask/kernel-crates.toml")]
        crates: String,
        #[arg(long, default_value = "xtask/loom-blocklist.toml")]
        blocklist: String,
        #[arg(long, default_value = "xtask/loom-allowlist.toml")]
        allowlist: String,
        #[arg(long)]
        json: bool,
    },
    /// AC8 — NFR-Test-2 service-boundary P1–P4 + Spirit-ABI reflection.
    CheckServiceBoundary {
        #[arg(long)]
        path: Option<String>,
        #[arg(
            long,
            default_value = "docs/ci-baselines/kernel-surface-v0.1-beta.json"
        )]
        baseline: String,
        #[arg(long, default_value = "xtask/kernel-api-classes.toml")]
        classes: String,
        #[arg(long, default_value = "xtask/p4-external-io-denylist.toml")]
        p4_denylist: String,
        #[arg(long, default_value = "xtask/p4-mediated-io-paths.toml")]
        p4_exemptions: String,
        #[arg(long, default_value = "crates/maos-spirit-abi/src/lifecycle.rs")]
        spirit_abi_lifecycle: String,
        #[arg(long, default_value = "crates/maos-spirit-derive/src/lib.rs")]
        spirit_abi_derive: String,
        #[arg(long)]
        json: bool,
    },
    /// AC2 — FR47 enforcement: vendor LLM SDK dependency scan.
    CheckFr47 {
        #[arg(long)]
        path: Option<String>,
        #[arg(long, default_value = "xtask/fr47-vendor-sdk-denylist.toml")]
        denylist: String,
        #[arg(long, default_value = "xtask/fr47-allowlist.toml")]
        allowlist: String,
        #[arg(long)]
        json: bool,
    },
    /// AC9 — NFR-Ops-4 + FR61 SECURITY.md section gate.
    CheckSecurityMd {
        #[arg(long)]
        json: bool,
    },
    /// AC5 — Invariant lock gate for constitutional amendments.
    ///
    /// `--write-journal` opts the run into persisting a journal entry; without
    /// this flag the gate validates only. `--journal-output` sets the file the
    /// entry is written to (default: `docs/invariants/journal.jsonl`). For
    /// Option (c) journal persistence (DF16), the merge-queue workflow sets
    /// `--journal-output /tmp/journal-entry.jsonl` and uploads the file as a
    /// CI artifact; the in-repo journal is rebuilt offline by the operator.
    /// `--pr-body` points at a file containing the PR body for revert detection;
    /// if the body matches the GitHub revert idiom, a paired "reverted" entry
    /// is appended after the primary entry.
    InvariantLock {
        #[arg(long)]
        changed_files: Option<String>,
        #[arg(long)]
        pr_number: Option<u64>,
        #[arg(long)]
        sha: Option<String>,
        #[arg(long)]
        write_journal: bool,
        #[arg(long, default_value = "docs/invariants/journal.jsonl")]
        journal_output: String,
        #[arg(long)]
        pr_body: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// AC1 — SHA-256-pinned JSONL corpus verification (NFR-Test-1).
    CheckCorpus {
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "tests/corpora")]
        corpora_dir: String,
        #[arg(long)]
        register: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// AC2 — Pinned-judge-LLM structural contract (NFR-Test-1).
    CheckJudgeConfig {
        #[arg(long, default_value = "tests/judge-config.toml")]
        config: String,
        #[arg(long, default_value = "xtask/judge-direct-call-identifiers.toml")]
        identifiers: String,
        #[arg(long)]
        json: bool,
    },
    /// AC4 — Coverage-matrix delivered-phase enforcement (NFR-Meta-3).
    CoverageMatrix {
        #[arg(long, default_value = "tests/coverage-matrix.yaml")]
        config: String,
        #[arg(long, default_value = "tests/phase-config.toml")]
        phase_config: String,
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "xtask/gate-registry.toml")]
        gate_registry: String,
        #[arg(long)]
        json: bool,
        /// Story 7.1 — Measure NFR-Test-3 coverage floor.
        #[arg(long)]
        measure_nfr_test_3: bool,
        /// Filter to a single Spirit name (requires --measure-nfr-test-3).
        #[arg(long)]
        spirit: Option<String>,
        /// Report without writing back to YAML.
        #[arg(long)]
        dry_run: bool,
    },
    /// AC5 — Corpus staleness `valid_until` enforcement (NFR-Meta-2).
    CorpusStaleness {
        #[arg(long, default_value = "tests/coverage-matrix.yaml")]
        config: String,
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "30")]
        warn_window_days: i64,
        #[arg(long)]
        json: bool,
    },
    /// AC6 — Calibration Wilson-CI math (NFR-Aud-8).
    Calibrate {
        #[arg(long)]
        corpus: String,
        #[arg(long)]
        n: u64,
        #[arg(long)]
        p: f64,
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "tests/corpora")]
        corpora_dir: String,
        #[arg(long)]
        synthetic_pass_rate: Option<f64>,
        #[arg(long)]
        json: bool,
    },
    /// AC3 — Quarterly rebaseline check (NFR-Test-1).
    RebaselineCheck {
        #[arg(long, default_value = "tests/corpora/MANIFEST.toml")]
        manifest: String,
        #[arg(long, default_value = "tests/corpora")]
        corpora_dir: String,
        #[arg(long, default_value = "tests/judge-config.toml")]
        judge_config: String,
        #[arg(long, default_value = "0.98")]
        threshold: f64,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Story 2.3 — Template-to-example drift detector and regenerator.
    /// DEPRECATED: use `templates-regen` instead.
    ExampleSpiritRegen {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1 — Generalized template-to-example regenerator (Rust + TS).
    TemplatesRegen {
        #[arg(long)]
        lang: Option<String>,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 2.5 AC3 — workspace-member-count guard (Cargo.toml vs architecture doc).
    CheckWorkspaceCount {
        #[arg(long, default_value = "Cargo.toml")]
        cargo_toml: String,
        #[arg(
            long,
            default_value = "_bmad-output/planning-artifacts/architecture-maos-minimal-opus/4-kernel-design.md"
        )]
        kernel_design: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 4.1 AC4 — deterministic 1000-scenario termination-corpus generator.
    GenTerminationCorpus {
        #[arg(
            long,
            default_value = "crates/maos-eval/fixtures/termination-corpus-v0"
        )]
        out_dir: String,
    },
    /// Story 4.5 — deterministic 200-scenario cross-spirit isolation-corpus generator.
    GenIsolationCorpus {
        #[arg(long, default_value = "crates/maos-eval/fixtures/isolation-corpus-v0")]
        out_dir: String,
    },
    /// Story 4.1 A2 — check release binary for forbidden test-double symbols.
    CheckMockNotInRelease {
        #[arg(long, default_value = "target/release/maos")]
        binary: String,
        #[arg(long)]
        build_first: bool,
        #[arg(long)]
        json: bool,
    },
    /// Epic 4 retro §A4 — pub fields with `Construct via ::new` doc-attr MUST have a matching `::new` impl.
    CheckPubFieldConstructors {
        #[arg(long)]
        json: bool,
    },
    /// Epic 4 retro §A5 — every `*Adapter` re-exported from `api.rs` MUST be constructed exactly once in `main.rs`.
    CheckCompositionRootCompleteness {
        #[arg(long, default_value = "crates/maos-kernel-core/src/api.rs")]
        api_rs: String,
        #[arg(long, default_value = "crates/maos-bin/src/main.rs")]
        main_rs: String,
        #[arg(long, default_value = "xtask/composition-root-whitelist.toml")]
        whitelist: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 5.5b — multi-provider drift check across aggregated reports.
    CheckMultiProviderDrift {
        #[arg(long)]
        report: String,
        #[arg(long, default_value = "10")]
        threshold: f64,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        json: bool,
    },
    /// Story 5.5e — v0.5 release-block: ADR-040 must be committed with status `accepted`.
    CheckAdr040Accepted {
        #[arg(long)]
        json: bool,
    },
    /// Epic 5 retro §A3 (closes Epic 4 retro §A6) — flag `.unwrap_or_default()` / `.unwrap()` / `.expect(...)` after serde calls.
    CheckSerdeErrorHandling {
        #[arg(long, default_value = "crates")]
        path: String,
        #[arg(long, default_value = "xtask/serde-error-allowlist.toml")]
        allowlist: String,
        #[arg(long)]
        json: bool,
    },
    /// Epic 5 retro §A5 — open Review Findings rows MUST block sprint-status `done`; closed rows MUST reference a File List entry.
    CheckReviewFindingsResolved {
        #[arg(long, default_value = "_bmad-output/implementation-artifacts")]
        stories_dir: String,
        #[arg(
            long,
            default_value = "_bmad-output/implementation-artifacts/sprint-status.yaml"
        )]
        sprint_status: String,
        #[arg(long)]
        json: bool,
    },
    /// Story 6.1 AC1 — Epic 6 bridge precondition gate (9 mechanical checks).
    /// Story 6.2 AC1 — extended with `--story 6.2` flag adding 6.2-specific rows
    /// (D-2.10 retract-corpus, D-4.* iac-routing-budget, D-3.7/3.8 DRR fairness,
    /// D-5.1/5.2 smoke-iac-bus-6, §A4-Debt-2c hook-count drift).
    /// Story 6.3 AC1 — extended with `--story 6.3` flag adding 10 6.3-specific
    /// row classifications (§A3/§A5/§A6 gate-exists, 6.2 smoke-arm, iac-routing
    /// budget, retract-corpus, DRR carry-forward, cli_wrapper bench carry-forward,
    /// §A2 backfill carry-forward, 6.2 RF count, smoke-iac-bus chain, maos-a2a
    /// baseline). When `--story 6.X` is set, blocking_6_X rows must clear before
    /// the command exits 0.
    /// Story 6.4 AC1 — extended with `--story 6.4` flag adding 10 6.4-specific
    /// row classifications (§A3/§A5/§A6 gate-exists, 6.3 smoke-arm, 6.3-P4 CI
    /// test-target verification, 6.3 RF count, DRR carry-forward, cli_wrapper
    /// bench carry-forward, §A2 backfill carry-forward, maos-providers baseline,
    /// FrameKind baseline, ScheduleWatchdog baseline).
    #[command(name = "check-epic-6-bridge")]
    CheckEpic6Bridge {
        #[arg(long)]
        json: bool,
        /// Story scope — "6.2" / "6.3" / "6.4" extends with the story-specific
        /// rows. Unset = Story 6.1 legacy 9-check semantics.
        #[arg(long)]
        story: Option<String>,
    },
    /// Epic 5 retro §A6 (closes Epic 4 retro §A7) — `done` stories MUST have non-TBD model + non-empty dev record + File List files in `git diff`.
    CheckDevRecordCompleteness {
        #[arg(long, default_value = "_bmad-output/implementation-artifacts")]
        stories_dir: String,
        #[arg(
            long,
            default_value = "_bmad-output/implementation-artifacts/sprint-status.yaml"
        )]
        sprint_status: String,
        #[arg(long, default_value_t = false)]
        check_git_diff: bool,
        #[arg(long)]
        json: bool,
    },
    /// Epic 6 §A4 (retro 2026-05-28) — keep MANIFEST_SCHEMA_VERSION + MIN/MAX
    /// constants self-consistent and ban hardcoded `manifest_schema_version`
    /// comparisons in `maos-manifest` production code.
    #[command(name = "check-manifest-schema-version")]
    CheckManifestSchemaVersion {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1 — assert ZERO deprecation annotations at v0.5 HEAD.
    #[command(name = "check-deprecations-declared")]
    CheckDeprecationsDeclared {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1.5 — assert ZERO bare `_No review findings._` placeholders remain.
    #[command(name = "check-bare-review-findings")]
    CheckBareReviewFindings {
        #[arg(long)]
        json: bool,
    },
    /// Story 7.1.5 — assert every story file has a populated `dev_model_used:` frontmatter field.
    #[command(name = "check-dev-model-used-populated")]
    CheckDevModelUsedPopulated {
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::CheckUnsafe { path, json } => check_unsafe::run(&path, json),
        Commands::CheckEmptyKernel {
            path,
            whitelist,
            denylist,
            exemptions,
            json,
        } => check_empty_kernel::run(&path, &whitelist, &denylist, &exemptions, json),
        Commands::CheckLoom {
            path,
            crates,
            blocklist,
            allowlist,
            json,
        } => check_loom::run(path.as_deref(), &crates, &blocklist, &allowlist, json),
        Commands::CheckFr47 {
            path,
            denylist,
            allowlist,
            json,
        } => check_fr47::run(path.as_deref(), &denylist, &allowlist, json),
        Commands::CheckServiceBoundary {
            path,
            baseline,
            classes,
            p4_denylist,
            p4_exemptions,
            spirit_abi_lifecycle,
            spirit_abi_derive,
            json,
        } => check_service_boundary::run(
            path.as_deref(),
            &baseline,
            &classes,
            &p4_denylist,
            &p4_exemptions,
            &spirit_abi_lifecycle,
            &spirit_abi_derive,
            json,
        ),
        Commands::CheckSecurityMd { json } => {
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            let report = check_security_md::check_security_md(&workspace_root);
            if json {
                let payload = serde_json::json!({
                    "passed": report.passed,
                    "present_sections": report.present_sections,
                    "missing_sections": report.missing_sections,
                });
                println!("{}", payload);
            } else if report.passed {
                eprintln!(
                    "check-security-md: PASS ({} sections found)",
                    report.present_sections.len()
                );
            } else {
                eprintln!(
                    "check-security-md: FAIL — missing sections: {:?}",
                    report.missing_sections
                );
            }
            if report.passed {
                Ok(())
            } else {
                Err("SECURITY.md missing required sections".into())
            }
        }
        Commands::KlocCheck { config, json } => kloc_check::run(&config, json),
        Commands::AbiDiff { base, json } => abi_diff::run(&base, json),
        Commands::InvariantLock {
            changed_files,
            pr_number,
            sha,
            write_journal,
            journal_output,
            pr_body,
            json,
        } => invariant_lock::run(
            changed_files.as_deref(),
            pr_number,
            sha.as_deref(),
            write_journal,
            &journal_output,
            pr_body.as_deref(),
            json,
        ),
        Commands::CheckCorpus {
            manifest,
            corpora_dir,
            register,
            json,
        } => check_corpus::run(&manifest, &corpora_dir, register.as_deref(), json),
        Commands::CheckJudgeConfig {
            config,
            identifiers,
            json,
        } => check_judge_config::run(&config, &identifiers, json),
        Commands::CoverageMatrix {
            config,
            phase_config,
            manifest,
            gate_registry,
            json,
            measure_nfr_test_3,
            spirit,
            dry_run,
        } => {
            if measure_nfr_test_3 {
                coverage_matrix_nfr_test_3::run(&config, spirit.as_deref(), dry_run, json)
            } else {
                coverage_matrix::run(&config, &phase_config, &manifest, &gate_registry, json)
            }
        }
        Commands::CorpusStaleness {
            config,
            manifest,
            warn_window_days,
            json,
        } => corpus_staleness::run(&config, &manifest, warn_window_days, json),
        Commands::Calibrate {
            corpus,
            n,
            p,
            manifest,
            corpora_dir,
            synthetic_pass_rate,
            json,
        } => calibrate::run(
            &corpus,
            n,
            p,
            &manifest,
            &corpora_dir,
            synthetic_pass_rate,
            json,
        ),
        Commands::RebaselineCheck {
            manifest,
            corpora_dir,
            judge_config,
            threshold,
            out,
            json,
        } => rebaseline_check::run(
            &manifest,
            &corpora_dir,
            &judge_config,
            threshold,
            out.as_deref(),
            json,
        ),
        Commands::ExampleSpiritRegen { check, json } => {
            eprintln!(
                "WARN: example-spirit-regen is deprecated; use templates-regen --lang rust instead"
            );
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            templates_regen::run(
                &workspace_root,
                Some(templates_regen::Language::Rust),
                check,
                json,
            )
        }
        Commands::TemplatesRegen { lang, check, json } => {
            let workspace_root = std::env::current_dir().expect("failed to get current dir");
            let lang = match lang.as_deref().map(|s| s.parse()).transpose() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            };
            templates_regen::run(&workspace_root, lang, check, json)
        }
        Commands::CheckWorkspaceCount {
            cargo_toml,
            kernel_design,
            json,
        } => check_workspace_count::run(&cargo_toml, &kernel_design, json),
        Commands::GenTerminationCorpus { out_dir } => gen_termination_corpus::run(&out_dir),
        Commands::GenIsolationCorpus { out_dir } => gen_isolation_corpus::run(&out_dir),
        Commands::CheckMockNotInRelease {
            binary,
            build_first,
            json,
        } => check_mock_not_in_release::run(&binary, build_first, json),
        Commands::CheckPubFieldConstructors { json } => check_pub_field_constructors::run(json),
        Commands::CheckCompositionRootCompleteness {
            api_rs,
            main_rs,
            whitelist,
            json,
        } => check_composition_root_completeness::run(&api_rs, &main_rs, &whitelist, json),
        Commands::CheckMultiProviderDrift {
            report,
            threshold,
            strict,
            json,
        } => {
            let exit_code =
                check_multi_provider_drift::run(Path::new(&report), threshold, strict, json);
            if exit_code != 0 {
                process::exit(exit_code);
            }
            Ok(())
        }
        Commands::CheckAdr040Accepted { json } => check_adr_040_accepted::run(json),
        Commands::CheckSerdeErrorHandling {
            path,
            allowlist,
            json,
        } => check_serde_error_handling::run(&path, &allowlist, json),
        Commands::CheckReviewFindingsResolved {
            stories_dir,
            sprint_status,
            json,
        } => check_review_findings_resolved::run(&stories_dir, &sprint_status, json),
        Commands::CheckEpic6Bridge { json, story } => {
            check_epic_6_bridge::run_with_story(json, story.as_deref())
        }
        Commands::CheckDevRecordCompleteness {
            stories_dir,
            sprint_status,
            check_git_diff,
            json,
        } => check_dev_record_completeness::run(&stories_dir, &sprint_status, check_git_diff, json),
        Commands::CheckManifestSchemaVersion { json } => check_manifest_schema_version::run(json),
        Commands::CheckDeprecationsDeclared { json } => check_deprecations_declared::run(json),
        Commands::CheckBareReviewFindings { json } => check_bare_review_findings::run(json),
        Commands::CheckDevModelUsedPopulated { json } => check_dev_model_used_populated::run(json),
    };
    if let Err(e) = result {
        eprintln!("{e}");
        process::exit(1);
    }
}
