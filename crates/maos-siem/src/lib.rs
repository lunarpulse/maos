#![forbid(unsafe_code)]

//! `maos-siem` — read-only Transparency Log projection for SIEM export
//! (Story 11.4c Task 4, ADR-051 / NFR-Aud-11 second half).
//!
//! # Redaction boundary
//!
//! [`export_from_tl`] is the SANCTIONED, redaction-applying entry: it routes
//! every TL row through `maos_audit::query_with_redaction` before projecting
//! it into the NDJSON / CEF / RFC5424 transport frames. The lower-level
//! [`project`] is crate-private, so external callers cannot bypass redaction —
//! they go through [`export_from_tl`] or the [`SiemProjectionPort`] impl.
//!
//! # Sinks
//!
//! The shipped sink is a LOCALHOST-ONLY file appender ([`forward_to_file`]).
//! Network / HTTPS SIEM collectors are additive-deferred — when introduced they
//! MUST be TLS-only and live behind a feature flag (ADR-051 / NFR-Aud-11).

#[cfg(all(feature = "siem-fault-inject", not(debug_assertions)))]
compile_error!("siem-fault-inject is dev/CI-only and MUST NOT ship in release builds");

use std::path::Path;

#[cfg(not(feature = "siem-fault-inject"))]
use maos_audit::{query, query_with_redaction, AuditEntry, AuditFilter};
#[cfg(feature = "siem-fault-inject")]
use maos_audit::{query, AuditEntry, AuditFilter};
use maos_domain::ports::{SiemProjectionError, SiemProjectionPort};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedRecord {
    pub ndjson: String,
    pub cef: String,
    pub rfc5424: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    pub forwarded_count: Option<usize>,
}

#[derive(Debug, thiserror::Error)]
pub enum SiemError {
    #[error("SIEM projection encode failed: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("SIEM audit read failed: {0}")]
    Audit(#[from] maos_audit::AuditError),
    /// Local file-sink I/O failure. Surface, never silently drop — buffering /
    /// backpressure is the caller's responsibility (wired in `maos-bin`).
    #[error("SIEM sink I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Localhost-only SIEM exporter. Implements [`SiemProjectionPort`] by driving
/// the (redaction-applying) projection over already-redacted audit entries.
///
/// The shipped sink is a local-file appender — see [`forward_to_file`].
/// Network / HTTPS collectors are additive-deferred and MUST be TLS-only when
/// introduced.
#[derive(Debug, Clone, Default)]
pub struct SiemExporter;

impl SiemProjectionPort for SiemExporter {
    fn project_redacted_entry(
        &self,
        redacted_entry_json: &str,
    ) -> Result<String, SiemProjectionError> {
        let entry: AuditEntry = serde_json::from_str(redacted_entry_json)
            .map_err(|e| SiemProjectionError::Projection(format!("decode redacted entry: {e}")))?;
        let record = project_one(&entry)
            .map_err(|e| SiemProjectionError::Projection(format!("project entry: {e}")))?;
        // The RFC5424-framed CEF line is the syslog transport frame.
        Ok(record.rfc5424)
    }

    fn is_healthy(&self) -> bool {
        // The localhost file sink is unconditionally usable from the projection
        // layer's perspective; sink reachability (file writable) is enforced at
        // write time by `forward_to_file`, which surfaces I/O errors rather than
        // silently dropping records.
        true
    }
}

/// The SANCTIONED, redaction-applying export entry.
///
/// Reads the (quiesced) Transparency Log through `maos_audit::query_with_redaction`
/// — the ONLY sanctioned populator of `AuditEntry::redaction` provenance — and
/// projects each row into NDJSON + CEF + RFC5424 frames.
#[cfg(not(feature = "siem-fault-inject"))]
pub fn export_from_tl(
    db_path: &Path,
    filter: AuditFilter,
) -> Result<Vec<ProjectedRecord>, SiemError> {
    let entries = query_with_redaction(db_path, filter)?;
    project(&entries)
}

/// FAULT-INJECT BYPASS variant of [`export_from_tl`].
///
/// Under the `siem-fault-inject` feature this DELIBERATELY routes through plain
/// `query(...)` instead of `query_with_redaction(...)`, dropping the redaction
/// provenance metadata so a leak regression suite can invert the guarantee. The
/// feature is dev/CI-only — the `compile_error!` guard above blocks any release
/// build that turns it on.
#[cfg(feature = "siem-fault-inject")]
pub fn export_from_tl(
    db_path: &Path,
    filter: AuditFilter,
) -> Result<Vec<ProjectedRecord>, SiemError> {
    let entries = query(db_path, filter)?;
    project(&entries)
}

/// Reconcile a forward report from a real TL tail.
///
/// `forwarded_count` distinguishes the two "zero projected" cases that were
/// previously conflated:
///   - `None` — the TL is genuinely empty (zero rows at all): reported N/A,
///     never a vacuous green zero.
///   - `Some(n)` — the TL is non-empty; `n` is the count of rows the filter
///     matched and projected (which MAY be `0` when a non-empty TL matches
///     nothing).
pub fn export_report_from_tl(
    db_path: &Path,
    filter: AuditFilter,
) -> Result<ExportReport, SiemError> {
    let records = export_from_tl(db_path, filter)?;

    // Distinguish an empty TL from a non-empty TL whose filter matched zero
    // rows. Probe with a 1-row UNFILTERED read so we never pull the whole log
    // just to count: if any row exists the TL is non-empty.
    let mut probe = AuditFilter::default();
    probe.limit = Some(1);
    let tl_has_any_rows = !query(db_path, probe)?.is_empty();

    let forwarded_count = if tl_has_any_rows {
        Some(records.len())
    } else {
        None
    };
    Ok(ExportReport { forwarded_count })
}

/// Project already-read audit entries into transport frames. Crate-private:
/// external callers MUST go through [`export_from_tl`] (redaction-applying) or
/// the [`SiemProjectionPort`] impl so redaction is never bypassable.
pub(crate) fn project(entries: &[AuditEntry]) -> Result<Vec<ProjectedRecord>, SiemError> {
    entries.iter().map(project_one).collect()
}

fn project_one(entry: &AuditEntry) -> Result<ProjectedRecord, SiemError> {
    let ndjson = serde_json::to_string(entry)?;
    let cef = to_cef(entry);
    let rfc5424 = to_rfc5424(entry, &cef);
    Ok(ProjectedRecord {
        ndjson,
        cef,
        rfc5424,
    })
}

fn to_cef(entry: &AuditEntry) -> String {
    let signature = cef_escape(&entry.kind);
    let name = cef_escape(&entry.intent);
    let payload = cef_escape(&entry.payload);
    let severity = derive_cef_severity(&entry.kind);
    format!(
        "CEF:0|MAOS|maos-siem|0.1|{signature}|{name}|{severity}|rt={} sproc={} cs1Label=kind cs1={} msg={payload}",
        entry.timestamp_ns, entry.spirit_pid, signature,
    )
}

fn to_rfc5424(entry: &AuditEntry, message: &str) -> String {
    // PRI 134 = facility local0 (16) * 8 + severity info (6).
    // TIMESTAMP: ISO-8601 UTC derived from the entry's epoch-nanosecond clock.
    // HOSTNAME: from MAOS_SIEM_HOSTNAME or a constant default (never nil `-`).
    let timestamp = rfc3339_from_nanos(entry.timestamp_ns);
    let hostname = siem_hostname();
    format!(
        "<134>1 {timestamp} {hostname} maos-siem {} ID{} - {}",
        entry.spirit_pid, entry.timestamp_ns, message,
    )
}

/// Hostname for the RFC5424 frame. Precedence: `MAOS_SIEM_HOSTNAME` env, then a
/// constant default. Never nil — RFC5424 permits `-` but a concrete hostname
/// keeps the frame self-describing for downstream collectors.
fn siem_hostname() -> String {
    /// Fallback hostname when `MAOS_SIEM_HOSTNAME` is unset/invalid.
    const DEFAULT_SIEM_HOSTNAME: &str = "localhost";
    std::env::var("MAOS_SIEM_HOSTNAME").unwrap_or_else(|_| DEFAULT_SIEM_HOSTNAME.to_string())
}

/// Derive a CEF severity (0–10, 10 = highest) from the audit kind.
///
/// Documented heuristic default — NOT a per-event constant. Higher severity is
/// assigned to enforcement / violation kinds (block / deny / revoke / …),
/// medium to failure kinds, low otherwise. This is the SIEM's coarse triage
/// signal; the authoritative disposition still lives in the audit record.
fn derive_cef_severity(kind: &str) -> u8 {
    const HIGH: &[&str] = &[
        "block",
        "deny",
        "denied",
        "revoke",
        "revok",
        "violation",
        "breach",
        "evict",
        "kill",
        "crash",
        "oom",
    ];
    const MEDIUM: &[&str] = &["error", "fail", "abort", "halt", "panic", "fatal"];
    let k = kind.to_ascii_lowercase();
    if HIGH.iter().any(|t| k.contains(t)) {
        8
    } else if MEDIUM.iter().any(|t| k.contains(t)) {
        6
    } else if k.contains("warn") {
        5
    } else {
        3
    }
}

/// Format epoch nanoseconds as an RFC3339 / ISO-8601 UTC timestamp.
///
/// `YYYY-MM-DDTHH:MM:SS.fffffffffZ`. Uses Howard Hinnant's `civil_from_days`
/// algorithm — integer-only, no `chrono`/`time` dependency, valid for all
/// non-negative epoch seconds.
fn rfc3339_from_nanos(timestamp_ns: u64) -> String {
    const NANOS_PER_SEC: u64 = 1_000_000_000;
    const SECS_PER_DAY: u64 = 86_400;

    let secs = timestamp_ns / NANOS_PER_SEC;
    let nanos = timestamp_ns % NANOS_PER_SEC;
    let days = (secs / SECS_PER_DAY) as i64;
    let secs_in_day = secs % SECS_PER_DAY;

    let (year, month, day) = civil_from_days(days);
    let hour = secs_in_day / 3600;
    let minute = (secs_in_day % 3600) / 60;
    let second = secs_in_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{nanos:09}Z")
}

/// `civil_from_days` — Howard Hinnant (public domain). Days since 1970-01-01 →
/// (proleptic Gregorian year, month [1-12], day [1-31]).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year, m, d)
}

/// Escape an audit field for safe embedding in a CEF extension.
///
/// Applies the CEF backslash-escapes plus:
///   - space → `\s` (CEF-standard backslash-space) so no raw space inside a
///     value can collide with the extension-separator space, and
///   - EVERY control byte (0x00–0x1F and 0x7F, including NUL) → `\xNN`, so no
///     raw control byte survives into a `msg=` value or the RFC5424 frame.
///
/// `AuditEntry.payload` originates from `String::from_utf8_lossy`, so NUL and
/// other control bytes can be present and MUST be neutralised here.
fn cef_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '|' => out.push_str(r"\|"),
            '=' => out.push_str(r"\="),
            ' ' => out.push_str(r"\s"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            c if (c as u32) <= 0x1F || (c as u32) == 0x7F => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// LOCALHOST-ONLY file sink.
///
/// Tails the (quiesced) Transparency Log read-only via the sanctioned
/// [`export_from_tl`] (redaction applied), then APPENDS one RFC5424-framed CEF
/// line per projected record to `sink_path` (creating the file if absent).
/// Returns the number of records appended.
///
/// On I/O error this returns [`SiemError::Io`] — it does NOT silently drop
/// records; buffering / backpressure is the caller's responsibility (wired in
/// `maos-bin`). Network / HTTPS SIEM collectors are deferred and MUST be
/// TLS-only when introduced.
pub fn forward_to_file(
    db_path: &Path,
    filter: AuditFilter,
    sink_path: &Path,
) -> Result<usize, SiemError> {
    let records = export_from_tl(db_path, filter)?;
    append_records_to_file(&records, sink_path)?;
    Ok(records.len())
}

fn append_records_to_file(records: &[ProjectedRecord], sink_path: &Path) -> Result<(), SiemError> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(sink_path)?;
    for record in records {
        // One RFC5424-framed CEF line per record (the syslog transport frame).
        file.write_all(record.rfc5424.as_bytes())?;
        file.write_all(b"\n")?;
    }
    file.flush()?;
    Ok(())
}

// Compile-time pin: SiemProjectionPort must remain object-safe (maos-bin holds
// it as `Arc<dyn SiemProjectionPort>`).
const _: fn() = || {
    fn _needs_object_safe_projection_port(_: &dyn SiemProjectionPort) {}
};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(kind: &str, pid: u32, ts_ns: u64, payload: &str) -> AuditEntry {
        AuditEntry {
            frame_id_hex: format!("{:032x}", ts_ns),
            timestamp_ns: ts_ns,
            spirit_pid: pid,
            boot_nonce: 7,
            capability_token_hex: None,
            kind: kind.to_string(),
            intent: format!("intent-{pid}"),
            payload: payload.to_string(),
            redaction: None,
        }
    }

    fn sample_entries() -> Vec<AuditEntry> {
        vec![
            sample_entry("task.assign", 42, 1_000_000_000, r#"{"v":1}"#),
            sample_entry("sandbox.block", 43, 2_000_000_000, r#"{"reason":"denied"}"#),
        ]
    }

    // ── Format projection (moved inline: `project` is now crate-private) ──

    #[test]
    fn ndjson_records_are_single_line_valid_json_preserving_audit_fields() {
        let entries = sample_entries();
        let records: Vec<ProjectedRecord> =
            project(&entries).expect("projection must succeed for well-formed entries");

        assert_eq!(
            records.len(),
            entries.len(),
            "one projected record per input entry — no silent merge or drop"
        );

        for (rec, entry) in records.iter().zip(entries.iter()) {
            let line = rec.ndjson.trim_end_matches(['\n', '\r']);
            assert!(!line.is_empty(), "NDJSON record must not be empty");
            assert!(
                !line.contains('\n'),
                "NDJSON record must be exactly one line (newline-delimited JSON)"
            );
            let value: serde_json::Value =
                serde_json::from_str(line).expect("NDJSON record must be valid JSON");
            assert_eq!(value["kind"], serde_json::json!(entry.kind));
            assert_eq!(value["spirit_pid"], serde_json::json!(entry.spirit_pid));
        }
    }

    #[test]
    fn cef_records_carry_the_arc_cef_header_shape() {
        let entries = sample_entries();
        let records = project(&entries).expect("projection succeeds");

        for rec in &records {
            assert!(
                rec.cef.starts_with("CEF:"),
                "CEF record must begin with the CEF header prefix"
            );
            let pipes = rec.cef.matches('|').count();
            assert!(
                pipes >= 7,
                "CEF header must carry at least 7 '|' separators (8 header fields), got {pipes}"
            );
        }

        assert_ne!(
            records[0].cef, records[1].cef,
            "distinct audit entries must project to distinct CEF lines"
        );
    }

    #[test]
    fn rfc5424_framing_wraps_each_record_for_syslog_transport() {
        let entries = sample_entries();
        let records = project(&entries).expect("projection succeeds");

        for rec in &records {
            let first = rec
                .rfc5424
                .chars()
                .next()
                .expect("RFC5424 record must be non-empty");
            assert_eq!(
                first, '<',
                "RFC5424 frame must open with '<' (start of PRI)"
            );

            let close = rec
                .rfc5424
                .find('>')
                .expect("RFC5424 PRI must terminate with '>'");
            let pri: u32 = rec.rfc5424[1..close]
                .parse()
                .expect("RFC5424 PRI must be numeric (facility*8 + severity)");
            assert!(
                pri <= 191,
                "RFC5424 PRI must be a valid syslog priority (<=191), got {pri}"
            );

            let after_pri = &rec.rfc5424[close + 1..];
            assert!(
                after_pri.starts_with("1 "),
                "RFC5424 VERSION must be 1 followed by a space, got: {after_pri:?}"
            );
        }

        assert_ne!(
            records[0].rfc5424, records[1].rfc5424,
            "distinct audit entries must produce distinct RFC5424 frames"
        );
    }

    // ── Patch 3: CEF / RFC5424 sanitisation ───────────────────────────────

    #[test]
    fn cef_escape_neutralises_nul_control_bytes_and_spaces_in_payload() {
        // NUL, a raw space, a tab, and a DEL (0x7F) — none may survive raw into
        // the CEF msg= value or the RFC5424 frame.
        let entry = sample_entry("task.assign", 9, 3_000_000_000, "a\0b c\td\x7fe");
        let rec = &project(&[entry]).expect("projection succeeds")[0];

        // No raw NUL anywhere in the content / transport frames.
        for (label, frame) in [
            ("ndjson", rec.ndjson.as_str()),
            ("cef", rec.cef.as_str()),
            ("rfc5424", rec.rfc5424.as_str()),
        ] {
            assert!(
                !frame.contains('\0'),
                "{label} frame leaked a raw NUL byte: {frame:?}"
            );
        }

        // The CEF `msg=` value: no raw space / tab; controls render as hex.
        let msg = rec
            .cef
            .split("msg=")
            .nth(1)
            .expect("CEF must carry a msg= extension");
        assert!(
            !msg.contains(' '),
            "CEF msg= must not contain a raw unescaped space (space → \\s): {msg:?}"
        );
        assert!(
            !msg.contains('\t'),
            "CEF msg= must not contain a raw tab: {msg:?}"
        );
        assert!(msg.contains("\\x00"), "NUL must render as \\x00: {msg:?}");
        assert!(msg.contains("\\x7f"), "DEL must render as \\x7f: {msg:?}");
        assert!(msg.contains(r"\s"), "space must render as \\s: {msg:?}");

        assert!(
            !rec.rfc5424.contains('\t'),
            "RFC5424 frame must not contain a raw tab: {:?}",
            rec.rfc5424
        );
    }

    // ── Patch 5: RFC5424 TIMESTAMP / HOSTNAME + CEF severity ──────────────

    #[test]
    fn rfc3339_from_nanos_pins_known_epochs() {
        assert_eq!(
            rfc3339_from_nanos(0),
            "1970-01-01T00:00:00.000000000Z",
            "the UNIX epoch must format to 1970-01-01T00:00:00Z"
        );
        // 1_672_531_200 s = 2023-01-01T00:00:00Z.
        assert_eq!(
            rfc3339_from_nanos(1_672_531_200_000_000_000),
            "2023-01-01T00:00:00.000000000Z",
        );
        // 1 ns past the epoch carries the full 9-digit fractional second.
        assert_eq!(rfc3339_from_nanos(1), "1970-01-01T00:00:00.000000001Z",);
    }

    #[test]
    fn rfc5424_frame_carries_real_timestamp_and_hostname_not_nil() {
        let entry = sample_entry("task.assign", 42, 1_672_531_200_000_000_000, "{}");
        let frame = project(&[entry]).expect("projection succeeds")[0]
            .rfc5424
            .clone();

        // <PRI>VERSION SP TIMESTAMP SP HOSTNAME SP APP-NAME ...
        let rest = frame.strip_prefix("<134>1 ").expect("PRI+VERSION prefix");
        let mut it = rest.splitn(3, ' ');
        let timestamp = it.next().unwrap();
        let hostname = it.next().unwrap();
        assert_ne!(timestamp, "-", "RFC5424 TIMESTAMP must not be nil");
        assert!(
            timestamp.starts_with("2023-01-01T") && timestamp.ends_with('Z'),
            "RFC5424 TIMESTAMP must be ISO-8601 UTC: {timestamp}"
        );
        assert_ne!(hostname, "-", "RFC5424 HOSTNAME must not be nil");
        assert!(!hostname.is_empty(), "RFC5424 HOSTNAME must be non-empty");
    }

    #[test]
    fn cef_severity_is_derived_from_kind_not_a_constant() {
        fn severity_of(cef: &str) -> &str {
            // Severity is the 7th pipe-delimited CEF field (index 6 after CEF:V).
            cef.split('|').nth(6).unwrap()
        }
        let blocked = project(&[sample_entry("sandbox.block", 1, 1, "{}")]).unwrap()[0]
            .cef
            .clone();
        let revoked = project(&[sample_entry("capability.revoke", 2, 1, "{}")]).unwrap()[0]
            .cef
            .clone();
        let benign = project(&[sample_entry("task.assign", 3, 1, "{}")]).unwrap()[0]
            .cef
            .clone();

        assert_eq!(severity_of(&blocked), "8", "sandbox.block → high severity");
        assert_eq!(
            severity_of(&revoked),
            "8",
            "capability.revoke → high severity"
        );
        assert_eq!(severity_of(&benign), "3", "task.assign → low severity");
        assert_ne!(
            severity_of(&benign),
            "5",
            "severity must not stay pinned to literal 5"
        );
    }

    // ── Patch 6: SiemProjectionPort impl ──────────────────────────────────

    #[test]
    fn siem_exporter_projects_redacted_entry_json_into_a_syslog_frame() {
        let exporter = SiemExporter;
        assert!(exporter.is_healthy(), "localhost sink is healthy");

        let entry = sample_entry("task.assign", 42, 1_672_531_200_000_000_000, r#"{"v":1}"#);
        let redacted_json = serde_json::to_string(&entry).unwrap();
        let frame = exporter
            .project_redacted_entry(&redacted_json)
            .expect("projection of a redacted entry must succeed");
        assert!(
            frame.starts_with("<134>1 "),
            "SiemProjectionPort must emit an RFC5424 frame, got: {frame}"
        );
        assert!(
            frame.contains("2023-01-01T00:00:00"),
            "projected frame must carry the derived timestamp: {frame}"
        );

        // Malformed JSON surfaces a typed projection error, not a panic.
        let err = exporter.project_redacted_entry("{not json").unwrap_err();
        assert!(
            matches!(err, SiemProjectionError::Projection(_)),
            "malformed redacted entry must surface SiemProjectionError::Projection: {err:?}"
        );
    }
}
