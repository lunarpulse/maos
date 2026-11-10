//! Finite snapshot plus matching-tail sweeps. Only the final writer transaction
//! certifies completion; a busy/ambiguous tail never becomes an erasure claim.
use super::*;
use std::collections::{HashMap, HashSet};

type Admissions = HashMap<(u64, u32), Vec<(i64, String)>>;
#[derive(Debug, Clone, Copy)]
pub struct SweepCompletion {
    pub frame_id: [u8; 16],
    pub through_rowid: u64,
}
pub struct ErasureSweep {
    subjects: Vec<String>,
    spirit: Option<String>,
    through: i64,
    admissions: Admissions,
    sources: HashSet<String>,
    pub distillates: Vec<(i64, String)>,
    pub principal_frames: Vec<(i64, String)>,
    pub requests: Vec<RedistillRequest>,
    pub held: Vec<(i64, String)>,
    pub malformed: Vec<(i64, String)>,
    pub held_principals: Vec<String>,
    pub unattributed: Vec<String>,
    pub completion: Option<SweepCompletion>,
}
impl ErasureSweep {
    pub fn new(subjects: Vec<String>, spirit: Option<String>) -> Self {
        Self {
            subjects,
            spirit,
            through: 0,
            admissions: HashMap::new(),
            sources: HashSet::new(),
            distillates: Vec::new(),
            principal_frames: Vec::new(),
            requests: Vec::new(),
            held: Vec::new(),
            malformed: Vec::new(),
            held_principals: Vec::new(),
            unattributed: Vec::new(),
            completion: None,
        }
    }
    pub fn include_sources(&mut self, sources: impl Iterator<Item = String>) {
        self.sources.extend(sources);
    }
    pub fn blocked(&self) -> bool {
        !self.held.is_empty()
            || !self.held_principals.is_empty()
            || !self.malformed.is_empty()
            || !self.unattributed.is_empty()
    }
}
impl TransparencyLogAdapter {
    fn load_admissions(
        &self,
        range: (i64, i64),
        owners: &mut Admissions,
    ) -> Result<(), AuditError> {
        self.scan_rowid_range(
            "SELECT rowid, boot_nonce, spirit_pid, CASE WHEN json_valid(CAST(payload_redacted AS TEXT)) \
             AND json_type(CAST(payload_redacted AS TEXT),'$.spirit_id') = 'text' \
             THEN json_extract(CAST(payload_redacted AS TEXT),'$.spirit_id') ELSE intent END \
             FROM transparency_log WHERE rowid>?2 AND rowid<=?3 AND ?1 IS ?1 \
             AND (kind=19 OR (kind=7 AND intent='lifecycle.load' AND capability_token IS NULL))", None, range,
            |row| { owners.entry((row.get::<_, i64>(1).map_err(AuditError::SqliteRead)? as u64,
                row.get::<_, i64>(2).map_err(AuditError::SqliteRead)? as u32)).or_default()
                .push((row.get(0).map_err(AuditError::SqliteRead)?, row.get(3).map_err(AuditError::SqliteRead)?)); Ok(()) })
    }
    fn owner_at(owners: &Admissions, boot: u64, pid: u32, rowid: i64) -> Option<&str> {
        owners
            .get(&(boot, pid))?
            .iter()
            .rev()
            .find(|(at, _)| *at <= rowid)
            .map(|(_, name)| name.as_str())
    }
    fn owned_lineage(
        &self,
        spirit: &str,
        owners: &Admissions,
        range: (i64, i64),
        gaps: &mut Vec<String>,
    ) -> Result<Vec<ForgetFrame>, AuditError> {
        let mut frames = Vec::new();
        self.scan_rowid_range(
            "SELECT frame_id,boot_nonce,spirit_pid,payload_redacted,timestamp_ns,rowid FROM transparency_log INDEXED BY idx_tlog_distillate_rowid \
             WHERE rowid>?2 AND rowid<=?3 AND ?1 IS ?1 AND kind=11", None, range, |row| {
                let boot = row.get::<_,i64>(1).map_err(AuditError::SqliteRead)? as u64;
                let pid = row.get::<_,i64>(2).map_err(AuditError::SqliteRead)? as u32;
                let body: Vec<u8> = row.get(3).map_err(AuditError::SqliteRead)?;
                if Self::is_kernel_tombstone(&body) { return Ok(()); }
                let bytes: Vec<u8> = row.get(0).map_err(AuditError::SqliteRead)?;
                let frame_id: [u8;16] = bytes.try_into().map_err(|v: Vec<u8>| AuditError::MalformedFrameId(v.len()))?;
                match Self::owner_at(owners,boot,pid,row.get(5).map_err(AuditError::SqliteRead)?) {
                    Some(owner) if owner==spirit => frames.push(
                        ForgetFrame { timestamp_ns:row.get(4).map_err(AuditError::SqliteRead)?,frame_id,kind:11,spirit_pid:pid,boot_nonce:boot,source_log_ref:Self::distillate_source_refs(&body).unwrap_or_default() }),
                    None => gaps.push(format!("{:032x}",u128::from_be_bytes(frame_id))), _ => {}
                } Ok(())
            })?;
        frames.sort_by_key(|frame| (frame.timestamp_ns, frame.frame_id));
        Ok(frames)
    }
    /// Inventory uses admission identity at the frame's row, not a recycled pid.
    pub fn distillates_written_by_identity(
        &self,
        spirit: &str,
    ) -> Result<(Vec<ForgetFrame>, Vec<String>), AuditError> {
        let range = (0, self.frame_high_water()?);
        let mut owners = Admissions::new();
        let mut gaps = Vec::new();
        self.load_admissions(range, &mut owners)?;
        Ok((self.owned_lineage(spirit, &owners, range, &mut gaps)?, gaps))
    }
    pub fn current_pid_owned_by(&self, spirit: &str, pid: u32) -> Result<bool, AuditError> {
        let high = self.frame_high_water()?;
        let mut owners = Admissions::new();
        self.load_admissions((0, high), &mut owners)?;
        Ok(Self::owner_at(&owners, self.boot_nonce(), pid, high) == Some(spirit))
    }
    /// One finite range; callers dispatch its real requests before certification.
    pub fn sweep_erasure(&self, state: &mut ErasureSweep) -> Result<(), AuditError> {
        let range = (state.through, self.frame_high_water()?);
        state.sources.extend(self.prior_tombstones(range)?);
        let mut frames = Vec::new();
        let mut seen = HashSet::new();
        let mut owned = HashSet::new();
        for subject in &state.subjects {
            for frame in self.forget_candidates(
                Some(subject),
                Some(&HashSet::new()),
                false,
                range,
                &state.sources,
            )? {
                if seen.insert(frame.frame_id) {
                    frames.push(frame);
                }
            }
        }
        if let Some(spirit) = &state.spirit {
            self.load_admissions(range, &mut state.admissions)?;
            for frame in
                self.owned_lineage(spirit, &state.admissions, range, &mut state.unattributed)?
            {
                owned.insert(frame.frame_id);
                if seen.insert(frame.frame_id) {
                    frames.push(frame);
                }
            }
        }
        let mut seeds: HashSet<_> = frames
            .iter()
            .map(|frame| format!("{:032x}", u128::from_be_bytes(frame.frame_id)))
            .collect();
        for frame in self.closure_with_prior(&seeds, true, range, &state.sources)? {
            seeds.insert(format!("{:032x}", u128::from_be_bytes(frame.frame_id)));
            if seen.insert(frame.frame_id) {
                frames.push(frame);
            }
        }
        let mut holds = self.list_legal_holds()?;
        state.held_principals.extend(
            holds
                .iter()
                .filter(|hold| state.subjects.contains(&hold.principal_id))
                .map(|hold| hold.principal_id.clone()),
        );
        frames.sort_by_key(|frame| (frame.timestamp_ns, frame.frame_id));
        let mut protected = self.protected_by_holds(&holds)?;
        for frame in frames {
            let id = format!("{:032x}", u128::from_be_bytes(frame.frame_id));
            let request =
                (frame.kind == 11 && !owned.contains(&frame.frame_id)).then(|| RedistillRequest {
                    redacted_distillate_frame_id: id.clone(),
                    writer_spirit_pid: frame.spirit_pid,
                    writer_boot_nonce: (frame.boot_nonce as u128).to_le_bytes(),
                    surviving_source_log_ref: frame
                        .source_log_ref
                        .iter()
                        .filter(|id| !seeds.contains(*id) && !state.sources.contains(*id))
                        .cloned()
                        .collect(),
                });
            let mut written = false;
            for _ in 0..8 {
                match self.write_erasure_body(
                    ErasureTarget {
                        frame_id: &frame.frame_id,
                        kind: frame.kind,
                        id: &id,
                        principals: &state.subjects,
                        spirit: state
                            .spirit
                            .as_deref()
                            .filter(|_| owned.contains(&frame.frame_id)),
                        reason: if owned.contains(&frame.frame_id) {
                            "spirit-uninstall"
                        } else {
                            "gdpr-forget"
                        },
                        request: request.as_ref(),
                    },
                    &holds,
                    &protected,
                )? {
                    ErasureWrite::Retry => {
                        holds = self.list_legal_holds()?;
                        protected = self.protected_by_holds(&holds)?;
                    }
                    ErasureWrite::Held => {
                        state.held.push((frame.timestamp_ns, id.clone()));
                        written = true;
                        break;
                    }
                    ErasureWrite::Redacted => {
                        if frame.kind == 11 {
                            state.distillates.push((frame.timestamp_ns, id.clone()));
                        } else {
                            state
                                .principal_frames
                                .push((frame.timestamp_ns, id.clone()));
                        }
                        state.sources.insert(id.clone());
                        if let Some(request) = request {
                            state.requests.push(request);
                        }
                        written = true;
                        break;
                    }
                }
            }
            if !written {
                return Err(AuditError::ErasurePending);
            }
        }
        let mut malformed = Vec::new();
        self.scan_rowid_range(
            "SELECT lower(hex(frame_id)),payload_redacted,timestamp_ns FROM transparency_log INDEXED BY idx_tlog_distillate_rowid \
             WHERE rowid>?2 AND rowid<=?3 AND ?1 IS ?1 AND kind=11", None,range,|row| {
                let body: Vec<u8>=row.get(1).map_err(AuditError::SqliteRead)?;
                if Self::distillate_source_refs(&body).is_err() { malformed.push((row.get::<_,i64>(2).map_err(AuditError::SqliteRead)?,row.get::<_,String>(0).map_err(AuditError::SqliteRead)?)); } Ok(())
            })?;
        malformed.sort_unstable();
        state.malformed.extend(malformed);
        state.through = range.1;
        Ok(())
    }
    /// Return false only for a NEW matching tail; a tail larger than one bounded
    /// page returns pending instead of following a sustained writer forever.
    pub fn certify_erasure(&self, state: &mut ErasureSweep) -> Result<bool, AuditError> {
        self.certify_frame_scope(state, true)
    }

    pub(super) fn certify_frame_scope(
        &self,
        state: &mut ErasureSweep,
        publish: bool,
    ) -> Result<bool, AuditError> {
        if state.blocked() {
            return Ok(false);
        }
        let mut inner = self.inner.lock();
        let authority = inner
            .legal_hold_authority
            .ok_or(AuditError::LegalHoldAuthorityUnbound)?;
        let TransparencyLogInner {
            conn,
            next_frame_id_counter,
            last_frame_id,
            boot_nonce,
            ..
        } = &mut *inner;
        let transaction = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(AuditError::SqliteWriteFatal)?;
        let high: i64 = transaction
            .query_row(
                "SELECT coalesce(max(rowid),0) FROM transparency_log",
                [],
                |row| row.get(0),
            )
            .map_err(AuditError::SqliteRead)?;
        let mut stmt=transaction.prepare("SELECT rowid,kind,intent,payload_redacted,boot_nonce,spirit_pid,capability_token FROM transparency_log WHERE rowid>?1 ORDER BY rowid LIMIT 4097").map_err(AuditError::SqliteRead)?;
        let mut rows = stmt
            .query([state.through])
            .map_err(AuditError::SqliteRead)?;
        let mut count = 0;
        while let Some(row) = rows.next().map_err(AuditError::SqliteRead)? {
            count += 1;
            if count > 4096 {
                return Err(AuditError::ErasurePending);
            }
            let kind: i64 = row.get(1).map_err(AuditError::SqliteRead)?;
            let intent: String = row.get(2).map_err(AuditError::SqliteRead)?;
            if intent.starts_with("principal.uninstall.")
                || matches!(
                    intent.as_str(),
                    "principal.forget" | "principal.forget.held" | "distillate.redacted"
                )
            {
                continue;
            }
            let body: Vec<u8> = row.get(3).map_err(AuditError::SqliteRead)?;
            // Any new distillate needs provenance and malformed checks, including
            // admission changes. Non-lineage frames use token-bounded matching.
            let at: i64 = row.get(0).map_err(AuditError::SqliteRead)?;
            let boot = row.get::<_, i64>(4).map_err(AuditError::SqliteRead)? as u64;
            let pid = row.get::<_, i64>(5).map_err(AuditError::SqliteRead)? as u32;
            if kind == 19
                || (kind == 7
                    && intent == "lifecycle.load"
                    && matches!(
                        row.get_ref(6).map_err(AuditError::SqliteRead)?,
                        rusqlite::types::ValueRef::Null
                    ))
            {
                let name = serde_json::from_slice::<serde_json::Value>(&body)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("spirit_id")
                            .and_then(|id| id.as_str())
                            .map(str::to_owned)
                    })
                    .unwrap_or(intent);
                state
                    .admissions
                    .entry((boot, pid))
                    .or_default()
                    .push((at, name));
            }
            let lineage = kind == 11
                && !Self::is_kernel_tombstone(&body)
                && (Self::distillate_source_refs(&body).map_or(true, |refs| {
                    refs.iter().any(|id| state.sources.contains(id))
                }) || state.spirit.as_ref().is_some_and(|spirit| {
                    Self::owner_at(&state.admissions, boot, pid, at)
                        .is_none_or(|owner| owner == spirit)
                }));
            if lineage
                || state
                    .subjects
                    .iter()
                    .any(|subject| Self::subject_in_frame(kind, &body, subject))
            {
                return Ok(false);
            }
        }
        drop(rows);
        drop(stmt);
        let held: Vec<_> = Self::read_legal_holds(&transaction, authority)?
            .into_iter()
            .filter(|hold| state.subjects.contains(&hold.principal_id))
            .collect();
        if !held.is_empty() {
            state
                .held_principals
                .extend(held.into_iter().map(|hold| hold.principal_id));
            return Ok(false);
        }
        if !publish {
            transaction.commit().map_err(AuditError::SqliteWriteFatal)?;
            state.through = high;
            return Ok(true);
        }
        let payload=serde_json::to_vec(&serde_json::json!({"through_rowid":high,"completion_boundary":"durable_transactional_marker"})).map_err(|e|AuditError::Serialization(e.to_string()))?;
        let id = Self::append_erasure_row(
            &transaction,
            next_frame_id_counter,
            *boot_nonce,
            FrameKind::Decision,
            "principal.uninstall.frame_sweep_complete",
            &payload,
        )?;
        transaction.commit().map_err(AuditError::SqliteWriteFatal)?;
        *last_frame_id = id;
        state.through = high;
        state.completion = Some(SweepCompletion {
            frame_id: id,
            through_rowid: high as u64,
        });
        Ok(true)
    }
}
