#![forbid(unsafe_code)]

//! OTel-backed [`TraceSink`] implementation (Story 9.5b).
//!
//! Emits spans through the OpenTelemetry SDK. Completed spans are handed to a
//! bounded queueing exporter; on a full queue, the hot path drops the span and
//! increments a counter instead of blocking on exporter work (AC-3).

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use maos_domain::ports::trace_sink::{
    CapabilitySpanAttrs, HaltSpanAttrs, IacFrameSpanAttrs, SpanContext, SpanGuard, TraceSink,
};

use opentelemetry::trace::{
    SpanBuilder, SpanId, SpanKind, Status, TraceContextExt, Tracer, TracerProvider as _,
};
use opentelemetry::{Context, KeyValue};
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::trace::{SdkTracerProvider, SpanData, SpanExporter};

/// Service metadata injected into every span (R2-6).
#[derive(Debug, Clone)]
pub struct OtelTraceSinkConfig {
    pub service_name: String,
    pub service_instance_id: String,
}

impl Default for OtelTraceSinkConfig {
    fn default() -> Self {
        Self {
            service_name: "maos".into(),
            service_instance_id: "default".into(),
        }
    }
}

const SCOPE_NAME: &str = "maos-telemetry";
const SCOPE_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_EMIT_CAPACITY: usize = 256;
const WORKER_NAME: &str = "maos-otel-export";

#[derive(Debug)]
struct BoundedExportState {
    drop_count: AtomicU64,
    queued_spans: AtomicUsize,
    exported_spans: AtomicUsize,
    paused: Mutex<bool>,
    pause_cv: Condvar,
    exported_notify: tokio::sync::Notify,
    last_export_error: Mutex<Option<String>>,
    worker_running: AtomicBool,
}

impl BoundedExportState {
    fn new() -> Self {
        Self {
            drop_count: AtomicU64::new(0),
            queued_spans: AtomicUsize::new(0),
            exported_spans: AtomicUsize::new(0),
            paused: Mutex::new(false),
            pause_cv: Condvar::new(),
            exported_notify: tokio::sync::Notify::new(),
            last_export_error: Mutex::new(None),
            worker_running: AtomicBool::new(true),
        }
    }

    fn wait_until_resumed(&self) {
        let mut paused = self.paused.lock().unwrap_or_else(|p| p.into_inner());
        while *paused {
            paused = self.pause_cv.wait(paused).unwrap_or_else(|p| p.into_inner());
        }
    }

    fn record_export_result(&self, exported: usize, result: OTelSdkResult) {
        self.queued_spans.fetch_sub(exported, Ordering::AcqRel);
        match result {
            Ok(()) => {
                self.exported_spans.fetch_add(exported, Ordering::AcqRel);
                self.exported_notify.notify_waiters();
            }
            Err(err) => {
                *self
                    .last_export_error
                    .lock()
                    .unwrap_or_else(|p| p.into_inner()) = Some(err.to_string());
                self.exported_notify.notify_waiters();
            }
        }
    }
}

/// Test/diagnostic handle for the bounded exporter path.
#[derive(Clone)]
pub struct BoundedExportProbe {
    shared: Arc<BoundedExportState>,
}

impl BoundedExportProbe {
    pub fn drop_count(&self) -> u64 {
        self.shared.drop_count.load(Ordering::Acquire)
    }

    pub fn queued_spans(&self) -> usize {
        self.shared.queued_spans.load(Ordering::Acquire)
    }

    pub fn exported_spans(&self) -> usize {
        self.shared.exported_spans.load(Ordering::Acquire)
    }

    pub fn pause_consumer(&self) {
        *self.shared.paused.lock().unwrap_or_else(|p| p.into_inner()) = true;
    }

    pub fn resume_consumer(&self) {
        let mut paused = self.shared.paused.lock().unwrap_or_else(|p| p.into_inner());
        *paused = false;
        self.shared.pause_cv.notify_all();
    }

    pub async fn wait_until_exported(&self, expected: usize) {
        while self.exported_spans() < expected {
            self.shared.exported_notify.notified().await;
        }
    }

    pub fn last_export_error(&self) -> Option<String> {
        self.shared
            .last_export_error
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn worker_running(&self) -> bool {
        self.shared.worker_running.load(Ordering::Acquire)
    }
}

impl std::fmt::Debug for BoundedExportProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedExportProbe")
            .field("drop_count", &self.drop_count())
            .field("queued_spans", &self.queued_spans())
            .field("exported_spans", &self.exported_spans())
            .field("worker_running", &self.worker_running())
            .finish()
    }
}

enum QueueMessage {
    Batch(Vec<SpanData>),
    Flush(std::sync::mpsc::SyncSender<OTelSdkResult>),
    Shutdown {
        timeout: Duration,
        reply: std::sync::mpsc::SyncSender<OTelSdkResult>,
    },
}

impl std::fmt::Debug for QueueMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Batch(batch) => f
                .debug_tuple("Batch")
                .field(&format_args!("{} spans", batch.len()))
                .finish(),
            Self::Flush(_) => f.write_str("Flush(..)"),
            Self::Shutdown { timeout, .. } => f
                .debug_struct("Shutdown")
                .field("timeout", timeout)
                .finish(),
        }
    }
}

#[derive(Debug)]
struct QueueingSpanExporter {
    tx: tokio::sync::mpsc::Sender<QueueMessage>,
    shared: Arc<BoundedExportState>,
}

impl QueueingSpanExporter {
    fn control_send_error(label: &str) -> OTelSdkError {
        OTelSdkError::InternalFailure(format!("{label} control send failed"))
    }

    fn control_recv_error(label: &str) -> OTelSdkError {
        OTelSdkError::InternalFailure(format!("{label} control reply failed"))
    }

    fn send_control(&self, mut message: QueueMessage, label: &str, timeout: Duration) -> OTelSdkResult {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            match self.tx.try_send(message) {
                Ok(()) => return Ok(()),
                Err(tokio::sync::mpsc::error::TrySendError::Full(returned)) => {
                    if std::time::Instant::now() >= deadline {
                        return Err(Self::control_send_error(label));
                    }
                    message = returned;
                    std::thread::yield_now();
                }
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => return Ok(()),
            }
        }
    }
}

impl SpanExporter for QueueingSpanExporter {
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let span_count = batch.len();
        if span_count == 0 {
            return Ok(());
        }

        self.shared.queued_spans.fetch_add(span_count, Ordering::AcqRel);
        match self.tx.try_send(QueueMessage::Batch(batch)) {
            Ok(()) => Ok(()),
            Err(_err) => {
                self.shared.queued_spans.fetch_sub(span_count, Ordering::AcqRel);
                self.shared
                    .drop_count
                    .fetch_add(span_count as u64, Ordering::AcqRel);
                Ok(())
            }
        }
    }

    fn force_flush(&self) -> OTelSdkResult {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        self.send_control(
            QueueMessage::Flush(reply_tx),
            "force_flush",
            Duration::from_secs(5),
        )?;
        reply_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| Self::control_recv_error("force_flush"))?
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
        self.send_control(
            QueueMessage::Shutdown {
                timeout,
                reply: reply_tx,
            },
            "shutdown",
            timeout,
        )?;
        reply_rx
            .recv_timeout(timeout)
            .map_err(|_| Self::control_recv_error("shutdown"))?
    }
}

fn spawn_export_worker<E: SpanExporter + 'static>(
    exporter: E,
    mut rx: tokio::sync::mpsc::Receiver<QueueMessage>,
    shared: Arc<BoundedExportState>,
) {
    let thread_shared = Arc::clone(&shared);
    std::thread::Builder::new()
        .name(WORKER_NAME.into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("OTel worker runtime must build");
            let exporter = exporter;
            while let Some(message) = {
                thread_shared.wait_until_resumed();
                rx.blocking_recv()
            } {
                match message {
                    QueueMessage::Batch(batch) => {
                        let exported = batch.len();
                        let result = runtime.block_on(exporter.export(batch));
                        thread_shared.record_export_result(exported, result);
                    }
                    QueueMessage::Flush(reply) => {
                        let _ = reply.send(exporter.force_flush());
                    }
                    QueueMessage::Shutdown { timeout, reply } => {
                        let _ = reply.send(exporter.shutdown_with_timeout(timeout));
                        break;
                    }
                }
            }
            thread_shared.worker_running.store(false, Ordering::Release);
            thread_shared.exported_notify.notify_waiters();
        })
        .expect("OTel export worker thread must spawn");
}

/// OTel-backed trace sink.
///
/// Holds a [`TracerProvider`] that writes completed spans to a bounded queueing
/// exporter. When the queue is full, the exporter's `try_send` drops the span,
/// increments `drop_count`, and returns immediately (AC-3).
pub struct OtelTraceSink {
    tracer: opentelemetry_sdk::trace::Tracer,
    _provider: SdkTracerProvider,
    config: OtelTraceSinkConfig,
    probe: BoundedExportProbe,
}

impl OtelTraceSink {
    /// Create a sink backed by the given span exporter.
    ///
    /// # No default endpoint (M3)
    ///
    /// This constructor takes an explicit exporter — it never defaults
    /// to a collector endpoint. For tests, pass the SDK's in-memory
    /// `InMemorySpanExporter`. For production, pass an OTLP exporter
    /// configured with an explicit endpoint URL.
    pub fn new(exporter: impl SpanExporter + 'static, config: OtelTraceSinkConfig) -> Self {
        Self::build(exporter, config, DEFAULT_EMIT_CAPACITY).0
    }

    /// Create a sink with an explicit bounded export capacity and a probe for
    /// structural AC-3 assertions.
    pub fn with_bounded_channel(
        exporter: impl SpanExporter + 'static,
        config: OtelTraceSinkConfig,
        capacity: usize,
    ) -> (Self, BoundedExportProbe) {
        Self::build(exporter, config, capacity)
    }

    fn build(
        exporter: impl SpanExporter + 'static,
        config: OtelTraceSinkConfig,
        capacity: usize,
    ) -> (Self, BoundedExportProbe) {
        let shared = Arc::new(BoundedExportState::new());
        let (tx, rx) = tokio::sync::mpsc::channel(capacity.max(1));
        spawn_export_worker(exporter, rx, Arc::clone(&shared));
        let probe = BoundedExportProbe { shared };
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(QueueingSpanExporter {
                tx,
                shared: Arc::clone(&probe.shared),
            })
            .build();
        let tracer = provider.tracer_with_scope(
            opentelemetry::InstrumentationScope::builder(SCOPE_NAME)
                .with_version(SCOPE_VERSION)
                .build(),
        );
        (
            Self {
                tracer,
                _provider: provider,
                config,
                probe: probe.clone(),
            },
            probe,
        )
    }

    /// AC-3 — number of spans dropped due to a full bounded queue.
    pub fn drop_count(&self) -> u64 {
        self.probe.drop_count()
    }

    /// Access the bounded-export probe.
    pub fn probe(&self) -> &BoundedExportProbe {
        &self.probe
    }

    /// Format a `[u8; 16]` as a hex string for span attributes.
    fn hex16(bytes: &[u8; 16]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Build the base attributes shared by all span kinds (R2-6).
    fn base_attrs(&self) -> Vec<KeyValue> {
        vec![
            KeyValue::new("service.name", self.config.service_name.clone()),
            KeyValue::new(
                "service.instance.id",
                self.config.service_instance_id.clone(),
            ),
            KeyValue::new("otel.scope.name", SCOPE_NAME),
            KeyValue::new("otel.scope.version", SCOPE_VERSION),
        ]
    }
}

impl TraceSink for OtelTraceSink {
    fn iac_frame_span(&self, attrs: IacFrameSpanAttrs) -> SpanGuard {
        let mut kv = self.base_attrs();
        kv.push(KeyValue::new("maos.frame_id", Self::hex16(&attrs.frame_id)));
        kv.push(KeyValue::new("maos.frame_kind", attrs.kind));
        kv.push(KeyValue::new("maos.intent", attrs.intent));

        let span_builder = SpanBuilder::from_name("maos.iac_frame")
            .with_kind(SpanKind::Internal)
            .with_attributes(kv);

        let span = self.tracer.build_with_context(span_builder, &Context::new());

        use opentelemetry::trace::Span as _;
        let otel_span_ctx = span.span_context().clone();
        let ctx = SpanContext {
            trace_id: otel_span_ctx.trace_id().to_bytes(),
            span_id: otel_span_ctx.span_id().to_bytes(),
        };

        let span = Arc::new(Mutex::new(Some(span)));
        let span_clone = Arc::clone(&span);
        SpanGuard::new(ctx, move || {
            if let Some(span) = span_clone.lock().unwrap_or_else(|p| p.into_inner()).take() {
                drop(span);
            }
        })
    }

    fn capability_span(&self, parent: &SpanContext, attrs: CapabilitySpanAttrs) -> SpanGuard {
        let mut kv = self.base_attrs();
        kv.push(KeyValue::new("maos.scope_label", attrs.scope_label));
        kv.push(KeyValue::new("maos.spirit_pid", attrs.spirit_pid as i64));

        let parent_span_ctx = opentelemetry::trace::SpanContext::new(
            opentelemetry::trace::TraceId::from_bytes(parent.trace_id),
            SpanId::from_bytes(parent.span_id),
            opentelemetry::trace::TraceFlags::SAMPLED,
            false,
            opentelemetry::trace::TraceState::default(),
        );
        let parent_otel_ctx = Context::new().with_remote_span_context(parent_span_ctx);

        let span_builder = SpanBuilder::from_name("maos.capability")
            .with_kind(SpanKind::Internal)
            .with_attributes(kv);

        let span = self
            .tracer
            .build_with_context(span_builder, &parent_otel_ctx);

        use opentelemetry::trace::Span as _;
        let otel_span_ctx = span.span_context().clone();
        let ctx = SpanContext {
            trace_id: otel_span_ctx.trace_id().to_bytes(),
            span_id: otel_span_ctx.span_id().to_bytes(),
        };

        let span = Arc::new(Mutex::new(Some(span)));
        let span_clone = Arc::clone(&span);
        SpanGuard::new(ctx, move || {
            if let Some(span) = span_clone.lock().unwrap_or_else(|p| p.into_inner()).take() {
                drop(span);
            }
        })
    }

    fn halt_event(&self, attrs: HaltSpanAttrs) {
        let mut kv = self.base_attrs();
        kv.push(KeyValue::new("maos.halt_id", attrs.halt_id));
        kv.push(KeyValue::new("maos.tag", attrs.tag));
        kv.push(KeyValue::new("maos.predicate_kind", attrs.predicate_kind));
        kv.push(KeyValue::new("maos.value_band", attrs.value_band));
        kv.push(KeyValue::new(
            "maos.threshold",
            attrs
                .threshold
                .map(|threshold| threshold.to_string())
                .unwrap_or_else(|| "none".to_string()),
        ));
        kv.push(KeyValue::new("maos.frame_id", Self::hex16(&attrs.frame_id)));

        let span_builder = SpanBuilder::from_name("maos.halt")
            .with_kind(SpanKind::Internal)
            .with_attributes(kv);

        let mut span = self.tracer.build_with_context(span_builder, &Context::new());
        use opentelemetry::trace::Span;
        span.set_status(Status::error("halt"));
        drop(span);
    }
}

impl std::fmt::Debug for OtelTraceSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OtelTraceSink")
            .field("config", &self.config)
            .field("probe", &self.probe)
            .finish()
    }
}
