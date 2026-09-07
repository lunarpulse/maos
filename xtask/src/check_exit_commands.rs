#![forbid(unsafe_code)]

//! Story 15-3 AC5 — `check-exit-commands`: confidence rule 1 made mechanical.
//!
//! Rule 1 says *"exit verbs exist at HEAD or are owed by a named, open story."*
//! Before this gate that rule was enforced by nobody: `grep -rn
//! "check-exit-commands" xtask .github` was empty, so every epic's hermetic exit
//! block was prose a reviewer had to read.
//!
//! # Shape
//!
//! [`audit`] is PURE over an explicit [`Corpus`] + [`Surfaces`], and [`run`]
//! does the I/O. That split is the reason `xtask/tests/` can drive the REAL
//! logic (F11); `xtask/tests/decision_register_gate.rs` records why a vector
//! exercising a parallel copy of the parser proves nothing about the gate CI
//! runs.
//!
//! # The five things that make it a control rather than a receipt
//!
//! 1. **Positive roster (F10).** Every non-`done` `epic-N` key must have
//!    EXACTLY ONE file carrying the exit-block marker. Zero or two is a
//!    finding. A glob-and-skip corpus passes silently the day a real epic loses
//!    its block, and `epic-15-*` alone matches three files (two preflights).
//! 2. **Three buckets, never a skip (F7).** A token is `Resolvable`,
//!    `DeclaredNonCommand` (a CLOSED list), or `Unrecognised` — and
//!    `Unrecognised` is a FINDING. An open `if !starts_with("maos") { continue }`
//!    means one typo in a prefix constant silently empties the corpus.
//! 3. **Command substitutions are matched BEFORE separator splitting.**
//!    `MAOS_SPIRIT_SIGNING_KEY=$(od … | tr …) maos-spirit publish …` contains a
//!    pipe inside `$( )`; splitting first re-heads the command to `tr`.
//! 4. **Resolve-or-be-named, strengthened (F8 + F14).** An unresolvable token
//!    passes only if its epic's provenance names a story that CREATES it, that
//!    story key exists in sprint-status, its status is not `done`, AND the
//!    owning story actually claims the token. Without the last clause the gate
//!    checks that a string exists in a YAML file.
//! 5. **Counters that hard-fail at zero.** `findings.is_empty()` cannot tell
//!    "everything resolved" from "the glob matched nothing"
//!    (`gate_common::vacuous_legs` is the same guard one level up).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::gate_common::emit_command;

const GATE_NAME: &str = "check-exit-commands";
const EPICS_DIR: &str = "_bmad-output/planning-artifacts/epics";
const STORIES_DIR: &str = "_bmad-output/implementation-artifacts";
const SPRINT_STATUS: &str = "_bmad-output/implementation-artifacts/sprint-status.yaml";

/// The marker that designates the ONE file per epic carrying the exit block.
const EXIT_BLOCK_MARKER: &str = "Hermetic exit command";

/// Hard sub-second timeout for every `--help` subprocess.
///
/// MANDATORY, not defensive: a regressed `maos --help` once hung through
/// daemon boot. Eight hundred milliseconds leaves ordinary process-startup
/// margin while preserving the AC4 requirement that help respond in under one
/// second. GNU `timeout` gets a further 100 ms to escalate TERM to KILL.
const HELP_TIMEOUT: &str = "0.8";
const HELP_KILL_AFTER: &str = "0.1";

/// Shell keywords that introduce a nested command; the real head follows them.
const COMPOUND_PREFIXES: &[&str] = &["do", "then", "else", "elif", "!", "time"];

/// The CLOSED declared-non-command list (F7).
///
/// Every entry is a shell keyword/builtin or a toolchain/coreutils binary that
/// is present on any CI runner and is NOT a maos surface. Additions are
/// deliberate: an unrecognised head is a finding, so extending this list is a
/// reviewed act rather than a silent skip. `break`, `mktemp`, `od`, `tr`, `seq`
/// and `exec` are here because §6 measured them missing from the first draft.
const DECLARED_NON_COMMANDS: &[&str] = &[
    // shell keywords / builtins
    "break",
    "case",
    "cd",
    "continue",
    "done",
    "esac",
    "eval",
    "exec",
    "exit",
    "export",
    "fi",
    "for",
    "if",
    "in",
    "read",
    "return",
    "set",
    "shift",
    "test",
    "then",
    "trap",
    "unset",
    "wait",
    "while",
    // coreutils and friends
    "awk",
    "basename",
    "cat",
    "chmod",
    "cp",
    "cut",
    "date",
    "dirname",
    "echo",
    "env",
    "false",
    "grep",
    "head",
    "kill",
    "ln",
    "ls",
    "mkdir",
    "mktemp",
    "mv",
    "od",
    "printf",
    "pwd",
    "rm",
    "sed",
    "seq",
    "sleep",
    "sort",
    "sha256sum",
    "tail",
    "tar",
    "tee",
    "touch",
    "tr",
    "true",
    "uniq",
    "wc",
    "xargs",
    // toolchain
    "bash",
    "cargo",
    "dpkg",
    "git",
    "jq",
    "python3",
    "rustc",
    "sh",
];

/// Binaries this repo SHIPS. Every binary resolves against a real `--help`
/// response or remains owed by the exact exit line that names it.
const MAOS_BINARIES: &[&str] = &[
    "maos",
    "maosctl",
    "xtask",
    "maos-registry-server",
    "maos-spirit",
    "maos-wasm-runner",
];

// ───────────────────────────── inputs ─────────────────────────────

/// One candidate epic document.
#[derive(Clone, Debug)]
pub struct EpicFile {
    pub name: String,
    pub content: String,
}

/// Everything [`audit`] reads, supplied explicitly so tests can plant it.
#[derive(Clone, Debug, Default)]
pub struct Corpus {
    /// epic number → every `epic-<N>-*.md` that exists, block-carrying or not.
    pub epics: BTreeMap<u32, Vec<EpicFile>>,
    /// sprint-status `development_status` keys → status.
    pub statuses: BTreeMap<String, String>,
    /// story key → that story's file text, for F14's claim check.
    pub story_files: BTreeMap<String, String>,
}

/// The verb surfaces resolved from real `--help` output.
#[derive(Clone, Debug, Default)]
pub struct Surfaces {
    pub maos: BTreeSet<String>,
    pub maosctl: BTreeSet<String>,
    pub xtask: BTreeSet<String>,
    /// Nested or secondary command levels keyed by their command path, e.g.
    /// `maosctl spirit` → `{hot-swap-precheck, inspect, upgrade}` and
    /// `maos-spirit` → `{inspect, publish, validate}`.
    pub subcommands: BTreeMap<String, BTreeSet<String>>,
    pub one_shot_modes: BTreeSet<String>,
    /// Binaries that exist right now (resolved, not owed).
    pub present_binaries: BTreeSet<String>,
    /// Binaries present on disk whose `--help` response could not be read.
    ///
    /// Kept separate from global surface failures so an exit line may classify
    /// an intentionally unimplemented binary as owed by its named open story.
    pub unavailable_binaries: BTreeMap<String, String>,
    /// A binary existed but its required help surface could not be read.
    /// These are findings, not warnings: treating an unreadable surface as
    /// commandless would make the blocking gate fail open.
    pub surface_failures: Vec<String>,
}

// ───────────────────────────── findings ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// A non-`done` epic with no file carrying the exit-block marker.
    RosterZero,
    /// More than one file carrying the marker for the same epic.
    RosterMultiple,
    /// The marker is present but no complete fenced block follows it.
    ExitBlockMissing,
    /// A command head that is neither resolvable nor declared.
    UnrecognisedToken,
    /// A verb that does not exist and is not owed by a named, open story.
    UnresolvedVerb,
    /// A `MAOS_ONE_SHOT=<mode>` prefix naming a mode the binary does not have.
    UnresolvedOneShotMode,
    /// A `<placeholder>` bash would read as a redirection, or an undeclared one.
    PlaceholderHazard,
    /// Quotes, escapes, or command substitutions do not form a complete shell line.
    MalformedCommand,
    /// A required binary help surface exists but cannot be loaded or parsed.
    SurfaceUnavailable,
    /// The gate read no blocks or resolved no tokens.
    VacuousCorpus,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub epic: Option<u32>,
    pub file: String,
    pub line: usize,
    pub detail: String,
}

/// A token that does not resolve at HEAD but is lawfully owed (F8 + F14).
#[derive(Clone, Debug, serde::Serialize)]
pub struct OwedToken {
    pub token: String,
    pub epic: u32,
    pub owner_story: String,
    pub owner_status: String,
    pub claimed_in: String,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Audit {
    pub findings: Vec<Finding>,
    pub owed: Vec<OwedToken>,
    pub blocks_read: usize,
    pub tokens_resolved: usize,
    pub epics_in_roster: usize,
}

impl Audit {
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }
}

// ───────────────────────────── tokenising ─────────────────────────────

/// One command parsed out of an exit-block line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRef {
    pub head: String,
    pub args: Vec<String>,
    /// `MAOS_ONE_SHOT=<mode>` values that prefixed this command.
    pub one_shot_modes: Vec<String>,
    pub line: usize,
}

/// Split `text` on top-level separators, honouring quotes.
///
/// Command substitutions must already have been replaced by sentinels — a pipe
/// inside `$( )` otherwise re-heads the command (epic-20 line 7, measured).
fn split_separators(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut single = false;
    let mut double = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\'' if !double => {
                single = !single;
                cur.push(c);
            }
            '"' if !single => {
                double = !double;
                cur.push(c);
            }
            '&' | '|' | ';' if !single && !double => {
                // Consume a doubled operator as one separator.
                if i + 1 < chars.len() && chars[i + 1] == c {
                    i += 1;
                }
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
        i += 1;
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract `$( … )` bodies (nesting-aware), replacing each with a sentinel.
///
/// Returned bodies are tokenised as commands in their own right, which is why
/// `mktemp`, `od` and `tr` appear in [`DECLARED_NON_COMMANDS`]: they are heads
/// of real substituted commands, not tokens the gate may ignore.
fn extract_substitutions(text: &str) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut bodies = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '(' {
            let mut depth = 0usize;
            let mut j = i + 1;
            let mut body = String::new();
            while j < chars.len() {
                match chars[j] {
                    '(' => {
                        depth += 1;
                        if depth > 1 {
                            body.push('(');
                        }
                    }
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        body.push(')');
                    }
                    other => body.push(other),
                }
                j += 1;
            }
            if depth == 0 {
                bodies.push(body);
                out.push_str("SUBST");
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    (out, bodies)
}

/// Byte offset of a trailing `# …` comment that sits outside quotes.
fn trailing_comment_start(text: &str) -> Option<usize> {
    let mut single = false;
    let mut double = false;
    for (idx, c) in text.char_indices() {
        match c {
            '\'' if !double => single = !single,
            '"' if !single => double = !double,
            '#' if !single && !double => {
                let preceded_by_space = text[..idx].ends_with(char::is_whitespace);
                if idx == 0 || preceded_by_space {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

/// Strip a trailing `# …` comment that sits outside quotes.
///
/// A WHOLE-LINE comment is handled by the caller: five of epic-19's six block
/// lines are whole-line comments, and AC3's original clause covered only
/// trailing ones.
fn strip_trailing_comment(text: &str) -> &str {
    trailing_comment_start(text).map_or(text, |start| &text[..start])
}

/// Provenance label for one physical exit-block line.
///
/// Most blocks use `Line 1`, `Line 2`, … and need no inline labels. A block
/// that splits one logical command across `1`/`1b` records the label in its
/// trailing shell comment, which takes precedence over the physical ordinal.
fn exit_line_label(ordinal: usize, text: &str) -> String {
    let Some(start) = trailing_comment_start(text) else {
        return ordinal.to_string();
    };
    let candidate = text[start + 1..]
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches(|c: char| !c.is_ascii_alphanumeric());
    let digit_count = candidate.chars().take_while(|c| c.is_ascii_digit()).count();
    let suffix = &candidate[digit_count..];
    if digit_count > 0
        && (suffix.is_empty()
            || (suffix.len() == 1 && suffix.chars().all(|c| c.is_ascii_lowercase())))
    {
        candidate.to_string()
    } else {
        ordinal.to_string()
    }
}

/// Is `word` a `VAR=value` assignment prefix?
fn assignment(word: &str) -> Option<(&str, &str)> {
    let (name, value) = word.split_once('=')?;
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    // A leading digit is not a valid shell name (`3<>/dev/tcp` is a redirection).
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some((name, value))
}

/// Split a command segment into whitespace-delimited words, honouring quotes.
fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut single = false;
    let mut double = false;
    for c in text.chars() {
        match c {
            '\'' if !double => {
                single = !single;
                cur.push(c);
            }
            '"' if !single => {
                double = !double;
                cur.push(c);
            }
            c if c.is_whitespace() && !single && !double => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Reject syntax states that the lightweight tokeniser cannot safely classify.
/// This is deliberately narrower than a shell implementation: it guards the
/// quote, escape, and command-substitution boundaries this gate consumes.
fn validate_shell_balance(text: &str) -> Result<(), String> {
    let chars: Vec<char> = text.chars().collect();
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    let mut substitution_depth = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if c == '\\' && !single {
            escaped = true;
            i += 1;
            continue;
        }
        if c == '\'' && !double {
            single = !single;
            i += 1;
            continue;
        }
        if c == '"' && !single {
            double = !double;
            i += 1;
            continue;
        }
        if !single && c == '$' && chars.get(i + 1) == Some(&'(') {
            substitution_depth += 1;
            i += 2;
            continue;
        }
        if !single && c == ')' && substitution_depth > 0 {
            substitution_depth -= 1;
        }
        i += 1;
    }
    if escaped {
        return Err("line ends with an unfinished escape".to_string());
    }
    if single {
        return Err("line has an unterminated single quote".to_string());
    }
    if double {
        return Err("line has an unterminated double quote".to_string());
    }
    if substitution_depth != 0 {
        return Err("line has an unterminated command substitution".to_string());
    }
    Ok(())
}

/// Parse one exit-block line into zero or more commands.
pub fn tokenise_line(line: usize, text: &str) -> Result<Vec<CommandRef>, String> {
    let trimmed = text.trim();
    // Whole-line comment: no tokens at all.
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(Vec::new());
    }
    let body = strip_trailing_comment(trimmed);
    validate_shell_balance(body)?;
    // Substitutions FIRST, so their internals cannot re-head the outer command.
    let (masked, substitutions) = extract_substitutions(body);

    let mut commands = Vec::new();
    let mut segments: Vec<String> = split_separators(&masked);
    // Substituted bodies are commands in their own right.
    for sub in &substitutions {
        segments.extend(split_separators(sub));
    }

    for segment in segments {
        let mut segment = segment.trim().to_string();
        // Peel subshell/group wrappers.
        while (segment.starts_with('(') && segment.ends_with(')'))
            || (segment.starts_with('{') && segment.ends_with('}'))
        {
            segment = segment[1..segment.len() - 1].trim().to_string();
        }
        let mut ws = words(&segment);
        let mut one_shot_modes = Vec::new();
        // Assignments, compound keywords and group parens can appear in any
        // order at the head (`do (exec 3<>/dev/tcp/…)` glues `(` to `exec`, and
        // `for i in $(seq 1 50); do (exec …)` is the measured case), so peel
        // them in ONE loop until the head stops changing rather than in a fixed
        // sequence of passes.
        loop {
            let Some(first) = ws.first().cloned() else {
                break;
            };
            // A group paren/brace glued to the head word.
            let unglued = first.trim_start_matches(['(', '{']).to_string();
            if unglued != first {
                if unglued.is_empty() {
                    ws.remove(0);
                } else {
                    ws[0] = unglued;
                }
                continue;
            }
            if let Some((name, value)) = assignment(&first) {
                if name == "MAOS_ONE_SHOT" {
                    one_shot_modes.push(value.to_string());
                }
                ws.remove(0);
                continue;
            }
            if COMPOUND_PREFIXES.contains(&first.as_str()) {
                ws.remove(0);
                continue;
            }
            break;
        }
        let Some(head) = ws.first().cloned() else {
            // A pure assignment (`RUN_PID=$!`) or an empty group: no command.
            // A bare `MAOS_ONE_SHOT=<mode>` with no head still carries a mode
            // to resolve — a verb-less `maos` under an undeclared mode is the
            // hole §6 found, so it must not vanish here.
            if !one_shot_modes.is_empty() {
                commands.push(CommandRef {
                    head: "maos".to_string(),
                    args: Vec::new(),
                    one_shot_modes,
                    line,
                });
            }
            continue;
        };
        commands.push(CommandRef {
            head,
            args: ws[1..].to_vec(),
            one_shot_modes,
            line,
        });
    }
    Ok(commands)
}

// ───────────────────────────── block extraction ─────────────────────────────

/// Fence state machine, the shape of `gen_abi_docs`'s `FenceTracker`.
///
/// Every exit-block fence in this repo is a bare ` ``` ` with NO info string,
/// so a `” ```bash ”` locator finds nothing.
#[derive(Default)]
struct FenceTracker {
    open: Option<(char, usize)>,
}

impl FenceTracker {
    fn toggle(&mut self, line: &str) -> bool {
        let trimmed = line.trim_start();
        let Some(first) = trimmed.chars().next() else {
            return false;
        };
        if first != '`' && first != '~' {
            return false;
        }
        let len = trimmed.chars().take_while(|&c| c == first).count();
        if len < 3 {
            return false;
        }
        match self.open {
            Some((c, l)) if c == first && l == len => {
                self.open = None;
                true
            }
            None => {
                self.open = Some((first, len));
                true
            }
            _ => false,
        }
    }

    fn is_open(&self) -> bool {
        self.open.is_some()
    }
}

/// The first fenced block following the exit-block marker, as `(line_no, text)`.
fn exit_block(content: &str) -> Result<Vec<(usize, String)>, String> {
    let mut out = Vec::new();
    let mut seen_marker = false;
    let mut tracker = FenceTracker::default();
    let mut opened = false;
    let mut closed = false;
    for (index, line) in content.lines().enumerate() {
        if !seen_marker {
            if line.contains(EXIT_BLOCK_MARKER) {
                seen_marker = true;
            }
            continue;
        }
        if closed {
            break;
        }
        let was_open = tracker.is_open();
        let toggled = tracker.toggle(line);
        if toggled {
            if was_open {
                closed = true;
            } else {
                opened = true;
            }
            continue;
        }
        if tracker.is_open() {
            out.push((index + 1, line.to_string()));
        }
    }
    if !opened {
        return Err(format!(
            "`{EXIT_BLOCK_MARKER}` is present but no fenced block follows it"
        ));
    }
    if !closed {
        return Err(format!(
            "`{EXIT_BLOCK_MARKER}` opens a fenced block that never closes"
        ));
    }
    if out.is_empty() {
        return Err(format!(
            "`{EXIT_BLOCK_MARKER}` is present but its fenced block is empty"
        ));
    }
    Ok(out)
}

// ───────────────────────────── resolve-or-be-named ─────────────────────────────

/// Does `text` contain `token` as a command-like token rather than as a
/// substring of a larger word such as `run` in `runtime`?
fn contains_token(text: &str, token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    token_positions(text, token).next().is_some()
}

fn token_positions<'a>(text: &'a str, token: &'a str) -> impl Iterator<Item = usize> + 'a {
    text.match_indices(token).filter_map(move |(start, _)| {
        let before = text[..start].chars().next_back();
        let after = text[start + token.len()..].chars().next();
        let token_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
        (before.is_none_or(|c| !token_char(c)) && after.is_none_or(|c| !token_char(c)))
            .then_some(start)
    })
}

/// Story references on the same provenance bullet, nearest to the exact token
/// first. This binds each token to its own clause instead of accepting the
/// first unrelated `created by` phrase elsewhere on a long `- Line` bullet.
fn provenance_story_refs(line: &str, token: &str) -> Vec<String> {
    let reference = regex::Regex::new(r"\*{0,2}(\d+-\d+[a-z]?)\*{0,2}\s*(?:AC|ADR)\d*")
        .expect("valid provenance story-reference regex");
    let token_positions: Vec<usize> = token_positions(line, token).collect();
    let mut ranked: Vec<(usize, usize, String)> = reference
        .captures_iter(line)
        .filter_map(|capture| {
            let whole = capture.get(0)?;
            let short = capture.get(1)?.as_str().to_string();
            let distance = token_positions
                .iter()
                .map(|position| position.abs_diff(whole.start()))
                .min()?;
            Some((distance, whole.start(), short))
        })
        .collect();
    ranked.sort_by_key(|(distance, position, _)| (*distance, *position));
    ranked.dedup_by(|left, right| left.2 == right.2);
    ranked.into_iter().map(|(_, _, short)| short).collect()
}

fn owner_epic_number(short_story: &str) -> Option<u32> {
    short_story.split('-').next()?.parse().ok()
}

/// Does `text` name the complete command path in order?
///
/// Tokens may be separated by wrapper arguments (`cargo run -p xtask --
/// release-dry-run`), but every path component must be present and ordered.
fn claims_command(text: &str, token: &str, command_path: &str) -> bool {
    if !contains_token(text, token) {
        return false;
    }
    let mut remainder = text;
    for component in command_path.split_whitespace() {
        let Some(position) = token_positions(remainder, component).next() else {
            return false;
        };
        remainder = &remainder[position + component.len()..];
    }
    true
}

/// Does an `- **ACn.**` bullet in the owning epic claim this exact command?
fn epic_claims_command(corpus: &Corpus, owner_epic: u32, token: &str, command_path: &str) -> bool {
    corpus
        .epics
        .get(&owner_epic)
        .into_iter()
        .flatten()
        .flat_map(|file| file.content.lines())
        .filter(|line| line.trim_start().starts_with("- **AC"))
        .any(|line| claims_command(line, token, command_path))
}

/// Does this exact exit line name an OPEN story that creates this command?
fn owed_by_open_story(
    token: &str,
    command_path: &str,
    provenance_label: &str,
    epic: u32,
    epic_text: &str,
    corpus: &Corpus,
) -> Result<OwedToken, String> {
    let line_prefix = format!("- Line {provenance_label} ");
    let Some(provenance) = epic_text
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with(&line_prefix) && claims_command(line, token, command_path))
    else {
        return Err(format!(
            "provenance `Line {provenance_label}` does not bind `{command_path}` to a story"
        ));
    };

    let mut tried = Vec::new();
    for short in provenance_story_refs(provenance, token) {
        // Short key → the full sprint-status key by prefix.
        let Some((full, status)) = corpus
            .statuses
            .iter()
            .find(|(key, _)| key.starts_with(&format!("{short}-")) || **key == short)
        else {
            tried.push(format!("{short} (no sprint-status key)"));
            continue;
        };
        if status == "done" {
            tried.push(format!("{full} is done"));
            continue;
        }
        let owner_epic = owner_epic_number(&short).unwrap_or(epic);
        // F14: the owner must actually claim the exact command.
        let claimed_in = match corpus.story_files.get(full) {
            Some(text) if claims_command(text, token, command_path) => {
                format!("story file {full}.md")
            }
            _ if epic_claims_command(corpus, owner_epic, token, command_path) => {
                format!("epic-{owner_epic} AC text")
            }
            _ => {
                tried.push(format!("{full} never names `{command_path}`"));
                continue;
            }
        };
        return Ok(OwedToken {
            token: token.to_string(),
            epic,
            owner_story: full.clone(),
            owner_status: status.clone(),
            claimed_in,
        });
    }
    Err(if tried.is_empty() {
        format!("provenance `Line {provenance_label}` names no story for `{command_path}`")
    } else {
        format!("provenance rejected: {}", tried.join("; "))
    })
}

// ───────────────────────────── the audit ─────────────────────────────

/// `<name>` written unquoted is TWO bash redirections, not a placeholder.
///
/// Measured on epic-16 line 3: run verbatim against a stub, `maosctl halt
/// resolve <halt_id> --spirit hello-spirit …` yields
/// `ARGV: [halt] [resolve] [hello-spirit] …` and CREATES A FILE named
/// `--spirit` — the required flag is swallowed by `> --spirit`. `bash -n` is
/// clean throughout, which is why prose review never caught it (F15).
fn placeholder(arg: &str) -> Option<(&str, bool)> {
    let (candidate, quoted) = if let Some(inner) = arg
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        (inner, true)
    } else if let Some(inner) = arg
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        (inner, true)
    } else {
        (arg, false)
    };
    let inner = candidate.strip_prefix('<')?.strip_suffix('>')?;
    if inner.is_empty() || inner.contains('<') || inner.contains('>') {
        return None;
    }
    Some((inner, quoted))
}

/// Is the placeholder declared somewhere in its epic?
fn placeholder_declared(epic_text: &str, name: &str) -> bool {
    epic_text.contains(&format!("<{name}>"))
        && epic_text
            .lines()
            .any(|line| line.contains(&format!("<{name}>")) && line.contains("substitut"))
}

/// Audit the corpus. PURE — every input is an argument (F11).
pub fn audit(corpus: &Corpus, surfaces: &Surfaces) -> Audit {
    let mut result = Audit::default();

    for failure in &surfaces.surface_failures {
        result.findings.push(Finding {
            kind: FindingKind::SurfaceUnavailable,
            epic: None,
            file: String::new(),
            line: 0,
            detail: failure.clone(),
        });
    }

    for (epic_key, status) in &corpus.statuses {
        let Some(epic) = epic_key
            .strip_prefix("epic-")
            .and_then(|number| number.parse::<u32>().ok())
        else {
            continue;
        };
        if status == "done" {
            continue;
        }
        result.epics_in_roster += 1;
        let files: &[EpicFile] = corpus.epics.get(&epic).map(Vec::as_slice).unwrap_or(&[]);
        let carriers: Vec<&EpicFile> = files
            .iter()
            .filter(|file| file.content.contains(EXIT_BLOCK_MARKER))
            .collect();

        if carriers.is_empty() {
            result.findings.push(Finding {
                kind: FindingKind::RosterZero,
                epic: Some(epic),
                file: String::new(),
                line: 0,
                detail: format!(
                    "epic {epic} is `{status}` but no file carries `{EXIT_BLOCK_MARKER}` \
                     ({} candidate file(s) matched `epic-{epic}-*.md`)",
                    files.len()
                ),
            });
            continue;
        }
        if carriers.len() > 1 {
            result.findings.push(Finding {
                kind: FindingKind::RosterMultiple,
                epic: Some(epic),
                file: carriers
                    .iter()
                    .map(|file| file.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                line: 0,
                detail: format!(
                    "epic {epic} has {} files carrying `{EXIT_BLOCK_MARKER}`; exactly one is required",
                    carriers.len()
                ),
            });
            continue;
        }

        let carrier = carriers[0];
        let block = match exit_block(&carrier.content) {
            Ok(block) => block,
            Err(detail) => {
                result.findings.push(Finding {
                    kind: FindingKind::ExitBlockMissing,
                    epic: Some(epic),
                    file: carrier.name.clone(),
                    line: 0,
                    detail,
                });
                continue;
            }
        };
        result.blocks_read += 1;

        for (ordinal, (line_no, text)) in block.iter().enumerate() {
            let commands = match tokenise_line(*line_no, text) {
                Ok(commands) => commands,
                Err(detail) => {
                    result.findings.push(Finding {
                        kind: FindingKind::MalformedCommand,
                        epic: Some(epic),
                        file: carrier.name.clone(),
                        line: *line_no,
                        detail,
                    });
                    continue;
                }
            };
            let provenance_label = exit_line_label(ordinal + 1, text);
            for command in commands {
                audit_command(
                    &command,
                    &provenance_label,
                    epic,
                    &carrier.name,
                    &carrier.content,
                    corpus,
                    surfaces,
                    &mut result,
                );
            }
        }
    }

    // Counters hard-fail at zero: `findings.is_empty()` cannot distinguish
    // "all resolved" from "the glob matched nothing".
    if result.blocks_read == 0 {
        result.findings.push(Finding {
            kind: FindingKind::VacuousCorpus,
            epic: None,
            file: String::new(),
            line: 0,
            detail: "read ZERO exit blocks — refusing to report green over a corpus this gate \
                     could not find"
                .to_string(),
        });
    }
    // A token that is lawfully OWED was still classified, so it counts as work
    // done. The condition this guard exists for is "the gate classified NOTHING".
    if result.tokens_resolved == 0 && result.owed.is_empty() {
        result.findings.push(Finding {
            kind: FindingKind::VacuousCorpus,
            epic: None,
            file: String::new(),
            line: 0,
            detail: "classified ZERO tokens (none resolved, none owed) — a gate that resolves \
                     nothing passes for the wrong reason (D19)"
                .to_string(),
        });
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn audit_command(
    command: &CommandRef,
    provenance_label: &str,
    epic: u32,
    file: &str,
    epic_text: &str,
    corpus: &Corpus,
    surfaces: &Surfaces,
    result: &mut Audit,
) {
    let finding = |kind: FindingKind, detail: String, result: &mut Audit| {
        result.findings.push(Finding {
            kind,
            epic: Some(epic),
            file: file.to_string(),
            line: command.line,
            detail,
        });
    };

    // `<placeholder>` arguments (F15). Quoting removes the bash redirection
    // hazard, but it does not remove the requirement that the epic declare how
    // the placeholder is substituted.
    for arg in &command.args {
        if let Some((name, quoted)) = placeholder(arg) {
            if !placeholder_declared(epic_text, name) {
                finding(
                    FindingKind::PlaceholderHazard,
                    format!("`<{name}>` is not declared as a substituted placeholder in this epic"),
                    result,
                );
            } else if !quoted {
                finding(
                    FindingKind::PlaceholderHazard,
                    format!(
                        "`<{name}>` is declared but written UNQUOTED — bash reads it as two \
                         redirections, swallowing the following flag and creating a file named \
                         after it. Quote it (`\"<{name}>\"`)."
                    ),
                    result,
                );
            }
        }
    }

    // `MAOS_ONE_SHOT=<mode>` is RESOLVED against the mode table, never stripped.
    for mode in &command.one_shot_modes {
        if surfaces.one_shot_modes.contains(mode) {
            result.tokens_resolved += 1;
        } else {
            match owed_by_open_story(
                mode,
                &format!("MAOS_ONE_SHOT={mode}"),
                provenance_label,
                epic,
                epic_text,
                corpus,
            ) {
                Ok(owed) => result.owed.push(owed),
                Err(why) => finding(
                    FindingKind::UnresolvedOneShotMode,
                    format!("`MAOS_ONE_SHOT={mode}` names no known one-shot mode; {why}"),
                    result,
                ),
            }
        }
    }

    // Path-qualified binaries (`target/debug/maos run …`) head at their stem.
    let head = command
        .head
        .rsplit('/')
        .next()
        .unwrap_or(&command.head)
        .to_string();

    // `env [OPTION]... [NAME=VALUE]... COMMAND ...` wraps a real child command.
    // Treating `env` as terminal silently bypasses every resolver below.
    if head == "env" {
        let mut index = 0usize;
        let mut inherited_modes = command.one_shot_modes.clone();
        while let Some(arg) = command.args.get(index) {
            if arg == "--" {
                index += 1;
                break;
            }
            if matches!(
                arg.as_str(),
                "-u" | "--unset" | "-C" | "--chdir" | "-S" | "--split-string"
            ) {
                index += 2;
                continue;
            }
            if arg.starts_with('-') {
                index += 1;
                continue;
            }
            if let Some((name, value)) = assignment(arg) {
                if name == "MAOS_ONE_SHOT" {
                    inherited_modes.push(value.to_string());
                }
                index += 1;
                continue;
            }
            break;
        }
        if let Some(child_head) = command.args.get(index) {
            let child = CommandRef {
                head: child_head.clone(),
                args: command.args[index + 1..].to_vec(),
                one_shot_modes: inherited_modes,
                line: command.line,
            };
            audit_command(
                &child,
                provenance_label,
                epic,
                file,
                epic_text,
                corpus,
                surfaces,
                result,
            );
        }
        return;
    }

    // `cargo run -p xtask -- <verb>` carries an xtask verb; every other cargo
    // invocation is cargo's own surface.
    if head == "cargo" {
        if let Some(verb) = xtask_verb_after_double_dash(&command.args) {
            let _ = resolve_verb(
                "xtask",
                verb,
                &surfaces.xtask,
                provenance_label,
                epic,
                epic_text,
                corpus,
                command,
                file,
                result,
            );
        }
        return;
    }

    if DECLARED_NON_COMMANDS.contains(&head.as_str()) {
        return;
    }

    if !MAOS_BINARIES.contains(&head.as_str()) {
        finding(
            FindingKind::UnrecognisedToken,
            format!(
                "`{head}` is neither a maos surface nor a declared non-command. Refusing to skip \
                 a token this gate does not understand: an open skip means one typo in a prefix \
                 constant silently empties the corpus (F7)."
            ),
            result,
        );
        return;
    }

    if let Some(why) = surfaces.unavailable_binaries.get(&head) {
        match owed_by_open_story(&head, &head, provenance_label, epic, epic_text, corpus) {
            Ok(owed) => result.owed.push(owed),
            Err(owner) => finding(
                FindingKind::SurfaceUnavailable,
                format!("`{head} --help` could not be read: {why}; {owner}"),
                result,
            ),
        }
        return;
    }

    // The binary itself may be owed by an open story.
    if !surfaces.present_binaries.contains(&head) {
        match owed_by_open_story(&head, &head, provenance_label, epic, epic_text, corpus) {
            Ok(owed) => result.owed.push(owed),
            Err(why) => finding(
                FindingKind::UnresolvedVerb,
                format!("binary `{head}` does not exist at HEAD; {why}"),
                result,
            ),
        }
        return;
    }

    let root_surface = match head.as_str() {
        "maos" => Some(&surfaces.maos),
        "maosctl" => Some(&surfaces.maosctl),
        "xtask" => Some(&surfaces.xtask),
        _ => surfaces.subcommands.get(&head),
    };
    let Some(surface) = root_surface else {
        // A shipped binary with successful help and no verb grammar resolves
        // at the binary. Unlike an unreadable help response, this is observed.
        result.tokens_resolved += 1;
        return;
    };

    let Some((mut argument_index, verb)) = command
        .args
        .iter()
        .enumerate()
        .find(|(_, arg)| !arg.starts_with('-'))
    else {
        // A bare `maos` IS legal (shell mode, exit 0).
        if head != "maos" && !surface.is_empty() {
            finding(
                FindingKind::UnresolvedVerb,
                format!("`{head}` requires a subcommand"),
                result,
            );
        }
        return;
    };
    if !resolve_verb(
        &head,
        verb,
        surface,
        provenance_label,
        epic,
        epic_text,
        corpus,
        command,
        file,
        result,
    ) {
        return;
    }

    let mut command_path = format!("{head} {verb}");
    loop {
        let Some(children) = surfaces.subcommands.get(&command_path) else {
            break;
        };
        argument_index += 1;
        let Some((next_index, child)) = command
            .args
            .iter()
            .enumerate()
            .skip(argument_index)
            .find(|(_, arg)| !arg.starts_with('-'))
        else {
            finding(
                FindingKind::UnresolvedVerb,
                format!("`{command_path}` requires a subcommand"),
                result,
            );
            break;
        };
        if !resolve_verb(
            &command_path,
            child,
            children,
            provenance_label,
            epic,
            epic_text,
            corpus,
            command,
            file,
            result,
        ) {
            break;
        }
        argument_index = next_index;
        command_path.push(' ');
        command_path.push_str(child);
    }
}

/// The verb in `cargo run -p xtask -- <verb>`, if this is that shape.
fn xtask_verb_after_double_dash(args: &[String]) -> Option<&str> {
    if args.first().map(String::as_str) != Some("run") {
        return None;
    }
    let mut targets_xtask = false;
    for (index, arg) in args.iter().enumerate() {
        if arg == "-p" && args.get(index + 1).map(String::as_str) == Some("xtask") {
            targets_xtask = true;
        }
        if arg == "--" && targets_xtask {
            return args
                .get(index + 1..)?
                .iter()
                .map(String::as_str)
                .find(|arg| !arg.starts_with('-'));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_verb(
    binary: &str,
    verb: &str,
    surface: &BTreeSet<String>,
    provenance_label: &str,
    epic: u32,
    epic_text: &str,
    corpus: &Corpus,
    command: &CommandRef,
    file: &str,
    result: &mut Audit,
) -> bool {
    if surface.contains(verb) {
        result.tokens_resolved += 1;
        return true;
    }
    let command_path = format!("{binary} {verb}");
    match owed_by_open_story(
        verb,
        &command_path,
        provenance_label,
        epic,
        epic_text,
        corpus,
    ) {
        Ok(owed) => result.owed.push(owed),
        Err(why) => result.findings.push(Finding {
            kind: FindingKind::UnresolvedVerb,
            epic: Some(epic),
            file: file.to_string(),
            line: command.line,
            detail: format!("`{binary} {verb}` does not resolve against `{binary} --help`; {why}"),
        }),
    }
    false
}

// ───────────────────────────── I/O ─────────────────────────────

/// Run `<program> --help` under a hard timeout and return its stdout.
fn help_output(program: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("timeout");
    command
        .arg("-k")
        .arg(HELP_KILL_AFTER)
        .arg(HELP_TIMEOUT)
        .arg(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A scratch HOME keeps a `--help` that still boots from touching the real one.
    let scratch = std::env::temp_dir().join(format!("maos-exit-cmd-help-{}", std::process::id()));
    let _ = fs::create_dir_all(&scratch);
    command.env("MAOS_HOME", &scratch);
    let output = command
        .output()
        .map_err(|error| format!("cannot invoke {} --help: {error}", program.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} {} exited {} (a hang is EXIT 124 under `timeout`; the gate must RED on it, \
             never inherit it)",
            program.display(),
            args.join(" "),
            output.status
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse a section of `maos --help` written to the AC4 contract.
///
/// ```text
/// VERBS:
///   init          [air-gap,network]  Initialise a MAOS home
/// MAOS_ONE_SHOT MODES:
///   registry-server
/// ```
fn parse_help_section(help: &str, header: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut inside = false;
    for line in help.lines() {
        if line.trim() == header {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.trim().is_empty() || !line.starts_with("  ") {
            break;
        }
        if let Some(name) = line.split_whitespace().next() {
            out.insert(name.to_string());
        }
    }
    out
}

/// Parse clap's `Commands:` block into a subcommand set.
fn parse_clap_subcommands(help: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut inside = false;
    for line in help.lines() {
        let trimmed = line.trim_end();
        if trimmed.ends_with("ommands:") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if trimmed.is_empty() {
            inside = false;
            continue;
        }
        if !trimmed.starts_with("  ") {
            continue;
        }
        let word = trimmed.trim_start();
        if word.starts_with('-') {
            continue;
        }
        if let Some(name) = word.split_whitespace().next() {
            out.insert(name.to_string());
        }
    }
    out
}

fn workspace_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn load_corpus(root: &Path) -> Corpus {
    let mut corpus = Corpus {
        statuses: crate::sprint_status::load_sprint_status(
            root.join(SPRINT_STATUS).to_string_lossy().as_ref(),
        )
        .into_iter()
        .collect(),
        ..Default::default()
    };

    // The `epic-N-` walk, the shape of `check_epic_close_coherence::epic_doc_pins`.
    if let Ok(entries) = fs::read_dir(root.join(EPICS_DIR)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("epic-") || !name.ends_with(".md") {
                continue;
            }
            // `epic-list-revised-…` has no derivable number: classified
            // explicitly rather than `continue`d past (F10).
            let Some(number) = name
                .trim_start_matches("epic-")
                .split('-')
                .next()
                .and_then(|head| head.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(content) = fs::read_to_string(entry.path()) else {
                continue;
            };
            corpus
                .epics
                .entry(number)
                .or_default()
                .push(EpicFile { name, content });
        }
    }
    for files in corpus.epics.values_mut() {
        files.sort_by(|a, b| a.name.cmp(&b.name));
    }

    // Story files, for F14's claim check.
    if let Ok(entries) = fs::read_dir(root.join(STORIES_DIR)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let Some(key) = name.strip_suffix(".md") else {
                continue;
            };
            if let Ok(content) = fs::read_to_string(entry.path()) {
                corpus.story_files.insert(key.to_string(), content);
            }
        }
    }
    corpus
}

fn load_nested_clap_surfaces(
    program: &Path,
    binary: &str,
    prefix: &[String],
    help: &str,
    surfaces: &mut Surfaces,
) {
    let children = parse_clap_subcommands(help);
    if children.is_empty() {
        return;
    }
    let key = if prefix.is_empty() {
        binary.to_string()
    } else {
        format!("{binary} {}", prefix.join(" "))
    };
    surfaces.subcommands.insert(key, children.clone());
    if prefix.len() >= 3 {
        return;
    }
    for child in children {
        if child == "help" {
            // clap synthesizes `help [COMMAND]...`; it describes the parent
            // tree and is not itself another command namespace.
            continue;
        }
        let mut child_prefix = prefix.to_vec();
        child_prefix.push(child);
        let mut args: Vec<&str> = child_prefix.iter().map(String::as_str).collect();
        args.push("--help");
        match help_output(program, &args) {
            Ok(child_help) => {
                load_nested_clap_surfaces(program, binary, &child_prefix, &child_help, surfaces);
            }
            Err(why) => surfaces.surface_failures.push(format!(
                "{binary} {} --help could not be read: {why}",
                child_prefix.join(" ")
            )),
        }
    }
}

/// Resolve every surface by subprocess `--help` (F9).
///
/// Rule 1 means *the binary answers*, not *the source lists it*. xtask resolves
/// its own surface through `current_exe()` — exact, no new dependency, and no
/// need to move ~950 lines of `enum Commands` into the lib.
fn load_surfaces(root: &Path) -> (Surfaces, Vec<String>) {
    let mut surfaces = Surfaces::default();
    let mut notes = Vec::new();

    match std::env::current_exe().map_err(|e| e.to_string()) {
        Ok(exe) => match help_output(&exe, &["--help"]) {
            Ok(help) => {
                surfaces.xtask = parse_clap_subcommands(&help);
                if surfaces.xtask.is_empty() {
                    surfaces
                        .surface_failures
                        .push("xtask --help produced no command surface".to_string());
                } else {
                    surfaces.present_binaries.insert("xtask".to_string());
                }
            }
            Err(why) => surfaces
                .surface_failures
                .push(format!("xtask --help could not be read: {why}")),
        },
        Err(why) => surfaces
            .surface_failures
            .push(format!("cannot resolve the xtask executable: {why}")),
    }

    for (binary, target) in [
        ("maos", "maos"),
        ("maosctl", "maosctl"),
        ("maos-registry-server", "maos-registry-server"),
        ("maos-spirit", "maos-spirit"),
        ("maos-wasm-runner", "maos-wasm-runner"),
    ] {
        let path = root.join("target/debug").join(target);
        if !path.exists() {
            notes.push(format!(
                "{binary}: not built at target/debug/{target} — treated as absent, so its tokens \
                 must be owed by a named open story"
            ));
            continue;
        }
        match help_output(&path, &["--help"]) {
            Ok(help) => {
                surfaces.present_binaries.insert(binary.to_string());
                match binary {
                    "maos" => {
                        surfaces.maos = parse_help_section(&help, "VERBS:");
                        surfaces.one_shot_modes = parse_help_section(&help, "MAOS_ONE_SHOT MODES:");
                        if surfaces.maos.is_empty() {
                            surfaces
                                .surface_failures
                                .push("maos --help produced no `VERBS:` section".to_string());
                        }
                        if surfaces.one_shot_modes.is_empty() {
                            surfaces.surface_failures.push(
                                "maos --help produced no `MAOS_ONE_SHOT MODES:` section"
                                    .to_string(),
                            );
                        }
                    }
                    "maosctl" => {
                        surfaces.maosctl = parse_clap_subcommands(&help);
                        if surfaces.maosctl.is_empty() {
                            surfaces
                                .surface_failures
                                .push("maosctl --help produced no command surface".to_string());
                        } else {
                            load_nested_clap_surfaces(&path, binary, &[], &help, &mut surfaces);
                        }
                    }
                    "maos-spirit" => {
                        let commands = parse_clap_subcommands(&help);
                        if commands.is_empty() {
                            surfaces
                                .surface_failures
                                .push("maos-spirit --help produced no command surface".to_string());
                        } else {
                            load_nested_clap_surfaces(&path, binary, &[], &help, &mut surfaces);
                        }
                    }
                    _ => {
                        let commands = parse_clap_subcommands(&help);
                        if !commands.is_empty() {
                            surfaces.subcommands.insert(binary.to_string(), commands);
                        }
                    }
                }
            }
            Err(why) => {
                surfaces
                    .unavailable_binaries
                    .insert(binary.to_string(), why);
            }
        }
    }
    (surfaces, notes)
}

pub fn run(json: bool) -> Result<(), String> {
    let root = workspace_root();
    let corpus = load_corpus(&root);
    let (surfaces, notes) = load_surfaces(&root);
    let result = audit(&corpus, &surfaces);

    for note in &notes {
        emit_command(json, "warning", &format!("{GATE_NAME}: {note}"));
    }

    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE_NAME,
                "passed": result.passed(),
                "epics_in_roster": result.epics_in_roster,
                "blocks_read": result.blocks_read,
                "tokens_resolved": result.tokens_resolved,
                "owed": result.owed,
                "findings": result.findings,
                "surface_notes": notes,
                "surfaces": {
                    "maos": surfaces.maos.len(),
                    "maosctl": surfaces.maosctl.len(),
                    "xtask": surfaces.xtask.len(),
                    "one_shot_modes": surfaces.one_shot_modes.len(),
                },
            })
        );
    }

    if result.passed() {
        if !json {
            println!(
                "{GATE_NAME}: PASS ({} epics, {} blocks, {} tokens resolved, {} owed)",
                result.epics_in_roster,
                result.blocks_read,
                result.tokens_resolved,
                result.owed.len()
            );
            for owed in &result.owed {
                println!(
                    "  owed: `{}` (epic-{}) → {} [{}], claimed in {}",
                    owed.token, owed.epic, owed.owner_story, owed.owner_status, owed.claimed_in
                );
            }
        }
        return Ok(());
    }

    let mut detail = String::new();
    for found in &result.findings {
        detail.push_str(&format!(
            "- [{:?}] {}:{} — {}\n",
            found.kind, found.file, found.line, found.detail
        ));
    }
    let msg = format!(
        "{GATE_NAME}: FAIL — {} finding(s) over {} block(s), {} token(s) resolved:\n{detail}",
        result.findings.len(),
        result.blocks_read,
        result.tokens_resolved
    );
    emit_command(json, "error", &msg);
    if !json {
        eprintln!("{msg}");
    }
    Err(msg)
}
