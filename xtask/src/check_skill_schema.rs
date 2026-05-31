#![forbid(unsafe_code)]

//! Gate — `check-skill-schema` (Story 7.4 AC6).
//!
//! Asserts the `maos.skill.v1` schema posture as executable invariants:
//!   1. a valid skill round-trips (`parse_skill` → Ok);
//!   2. an unknown frontmatter field rejects with `ESkillSchema::UnknownField`
//!      (the `#[serde(deny_unknown_fields)]` posture — NO silent default);
//!   3. a non-semver `version` rejects with `ESkillSchema::InvalidSemver`.
//!
//! These are the load-bearing schema-precision invariants the discipline floor
//! depends on: a regression that silently coerces an unknown field, or accepts
//! a non-semver version, would be a CORRECTNESS bug, not a style nit.

use maos_skill::{parse_skill, ESkillSchema};

const VALID: &str = "---\nid = \"check.valid\"\nversion = \"1.2.3\"\nname = \"Valid\"\ndescription = \"d\"\n---\nBody present.\n";
const UNKNOWN_FIELD: &str = "---\nid = \"check.unknown\"\nversion = \"1.0.0\"\nname = \"X\"\ndescription = \"d\"\nbogus = 1\n---\nbody\n";
const BAD_SEMVER: &str = "---\nid = \"check.semver\"\nversion = \"not-semver\"\nname = \"X\"\ndescription = \"d\"\n---\nbody\n";

pub fn run(json: bool) -> Result<(), String> {
    let mut failures: Vec<String> = Vec::new();

    // 1. Valid skill round-trips.
    match parse_skill(VALID) {
        Ok(skill) => {
            if skill.manifest.id != "check.valid" {
                failures.push(format!("valid skill parsed but id wrong: {}", skill.manifest.id));
            }
        }
        Err(e) => failures.push(format!("valid skill failed to parse: {e}")),
    }

    // 2. Unknown field → ESkillSchema::UnknownField (deny_unknown_fields posture).
    match parse_skill(UNKNOWN_FIELD) {
        Err(ESkillSchema::UnknownField(_)) => {}
        Err(other) => failures.push(format!(
            "unknown-field skill rejected with wrong variant: {other:?} (want UnknownField)"
        )),
        Ok(_) => failures.push(
            "unknown-field skill PARSED — deny_unknown_fields posture broken (silent default!)"
                .into(),
        ),
    }

    // 3. Non-semver version → ESkillSchema::InvalidSemver.
    match parse_skill(BAD_SEMVER) {
        Err(ESkillSchema::InvalidSemver(_, _)) => {}
        Err(other) => failures.push(format!(
            "non-semver skill rejected with wrong variant: {other:?} (want InvalidSemver)"
        )),
        Ok(_) => failures.push("non-semver version PARSED — semver validation broken".into()),
    }

    let passed = failures.is_empty();
    if json {
        let payload = serde_json::json!({
            "gate": "check-skill-schema",
            "passed": passed,
            "checks": 3,
            "failures": failures,
        });
        println!("{payload}");
    } else if passed {
        eprintln!("check-skill-schema: PASS (maos.skill.v1 round-trip + deny_unknown_fields + semver)");
    } else {
        for f in &failures {
            eprintln!("  [FAIL] {f}");
        }
        eprintln!("check-skill-schema: FAIL");
    }

    if passed {
        Ok(())
    } else {
        Err("maos.skill.v1 schema posture violated".into())
    }
}
