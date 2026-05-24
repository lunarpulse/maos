#![forbid(unsafe_code)]

//! Upgrade orchestrator — three-policy upgrade verb body.
//!
//! Dispatches `hot-swap` (Story 5.2), `cold-swap` (sequenced unload+load),
//! and `migrator` (cross-major with explicit declaration check).

use std::io;
use std::path::Path;
use std::sync::Arc;

use maos_domain::halt::TerminationKind;
use maos_domain::invariants::i10::{JournalEntry, LifecycleEntry, LifecycleEvent};

use crate::capability::cap_tokens::monotonic_now_ns;
use crate::halt::terminate_spirit;
use crate::hot_swap::HotSwapCoordinator;
use crate::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use crate::journal::JournalAdapter;
use crate::scheduler::control_block::SpiritManifestBundle;
use crate::scheduler::SpiritSchedulerAdapter;
use crate::security::manifest::ClassSection;
use crate::telemetry::iac_rt::{IacRtMetrics, Outcome, Service};
use maos_domain::invariants::i3::FrameOrigin;

#[maos_attrs::i9_exempt(reason = "upgrade orchestrator composite; holds exempt adapter Arcs")]
pub struct UpgradeOrchestrator {
    scheduler: Arc<SpiritSchedulerAdapter>,
    hot_swap: Arc<HotSwapCoordinator>,
    tl: Arc<TransparencyLogAdapter>,
    journal: Arc<JournalAdapter>,
    telemetry: Arc<IacRtMetrics>,
}

impl UpgradeOrchestrator {
    pub fn new(
        scheduler: Arc<SpiritSchedulerAdapter>,
        hot_swap: Arc<HotSwapCoordinator>,
        tl: Arc<TransparencyLogAdapter>,
        journal: Arc<JournalAdapter>,
        telemetry: Arc<IacRtMetrics>,
    ) -> Self {
        Self {
            scheduler,
            hot_swap,
            tl,
            journal,
            telemetry,
        }
    }

    pub async fn upgrade(
        &self,
        spirit_id: &str,
        successor_manifest_path: &Path,
        policy: UpgradePolicy,
    ) -> Result<UpgradeReport, UpgradeError> {
        let start_ns = monotonic_now_ns();

        // 1. Parse successor manifest
        let successor_manifest = load_bundle_from_file(successor_manifest_path).map_err(|e| {
            let reason = e.to_string();
            if reason.contains("manifest not found")
                || reason.contains("No such file")
                || reason.contains("NotFound")
            {
                UpgradeError::ManifestNotFound {
                    path: successor_manifest_path.display().to_string(),
                }
            } else {
                UpgradeError::ManifestParse {
                    path: successor_manifest_path.display().to_string(),
                    reason,
                }
            }
        })?;

        // 2. Resolve predecessor PID
        let predecessor_pid =
            self.scheduler
                .resolve_pid(spirit_id)
                .ok_or_else(|| UpgradeError::NotLoaded {
                    spirit_id: spirit_id.into(),
                })?;

        // 3. Capture predecessor version + spirit_obj under a single read lock
        let (predecessor_version, successor_spirit_obj) = {
            let scbs = self.scheduler.scbs();
            let map = scbs.read().expect("spirits lock poisoned");
            let scb = map
                .get(&predecessor_pid)
                .ok_or_else(|| UpgradeError::NotLoaded {
                    spirit_id: spirit_id.into(),
                })?;
            let version = scb
                .manifest
                .class
                .as_ref()
                .map(|c| c.version.clone())
                .unwrap_or_else(|| "unknown".into());
            let obj = Arc::clone(&scb.spirit_obj);
            (version, obj)
        };

        let successor_version = successor_manifest
            .class
            .as_ref()
            .map(|c| c.version.clone())
            .unwrap_or_else(|| "unknown".into());

        let mut halt_receipts_produced = 0usize;
        let mut outcome = UpgradeOutcome::Completed;

        // 4. Dispatch per policy
        match policy {
            UpgradePolicy::HotSwap => {
                match self
                    .hot_swap
                    .initiate_swap(spirit_id, &successor_manifest, Arc::clone(&successor_spirit_obj))
                    .await
                {
                    Ok(_result) => {
                        outcome = UpgradeOutcome::Completed;
                    }
                    Err(e) => {
                        outcome = UpgradeOutcome::Failed;
                        return Err(UpgradeError::HotSwap(e));
                    }
                }
            }
            UpgradePolicy::ColdSwap => {
                let unload_start_ns = monotonic_now_ns();
                self.scheduler
                    .unload(predecessor_pid)
                    .await
                    .map_err(|e| UpgradeError::Lifecycle(e))?;

                // Count receipts in the unload window
                let entries = self
                    .tl
                    .query_frames(FrameFilter {
                        kind: Some(FrameKind::EpistemicHalt),
                        spirit_pid: Some(predecessor_pid),
                        since_ns: Some(unload_start_ns),
                        ..Default::default()
                    })
                    .unwrap_or_default();
                halt_receipts_produced = entries.len();

                // v0.3-β: manual SCB insertion. scheduler.load() requires T: Spirit
                // (concrete type), but we only have Arc<dyn AnySpiritObj> from the
                // predecessor's SCB. The scheduler's load path (security admission,
                // on_load hooks, Load journaling) is bypassed — the successor is
                // admitted via the predecessor's prior admission. Production
                // cold-swap admission arrives at Story 5.5x with subprocess wire
                // protocol.
                let boot_nonce = 0u64;
                let new_pid = crate::scheduler::scheduler_loop::allocate_pid();
                let new_scb =
                    Arc::new(crate::scheduler::control_block::SpiritControlBlock::new(
                        new_pid,
                        spirit_id.into(),
                        successor_manifest,
                        Arc::clone(&successor_spirit_obj),
                        boot_nonce,
                    ));
                {
                    let scbs = self.scheduler.scbs();
                    let mut map = scbs.write().expect("spirits lock poisoned");
                    map.insert(new_pid, new_scb);
                }
            }
            UpgradePolicy::Migrator => {
                if successor_manifest.migrates_from.is_none() {
                    return Err(UpgradeError::MigratorNotDeclared);
                }
                match self
                    .hot_swap
                    .initiate_swap(spirit_id, &successor_manifest, Arc::clone(&successor_spirit_obj))
                    .await
                {
                    Ok(_result) => {
                        outcome = UpgradeOutcome::Completed;
                    }
                    Err(e) => {
                        outcome = UpgradeOutcome::Failed;
                        return Err(UpgradeError::HotSwap(e));
                    }
                }
            }
        }

        let latency_ns = monotonic_now_ns().saturating_sub(start_ns);

        // 5. Journal LifecycleEvent::Upgrade
        let now_ns = monotonic_now_ns();
        self.journal
            .append_transition(JournalEntry::Lifecycle(LifecycleEntry {
                timestamp: now_ns / 1_000_000_000,
                lifecycle_event: LifecycleEvent::Upgrade,
                spirit_id: spirit_id.into(),
                payload: None,
                effective_sandbox_tier: None,
            }));

        // 6. Emit TL row
        let payload = serde_json::json!({
            "spirit_id": spirit_id,
            "predecessor_version": predecessor_version,
            "successor_version": successor_version,
            "policy": policy.as_str(),
            "outcome": outcome.as_str(),
            "latency_ns": latency_ns,
            "halt_receipts_produced": halt_receipts_produced,
        });
        let payload_bytes = serde_json::to_vec(&payload).unwrap_or_else(|e| {
            eprintln!("upgrade orchestrator: payload serialize failed: {e}");
            vec![]
        });
        self.tl.insert_frame_event(
            FrameKind::CapabilityInvocation,
            predecessor_pid,
            None,
            "spirit.upgrade",
            &payload_bytes,
            FrameOrigin::Kernel,
        );

        // 7. Telemetry
        let telemetry_outcome = match outcome {
            UpgradeOutcome::Completed => Outcome::Ok,
            UpgradeOutcome::Reverted => Outcome::Ok, // compensated but not an error
            UpgradeOutcome::Failed => Outcome::Err,
        };
        self.telemetry
            .record_iac_rt(Service::UpgradeOrchestrator, telemetry_outcome, latency_ns / 1000);

        Ok(UpgradeReport {
            spirit_id: spirit_id.into(),
            predecessor_version,
            successor_version,
            policy,
            outcome,
            latency_ns,
            halt_receipts_produced,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradePolicy {
    HotSwap,
    ColdSwap,
    Migrator,
}

impl UpgradePolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HotSwap => "hot-swap",
            Self::ColdSwap => "cold-swap",
            Self::Migrator => "migrator",
        }
    }
}

impl std::str::FromStr for UpgradePolicy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hot-swap" => Ok(Self::HotSwap),
            "cold-swap" => Ok(Self::ColdSwap),
            "migrator" => Ok(Self::Migrator),
            other => Err(format!("unknown upgrade policy: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UpgradeReport {
    pub spirit_id: String,
    pub predecessor_version: String,
    pub successor_version: String,
    pub policy: UpgradePolicy,
    pub outcome: UpgradeOutcome,
    pub latency_ns: u64,
    pub halt_receipts_produced: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeOutcome {
    Completed,
    Reverted,
    Failed,
}

impl UpgradeOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Reverted => "reverted",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum UpgradeError {
    #[error("spirit '{spirit_id}' not loaded")]
    NotLoaded { spirit_id: String },
    #[error("manifest at '{path}' not found")]
    ManifestNotFound { path: String },
    #[error("manifest at '{path}' parse failed: {reason}")]
    ManifestParse { path: String, reason: String },
    #[error("--policy migrator requested but successor manifest does not declare [migrates_from]")]
    MigratorNotDeclared,
    #[error("hot-swap coordinator error: {0}")]
    HotSwap(#[from] maos_domain::hot_swap::HotSwapError),
    #[error("lifecycle error during cold-swap: {0}")]
    Lifecycle(#[from] maos_domain::lifecycle::LifecycleError),
}

/// Load a `SpiritManifestBundle` from a TOML file path.
/// v0.3-β minimal implementation — reads file, extracts known sections.
fn load_bundle_from_file(
    path: &Path,
) -> Result<SpiritManifestBundle, crate::security::manifest::ManifestError> {
    use crate::security::manifest::*;

    let toml_str = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            ManifestError::Toml(format!(
                "manifest not found at {}: {}",
                path.display(),
                e
            ))
        } else {
            ManifestError::Toml(format!("read {}: {}", path.display(), e))
        }
    })?;
    let root: toml::Value =
        toml::from_str(&toml_str).map_err(|e| ManifestError::Toml(format!("parse: {e}")))?;

    let extract = |section: &str| -> Result<String, ManifestError> {
        let value = root
            .get(section)
            .ok_or_else(|| ManifestError::Toml(format!("missing section [{section}]")))?;
        toml::to_string(value)
            .map_err(|e| ManifestError::Toml(format!("serialize [{section}]: {e}")))
    };

    let scheduling = SchedulingSection::from_toml_str(&extract("scheduling")?)?;
    let lifecycle = LifecycleSection::from_toml_str(&extract("lifecycle")?)?;
    let class = root
        .get("class")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| ClassSection::from_toml_str(&s))
        .transpose()?;
    let hot_swap = root
        .get("hot_swap")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| HotSwapManifestSection::from_toml_str(&s))
        .transpose()?;
    let migrates_from = root
        .get("migrates_from")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| MigratesFromSection::from_toml_str(&s))
        .transpose()?;
    let halt_protocol_compatibility = root
        .get("halt_protocol_compatibility")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| HaltProtocolCompatibilitySection::from_toml_str(&s))
        .transpose()?;
    let on_crash = root
        .get("on_crash")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| OnCrashSection::from_toml_str(&s))
        .transpose()?;
    let on_revocation = root
        .get("on_revocation")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| OnRevocationSection::from_toml_str(&s))
        .transpose()?;
    let supervision = root
        .get("supervision")
        .and_then(|v| toml::to_string(v).ok())
        .map(|s| SupervisionSection::from_toml_str(&s))
        .transpose()?;

    Ok(SpiritManifestBundle {
        scheduling,
        lifecycle,
        class,
        hot_swap,
        migrates_from,
        halt_protocol_compatibility,
        on_crash,
        on_revocation,
        supervision,
    })
}
