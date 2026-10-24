#![forbid(unsafe_code)]

//! `maos-escape-detector` — out-of-kernel sandbox-escape structural anomaly
//! detector (ADR-024, NFR-Sec-3 signal 1 of 3 @v2.0).
//!
//! A read-only Transparency-Log consumer, identical in shape to `maos-audit`:
//! opens the TL SQLite file with `SQLITE_OPEN_READ_ONLY` and correlates the
//! kernel's raw structural facts (`FrameKind::SandboxBlock = 8`, emitted with
//! `FrameOrigin::Kernel`) against the Spirit's manifest declaration. It raises
//! an **anomaly signal with a structural rationale** on an operator-observable
//! surface. It has **no `maos-kernel-core` dependency** at the library level
//! (ADR-024 Gate; Story-1a.4 decoupling rule; `maos-audit` precedent).
//!
//! # The structural-not-semantic boundary (ADR-024 §2)
//!
//! The kernel contributes ONLY the raw fact — `{ spirit_pid, attempted_syscall,
//! sandbox_tier }`. It carries **no** `malice`/`verdict`/`severity`/`intent`
//! field (AC2). ALL interpretation ("is this an escape? did the manifest
//! anticipate it?") lives HERE, in user-space. The kernel learns no patterns
//! (ADR-006); the detector's correlation logic is auditable, tunable
//! (TP-floor / FP-ceiling), and replaceable without a kernel upgrade.
//!
//! # `escape-fault-inject` (AC3/AC4 falsifier — dev/CI only)
//!
//! The `escape-fault-inject` feature stubs the correlation to canned-TP (an
//! anomaly regardless of the manifest declaration) so the gate can prove the
//! detection-quality + producer-wired legs are derived from the REAL correlation
//! (the flag REMOVES the real logic, the verdict flips — §A7.3). It MUST NOT
//! ship in release builds: the `compile_error!` below hard-fails any release
//! build with the feature on, and the `check-escape-detector` gate's
//! release-graph-absence leg is the belt-and-suspenders graph guard.

// Story 11.4b — `escape-fault-inject` is dev/CI-only. A release build
// (`not(debug_assertions)`) with this feature enabled MUST NOT compile. Mirrors
// the `pdp-fault-inject` / `churn-fault-inject` / `slo-fault-inject` ship-blocker.
#[cfg(all(feature = "escape-fault-inject", not(debug_assertions)))]
compile_error!(
    "escape-fault-inject is a dev/CI-only fault-injection feature and MUST NOT \
     appear in release builds (Story 11.4b ship-blocker)."
);

use std::path::Path;

use rusqlite::OpenFlags;
use serde::{Deserialize, Serialize};

/// The `FrameOrigin` discriminator for a kernel-generated frame
/// (`maos_domain::invariants::i3::FrameOrigin::Kernel = 3`). The escape-source-
/// identity reflex (D8) verifies every correlated anomaly traces to a frame
/// with exactly this origin — a synthesized/injected frame reds it.
pub const FRAME_ORIGIN_KERNEL: i64 = 3;

/// The `FrameKind` discriminator for a sandbox-block frame
/// (`FrameKind::SandboxBlock = 8`).
pub const FRAME_KIND_SANDBOX_BLOCK: i64 = 8;

/// A decoded `SandboxBlock` frame read from the Transparency Log (kind = 8).
///
/// `frame_id_hex` is the 16-byte TL frame id as lowercase hex — the identity a
/// detected anomaly traces back to (D8). `origin` MUST equal
/// [`FRAME_ORIGIN_KERNEL`] for the frame to be a real kernel-emitted fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxBlockFrame {
    pub frame_id_hex: String,
    pub timestamp_ns: i64,
    pub spirit_pid: i64,
    /// `"sandbox.block.<attempted_syscall>"` (writer_task.rs:112).
    pub intent: String,
    /// `"tier=<n>"` (writer_task.rs:113).
    pub payload: String,
    /// The frame origin discriminator; `3` == Kernel.
    pub origin: i64,
}

impl SandboxBlockFrame {
    /// The attempted-syscall label carried by the kernel fact (the
    /// post-mortem classifier cannot name the syscall — L5; it carries a
    /// coarse label like `"unknown"` / `"possible-oom-or-resource-cap"`).
    pub fn attempted_syscall(&self) -> &str {
        extract_syscall(&self.intent)
    }

    /// The observed sandbox tier parsed from `"tier=<n>"`.
    pub fn observed_tier(&self) -> Option<u8> {
        extract_tier(&self.payload)
    }

    /// True iff this frame was emitted by the kernel (`FrameOrigin::Kernel`).
    pub fn is_kernel_origin(&self) -> bool {
        self.origin == FRAME_ORIGIN_KERNEL
    }
}

/// Story 17-10 — one cursor batch of [`Detector::read_kernel_sandbox_blocks_after`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxBlockBatch {
    /// The TL's `MAX(rowid)` in the batch's snapshot — the caller's next cursor.
    pub through_rowid: i64,
    /// This boot's kind-8 rows in `(after_rowid, through_rowid]`, rowid order.
    pub frames: Vec<SandboxBlockFrame>,
}

/// The Spirit manifest declaration the detector correlates against (F3). The
/// detector does NOT read the manifest store itself (it stays a pure, decoupled
/// consumer); the composition root / test supplies the declaration per Spirit.
///
/// The load-bearing field is [`anticipated_kill`]: whether the manifest declared
/// that a sandbox kill at the observed tier was anticipated (e.g. the Spirit
/// declared it would exercise privileged syscalls that T2 denies). This is the
/// axis the **correlation decision** turns on — NOT the deterministic
/// `classify_exit` map (Murat: a TP-rate over `classify_exit` is vacuous, since
/// every SIGSYS→T2 identically). The same real seccomp kill correlated against
/// an anticipated vs. unanticipated manifest yields a different outcome — that
/// contrast is what the detection-quality leg measures.
///
/// [`anticipated_kill`]: ManifestDeclaration::anticipated_kill
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDeclaration {
    pub spirit_pid: i64,
    /// Declared sandbox tier (0=T0 … 2=T2). Informational; the correlation
    /// turns on `anticipated_kill`, not the raw tier number.
    pub declared_tier: u8,
    /// Whether the manifest anticipated a sandbox kill at the observed tier.
    pub anticipated_kill: bool,
}

/// A structural anomaly signal — the detector's output on an operator-
/// observable surface. Carries the **rationale** (ADR-024 §2): the
/// interpretation lives HERE, never in the kernel frame. The report is emitted
/// OUT of the TL (the detector is a consumer, not a producer — ADR-006); the
/// composition root logs it / writes it to an operator report sink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnomalyReport {
    /// The real TL `frame_id` this anomaly traces to (D8 escape-source-identity).
    pub frame_id_hex: String,
    pub spirit_pid: i64,
    /// Human-readable structural rationale (the user-space interpretation).
    pub rationale: String,
    pub attempted_syscall: String,
    pub observed_tier: u8,
    pub declared_tier: u8,
}

/// The correlation decision — the metric the detection-quality leg measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrelationOutcome {
    /// A structural anomaly: a real kernel-emitted sandbox kill that the
    /// manifest did NOT anticipate at the observed tier (a true escape
    /// signature).
    Anomaly(AnomalyReport),
    /// No anomaly: the kill was anticipated by the manifest, or (for the FP
    /// corpus) no kill frame was produced at all.
    NoAnomaly,
}

/// Correlate a single real `SandboxBlock` frame against a manifest declaration.
///
/// This is the **detection decision** — measured by the detection-quality leg,
/// NOT the deterministic `classify_exit` map. An anomaly is raised iff the kill
/// frame is real (kernel-origin) AND the manifest did not anticipate it.
pub fn correlate(frame: &SandboxBlockFrame, manifest: &ManifestDeclaration) -> CorrelationOutcome {
    // AC3/AC4 FALSIFIER — `escape-fault-inject` stubs the correlation to a
    // canned anomaly regardless of the manifest declaration. With it ON, the
    // FP set (anticipated kills / benign completions) anomalizes → the
    // detection-quality FP-ceiling leg REDS, proving the metric is derived from
    // the REAL correlation, not a constant (§A7.3: the flag REMOVES the real
    // logic, the verdict flips). Dev/CI-only; `compile_error!` blocks release.
    #[cfg(feature = "escape-fault-inject")]
    {
        let _ = manifest;
        return CorrelationOutcome::Anomaly(AnomalyReport {
            frame_id_hex: frame.frame_id_hex.clone(),
            spirit_pid: frame.spirit_pid,
            rationale: "escape-fault-inject canned-TP (real correlation bypassed)".to_string(),
            attempted_syscall: frame.attempted_syscall().to_string(),
            observed_tier: frame.observed_tier().unwrap_or(2),
            declared_tier: 0,
        });
    }

    if !frame.is_kernel_origin() {
        // A non-kernel-origin SandboxBlock frame cannot exist in production
        // (only the kernel emits kind=8); treat as no-anomaly rather than
        // honor a synthesized source (the source-identity reflex reds this
        // elsewhere via the blind).
        return CorrelationOutcome::NoAnomaly;
    }

    if manifest.anticipated_kill {
        // The manifest anticipated a kill at this tier → not an escape
        // signature. This is the FP-avoidance path: the SAME real seccomp kill
        // correlated against an anticipated manifest yields NoAnomaly.
        return CorrelationOutcome::NoAnomaly;
    }

    // Story 17-10 (D-17-10-FW-04; review D1, ruled A) — only kill facts the
    // kernel itself observed are correlated (`is_kernel_observed_kill`). Every
    // other kind-8 row is a refusal or a self-report — the T3 Worker's
    // `maos-sandbox-block:` stderr line (`tier=3`, child-chosen label, the
    // process keeps running) and the egress proxy's `sandbox.block.egress.<dest>`
    // row (JSON payload, `maos-bin/src/egress.rs`) — and is never an anomaly.
    if !is_kernel_observed_kill(frame) {
        return CorrelationOutcome::NoAnomaly;
    }
    let observed = KERNEL_KILL_TIER;
    CorrelationOutcome::Anomaly(AnomalyReport {
        frame_id_hex: frame.frame_id_hex.clone(),
        spirit_pid: frame.spirit_pid,
        rationale: format!(
            "sandbox kill at tier={observed} (syscall={}) not anticipated by \
             manifest (declared tier={}); the OS enforcement layer observed a \
             structural violation the Spirit did not declare",
            frame.attempted_syscall(),
            manifest.declared_tier,
        ),
        attempted_syscall: frame.attempted_syscall().to_string(),
        observed_tier: observed,
        declared_tier: manifest.declared_tier,
    })
}

/// The detector: a read-only TL consumer that reads `SandboxBlock` frames,
/// verifies each traces to a real kernel-emitted frame (D8), de-dups replayed
/// `frame_id`s (Boundary — a replay MUST NOT inflate the count), and correlates
/// each against the caller-supplied manifest declaration.
pub struct Detector;

impl Detector {
    /// Read all real kernel-origin `SandboxBlock` frames (kind=8) from the TL,
    /// de-duped by `frame_id` (idempotent under replay — D8), in insertion order.
    ///
    /// Opens the SQLite file **read-only** (`SQLITE_OPEN_READ_ONLY`) — the
    /// detector is a pure consumer (ADR-024 §3; the maos-audit precedent). A
    /// read-only connection sees a transaction-consistent snapshot, so it
    /// tolerates a half-written frame while the writer is live (Boundary's
    /// concurrent-read catch, AC1).
    pub fn read_kernel_sandbox_blocks(
        db_path: &Path,
    ) -> Result<Vec<SandboxBlockFrame>, DetectorError> {
        let conn = open_read_only(db_path)?;
        let mut stmt = conn
            .prepare(
                "SELECT frame_id, timestamp_ns, spirit_pid, intent, payload_redacted, origin \
                 FROM transparency_log \
                 WHERE kind = ?1 \
                 ORDER BY rowid",
            )
            .map_err(DetectorError::Query)?;
        let rows = stmt
            .query_map(rusqlite::params![FRAME_KIND_SANDBOX_BLOCK], decode_row)
            .map_err(DetectorError::Query)?;

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out: Vec<SandboxBlockFrame> = Vec::new();
        for row in rows {
            let frame = row.map_err(DetectorError::Row)?;
            // Idempotent replay-dedup (D8): a replayed/duplicate frame_id MUST
            // NOT inflate the anomaly count — else the source-identity reflex
            // lies. First-wins preserves insertion order.
            if seen.insert(frame.frame_id_hex.clone()) {
                out.push(frame);
            }
        }
        Ok(out)
    }

    /// Story 17-10 (D-17-10-FW-02) — the live tail's cursor read, in ONE read
    /// transaction: `through_rowid` is the TL's `MAX(rowid)` and `frames` the
    /// kind-8 rows of boot `boot_nonce` with `after_rowid < rowid <=
    /// through_rowid`, in rowid order, from the same snapshot. The caller's
    /// next cursor is `through_rowid`, so non-kill rows are never re-walked.
    /// Same read-only connection as [`Self::read_kernel_sandbox_blocks`].
    pub fn read_kernel_sandbox_blocks_after(
        db_path: &Path,
        boot_nonce: u64,
        after_rowid: i64,
    ) -> Result<SandboxBlockBatch, DetectorError> {
        let mut conn = open_read_only(db_path)?;
        let tx = conn.transaction().map_err(DetectorError::Query)?;
        let through_rowid = query_max_rowid(&tx)?;
        let mut stmt = tx
            .prepare(
                "SELECT frame_id, timestamp_ns, spirit_pid, intent, payload_redacted, origin \
                 FROM transparency_log \
                 WHERE kind = ?1 AND boot_nonce = ?2 AND rowid > ?3 AND rowid <= ?4 \
                 ORDER BY rowid",
            )
            .map_err(DetectorError::Query)?;
        let frames = stmt
            .query_map(
                rusqlite::params![
                    FRAME_KIND_SANDBOX_BLOCK,
                    boot_nonce as i64,
                    after_rowid,
                    through_rowid
                ],
                decode_row,
            )
            .map_err(DetectorError::Query)?
            .map(|r| r.map_err(DetectorError::Row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SandboxBlockBatch {
            through_rowid,
            frames,
        })
    }

    /// Story 17-10 — the highest rowid in the TL (0 when empty): the tail's
    /// starting cursor, so rows written before it was spawned are never
    /// surfaced.
    pub fn max_rowid(db_path: &Path) -> Result<i64, DetectorError> {
        query_max_rowid(&open_read_only(db_path)?)
    }

    /// Correlate the real kernel-origin `SandboxBlock` frames in `db_path`
    /// against `manifests` (keyed by `spirit_pid`), returning one
    /// [`AnomalyReport`] per real unanticipated kill. Each anomaly traces to a
    /// real kernel-emitted `frame_id` (D8); replayed frames are already
    /// de-duped by [`read_kernel_sandbox_blocks`].
    pub fn detect(
        db_path: &Path,
        manifests: &[ManifestDeclaration],
    ) -> Result<Vec<AnomalyReport>, DetectorError> {
        let frames = Self::read_kernel_sandbox_blocks(db_path)?;
        let manifest_by_pid: std::collections::HashMap<i64, &ManifestDeclaration> =
            manifests.iter().map(|m| (m.spirit_pid, m)).collect();

        let mut anomalies: Vec<AnomalyReport> = Vec::new();
        for frame in &frames {
            // Only correlate frames that trace to a real kernel emission
            // (D8 source-identity reflex). A synthesized/injected frame reds
            // the source-identity-blind leg; here it is simply not counted.
            if !frame.is_kernel_origin() {
                continue;
            }
            let manifest = match manifest_by_pid.get(&frame.spirit_pid) {
                Some(m) => *m,
                // No manifest declaration for this Spirit — treat as maximally
                // suspicious (no declared scope anticipated any kill). Use a
                // default unanticipated declaration.
                None => {
                    let default = ManifestDeclaration {
                        spirit_pid: frame.spirit_pid,
                        declared_tier: 0,
                        anticipated_kill: false,
                    };
                    if let CorrelationOutcome::Anomaly(report) = correlate(frame, &default) {
                        anomalies.push(report);
                    }
                    continue;
                }
            };
            if let CorrelationOutcome::Anomaly(report) = correlate(frame, manifest) {
                anomalies.push(report);
            }
        }
        Ok(anomalies)
    }
}

/// Render an anomaly report as a single operator-observable log line (Sally's
/// observability constraint — the anomaly goes to a surface a human/Spirit can
/// see, OUT of the TL). The composition root pipes this to stderr / an operator
/// report sink.
pub fn format_anomaly_line(report: &AnomalyReport) -> String {
    format!(
        "escape-anomaly: spirit_pid={} tier={}/declared={} syscall={} frame={} rationale=\"{}\"",
        report.spirit_pid,
        report.observed_tier,
        report.declared_tier,
        report.attempted_syscall,
        report.frame_id_hex,
        report.rationale,
    )
}

// ────────────────────────────── helpers ──────────────────────────────

/// Extract the attempted-syscall label from `"sandbox.block.<syscall>"`.
fn extract_syscall(intent: &str) -> &str {
    intent
        .strip_prefix("sandbox.block.")
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown")
}

/// Extract the tier number from `"tier=<n>"`.
fn extract_tier(payload: &str) -> Option<u8> {
    payload.strip_prefix("tier=").and_then(|n| n.parse().ok())
}

/// Story 17-10 (review D1, ruled A) — the only tier at which the kernel itself
/// observes a sandbox kill: both bridge sites gate on `SandboxTier::T2`
/// (`maos-kernel-core` `lifecycle/cli_wrapper/runtime.rs`, SIGSYS and cgroup
/// OOM) and `security::sandbox::classify_exit` hard-codes T2.
const KERNEL_KILL_TIER: u8 = 2;

/// True iff `frame` is a kill fact the kernel itself observed, at
/// [`KERNEL_KILL_TIER`]. The labels are the kernel producers' own:
/// `seccomp-kill-process (SIGSYS; …)` and `resource-cap: cgroup memory.max
/// oom_kill x<n>` from the bridge's `wait_and_finalize` (cap-audit writer or
/// its direct fallback), `unknown` / `access-violation-possible-sandbox` from
/// `classify_exit` → `emit_sandbox_block` (11.4b). A T3 self-report always
/// carries `tier=3`, whatever label the child chose.
fn is_kernel_observed_kill(frame: &SandboxBlockFrame) -> bool {
    let Some(label) = frame.intent.strip_prefix("sandbox.block.") else {
        return false;
    };
    let kernel_label = label.starts_with("seccomp-kill-process")
        || label.starts_with("resource-cap")
        || label == "unknown"
        || label == "access-violation-possible-sandbox";
    kernel_label && frame.observed_tier() == Some(KERNEL_KILL_TIER)
}

/// Open the TL **read-only** (`SQLITE_OPEN_READ_ONLY`) — the detector is a
/// pure consumer (ADR-024 §3; the maos-audit precedent). A read-only
/// connection sees a transaction-consistent snapshot, so it tolerates a
/// half-written frame while the writer is live.
fn open_read_only(db_path: &Path) -> Result<rusqlite::Connection, DetectorError> {
    rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(DetectorError::Open)
}

/// The one row decoder shared by both reads (columns `frame_id, timestamp_ns,
/// spirit_pid, intent, payload_redacted, origin`): `payload_redacted` holds the
/// writer's `"tier=<n>"` bytes; `origin` the FrameOrigin discriminator.
fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SandboxBlockFrame> {
    let frame_id_blob: Vec<u8> = row.get(0)?;
    let payload_bytes: Vec<u8> = row.get(4)?;
    Ok(SandboxBlockFrame {
        frame_id_hex: hex_encode(&frame_id_blob),
        timestamp_ns: row.get(1)?,
        spirit_pid: row.get(2)?,
        intent: row.get(3)?,
        payload: String::from_utf8_lossy(&payload_bytes).into_owned(),
        origin: row.get(5)?,
    })
}

/// `MAX(rowid)` of the TL (0 when empty) on an open connection/transaction.
fn query_max_rowid(conn: &rusqlite::Connection) -> Result<i64, DetectorError> {
    conn.query_row(
        "SELECT COALESCE(MAX(rowid), 0) FROM transparency_log",
        [],
        |r| r.get(0),
    )
    .map_err(DetectorError::Query)
}

/// Lowercase-hex-encode a byte slice (the 16-byte TL `frame_id` blob). No `hex`
/// crate dep — the detector stays minimal (the maos-audit `hex` dep is for
/// sealed-export; here a 16-byte encode is trivial and dependency-free).
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Typed detector error.
#[derive(Debug, thiserror::Error)]
pub enum DetectorError {
    #[error("sqlite open failed: {0}")]
    Open(rusqlite::Error),
    #[error("sqlite query failed: {0}")]
    Query(rusqlite::Error),
    #[error("sqlite row decode failed: {0}")]
    Row(rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(intent: &str, payload: &str, origin: i64) -> SandboxBlockFrame {
        SandboxBlockFrame {
            frame_id_hex: "abcdef0123456789abcdef0123456789".to_string(),
            timestamp_ns: 1,
            spirit_pid: 42,
            intent: intent.to_string(),
            payload: payload.to_string(),
            origin,
        }
    }

    fn manifest(anticipated: bool) -> ManifestDeclaration {
        ManifestDeclaration {
            spirit_pid: 42,
            declared_tier: 2,
            anticipated_kill: anticipated,
        }
    }

    #[cfg(not(feature = "escape-fault-inject"))]
    #[test]
    fn unanticipated_real_kill_is_anomaly() {
        let f = frame("sandbox.block.unknown", "tier=2", FRAME_ORIGIN_KERNEL);
        let outcome = correlate(&f, &manifest(false));
        assert!(matches!(outcome, CorrelationOutcome::Anomaly(_)));
        if let CorrelationOutcome::Anomaly(r) = outcome {
            assert_eq!(r.attempted_syscall, "unknown");
            assert_eq!(r.observed_tier, 2);
            assert_eq!(r.declared_tier, 2);
            assert!(r.rationale.contains("not anticipated"));
        }
    }

    #[cfg(not(feature = "escape-fault-inject"))]
    #[test]
    fn anticipated_real_kill_is_no_anomaly() {
        // The SAME real seccomp kill, correlated against an anticipated manifest,
        // yields NoAnomaly — the FP-avoidance path (Murat: the metric turns on
        // the correlation, not deterministic classify_exit).
        let f = frame("sandbox.block.unknown", "tier=2", FRAME_ORIGIN_KERNEL);
        let outcome = correlate(&f, &manifest(true));
        assert_eq!(outcome, CorrelationOutcome::NoAnomaly);
    }

    #[cfg(not(feature = "escape-fault-inject"))]
    #[test]
    fn synthesized_non_kernel_frame_is_no_anomaly() {
        let f = frame("sandbox.block.unknown", "tier=2", 0);
        let outcome = correlate(&f, &manifest(false));
        assert_eq!(outcome, CorrelationOutcome::NoAnomaly);
    }

    #[test]
    fn extract_syscall_and_tier_helpers() {
        assert_eq!(extract_syscall("sandbox.block.foo"), "foo");
        assert_eq!(extract_syscall("sandbox.block."), "unknown");
        assert_eq!(extract_syscall("other"), "unknown");
        assert_eq!(extract_tier("tier=2"), Some(2));
        assert_eq!(extract_tier("tier=99"), Some(99));
        assert_eq!(extract_tier("nope"), None);
    }

    #[test]
    fn hex_encode_is_lowercase() {
        assert_eq!(hex_encode(&[0xAB, 0xCD, 0xEF, 0x01]), "abcdef01");
    }
}
