//! Story 22-6 (ADR-072) — the rotation driver shared by the Host daemon (its 60 s policy task and
//! the `audit rotate` door verb) and `maosctl audit rotate` when no daemon answers.
//!
//! The Host never writes an unsigned segment: [`DaemonSealer`] signs with the operator audit
//! seed (region-derived when the Host has a region). With no key, rotation is refused with a
//! typed line and the live log keeps growing; logging never fails because of sealing.

use maos_audit::segment::{seal_segment, SealParams};
use maos_domain::region::Region;
use maos_iac::adapter::transparency_log::{
    RotationError, RotationOutcome, SealedSegment, SegmentSealRequest, SegmentSealer,
    TransparencyLogAdapter,
};

pub const ENV_ROTATE_BYTES: &str = "MAOS_AUDIT_ROTATE_BYTES";
pub const ENV_ROTATE_DAYS: &str = "MAOS_AUDIT_ROTATE_DAYS";
pub const ENV_KEEP_DAYS: &str = "MAOS_AUDIT_KEEP_DAYS";
/// D-22-6-08 A defaults: 256 MiB, 30 days; expiry is off unless `audit_keep_days` is set.
pub const DEFAULT_ROTATE_BYTES: u64 = 256 * 1024 * 1024;
pub const DEFAULT_ROTATE_DAYS: u64 = 30;
/// NFR-Aud-4 / architecture §7.3: nothing younger than 90 days is ever expired.
pub const KEEP_DAYS_FLOOR: u64 = maos_iac::adapter::transparency_log::KEEP_FLOOR_DAYS;

/// The rotation policy: three `[host]` keys, each registered as one environment name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RotationPolicy {
    pub rotate_bytes: u64,
    pub rotate_days: u64,
    /// `None` = no expiry: MAOS deletes no audit record.
    pub keep_days: Option<u64>,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            rotate_bytes: DEFAULT_ROTATE_BYTES,
            rotate_days: DEFAULT_ROTATE_DAYS,
            keep_days: None,
        }
    }
}

impl RotationPolicy {
    /// Parse one setting; the same rule serves the Host file and the environment.
    pub fn check(name: &str, value: &str) -> Result<u64, String> {
        let parsed: u64 = value
            .trim()
            .parse()
            .map_err(|_| format!("{name} = {value:?}: must be a positive whole number"))?;
        if parsed == 0 {
            return Err(format!("{name} = {value:?}: must be greater than zero"));
        }
        if name == ENV_KEEP_DAYS && parsed < KEEP_DAYS_FLOOR {
            return Err(format!(
                "{name} = {value:?}: expiry never deletes a segment younger than {KEEP_DAYS_FLOOR} days"
            ));
        }
        Ok(parsed)
    }

    pub fn from_lookup(get: &dyn Fn(&str) -> Option<String>) -> Result<Self, String> {
        let mut policy = Self::default();
        if let Some(value) = get(ENV_ROTATE_BYTES).filter(|v| !v.is_empty()) {
            policy.rotate_bytes = Self::check(ENV_ROTATE_BYTES, &value)?;
        }
        if let Some(value) = get(ENV_ROTATE_DAYS).filter(|v| !v.is_empty()) {
            policy.rotate_days = Self::check(ENV_ROTATE_DAYS, &value)?;
        }
        if let Some(value) = get(ENV_KEEP_DAYS).filter(|v| !v.is_empty()) {
            policy.keep_days = Some(Self::check(ENV_KEEP_DAYS, &value)?);
        }
        Ok(policy)
    }

    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(&|name| std::env::var(name).ok())
    }
}

/// Signs a finished segment with the operator audit seed.
#[derive(Debug)]
pub struct DaemonSealer {
    seed: [u8; 32],
    region: Option<Region>,
    host: Option<String>,
}

impl DaemonSealer {
    pub fn new(seed: [u8; 32], region: Option<Region>, host: Option<String>) -> Self {
        Self { seed, region, host }
    }

    /// The sealer the Host would use: the operator audit key (`--audit-key` is not a daemon
    /// input; `MAOS_AUDIT_KEY`, then the default location) and the Host's home region.
    pub fn from_host() -> Result<Self, String> {
        let seed = maos_domain::audit_key::load_audit_key_seed(&None)
            .map_err(|error| format!("operator audit key unavailable: {error}"))?;
        let region = crate::subcommands::resolve_region_home()
            .map_err(|error| format!("invalid region config: {error}"))?;
        Ok(Self::new(seed, region, None))
    }
}

impl SegmentSealer for DaemonSealer {
    fn seal(&self, request: &SegmentSealRequest) -> Result<SealedSegment, String> {
        let params = SealParams {
            seq: request.seq,
            file_name: request.file_name.clone(),
            after_rowid: request.after_rowid,
            first_rowid: request.first_rowid,
            last_rowid: request.last_rowid,
            row_count: request.row_count,
            min_timestamp_ns: request.min_timestamp_ns,
            max_timestamp_ns: request.max_timestamp_ns,
            prev_manifest_sha256: request.prev_manifest_sha256.clone(),
            replaces_manifest: request.replaces_manifest.clone(),
            scrubbed_frame_ids: request.scrubbed_frame_ids.clone(),
            region: self.region.clone(),
            host: self.host.clone(),
        };
        seal_segment(&request.segment_path, &params, &self.seed)
            .map(|(manifest, manifest_sha256)| SealedSegment {
                manifest,
                manifest_sha256,
            })
            .map_err(|error| error.to_string())
    }
}

/// Install the Host's sealer on `tl`. `Err` carries the typed refusal line; the log keeps growing.
pub fn install_sealer(tl: &TransparencyLogAdapter) -> Result<(), String> {
    let sealer = DaemonSealer::from_host()?;
    tl.install_segment_sealer(std::sync::Arc::new(sealer))
        .map_err(|error| error.to_string())
}

/// What one rotation pass did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RotationReport {
    pub outcome: Option<RotationOutcome>,
    pub expired: Vec<u64>,
}

impl RotationReport {
    pub fn to_json(&self) -> serde_json::Value {
        let (word, detail) = match &self.outcome {
            Some(RotationOutcome::Sealed(segment)) => (
                "sealed",
                serde_json::json!({
                    "seq": segment.seq,
                    "rows": segment.row_count,
                    "first_rowid": segment.first_rowid,
                    "last_rowid": segment.last_rowid,
                    "manifest_sha256": segment.manifest_sha256,
                }),
            ),
            Some(RotationOutcome::NothingToSeal { cut }) => {
                ("nothing_to_seal", serde_json::json!({ "cut": cut }))
            }
            Some(RotationOutcome::Deferred) => ("deferred", serde_json::Value::Null),
            None => ("not_due", serde_json::Value::Null),
        };
        serde_json::json!({
            "outcome": word,
            "segment": detail,
            "expired_segments": self.expired,
            "terminal_code": 0,
        })
    }
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() as u64)
}

/// One rotation pass. `force` seals everything the floors allow (the manual verb); otherwise
/// the policy decides. `floor` is the minimum committed cursor of every in-process tail
/// (`i64::MAX` when none). Expiry runs only when the policy sets `keep_days` and `expire` is set.
pub fn rotate_once(
    tl: &TransparencyLogAdapter,
    policy: &RotationPolicy,
    floor: i64,
    force: bool,
    expire: bool,
) -> Result<RotationReport, RotationError> {
    let now = now_ns();
    let cut = if force {
        Some(i64::MAX)
    } else {
        tl.rotation_due(Some(policy.rotate_bytes), Some(policy.rotate_days), now)?
    };
    let outcome = cut.map(|cut| tl.rotate_through(cut, floor)).transpose()?;
    let expired = match (expire, policy.keep_days) {
        (true, Some(days)) => tl.expire_segments(days, now)?,
        _ => Vec::new(),
    };
    Ok(RotationReport { outcome, expired })
}
