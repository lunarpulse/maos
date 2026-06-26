#![forbid(unsafe_code)]

use maos_domain::ports::trace_sink::{IacFrameSpanAttrs, TraceSink};
use maos_telemetry::{OtelTraceSink, OtelTraceSinkConfig};
use opentelemetry_sdk::trace::InMemorySpanExporter;

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "off".to_string());
    match mode.as_str() {
        "off" => {}
        "on" => {
            let sink = OtelTraceSink::new(
                InMemorySpanExporter::default(),
                OtelTraceSinkConfig::default(),
            );
            let guard = sink.iac_frame_span(IacFrameSpanAttrs {
                frame_id: [0u8; 16],
                kind: "task_assign",
                intent: "standard",
            });
            drop(guard);
        }
        other => panic!("unknown fixture mode: {other}"),
    }
}
