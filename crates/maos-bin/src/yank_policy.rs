//! Kernel-free yank decisions and running-promotion observation. Sticky quarantine
//! effects are deliberately owned by the final KQ1 landing, not a pause shim.
use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::ports::registry::{RegistryError, SignedPackage, YankEntry, YankOrigin};
use maos_kernel_core::iac::transparency_log::{FrameKind, TransparencyLogAdapter};
use maos_kernel_core::scheduler::{control_block::ScbLifecycleState, SpiritSchedulerAdapter};
use maos_registry::yank::{YankDisposition, YankObserver, YankPolicy, YankPolicyOutcome};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub struct YankRuntime {
    tl: Arc<TransparencyLogAdapter>,
    scheduler: Arc<SpiritSchedulerAdapter>,
    applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
    policy: YankPolicy,
    observed: Mutex<BTreeMap<(u32, String, String), bool>>,
    registry_uri: String,
    clock: fn() -> Result<u64, String>,
}
impl YankRuntime {
    pub fn new(
        tl: Arc<TransparencyLogAdapter>,
        scheduler: Arc<SpiritSchedulerAdapter>,
        applier: Arc<maos_kernel_core::revocation::RevocationApplier>,
        policy: YankPolicy,
        registry_uri: String,
    ) -> Self {
        Self {
            tl,
            scheduler,
            applier,
            policy,
            observed: Mutex::new(BTreeMap::new()),
            registry_uri,
            clock: maos_registry::yank_auth::now_unix_ms,
        }
    }
    pub fn with_clock(mut self, clock: fn() -> Result<u64, String>) -> Self {
        self.clock = clock;
        self
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
            let pkg: SignedPackage =
                serde_json::from_slice(&bytes).map_err(|_| RegistryError::SignatureInvalid)?;
            if pkg.spirit_id.as_str() != name
                || !maos_registry::admission::verify_publisher_sig(&pkg)
            {
                return Err(RegistryError::SignatureInvalid);
            }
            return Ok(Some((pkg, directory.to_path_buf())));
        }
        Ok(None)
    }
    fn request_for_running(
        &self,
        spirit: &maos_kernel_core::scheduler::control_block::SpiritControlBlock,
        pkg: &SignedPackage,
        origin: YankOrigin,
        cause: &str,
        inputs: &maos_compliance::TerminalInputs,
        now: u64,
    ) -> Result<(), RegistryError> {
        let key = (spirit.pid, pkg.version.clone(), cause.to_owned());
        {
            let mut observed = self
                .observed
                .lock()
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            match observed.get(&key) {
                Some(true) => return Ok(()),
                Some(false) => {}
                None => {
                    maos_compliance::observe_running_spirit(
                        pkg.spirit_id.as_str(),
                        &pkg.version,
                        None,
                        inputs,
                        now,
                        self,
                    );
                    observed.insert(key.clone(), false);
                }
            }
        }
        // Journal dedupe precedes effects; retry failed effects without holding
        // this mutex across the applier's asynchronous boundary.
        self.policy_request(pkg.spirit_id.as_str(), &pkg.version, origin, cause)?;
        self.observed
            .lock()
            .map_err(|e| RegistryError::Transport(e.to_string()))?
            .insert(key, true);
        Ok(())
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
        let mut first_error = None;
        for spirit in spirits {
            let result = (|| {
                if spirit.current_state() != ScbLifecycleState::Running {
                    return Ok(());
                }
                let Some((pkg, directory)) = Self::installed(&spirit.spirit_id)? else {
                    return Ok(());
                };
                if spirit
                    .runtime_snapshot()
                    .manifest
                    .class
                    .as_ref()
                    .is_none_or(|class| class.version != pkg.version)
                {
                    return Ok(());
                }
                let bytes = match std::fs::read(directory.join("vetting.json")) {
                    Ok(bytes) => bytes,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    Err(e) => return Err(RegistryError::Transport(e.to_string())),
                };
                let promotion: maos_registry::install::InstalledVetting =
                    serde_json::from_slice(&bytes)
                        .map_err(|e| RegistryError::Transport(e.to_string()))?;
                let seed = maos_domain::audit_key::load_audit_key_seed(&None)
                    .map_err(|e| RegistryError::Transport(e.to_string()))?;
                let root = maos_domain::audit_key::derive_audit_public_key(&seed);
                use maos_compliance::vetting::VettingRejection;
                let (cause, inputs) = match promotion.verify(&pkg, &root, now) {
                    Ok(_) => return Ok(()),
                    Err(VettingRejection::Expired { .. }) => (
                        "expiry-lapse",
                        maos_compliance::TerminalInputs {
                            attestation_expired: true,
                            ..Default::default()
                        },
                    ),
                    Err(
                        VettingRejection::VetterKeyRevoked { .. }
                        | VettingRejection::AttestationRevoked,
                    ) => (
                        "vetting-revocation",
                        maos_compliance::TerminalInputs {
                            vetting_revoked: true,
                            ..Default::default()
                        },
                    ),
                    Err(error) => return Err(RegistryError::Transport(error.to_string())),
                };
                self.request_for_running(&spirit, &pkg, YankOrigin::Vetter, cause, &inputs, now)
            })();
            if let Err(error) = result {
                self.record(
                    "vetting-reevaluation-rejected",
                    &serde_json::json!({"spirit_id":spirit.spirit_id, "reason":error.to_string()}),
                )?;
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
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
    fn on_yank(&self, entry: &YankEntry) -> Result<YankDisposition, RegistryError> {
        let installed = match Self::installed(entry.spirit_id.as_str()) {
            Ok(installed) => installed,
            Err(RegistryError::SignatureInvalid) => {
                return Ok(YankDisposition::Rejected(
                    "installed signed subject is invalid".into(),
                ))
            }
            Err(error) => return Err(error),
        };
        let publisher = match installed.as_ref() {
            Some((pkg, _)) => pkg.publisher_pubkey,
            None => {
                match maos_registry::storage::LocalFsRegistryStorage::default_installed_publisher(
                    entry.spirit_id.as_str(),
                )
                .map_err(|e| RegistryError::Transport(e.to_string()))?
                {
                    Some(publisher) => publisher,
                    None => {
                        return Ok(YankDisposition::Rejected(
                            "no pinned publisher for uninstalled subject".into(),
                        ))
                    }
                }
            }
        };
        // Publisher withdrawals do not depend on the availability of vetter metadata.
        let keyring = if entry.origin != Some(YankOrigin::Vetter) {
            None
        } else if let Some((_, directory)) = &installed {
            match std::fs::read(directory.join("vetting.json")) {
                Ok(bytes) => {
                    let promotion: maos_registry::install::InstalledVetting =
                        match serde_json::from_slice(&bytes) {
                            Ok(promotion) => promotion,
                            Err(error) => {
                                return Ok(YankDisposition::Rejected(format!(
                                    "invalid promotion metadata: {error}"
                                )))
                            }
                        };
                    let bytes = std::fs::read(promotion.keyring_path)
                        .map_err(|e| RegistryError::Transport(e.to_string()))?;
                    match serde_cbor::from_slice::<maos_compliance::VetterKeyring>(&bytes) {
                        Ok(keyring) => Some(keyring),
                        Err(error) => {
                            return Ok(YankDisposition::Rejected(format!(
                                "invalid vetter keyring: {error}"
                            )))
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(RegistryError::Transport(e.to_string())),
            }
        } else {
            None
        };
        let root = if keyring.is_some() {
            let seed = maos_domain::audit_key::load_audit_key_seed(&None)
                .map_err(|e| RegistryError::Transport(e.to_string()))?;
            Some(maos_domain::audit_key::derive_audit_public_key(&seed))
        } else {
            None
        };
        let now = (self.clock)().map_err(RegistryError::Transport)?;
        if let Err(reason) = maos_registry::yank_auth::verify_entry(
            entry,
            &publisher,
            keyring.as_ref().zip(root.as_ref()),
            &self.registry_uri,
            now,
        ) {
            return Ok(YankDisposition::Rejected(reason));
        }
        let Some((pkg, directory)) = installed else {
            self.record("yank-verified-not-actionable", entry)?;
            return Ok(YankDisposition::Verified);
        };
        let origin: maos_registry::origin::RegistryOrigin = serde_json::from_slice(
            &std::fs::read(directory.join("origin.json"))
                .map_err(|e| RegistryError::Transport(e.to_string()))?,
        )
        .map_err(|e| RegistryError::Transport(e.to_string()))?;
        if pkg.version != entry.version
            || !matches!(origin,
            maos_registry::origin::RegistryOrigin::Remote { ref uri } if uri == &self.registry_uri)
        {
            self.record("yank-verified-not-actionable", entry)?;
            return Ok(YankDisposition::Verified);
        }
        self.record("yank-poller:remote-yank-propagated", entry)?;
        let origin = entry.origin.ok_or(RegistryError::SignatureInvalid)?;
        let spirits: Vec<_> = self
            .scheduler
            .scbs()
            .read()
            .map_err(|e| RegistryError::Transport(e.to_string()))?
            .values()
            .filter(|spirit| {
                spirit.spirit_id == entry.spirit_id.as_str()
                    && spirit.current_state() == ScbLifecycleState::Running
                    && spirit
                        .runtime_snapshot()
                        .manifest
                        .class
                        .as_ref()
                        .is_some_and(|class| class.version == entry.version)
            })
            .cloned()
            .collect();
        if spirits.is_empty() {
            self.policy_request(pkg.spirit_id.as_str(), &pkg.version, origin, &entry.reason)?;
        } else {
            for spirit in spirits {
                self.request_for_running(
                    &spirit,
                    &pkg,
                    origin,
                    "registry-yank",
                    &maos_compliance::TerminalInputs {
                        registry_yanked: true,
                        ..Default::default()
                    },
                    now,
                )?;
            }
        }
        Ok(YankDisposition::Verified)
    }
    fn on_rejected(&self, entry: &YankEntry, reason: &str) -> Result<(), RegistryError> {
        let label = if reason.starts_with("yank-sequence-anomaly") {
            "yank-sequence-anomaly"
        } else {
            "yank-rejected"
        };
        self.record(
            label,
            &serde_json::json!({"sequence":entry.sequence,
            "spirit_id":entry.spirit_id, "version":entry.version, "reason":reason}),
        )
    }
    fn on_poll(&self) -> Result<(), RegistryError> {
        self.reevaluate_at((self.clock)().map_err(RegistryError::Transport)?)
    }
}

/// Own the poller's kernel references through graceful and early-return teardown.
#[derive(Default)]
pub struct PollTask {
    task: Option<(
        tokio::sync::watch::Sender<bool>,
        tokio::task::JoinHandle<()>,
    )>,
}
impl PollTask {
    pub fn start(
        poller: Arc<maos_registry::yank::YankPoller>,
        interval: std::time::Duration,
    ) -> Self {
        let (stop, stopped) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(maos_registry::yank::yank_poller_production_loop(
            poller, stopped, interval,
        ));
        Self {
            task: Some((stop, task)),
        }
    }
    /// No registry means no remote poller. This independent local timer still
    /// discovers later promotions, without blocking boot's first tick.
    pub fn start_local(observer: Arc<dyn YankObserver>, interval: std::time::Duration) -> Self {
        let (stop, mut stopped) = tokio::sync::watch::channel(false);
        let task = tokio::spawn(async move {
            loop {
                if *stopped.borrow() {
                    break;
                }
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        if let Err(error) = observer.on_poll() { eprintln!("local vetting re-evaluation failed: {error}"); }
                    },
                    _ = stopped.changed() => break,
                }
            }
        });
        Self {
            task: Some((stop, task)),
        }
    }
    pub async fn shutdown(&mut self) {
        let Some((stop, task)) = self.task.take() else {
            return;
        };
        let _ = stop.send(true);
        if let Err(error) = task.await {
            eprintln!("yank poller shutdown failed: {error}");
        }
    }
}
impl Drop for PollTask {
    fn drop(&mut self) {
        if let Some((stop, task)) = &self.task {
            let _ = stop.send(true);
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
