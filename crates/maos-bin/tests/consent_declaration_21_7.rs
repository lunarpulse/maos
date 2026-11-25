#![cfg(feature = "network")]
//! Story 21-7 (D-21-7-01 A) — admission reads and validates the `[consent.recall]` declaration.
use maos_bin::admission::gate_manifest;
use maos_domain::frame::ConsentRecall;

fn manifest(section: &str) -> String {
    format!(
        r#"[class]
name = "butler"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 9
min_substrate_version = "0.1.0"
forms = ["rust-inproc"]
trust_tier = "local"
description = "consent witness"
[resources]
cpu_max_pct = 10
memory_max_mb = 64
[output_shape]
required_fields = ["result"]
[posture]
default = "assistive"
allowed_max = "assistive"
[sandbox]
tier = "T0"
{section}
"#
    )
}

#[test]
fn a_declaration_is_carried_and_an_absent_one_declares_nothing() {
    let gated = gate_manifest(
        &manifest("[consent.recall]\npeer-b = \"header_only\"\npeer-c = \"withheld\""),
        &|_| true,
    )
    .unwrap();
    let consent = gated.consent.expect("declaration carried to admission");
    assert_eq!(consent.recall["peer-b"], ConsentRecall::HeaderOnly);
    assert_eq!(consent.recall["peer-c"], ConsentRecall::Withheld);
    assert!(gate_manifest(&manifest(""), &|_| true)
        .unwrap()
        .consent
        .is_none());
}

#[test]
fn a_malformed_declaration_is_refused_typed_at_the_gate() {
    for section in [
        "[consent.recall]\npeer-b = \"everything\"",
        "[consent.recall]\n\"Peer B\" = \"full\"",
        "[consent.recall]",
        "[consent]\nrecall = { peer-b = \"full\" }\nunknown = true",
    ] {
        let refusal = gate_manifest(&manifest(section), &|_| true).unwrap_err();
        assert_eq!(refusal.code(), "manifest_section_invalid", "{section}");
        assert!(refusal.to_string().contains("consent"), "{refusal}");
    }
}

#[test]
fn a_cold_swap_may_only_keep_or_narrow_the_declared_consent() {
    use maos_bin::admission::widened_consent;
    let declared = |rows: &str| manifest(&format!("[consent.recall]\n{rows}"));
    let old = declared("peer-b = \"header_only\"\npeer-c = \"withheld\"");
    assert!(widened_consent(&old, &old).is_none());
    assert!(widened_consent(&old, &declared("peer-b = \"withheld\"")).is_none());
    assert!(
        widened_consent(&old, &manifest("")).is_some(),
        "dropping it is accept-all"
    );
    assert!(widened_consent(&old, &declared("peer-b = \"full\"")).is_some());
    assert!(widened_consent(&old, &declared("peer-d = \"withheld\"")).is_some());
    assert!(
        widened_consent(&manifest(""), &old).is_none(),
        "narrowing an accept-all"
    );
    assert!(
        widened_consent(&old, "not toml [").is_none(),
        "the admission gate refuses it itself"
    );
}

// Story 22-9 (D-22-9-04 A) — `[consent]` has a schema floor and a recipient roster.
#[test]
fn a_consent_table_below_schema_9_is_refused_typed_at_the_gate() {
    let below = manifest("[consent.recall]\npeer-b = \"full\"")
        .replace("manifest_schema_version = 9", "manifest_schema_version = 8");
    let refusal = gate_manifest(&below, &|_| true).unwrap_err();
    assert_eq!(refusal.code(), "manifest_section_invalid");
    assert!(
        refusal.to_string().contains("consent") && refusal.to_string().contains(">= 9"),
        "{refusal}"
    );
}

#[test]
fn a_consent_recipient_the_host_does_not_know_is_refused_typed() {
    use maos_bin::admission::check_consent_recipients;
    let gated = gate_manifest(
        &manifest("[consent.recall]\nstranger = \"full\"\nbutler = \"withheld\""),
        &|_| true,
    )
    .unwrap();
    let refusal = check_consent_recipients(&gated, &|_| false).unwrap_err();
    assert_eq!(refusal.code(), "consent_recipient_unknown");
    assert!(refusal.to_string().contains("stranger"), "{refusal}");
    // Its own id always counts; a known recipient is admitted.
    assert!(check_consent_recipients(&gated, &|id| id == "stranger").is_ok());
}

#[test]
fn a_recipient_installed_after_its_sender_is_admitted() {
    use maos_bin::admission::{check_consent_recipients, declare_known_spirits, host_knows};
    let gated = gate_manifest(
        &manifest("[consent.recall]\nlate-installed-peer = \"header_only\""),
        &|_| true,
    )
    .unwrap();
    let roster = |id: &str| host_knows(id, &|_| false);
    assert!(
        check_consent_recipients(&gated, &roster).is_err(),
        "nobody knows the recipient yet"
    );
    // The recipient is declared (installed, or a cast member) after the sender was first judged.
    declare_known_spirits(["late-installed-peer".to_string()]);
    assert!(
        check_consent_recipients(&gated, &roster).is_ok(),
        "load order never decides the roster"
    );
}
