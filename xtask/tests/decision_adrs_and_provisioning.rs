#![forbid(unsafe_code)]

//! Story 15-5 — decision ADR and provisioning governance.
//!
//! Acceptance is the planted red, not the helper. The real-corpus test makes the
//! five decisions and their operator inventory review-blocking. Coverage is
//! per-contract-clause, not per-helper-line: one isolated planted red per
//! clause (a)–(f), per denominator (workflows, secrets, ADRs, checklist rows),
//! per `require_absent` regression direction, and per artifact guard
//! (STABILITY fence, README flag, Cargo.lock wasmtime pin). Helper clauses
//! without their own vector are refactors pending a red, not proven controls.

use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

const CHECKLIST: &str = "docs/runbooks/provisioning-checklist.md";
const REQUIRED_ADRS: &[(&str, &str)] = &[
    ("060", "17-3b-wasm-third-party-form-with-log-recall"),
    ("061", "17-1-worker-egress-allowlist-and-scoped-credential"),
    ("062", "16-1-daemon-post-surface-and-verb-retarget"),
    ("063", "21-3-durable-tofu-and-self-leaf-rotation"),
    ("064", "15-6-inference-replay-seam"),
];
const REQUIRED_FRONTMATTER: &[&str] = &[
    "Status",
    "Gate",
    "Decided",
    "Accepted-in-PR",
    "Revisits",
    "Supersedes",
];
const LEGAL_STATES: &str =
    "present | absent | waived-by <name> <date> | not-a-provisioning-item (<reason>)";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must have a workspace parent")
        .to_path_buf()
}

fn read(root: &Path, rel: &str, findings: &mut Vec<String>) -> Option<String> {
    match std::fs::read_to_string(root.join(rel)) {
        Ok(value) => Some(value),
        Err(err) => {
            findings.push(format!("cannot read {rel}: {err}"));
            None
        }
    }
}

fn top_level_files(root: &Path, rel: &str, suffix: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
        })
        .collect();
    files.sort();
    files
}
fn workflow_inventory(root: &Path) -> (Vec<PathBuf>, BTreeMap<String, BTreeSet<String>>) {
    let mut workflows = top_level_files(root, ".github/workflows", ".yml");
    for path in top_level_files(root, ".github/workflows", ".yaml") {
        if !workflows.contains(&path) {
            workflows.push(path);
        }
    }
    workflows.sort();
    // GitHub registers both extensions and secret names are case-insensitive;
    // scanning both cases keeps a lowercase or .yaml reference from slipping
    // past the denominator into ungoverned territory.
    let secret_re = Regex::new(r"(?i)secrets\.([A-Za-z_][A-Za-z_0-9]*)").unwrap();
    let mut secrets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in &workflows {
        let Ok(body) = std::fs::read_to_string(path) else {
            continue;
        };
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        for capture in secret_re.captures_iter(&body) {
            let name = capture[1].to_string();
            if !name.eq_ignore_ascii_case("GITHUB_TOKEN") {
                secrets.entry(name).or_default().insert(rel.clone());
            }
        }
    }
    (workflows, secrets)
}

fn adr_files(root: &Path) -> Vec<PathBuf> {
    let adr_re = Regex::new(r"^ADR-[0-9]{3}-.+\.md$").unwrap();
    let mut files: Vec<_> = top_level_files(root, "docs/adr", ".md")
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| adr_re.is_match(name))
        })
        .collect();
    files.sort();
    files
}

fn frontmatter(body: &str) -> Option<BTreeMap<&str, &str>> {
    let mut lines = body.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut values = BTreeMap::new();
    for line in lines {
        if line.trim() == "---" {
            return Some(values);
        }
        if let Some((key, value)) = line.split_once(':') {
            values.insert(key.trim(), value.trim().trim_matches('"'));
        }
    }
    None
}

fn context_section(body: &str) -> &str {
    let Some((_, after)) = body.split_once("## Context") else {
        return "";
    };
    after.split("\n## ").next().unwrap_or(after)
}

fn validate_shape(number: &str, body: &str, findings: &mut Vec<String>) {
    let Some(metadata) = frontmatter(body) else {
        findings.push(format!(
            "ADR-{number} needs YAML frontmatter with keys: {}",
            REQUIRED_FRONTMATTER.join(", ")
        ));
        return;
    };
    for key in REQUIRED_FRONTMATTER {
        if !metadata.contains_key(key) {
            findings.push(format!("ADR-{number} frontmatter is missing `{key}`"));
        }
    }
    if !metadata
        .get("Status")
        .is_some_and(|value| value.starts_with("ACCEPTED"))
    {
        findings.push(format!("ADR-{number} Status must begin `ACCEPTED`"));
    }
    if body.lines().count() < 40 {
        findings.push(format!("ADR-{number} must contain at least 40 lines"));
    }
    for heading in ["## Context", "## Decision"] {
        if !body.lines().any(|line| line.trim() == heading) {
            findings.push(format!("ADR-{number} is missing `{heading}`"));
        }
    }
    let lowercase = body.to_ascii_lowercase();
    for forbidden in ["todo", "tbd", "<placeholder", "to be decided"] {
        if lowercase.contains(forbidden) {
            findings.push(format!(
                "ADR-{number} contains forbidden unresolved marker `{forbidden}`"
            ));
        }
    }
}

fn require_contains(
    number: &str,
    context: &str,
    context_token: &str,
    rel: &str,
    source: &str,
    expected: &str,
    findings: &mut Vec<String>,
) {
    if !context.contains(context_token) {
        findings.push(format!(
            "ADR-{number} Context must name `{context_token}` for anti-rot anchor {rel}"
        ));
    }
    if !source.contains(expected) {
        findings.push(format!(
            "ADR-{number} anti-rot anchor moved: {rel} no longer contains `{expected}`"
        ));
    }
}

fn require_absent(
    number: &str,
    context: &str,
    context_token: &str,
    rel: &str,
    source: &str,
    forbidden: &str,
    findings: &mut Vec<String>,
) {
    if !context.contains(context_token) {
        findings.push(format!(
            "ADR-{number} Context must name `{context_token}` for anti-rot anchor {rel}"
        ));
    }
    if source.contains(forbidden) {
        findings.push(format!(
            "ADR-{number} anti-rot anchor moved: {rel} now contains `{forbidden}`"
        ));
    }
}

fn anchor_source(
    root: &Path,
    number: &str,
    rel: &str,
    findings: &mut Vec<String>,
) -> Option<String> {
    match std::fs::read_to_string(root.join(rel)) {
        Ok(body) => Some(body),
        Err(_) => {
            // A missing anchor file must red, never silently satisfy a
            // `require_absent` clause with an empty string.
            findings.push(format!(
                "ADR-{number} anti-rot anchor file unreadable: `{rel}` is missing or unreadable; a moved anchor must be re-pointed in the ADR, never silently dropped"
            ));
            None
        }
    }
}

fn validate_anchors(root: &Path, number: &str, body: &str, findings: &mut Vec<String>) {
    let context = context_section(body);
    match number {
        "060" => {
            if let Some(manifest) = anchor_source(
                root,
                number,
                "crates/maos-manifest/src/manifest.rs",
                findings,
            ) {
                require_contains(
                    number,
                    context,
                    "class.forms",
                    "crates/maos-manifest/src/manifest.rs",
                    &manifest,
                    "matches!(f.as_str(), \"rust-inproc\" | \"subprocess\")",
                    findings,
                );
            }
            if let Some(admission) = anchor_source(
                root,
                number,
                "crates/maos-registry/src/admission.rs",
                findings,
            ) {
                require_absent(
                    number,
                    context,
                    "admit_spirit",
                    "crates/maos-registry/src/admission.rs",
                    &admission,
                    "class.forms",
                    findings,
                );
            }
        }
        "061" => {
            if let Some(posture) = anchor_source(
                root,
                number,
                "crates/maos-cli/tests/credential_posture_2c.rs",
                findings,
            ) {
                require_contains(
                    number,
                    context,
                    "env_clear",
                    "crates/maos-cli/tests/credential_posture_2c.rs",
                    &posture,
                    "!RUNTIME_SRC.contains(\"env_clear\")",
                    findings,
                );
            }
            if let Some(argv) = anchor_source(
                root,
                number,
                "crates/maos-kernel-core/src/security/sandbox/t3/argv.rs",
                findings,
            ) {
                require_contains(
                    number,
                    context,
                    "--network=none",
                    "crates/maos-kernel-core/src/security/sandbox/t3/argv.rs",
                    &argv,
                    "\"--network=none\".to_string()",
                    findings,
                );
            }
        }
        "062" => {
            if let Some(control) =
                anchor_source(root, number, "crates/maos-control/src/lib.rs", findings)
            {
                for route in [
                    "GET /v1/a2a/rotation-windows",
                    "GET /v1/cohort/peer-versions",
                    "GET /v1/cohort/self-identity",
                    "GET /v1/spirits/",
                ] {
                    require_contains(
                        number,
                        context,
                        "four GET routes",
                        "crates/maos-control/src/lib.rs",
                        &control,
                        route,
                        findings,
                    );
                }
                require_absent(
                    number,
                    context,
                    "405",
                    "crates/maos-control/src/lib.rs",
                    &control,
                    "Method Not Allowed",
                    findings,
                );
            }
            if let Some(dep_test) = anchor_source(
                root,
                number,
                "crates/maos-cli/tests/dep_kernel_core_free_test.rs",
                findings,
            ) {
                require_contains(
                    number,
                    context,
                    "maos-kernel-core",
                    "crates/maos-cli/tests/dep_kernel_core_free_test.rs",
                    &dep_test,
                    "maos-cli MUST NOT depend on maos-kernel-core",
                    findings,
                );
            }
        }
        "063" => {
            if let Some(frame) =
                anchor_source(root, number, "crates/maos-domain/src/frame.rs", findings)
            {
                require_absent(
                    number,
                    context,
                    "RuptureReason",
                    "crates/maos-domain/src/frame.rs",
                    &frame,
                    "CertPostGraceReject",
                    findings,
                );
            }
            if let Some(transport) = anchor_source(
                root,
                number,
                "crates/maos-a2a-tcp/src/transport.rs",
                findings,
            ) {
                require_contains(
                    number,
                    context,
                    "build_server_config",
                    "crates/maos-a2a-tcp/src/transport.rs",
                    &transport,
                    "None, // listen side learns the peer from the cert",
                    findings,
                );
            }
        }
        "064" => {
            if let Some(env_contract) = anchor_source(
                root,
                number,
                "crates/maos-bin/src/env_contract.rs",
                findings,
            ) {
                require_absent(
                    number,
                    context,
                    "MAOS_INFERENCE_MODE",
                    "crates/maos-bin/src/env_contract.rs",
                    &env_contract,
                    "MAOS_INFERENCE_MODE",
                    findings,
                );
            }
            if let Some(worker_spawn) = anchor_source(
                root,
                number,
                "crates/maos-bin/src/worker_spawn.rs",
                findings,
            ) {
                require_contains(
                    number,
                    context,
                    "--replay-llm",
                    "crates/maos-bin/src/worker_spawn.rs",
                    &worker_spawn,
                    "\"--replay-llm\" => live = false",
                    findings,
                );
            }
        }
        _ => unreachable!("only required ADRs have anti-rot anchors"),
    }
}

fn sprint_keys(root: &Path, findings: &mut Vec<String>) -> BTreeSet<String> {
    let Some(body) = read(
        root,
        "_bmad-output/implementation-artifacts/sprint-status.yaml",
        findings,
    ) else {
        return BTreeSet::new();
    };
    let key_re = Regex::new(r"(?m)^  ([0-9]+-[0-9]+[a-z]?-[a-z0-9-]+):").unwrap();
    key_re
        .captures_iter(&body)
        .map(|capture| capture[1].to_string())
        .collect()
}

/// `ADR-060` must not match inside `ADR-0600`: the match is only credited when
/// no ASCII digit follows it.
fn contains_adr_number(haystack: &str, adr: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = haystack[from..].find(adr) {
        let at = from + offset;
        let after = &haystack[at + adr.len()..];
        if after.chars().next().map_or(true, |ch| !ch.is_ascii_digit()) {
            return true;
        }
        from = at + adr.len();
    }
    false
}

fn epic_section_has_backreference(root: &Path, consumer: &str, adr: &str) -> bool {
    for path in top_level_files(root, "_bmad-output/planning-artifacts/epics", ".md") {
        let Ok(body) = std::fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<_> = body.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if line.starts_with("### ") && line.contains(consumer) {
                let end = lines[index + 1..]
                    .iter()
                    .position(|candidate| candidate.starts_with("### "))
                    .map_or(lines.len(), |offset| index + 1 + offset);
                if lines[index..end]
                    .iter()
                    .any(|candidate| contains_adr_number(candidate, adr))
                {
                    return true;
                }
            }
        }
    }
    false
}

fn validate_consumers(
    root: &Path,
    number: &str,
    expected_consumer: &str,
    body: &str,
    keys: &BTreeSet<String>,
    findings: &mut Vec<String>,
) {
    if !body.contains(expected_consumer) {
        findings.push(format!(
            "ADR-{number} must name consuming story `{expected_consumer}`"
        ));
    }
    if !keys.contains(expected_consumer) {
        findings.push(format!(
            "ADR-{number} names `{expected_consumer}`, but sprint-status.yaml does not declare it"
        ));
    }
    let adr = format!("ADR-{number}");
    if !epic_section_has_backreference(root, expected_consumer, &adr) {
        findings.push(format!(
            "consumer `{expected_consumer}` must cite {adr} back in its own epic section"
        ));
    }
}

fn validate_index(root: &Path, files: &[PathBuf], findings: &mut Vec<String>) {
    let Some(index) = read(root, "docs/adr/index.md", findings) else {
        return;
    };
    let link_re = Regex::new(r"^\[ADR-([0-9]{3})\]\(([^)]+)\)$").unwrap();
    let mut indexed = BTreeSet::new();
    let mut previous = None;
    for (line_number, line) in index.lines().enumerate() {
        if !line.starts_with("| [ADR-") {
            continue;
        }
        if !line.trim_end().ends_with('|') {
            findings.push(format!(
                "docs/adr/index.md:{} must end its four-cell table row with `|`",
                line_number + 1
            ));
            continue;
        }
        let cells: Vec<_> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.len() != 4 {
            findings.push(format!(
                "docs/adr/index.md:{} must be a four-cell table row with a trailing `|`",
                line_number + 1
            ));
            continue;
        }
        let Some(capture) = link_re.captures(cells[0]) else {
            findings.push(format!(
                "docs/adr/index.md:{} has a malformed ADR link",
                line_number + 1
            ));
            continue;
        };
        let number: u16 = capture[1].parse().unwrap();
        if previous.is_some_and(|prior| number <= prior) {
            findings.push(format!(
                "docs/adr/index.md:{} is not strictly ascending at ADR-{number:03}",
                line_number + 1
            ));
        }
        previous = Some(number);
        let target = capture[2].to_string();
        if !root.join("docs/adr").join(&target).is_file() {
            findings.push(format!(
                "docs/adr/index.md:{} points to missing docs/adr/{target}",
                line_number + 1
            ));
        }
        if !indexed.insert(target.clone()) {
            findings.push(format!("docs/adr/index.md duplicates `{target}`"));
        }
    }
    let corpus: BTreeSet<_> = files
        .iter()
        .filter_map(|path| path.file_name()?.to_str().map(str::to_string))
        .collect();
    for missing in corpus.difference(&indexed) {
        findings.push(format!(
            "docs/adr/{missing} has no docs/adr/index.md row; add a four-cell row in numeric order"
        ));
    }
    for stale in indexed.difference(&corpus) {
        findings.push(format!(
            "docs/adr/index.md names `{stale}` but no matching ADR file exists"
        ));
    }
}

fn legal_state(value: &str) -> bool {
    value == "present"
        || value == "absent"
        || Regex::new(r"^waived-by \S+ \d{4}-\d{2}-\d{2}$")
            .unwrap()
            .is_match(value)
        || (value.starts_with("not-a-provisioning-item (") && value.ends_with(')'))
}

fn checklist_rows(root: &Path, findings: &mut Vec<String>) -> BTreeMap<String, String> {
    let Some(body) = read(root, CHECKLIST, findings) else {
        return BTreeMap::new();
    };
    let mut rows = BTreeMap::new();
    let mut in_checklist = false;
    for (line_number, line) in body.lines().enumerate() {
        if line.trim() == "## Checklist" {
            in_checklist = true;
            continue;
        }
        if in_checklist && line.starts_with("## ") {
            break;
        }
        if !in_checklist || !line.starts_with('|') {
            continue;
        }
        let cells: Vec<_> = line
            .trim()
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect();
        if cells.is_empty() {
            continue;
        }
        // Skip by row shape, never by substring: a data cell containing
        // "---" or " Item " must not smuggle its row out of governance.
        if cells.iter().all(|cell| {
            !cell.is_empty() && cell.contains('-') && cell.chars().all(|ch| ch == '-' || ch == ':')
        }) {
            continue; // table separator row
        }
        if cells[0] == "Item" {
            continue; // header row
        }
        if cells.len() < 2 {
            findings.push(format!(
                "{CHECKLIST}:{} is not a parseable checklist row",
                line_number + 1
            ));
            continue;
        }
        let item = cells[0].trim_matches('`').to_string();
        let state = cells.last().unwrap().trim_matches('`').to_string();
        if !legal_state(&state) {
            findings.push(format!(
                "{CHECKLIST}:{} item `{item}` has state `{state}`; legal states: {LEGAL_STATES}",
                line_number + 1
            ));
        }
        if rows.contains_key(&item) {
            findings.push(format!(
                "{CHECKLIST}:{} duplicates item `{item}`; one row per item — the last row must not silently win",
                line_number + 1
            ));
        }
        rows.insert(item, state);
    }
    rows
}

fn validate(root: &Path) -> Vec<String> {
    let mut findings = Vec::new();
    let (workflows, secrets) = workflow_inventory(root);
    let adrs = adr_files(root);

    // Denominators first: no universal assertion below may pass over an empty or
    // suspiciously incomplete corpus.
    if workflows.len() < 11 {
        findings.push(format!(
            "workflow denominator: found {} .github/workflows/*.yml|*.yaml files, need >=11; check the workspace-root path before weakening this threshold",
            workflows.len()
        ));
    }
    if secrets.len() < 7 {
        findings.push(format!(
            "secret denominator: found {} distinct non-GITHUB_TOKEN secrets.<NAME> references, need >=7; scan .github/workflows/*.yml and add every result to {CHECKLIST}",
            secrets.len()
        ));
    }
    if adrs.len() < 44 {
        findings.push(format!(
            "ADR denominator: found {} docs/adr/ADR-*.md files, need >=44; check the workspace-root path before accepting a vacuous bijection",
            adrs.len()
        ));
    }

    let keys = sprint_keys(root, &mut findings);
    for (number, consumer) in REQUIRED_ADRS {
        let prefix = format!("ADR-{number}-");
        let Some(path) = adrs.iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        }) else {
            findings.push(format!(
                "missing docs/adr/ADR-{number}-*.md; author the accepted decision ADR"
            ));
            continue;
        };
        let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy();
        let Some(body) = read(root, &rel, &mut findings) else {
            continue;
        };
        validate_shape(number, &body, &mut findings);
        validate_anchors(root, number, &body, &mut findings);
        validate_consumers(root, number, consumer, &body, &keys, &mut findings);
    }

    validate_index(root, &adrs, &mut findings);
    let checklist = checklist_rows(root, &mut findings);
    for (secret, source_workflows) in secrets {
        if !checklist.contains_key(&secret) {
            findings.push(format!(
                "workflow secret `{secret}` from {} has no `{secret}` row in {CHECKLIST}; add it with one legal state: {LEGAL_STATES}",
                source_workflows.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }

    // (g) The WASM-off-until-Hold-2 statement lives inside the preserved
    // export fence; every other region of this generated file is byte-locked.
    if let Some(stability) = read(root, "STABILITY.md", &mut findings) {
        let open = "<!-- PRESERVED:export -->";
        let close = "<!-- END PRESERVED:export -->";
        let fence = match (stability.find(open), stability.find(close)) {
            (Some(start), Some(end)) if end > start => Some(&stability[start + open.len()..end]),
            _ => None,
        };
        match fence {
            Some(fence) if fence.contains("wasm-host") && fence.contains("5D002") => {}
            Some(_) => findings.push(
                "STABILITY.md PRESERVED:export fence must carry the WASM-off-until-Hold-2 statement naming `wasm-host` and the 5D002.c.1 precondition; keep the statement inside the fence — regenerate with `cargo run -p xtask -- stability-matrix` never preserves edits outside it"
                    .to_string(),
            ),
            None => findings.push(
                "STABILITY.md must carry both `<!-- PRESERVED:export -->` fence markers; the WASM-off statement belongs between them"
                    .to_string(),
            ),
        }
    }

    // (h) Self-builders get the opt-in build command.
    if let Some(readme) = read(root, "README.md", &mut findings) {
        if !readme.contains("--features wasm-host") {
            findings.push(
                "README.md build documentation must include the self-builder `--features wasm-host` command (the WASM host is opt-in and off by default)"
                    .to_string(),
            );
        }
    }

    // (i) The coordinated wasmtime security upgrade stays in the lockfile.
    if let Some(lock) = read(root, "Cargo.lock", &mut findings) {
        let mut saw_wasmtime = false;
        for block in lock.split("[[package]]").skip(1) {
            let Some(name) = block.lines().find_map(|line| {
                line.strip_prefix("name = \"")
                    .and_then(|rest| rest.strip_suffix('"'))
            }) else {
                continue;
            };
            if !name.starts_with("wasmtime") {
                continue;
            }
            saw_wasmtime = true;
            if let Some(version) = block.lines().find_map(|line| {
                line.strip_prefix("version = \"")
                    .and_then(|rest| rest.strip_suffix('"'))
            }) {
                if version == "46.0.2" {
                    findings.push(format!(
                        "Cargo.lock pins wasmtime crate `{name}` at 46.0.2, the RUSTSEC-2026-0268/0269-affected version Story 15-5 cleared; re-run the coordinated `cargo update -p wasmtime -p wasmtime-wasi --precise <fixed>` — never a deny.toml exception"
                    ));
                }
            }
        }
        if !saw_wasmtime {
            findings.push(
                "Cargo.lock no longer pins any wasmtime crate; the wasm-host feature resolves against this lockfile — an accidental removal must be a reviewable red, not a silent pass"
                    .to_string(),
            );
        }
    }

    // (j) Checklist inventory denominator: the secret rows alone are not the list.
    if checklist.len() < 20 {
        findings.push(format!(
            "checklist denominator: found {} data rows in {CHECKLIST}, need >=20 (the seven secret rows plus the non-secret operator inventory); dropping ungoverned rows must red",
            checklist.len()
        ));
    }
    findings
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(body.as_bytes()).unwrap();
}

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn complete() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for index in 0..11 {
            let body = if index == 0 {
                (1..=7)
                    .map(|number| format!("token: ${{{{ secrets.SECRET_{number} }}}}\n"))
                    .collect()
            } else {
                format!("name: workflow-{index}\n")
            };
            write_file(
                root,
                &format!(".github/workflows/workflow-{index}.yml"),
                &body,
            );
        }
        write_file(
            root,
            "_bmad-output/implementation-artifacts/sprint-status.yaml",
            &format!(
                "last_updated: 2026-09-08\ndevelopment_status:\n{}",
                REQUIRED_ADRS
                    .iter()
                    .map(|(_, consumer)| format!("  {consumer}: backlog\n"))
                    .collect::<String>()
            ),
        );

        let mut index = String::from(
            "# Architecture Decision Records\n\n| ADR | Title | Status | Gate |\n|-----|-------|--------|------|\n",
        );
        let mut numbers: Vec<u16> = (1..=39).collect();
        numbers.extend(60..=64);
        for number in numbers {
            let name = format!("ADR-{number:03}-fixture-decision.md");
            let required = REQUIRED_ADRS
                .iter()
                .find(|(candidate, _)| *candidate == format!("{number:03}"));
            let consumer = required.map_or("99-1-fixture-consumer", |(_, consumer)| *consumer);
            let context = match number {
                60 => "class.forms accepts rust-inproc and subprocess today; admit_spirit does not read it.",
                61 => "env_clear is forbidden before the proxy; --network=none is the T3 model.",
                62 => "The four GET routes have no 405 today and maos-kernel-core remains excluded.",
                63 => "RuptureReason lacks the variant and build_server_config uses a flat lookup.",
                64 => "MAOS_INFERENCE_MODE is absent and --replay-llm remains accepted.",
                _ => "Fixture context anchored to no governed decision.",
            };
            let filler = (0..35)
                .map(|line| format!("Fixture rationale line {line}.\n"))
                .collect::<String>();
            let adr = format!(
                "---\nStatus: ACCEPTED — fixture\nGate: fixture\nDecided: 2026-09-08\nAccepted-in-PR: pending\nRevisits: fixture\nSupersedes: nothing\n---\n\n# ADR-{number:03}\n\n## Context\n\n{context}\n\n{filler}\n## Decision\n\nThe fixture decision is binding.\n\n## Consumers\n\n- `{consumer}`\n"
            );
            write_file(root, &format!("docs/adr/{name}"), &adr);
            index.push_str(&format!(
                "| [ADR-{number:03}]({name}) | Fixture {number:03} | accepted | fixture |\n"
            ));
        }
        write_file(root, "docs/adr/index.md", &index);

        let mut epic = String::from("# Fixture epic\n\n");
        for (number, consumer) in REQUIRED_ADRS {
            epic.push_str(&format!(
                "### {consumer} — fixture\n\nBuilds the decision in ADR-{number}.\n\n"
            ));
        }
        write_file(
            root,
            "_bmad-output/planning-artifacts/epics/epic-fixture.md",
            &epic,
        );

        write_file(
            root,
            "crates/maos-manifest/src/manifest.rs",
            "matches!(f.as_str(), \"rust-inproc\" | \"subprocess\")\n",
        );
        write_file(
            root,
            "crates/maos-registry/src/admission.rs",
            "pub fn admit_spirit() {}\n",
        );
        write_file(
            root,
            "crates/maos-cli/tests/credential_posture_2c.rs",
            "assert!(!RUNTIME_SRC.contains(\"env_clear\"));\n",
        );
        write_file(
            root,
            "crates/maos-kernel-core/src/security/sandbox/t3/argv.rs",
            "\"--network=none\".to_string()\n",
        );
        write_file(
            root,
            "crates/maos-control/src/lib.rs",
            "GET /v1/a2a/rotation-windows\nGET /v1/cohort/peer-versions\nGET /v1/cohort/self-identity\nGET /v1/spirits/\n",
        );
        write_file(
            root,
            "crates/maos-cli/tests/dep_kernel_core_free_test.rs",
            "maos-cli MUST NOT depend on maos-kernel-core\n",
        );
        write_file(
            root,
            "crates/maos-domain/src/frame.rs",
            "pub enum RuptureReason { PeerIdentityUnverified }\n",
        );
        write_file(
            root,
            "crates/maos-a2a-tcp/src/transport.rs",
            "pub fn build_server_config() { None, // listen side learns the peer from the cert\n}\n",
        );
        write_file(
            root,
            "crates/maos-bin/src/env_contract.rs",
            "MAOS_REPLAY_CASSETTE\n",
        );
        write_file(
            root,
            "crates/maos-bin/src/worker_spawn.rs",
            "\"--replay-llm\" => live = false\n",
        );

        write_file(
            root,
            "STABILITY.md",
            "<!-- GENERATED FILE -->\n\n<!-- PRESERVED:export -->\nThe WASM host stays outside the default closure; `wasm-host` is the 5D002.c.1 opt-in precondition.\n<!-- END PRESERVED:export -->\n",
        );
        write_file(
            root,
            "README.md",
            "# Fixture\n\nBuild the engine explicitly:\n\ncargo build -p maos-bin --release --features wasm-host\n",
        );
        write_file(
            root,
            "Cargo.lock",
            "[[package]]\nname = \"wasmtime\"\nversion = \"46.0.3\"\n",
        );

        let mut checklist = String::from(
            "# Provisioning checklist\n\n## Checklist\n\n| Item | Consumer | Unset behaviour | Owner | Evidence | Notes | State |\n|---|---|---|---|---|---|---|\n",
        );
        for number in 1..=7 {
            checklist.push_str(&format!(
                "| `SECRET_{number}` | workflow-0.yml | hard fail | ops-fixture | fixture | fixture | absent |\n"
            ));
        }
        for number in 1..=13 {
            checklist.push_str(&format!(
                "| `OPS_FIXTURE_{number}` | workflow-0.yml | hard fail | ops-fixture | fixture | fixture | absent |\n"
            ));
        }
        write_file(root, CHECKLIST, &checklist);
        Self { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn rewrite(&self, rel: &str, f: impl FnOnce(String) -> String) {
        let path = self.root().join(rel);
        let current = std::fs::read_to_string(&path).unwrap();
        write_file(self.root(), rel, &f(current));
    }
}

fn assert_red_contains(fx: &Fixture, needle: &str) {
    let findings = validate(fx.root());
    assert!(
        findings.iter().any(|finding| finding.contains(needle)),
        "fixture must red with `{needle}`; findings:\n{}",
        findings.join("\n")
    );
}

#[test]
fn complete_fixture_is_green_before_any_red_is_credited() {
    let fx = Fixture::complete();
    let findings = validate(fx.root());
    assert!(
        findings.is_empty(),
        "complete fixture must be green:\n{}",
        findings.join("\n")
    );
}

#[test]
fn every_clause_and_denominator_has_a_planted_red() {
    // Denominator: empty workflow glob.
    let fx = Fixture::complete();
    std::fs::remove_dir_all(fx.root().join(".github/workflows")).unwrap();
    assert_red_contains(&fx, "workflow denominator");

    // Denominator: fewer than seven distinct operator secrets.
    let fx = Fixture::complete();
    fx.rewrite(".github/workflows/workflow-0.yml", |_| {
        "name: no-secrets\n".into()
    });
    assert_red_contains(&fx, "secret denominator");

    // Denominator: fewer than 44 ADR files.
    let fx = Fixture::complete();
    std::fs::remove_file(fx.root().join("docs/adr/ADR-001-fixture-decision.md")).unwrap();
    assert_red_contains(&fx, "ADR denominator");

    // (a) A required ADR is absent.
    let fx = Fixture::complete();
    std::fs::remove_file(fx.root().join("docs/adr/ADR-060-fixture-decision.md")).unwrap();
    assert_red_contains(&fx, "missing docs/adr/ADR-060-*.md");

    // (b) A required frontmatter key is absent.
    let fx = Fixture::complete();
    fx.rewrite("docs/adr/ADR-061-fixture-decision.md", |body| {
        body.replace("Gate: fixture\n", "")
    });
    assert_red_contains(&fx, "frontmatter is missing `Gate`");

    // (c) Shape remains valid but the source fact moved.
    let fx = Fixture::complete();
    fx.rewrite("crates/maos-manifest/src/manifest.rs", |_| {
        "matches!(f.as_str(), \"rust-inproc\")\n".into()
    });
    assert_red_contains(&fx, "anti-rot anchor moved");

    // (d) ADR names the consumer, but the epic section no longer cites back.
    let fx = Fixture::complete();
    fx.rewrite(
        "_bmad-output/planning-artifacts/epics/epic-fixture.md",
        |body| body.replace("ADR-064", "the replay decision"),
    );
    assert_red_contains(&fx, "must cite ADR-064 back");

    // (e) An ADR file has no index row.
    let fx = Fixture::complete();
    fx.rewrite("docs/adr/index.md", |body| {
        body.lines()
            .filter(|line| !line.contains("ADR-060"))
            .collect::<Vec<_>>()
            .join("\n")
    });
    assert_red_contains(&fx, "has no docs/adr/index.md row");

    // (f) A workflow secret has no checklist row; the message names the fix.
    let fx = Fixture::complete();
    fx.rewrite(".github/workflows/workflow-1.yml", |body| {
        format!("{body}token: ${{{{ secrets.NEW_OPERATOR_SECRET }}}}\n")
    });
    assert_red_contains(&fx, "NEW_OPERATOR_SECRET");
    assert_red_contains(&fx, CHECKLIST);
    assert_red_contains(&fx, LEGAL_STATES);
    // (g) The STABILITY fence keeps its markers but loses the statement.
    let fx = Fixture::complete();
    fx.rewrite("STABILITY.md", |body| {
        body.replace(
            "The WASM host stays outside the default closure; `wasm-host` is the 5D002.c.1 opt-in precondition.\n",
            "",
        )
    });
    assert_red_contains(&fx, "PRESERVED:export fence must carry");

    // (h) README drops the self-builder command.
    let fx = Fixture::complete();
    fx.rewrite("README.md", |body| body.replace("--features wasm-host", ""));
    assert_red_contains(&fx, "must include the self-builder");

    // (i) Cargo.lock regresses to the advisory-affected wasmtime version.
    let fx = Fixture::complete();
    fx.rewrite("Cargo.lock", |body| body.replace("46.0.3", "46.0.2"));
    assert_red_contains(&fx, "RUSTSEC-2026-0268");

    // (j) The checklist is reduced to its secret rows.
    let fx = Fixture::complete();
    fx.rewrite(CHECKLIST, |body| {
        body.lines()
            .filter(|line| !line.contains("OPS_FIXTURE_"))
            .collect::<Vec<_>>()
            .join("\n")
    });
    assert_red_contains(&fx, "checklist denominator");

    // require_absent direction: the forbidden control reappears in its anchor.
    let fx = Fixture::complete();
    fx.rewrite("crates/maos-registry/src/admission.rs", |body| {
        format!("{body}let _ = \"class.forms\";\n")
    });
    assert_red_contains(&fx, "now contains `class.forms`");

    // An anchor file that disappears must red, never pass silently.
    let fx = Fixture::complete();
    std::fs::remove_file(fx.root().join("crates/maos-registry/src/admission.rs")).unwrap();
    assert_red_contains(
        &fx,
        "anchor file unreadable: `crates/maos-registry/src/admission.rs`",
    );
}

#[test]
fn real_repository_satisfies_decision_adrs_and_provisioning_contract() {
    let findings = validate(&workspace_root());
    assert!(
        findings.is_empty(),
        "Story 15-5 decision/provisioning contract failed:\n{}",
        findings.join("\n")
    );
}
