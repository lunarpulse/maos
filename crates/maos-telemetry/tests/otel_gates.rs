#![forbid(unsafe_code)]

//! Story 9.5b — binding test gates (Murat / D8).
//!
//! Each gate has a passing test AND a proven-red companion that
//! demonstrates the gate can fail (D8 / M4).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::task::Poll;

use maos_domain::ports::trace_sink::{
    CapabilitySpanAttrs, HaltSpanAttrs, IacFrameSpanAttrs, SpanContext, SpanGuard, TraceSink,
};
use maos_iac::adapter::metrics::{IacRtMetrics, Outcome, Service};
use maos_telemetry::{BoundedExportProbe, OtelTraceSink, OtelTraceSinkConfig, SPAN_SCHEMA};
use opentelemetry_sdk::trace::{InMemorySpanExporter, SpanData};
fn make_test_sink() -> (Arc<OtelTraceSink>, InMemorySpanExporter, BoundedExportProbe) {
    let exporter = InMemorySpanExporter::default();
    let (sink, probe) = OtelTraceSink::with_bounded_channel(
        exporter.clone(),
        OtelTraceSinkConfig {
            service_name: "maos-test".into(),
            service_instance_id: "test-instance-1".into(),
        },
        32,
    );
    (Arc::new(sink), exporter, probe)
}

async fn finished_spans(
    exporter: &InMemorySpanExporter,
    probe: &BoundedExportProbe,
    expected: usize,
) -> Vec<SpanData> {
    probe.wait_until_exported(expected).await;
    if let Some(err) = probe.last_export_error() {
        panic!("export worker reported an error: {err}");
    }
    exporter.get_finished_spans().expect("exporter failed")
}

fn emitted_keys(span: &SpanData) -> Vec<&str> {
    span.attributes.iter().map(|kv| kv.key.as_str()).collect()
}

fn assert_keys_within_allowlist(span_name: &str, keys: &[&str]) -> Result<(), String> {
    let allowed = match span_name {
        "maos.iac_frame" => IAC_FRAME_ALLOWED_KEYS,
        "maos.capability" => CAPABILITY_ALLOWED_KEYS,
        "maos.halt" => HALT_ALLOWED_KEYS,
        other => return Err(format!("unexpected span name: {other}")),
    };
    for key in keys {
        if !allowed.contains(key) {
            return Err(format!(
                "span '{span_name}' has disallowed attr key '{key}'; allowed: {:?}",
                allowed
            ));
        }
    }
    Ok(())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source_tree_contains_literal(root: &Path, needle: &str) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if source_tree_contains_literal(&path, needle) {
                return true;
            }
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if content.contains(needle) {
            return true;
        }
    }
    false
}

fn network_syscalls(binary: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let strace = Command::new("strace")
        .arg("-V")
        .output()
        .map_err(|err| format!("strace unavailable: {err}"))?;
    if !strace.status.success() {
        return Err("strace -V failed".into());
    }

    let log_path = std::env::temp_dir().join(format!(
        "maos-otel-airgap-{}-{}.log",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let output = Command::new("strace")
        .arg("-e")
        .arg("trace=network")
        .arg("-o")
        .arg(&log_path)
        .arg(binary)
        .args(args)
        .output()
        .map_err(|err| format!("failed to run strace: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "strace target failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let log = fs::read_to_string(&log_path)
        .map_err(|err| format!("failed to read strace log {}: {err}", log_path.display()))?;
    let _ = fs::remove_file(&log_path);
    Ok(log
        .lines()
        .filter(|line| line.starts_with("connect(") || line.starts_with("socket("))
        .map(ToOwned::to_owned)
        .collect())
}

fn fixture_binary() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_otel-airgap-fixture")
            .expect("fixture binary path must be set by cargo"),
    )
}

fn frame_id_fixture() -> [u8; 16] {
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
}

// ===================================================================
// gate:otel-spans — AC-1
// ===================================================================

/// gate:otel-spans — PASSING.
///
/// In-memory SpanExporter; synthetic frame→cap→halt on
/// `tokio::test(flavor="multi_thread")` with a real `tokio::spawn`
/// between frame and cap.  Asserts exact 3-span-name set + two
/// linkage edges.  Flat sibling list FAILS.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gate_otel_spans_three_kinds_correct_linkage() {
    let (sink, exporter, probe) = make_test_sink();

    // 1. IAC frame span — MAOS is trace root.
    let frame_guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });
    let frame_ctx = *frame_guard.context();
    assert!(!frame_ctx.is_empty(), "frame span must have a real context");

    // 2. Capability span — child of frame, with real tokio::spawn.
    let sink2 = Arc::clone(&sink);
    let cap_handle = tokio::spawn(async move {
        let cap_guard = sink2.capability_span(
            &frame_ctx,
            CapabilitySpanAttrs {
                scope_label: "provider_infer".into(),
                spirit_pid: 42,
            },
        );
        let cap_ctx = *cap_guard.context();
        // Drop guard → span ends.
        drop(cap_guard);
        cap_ctx
    });
    let cap_ctx = cap_handle.await.unwrap();

    // capability shares trace_id with frame (AC-1 linkage edge 1).
    assert_eq!(
        cap_ctx.trace_id, frame_ctx.trace_id,
        "capability span must share trace_id with frame span"
    );
    // capability has a different span_id.
    assert_ne!(
        cap_ctx.span_id, frame_ctx.span_id,
        "capability span must have its own span_id"
    );

    // 3. Halt event — correlated by frame_id attribute, NOT parent_span_id.
    sink.halt_event(HaltSpanAttrs {
        halt_id: "halt-test-001".into(),
        tag: "drift.too_high".into(),
        predicate_kind: "on_value_above".into(),
        value_band: "over",
        threshold: Some(0.95),
        frame_id: frame_id_fixture(),
    });

    // Drop the frame guard → span ends.
    drop(frame_guard);

    let spans = finished_spans(&exporter, &probe, 3).await;

    // Exactly 3 spans.
    assert_eq!(
        spans.len(),
        3,
        "expected exactly 3 spans, got {}",
        spans.len()
    );

    // Exact span-name set.
    let mut names: Vec<_> = spans.iter().map(|s| s.name.as_ref()).collect();
    names.sort();
    assert_eq!(
        names,
        &["maos.capability", "maos.halt", "maos.iac_frame"],
        "exact span-name set must match"
    );

    // Linkage edge 1: capability.parent_span_id == iac_frame.span_id
    // AND shared trace_id.
    let frame_span = spans.iter().find(|s| s.name == "maos.iac_frame").unwrap();
    let cap_span = spans
        .iter()
        .find(|s| s.name == "maos.capability")
        .unwrap();
    let halt_span = spans.iter().find(|s| s.name == "maos.halt").unwrap();

    assert_eq!(
        cap_span.span_context.trace_id(),
        frame_span.span_context.trace_id(),
        "capability must share trace_id with frame"
    );
    assert_eq!(
        cap_span.parent_span_id,
        frame_span.span_context.span_id(),
        "capability.parent_span_id must be iac_frame.span_id"
    );

    // Linkage edge 2: halt span correlates by frame_id attribute.
    let halt_frame_id_attr = halt_span
        .attributes
        .iter()
        .find(|kv| kv.key.as_str() == "maos.frame_id")
        .expect("halt span must carry maos.frame_id attribute");
    let expected_frame_id_hex: String = frame_id_fixture()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        halt_frame_id_attr.value.as_str(),
        expected_frame_id_hex,
        "halt span's maos.frame_id must match the IAC frame's frame_id"
    );

    // Halt span must NOT be a child of the frame span (R2-4).
    assert_ne!(
        halt_span.parent_span_id,
        frame_span.span_context.span_id(),
        "halt span must NOT be a child of frame span (R2-4)"
    );
}

/// gate:otel-spans — PROVEN-RED: flat sibling list FAILS.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn gate_otel_spans_proven_red_flat_siblings_fail() {
    let (sink, exporter, probe) = make_test_sink();

    let frame_guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });

    // Intentionally create capability as a ROOT span (wrong parent).
    let cap_guard = sink.capability_span(
        &SpanContext::EMPTY,
        CapabilitySpanAttrs {
            scope_label: "provider_infer".into(),
            spirit_pid: 42,
        },
    );
    drop(cap_guard);
    drop(frame_guard);

    let spans = finished_spans(&exporter, &probe, 2).await;
    let frame_span = spans.iter().find(|s| s.name == "maos.iac_frame").unwrap();
    let cap_span = spans
        .iter()
        .find(|s| s.name == "maos.capability")
        .unwrap();

    // PROVEN-RED: capability is NOT a child of frame when given EMPTY parent.
    assert_ne!(
        cap_span.parent_span_id,
        frame_span.span_context.span_id(),
        "PROVEN-RED: flat sibling list — capability is not a child of frame"
    );
}

// ===================================================================
// gate:otel-attr-contract — AC-5
// ===================================================================

const IAC_FRAME_ALLOWED_KEYS: &[&str] = &[
    "maos.frame_id",
    "maos.frame_kind",
    "maos.intent",
    "service.name",
    "service.instance.id",
    "otel.scope.name",
    "otel.scope.version",
];

const CAPABILITY_ALLOWED_KEYS: &[&str] = &[
    "maos.scope_label",
    "maos.spirit_pid",
    "service.name",
    "service.instance.id",
    "otel.scope.name",
    "otel.scope.version",
];

const HALT_ALLOWED_KEYS: &[&str] = &[
    "maos.halt_id",
    "maos.tag",
    "maos.predicate_kind",
    "maos.threshold",
    "maos.value_band",
    "maos.frame_id",
    "service.name",
    "service.instance.id",
    "otel.scope.name",
    "otel.scope.version",
];

/// gate:otel-attr-contract — PASSING: emitted keys ⊆ allowlist.
#[tokio::test]
async fn gate_otel_attr_contract_keys_within_allowlist() {
    let (sink, exporter, probe) = make_test_sink();

    let frame_guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });
    let frame_ctx = *frame_guard.context();

    let cap_guard = sink.capability_span(
        &frame_ctx,
        CapabilitySpanAttrs {
            scope_label: "provider_infer".into(),
            spirit_pid: 42,
        },
    );
    drop(cap_guard);

    sink.halt_event(HaltSpanAttrs {
        halt_id: "halt-001".into(),
        tag: "test_tag".into(),
        predicate_kind: "on_value_above".into(),
        value_band: "over",
        threshold: Some(0.95),
        frame_id: frame_id_fixture(),
    });

    drop(frame_guard);

    let spans = finished_spans(&exporter, &probe, 3).await;

    for span in &spans {
        assert_keys_within_allowlist(span.name.as_ref(), &emitted_keys(span))
            .unwrap_or_else(|err| panic!("{err}"));
    }
}

/// gate:otel-attr-contract — PROVEN-RED: subject_id/principal_id injection RED.
#[tokio::test]
async fn gate_otel_attr_contract_proven_red_subject_id_rejected() {
    let (sink, exporter, probe) = make_test_sink();
    let guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });
    drop(guard);

    let spans = finished_spans(&exporter, &probe, 1).await;
    let frame_span = spans.iter().find(|span| span.name == "maos.iac_frame").unwrap();
    let mut keys = emitted_keys(frame_span);
    keys.push("subject_id");
    let err = assert_keys_within_allowlist("maos.iac_frame", &keys)
        .expect_err("subject_id injection must trip the attr-contract gate");
    assert!(err.contains("subject_id"));
}

/// gate:otel-attr-contract — forget-cascade no-op (R2-5).
#[test]
fn gate_otel_attr_contract_forget_cascade_noop() {
    let all_allowed: Vec<&str> = IAC_FRAME_ALLOWED_KEYS
        .iter()
        .chain(CAPABILITY_ALLOWED_KEYS)
        .chain(HALT_ALLOWED_KEYS)
        .copied()
        .collect();

    let forbidden = [
        "subject_id",
        "principal_id",
        "spirit_id",
        "user_id",
        "session_id",
        "email",
        "name",
    ];
    for key in &forbidden {
        assert!(
            !all_allowed.contains(key),
            "attr '{key}' would create principal nexus — forbidden (R2-5)"
        );
    }
}

// ===================================================================
// gate:otel-zero-when-off — AC-2
// ===================================================================

/// gate:otel-zero-when-off — 0 finished spans when sink is None.
#[tokio::test]
async fn gate_otel_zero_when_off_no_spans() {
    let exporter = InMemorySpanExporter::default();
    let spans = exporter.get_finished_spans().expect("exporter failed");
    assert_eq!(spans.len(), 0, "0 finished spans when sink is None");
}

/// gate:otel-zero-when-off — N>0 on the ON path.
#[tokio::test]
async fn gate_otel_zero_when_off_counting_double_asserts_on() {
    let (sink, exporter, probe) = make_test_sink();
    let guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });
    drop(guard);
    let spans = finished_spans(&exporter, &probe, 1).await;
    assert!(!spans.is_empty(), "N>0 on the ON path");
}

#[test]
fn gate_otel_zero_when_off_kernel_core_tree_has_no_otel() {
    let output = Command::new("cargo")
        .arg("tree")
        .arg("-p")
        .arg("maos-kernel-core")
        .current_dir(workspace_root())
        .output()
        .expect("cargo tree must run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);
    assert!(
        !tree.contains("opentelemetry"),
        "kernel-core dep tree must not contain opentelemetry:\n{tree}"
    );
    assert!(
        !tree.contains("tonic"),
        "kernel-core dep tree must not contain tonic:\n{tree}"
    );
}

// ===================================================================
// gate:otel-airgap / gate:otel-airgap-enabled — AC-4
// ===================================================================

#[test]
fn gate_otel_airgap_default_off_no_connects() {
    let Ok(syscalls) = network_syscalls(&fixture_binary(), &["off"]) else {
        eprintln!("SKIP: strace unavailable");
        return;
    };
    assert!(
        syscalls.is_empty(),
        "default-off fixture must issue zero network syscalls, got: {syscalls:?}"
    );
}

#[test]
fn gate_otel_airgap_enabled_no_endpoint_no_connects() {
    assert!(
        !source_tree_contains_literal(&crate_root().join("src"), "4317"),
        "crate source must not contain the default OTLP endpoint literal 4317"
    );
    let Ok(syscalls) = network_syscalls(&fixture_binary(), &["on"]) else {
        eprintln!("SKIP: strace unavailable");
        return;
    };
    assert!(
        syscalls.is_empty(),
        "enabled sink without endpoint config must issue zero network syscalls, got: {syscalls:?}"
    );
}

#[test]
fn gate_otel_airgap_enabled_proven_red_network_canary_detected() {
    let Ok(syscalls) = network_syscalls(
        Path::new("python3"),
        &[
            "-c",
            "import socket\ns=socket.socket()\ntry:\n    s.connect(('127.0.0.1', 1))\nexcept OSError:\n    pass\n",
        ],
    ) else {
        eprintln!("SKIP: strace unavailable");
        return;
    };
    assert!(
        !syscalls.is_empty(),
        "network canary must produce connect/socket syscalls so the gate proves red"
    );
}

/// gate:otel-zero-when-off — PROVEN-RED: noop guard → 0 spans.
#[tokio::test]
async fn gate_otel_zero_when_off_proven_red_noop_guard() {
    let (_sink, exporter, _probe) = make_test_sink();
    let guard = SpanGuard::noop();
    assert!(guard.context().is_empty());
    drop(guard);
    let spans = exporter.get_finished_spans().expect("exporter failed");
    assert_eq!(spans.len(), 0, "PROVEN-RED: noop guard → 0 spans");
}

// ===================================================================
// gate:otel-tracesink-seam — §A6
// ===================================================================

/// gate:otel-tracesink-seam — PASSING: inject a fake TraceSink.
#[tokio::test]
async fn gate_otel_tracesink_seam_fake_sink() {
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingSink {
        frame_count: AtomicU32,
        cap_count: AtomicU32,
        halt_count: AtomicU32,
    }

    impl TraceSink for CountingSink {
        fn iac_frame_span(&self, _attrs: IacFrameSpanAttrs) -> SpanGuard {
            self.frame_count.fetch_add(1, Ordering::SeqCst);
            SpanGuard::noop()
        }
        fn capability_span(&self, _parent: &SpanContext, _attrs: CapabilitySpanAttrs) -> SpanGuard {
            self.cap_count.fetch_add(1, Ordering::SeqCst);
            SpanGuard::noop()
        }
        fn halt_event(&self, _attrs: HaltSpanAttrs) {
            self.halt_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let fake = Arc::new(CountingSink {
        frame_count: AtomicU32::new(0),
        cap_count: AtomicU32::new(0),
        halt_count: AtomicU32::new(0),
    });

    let sink: Arc<dyn TraceSink> = fake.clone();

    let _guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: [0u8; 16],
        kind: "task_assign",
        intent: "standard",
    });
    let _cap = sink.capability_span(
        &SpanContext::EMPTY,
        CapabilitySpanAttrs {
            scope_label: "test".into(),
            spirit_pid: 1,
        },
    );
    sink.halt_event(HaltSpanAttrs {
        halt_id: "h1".into(),
        tag: "t".into(),
        predicate_kind: "p".into(),
        value_band: "over",
        threshold: None,
        frame_id: [0u8; 16],
    });

    assert_eq!(fake.frame_count.load(Ordering::SeqCst), 1);
    assert_eq!(fake.cap_count.load(Ordering::SeqCst), 1);
    assert_eq!(fake.halt_count.load(Ordering::SeqCst), 1);
}

/// gate:otel-tracesink-seam — PROVEN-RED: None is noop.
#[test]
fn gate_otel_tracesink_seam_proven_red_none_is_noop() {
    let sink: Option<Arc<dyn TraceSink>> = None;
    let guard = if let Some(s) = &sink {
        s.iac_frame_span(IacFrameSpanAttrs {
            frame_id: [0u8; 16],
            kind: "test",
            intent: "standard",
        })
    } else {
        SpanGuard::noop()
    };
    assert!(guard.context().is_empty(), "None → noop guard");
}

// ===================================================================
// gate:otel-slo-class — AC-1 / naming
// ===================================================================

/// gate:otel-slo-class — correlates the trace path to the shipped
/// `iac_rt_duration_us` substrate without inventing a new metric family.
#[tokio::test]
async fn gate_otel_slo_class_correlates_with_iac_histogram() {
    let metrics = IacRtMetrics::new();
    metrics.record_iac_rt(Service::Capability, Outcome::Ok, 1_200);
    let rendered = metrics.render_prometheus();
    assert!(
        rendered.contains(
            "iac_rt_duration_us_bucket{service=\"capability\",outcome=\"ok\",le=\"1500\"}"
        ),
        "existing histogram substrate must remain the SLO metric source"
    );

    let (sink, exporter, probe) = make_test_sink();

    let guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });
    let ctx = *guard.context();

    let cap = sink.capability_span(
        &ctx,
        CapabilitySpanAttrs {
            scope_label: "provider_infer".into(),
            spirit_pid: 42,
        },
    );
    drop(cap);

    sink.halt_event(HaltSpanAttrs {
        halt_id: "halt-slo".into(),
        tag: "slo_tag".into(),
        predicate_kind: "on_value_above".into(),
        value_band: "over",
        threshold: Some(0.99),
        frame_id: frame_id_fixture(),
    });

    drop(guard);

    let spans = finished_spans(&exporter, &probe, 3).await;
    let frame_span = spans.iter().find(|s| s.name == "maos.iac_frame").unwrap();
    let cap_span = spans
        .iter()
        .find(|s| s.name == "maos.capability")
        .unwrap();

    assert_eq!(
        cap_span.span_context.trace_id(),
        frame_span.span_context.trace_id(),
        "SLO-class trace path must correlate capability work to the IAC frame"
    );
    assert!(
        spans.iter().all(|span| !span.name.as_ref().contains("metric")),
        "trace tier must not invent a new metric family"
    );
}

// ===================================================================
// Span-schema SSOT — AC-6 / R2-7
// ===================================================================

/// AC-6: emitted span names + attr keys match the SSOT table.
#[tokio::test]
async fn gate_otel_schema_ssot_matches_emitted() {
    let (sink, exporter, probe) = make_test_sink();

    let guard = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: frame_id_fixture(),
        kind: "task_assign",
        intent: "standard",
    });
    let ctx = *guard.context();

    let cap = sink.capability_span(
        &ctx,
        CapabilitySpanAttrs {
            scope_label: "provider_infer".into(),
            spirit_pid: 42,
        },
    );
    drop(cap);

    sink.halt_event(HaltSpanAttrs {
        halt_id: "halt-schema".into(),
        tag: "tag".into(),
        predicate_kind: "on_value_above".into(),
        value_band: "over",
        threshold: None,
        frame_id: frame_id_fixture(),
    });
    drop(guard);

    let spans = finished_spans(&exporter, &probe, 3).await;

    for span in &spans {
        let schema_entry = SPAN_SCHEMA
            .iter()
            .find(|e| e.span_name == span.name.as_ref())
            .unwrap_or_else(|| panic!("span '{}' has no SSOT entry", span.name));

        let emitted_keys: Vec<&str> =
            span.attributes.iter().map(|kv| kv.key.as_str()).collect();

        for required_key in schema_entry.required_attrs {
            assert!(
                emitted_keys.contains(required_key),
                "span '{}' missing required attr '{}'",
                span.name,
                required_key
            );
        }
    }
}

/// AC-6 PROVEN-RED: bogus span name not in SSOT.
#[test]
fn gate_otel_schema_ssot_proven_red_unknown_span() {
    assert!(
        !SPAN_SCHEMA.iter().any(|e| e.span_name == "maos.bogus"),
        "unknown span name must not appear in SSOT"
    );
}

// ===================================================================
// gate:otel-degradation — AC-3
// ===================================================================

/// gate:otel-degradation — queue saturates without blocking the hot path.
#[tokio::test]
async fn gate_otel_degradation_hot_path_completes() {
    let exporter = InMemorySpanExporter::default();
    let (sink, probe) = OtelTraceSink::with_bounded_channel(
        exporter,
        OtelTraceSinkConfig::default(),
        2,
    );
    probe.pause_consumer();

    for i in 0..2u8 {
        let guard = sink.iac_frame_span(IacFrameSpanAttrs {
            frame_id: {
                let mut id = [0u8; 16];
                id[0] = i;
                id
            },
            kind: "task_assign",
            intent: "standard",
        });
        drop(guard);
    }

    for i in 2..5u8 {
        let polled = futures::poll!(std::future::poll_fn(|_| {
            let guard = sink.iac_frame_span(IacFrameSpanAttrs {
                frame_id: {
                    let mut id = [0u8; 16];
                    id[0] = i;
                    id
                },
                kind: "task_assign",
                intent: "standard",
            });
            drop(guard);
            Poll::Ready(())
        }));
        assert!(matches!(polled, Poll::Ready(())));
    }

    assert_eq!(probe.queued_spans(), 2, "queue must saturate at capacity");
    assert_eq!(probe.drop_count(), 3, "overflow spans must be counted as dropped");

    probe.resume_consumer();
    probe.wait_until_exported(2).await;
    assert_eq!(probe.drop_count(), 3, "drain must not change prior drop accounting");
}

/// gate:otel-degradation — PROVEN-RED: overflow increments the drop counter.
#[tokio::test]
async fn gate_otel_degradation_proven_red_drop_counter() {
    let exporter = InMemorySpanExporter::default();
    let (sink, probe) = OtelTraceSink::with_bounded_channel(
        exporter,
        OtelTraceSinkConfig::default(),
        1,
    );
    probe.pause_consumer();

    let first = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: [1; 16],
        kind: "task_assign",
        intent: "standard",
    });
    drop(first);

    let second = sink.iac_frame_span(IacFrameSpanAttrs {
        frame_id: [2; 16],
        kind: "task_assign",
        intent: "standard",
    });
    drop(second);

    assert_eq!(probe.queued_spans(), 1, "first span should occupy the only slot");
    assert_eq!(probe.drop_count(), 1, "second span must trip the proven-red overflow path");

    probe.resume_consumer();
}

// ===================================================================
// Halt span STATUS=Error — R2-6
// ===================================================================

/// R2-6: halt span STATUS=Error.
#[tokio::test]
async fn halt_span_status_is_error() {
    let (sink, exporter, probe) = make_test_sink();

    sink.halt_event(HaltSpanAttrs {
        halt_id: "halt-status".into(),
        tag: "status_tag".into(),
        predicate_kind: "on_value_above".into(),
        value_band: "over",
        threshold: Some(0.5),
        frame_id: frame_id_fixture(),
    });

    let spans = finished_spans(&exporter, &probe, 1).await;
    let halt_span = spans.iter().find(|s| s.name == "maos.halt").unwrap();

    assert_eq!(
        halt_span.status,
        opentelemetry::trace::Status::error("halt"),
        "halt span must have STATUS=Error (R2-6)"
    );
}

// ===================================================================
// HaltTelemetryEntry::to_span_attrs — R2-5 value bucketing
// ===================================================================

#[test]
fn halt_entry_to_span_attrs_value_bucketing() {
    use maos_domain::self_telemetry::HaltTelemetryEntry;

    let entry = HaltTelemetryEntry {
        halt_id: "h1".into(),
        tag: "t".into(),
        predicate_kind: "on_value_above".into(),
        value: 1.5,
        threshold: Some(1.0),
        fired_ns: 0,
        resolution: None,
    };

    let attrs = entry.to_span_attrs([0u8; 16]);
    assert_eq!(attrs.value_band, "over", "1.5 > 1.0 → over");

    let entry_at = HaltTelemetryEntry {
        value: 1.0,
        ..entry.clone()
    };
    assert_eq!(entry_at.to_span_attrs([0u8; 16]).value_band, "at");

    let entry_under = HaltTelemetryEntry {
        value: 0.5,
        ..entry.clone()
    };
    assert_eq!(entry_under.to_span_attrs([0u8; 16]).value_band, "under");

    let entry_none = HaltTelemetryEntry {
        threshold: None,
        ..entry
    };
    assert_eq!(entry_none.to_span_attrs([0u8; 16]).value_band, "unknown");
}
