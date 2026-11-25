//! Story 22-8 (D-22-8-05 A) — the halts a cast member's admitted identity raised and nobody
//! resolved, read across sealed segments and the live log. The host re-admits them at boot
//! (`maos-bin::halt_carry`); this is the read side only.

use maos_domain::frame::EpistemicHaltPayload;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

/// `intent` of the row the host journals when it carries a halt into a new incarnation.
pub const CARRIED_INTENT: &str = "halt.carried";

/// `correlation_id` of a halt's carry rows.
pub fn carry_correlation(halt_id: &str) -> String {
    format!("maos.halt-carry/{halt_id}")
}

/// Rows examined before a scan stops and says so (the bound `orphaned_halts` keeps).
const SCAN_ROWS: usize = 50_000;

/// One unresolved halt, and the incarnation that holds it now.
#[derive(Debug, Clone, PartialEq)]
pub struct UnresolvedHalt {
    pub payload: EpistemicHaltPayload,
    /// The admitted identity (`spirit_id`) of the holder.
    pub spirit_id: String,
    /// The holder's incarnation: the raise, or the newest carry row's.
    pub boot: u64,
    pub pid: u32,
}

/// Every halt raised by one of `spirits` with no `halt.resolve` decision, other than those the
/// boot `except_boot` (the running one) holds, plus every reason the answer may be incomplete.
/// A raise is a kernel-writer kind-3 row (no sender, no addressee) that carries no
/// `terminal_state`: receipts do, and a frame a Spirit routed is not an invocation.
pub fn unresolved_halts(
    db_path: &Path,
    spirits: &[&str],
    except_boot: u64,
) -> Result<(Vec<UnresolvedHalt>, Vec<String>), String> {
    let (mut gaps, mut raised) = (Vec::new(), BTreeMap::new());
    if spirits.is_empty() || !db_path.exists() {
        return Ok((Vec::new(), gaps));
    }
    let names = crate::spirit_names(db_path)?;
    let sources = crate::segment::open_log_sources(db_path)
        .map_err(|e| format!("halt scan: cannot open the log: {e}"))?;
    let failed = |e: rusqlite::Error| format!("halt scan failed: {e}");
    let mut examined = 0_usize;
    'sources: for conn in &sources {
        let mut statement = conn
            .prepare(
                "SELECT boot_nonce, spirit_pid, from_spirit_id = '' AND to_spirit_id = '', \
                 CAST(payload_redacted AS TEXT) FROM transparency_log \
                 WHERE kind = 3 AND length(payload_redacted) > 0 ORDER BY rowid",
            )
            .map_err(failed)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, i64>(1)? as u32,
                    row.get::<_, bool>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(failed)?;
        for row in rows {
            examined += 1;
            if examined > SCAN_ROWS {
                gaps.push(format!(
                    "halt scan bounded at {SCAN_ROWS} rows; later rows not examined"
                ));
                break 'sources;
            }
            let (boot, pid, kernel_row, text) = row.map_err(failed)?;
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
                continue;
            };
            if !kernel_row || value.get("terminal_state").is_some() || value["halt_id"].is_null() {
                continue;
            }
            match serde_json::from_value::<EpistemicHaltPayload>(value) {
                Ok(payload) if !payload.halt_id.is_empty() => {
                    raised
                        .entry(payload.halt_id.clone())
                        .or_insert((boot, pid, payload));
                }
                _ => gaps.push(format!(
                    "a malformed halt row at pid {pid} was not examined"
                )),
            }
        }
    }
    let mut resolved = HashSet::new();
    if !raised.is_empty() {
        if let Some(conn) = crate::open_tenant_artifact_readonly(db_path)
            .map_err(|e| format!("halt scan: approval history unreadable: {e}"))?
        {
            let mut statement = conn
                .prepare(
                    "SELECT reasoning FROM approval_decision_log WHERE capability = 'halt.resolve' \
                     AND target = ?1 ORDER BY decision_id DESC LIMIT ?2",
                )
                .map_err(failed)?;
            for spirit in spirits {
                let history = statement
                    .query_map(rusqlite::params![spirit, SCAN_ROWS as i64 + 1], |row| {
                        row.get::<_, Option<String>>(0)
                    })
                    .map_err(failed)?;
                for (read, text) in history.enumerate() {
                    // Not provably unresolved: carry none rather than a halt an operator closed.
                    if read >= SCAN_ROWS {
                        gaps.push(format!(
                            "approval history of '{spirit}' bounded at {SCAN_ROWS} rows; no halt carried"
                        ));
                        return Ok((Vec::new(), gaps));
                    }
                    let text = text.map_err(failed)?.unwrap_or_default();
                    if let Some((id, _)) = text
                        .strip_prefix("halt=")
                        .and_then(|rest| rest.split_once(':'))
                    {
                        resolved.insert(id.to_owned());
                    }
                }
            }
        }
    }
    let mut unresolved = Vec::new();
    for (halt_id, (boot, pid, payload)) in raised {
        if boot == except_boot || resolved.contains(&halt_id) {
            continue;
        }
        // The newest incarnation that held the halt: the latest carry row, else the raise.
        let (boot, pid) = sources
            .iter()
            .filter_map(|conn| {
                conn.query_row(
                    "SELECT boot_nonce, spirit_pid FROM transparency_log \
                     WHERE correlation_id = ?1 AND intent = ?2 ORDER BY rowid DESC LIMIT 1",
                    rusqlite::params![carry_correlation(&halt_id), CARRIED_INTENT],
                    |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, i64>(1)? as u32)),
                )
                .ok()
            })
            .next_back()
            .unwrap_or((boot, pid));
        let holder = names
            .get(&(boot, pid))
            .and_then(|events| events.last())
            .map(|(_, name)| name.as_str());
        if let Some(holder) = holder.filter(|name| spirits.contains(name)) {
            unresolved.push(UnresolvedHalt {
                payload,
                spirit_id: holder.to_owned(),
                boot,
                pid,
            });
        }
    }
    Ok((unresolved, gaps))
}
