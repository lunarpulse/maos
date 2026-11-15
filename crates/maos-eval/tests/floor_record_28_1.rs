//! Story 28-1 (D-28-1-04 A, AC4, AC7) — the calibrated-floor record, the
//! `format_errors` score field and its missed floor.

use maos_eval::halt_replay::{
    missed_floors, round_down_hundredth, ClassFloors, FloorRecord, HaltScore, FLOORS_SCHEMA,
};

const BASE: ClassFloors = ClassFloors {
    halt_recall_calendar_conflict: Some(0.90),
    halt_recall_overall: 0.90,
    halt_precision_overall: 0.85,
};

fn record(schema: &str, entries: &[(&str, f64, Option<&str>)]) -> FloorRecord {
    let mut text = format!("schema = \"{schema}\"\n");
    for (metric, floor, ruling) in entries {
        text += &format!(
            "[[floor]]\nmetric = \"{metric}\"\nfloor = {floor}\nspec_minimum = 0.85\n\
             spec_source = \"NFR-Onb-1\"\nmeasured_mean = 0.92\nwilson95 = [0.905, 0.95]\nk = 5\n\
             model_id = \"claude-haiku-5-5\"\nprompt_sha256 = \"ab\"\nrule = \"D-28-1-03 A\"\n\
             rationale = \"test\"\ndate = \"2026-11-15\"\nresult_file = \"r.json\"\n"
        );
        if let Some(ruling) = ruling {
            text += &format!("operator_ruling = \"{ruling}\"\n");
        }
    }
    toml::from_str(&text).expect("a floor record")
}

fn score(format_errors: usize) -> HaltScore {
    HaltScore {
        rows: 30,
        expected_halts: 15,
        true_positives: 15,
        false_positives: 0,
        false_negatives: 0,
        halt_recall_calendar_conflict: Some(1.0),
        halt_recall_overall: 1.0,
        halt_precision_overall: 1.0,
        format_errors,
    }
}

#[test]
fn a_record_raises_a_floor_and_leaves_the_others_at_the_spec_minimum() {
    let floors = record(FLOORS_SCHEMA, &[("halt_precision_overall", 0.91, None)])
        .apply(BASE)
        .unwrap();
    assert_eq!(floors.halt_precision_overall, 0.91);
    assert_eq!(floors.halt_recall_overall, 0.90);
    assert_eq!(floors.halt_recall_calendar_conflict, Some(0.90));
}

#[test]
fn a_floor_below_the_spec_minimum_needs_an_operator_ruling() {
    let low = [("halt_precision_overall", 0.80, None)];
    let err = record(FLOORS_SCHEMA, &low).apply(BASE).unwrap_err();
    assert!(
        err.contains("below its calibrated 0.9 (spec minimum 0.85"),
        "{err}"
    );

    let ruled = [(
        "halt_precision_overall",
        0.80,
        Some("decisions-2026-12-01 D-x"),
    )];
    let floors = record(FLOORS_SCHEMA, &ruled).apply(BASE).unwrap();
    assert_eq!(floors.halt_precision_overall, 0.80);
}

/// The floor must follow from the record's own measurement: Wilson lower bound
/// 0.9 against a spec minimum of 0.85 requires 0.9, not the spec minimum.
#[test]
fn a_floor_below_its_own_wilson_lower_bound_needs_an_operator_ruling() {
    let edited = [("halt_precision_overall", 0.88, None)];
    let err = record(FLOORS_SCHEMA, &edited).apply(BASE).unwrap_err();
    assert!(err.contains("Wilson lower bound 0.9"), "{err}");

    let exact = [("halt_precision_overall", 0.90, None)];
    let floors = record(FLOORS_SCHEMA, &exact).apply(BASE).unwrap();
    assert_eq!(floors.halt_precision_overall, 0.90);

    let ruled = [(
        "halt_precision_overall",
        0.88,
        Some("decisions-2026-12-01 D-x"),
    )];
    let floors = record(FLOORS_SCHEMA, &ruled).apply(BASE).unwrap();
    assert_eq!(floors.halt_precision_overall, 0.88);
}

#[test]
fn the_calibrated_bound_rounds_strictly_down() {
    assert_eq!(round_down_hundredth(0.899999999999), 0.89);
    assert_eq!(round_down_hundredth(0.7961), 0.79);
    assert_eq!(round_down_hundredth(0.905), 0.9);
}

/// `nan` is valid TOML and `f64::max` ignores it: evidence is validated before
/// it is used, or a NaN lower bound would collapse the floor to the spec minimum.
#[test]
fn evidence_that_is_not_a_measurement_is_refused_before_it_is_used() {
    const LOW: ClassFloors = ClassFloors {
        halt_recall_calendar_conflict: None,
        halt_recall_overall: 0.5,
        halt_precision_overall: 0.5,
    };
    let apply = |mean: &str, wilson: &str| {
        let text = format!(
            "schema = \"{FLOORS_SCHEMA}\"\n[[floor]]\nmetric = \"halt_recall_overall\"\nfloor = 0.5\n\
             spec_minimum = 0.5\nspec_source = \"t\"\nmeasured_mean = {mean}\nwilson95 = {wilson}\n\
             k = 5\nmodel_id = \"m\"\nprompt_sha256 = \"ab\"\nrule = \"r\"\nrationale = \"t\"\n\
             date = \"2026-11-15\"\nresult_file = \"r.json\"\n"
        );
        toml::from_str::<FloorRecord>(&text).unwrap().apply(LOW)
    };
    assert_eq!(
        apply("0.9", "[0.505, 0.95]").unwrap().halt_recall_overall,
        0.5
    );
    for (mean, wilson) in [
        ("0.9", "[nan, 0.95]"),
        ("0.9", "[0.8, nan]"),
        ("nan", "[0.8, 0.95]"),
        ("0.9", "[-0.1, 0.95]"),
        ("0.9", "[0.9, 0.8]"),
        ("0.9", "[0.8, 1.5]"),
        ("0.9", "[0.8, inf]"),
        ("1.5", "[0.8, 0.95]"),
    ] {
        let err = apply(mean, wilson).unwrap_err();
        assert!(err.contains("finite"), "{mean} {wilson}: {err}");
    }
}

#[test]
fn a_malformed_record_is_refused() {
    let one = [("halt_recall_overall", 0.95, None)];
    assert!(record("maos.eval.floors/v0", &one)
        .apply(BASE)
        .unwrap_err()
        .contains("schema"));
    let unknown = [("format_error_rate", 0.5, None)];
    assert!(record(FLOORS_SCHEMA, &unknown)
        .apply(BASE)
        .unwrap_err()
        .contains("not a floor"));
    let twice = [
        ("halt_recall_overall", 0.95, None),
        ("halt_recall_overall", 0.96, None),
    ];
    assert!(record(FLOORS_SCHEMA, &twice)
        .apply(BASE)
        .unwrap_err()
        .contains("repeats"));
    let above = [("halt_recall_overall", 1.5, None)];
    assert!(record(FLOORS_SCHEMA, &above)
        .apply(BASE)
        .unwrap_err()
        .contains("within [0, 1]"));
    let no_cc = ClassFloors {
        halt_recall_calendar_conflict: None,
        ..BASE
    };
    let cc = [("halt_recall_calendar_conflict", 0.95, None)];
    assert!(record(FLOORS_SCHEMA, &cc)
        .apply(no_cc)
        .unwrap_err()
        .contains("no floor"));
}

#[test]
fn a_malformed_completion_misses_the_zero_floor() {
    assert!(missed_floors(&score(0), &BASE).is_empty());
    assert_eq!(
        missed_floors(&score(1), &BASE),
        ["format_error_rate 0.033 above floor 0.00"]
    );
}

#[test]
fn a_score_without_the_field_reads_as_zero_format_errors() {
    let mut json = serde_json::to_value(score(2)).unwrap();
    json.as_object_mut().unwrap().remove("format_errors");
    let read: HaltScore = serde_json::from_value(json).unwrap();
    assert_eq!(read, score(0));
}
