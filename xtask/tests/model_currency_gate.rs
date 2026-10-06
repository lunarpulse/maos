//! Story `17-5` — proven-red vectors for `check-model-currency`.
//!
//! **Every red here is planted in a COPY of the real tree and read back through
//! the SAME `audit()` the CI job runs.** A vector that exercised a parallel copy
//! of the scanner would prove nothing about the gate that actually runs, and a
//! hand-written fixture can be satisfied by the fixture's own shape (E15-A6: does
//! the test read the tree, or only what it set itself?). So the control is the
//! unmodified copy — GREEN — and each vector is one substitution on a real
//! surface, asserted to have CHANGED that surface (RT-10: show the plant, or the
//! fault was not planted) and to red naming that surface's exact `file:line`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use xtask::check_model_currency::{audit, parse_data, Report};

fn real_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives under the workspace root")
        .to_path_buf()
}

/// A copy of exactly the files the gate reads, so a plant never touches the repo.
struct Tree {
    dir: tempfile::TempDir,
}

impl Tree {
    fn real() -> Self {
        let tree = Tree {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        let mut rels = vec![
            "xtask/model-currency.toml".to_string(),
            "xtask/provider-pricing.toml".to_string(),
            "crates/maos-bin/src/main.rs".to_string(),
        ];
        for top in ["spirits", "templates", "examples"] {
            for entry in fs::read_dir(real_root().join(top)).expect("surface dir") {
                let rel = format!(
                    "{top}/{}/manifest.toml",
                    entry.unwrap().file_name().to_string_lossy()
                );
                if real_root().join(&rel).is_file() {
                    rels.push(rel);
                }
            }
        }
        for rel in &rels {
            tree.write(
                rel,
                &fs::read_to_string(real_root().join(rel)).expect("real surface"),
            );
        }
        tree
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn text(&self, rel: &str) -> String {
        fs::read_to_string(self.root().join(rel)).expect("read")
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.root().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Replace the single occurrence of `from` in `rel`; return its 1-based line.
    fn plant(&self, rel: &str, from: &str, to: &str) -> usize {
        let text = self.text(rel);
        assert_eq!(
            text.matches(from).count(),
            1,
            "`{from}` must occur exactly once in {rel}"
        );
        let line = text[..text.find(from).unwrap()].matches('\n').count() + 1;
        self.write(rel, &text.replacen(from, to, 1));
        assert_ne!(self.text(rel), text, "the plant must change {rel}");
        line
    }

    fn judge(&self) -> Report {
        audit(self.root()).expect("the planted tree must still be readable")
    }
}

#[test]
fn the_swept_tree_is_green_and_governs_its_pins() {
    // Both the real tree and the copy: if only the copy were checked, a harness
    // that copied the wrong files could stay green forever. The real root also
    // holds the inert `#[cfg(test)]` fixtures and fuzz seeds that pin retired ids
    // (AC7) — they are outside the scanned set by construction, so it stays green.
    for report in [
        audit(&real_root()).expect("real tree"),
        Tree::real().judge(),
    ] {
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        // 5 manifest pins + 3 price-book rows + 3 provider constructors. `>=`, so
        // a new spirit does not red this, but a scanner that finds nothing does.
        assert!(
            report.pins >= 11,
            "the gate must see the known pins: {}",
            report.pins
        );
    }
}

#[test]
fn each_retired_or_unknown_pin_reds_naming_its_file_and_line() {
    const RETIRED: &str = "RETIRED model id";
    // (surface, current text, planted text, expected detail fragment)
    let vectors: &[(&str, &str, &str, &str)] = &[
        ("spirits/butler/manifest.toml", "anthropic.claude-haiku-4-5-20251001", "anthropic.claude-3-haiku-20240307",
         "RETIRED model id `anthropic.claude-3-haiku-20240307` — replacement (display-only guidance): `claude-haiku-4-5-20251001`"),
        ("spirits/researcher/manifest.toml", "anthropic.claude-sonnet-5-5", "anthropic.claude-3-5-sonnet-20241022",
         "replacement (display-only guidance): `claude-sonnet-5-5`"),
        ("spirits/hello-spirit/manifest.toml", "anthropic.claude-haiku-4-5-20251001", "anthropic.claude-sonnet-4-5-20250929", RETIRED),
        ("templates/spirit-rust/manifest.toml", "anthropic.claude-haiku-4-5-20251001", "anthropic.claude-3-haiku-20240307", RETIRED),
        ("examples/example-spirit/manifest.toml", "anthropic.claude-haiku-4-5-20251001", "anthropic.claude-3-haiku-20240307", RETIRED),
        ("xtask/provider-pricing.toml", "model = \"gpt-4o-mini\"", "model = \"gpt-4\"",
         "replacement (display-only guidance): `gpt-5.6-sol`"),
        ("xtask/provider-pricing.toml", "model = \"claude-sonnet-5-5\"", "model = \"claude-3-5-sonnet\"",
         "UNKNOWN model id `anthropic.claude-3-5-sonnet`"),
        ("crates/maos-bin/src/main.rs", "\"claude-haiku-4-5-20251001\".into()", "\"claude-3-haiku-20240307\".into()", RETIRED),
        ("crates/maos-bin/src/main.rs", "\"gpt-4o-mini\".into()", "\"gpt-4o-minnie\".into()",
         "UNKNOWN model id `openai.gpt-4o-minnie`"),
        ("crates/maos-bin/src/main.rs", "\"llama3.1:8b\".into()", "\"llama3:8b\".into()",
         "UNKNOWN model id `ollama.llama3:8b`"),
        ("spirits/butler/manifest.toml", "\"anthropic.claude-haiku", "\"acme.claude-haiku", "UNKNOWN provider for `acme."),
    ];
    for (surface, from, to, expect) in vectors {
        let tree = Tree::real();
        let line = tree.plant(surface, from, to);
        let report = tree.judge();
        assert_eq!(report.findings.len(), 1, "{surface}: {:?}", report.findings);
        let found = &report.findings[0];
        assert_eq!(
            (found.file.as_str(), found.line),
            (*surface, line),
            "{surface}"
        );
        assert!(found.detail.contains(expect), "{surface}: {}", found.detail);
    }
}

#[test]
fn a_provider_table_reds_on_the_entry_line_through_comments_literals_and_duplicates() {
    let tree = Tree::real();
    tree.write(
        "spirits/zz-fixture/manifest.toml",
        "# was \"anthropic.claude-3-haiku-20240307\" — migrate\n\
         [capabilities.required.provider]\n\
         complete = [\n\
         \x20   \"anthropic.claude-sonnet-5-5\", # was \"anthropic.claude-3-haiku-20240307\"\n\
         \x20   'anthropic.claude-3-haiku-20240307',\n\
         \x20   \"anthropic.claude-3-haiku-20240307\",\n\
         \x20   \"anthropic\",\n\
         ]\n",
    );
    let report = tree.judge();
    let lines: Vec<usize> = report.findings.iter().map(|f| f.line).collect();
    assert_eq!(
        lines,
        [5, 6],
        "the literal-string and the duplicate, each on its own line; the dotless \
         provider-wide entry pins no model; no comment counts: {:?}",
        report.findings
    );
    assert!(report
        .findings
        .iter()
        .all(|f| f.file == "spirits/zz-fixture/manifest.toml"));
}

#[test]
fn two_price_rows_sharing_a_model_each_name_their_own_line() {
    let tree = Tree::real();
    let row = |provider: &str| {
        format!("[[entries]]\nprovider = \"{provider}\"\nmodel = \"gpt-4o-mini\"\ninput_price_micro_per_1k = 1\noutput_price_micro_per_1k = 1\n\n")
    };
    tree.write(
        "xtask/provider-pricing.toml",
        &format!("{}{}", row("openai"), row("acme")),
    );
    let report = tree.judge();
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert_eq!(
        report.findings[0].line, 9,
        "the acme row, not the first row"
    );
    assert!(report.findings[0]
        .detail
        .contains("UNKNOWN provider for `acme.gpt-4o-mini`"));
}

#[test]
fn a_manifest_that_hides_its_pins_from_the_scan_reds() {
    for hidden in [
        // A string where an array belongs, and a misspelled key.
        "[capabilities.required]\nprovider.complete = \"anthropic.claude-3-haiku-20240307\"\n",
        "[capabilities.required]\nprovider.compelte = [\"anthropic.claude-3-haiku-20240307\"]\n",
    ] {
        let tree = Tree::real();
        tree.write("spirits/zz-hidden/manifest.toml", hidden);
        let report = tree.judge();
        assert_eq!(report.findings.len(), 1, "{hidden}: {:?}", report.findings);
        assert!(report.findings[0].detail.contains("no `complete` array"));
    }
}

#[test]
fn main_rs_constructors_are_read_in_every_layout_and_comments_open_no_call() {
    let tree = Tree::real();
    tree.write(
        "crates/maos-bin/src/main.rs",
        "fn a() {\n\
         \x20   // AnthropicProvider::new(\n\
         \x20   let _ = AnthropicProvider::with_api_key(io, \"https://x\".into(), \"claude-3-haiku-20240307\".into(), \"sk-secret\".into()); // one line\n\
         \x20   match maos_providers::OpenAiProvider::new(\n\
         \x20       io,\n\
         \x20       \"https://api.openai.com\".into(),\n\
         \x20       \"gpt-4o-mini\".into(), // trailing comment\n\
         \x20       None,\n\
         \x20   ) {\n\
         \x20       _ => {}\n\
         \x20   }\n\
         \x20   let _ = maos_providers::OllamaProvider::new(io, url, \"llama3.1:8b\".to_string());\n\
         }\n",
    );
    let report = tree.judge();
    assert!(report.pins >= 3, "all three constructors are read");
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let found = &report.findings[0];
    assert_eq!(
        found.line, 3,
        "the one-line `with_api_key` call; its api-key literal is never judged"
    );
    assert!(found
        .detail
        .contains("RETIRED model id `anthropic.claude-3-haiku-20240307`"));
}

#[test]
fn a_host_provider_constructor_that_goes_missing_reds_even_if_the_others_remain() {
    let tree = Tree::real();
    tree.write(
        "crates/maos-bin/src/main.rs",
        "fn a() {\n    let _ = AnthropicProvider::new(\n        io,\n        \"https://x\".into(),\n        \"claude-haiku-4-5-20251001\".into(),\n    );\n    let _ = OpenAiProvider::new(\n        io,\n        \"https://y\".into(),\n        \"gpt-4o-mini\".into(),\n    );\n}\n",
    );
    assert!(refusal(&tree).contains("no `ollama` provider constructor found"));
}

#[test]
fn an_unparseable_manifest_reds_instead_of_passing_unread() {
    let tree = Tree::real();
    tree.write(
        "templates/zz-broken/manifest.toml",
        "[capabilities.required\nprovider.complete = [",
    );
    let report = tree.judge();
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert!(report.findings[0].detail.contains("unparseable TOML"));
}

/// Why the gate refused this tree: an `Err`, or findings. Never green.
fn refusal(tree: &Tree) -> String {
    match audit(tree.root()) {
        Err(e) => e,
        Ok(r) if !r.findings.is_empty() => r
            .findings
            .iter()
            .map(|f| format!("{}:{} {}", f.file, f.line, f.detail))
            .collect::<Vec<_>>()
            .join("\n"),
        Ok(r) => panic!("a tree the gate cannot govern passed ({} pins)", r.pins),
    }
}

#[test]
fn a_gate_that_could_scan_nothing_is_a_refusal_not_a_pass() {
    // A missing data file.
    let tree = Tree::real();
    fs::remove_file(tree.root().join("xtask/model-currency.toml")).unwrap();
    assert!(refusal(&tree).contains("cannot read xtask/model-currency.toml"));

    // Zero manifests.
    let tree = Tree::real();
    for top in ["spirits", "templates", "examples"] {
        for entry in fs::read_dir(tree.root().join(top)).unwrap() {
            let _ = fs::remove_file(entry.unwrap().path().join("manifest.toml"));
        }
    }
    assert!(refusal(&tree).contains("no manifest found"));

    // No provider constructor in main.rs.
    let tree = Tree::real();
    tree.write("crates/maos-bin/src/main.rs", "fn main() {}\n");
    assert!(refusal(&tree)
        .contains("crates/maos-bin/src/main.rs:1 no `anthropic` provider constructor found"));

    // A constructor whose model is no longer a readable literal.
    let tree = Tree::real();
    tree.plant(
        "crates/maos-bin/src/main.rs",
        "\"claude-haiku-4-5-20251001\".into()",
        "default_model()",
    );
    assert!(refusal(&tree).contains("no readable model literal"));

    // A price book with no rows.
    let tree = Tree::real();
    tree.write("xtask/provider-pricing.toml", "# empty\n");
    assert!(refusal(&tree).contains("xtask/provider-pricing.toml:1 no `[[entries]]`"));
}

#[test]
fn the_data_file_is_refused_when_it_would_make_the_guidance_lie() {
    let ok = "[providers.p]\nallowed = [\"a\", \"b\"]\n[[providers.p.retired]]\nid = \"old\"\nreplacement = \"a\"\n";
    assert!(parse_data(ok).is_ok(), "the control data must parse");
    let cases = [
        // (broken data, expected error fragment)
        (ok.replace("replacement = \"a\"\n", ""), "replacement"),
        (
            ok.replace("replacement = \"a\"", "replacement = \"nowhere\""),
            "in `p`'s `allowed`",
        ),
        (
            ok.replace("id = \"old\"", "id = \"b\""),
            "both allowed and retired",
        ),
        (
            ok.replace("allowed = [\"a\", \"b\"]", "allowed = []"),
            "non-empty `allowed`",
        ),
        (ok.replace("allowed", "alowed"), "alowed"),
        ("# nothing\n".to_string(), "providers"),
    ];
    for (broken, fragment) in cases {
        let err = parse_data(&broken)
            .err()
            .unwrap_or_else(|| panic!("accepted: {broken}"));
        assert!(err.contains(fragment), "`{fragment}` not in: {err}");
    }
}

#[test]
fn the_shipped_data_names_a_replacement_for_every_retired_id() {
    // `parse_data` already enforces replacement ∈ allowed; this pins the real file
    // to it so a hand edit that breaks the guidance cannot reach CI green.
    let data = fs::read_to_string(real_root().join("xtask/model-currency.toml")).unwrap();
    parse_data(&data).expect("xtask/model-currency.toml must satisfy its own invariants");
}

#[test]
fn the_cli_subcommand_is_wired_green_at_head_and_red_on_a_plant() {
    let xtask = env!("CARGO_BIN_EXE_xtask");
    let green = Command::new(xtask)
        .args(["check-model-currency", "--json"])
        .current_dir(real_root())
        .output()
        .unwrap();
    assert!(
        green.status.success(),
        "{}",
        String::from_utf8_lossy(&green.stderr)
    );
    assert!(String::from_utf8_lossy(&green.stdout).contains("\"passed\":true"));

    let tree = Tree::real();
    let line = tree.plant(
        "spirits/butler/manifest.toml",
        "anthropic.claude-haiku-4-5-20251001",
        "anthropic.claude-3-haiku-20240307",
    );
    let red = Command::new(xtask)
        .arg("check-model-currency")
        .current_dir(tree.root())
        .output()
        .unwrap();
    assert!(!red.status.success(), "a retired pin must fail the process");
    let stderr = String::from_utf8_lossy(&red.stderr);
    assert!(
        stderr.contains(&format!("spirits/butler/manifest.toml:{line} — RETIRED")),
        "{stderr}"
    );
}

// ── Frontier re-run review (2026-10-06) — each vector reds the pre-patch gate. ──

#[test]
fn a_comment_inside_a_constructor_never_stands_in_for_its_model() {
    // Pre-patch, the first non-URL literal in the window won, so a commented-out
    // current id hid the live retired one (fail-open).
    let tree = Tree::real();
    let line = tree.plant(
        "crates/maos-bin/src/main.rs",
        "\"claude-haiku-4-5-20251001\".into(),",
        "// was \"claude-haiku-4-5-20251001\".into(),\n        \"claude-3-haiku-20240307\".into(),",
    ) + 1;
    let report = tree.judge();
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert_eq!(report.findings[0].line, line, "{:?}", report.findings);
    assert!(report.findings[0]
        .detail
        .contains("RETIRED model id `anthropic.claude-3-haiku-20240307`"));
}

#[test]
fn a_one_line_call_with_a_non_literal_model_refuses_instead_of_reading_past_it() {
    // Pre-patch, the window ran 8 lines past a call that closed on its own line
    // and judged the next unrelated literal (here an allowed id) as the model.
    let tree = Tree::real();
    tree.write(
        "crates/maos-bin/src/main.rs",
        "fn a() {\n    let _ = AnthropicProvider::new(io, url, model, None);\n    let _ = x(\"claude-haiku-4-5-20251001\".into());\n    \
         let _ = OpenAiProvider::new(io, url, \"gpt-4o-mini\".into(), None);\n    \
         let _ = OllamaProvider::new(io, url, \"llama3.1:8b\".into());\n}\n",
    );
    assert!(refusal(&tree)
        .contains("line 2: `AnthropicProvider` constructor has no readable model literal"));
}

#[test]
fn the_api_key_argument_is_never_judged_as_the_model() {
    let tree = Tree::real();
    tree.write(
        "crates/maos-bin/src/main.rs",
        "fn a() {\n    AnthropicProvider::with_api_key(io, url, model, \"sk-secret\".into());\n    \
         OpenAiProvider::new(io, url, \"gpt-4o-mini\".into(), None);\n    \
         OllamaProvider::new(io, url, \"llama3.1:8b\".into());\n}\n",
    );
    let why = refusal(&tree);
    assert!(
        why.contains("no readable model literal") && !why.contains("sk-secret"),
        "{why}"
    );
}

#[test]
fn an_empty_entries_array_is_an_empty_price_book() {
    let tree = Tree::real();
    tree.write("xtask/provider-pricing.toml", "entries = []\n");
    assert!(refusal(&tree).contains("xtask/provider-pricing.toml:1 no `[[entries]]`"));
}

#[test]
fn a_duplicate_on_one_line_names_that_line_not_line_one() {
    let tree = Tree::real();
    tree.write(
        "spirits/zz-dup/manifest.toml",
        "[capabilities.required]\nprovider.complete = [\"anthropic.claude-3-haiku-20240307\", \"anthropic.claude-3-haiku-20240307\"]\n",
    );
    let lines: Vec<usize> = tree.judge().findings.iter().map(|f| f.line).collect();
    assert_eq!(lines, [2, 2]);
}

#[test]
fn a_providers_section_model_id_is_a_pin() {
    // Story 5.5b's `[providers]` schema pins a model by `model_id`; pre-patch it
    // escaped the scan green.
    let tree = Tree::real();
    tree.write(
        "examples/zz-providers/manifest.toml",
        "[providers.primary]\nid = \"openai\"\nmodel_id = \"gpt-4o-mini\"\n\n\
         [[providers.fallback]]\nid = \"anthropic\"\nmodel_id = \"claude-3-haiku-20240307\"\n",
    );
    let report = tree.judge();
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let found = &report.findings[0];
    assert_eq!(
        (found.file.as_str(), found.line),
        ("examples/zz-providers/manifest.toml", 7)
    );
    assert!(found
        .detail
        .contains("RETIRED model id `anthropic.claude-3-haiku-20240307`"));
}

#[test]
fn a_dangling_manifest_link_is_a_refusal_not_a_skip() {
    let tree = Tree::real();
    fs::create_dir_all(tree.root().join("spirits/zz-dangling")).unwrap();
    std::os::unix::fs::symlink(
        tree.root().join("nowhere.toml"),
        tree.root().join("spirits/zz-dangling/manifest.toml"),
    )
    .unwrap();
    assert!(refusal(&tree).contains("cannot read spirits/zz-dangling/manifest.toml"));
}

#[test]
fn the_data_file_refuses_an_empty_provider_map_and_blank_or_repeated_retired_ids() {
    let ok = "[providers.p]\nallowed = [\"a\"]\n[[providers.p.retired]]\nid = \"old\"\nreplacement = \"a\"\n";
    assert!(parse_data(ok).is_ok());
    let twice = format!("{ok}[[providers.p.retired]]\nid = \"old\"\nreplacement = \"a\"\n");
    for (broken, fragment) in [
        ("providers = {}\n".to_string(), "no providers"),
        (ok.replace("id = \"old\"", "id = \" \""), "retired twice"),
        (twice, "retired twice"),
    ] {
        let err = parse_data(&broken)
            .err()
            .unwrap_or_else(|| panic!("accepted: {broken}"));
        assert!(err.contains(fragment), "`{fragment}` not in: {err}");
    }
}
