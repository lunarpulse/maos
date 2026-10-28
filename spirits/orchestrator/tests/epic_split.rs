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

/// TI-5 — `~~~` fences hide headings exactly like backtick fences, and a
/// backtick line inside a tilde fence does not close it.
#[test]
fn a_tilde_fence_hides_its_headings_and_ignores_backtick_lines() {
    let epic = "### Story 1\n~~~\n```\n### Notes\n~~~\n### Story 2\nB\n";
    let sections = split_epic(epic, 2).expect("headings inside `~~~` are body text");
    assert_eq!(sections[0].text, "### Story 1\n~~~\n```\n### Notes\n~~~");
    assert_eq!(sections[1].line, 6);
}

/// TI-5 — a fence line with text after its run is not a closing fence, so the
/// heading after it stays inside the fence.
#[test]
fn a_closing_fence_with_trailing_text_does_not_close() {
    let epic = "### Story 1\n```\n``` not a close\n### Notes\n```\n### Story 2\nB\n";
    let sections = split_epic(epic, 2).expect("the fence closes only at the bare run");
    assert_eq!(
        sections[0].text,
        "### Story 1\n```\n``` not a close\n### Notes\n```"
    );
    assert_eq!(sections[1].heading, "Story 2");
}

/// TI-5 — a longer fence wraps a shorter one: the inner ```` ``` ```` neither
/// opens nor closes anything.
#[test]
fn a_longer_fence_wraps_a_shorter_one() {
    let epic = "### Story 1\n````md\n```\n### Notes\n```\n````\n### Story 2\nB\n";
    let sections = split_epic(epic, 2).expect("the four-tick fence closes only at four ticks");
    assert_eq!(
        sections[0].text,
        "### Story 1\n````md\n```\n### Notes\n```\n````"
    );
    assert_eq!(sections[1].line, 7);
}

/// TI-5 — an unclosed fence swallows every later heading to end of file; the
/// split fails closed through the count check rather than dispatching it.
#[test]
fn an_unclosed_fence_swallows_later_headings_and_fails_closed() {
    let epic = "### Story 1\nA\n```\n### Story 2\nB\n### Story 3\nC\n";
    assert_eq!(
        split_epic(epic, 3),
        Err(EpicSplitError::CountMismatch {
            sections: 1,
            entries: 3,
        })
    );
}

/// TI-5 — a fence line indented four spaces is indented code, not a fence, so
/// the heading after it still opens a section.
#[test]
fn a_four_space_indented_fence_line_opens_no_fence() {
    let epic = "### Story 1\n    ```\n### Story 2\nB\n";
    let sections = split_epic(epic, 2).expect("an indented-code ``` line is body text");
    assert_eq!(sections[0].text, "### Story 1\n    ```");
    assert_eq!(sections[1].line, 3);
}

/// TI-5 — a leading U+FEFF on the first line is dropped: the first heading is
/// recognised and no section's text carries the BOM. A BOM on a later line is
/// not stripped, so that line is body text.
#[test]
fn a_bom_before_the_first_heading_is_dropped() {
    let epic = "\u{feff}### Story 1\nA\n\u{feff}### Story 9\n### Story 2\nB\n";
    let sections = split_epic(epic, 2).expect("a BOM-prefixed epic splits");
    assert_eq!(sections[0].line, 1);
    assert_eq!(sections[0].heading, "Story 1");
    assert_eq!(sections[0].text, "### Story 1\nA\n\u{feff}### Story 9");
    assert_eq!(sections[1].line, 4);
    assert_eq!(
        split_epic("\u{feff}### Notes\nA\n", 1),
        Err(EpicSplitError::NotAStoryHeading {
            section: 1,
            line: 1,
            heading: "### Notes".into(),
            story_sections: 0,
            entries: 1,
        })
    );
}

/// TI-5 — CommonMark ATX: a heading indented 1–3 spaces is a heading, so a
/// non-Story one is refused and a Story one opens a section; four spaces is
/// indented code and stays body text.
#[test]
fn a_heading_indented_one_to_three_spaces_is_still_a_heading() {
    for indent in [" ", "  ", "   "] {
        let epic = format!("### Story 1\nA\n{indent}### Notes\nB\n");
        assert_eq!(
            split_epic(&epic, 1),
            Err(EpicSplitError::NotAStoryHeading {
                section: 2,
                line: 3,
                heading: "### Notes".into(),
                story_sections: 1,
                entries: 1,
            }),
            "`{indent}### Notes` is a level-3 heading"
        );
        let epic = format!("### Story 1\nA\n{indent}## Appendix\nB\n");
        assert!(
            matches!(
                split_epic(&epic, 1),
                Err(EpicSplitError::NotAStoryHeading { section: 2, .. })
            ),
            "`{indent}## Appendix` is a level-2 heading"
        );
        let epic = format!("{indent}### Story 1\nA\n{indent}### Story 2\nB\n");
        let sections = split_epic(&epic, 2).expect("an indented `### Story` opens a section");
        assert_eq!(sections[0].heading, "Story 1");
        assert_eq!(sections[0].text, format!("{indent}### Story 1\nA"));
        assert_eq!(sections[1].line, 3);
        assert_eq!(sections[1].heading, "Story 2");
    }
    let epic = "### Story 1\nA\n    ### Notes\n    ### Story 2\nB\n";
    let sections = split_epic(epic, 1).expect("four spaces is indented code, never a heading");
    assert_eq!(
        sections[0].text,
        "### Story 1\nA\n    ### Notes\n    ### Story 2\nB"
    );
    assert_eq!(
        split_epic("    ### Story 1\nA\n", 1),
        Err(EpicSplitError::NoStorySections)
    );
}
