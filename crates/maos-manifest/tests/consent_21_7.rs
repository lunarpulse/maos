//! Story 21-7 (D-21-7-01 A) — the `[consent.recall]` declaration parser.

use maos_domain::frame::ConsentRecall;
use maos_manifest::ConsentSection;

#[test]
fn every_policy_parses_and_keeps_its_recipient() {
    let section = ConsentSection::from_toml_str(
        "[recall]\npeer-b = \"header_only\"\npeer-c = \"withheld\"\npeer-d = \"full\"\n",
    )
    .unwrap();
    assert_eq!(section.recall.len(), 3);
    assert_eq!(section.recall["peer-b"], ConsentRecall::HeaderOnly);
    assert_eq!(section.recall["peer-c"], ConsentRecall::Withheld);
    assert_eq!(section.recall["peer-d"], ConsentRecall::Full);
}

#[test]
fn malformed_declarations_are_refused_never_defaulted() {
    let long_id = "x".repeat(129);
    let too_many: String = (0..33).map(|n| format!("p{n} = \"full\"\n")).collect();
    for (label, body) in [
        (
            "unknown policy",
            "[recall]\nb = \"everything\"\n".to_string(),
        ),
        ("uppercase id", "[recall]\nB = \"full\"\n".to_string()),
        ("path id", "[recall]\n\"../b\" = \"full\"\n".to_string()),
        ("empty id", "[recall]\n\"\" = \"full\"\n".to_string()),
        ("long id", format!("[recall]\n{long_id} = \"full\"\n")),
        ("empty table", "[recall]\n".to_string()),
        ("missing table", "".to_string()),
        (
            "unknown key",
            "[recall]\nb = \"full\"\n[extra]\nx = 1\n".to_string(),
        ),
        ("non-string policy", "[recall]\nb = 1\n".to_string()),
        ("too many", format!("[recall]\n{too_many}")),
    ] {
        assert!(
            ConsentSection::from_toml_str(&body).is_err(),
            "{label} must be refused"
        );
    }
}
