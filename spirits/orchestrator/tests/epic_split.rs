#![forbid(unsafe_code)]

//! Story 19-1 (AC1.4, AC1.6; D-19-1-03 A) — the deterministic `--epic` split:
//! one section per `### Story` heading in file order, bound 1:1 to the
//! host-bearing entries; a section headed otherwise and a count mismatch are
//! typed refusals raised before any frame exists.

use orchestrator::{split_epic, EpicSection, EpicSplitError, STORY_HEADING};

const EPIC: &str = "\
# Epic J1 — founder loop

Preamble prose is never dispatched.

## Stories

### Story 1: parse the input
Accept a CSV file.

#### Acceptance
- rows parse

### Story 2: validate the schema
```text
### Story inside a fence is body text
```
Reject unknown columns.

### Story 3: persist the record
Write one row per input line.
";

#[test]
fn three_story_sections_split_in_file_order_byte_for_byte() {
    let sections = split_epic(EPIC, 3).expect("a well-formed three-story epic splits");
    assert_eq!(
        sections,
        vec![
            EpicSection {
                ordinal: 1,
                line: 7,
                heading: "Story 1: parse the input".into(),
                text: "### Story 1: parse the input\nAccept a CSV file.\n\n#### Acceptance\n- rows parse"
                    .into(),
            },
            EpicSection {
                ordinal: 2,
                line: 13,
                heading: "Story 2: validate the schema".into(),
                text: "### Story 2: validate the schema\n```text\n### Story inside a fence is body \
                       text\n```\nReject unknown columns."
                    .into(),
            },
            EpicSection {
                ordinal: 3,
                line: 19,
                heading: "Story 3: persist the record".into(),
                text: "### Story 3: persist the record\nWrite one row per input line.".into(),
            },
        ]
    );
    // Decision E: the same bytes always give the same split.
    assert_eq!(split_epic(EPIC, 3), Ok(sections));
}

#[test]
fn a_one_section_epic_keeps_a_non_ascii_goal_byte_for_byte() {
    let epic = "### Story 1: sentinel — ünïcødé ✓ 目標\nKeep every byte.\n\n\n";
    let sections = split_epic(epic, 1).expect("one section, one entry");
    assert_eq!(sections.len(), 1);
    assert_eq!(
        sections[0].text,
        "### Story 1: sentinel — ünïcødé ✓ 目標\nKeep every byte."
    );
    assert_eq!(sections[0].heading, "Story 1: sentinel — ünïcødé ✓ 目標");
}

#[test]
fn crlf_line_endings_split_like_lf_and_keep_interior_bytes() {
    let epic = "### Story 1\r\nA\r\n### Story 2\r\nB\r\n";
    let sections = split_epic(epic, 2).expect("CRLF epic splits");
    assert_eq!(sections[0].heading, "Story 1");
    assert_eq!(sections[0].text, "### Story 1\r\nA");
    assert_eq!(sections[1].line, 3);
    assert_eq!(sections[1].text, "### Story 2\r\nB");
}

/// AC1.6 proven-red — the second heading is not `### Story`: typed refusal
/// naming 2 sections vs 3 entries; the `### Notes` body is neither folded into
/// section 1 nor dropped.
#[test]
fn a_section_not_headed_story_is_refused_naming_both_counts() {
    let epic = "\
### Story 1: parse
Accept a CSV file.

### Notes
Remember the schema.

### Story 3: persist
Write rows.
";
    let err = split_epic(epic, 3).expect_err("a `### Notes` section must refuse");
    assert_eq!(
        err,
        EpicSplitError::NotAStoryHeading {
            section: 2,
            line: 4,
            heading: "### Notes".into(),
            story_sections: 2,
            entries: 3,
        }
    );
    let message = err.to_string();
    assert!(
        message.contains("2 `### Story` sections vs 3 host-bearing entries"),
        "the refusal names both counts: {message}"
    );
    assert!(message.contains("`### Notes`"), "{message}");
}

#[test]
fn a_level_one_or_two_heading_after_the_first_section_is_refused() {
    let epic = "### Story 1\nA\n## Appendix\nB\n";
    assert_eq!(
        split_epic(epic, 1),
        Err(EpicSplitError::NotAStoryHeading {
            section: 2,
            line: 3,
            heading: "## Appendix".into(),
            story_sections: 1,
            entries: 1,
        })
    );
}

#[test]
fn a_first_level_three_heading_that_is_not_story_is_refused() {
    let epic = "# Epic\n### Overview\nprose\n### Story 1\nA\n";
    assert!(matches!(
        split_epic(epic, 1),
        Err(EpicSplitError::NotAStoryHeading {
            section: 1,
            line: 2,
            story_sections: 1,
            ..
        })
    ));
}

#[test]
fn only_an_exact_story_heading_opens_a_section() {
    for heading in ["### Stories", "### Storyline", "###Story 1", "#### Story 1"] {
        let epic = format!("### Story 1\nA\n{heading}\nB\n");
        let result = split_epic(&epic, 1);
        if heading.starts_with("### ") {
            assert!(
                matches!(
                    result,
                    Err(EpicSplitError::NotAStoryHeading { section: 2, .. })
                ),
                "`{heading}` is a level-3 heading but not `{STORY_HEADING}`: {result:?}"
            );
        } else {
            let sections = result.expect("a non-level-3 line is section body");
            assert_eq!(sections[0].text, format!("### Story 1\nA\n{heading}\nB"));
        }
    }
    for heading in ["### Story", "### Story: colon", "### Story\t7"] {
        let epic = format!("{heading}\nA\n");
        assert_eq!(
            split_epic(&epic, 1).map(|s| s.len()),
            Ok(1),
            "`{heading}` opens a section"
        );
    }
}

/// D-19-1-03 A — strict 1:1: a count mismatch refuses in either direction.
#[test]
fn a_section_count_other_than_the_entry_count_is_refused() {
    assert_eq!(
        split_epic(EPIC, 2),
        Err(EpicSplitError::CountMismatch {
            sections: 3,
            entries: 2,
        })
    );
    let err = split_epic(EPIC, 4).expect_err("3 sections cannot bind 4 entries");
    assert_eq!(
        err,
        EpicSplitError::CountMismatch {
            sections: 3,
            entries: 4,
        }
    );
    assert!(err
        .to_string()
        .contains("3 `### Story` sections vs 4 host-bearing entries"));
    assert!(matches!(
        split_epic(EPIC, 0),
        Err(EpicSplitError::CountMismatch { entries: 0, .. })
    ));
}

#[test]
fn an_epic_with_no_story_section_is_refused() {
    for epic in ["", "# Epic\n\nprose only\n", "```\n### Story 1\n```\n"] {
        assert_eq!(
            split_epic(epic, 3),
            Err(EpicSplitError::NoStorySections),
            "{epic:?}"
        );
    }
}
