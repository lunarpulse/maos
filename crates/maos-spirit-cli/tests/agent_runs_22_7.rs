//! Story 22-7 AC3 (D-22-7-11 A) — the recorded agent runs are labelled, digest-bound and scrubbed, and
//! the success count the release states is the records' own.
//!
//! `docs/agent-runs/22-7/README.md` carries one machine-read line: `status=pending` (the three runs are an
//! operator-lane step and none is committed yet) or `status=recorded k=<n>`. Pending admits no run
//! directory and requires `RELEASE-HOLDS.md` K1 to say the runs have not been recorded; recorded requires
//! exactly `run-1`, `run-2`, `run-3`, each a `record.toml` plus its transcript. The checker takes
//! its directory as an argument so the planted-red cases below run it over synthetic trees.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

const MAX_TRANSCRIPT_BYTES: u64 = 2 * 1024 * 1024;
/// The five binaries a run's container holds (D-22-7-08 A).
const BINARIES: [&str; 5] = [
    "maos",
    "maosctl",
    "maos-registry-server",
    "maos-spirit",
    "maos-wasm-runner",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    author_kind: String,
    harness: String,
    model: String,
    kit_commit: String,
    start: String,
    end: String,
    outcome: String,
    ask_result: String,
    transcript: String,
    transcript_sha256: String,
    binaries_sha256: BTreeMap<String, String>,
    /// 64-hex digests the transcript may legitimately contain (a `sha256sum` the agent printed).
    #[serde(default)]
    public_digests: Vec<String>,
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Secret shapes a committed transcript must not carry. Extend with the harnesses actually used.
fn secret_in(transcript: &str, record: &Record) -> Option<String> {
    for needle in [
        "sk-",
        "ghp_",
        "github_pat_",
        "AKIA",
        "-----BEGIN",
        "xoxb-",
        "AIza",
    ] {
        if transcript.contains(needle) {
            return Some(format!("contains `{needle}`"));
        }
    }
    for line in transcript.lines() {
        for name in ["API_KEY", "_TOKEN", "_SECRET", "_PASSWORD"] {
            if let Some(at) = line.find(&format!("{name}=")) {
                let value = line[at + name.len() + 1..].trim_start_matches(['"', '\'']);
                if !value.is_empty() && !value.starts_with("[REDACTED]") && !value.starts_with('$')
                {
                    return Some(format!("an unredacted `{name}=` value: {line}"));
                }
            }
        }
    }
    // A bare 64-hex run is a key seed unless it is a digest the record names.
    let mut run = String::new();
    let flush = |run: &mut String| {
        let found = run.len() >= 64
            && !record
                .public_digests
                .iter()
                .any(|d| run.contains(d.as_str()));
        let digest_of_binary = record
            .binaries_sha256
            .values()
            .any(|d| run.contains(d.as_str()));
        let hit = found && !digest_of_binary;
        let text = std::mem::take(run);
        hit.then_some(text)
    };
    for ch in transcript.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_hexdigit() {
            run.push(ch);
        } else if let Some(hit) = flush(&mut run) {
            return Some(format!("a 64-hex run that is not a named digest: {hit}"));
        }
    }
    None
}

fn check_record(dir: &Path) -> Result<(Record, bool), String> {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let text = std::fs::read_to_string(dir.join("record.toml"))
        .map_err(|e| format!("{name}: record.toml: {e}"))?;
    let record: Record = toml::from_str(&text).map_err(|e| format!("{name}: record.toml: {e}"))?;
    if record.author_kind != "agent" {
        return Err(format!(
            "{name}: author_kind must be \"agent\", got {:?}",
            record.author_kind
        ));
    }
    for (field, value) in [
        ("harness", &record.harness),
        ("model", &record.model),
        ("start", &record.start),
        ("end", &record.end),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name}: {field} is empty"));
        }
    }
    if !is_hex(&record.kit_commit, 40) {
        return Err(format!(
            "{name}: kit_commit must be a 40-hex commit, got {:?}",
            record.kit_commit
        ));
    }
    for binary in BINARIES {
        let digest = record.binaries_sha256.get(binary);
        if !digest.is_some_and(|d| is_hex(d, 64)) {
            return Err(format!("{name}: binaries_sha256.{binary} must be a sha256"));
        }
    }
    let succeeded = match record.outcome.as_str() {
        "succeeded" => true,
        "failed" => false,
        other => {
            return Err(format!(
                "{name}: outcome must be succeeded or failed, got {other:?}"
            ))
        }
    };
    if succeeded {
        let task: serde_json::Value = serde_json::from_str(&record.ask_result).map_err(|e| {
            format!("{name}: a succeeded run carries the final `maosctl ask --wait` JSON: {e}")
        })?;
        if task["state"] != "completed" || task["result"].as_str().is_none_or(str::is_empty) {
            return Err(format!(
                "{name}: ask_result is not a completed task with a result: {task}"
            ));
        }
    }
    let transcript = dir.join(&record.transcript);
    if record.transcript.contains('/') || record.transcript.contains("..") {
        return Err(format!(
            "{name}: transcript must name a file in the run directory"
        ));
    }
    let bytes = std::fs::read(&transcript).map_err(|e| format!("{name}: transcript: {e}"))?;
    if bytes.len() as u64 > MAX_TRANSCRIPT_BYTES {
        return Err(format!(
            "{name}: transcript is {} bytes (max {MAX_TRANSCRIPT_BYTES})",
            bytes.len()
        ));
    }
    let digest = hex::encode(Sha256::digest(&bytes));
    if digest != record.transcript_sha256 {
        return Err(format!(
            "{name}: transcript sha256 {digest} does not match the record's {}",
            record.transcript_sha256
        ));
    }
    let transcript_text = String::from_utf8_lossy(&bytes);
    if let Some(found) = secret_in(&transcript_text, &record) {
        return Err(format!("{name}: the transcript {found}"));
    }
    Ok((record, succeeded))
}

/// The checker: `runs` is `docs/agent-runs/22-7`, `holds` the text of `RELEASE-HOLDS.md`.
fn check_runs(runs: &Path, holds: &str) -> Result<usize, String> {
    let readme =
        std::fs::read_to_string(runs.join("README.md")).map_err(|e| format!("README.md: {e}"))?;
    let status = readme
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("<!-- agent-runs-22-7:")?
                .strip_suffix("-->")
        })
        .map(str::trim)
        .ok_or("README.md has no `<!-- agent-runs-22-7: status=... -->` line")?;
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(runs)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let names: Vec<String> = dirs
        .iter()
        .map(|d| d.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    if !holds.contains("| K1 | **k of 3 agent runs succeeded with the kit.**") {
        return Err("RELEASE-HOLDS.md lost the K1 row".into());
    }
    if status == "status=pending" {
        if !names.is_empty() {
            return Err(format!(
                "status=pending but run directories exist: {names:?}"
            ));
        }
        if !holds.contains("have not been recorded yet") {
            return Err(
                "status=pending but RELEASE-HOLDS.md K1 does not say the runs are unrecorded"
                    .into(),
            );
        }
        return Ok(0);
    }
    let stated: usize = status
        .strip_prefix("status=recorded k=")
        .and_then(|k| k.parse().ok())
        .ok_or_else(|| format!("unreadable status line {status:?}"))?;
    if names != ["run-1", "run-2", "run-3"] {
        return Err(format!(
            "expected exactly run-1, run-2, run-3, found {names:?}"
        ));
    }
    let mut k = 0;
    let mut harnesses = std::collections::BTreeSet::new();
    for dir in &dirs {
        let (record, succeeded) = check_record(dir)?;
        k += usize::from(succeeded);
        harnesses.insert(record.harness.to_lowercase());
    }
    if harnesses.len() < 2 {
        return Err(format!(
            "the runs span {} harness famil(ies); D-22-7-08 A needs at least two",
            harnesses.len()
        ));
    }
    if k != stated {
        return Err(format!("README.md says k={stated}, the records say {k}"));
    }
    if !holds.contains(&format!("{k} of 3 agent runs succeeded"))
        || holds.contains("have not been recorded yet")
    {
        return Err(format!("RELEASE-HOLDS.md K1 must state \"{k} of 3 agent runs succeeded\" and drop the pending note"));
    }
    Ok(k)
}

#[test]
fn three_agent_run_records_are_labelled_and_digest_bound() {
    let holds = std::fs::read_to_string(repo().join("RELEASE-HOLDS.md")).unwrap();
    check_runs(&repo().join("docs/agent-runs/22-7"), &holds).unwrap();
}

// ---- planted reds over a synthetic tree -------------------------------------------------------

const HOLDS_RECORDED: &str =
    "| K1 | **k of 3 agent runs succeeded with the kit.** 2 of 3 agent runs succeeded";

fn synthetic(harnesses: [&str; 3]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("README.md"),
        "<!-- agent-runs-22-7: status=recorded k=2 -->\n",
    )
    .unwrap();
    for (i, harness) in harnesses.iter().enumerate() {
        let run = dir.path().join(format!("run-{}", i + 1));
        std::fs::create_dir(&run).unwrap();
        let transcript = format!("agent session {i}\nran maosctl ask --wait\n");
        std::fs::write(run.join("transcript.txt"), &transcript).unwrap();
        let ok = i < 2;
        let binaries: String = BINARIES
            .iter()
            .map(|b| format!("{b} = \"{}\"\n", "ab".repeat(32)))
            .collect();
        let ask = if ok {
            r#"{"task_id":"01M","state":"completed","result":"sev1 | rule: x"}"#
        } else {
            ""
        };
        std::fs::write(
            run.join("record.toml"),
            format!(
                "author_kind = \"agent\"\nharness = \"{harness}\"\nmodel = \"m\"\nkit_commit = \"{}\"\n\
                 start = \"2026-11-24T10:00:00Z\"\nend = \"2026-11-24T10:30:00Z\"\noutcome = \"{}\"\n\
                 ask_result = '{ask}'\ntranscript = \"transcript.txt\"\ntranscript_sha256 = \"{}\"\n\n[binaries_sha256]\n{binaries}",
                "0123456789".repeat(4),
                if ok { "succeeded" } else { "failed" },
                hex::encode(Sha256::digest(transcript.as_bytes())),
            ),
        )
        .unwrap();
    }
    dir
}

fn holds_with_k() -> String {
    HOLDS_RECORDED.to_owned()
}

#[test]
fn a_synthetic_recorded_tree_passes() {
    let dir = synthetic(["claude-code", "codex-cli", "claude-code"]);
    assert_eq!(check_runs(dir.path(), &holds_with_k()), Ok(2));
}

#[test]
fn one_changed_transcript_byte_fails_the_digest_check() {
    let dir = synthetic(["claude-code", "codex-cli", "claude-code"]);
    let path = dir.path().join("run-2/transcript.txt");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&path, bytes).unwrap();
    let error = check_runs(dir.path(), &holds_with_k()).unwrap_err();
    assert!(error.contains("does not match the record's"), "{error}");
}

#[test]
fn a_human_author_kind_fails_the_label_check() {
    let dir = synthetic(["claude-code", "codex-cli", "claude-code"]);
    let path = dir.path().join("run-3/record.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap()
        .replace("author_kind = \"agent\"", "author_kind = \"human\"");
    std::fs::write(&path, text).unwrap();
    let error = check_runs(dir.path(), &holds_with_k()).unwrap_err();
    assert!(error.contains("author_kind must be \"agent\""), "{error}");
}

#[test]
fn a_secret_in_a_transcript_fails_the_scrub_check() {
    let dir = synthetic(["claude-code", "codex-cli", "claude-code"]);
    let run = dir.path().join("run-1");
    let transcript = format!("export OPENAI_API_KEY=sk-abc\n{}\n", "ab".repeat(32));
    std::fs::write(run.join("transcript.txt"), &transcript).unwrap();
    let record = std::fs::read_to_string(run.join("record.toml")).unwrap();
    let old = record
        .lines()
        .find(|l| l.starts_with("transcript_sha256"))
        .unwrap()
        .to_owned();
    let new = format!(
        "transcript_sha256 = \"{}\"",
        hex::encode(Sha256::digest(transcript.as_bytes()))
    );
    std::fs::write(run.join("record.toml"), record.replace(&old, &new)).unwrap();
    let error = check_runs(dir.path(), &holds_with_k()).unwrap_err();
    assert!(error.contains("the transcript"), "{error}");
}

#[test]
fn one_harness_family_is_not_enough_and_k_must_match_the_readme() {
    let dir = synthetic(["claude-code", "claude-code", "Claude-Code"]);
    let error = check_runs(dir.path(), &holds_with_k()).unwrap_err();
    assert!(error.contains("at least two"), "{error}");

    let dir = synthetic(["claude-code", "codex-cli", "claude-code"]);
    std::fs::write(
        dir.path().join("README.md"),
        "<!-- agent-runs-22-7: status=recorded k=3 -->\n",
    )
    .unwrap();
    let error = check_runs(dir.path(), &holds_with_k()).unwrap_err();
    assert!(error.contains("k=3"), "{error}");
}

#[test]
fn a_pending_tree_admits_no_run_directory() {
    let dir = synthetic(["claude-code", "codex-cli", "claude-code"]);
    std::fs::write(
        dir.path().join("README.md"),
        "<!-- agent-runs-22-7: status=pending -->\n",
    )
    .unwrap();
    let pending = "| K1 | **k of 3 agent runs succeeded with the kit.** have not been recorded yet";
    let error = check_runs(dir.path(), pending).unwrap_err();
    assert!(error.contains("run directories exist"), "{error}");
}
