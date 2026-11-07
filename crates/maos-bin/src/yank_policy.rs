//! Kernel-free yank decisions and running-promotion observation. Sticky quarantine
//! effects are deliberately owned by the final KQ1 landing, not a pause shim.
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::registry::{RegistryError, SignedPackage, YankEntry, YankOrigin};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::{control_block::ScbLifecycleState, SpiritSchedulerAdapter};
use maos_registry::yank::{YankObserver, YankPolicy, YankPolicyOutcome};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

pub struct YankRuntime {
    tl: Arc<TransparencyLogAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
    policy: YankPolicy,
    observed: Mutex<BTreeSet<(u32, String, String)>>,
}
impl YankRuntime {
    pub fn new(
        tl: Arc<TransparencyLogAdapter>,
        scheduler: Arc<SpiritSchedulerAdapter>,
        applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
        policy: YankPolicy,
    ) -> Self {
        Self {
            tl,
            scheduler,
            applier,
            policy,
            observed: Mutex::new(BTreeSet::new()),
        }
    }
    fn record(&self, label: &str, value: &impl serde::Serialize) -> Result<(), RegistryError> {
        let bytes =
            serde_json::to_vec(value).map_err(|e| RegistryError::Transport(e.to_string()))?;
        let _logged = self.tl.insert_frame_event(
            FrameKind::SpiritRevoked,
            0,
            None,
            label,
            &bytes,
            FrameOrigin::Kernel,
        );
        Ok(())
    }
    fn policy_request(
        &self,
        name: &str,
        version: &str,
        origin: YankOrigin,
        cause: &str,
    ) -> Result<YankPolicyOutcome, RegistryError> {
        let outcome = self.policy.outcome();
        // This durable record precedes the effect. L1 never substitutes a resumable pause.
        self.record(
            "registry-yank-policy",
            &serde_json::json!({
                "spirit_id": name, "version": version, "origin": origin,
                "cause": cause, "outcome": outcome,
                "quarantine_effect": "final-kq1-landing-not-claimed-in-l1",
            }),
        )?;
        if outcome == YankPolicyOutcome::AutoRevokeRequested {
            let crl = local_yank_crl(name, version, cause).map_err(RegistryError::Transport)?;
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(self.applier.apply_crl(crl))
            });
            match result {
                Ok(_) | Err(maos_domain::revocation::RevocationError::AlreadyApplied { .. }) => {}
                Err(error) => return Err(RegistryError::Transport(error.to_string())),
            }
        }
        Ok(outcome)
    }
    fn installed(name: &str) -> Result<Option<(SignedPackage, std::path::PathBuf)>, RegistryError> {
        for (id, path) in
            maos_registry::storage::LocalFsRegistryStorage::default_installed_manifests()
                .map_err(|e| RegistryError::Transport(e.to_string()))?
        {
            if id != name {
                continue;
            }
            let directory = path
                .parent()
                .ok_or_else(|| RegistryError::Transport("install has no parent".into()))?;
            let bytes = std::fs::read(directory.join("signed_package.json"))
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            let pkg: SignedPackage = serde_json::from_slice(&bytes)
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            if pkg.spirit_id.as_str() != name
                || !maos_registry::admission::verify_publisher_sig(&pkg)
            {
                return Err(RegistryError::SignatureInvalid);
            }
            return Ok(Some((pkg, directory.to_path_buf())));
        }
        Ok(None)
    }
    pub fn reevaluate_at(&self, now: u64) -> Result<(), RegistryError> {
        let spirits: Vec<_> = self
            .scheduler
            .scbs()
            .read()
            .map_err(|e| RegistryError::Transport(e.to_string()))?
            .values()
            .cloned()
            .collect();
        for spirit in spirits {
            if spirit.current_state() != ScbLifecycleState::Running {
                continue;
            }
            let Some((pkg, directory)) = Self::installed(&spirit.spirit_id)? else {
                continue;
            };
            let runtime = spirit.runtime_snapshot();
            if runtime
                .manifest
                .class
                .as_ref()
                .is_none_or(|class| class.version != pkg.version)
            {
                continue;
            }
            let bytes = match std::fs::read(directory.join("vetting.json")) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(RegistryError::Transport(e.to_string())),
            };
            let promotion: maos_registry::install::InstalledVetting =
                serde_json::from_slice(&bytes)
                    .map_err(|e| RegistryError::Transport(e.to_string()))?;
            let seed = maos_domain::audit_key::load_audit_key_seed(&None)
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            let root = maos_domain::audit_key::derive_audit_public_key(&seed);
            use maos_compliance::vetting::VettingRejection;
            let inputs = match promotion.verify(&pkg, &root, now) {
                Ok(_) => continue,
                Err(VettingRejection::Expired { .. }) => maos_compliance::TerminalInputs {
                    attestation_expired: true,
                    ..Default::default()
                },
                Err(
                    VettingRejection::VetterKeyRevoked { .. }
                    | VettingRejection::AttestationRevoked,
                ) => maos_compliance::TerminalInputs {
                    vetting_revoked: true,
                    ..Default::default()
                },
                Err(error) => return Err(RegistryError::Transport(error.to_string())),
            };
            let cause = if inputs.attestation_expired {
                "expiry-lapse"
            } else {
                "vetting-revocation"
            };
            let key = (spirit.pid, pkg.version.clone(), cause.to_owned());
            let mut observed = self
                .observed
                .lock()
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            if observed.contains(&key) {
                continue;
            }
            maos_compliance::observe_running_spirit(
                pkg.spirit_id.as_str(),
                &pkg.version,
                None,
                &inputs,
                now,
                self,
            );
            self.policy_request(
                pkg.spirit_id.as_str(),
                &pkg.version,
                YankOrigin::Vetter,
                cause,
            )?;
            observed.insert(key);
        }
        Ok(())
    }
}
impl maos_compliance::TerminalObservationSink for YankRuntime {
    fn journal_terminal_observation(
        &self,
        observation: &maos_compliance::RunningSpiritObservation,
    ) {
        if let Err(error) = self.record("vetting-terminal-observation", observation) {
            panic!("terminal observation serialization failed before delivery: {error}");
        }
    }
}
impl YankObserver for YankRuntime {
    fn on_yank(&self, entry: &YankEntry) -> Result<(), RegistryError> {
        let Some((pkg, directory)) = Self::installed(entry.spirit_id.as_str())? else {
            return Ok(());
        };
        if pkg.version != entry.version {
            return Ok(());
        }
        let keyring = match std::fs::read(directory.join("vetting.json")) {
            Ok(bytes) => {
                let promotion: maos_registry::install::InstalledVetting =
                    serde_json::from_slice(&bytes)
                        .map_err(|e| RegistryError::Transport(e.to_string()))?;
                Some(
                    serde_cbor::from_slice::<maos_compliance::VetterKeyring>(
                        &std::fs::read(promotion.keyring_path)
                            .map_err(|e| RegistryError::Transport(e.to_string()))?,
                    )
                    .map_err(|e| RegistryError::Transport(e.to_string()))?,
                )
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(RegistryError::Transport(e.to_string())),
        };
        let root = if keyring.is_some() {
            let seed = maos_domain::audit_key::load_audit_key_seed(&None)
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            Some(maos_domain::audit_key::derive_audit_public_key(&seed))
        } else {
            None
        };
        maos_registry::yank_auth::verify_entry(
            entry,
            &pkg.publisher_pubkey,
            keyring.as_ref().zip(root.as_ref()),
            maos_registry::yank_auth::now_unix_ms().map_err(RegistryError::Transport)?,
        )
        .map_err(RegistryError::Transport)?;
        self.record("yank-poller:remote-yank-propagated", entry)?;
        self.policy_request(
            entry.spirit_id.as_str(),
            &entry.version,
            entry.origin.ok_or(RegistryError::SignatureInvalid)?,
            &entry.reason,
        )?;
        Ok(())
    }
    fn on_poll(&self) -> Result<(), RegistryError> {
        self.reevaluate_at(
            maos_registry::yank_auth::now_unix_ms().map_err(RegistryError::Transport)?,
        )
    }
}

/// Own the poller's kernel references through graceful and early-return teardown.
#[derive(Default)]
pub struct PollTask {
    task: Option<(
        Arc<std::sync::atomic::AtomicBool>,
        tokio::task::JoinHandle<()>,
    )>,
}
impl PollTask {
    pub fn start(
        poller: Arc<maos_registry::yank::YankPoller>,
        interval: std::time::Duration,
    ) -> Self {
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let task = tokio::spawn(maos_registry::yank::yank_poller_production_loop(
            poller,
            stop.clone(),
            interval,
        ));
        Self {
            task: Some((stop, task)),
        }
    }
    pub async fn shutdown(&mut self) {
        let Some((stop, task)) = self.task.take() else {
            return;
        };
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Err(error) = task.await {
            eprintln!("yank poller shutdown failed: {error}");
        }
    }
}
impl Drop for PollTask {
    fn drop(&mut self) {
        if let Some((stop, task)) = &self.task {
            stop.store(true, std::sync::atomic::Ordering::SeqCst);
            task.abort();
        }
    }
}

pub fn local_yank_crl(
    name: &str,
    version: &str,
    reason: &str,
) -> Result<maos_domain::revocation::SignedRevocationList, String> {
    use maos_domain::revocation::*;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let seed = maos_domain::audit_key::load_audit_key_seed(&None).map_err(|e| e.to_string())?;
    let key = Ed25519KeyPair::from_seed_unchecked(&seed).map_err(|e| e.to_string())?;
    let entries = vec![RevocationEntry::new(
        name,
        format!("={version}"),
        format!("registry-yank: {reason}"),
        None,
    )
    .map_err(|e| e.to_string())?];
    let signature = key
        .sign(&canonical_entries_bytes(&entries).map_err(|e| e.to_string())?)
        .as_ref()
        .try_into()
        .map_err(|_| "invalid signature length")?;
    SignedRevocationList::new(
        CrlId::from_entries(&entries).map_err(|e| e.to_string())?,
        1,
        maos_registry::yank_auth::now_unix_ms()?.saturating_mul(1_000_000),
        RevocationOrigin::RegistryYank,
        entries,
        signature,
        key.public_key()
            .as_ref()
            .try_into()
            .map_err(|_| "invalid public key length")?,
    )
    .map_err(|e| e.to_string())
}
