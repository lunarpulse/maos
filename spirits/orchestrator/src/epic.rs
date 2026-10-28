//! Story 19-1 (AC1.4, D-19-1-03 A) — the deterministic `--epic` split.
//!
//! Decision E kept for dispatch: the epic file gives the section COUNT and
//! each section's text; no model and no cassette decides either. The split is
//! strict:
//!
//! * everything before the first level-3 heading is preamble (the epic's own
//!   `#` / `##` title and prose) and is never dispatched;
//! * from the first level-3 heading on, every heading of level 1–3 opens a
//!   section, and every such heading must be a `### Story` heading — a section
//!   headed otherwise is refused typed, never folded into its neighbour or
//!   dropped (the AC1 proven-red);
//! * `####`-and-deeper headings and fenced code blocks belong to the section
//!   they sit in;
//! * section k binds to the k-th host-bearing topology entry, so a section
//!   count that differs from the entry count is refused before any emit.
//!
//! Heading recognition follows CommonMark ATX: a leading U+FEFF on the first
//! line is dropped (never part of any section's text), and a heading may be
//! indented 0–3 spaces — four or more (or a tab) is indented code, i.e. body
//! text. Setext headings (a line underlined with `===` / `---`) are NOT
//! recognised: their text line is section body, so an epic must head every
//! section with an ATX `### Story` line.

/// The heading that opens one dispatchable section.
pub const STORY_HEADING: &str = "### Story";

/// One `### Story` section of an epic, in file order. Section k (`ordinal`)
/// is dispatched to the k-th host-bearing topology entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpicSection {
    /// 1-based position in file order.
    pub ordinal: usize,
    /// 1-based line number of the section's `### Story` heading.
    pub line: usize,
    /// The heading text after the `### ` marker, trimmed (e.g. `Story 1: …`).
    pub heading: String,
    /// The section verbatim — its heading line through the line before the
    /// next section (or end of file), trailing whitespace trimmed. This is the
    /// Worker goal, byte for byte.
    pub text: String,
}

/// A typed refusal of an epic file. Every variant is raised before any
/// `task.assign` is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpicSplitError {
    /// The file has no level-3 heading at all: nothing to dispatch.
    NoStorySections,
    /// Section `section` (1-based, at `line`) is headed `heading`, which is
    /// not a `### Story` heading. `story_sections` counts the `### Story`
    /// headings the file does carry; `entries` is the host-bearing entry count.
    NotAStoryHeading {
        section: usize,
        line: usize,
        heading: String,
        story_sections: usize,
        entries: usize,
    },
    /// D-19-1-03 A — strict 1:1 in file order: the section count must equal
    /// the host-bearing entry count.
    CountMismatch { sections: usize, entries: usize },
}

impl std::fmt::Display for EpicSplitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoStorySections => {
                write!(
                    f,
                    "the epic has no `{STORY_HEADING}` section: nothing to dispatch"
                )
            }
            Self::NotAStoryHeading {
                section,
                line,
                heading,
                story_sections,
                entries,
            } => write!(
                f,
                "epic section {section} (line {line}) is headed `{heading}`, not \
                 `{STORY_HEADING}`: the epic splits into {story_sections} `{STORY_HEADING}` \
                 sections vs {entries} host-bearing entries"
            ),
            Self::CountMismatch { sections, entries } => write!(
                f,
                "the epic splits into {sections} `{STORY_HEADING}` sections vs {entries} \
                 host-bearing entries (one section per entry, in file order)"
            ),
        }
    }
}

impl std::error::Error for EpicSplitError {}

/// Split `markdown` into its ordered `### Story` sections and bind them 1:1
/// to `host_entries` host-bearing topology entries (D-19-1-03 A). Pure and
/// deterministic: the same bytes always yield the same sections or the same
/// refusal.
pub fn split_epic(markdown: &str, host_entries: usize) -> Result<Vec<EpicSection>, EpicSplitError> {
    let markdown = markdown.strip_prefix('\u{feff}').unwrap_or(markdown);
    // (byte offset, 1-based line, unindented heading line) of every
    // section-opening heading from the first level-3 heading on.
    let mut openers: Vec<(usize, usize, &str)> = Vec::new();
    let mut fence: Option<(u8, usize)> = None;
    let mut offset = 0;
    for (index, raw) in markdown.split_inclusive('\n').enumerate() {
        let start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        if let Some((marker, run)) = fence_run(line) {
            match fence {
                None => fence = Some((marker, run)),
                Some((open, len)) if open == marker && run >= len && is_closing(line, run) => {
                    fence = None
                }
                Some(_) => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let Some(line) = unindent(line) else { continue };
        match atx_level(line) {
            Some(3) => openers.push((start, index + 1, line)),
            Some(1 | 2) if !openers.is_empty() => openers.push((start, index + 1, line)),
            _ => {}
        }
    }
    if openers.is_empty() {
        return Err(EpicSplitError::NoStorySections);
    }
    let story_sections = openers
        .iter()
        .filter(|(_, _, line)| is_story_heading(line))
        .count();
    if let Some((section, (_, line, heading))) = openers
        .iter()
        .enumerate()
        .find(|(_, (_, _, line))| !is_story_heading(line))
    {
        return Err(EpicSplitError::NotAStoryHeading {
            section: section + 1,
            line: *line,
            heading: heading.trim_end().to_string(),
            story_sections,
            entries: host_entries,
        });
    }
    if openers.len() != host_entries {
        return Err(EpicSplitError::CountMismatch {
            sections: openers.len(),
            entries: host_entries,
        });
    }
    let ends = openers
        .iter()
        .skip(1)
        .map(|(start, _, _)| *start)
        .chain(std::iter::once(markdown.len()));
    Ok(openers
        .iter()
        .zip(ends)
        .enumerate()
        .map(|(index, ((start, line, heading), end))| EpicSection {
            ordinal: index + 1,
            line: *line,
            heading: heading[4..].trim().to_string(),
            text: markdown[*start..end].trim_end().to_string(),
        })
        .collect())
}

/// `line` without its 0–3 spaces of indent; `None` at four or more (indented
/// code, never a heading or a fence).
fn unindent(line: &str) -> Option<&str> {
    let trimmed = line.trim_start_matches(' ');
    (line.len() - trimmed.len() <= 3).then_some(trimmed)
}

/// The ATX heading level (1–6) of an unindented `line`, if it is one.
fn atx_level(line: &str) -> Option<usize> {
    let level = line.bytes().take_while(|b| *b == b'#').count();
    let rest = &line[level..];
    ((1..=6).contains(&level) && (rest.is_empty() || rest.starts_with([' ', '\t'])))
        .then_some(level)
}

/// An unindented `### Story`, then end of line, whitespace or `:`
/// (`### Stories` is not one).
fn is_story_heading(line: &str) -> bool {
    line.strip_prefix(STORY_HEADING)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', ':']))
}

/// A fence line (```` ``` ```` or `~~~`, up to three spaces of indent): its
/// marker byte and run length.
fn fence_run(line: &str) -> Option<(u8, usize)> {
    let trimmed = unindent(line)?;
    let marker = *trimmed.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let run = trimmed.bytes().take_while(|b| *b == marker).count();
    (run >= 3).then_some((marker, run))
}

/// A closing fence carries nothing after its marker run but whitespace.
fn is_closing(line: &str, run: usize) -> bool {
    line.trim_start_matches(' ')[run..].trim().is_empty()
}
