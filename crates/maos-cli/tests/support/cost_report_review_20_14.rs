use super::*;
use maos_domain::cost::{
    AttributionConfidence, AttributionSource, CostAttributionPayload, CostDimension, CostFrameBody,
    PrincipalRef, ProviderPricingConfig, ProviderPricingEntry,
};
use std::collections::BTreeMap;

fn entry(
    boot: u64,
    task: Option<&str>,
    schema: u16,
    principal: PrincipalRef,
    input: i64,
) -> maos_audit::AuditEntry {
    let payload = CostAttributionPayload {
        schema_version: schema,
        timestamp_ns: 10,
        spirit_pid: 7,
        provider: "provider".into(),
        model: "model".into(),
        principal,
        attribution_source: AttributionSource::WriteTargetProxy,
        attribution_confidence: AttributionConfidence::Exact,
        dimensions: [
            (CostDimension::TokensIn, input),
            (CostDimension::TokensOut, input),
        ]
        .into_iter()
        .collect(),
    };
    maos_audit::AuditEntry {
        frame_id_hex: format!("{boot:032x}"),
        timestamp_ns: 10,
        spirit_pid: 7,
        boot_nonce: boot,
        capability_token_hex: None,
        kind: "cost.attribution".into(),
        intent: "cost:inference-attribution".into(),
        payload: serde_json::to_string(&CostFrameBody {
            payload,
            task_ref: task.map(str::to_owned),
        })
        .unwrap(),
        redaction: None,
    }
}
fn pricing(rate: u64) -> ProviderPricingConfig {
    ProviderPricingConfig::new(vec![ProviderPricingEntry {
        provider: "provider".into(),
        model: "model".into(),
        input_price_micro_per_1k: rate,
        output_price_micro_per_1k: rate,
    }])
}
fn resolved(name: &str) -> PrincipalRef {
    PrincipalRef::Resolved {
        principal_id: name.into(),
    }
}

#[test]
fn names_and_fractions_are_boot_local_and_unnamed_falls_back() {
    let names = [
        ((1, 7), vec![(1, "alpha".into()), (20, "future".into())]),
        ((2, 7), vec![(1, "beta".into())]),
    ]
    .into_iter()
    .collect();
    let rows = [
        entry(1, Some("same"), 2, resolved("alice"), 1000),
        entry(1, Some("same"), 2, resolved("alice"), 1000),
        entry(2, Some("same"), 2, resolved("alice"), 1000),
        entry(3, Some("same"), 2, PrincipalRef::Unattributed, 1000),
    ];
    let report = build_cost_report("2026-06", &rows, &pricing(1000), &names);
    assert_eq!(report.rows.len(), 3);
    assert!(report
        .rows
        .iter()
        .any(|r| r.spirit == "alpha" && r.tokens_in == 2000));
    assert!(report
        .rows
        .iter()
        .any(|r| r.spirit == "beta" && r.tokens_in == 1000));
    assert!(report
        .rows
        .iter()
        .any(|r| r.spirit == "pid:7" && r.tokens_in == 1000));
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(
        json["per_spirit_attributable_fraction"],
        serde_json::json!({"1:7": 1.0, "2:7": 1.0, "3:7": 0.0})
    );
}

#[test]
fn report_rejects_pid_mismatch_and_distinguishes_absent_task() {
    let mut mismatch = entry(4, Some("bad"), 2, resolved("alice"), 1);
    mismatch.spirit_pid = 8;
    let literal = entry(3, Some("none"), 2, resolved("alice"), 1);
    let rows = [
        entry(1, None, 1, resolved("alice"), 1),
        entry(2, None, 2, resolved("alice"), 1),
        literal,
        mismatch,
    ];
    let report = build_cost_report("2026-06", &rows, &pricing(1), &BTreeMap::new());
    assert_eq!(report.rows.len(), 3);
    let tasks: std::collections::BTreeSet<_> =
        report.rows.iter().map(|r| r.task.as_str()).collect();
    assert_eq!(tasks.len(), 3);
    assert!(!report.warnings.is_empty());
    let long = "x".repeat(10000);
    let report = build_cost_report(
        "2026-06",
        &[entry(1, Some(&long), 2, resolved("alice"), 1)],
        &pricing(1),
        &BTreeMap::new(),
    );
    assert!(report.rows.is_empty() && report.warnings.len() == 1);
}

#[test]
fn plain_report_escapes_control_characters_and_caps_fields() {
    let hostile = "\u{1b}[31m\n\u{7f}\u{85}\u{2028}\u{2029}".repeat(1000);
    let names = [((1, 7), vec![(1, hostile.clone())])].into_iter().collect();
    let report = build_cost_report(
        "2026-06",
        &[entry(1, Some("safe"), 2, resolved(&hostile), 1)],
        &pricing(1),
        &names,
    );
    let text = format_cost_report_plain(&report);
    assert!(text.len() < 10000);
    assert!(!text
        .chars()
        .any(|c| c != '\n' && (c.is_control() || c == '\u{2028}' || c == '\u{2029}')));
}

#[test]
fn unpriced_rows_warn() {
    let report = build_cost_report(
        "2026-06",
        &[
            entry(1, Some("x"), 2, resolved("alice"), 1),
            entry(1, Some("y"), 2, resolved("alice"), 1),
        ],
        &ProviderPricingConfig::new(vec![]),
        &BTreeMap::new(),
    );
    assert_eq!(report.warnings, ["unpriced provider/model: provider/model"]);
}

#[test]
fn extreme_costs_do_not_silently_wrap() {
    let rows = [
        entry(1, Some("a"), 2, resolved("alice"), i64::MAX),
        entry(1, Some("b"), 2, resolved("alice"), i64::MAX),
    ];
    let report = build_cost_report("2026-06", &rows, &pricing(u64::MAX), &BTreeMap::new());
    assert_eq!(report.total_cost_micro, u64::MAX);
    assert!(
        report
            .rows
            .iter()
            .fold(0u64, |sum, row| sum.saturating_add(row.cost_micro))
            <= report.total_cost_micro
    );
}

#[test]
fn plain_escape_is_injective_for_backslashes_and_hides_bidi_overrides() {
    let real = "alice\u{85}";
    let spelled = "alice\\u{85}";
    assert_eq!(cost_terminal_field(real), "alice\\u{85}");
    assert_eq!(cost_terminal_field(spelled), "alice\\\\u{85}");
    assert_ne!(cost_terminal_field(real), cost_terminal_field(spelled));
    assert_eq!(cost_terminal_field("a\\\\b"), "a\\\\\\\\b");
    assert_eq!(cost_terminal_field("résumé 日本語 ok"), "résumé 日本語 ok");
    for c in [
        '\u{ad}',
        '\u{600}',
        '\u{61c}',
        '\u{180e}',
        '\u{200b}',
        '\u{200f}',
        '\u{202a}',
        '\u{202e}',
        '\u{2060}',
        '\u{2066}',
        '\u{2069}',
        '\u{206a}',
        '\u{fe00}',
        '\u{fe0f}',
        '\u{feff}',
        '\u{fff9}',
        '\u{e0001}',
        '\u{e0020}',
        '\u{e0100}',
        '\u{e01ef}',
    ] {
        let shown = cost_terminal_field(&format!("x{c}y"));
        assert_eq!(shown, format!("x\\u{{{:x}}}y", c as u32));
        assert!(shown.is_ascii());
    }
    let rows = [
        entry(1, Some("t"), 2, resolved(real), 1),
        entry(2, Some("t"), 2, resolved(spelled), 1),
    ];
    let report = build_cost_report("2026-06", &rows, &pricing(1), &BTreeMap::new());
    let text = format_cost_report_plain(&report);
    assert!(
        text.contains(" alice\\u{85} (") && text.contains(" alice\\\\u{85} ("),
        "{text}"
    );
    let json = serde_json::to_value(&report).unwrap();
    let principals: Vec<_> = json["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["principal"].as_str().unwrap().to_owned())
        .collect();
    assert!(principals.contains(&real.to_owned()) && principals.contains(&spelled.to_owned()));
}

#[test]
fn a_session_principal_over_128_bytes_is_skipped_while_a_proxy_principal_is_not_capped() {
    let session = |name: &str| {
        let mut row = entry(1, Some("t"), 2, resolved(name), 1);
        row.payload = row
            .payload
            .replace("write_target_proxy", "session_principal");
        row
    };
    let (long, ok) = ("p".repeat(129), "p".repeat(128));
    let rows = |entries: &[maos_audit::AuditEntry]| {
        build_cost_report("2026-06", entries, &pricing(1), &BTreeMap::new())
    };
    let skipped = rows(&[session(&long)]);
    assert!(skipped.rows.is_empty() && skipped.warnings.len() == 1);
    assert_eq!(rows(&[session(&ok)]).rows.len(), 1);
    assert_eq!(
        rows(&[entry(1, Some("t"), 2, resolved(&long), 1)])
            .rows
            .len(),
        1
    );
}
