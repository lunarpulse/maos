//! Story 22-9 (D-22-9-04 A) — `[consent]` has a schema floor: below it the table is refused
//! typed, never degraded away; at it the declaration parses as before.

use maos_domain::frame::ConsentRecall;
use maos_manifest::{ConsentSection, CONSENT_SINCE_SCHEMA};

const DECLARATION: &str = "[recall]\npeer-b = \"header_only\"\n";

#[test]
fn a_consent_section_below_schema_9_is_refused() {
    for schema in [1, 7, 8] {
        assert!(schema < CONSENT_SINCE_SCHEMA);
        let error = ConsentSection::from_toml_str_at(schema, DECLARATION)
            .expect_err("a [consent] table below the floor is refused, not ignored")
            .to_string();
        assert!(
            error.contains("consent") && error.contains(&format!("got {schema}")),
            "{error}"
        );
    }
}

#[test]
fn a_consent_section_at_schema_9_parses() {
    assert_eq!(
        CONSENT_SINCE_SCHEMA,
        maos_spirit_abi::MANIFEST_SCHEMA_VERSION
    );
    let section = ConsentSection::from_toml_str_at(CONSENT_SINCE_SCHEMA, DECLARATION)
        .expect("the floor schema admits the declaration");
    assert_eq!(section.recall["peer-b"], ConsentRecall::HeaderOnly);
}
