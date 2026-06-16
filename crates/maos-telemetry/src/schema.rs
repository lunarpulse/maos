#![forbid(unsafe_code)]

//! Span-schema single source of truth (AC-6 / R2-7).
//!
//! 9.5b OWNS this table; Story 9.5 docs render FROM it.
//! A test asserts the actually-emitted span names + attr keys match.

/// One entry in the canonical span-schema table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanSchemaEntry {
    pub span_name: &'static str,
    pub kind: &'static str,
    pub required_attrs: &'static [&'static str],
    pub status_rule: &'static str,
}

/// Canonical `{span name, kind, required attrs, status rule}` table (R2-7).
///
/// This is the SINGLE SOURCE OF TRUTH for the three OTel span kinds
/// emitted by the MAOS telemetry adapter.  Story 9.5's docs render
/// from this table — one direction, code authoritative.
pub const SPAN_SCHEMA: &[SpanSchemaEntry] = &[
    SpanSchemaEntry {
        span_name: "maos.iac_frame",
        kind: "INTERNAL",
        required_attrs: &[
            "maos.frame_id",
            "maos.frame_kind",
            "maos.intent",
            "service.name",
            "service.instance.id",
            "otel.scope.name",
            "otel.scope.version",
        ],
        status_rule: "Ok (unset)",
    },
    SpanSchemaEntry {
        span_name: "maos.capability",
        kind: "INTERNAL",
        required_attrs: &[
            "maos.scope_label",
            "maos.spirit_pid",
            "service.name",
            "service.instance.id",
            "otel.scope.name",
            "otel.scope.version",
        ],
        status_rule: "Ok (unset)",
    },
    SpanSchemaEntry {
        span_name: "maos.halt",
        kind: "INTERNAL",
        required_attrs: &[
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
        ],
        status_rule: "Error",
    },
];
