//! FR13 durable quarantine holds (later-registry-quarantine-closure, D-LRQC-01/02/03/08).
//!
//! A hold is the identity `(spirit_id, version)` of a quarantined Spirit. It is persisted (atomic
//! temp + fsync + rename + directory fsync) BEFORE any fence takes effect, and re-applied by the
//! admission triple on every admission of that identity, so it survives unload, reload, crash and
//! daemon restart. Only the door's `quarantine-release` removes one. A CRL match is not a hold: it
//! stays durable by refusal at load. This fences effects; it is not process or container isolation.
use maos_domain::invariants::i3::FrameOrigin;
use maos_kernel_core::capability::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::iac::IacBusAdapter;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// One quarantined identity; `origin` and `cause` say what decided it, `publisher` (hex) is the
/// signing publisher where the decision knew one.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hold {
    pub spirit_id: String,
    pub version: String,
    pub origin: String,
    pub cause: String,
    pub publisher: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldFile {
    schema_version: u32,
    holds: Vec<Hold>,
}

/// The durable hold store plus the fences it applies (capability and sender; the Worker bearer
/// reads the sender fence). With no `path` (no home resolved) holds live in memory only.
pub struct QuarantineGate {
    path: Option<PathBuf>,
    holds: Mutex<Vec<Hold>>,
    capability: std::sync::Weak<CapabilityRegistryAdapter>,
    iac: std::sync::Weak<IacBusAdapter>,
    tl: std::sync::Weak<TransparencyLogAdapter>,
}

impl QuarantineGate {
    /// `MAOS_QUARANTINE_STORE_PATH`, else `<MAOS home>/quarantine/holds.json` (the one home rule:
    /// `MAOS_HOME`, else `$HOME/.maos`). `None` when no home resolves.
    pub fn default_path() -> io::Result<Option<PathBuf>> {
        if let Some(path) = std::env::var_os("MAOS_QUARANTINE_STORE_PATH") {
            if path.is_empty() {
                return Err(io::Error::other("empty quarantine store path"));
            }
            return Ok(Some(path.into()));
        }
        // An unresolvable home (absent, or refused as relative) leaves holds in memory only.
        let home = maos_domain::operator_door::maos_home().ok().flatten();
        Ok(home.map(|home| home.join("quarantine/holds.json")))
    }

    /// Load the store (absent is empty; unreadable or malformed is an error, so a damaged hold
    /// file stops the boot instead of silently dropping holds).
    pub fn open(
        path: Option<PathBuf>,
        capability: Arc<CapabilityRegistryAdapter>,
        iac: Arc<IacBusAdapter>,
        tl: Arc<TransparencyLogAdapter>,
    ) -> io::Result<Self> {
        let holds = match path.as_ref().map(std::fs::read) {
            None => Vec::new(),
            Some(Ok(bytes)) => match serde_json::from_slice::<HoldFile>(&bytes) {
                Ok(file) if file.schema_version == 1 => file.holds,
                Ok(file) => {
                    let found = file.schema_version;
                    return Err(io::Error::other(format!("unsupported hold schema {found}")));
                }
                Err(error) => return Err(io::Error::new(io::ErrorKind::InvalidData, error)),
            },
            Some(Err(error)) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Some(Err(error)) => return Err(error),
        };
        Ok(Self {
            path,
            holds: Mutex::new(holds),
            capability: Arc::downgrade(&capability),
            iac: Arc::downgrade(&iac),
            tl: Arc::downgrade(&tl),
        })
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Hold>> {
        self.holds.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn persist(&self, holds: &[Hold]) -> io::Result<()> {
        use std::io::Write;
        let Some(path) = &self.path else {
            return Ok(());
        };
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("hold store has no parent"))?;
        std::fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(&HoldFile {
            schema_version: 1,
            holds: holds.to_vec(),
        })
        .map_err(io::Error::other)?;
        let temp = path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        std::fs::File::open(parent)?.sync_all()
    }

    /// Every active hold of `spirit_id` (any version).
    pub fn holds_for(&self, spirit_id: &str) -> Vec<Hold> {
        let holds = self.lock();
        let mut held: Vec<Hold> = holds.clone();
        held.retain(|h| h.spirit_id == spirit_id);
        held
    }

    /// Fence (`true`) or release (`false`) one pid: its capability and its sender fence together.
    pub fn fence(&self, pid: u32, fenced: bool) {
        if let Some(capability) = self.capability.upgrade() {
            capability.set_quarantined(pid, fenced);
        }
        if let Some(iac) = self.iac.upgrade() {
            iac.set_sender_fenced(pid, fenced);
        }
    }

    fn receipt(&self, pid: u32, intent: &str, hold: &Hold, detail: &str) {
        // The log handle is weak: the gate must not keep the audit writer alive past the drain.
        let Some(tl) = self.tl.upgrade() else {
            return;
        };
        let payload = serde_json::json!({
            "spirit_id": hold.spirit_id, "spirit_pid": pid, "version": hold.version,
            "origin": hold.origin, "cause": hold.cause, "detail": detail,
        });
        let _logged = tl.insert_frame_event(
            FrameKind::TelemetryEvent,
            pid,
            None,
            intent,
            payload.to_string().as_bytes(),
            FrameOrigin::Kernel,
        );
    }

    /// Quarantine `hold`: persist it first, then fence `pid` when a Spirit of that identity is
    /// loaded. A failed durable write fails closed: the fence still applies, the failure is
    /// journaled, and the error is returned so the caller retries.
    pub fn quarantine(&self, pid: Option<u32>, hold: Hold) -> io::Result<()> {
        let persisted = {
            let mut holds = self.lock();
            let known = holds
                .iter()
                .any(|h| h.spirit_id == hold.spirit_id && h.version == hold.version);
            if known {
                Ok(())
            } else {
                let mut next = holds.clone();
                next.push(hold.clone());
                self.persist(&next).map(|()| *holds = next)
            }
        };
        if let Some(pid) = pid {
            self.fence(pid, true);
        }
        if let Err(error) = &persisted {
            self.receipt(
                pid.unwrap_or(0),
                "quarantine:hold-persist-failed",
                &hold,
                &error.to_string(),
            );
        }
        persisted
    }

    /// Re-apply an active hold on the admission of `spirit_id`@`version` at `pid`, before the SCB
    /// can run. Journals a `quarantine:reapplied` receipt; `true` when a hold applied.
    pub fn reapply(&self, pid: u32, spirit_id: &str, version: &str) -> bool {
        let hold = {
            let holds = self.lock();
            holds
                .iter()
                .find(|h| h.spirit_id == spirit_id && h.version == version)
                .cloned()
        };
        let Some(hold) = hold else {
            return false;
        };
        self.fence(pid, true);
        self.receipt(pid, "quarantine:reapplied", &hold, "");
        true
    }

    /// Release: clear `pid`'s fences, then drop every hold of `spirit_id`. A failed durable
    /// removal restores the fences and leaves the holds intact. Returns the holds removed.
    pub fn release(&self, spirit_id: &str, pid: Option<u32>) -> io::Result<usize> {
        if let Some(pid) = pid {
            self.fence(pid, false);
        }
        let removed = {
            let mut holds = self.lock();
            let mut next = holds.clone();
            next.retain(|h| h.spirit_id != spirit_id);
            let removed = holds.len() - next.len();
            self.persist(&next).map(|()| {
                *holds = next;
                removed
            })
        };
        if let (Err(_), Some(pid)) = (&removed, pid) {
            self.fence(pid, true);
        }
        removed
    }
}
