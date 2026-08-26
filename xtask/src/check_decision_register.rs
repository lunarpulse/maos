#![forbid(unsafe_code)]

//! Story `14-0` AC1 — **the decision register gets a reader.**
//!
//! The Epic-14 preflight decision register
//! (`_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md`)
//! declares as its binding rule 2 that *"deadlines are mechanical, not calendar
//! … so 'did we miss it' is a query, not a judgement."*
//!
//! At `9c5ae2db` that sentence was false. Grepping `epic-14-preflight-decisions`
//! across `*.rs` / `*.yml` / `*.toml` returned **two hits, both prose comments**
//! (`xtask/kloc.toml:208`, `xtask/src/gate_common.rs:12`). The residual gate
//! hard-codes `deferred-work.md` and walks only `_bmad-output/implementation-artifacts/`,
//! and the register lives in `planning-artifacts/epics/` — **outside every gate's
//! `STORY_DIR`**. There was no query and no queryer, so every deadline in the
//! register was a judgement. Eight of nineteen rows were already wrong at HEAD.
//! `D18` was marked `RESOLVED` with its substance unimplemented and its deadline
//! four stories in the past, and nothing noticed.
//!
//! This gate is the queryer. It asserts four things and nothing else — it never
//! judges whether a decision was *wise*:
//!
//! 1. **Every row declares a status** (`OPEN` / `CLOSED`) in its ID cell. The
//!    register had no expired-vs-resolved distinction at all, which is precisely
//!    how a `RESOLVED` tag hid an unimplemented, expired row.
//! 2. **Every Target-story cell resolves to a real `development_status` key**
//!    (AC1.1). An `epic-*` key is not a vehicle — that is the register's own
//!    founding defect. A retrospective action (`C3`, `C5`) is not a vehicle —
//!    *"owned by a retrospective is not an owner"* is the register's epigraph.
//!    A phrase that defers naming a vehicle is not a vehicle either.
//! 3. **An OPEN row whose deadline has passed REDS** (AC1.2). Deadline
//!    resolution reads `sprint-status.yaml` and nothing else: *"before X leaves
//!    `backlog`"* has passed when X's status is not `backlog`; *"before X reaches
//!    `done`"* has passed when X is `done`.
//! 4. **A deadline that is NON-MECHANICAL by construction must say so** (AC1.4).
//!    Three exist: a code event (`before any Epic 14 kernel-core edit`), a wave
//!    close with no observable transition, and *"before `j1-crosshost-2b` writes
//!    its first line"*. They are reported in their own `UNQUERYABLE` bucket and
//!    are **never counted as green** — but an *undeclared* one reds, so the next
//!    unqueryable deadline cannot be written silently.
//!
//! **FAILS CLOSED (AC1.3, Murat's condition at the round-table).** An unreadable
//! register, a table this cannot find, zero parsed rows, or a resolution that
//! yields zero targets is an `Err`, never a pass. `findings.is_empty()` is blind
//! to a gate that governs nothing, and a gate that governs nothing passes for the
//! wrong reason. The wording mirrors `gate_common.rs:72-79`, which is D19's own
//! fix for the same failure shape.
//!
//! **Binding class: Blocking, with no private `CURRENT_PHASE`.** AC4.6 measured
//! what a private phase const does to a gate: `check_escape_detector.rs` prints
//! `Escape-detector oracle RED` and exits 0 with `"passed": true`. This gate has
//! no advisory tail and no phase coupling — findings mean exit 1.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The gate name evidence and CI enrolment bind to.
pub const GATE: &str = "check-decision-register";

/// The register this gate reads. It lives OUTSIDE `STORY_DIR` — that is why no
/// story-file gate could ever have seen it.
pub const REGISTER: &str = "_bmad-output/planning-artifacts/epics/epic-14-preflight-decisions.md";

/// The literal a row must carry when its deadline cannot be resolved against a
/// `sprint-status.yaml` transition. Declaring it is the whole obligation; the
/// gate then reports the row as unqueryable rather than green.
pub const UNQUERYABLE: &str = "UNQUERYABLE";

/// Phrases that defer naming a vehicle. The register's founding defect was seven
/// rows pointing at `epic-14`; the defect then reproduced one level down as seven
/// rows pointing at *"14-0 decomposes into a named story"* — a decision vehicle
/// with a TBD target. Blocking condition 3 of story `14-0`: *"do not fix a row by
/// re-pointing it at another non-vehicle."*
const DEFERRED_NAMING: &[&str] = &[
    "decomposes into a named story",
    "implementation story assigned",
    "assigned by the ruling",
    "a story that will be named",
    "named later",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowStatus {
    Open,
    Closed,
}

/// How a deadline clause is anchored to an observable `sprint-status.yaml`
/// transition. Anything else is not mechanical and must be declared.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ClauseKind {
    LeavesBacklog,
    ReachesDone,
}

struct Clause {
    kind: Option<ClauseKind>,
    anchor: Option<String>,
    text: String,
}

struct Row {
    id: String,
    line: usize,
    status: Option<RowStatus>,
    target_cell: String,
    deadline_cell: String,
}

/// One thing wrong with one row. `kind` is a stable machine token so a consumer
/// can count classes without parsing prose.
#[derive(Debug)]
pub struct Finding {
    pub row: String,
    pub line: usize,
    pub kind: &'static str,
    pub detail: String,
}

// ── Parsing ────────────────────────────────────────────────────────────────

/// Split a markdown table row into its cells, dropping the leading and trailing
/// pipe. Cells are indexed FROM THE LEFT, so a row that carries extra trailing
/// columns (D13 and D15 both append a resolution column) still resolves its
/// canonical columns correctly.
fn cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    // Strip the two pipes INDEPENDENTLY. Chaining `.unwrap_or(trimmed)` off the
    // suffix strip reverts to the PRE-strip string when a row has no trailing
    // pipe — D13 has none, so it silently column-shifted and this gate dropped
    // the row entirely. A parser that drops a row is the vacuous-green failure
    // this gate exists to end, one level down.
    let inner = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    // MEASURED HAZARD, not a hypothetical: D19's Residual cell quotes the Rust
    // closure `name.starts_with(|c: char| c.is_ascii_digit())`, whose two literal
    // pipes live INSIDE a code span. A naive `split('|')` shifted that row's
    // columns by two and made this gate read the Decision cell as the deadline.
    // Split on pipes only OUTSIDE backtick spans.
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut in_code = false;
    for ch in inner.chars() {
        match ch {
            '`' => {
                in_code = !in_code;
                cell.push(ch);
            }
            '|' if !in_code => {
                out.push(cell.trim().to_string());
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }
    out.push(cell.trim().to_string());
    out
}

/// Locate the decisions table and return `(header_line_index, id_col, target_col,
/// deadline_col)`.
///
/// Columns are located BY NAME, not by position: a column inserted into the
/// register tomorrow must not silently re-point this gate at the wrong cell.
fn find_table(lines: &[&str]) -> Result<(usize, usize, usize, usize), String> {
    for (index, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with('|') {
            continue;
        }
        let header = cells(line);
        let column = |want: &str| {
            header
                .iter()
                .position(|c| c.to_ascii_lowercase().starts_with(want))
        };
        let (Some(id), Some(target), Some(deadline)) =
            (column("id"), column("target story"), column("deadline"))
        else {
            continue;
        };
        return Ok((index, id, target, deadline));
    }
    Err(format!(
        "{REGISTER}: no decisions table found (expected a header row carrying \
         `ID`, `Target story` and `Deadline`). Refusing to report green over a \
         register this gate could not parse (AC1.3)"
    ))
}

/// Parse every `D<n>` row of the decisions table.
///
/// Returns the rows AND a finding for every table row this could not name. A
/// row a parser silently drops is a row nothing governs, which is the vacuous
/// green this gate exists to end — one level down, inside the gate itself.
fn unparsable_row(line: usize, id_cell: &str) -> Finding {
    Finding {
        row: format!("line {line}"),
        line,
        kind: "unparsable-row",
        detail: format!(
            "table row whose ID cell {id_cell:?} yields no exact `D<n>` decision id; \
             a row this gate cannot name is a row it does not govern"
        ),
    }
}

fn parse_rows(
    lines: &[&str],
    header: usize,
    id_col: usize,
    target_col: usize,
    deadline_col: usize,
) -> (Vec<Row>, Vec<Finding>) {
    let mut rows = Vec::new();
    let mut malformed = Vec::new();
    for (offset, line) in lines.iter().enumerate().skip(header + 1) {
        let trimmed = line.trim_start();
        if !trimmed.starts_with('|') {
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.trim_start_matches('*').starts_with('D') {
                malformed.push(unparsable_row(offset + 1, trimmed));
                continue;
            }
            break;
        }
        let cells = cells(line);
        // The `|---|---|` alignment row is structure, not a decision.
        if cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
        {
            continue;
        }
        let id_cell = cells.get(id_col).map(String::as_str).unwrap_or_default();
        let Some(id) = decision_id(id_cell) else {
            malformed.push(unparsable_row(offset + 1, id_cell));
            continue;
        };
        rows.push(Row {
            id,
            line: offset + 1,
            status: declared_status(id_cell),
            target_cell: cells.get(target_col).cloned().unwrap_or_default(),
            deadline_cell: cells.get(deadline_col).cloned().unwrap_or_default(),
        });
    }
    (rows, malformed)
}

/// `**D13** · OPEN` → `D13`; `**D4a** · OPEN` → `D4a`.
///
/// The lowercase suffix is load-bearing: AC4.4 splits D4 into three obligations
/// with three different targets and three different deadlines. Truncating the
/// suffix would collapse them into one id and hide two of the three.
fn decision_id(cell: &str) -> Option<String> {
    let bare = cell.replace('*', "");
    let token = bare.split('·').next().unwrap_or(&bare).trim();
    let body = token.strip_prefix('D')?;
    let digit_end = body
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(body.len());
    let (digits, suffix) = body.split_at(digit_end);
    (!digits.is_empty() && matches!(suffix.as_bytes(), [] | [b'a'..=b'z']))
        .then(|| token.to_string())
}

/// `**D13** · OPEN` → `Open`. A row that declares nothing returns `None`, which
/// is a finding: an undeclared row is exactly how `D18` sat `RESOLVED` over an
/// unimplemented substance with an expired deadline.
fn declared_status(cell: &str) -> Option<RowStatus> {
    let (_, tail) = cell.split_once('·')?;
    if tail.contains('·') {
        return None;
    }
    match tail.trim().trim_matches('*').trim() {
        "OPEN" => Some(RowStatus::Open),
        "CLOSED" => Some(RowStatus::Closed),
        _ => None,
    }
}

/// Every single-token backtick or bold span. Shape validation happens later so
/// malformed formatted targets cannot disappear before the gate reports them.
fn candidate_tokens(cell: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |raw: &str| {
        let token = raw.trim().trim_matches('`').trim_matches(|c: char| {
            matches!(c, '.' | ',' | ';' | ':' | '(' | ')' | '*' | '"' | '\'')
        });
        if token.is_empty() || token.chars().any(char::is_whitespace) {
            return;
        }
        if !out.iter().any(|existing| existing == token) {
            out.push(token.to_string());
        }
    };
    for (index, span) in cell.split('`').enumerate() {
        if index % 2 == 1 {
            push(span);
        }
    }
    for (index, span) in cell.split("**").enumerate() {
        if index % 2 == 1 {
            push(span);
        }
    }
    out
}

/// Resolve a target token to a `development_status` key.
///
/// The rule is DELIBERATE, not the incidental prefix match at
/// `check_dev_record_completeness.rs:173-177`: exact key, else the *unique* key
/// beginning `<token>-`. An ambiguous short form resolves to nothing and is a
/// finding, because a register that names two possible vehicles has named none.
/// The register writes short forms (`14-4`) while `deferred-work.md` writes full
/// keys; both must resolve, and neither may resolve by accident.
fn resolve<'a>(keys: &'a BTreeSet<String>, token: &str) -> Option<&'a String> {
    if let Some(exact) = keys.get(token) {
        return Some(exact);
    }
    let mut matches = keys
        .iter()
        .filter(|key| key.strip_prefix(token).is_some_and(|s| s.starts_with('-')));
    let first = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(first)
}

/// A retrospective action row (`C3`, `C5`). It has no key, no file, and no owner
/// the tracker can page — the register's own epigraph rules it out as an owner,
/// and D1 and D11 both pointed their implementations at one anyway.
fn is_retro_action(token: &str) -> bool {
    let mut chars = token.chars();
    chars.next() == Some('C')
        && token.len() <= 3
        && chars.clone().count() > 0
        && chars.all(|c| c.is_ascii_digit())
}

fn valid_key_token(token: &str) -> bool {
    token.len() <= 80
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Is this formatted token a claim to be a TARGET, rather than an incidental
/// status or code identifier?
///
/// Exact/short story keys, epic keys and retro actions are always claims.
/// Unresolved hyphenated tokens are claims too: otherwise a valid first target
/// can hide a misspelled nonnumeric successor in the same cell. Sprint-status
/// values are annotations, while `MAOS_*` identifiers contain no hyphen.
fn is_target_shaped(token: &str, keys: &BTreeSet<String>) -> bool {
    resolve(keys, token).is_some()
        || token.starts_with("epic-")
        || is_retro_action(token)
        || !valid_key_token(token)
        || (token.contains('-') && !matches!(token, "in-progress" | "ready-for-dev"))
}

fn mechanical_deadline(
    clause: &str,
    keys: &BTreeSet<String>,
) -> (Option<ClauseKind>, Option<String>) {
    let lower = clause.to_ascii_lowercase();
    let Some(start) = lower.find("before").map(|index| index + "before".len()) else {
        return (None, None);
    };
    let after = &lower[start..];
    let transition = after
        .find(" leaves")
        .filter(|end| after[*end..].contains("backlog"))
        .map(|end| (end, ClauseKind::LeavesBacklog))
        .or_else(|| {
            after
                .find(" reaches")
                .filter(|end| after[*end..].contains("done"))
                .map(|end| (end, ClauseKind::ReachesDone))
        });
    let Some((end, kind)) = transition else {
        return (None, None);
    };
    let tokens = candidate_tokens(&clause[start..start + end]);
    let anchor = (tokens.len() == 1)
        .then(|| resolve(keys, &tokens[0]).cloned())
        .flatten();
    (Some(kind), anchor)
}

fn deadline_clauses(cell: &str, keys: &BTreeSet<String>) -> Vec<Clause> {
    cell.split(';')
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
        .map(|clause| {
            let (kind, anchor) = mechanical_deadline(clause, keys);
            Clause {
                kind,
                anchor,
                text: clause.to_string(),
            }
        })
        .collect()
}

/// Strip the trailing provenance comment the sprint-status parser leaves attached.
fn clean_status(raw: &str) -> &str {
    raw.split('#').next().unwrap_or(raw).trim()
}

/// Has the moment named by this clause gone by? This is the whole of binding
/// rule 2 — the tracker's own word, and no judgement.
fn has_passed(kind: ClauseKind, status: &str) -> bool {
    match kind {
        ClauseKind::LeavesBacklog => status != "backlog",
        ClauseKind::ReachesDone => status == "done",
    }
}

// ── The gate ───────────────────────────────────────────────────────────────

/// The gate's whole observation. Public so `xtask/tests/` can drive the real
/// audit over planted registers without a fixture filesystem — the vectors then
/// exercise the SAME code path production runs, not a parallel one.
#[derive(Debug)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub unqueryable: Vec<(String, String)>,
    pub rows: usize,
    pub open: usize,
    pub resolved_targets: usize,
}

/// Judge one register text against one story list and one status map.
///
/// **Fails closed at three separate points** (AC1.3, and the wording mirrors
/// `gate_common.rs`'s own fail-closed branch, which is D19's fix for this exact
/// failure shape): no findable table, zero parsed rows, or zero resolved
/// targets. Each is an `Err`, never a pass. `findings.is_empty()` cannot tell a
/// register that held from a register nothing read.
pub fn audit(
    register: &str,
    keys: &BTreeSet<String>,
    statuses: &BTreeMap<String, String>,
) -> Result<Report, String> {
    let lines: Vec<&str> = register.lines().collect();
    let (header, id_col, target_col, deadline_col) = find_table(&lines)?;
    let (rows, mut findings) = parse_rows(&lines, header, id_col, target_col, deadline_col);
    if rows.is_empty() {
        return Err(format!(
            "{REGISTER}: the decisions table parsed to ZERO rows, so this gate \
             governs nothing. Refusing to pass: a gate that governs nothing \
             passes for the wrong reason, and `findings.is_empty()` is blind to \
             it (AC1.3)"
        ));
    }

    let mut seen_ids = BTreeSet::new();

    let mut unqueryable: Vec<(String, String)> = Vec::new();
    let mut resolved_targets = 0usize;
    let mut open = 0usize;

    for row in &rows {
        if !seen_ids.insert(row.id.as_str()) {
            findings.push(Finding {
                row: row.id.clone(),
                line: row.line,
                kind: "duplicate-id",
                detail: String::from("duplicate decision ID; obligations need unique identities"),
            });
        }
        let status = match row.status {
            Some(status) => {
                if status == RowStatus::Open {
                    open += 1;
                }
                status
            }
            None => {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "undeclared-status",
                    detail: "the ID cell declares neither `· OPEN` nor `· CLOSED`, so \
                             expired and resolved are indistinguishable — the exact \
                             blindness that let D18 sit RESOLVED over an unimplemented \
                             substance with a deadline four stories in the past"
                        .to_string(),
                });
                // An undeclared row is treated as OPEN for the deadline check:
                // failing closed means assuming the obligation still stands.
                open += 1;
                RowStatus::Open
            }
        };

        // ── AC1.1 — the Target story cell ──────────────────────────────────
        let tokens: Vec<String> = candidate_tokens(&row.target_cell)
            .into_iter()
            .filter(|token| is_target_shaped(token, keys))
            .collect();
        let mut row_resolved = 0usize;
        for token in &tokens {
            if token.starts_with("epic-") {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "epic-target",
                    detail: format!(
                        "target `{token}` is an EPIC key, not a vehicle — the register's \
                         own founding defect (seven rows pointed at `epic-14`)"
                    ),
                });
            } else if is_retro_action(token) {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "retro-action-target",
                    detail: format!(
                        "target `{token}` is a retrospective action: no key, no file, no \
                         owner the tracker can page. \"Owned by a retrospective is not an \
                         owner\" is this register's own epigraph"
                    ),
                });
            } else if resolve(keys, token).is_some() {
                row_resolved += 1;
            } else {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "unresolvable-target",
                    detail: format!(
                        "target `{token}` resolves to no `development_status` key \
                         (exact, or a UNIQUE `{token}-…` expansion)"
                    ),
                });
            }
        }
        for phrase in DEFERRED_NAMING {
            if row.target_cell.to_ascii_lowercase().contains(phrase) {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "deferred-naming",
                    detail: format!(
                        "target defers naming a vehicle (\"{phrase}\") — a decision \
                         vehicle with a TBD target is the founding defect one level down"
                    ),
                });
            }
        }
        if row_resolved == 0 {
            findings.push(Finding {
                row: row.id.clone(),
                line: row.line,
                kind: "no-vehicle",
                detail: format!(
                    "Target story cell resolves to NO story key: {:?}",
                    row.target_cell
                ),
            });
        }
        resolved_targets += row_resolved;

        // ── AC1.2 / AC1.4 — the deadline ───────────────────────────────────
        let clauses = deadline_clauses(&row.deadline_cell, keys);
        if clauses.is_empty() {
            findings.push(Finding {
                row: row.id.clone(),
                line: row.line,
                kind: "no-deadline",
                detail: "the Deadline cell is empty — binding rule 2 requires a \
                         mechanical anchor or a declared UNQUERYABLE"
                    .to_string(),
            });
        }
        for clause in &clauses {
            let Some(kind) = clause.kind else {
                if clause.text.contains(UNQUERYABLE) {
                    unqueryable.push((row.id.clone(), clause.text.clone()));
                } else {
                    findings.push(Finding {
                        row: row.id.clone(),
                        line: row.line,
                        kind: "undeclared-unqueryable",
                        detail: format!(
                            "deadline clause {:?} is not resolvable against any \
                             `sprint-status.yaml` transition and is not declared \
                             `{UNQUERYABLE}`. Binding rule 2 promises a query; an \
                             unqueryable deadline that does not say so is counted \
                             green by every reader",
                            clause.text
                        ),
                    });
                }
                continue;
            };
            let Some(anchor) = clause.anchor.as_ref() else {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "unresolvable-anchor",
                    detail: format!(
                        "deadline clause {:?} names a transition but its anchor story \
                         resolves to no `development_status` key",
                        clause.text
                    ),
                });
                continue;
            };
            let anchor_status = statuses
                .get(anchor)
                .map(|s| clean_status(s))
                .unwrap_or("unknown");
            if status == RowStatus::Open && has_passed(kind, anchor_status) {
                findings.push(Finding {
                    row: row.id.clone(),
                    line: row.line,
                    kind: "expired-and-open",
                    detail: format!(
                        "row is OPEN and its deadline has PASSED: `{anchor}` is \
                         `{anchor_status}` and the clause was {:?}",
                        clause.text
                    ),
                });
            }
        }
    }

    if resolved_targets == 0 {
        return Err(format!(
            "{REGISTER}: {} row(s) parsed but ZERO target stories resolved, so no \
             row is governed by anything. Refusing to pass (AC1.3)",
            rows.len()
        ));
    }

    Ok(Report {
        findings,
        unqueryable,
        rows: rows.len(),
        open,
        resolved_targets,
    })
}

pub fn run(json: bool) -> Result<(), String> {
    let story_dir = Path::new(crate::gate_common::STORY_DIR);
    // FAIL CLOSED, twice over: an unreadable or empty story list is an `Err`
    // inside `governed_story_keys` itself (D19's own fix), and an unreadable
    // register is an `Err` here.
    let keys = crate::gate_common::governed_story_keys(story_dir)?;
    let statuses: BTreeMap<String, String> = crate::sprint_status::load_sprint_status(
        &story_dir.join("sprint-status.yaml").to_string_lossy(),
    )
    .into_iter()
    .collect();

    let register = std::fs::read_to_string(REGISTER).map_err(|e| {
        format!(
            "{GATE}: cannot read the decision register {REGISTER}: {e}. Refusing to \
             report green over a register nothing read (AC1.3)"
        )
    })?;

    let report = audit(&register, &keys, &statuses)?;

    if json {
        let findings: Vec<serde_json::Value> = report
            .findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "row": f.row,
                    "line": f.line,
                    "kind": f.kind,
                    "detail": f.detail,
                })
            })
            .collect();
        let unqueryable: Vec<serde_json::Value> = report
            .unqueryable
            .iter()
            .map(|(row, text)| serde_json::json!({ "row": row, "deadline": text }))
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE,
                "register": REGISTER,
                "rows": report.rows,
                "open_rows": report.open,
                "resolved_targets": report.resolved_targets,
                // Reported in its own bucket and NEVER counted green (AC1.4).
                "unqueryable_deadlines": unqueryable,
                "findings": findings,
                "passed": report.findings.is_empty(),
            })
        );
    } else {
        println!("{GATE}: {} rows, {} open", report.rows, report.open);
        for (row, text) in &report.unqueryable {
            println!("  {row}: deadline UNQUERYABLE (not green, not satisfied) — {text}");
        }
        for finding in &report.findings {
            println!(
                "  {}:{} [{}] {}",
                finding.row, finding.line, finding.kind, finding.detail
            );
        }
    }

    if report.findings.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{GATE}: {} finding(s) in {REGISTER}",
            report.findings.len()
        ))
    }
}
