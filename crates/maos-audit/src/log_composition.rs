#![forbid(unsafe_code)]

//! Kernel-side log-composition primitives for FR17 morning digests.
//!
//! Story 3.4 LANDS: ranged read-side composition over the three audit
//! surfaces (Transparency Log + Approval Decision Log + Lifecycle Journal)
//! so digest-shipping Spirits (Butler v0.3 / Researcher v0.5 /
//! Orchestrator v0.8+) consume kernel-mediated access without
//! reimplementing log queries.
//!
//! What the kernel does NOT do here: semantic distillation,
//! summarization, anomaly detection, ranking — all Spirit-side per
//! 4.0.7. This module returns typed rows in a uniform shape; the
//! Spirit decides what to make of them.
//!
//! Story 4.4 (log.recall + I11 audit chain) extends with
//! participant-scoping + A2A consent honoring; 3.4 covers the
//! same-Host director-surface recall path that Butler v0.3 needs.

use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::Path;

use crate::AuditError;

/// A unified row shape across the three log surfaces. The source field
/// is the discriminator; payload variants carry source-specific fields.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComposedLogEntry {
    pub timestamp_ns: u64,
    pub spirit_id: Option<String>,
    pub source: LogSource,
    pub payload: ComposedPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum LogSource {
    TransparencyLog,
    ApprovalDecisionLog,
    LifecycleJournal,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind")]
pub enum ComposedPayload {
    Frame {
        frame_kind: String,
        intent: String,
        #[serde(skip)]
        payload_redacted: Vec<u8>,
    },
    Approval {
        actor: String,
        capability: String,
        intent: String,
        decision: bool,
        reasoning: Option<String>,
    },
    Lifecycle {
        event: String,
        sandbox_tier: Option<u8>,
    },
}

/// Time range for ranged_recall. Nanosecond precision matches the
/// monotonic clock at cap_tokens::monotonic_now_ns.
#[derive(Debug, Clone, Copy)]
pub struct LogRange {
    pub since_ns: u64,
    pub until_ns: u64,
}

impl LogRange {
    /// Convenience: the last 24 hours from now_ns. Used by the FR17
    /// "morning digest" default. Callers MAY supply other ranges.
    pub fn last_24h(now_ns: u64) -> Self {
        let day_ns = 24 * 60 * 60 * 1_000_000_000u64;
        Self {
            since_ns: now_ns.saturating_sub(day_ns),
            until_ns: now_ns,
        }
    }
}

/// Ranged composition over the three log surfaces.
///
/// audit_db — path to the Transparency Log + Approval Decision Log
/// SQLite (shared adapter; see default_transparency_log_path()).
/// journal_path — path to the Lifecycle Journal NDJSON
/// (see default_journal_path()).
/// range — half-open [since_ns, until_ns).
/// spirit_filter — Some(name) scopes to one Spirit's rows; None
/// returns all rows across all Spirits.
///
/// Returns rows in timestamp ascending order (merge-sort across the
/// three sources). The order is the contract Butler v0.3 relies on for
/// digest narrative coherence.
///
/// What this function does NOT do: participant-scoping by IAC frame
/// from/to addressing (that's Story 4.4's log.recall), A2A consent
/// envelope honoring (Story 4.4), or any payload interpretation (always
/// Spirit-side).
pub fn ranged_recall(
    audit_db: &Path,
    journal_path: &Path,
    range: LogRange,
    spirit_filter: Option<&str>,
) -> Result<Vec<ComposedLogEntry>, AuditError> {
    let mut entries = Vec::new();

    // 1. Transparency Log frames
    {
        let filter = crate::AuditFilter {
            since_ns: Some(range.since_ns),
            until_ns: Some(range.until_ns),
            ..Default::default()
        };
        let audit_entries = crate::query(audit_db, filter)?;
        for e in audit_entries {
            // Transparency Log entries do not carry a spirit_id column
            // (v0.3-β schema limitation). When a spirit_filter is
            // requested, TL entries are included regardless — they
            // represent system-wide IAC frames visible to the director.
            let _ = spirit_filter;
            entries.push(ComposedLogEntry {
                timestamp_ns: e.timestamp_ns,
                spirit_id: None,
                source: LogSource::TransparencyLog,
                payload: ComposedPayload::Frame {
                    frame_kind: e.kind,
                    intent: e.intent,
                    payload_redacted: vec![],
                },
            });
        }
    }

    // 2. Approval Decision Log rows
    {
        use rusqlite::OpenFlags;
        let conn = rusqlite::Connection::open_with_flags(
            audit_db,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(AuditError::Open)?;

        let sql = "SELECT timestamp_ns, actor, target, capability, intent, decision, reasoning
                   FROM approval_decision_log
                   WHERE timestamp_ns >= ?1 AND timestamp_ns < ?2
                   ORDER BY timestamp_ns ASC";
        let mut stmt = conn.prepare(sql).map_err(AuditError::Read)?;
        let rows = stmt
            .query_map(
                rusqlite::params![range.since_ns as i64, range.until_ns as i64],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? as u64,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, bool>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .map_err(AuditError::Read)?;

        for row in rows {
            let (ts_ns, actor, target, capability, intent, decision, reasoning) =
                row.map_err(AuditError::Read)?;
            if let Some(sf) = spirit_filter {
                if target != sf {
                    continue;
                }
            }
            entries.push(ComposedLogEntry {
                timestamp_ns: ts_ns,
                spirit_id: Some(target),
                source: LogSource::ApprovalDecisionLog,
                payload: ComposedPayload::Approval {
                    actor,
                    capability,
                    intent,
                    decision,
                    reasoning,
                },
            });
        }
    }

    // 3. Lifecycle Journal (NDJSON)
    if journal_path.exists() {
        let file = fs::File::open(journal_path).map_err(|e| AuditError::Io(e))?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.map_err(|e| AuditError::Io(e))?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                let ts = match entry["timestamp"].as_u64() {
                    Some(t) => t,
                    None => continue,
                };
                if ts < range.since_ns || ts >= range.until_ns {
                    continue;
                }
                let spirit = entry["spirit_id"].as_str().map(|s| s.to_string());
                if let Some(sf) = spirit_filter {
                    if spirit.as_deref() != Some(sf) {
                        continue;
                    }
                }
                let event = entry["lifecycle_event"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string();
                let tier = entry["effective_sandbox_tier"]
                    .get("T0")
                    .or(entry["effective_sandbox_tier"].get("T1"))
                    .or(entry["effective_sandbox_tier"].get("T2"))
                    .or(entry["effective_sandbox_tier"].get("T3"));
                let tier_num = if let Some(t) = tier {
                    t.as_u64().map(|n| n as u8)
                } else {
                    entry["effective_sandbox_tier"].as_u64().map(|n| n as u8)
                };

                entries.push(ComposedLogEntry {
                    timestamp_ns: ts,
                    spirit_id: spirit,
                    source: LogSource::LifecycleJournal,
                    payload: ComposedPayload::Lifecycle {
                        event,
                        sandbox_tier: tier_num,
                    },
                });
            }
        }
    }

    // Sort by timestamp_ns ascending, ties broken by source: Lifecycle < Approval < Transparency
    entries.sort_by(|a, b| {
        a.timestamp_ns
            .cmp(&b.timestamp_ns)
            .then_with(|| source_rank(a.source).cmp(&source_rank(b.source)))
    });

    Ok(entries)
}

fn source_rank(s: LogSource) -> u8 {
    match s {
        LogSource::LifecycleJournal => 0,
        LogSource::ApprovalDecisionLog => 1,
        LogSource::TransparencyLog => 2,
    }
}

/// Count rows across the three log surfaces for the given range and
/// spirit filter. Materializes all rows via `ranged_recall` — adequate
/// for v0.3-β corpus sizes. Story 4.4 may optimize with SQL COUNT(*).
pub fn ranged_count(
    audit_db: &Path,
    journal_path: &Path,
    range: LogRange,
    spirit_filter: Option<&str>,
) -> Result<usize, AuditError> {
    let entries = ranged_recall(audit_db, journal_path, range, spirit_filter)?;
    Ok(entries.len())
}

// ─────────────────────────────────────────────────────────────────────────
// FR43 — Posture-delta classifier
// ─────────────────────────────────────────────────────────────────────────

/// Classification of a posture change event.
///
/// v0.5 limitation: consent dimension is rupture-only; allowlist config
/// changes are not tracked until a future release.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum PostureChange {
    CapabilityIssued {
        intent: String,
        capability_detail: Option<String>,
    },
    CapabilityRevoked {
        intent: String,
        capability_detail: Option<String>,
    },
    SandboxTierChange {
        from_tier: Option<u8>,
        to_tier: Option<u8>,
        event: String,
    },
    ConsentRupture {
        intent: String,
    },
}

/// Attribution of an approval decision to a posture change event.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ApprovalAttribution {
    pub actor: String,
    pub decision: bool,
    pub reasoning: Option<String>,
}

/// A single posture-delta event with optional approval attribution.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PostureDeltaEntry {
    pub timestamp_ns: u64,
    pub spirit_id: Option<String>,
    pub change: PostureChange,
    pub approval: Option<ApprovalAttribution>,
}

/// Summary counts for the posture-delta window.
///
/// Net-posture reporting: `capabilities_issued` and `capabilities_revoked` are
/// raw counts; `net_capability_delta` is the signed sum (issued - revoked), so
/// a grant-then-revoke within the window reports net 0 rather than 2.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PostureSummary {
    pub window_since_ns: u64,
    pub window_until_ns: u64,
    pub total_events: usize,
    pub capabilities_issued: usize,
    pub capabilities_revoked: usize,
    pub net_capability_delta: i64,
    pub sandbox_tier_changes: usize,
    pub consent_ruptures: usize,
    /// v0.5 limitation: only rupture events are surfaced; allowlist config
    /// changes are not tracked until a future release.
    pub consent_dimension_limitation: String,
}

/// The full posture-delta report: summary header + event list.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PostureDeltaReport {
    pub summary: PostureSummary,
    pub events: Vec<PostureDeltaEntry>,
}

/// Classify composed log entries into posture-change events within the
/// given time range. Joins with Approval entries on (capability, target)
/// for authoritative approval-chain attribution (AC3.2).
///
/// Classification rules:
/// - `Frame` with `frame_kind` `capability.invocation` → `CapabilityIssued`
/// - `Frame` with `frame_kind` `spirit.revoked` → `CapabilityRevoked`
/// - `Lifecycle` with a per-Spirit `sandbox_tier` change → `SandboxTierChange`
/// - `Frame` with `frame_kind` `consent.rupture` → `ConsentRupture`
///
/// v0.5 consent-dimension limitation: rupture-only, no allowlist config changes.
pub fn posture_delta(
    audit_db: &Path,
    journal_path: &Path,
    range: LogRange,
    spirit_filter: Option<&str>,
) -> Result<PostureDeltaReport, AuditError> {
    let entries = ranged_recall(audit_db, journal_path, range, spirit_filter)?;

    // Collect approval events keyed by capability for authoritative join.
    // AC3.2: join on capability (Approval) ↔ intent (Frame) rather than
    // timestamp proximity. TL Frame entries do not carry spirit_id, so the
    // join key is capability alone.
    let approvals_by_capability: HashMap<String, ApprovalAttribution> = entries
        .iter()
        .filter_map(|e| match &e.payload {
            ComposedPayload::Approval {
                actor,
                capability,
                intent: _,
                decision,
                reasoning,
            } => Some((
                capability.clone(),
                ApprovalAttribution {
                    actor: actor.clone(),
                    decision: *decision,
                    reasoning: reasoning.clone(),
                },
            )),
            _ => None,
        })
        .collect();

    let mut events: Vec<PostureDeltaEntry> = Vec::new();
    // Per-Spirit tier tracking: only emit a tier change when the SAME Spirit
    // changes tier, not when two different Spirits have different tiers.
    let mut prev_tier_by_spirit: HashMap<String, u8> = HashMap::new();
    let mut caps_issued: usize = 0;
    let mut caps_revoked: usize = 0;
    let mut tier_changes: usize = 0;
    let mut ruptures: usize = 0;

    for entry in &entries {
        match &entry.payload {
            ComposedPayload::Frame {
                frame_kind,
                intent,
                payload_redacted: _,
            } => {
                if frame_kind == "capability.invocation" {
                    let approval = approvals_by_capability.get(intent).cloned();
                    events.push(PostureDeltaEntry {
                        timestamp_ns: entry.timestamp_ns,
                        spirit_id: entry.spirit_id.clone(),
                        change: PostureChange::CapabilityIssued {
                            intent: intent.clone(),
                            capability_detail: None,
                        },
                        approval,
                    });
                    caps_issued += 1;
                } else if frame_kind == "spirit.revoked" {
                    let approval = approvals_by_capability.get(intent).cloned();
                    events.push(PostureDeltaEntry {
                        timestamp_ns: entry.timestamp_ns,
                        spirit_id: entry.spirit_id.clone(),
                        change: PostureChange::CapabilityRevoked {
                            intent: intent.clone(),
                            capability_detail: None,
                        },
                        approval,
                    });
                    caps_revoked += 1;
                } else if frame_kind == "consent.rupture" {
                    let approval = approvals_by_capability.get(intent).cloned();
                    events.push(PostureDeltaEntry {
                        timestamp_ns: entry.timestamp_ns,
                        spirit_id: entry.spirit_id.clone(),
                        change: PostureChange::ConsentRupture {
                            intent: intent.clone(),
                        },
                        approval,
                    });
                    ruptures += 1;
                }
            }
            ComposedPayload::Lifecycle {
                event: _,
                sandbox_tier,
            } => {
                if let Some(current_tier) = sandbox_tier {
                    let spirit_key = entry
                        .spirit_id
                        .clone()
                        .unwrap_or_else(|| format!("__pidless_{}", entry.timestamp_ns));
                    let prev = prev_tier_by_spirit.get(&spirit_key).copied();
                    if prev != Some(*current_tier) {
                        // Tier changes don't have a capability match key;
                        // no approval attribution unless the intent matches.
                        let approval = entry
                            .spirit_id
                            .as_ref()
                            .and_then(|_| approvals_by_capability.get("").cloned());
                        events.push(PostureDeltaEntry {
                            timestamp_ns: entry.timestamp_ns,
                            spirit_id: entry.spirit_id.clone(),
                            change: PostureChange::SandboxTierChange {
                                from_tier: prev,
                                to_tier: Some(*current_tier),
                                event: "sandbox_tier_change".to_string(),
                            },
                            approval,
                        });
                        tier_changes += 1;
                        prev_tier_by_spirit.insert(spirit_key, *current_tier);
                    }
                }
            }
            ComposedPayload::Approval { .. } => {
                // Approval events are used for attribution, not classified as posture changes
            }
        }
    }

    let total = events.len();
    let net_cap_delta = caps_issued as i64 - caps_revoked as i64;
    Ok(PostureDeltaReport {
        summary: PostureSummary {
            window_since_ns: range.since_ns,
            window_until_ns: range.until_ns,
            total_events: total,
            capabilities_issued: caps_issued,
            capabilities_revoked: caps_revoked,
            net_capability_delta: net_cap_delta,
            sandbox_tier_changes: tier_changes,
            consent_ruptures: ruptures,
            consent_dimension_limitation: "v0.5: only consent-rupture events are surfaced; \
                 allowlist config changes are not tracked until a future release."
                .to_string(),
        },
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_range_last_24h() {
        let now = 1_700_000_000_000_000_000u64;
        let range = LogRange::last_24h(now);
        let day_ns = 24 * 60 * 60 * 1_000_000_000u64;
        assert_eq!(range.until_ns, now);
        assert_eq!(range.since_ns, now - day_ns);
    }

    #[test]
    fn empty_corpus_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("empty.sqlite");
        let journal_path = tmp.path().join("empty.ndjson");

        // Create empty SQLite
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        drop(conn);

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let result = ranged_recall(&db_path, &journal_path, range, None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn merge_sorts_by_timestamp_then_source() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[1u8; 16] as &[u8], 100i64, 0i64, 1i64, 0i64, "delegate", b"x" as &[u8], 0i64,
            ],
        ).unwrap();
        conn.execute(
            "INSERT INTO approval_decision_log (timestamp_ns, actor, target, capability, intent)
             VALUES (100, 'director', 'hello-spirit', 'test.cap', 'test')",
            [],
        )
        .unwrap();
        drop(conn);

        let journal = r#"{"timestamp":100,"lifecycle_event":"Pause","spirit_id":"hello-spirit"}"#;
        std::fs::write(&journal_path, journal).unwrap();

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let result = ranged_recall(&db_path, &journal_path, range, None).unwrap();

        assert_eq!(result.len(), 3, "expected 3 entries");
        assert_eq!(result[0].source, LogSource::LifecycleJournal);
        assert_eq!(result[1].source, LogSource::ApprovalDecisionLog);
        assert_eq!(result[2].source, LogSource::TransparencyLog);
    }

    #[test]
    fn half_open_range_excludes_until_ns() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO approval_decision_log (timestamp_ns, actor, target, capability, intent)
             VALUES (100, 'director', 'spirit', 'cap', 'test')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO approval_decision_log (timestamp_ns, actor, target, capability, intent)
             VALUES (200, 'director', 'spirit', 'cap', 'test')",
            [],
        )
        .unwrap();
        drop(conn);

        let journal =
            "{\"timestamp\":200,\"lifecycle_event\":\"Pause\",\"spirit_id\":\"hello-spirit\"}\n";
        std::fs::write(&journal_path, journal).unwrap();

        let range = LogRange {
            since_ns: 50,
            until_ns: 200,
        };
        let result = ranged_recall(&db_path, &journal_path, range, None).unwrap();

        assert_eq!(
            result.len(),
            1,
            "half-open [50, 200) should exclude ts=200, got: {:?}",
            result
        );
        assert_eq!(result[0].source, LogSource::ApprovalDecisionLog);
        assert_eq!(result[0].timestamp_ns, 100);
    }

    #[test]
    fn spirit_filter_includes_transparency_log_regardless() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[1u8; 16] as &[u8], 100i64, 0i64, 1i64, 0i64, "delegate", b"x" as &[u8], 0i64,
            ],
        ).unwrap();
        conn.execute(
            "INSERT INTO approval_decision_log (timestamp_ns, actor, target, capability, intent)
             VALUES (100, 'director', 'other-spirit', 'cap', 'test')",
            [],
        )
        .unwrap();
        drop(conn);

        let journal =
            "{\"timestamp\":100,\"lifecycle_event\":\"Pause\",\"spirit_id\":\"other-spirit\"}\n";
        std::fs::write(&journal_path, journal).unwrap();

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let result = ranged_recall(&db_path, &journal_path, range, Some("hello-spirit")).unwrap();

        assert!(
            result
                .iter()
                .any(|e| e.source == LogSource::TransparencyLog),
            "spirit_filter should not exclude TransparencyLog entries"
        );
        assert!(
            !result
                .iter()
                .any(|e| matches!(e.source, LogSource::ApprovalDecisionLog)),
            "spirit_filter should exclude non-matching ADL entries"
        );
        assert!(
            !result
                .iter()
                .any(|e| matches!(e.source, LogSource::LifecycleJournal)),
            "spirit_filter should exclude non-matching journal entries"
        );
    }

    #[test]
    fn posture_delta_classifies_capability_changes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        // CapabilityInvocation (kind=7) — canonical capability-grant frame.
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[1u8; 16] as &[u8], 100i64, 0i64, 1i64, 7i64, "delegate", b"x" as &[u8], 0i64,
            ],
        )
        .unwrap();
        // SpiritRevoked (kind=17) — revocation is also a capability change.
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[2u8; 16] as &[u8], 200i64, 0i64, 1i64, 17i64, "revoke", b"x" as &[u8], 0i64,
            ],
        )
        .unwrap();
        drop(conn);

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let report = posture_delta(&db_path, &journal_path, range, None).unwrap();

        let issued: Vec<&PostureDeltaEntry> = report
            .events
            .iter()
            .filter(|e| matches!(e.change, PostureChange::CapabilityIssued { .. }))
            .collect();
        let revoked: Vec<&PostureDeltaEntry> = report
            .events
            .iter()
            .filter(|e| matches!(e.change, PostureChange::CapabilityRevoked { .. }))
            .collect();
        assert_eq!(
            issued.len(),
            1,
            "expected 1 CapabilityIssued event, got: {:?}",
            report.events
        );
        assert_eq!(
            revoked.len(),
            1,
            "expected 1 CapabilityRevoked event, got: {:?}",
            report.events
        );
        assert!(
            issued[0].change
                == PostureChange::CapabilityIssued {
                    intent: "delegate".to_string(),
                    capability_detail: None,
                },
            "CapabilityIssued should carry the invocation intent"
        );
        assert!(
            revoked[0].change
                == PostureChange::CapabilityRevoked {
                    intent: "revoke".to_string(),
                    capability_detail: None,
                },
            "CapabilityRevoked should carry the revocation intent"
        );
    }

    #[test]
    fn posture_delta_classifies_sandbox_tier_changes() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        drop(conn);

        // Two lifecycle entries with different effective_sandbox_tier (T1 → T2).
        let journal = "{\"timestamp\":100,\"lifecycle_event\":\"Admit\",\"spirit_id\":\"hello-spirit\",\"effective_sandbox_tier\":1}\n\
                       {\"timestamp\":200,\"lifecycle_event\":\"Elevate\",\"spirit_id\":\"hello-spirit\",\"effective_sandbox_tier\":2}\n";
        std::fs::write(&journal_path, journal).unwrap();

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let report = posture_delta(&db_path, &journal_path, range, None).unwrap();

        let tier_changes: Vec<&PostureDeltaEntry> = report
            .events
            .iter()
            .filter(|e| matches!(e.change, PostureChange::SandboxTierChange { .. }))
            .collect();
        assert!(
            !tier_changes.is_empty(),
            "expected at least one SandboxTierChange, got: {:?}",
            report.events
        );

        // The T1→T2 transition must surface as a tier change from 1 to 2.
        let has_t1_to_t2 = tier_changes.iter().any(|e| {
            matches!(
                e.change,
                PostureChange::SandboxTierChange {
                    from_tier: Some(1),
                    to_tier: Some(2),
                    ..
                }
            )
        });
        assert!(
            has_t1_to_t2,
            "expected a T1→T2 SandboxTierChange, got: {:?}",
            tier_changes
        );
    }

    #[test]
    fn posture_delta_classifies_consent_ruptures() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        // ConsentRupture (kind=22) — ADR-034 partial-consent rupture event.
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[3u8; 16] as &[u8], 100i64, 0i64, 1i64, 22i64, "consent.broken", b"x" as &[u8],
                0i64,
            ],
        )
        .unwrap();
        drop(conn);

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let report = posture_delta(&db_path, &journal_path, range, None).unwrap();

        let ruptures: Vec<&PostureDeltaEntry> = report
            .events
            .iter()
            .filter(|e| matches!(e.change, PostureChange::ConsentRupture { .. }))
            .collect();
        assert_eq!(
            ruptures.len(),
            1,
            "expected exactly 1 ConsentRupture event, got: {:?}",
            report.events
        );
    }

    #[test]
    fn posture_delta_joins_approval_attribution() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        // Capability change frame at ts=100.
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[4u8; 16] as &[u8], 100i64, 0i64, 1i64, 7i64, "delegate", b"x" as &[u8], 0i64,
            ],
        )
        .unwrap();
        // Approval decision — join is on capability (Approval) matching
        // intent (Frame). The approval's capability must equal the TL frame's
        // intent value for the authoritative join to succeed.
        conn.execute(
            "INSERT INTO approval_decision_log (timestamp_ns, actor, target, capability, intent, decision, reasoning)
             VALUES (100, 'director', 'hello-spirit', 'delegate', 'delegate', 1, 'approved by operator')",
            [],
        )
        .unwrap();
        drop(conn);

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let report = posture_delta(&db_path, &journal_path, range, None).unwrap();

        let cap = report
            .events
            .iter()
            .find(|e| matches!(e.change, PostureChange::CapabilityIssued { .. }))
            .expect("expected a CapabilityIssued event");
        let approval = cap
            .approval
            .as_ref()
            .expect("expected approval attribution on the capability change");
        assert_eq!(approval.actor, "director");
        assert!(
            approval.decision,
            "decision should be true/approved, got false"
        );
        assert_eq!(approval.reasoning.as_deref(), Some("approved by operator"));
    }

    #[test]
    fn posture_delta_summary_counts() {
        let tmp = tempfile::TempDir::new().unwrap();
        let db_path = tmp.path().join("test.sqlite");
        let journal_path = tmp.path().join("test.ndjson");

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS approval_decision_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_ns INTEGER NOT NULL DEFAULT 0,
                actor TEXT NOT NULL,
                target TEXT NOT NULL,
                capability TEXT NOT NULL,
                intent TEXT NOT NULL,
                decision INTEGER NOT NULL DEFAULT 1,
                reasoning TEXT DEFAULT ''
            );",
        )
        .unwrap();
        // 2 capability changes (CapabilityInvocation + SpiritRevoked).
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[10u8; 16] as &[u8], 100i64, 0i64, 1i64, 7i64, "delegate", b"x" as &[u8], 0i64,
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[11u8; 16] as &[u8], 200i64, 0i64, 1i64, 17i64, "revoke", b"x" as &[u8], 0i64,
            ],
        )
        .unwrap();
        // 1 consent rupture (ConsentRupture).
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &[12u8; 16] as &[u8], 300i64, 0i64, 1i64, 22i64, "consent.broken", b"x" as &[u8],
                0i64,
            ],
        )
        .unwrap();
        drop(conn);

        // 1 sandbox tier change (single journal entry: None → T1).
        let journal = "{\"timestamp\":150,\"lifecycle_event\":\"Admit\",\"spirit_id\":\"hello-spirit\",\"effective_sandbox_tier\":1}\n";
        std::fs::write(&journal_path, journal).unwrap();

        let range = LogRange {
            since_ns: 0,
            until_ns: i64::MAX as u64,
        };
        let report = posture_delta(&db_path, &journal_path, range, None).unwrap();

        assert_eq!(report.summary.capabilities_issued, 1, "capabilities_issued");
        assert_eq!(
            report.summary.capabilities_revoked, 1,
            "capabilities_revoked"
        );
        assert_eq!(
            report.summary.net_capability_delta, 0,
            "net_capability_delta should be 0 (issued - revoked)"
        );
        assert_eq!(
            report.summary.sandbox_tier_changes, 1,
            "sandbox_tier_changes"
        );
        assert_eq!(report.summary.consent_ruptures, 1, "consent_ruptures");
        assert_eq!(report.summary.total_events, 4, "total_events");
    }
}
