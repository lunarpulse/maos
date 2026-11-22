#![forbid(unsafe_code)]

//! Shared validated CRL state.
//!
//! The async admission gate serializes CRL installation with scheduler map
//! insertion: a rule is visible before a concurrent load can commit, and a
//! load admitted under the gate is visible to the ensuing CRL scan.

use std::collections::BTreeMap;
use std::sync::RwLock;

use tokio::sync::{Mutex, MutexGuard};

use crate::scheduler::SpiritManifestBundle;
use maos_domain::revocation::{semver_range_contains, CrlId, RevocationEntry};
#[maos_attrs::i9_exempt(
    reason = "revocation admission synchronization; pure validated CRL state shared by applier and scheduler"
)]
pub(crate) struct ValidatedRevocationRules {
    admission_gate: Mutex<()>,
    rules: RwLock<BTreeMap<CrlId, Vec<RevocationEntry>>>,
}
impl ValidatedRevocationRules {
    pub(crate) fn new() -> Self {
        Self {
            admission_gate: Mutex::new(()),
            rules: RwLock::new(BTreeMap::new()),
        }
    }

    /// Hold this guard while either installing rules and taking an SCB snapshot,
    /// or checking admission and inserting the new SCB.
    pub(crate) async fn admission_guard(&self) -> MutexGuard<'_, ()> {
        self.admission_gate.lock().await
    }

    pub(crate) fn contains_locked(&self, id: CrlId) -> bool {
        self.rules
            .read()
            .expect("validated revocation rules lock poisoned")
            .contains_key(&id)
    }

    pub(crate) fn install_locked(&self, id: CrlId, entries: Vec<RevocationEntry>) {
        self.rules
            .write()
            .expect("validated revocation rules lock poisoned")
            .insert(id, entries);
    }

    pub(crate) fn remove_locked(&self, id: CrlId) {
        self.rules
            .write()
            .expect("validated revocation rules lock poisoned")
            .remove(&id);
    }

    pub(crate) async fn forget(&self, id: CrlId) {
        let _gate = self.admission_guard().await;
        self.rules
            .write()
            .expect("validated revocation rules lock poisoned")
            .remove(&id);
    }

    pub(crate) fn list(&self) -> Vec<CrlId> {
        self.rules
            .read()
            .expect("validated revocation rules lock poisoned")
            .keys()
            .copied()
            .collect()
    }

    /// The applied CRLs with an entry matching class `name` at `version`.
    pub(crate) fn matching(&self, name: &str, version: &str) -> Vec<CrlId> {
        let rules = self
            .rules
            .read()
            .expect("validated revocation rules lock poisoned");
        rules
            .iter()
            .filter(|(_, entries)| {
                entries.iter().any(|entry| {
                    entry.spirit_class == name
                        && matches!(
                            semver_range_contains(version, &entry.version_range),
                            Ok(true)
                        )
                })
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Must be called while the async `admission_guard` is held.
    pub(crate) fn rejects_manifest_locked(&self, manifest: &SpiritManifestBundle) -> bool {
        let class = manifest.class.as_ref();
        class.is_some_and(|class| !self.matching(&class.name, &class.version).is_empty())
    }
}
