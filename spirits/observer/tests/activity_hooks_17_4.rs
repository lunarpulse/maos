//! Story 17-4 (AC1.2, AC3.4) — Observer's two telemetry hooks through its
//! real vtable: `on_telemetry_event` decodes the pump's `serde_json` payload,
//! counts what it cannot decode, applies the namespace view filter, reports
//! each received write and its drift warnings, and `on_consolidate` closes an
//! aggregate window. Values are compared by bits, never by tolerance.

use std::sync::{Arc, Mutex};

use maos_domain::invariants::i7::ScalarTapEvent;
use maos_spirit_sdk::local_runner::{LocalRunner, LocalRunnerFixture};
use maos_spirit_sdk::{ConsolidatePayload, Ctx, Spirit, TelemetryEventPayload};
use observer::__maos_spirit_vtable_Observer;
use observer::{
    ActivitySink, AnomalyKind, DriftDirection, ObservedActivity, Observer, PrincipalScope,
    StreamSummary, WatchThreshold,
};

#[derive(Debug, Default)]
struct Recorded(Mutex<Vec<ObservedActivity>>);

impl ActivitySink for Recorded {
    fn emit(&self, activity: ObservedActivity) {
        self.0.lock().unwrap().push(activity);
    }
}

impl Recorded {
    fn take(&self) -> Vec<ObservedActivity> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

fn payload(seq: u64, spirit_id: &str, tag: &str, value: f64) -> Vec<u8> {
    serde_json::to_vec(&ScalarTapEvent {
        seq,
        spirit_id: spirit_id.into(),
        tag: tag.into(),
        value,
        timestamp: 1_000 + seq,
    })
    .unwrap()
}

fn observer_with(sink: &Arc<Recorded>, scope: PrincipalScope) -> Observer {
    Observer::watching(scope, Vec::new()).with_activity_sink(Arc::clone(sink) as _)
}

fn run(observer: &Observer, events: Vec<Vec<u8>>, consolidations: usize) {
    let fixture = LocalRunnerFixture {
        telemetry_events: events,
        consolidate_payloads: vec![Vec::new(); consolidations],
        ..Default::default()
    };
    LocalRunner::run(observer, __maos_spirit_vtable_Observer(), &fixture);
}

fn scalar(seq: u64, spirit_id: &str, tag: &str, value: f64) -> ObservedActivity {
    ObservedActivity::Scalar {
        observer: "observer".into(),
        spirit_id: spirit_id.into(),
        tag: tag.into(),
        value,
        seq,
        timestamp_ms: 1_000 + seq,
    }
}

fn stream(spirit_id: &str, tag: &str, count: u64, last_value: f64) -> StreamSummary {
    StreamSummary {
        spirit_id: spirit_id.into(),
        tag: tag.into(),
        count,
        last_value,
    }
}

#[test]
fn received_writes_are_reported_in_delivery_order_and_aggregated() {
    let sink = Arc::new(Recorded::default());
    let observer = observer_with(&sink, PrincipalScope::all());
    run(
        &observer,
        vec![
            payload(0, "butler", "belief_variance", 0.5),
            payload(1, "mira", "diagnostic_confidence", 0.9),
            payload(2, "butler", "belief_variance", 0.75),
        ],
        1,
    );
    assert_eq!(
        sink.take(),
        vec![
            scalar(0, "butler", "belief_variance", 0.5),
            scalar(1, "mira", "diagnostic_confidence", 0.9),
            scalar(2, "butler", "belief_variance", 0.75),
            ObservedActivity::Consolidated {
                observer: "observer".into(),
                window_events: 3,
                streams: vec![
                    stream("butler", "belief_variance", 2, 0.75),
                    stream("mira", "diagnostic_confidence", 1, 0.9),
                ],
                undecodable: 0,
            },
        ]
    );
}

#[test]
fn undecodable_payloads_are_counted_not_reported_as_writes() {
    let sink = Arc::new(Recorded::default());
    let observer = observer_with(&sink, PrincipalScope::all());
    run(
        &observer,
        vec![b"not json".to_vec(), b"{\"seq\":1}".to_vec()],
        1,
    );
    assert_eq!(
        sink.take(),
        vec![ObservedActivity::Consolidated {
            observer: "observer".into(),
            window_events: 0,
            streams: Vec::new(),
            undecodable: 2,
        }]
    );
}

#[test]
fn the_namespace_view_filter_hides_other_emitters_entirely() {
    let sink = Arc::new(Recorded::default());
    let observer = observer_with(&sink, PrincipalScope::from_patterns(["mira"]));
    run(
        &observer,
        vec![
            payload(0, "butler", "belief_variance", 0.8),
            payload(1, "mira", "belief_variance", 0.4),
        ],
        1,
    );
    assert_eq!(
        sink.take(),
        vec![
            scalar(1, "mira", "belief_variance", 0.4),
            ObservedActivity::Consolidated {
                observer: "observer".into(),
                window_events: 1,
                streams: vec![stream("mira", "belief_variance", 1, 0.4)],
                undecodable: 0,
            },
        ]
    );
}

#[test]
fn a_configured_watch_reports_its_drift_warning_after_the_write() {
    let sink = Arc::new(Recorded::default());
    let observer = Observer::watching(
        PrincipalScope::all(),
        vec![WatchThreshold::new("belief_variance", 0.7, DriftDirection::Above, 0.15).unwrap()],
    )
    .with_activity_sink(Arc::clone(&sink) as _);
    run(
        &observer,
        vec![payload(4, "mira", "belief_variance", 0.66)],
        0,
    );
    let records = sink.take();
    assert_eq!(records.len(), 2, "{records:?}");
    assert_eq!(records[0], scalar(4, "mira", "belief_variance", 0.66));
    match &records[1] {
        ObservedActivity::Anomaly { observer, surface } => {
            assert_eq!(observer, "observer");
            assert_eq!(surface.subject, "mira");
            assert_eq!(surface.anomaly_kind, AnomalyKind::DriftEarlyWarning);
        }
        other => panic!("expected the drift warning, got {other:?}"),
    }
}

#[test]
fn an_empty_window_is_silent_and_a_closed_window_resets() {
    let sink = Arc::new(Recorded::default());
    let observer = observer_with(&sink, PrincipalScope::all());
    run(&observer, Vec::new(), 2);
    assert!(sink.take().is_empty(), "an empty window emits nothing");

    run(&observer, vec![payload(0, "butler", "a", 1.0)], 2);
    let first = sink.take();
    assert_eq!(
        first.len(),
        2,
        "one write, one window, the second close silent"
    );

    run(&observer, vec![payload(1, "butler", "b", 2.0)], 1);
    assert_eq!(
        sink.take()[1],
        ObservedActivity::Consolidated {
            observer: "observer".into(),
            window_events: 1,
            streams: vec![stream("butler", "b", 1, 2.0)],
            undecodable: 0,
        },
        "the next window counts only its own writes"
    );
}

#[test]
fn values_round_trip_bit_exact_through_payload_and_rendered_line() {
    for value in [
        51.248178375505404_f64,
        f64::from(0.8_f32),
        0.1 + 0.2,
        f64::MIN_POSITIVE,
    ] {
        let sink = Arc::new(Recorded::default());
        let observer = observer_with(&sink, PrincipalScope::all());
        run(
            &observer,
            vec![payload(9, "butler", "belief_variance", value)],
            1,
        );
        let records = sink.take();
        let ObservedActivity::Scalar { value: got, .. } = &records[0] else {
            panic!("expected a scalar record, got {records:?}");
        };
        assert_eq!(got.to_bits(), value.to_bits(), "decoded {value}");

        // The host prints `serde_json::to_string(&record)`; a reader parsing
        // that line must recover the same bits.
        for record in &records {
            let line = serde_json::to_string(record).unwrap();
            let back: ObservedActivity = serde_json::from_str(&line).unwrap();
            assert_eq!(&back, record, "line {line}");
        }
        let line: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&records[0]).unwrap()).unwrap();
        assert_eq!(line["event"], "observer.scalar");
        assert_eq!(line["value"].as_f64().unwrap().to_bits(), value.to_bits());
    }
}

#[test]
fn the_rendered_scalar_line_has_the_contracted_shape() {
    let line =
        serde_json::to_value(scalar(7, "butler", "belief_variance", f64::from(0.8_f32))).unwrap();
    assert_eq!(
        line,
        serde_json::json!({
            "event": "observer.scalar",
            "observer": "observer",
            "spirit_id": "butler",
            "tag": "belief_variance",
            "value": 0.800000011920929,
            "seq": 7,
            "timestamp_ms": 1007,
        })
    );
}

#[test]
fn concurrent_writes_and_closes_never_lose_or_split_a_count() {
    const WRITERS: u64 = 4;
    const PER_WRITER: u64 = 500;
    let sink = Arc::new(Recorded::default());
    let observer = Arc::new(observer_with(&sink, PrincipalScope::all()));

    let writers: Vec<_> = (0..WRITERS)
        .map(|w| {
            let observer = Arc::clone(&observer);
            std::thread::spawn(move || {
                let mut ctx = Ctx::mock();
                for i in 0..PER_WRITER {
                    let bytes = payload(w * PER_WRITER + i, &format!("s{w}"), "t", i as f64);
                    let p = TelemetryEventPayload {
                        event_data: &bytes,
                        event_len: bytes.len(),
                    };
                    Spirit::on_telemetry_event(&*observer, &mut ctx, &p);
                }
            })
        })
        .collect();
    let closer = {
        let observer = Arc::clone(&observer);
        std::thread::spawn(move || {
            let mut ctx = Ctx::mock();
            let p = ConsolidatePayload {
                batch_data: &[],
                batch_len: 0,
            };
            for _ in 0..2_000 {
                Spirit::on_consolidate(&*observer, &mut ctx, &p);
            }
        })
    };
    for writer in writers {
        writer.join().unwrap();
    }
    closer.join().unwrap();
    let empty = ConsolidatePayload {
        batch_data: &[],
        batch_len: 0,
    };
    Spirit::on_consolidate(&*observer, &mut Ctx::mock(), &empty);

    // Every Consolidated record counts exactly the Scalar records emitted
    // since the previous one, and the streams sum to the same figure.
    let mut since_close = 0_u64;
    let mut scalars = 0_u64;
    for record in sink.take() {
        match record {
            ObservedActivity::Scalar { .. } => {
                since_close += 1;
                scalars += 1;
            }
            ObservedActivity::Consolidated {
                window_events,
                streams,
                ..
            } => {
                assert_eq!(window_events, since_close);
                assert_eq!(streams.iter().map(|s| s.count).sum::<u64>(), since_close);
                since_close = 0;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(since_close, 0, "the final close took the last window");
    assert_eq!(scalars, WRITERS * PER_WRITER);
}
