//! Enterprise identity / at-rest / SIEM composition-root runtime (Story 11.4c).
//!
//! REAL adapter wiring (not a posture stub). When an enterprise integration is
//! configured AND healthy, the runtime routes through the real out-of-kernel
//! adapters:
//! - **SSO** — `maos_sso::OidcVerifier` (real JWKS signature verify + claim
//!   enforcement). A verified assertion is REQUIRED before issuance under the
//!   SSO posture, and an out-of-kernel `identity.asserted` (kind 30) provenance
//!   row is persisted to the Transparency Log.
//! - **KMS** — `maos_secrets` envelope AEAD via `seal_at_rest_opt`. Sealed rows
//!   are real ciphertext (not passthrough plaintext). The `identity.asserted`
//!   row above is plain JSON and is NOT sealed by Story 23-8:
//!   `EnterpriseRuntime::seal_row_at_rest` has no production caller.
//! - **SIEM** — `maos_siem::SiemExporter` + localhost file sink; redaction is
//!   applied before projection; sink-down fails closed.
//!
//! Zero config preserves the v1.5 byte-identical posture (no SSO gate, Option-A
//! plaintext rows, no SIEM forwarding). A configured-but-unavailable subsystem
//! is detected at construction (env present but adapter unbuildable / unhealthy)
//! AND re-checked at call time (`is_healthy`), failing closed per subsystem
//! instead of silently falling open (D10 / Grumbal).
//!
//! This module is `network`-gated and stays OUT of `api.rs` — it is a
//! composition-root concern, not a kernel-core adapter (keeps
//! `check-composition-root-completeness` green). It touches no kernel-core
//! surface (L1): the principal never welds into the frozen `CapabilityToken`,
//! and `identity.asserted` is written through the composition-root
//! [`append_identity_asserted`] helper below — a raw kind-30 row, not a kernel
//! `FrameKind` variant. The helper lives HERE (the writer side), not in the
//! read-only `maos-audit` crate (Story 9.2 Decision A.2 / `maos-audit-read-only`
//! gate).

use std::net::{IpAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maos_audit::AuditError;
pub use maos_audit::AuditFilter;
#[cfg(feature = "online-adapters")]
use maos_domain::outbound::OutboundPolicy;
use maos_domain::outbound::{ResolveFailed, ResolveHost};
#[cfg(feature = "online-adapters")]
use maos_domain::ports::KmsError;
use maos_domain::ports::{
    AuthenticatedPrincipal, CryptoProvider, IdentityAssertionPort, KeyManagementPort,
    SiemProjectionPort,
};
use maos_loom_lite::seal::AtRestSeal;
use maos_secrets::{seal_at_rest_opt, LocalMasterKeyKms};
use maos_siem::cursor::CursorStore;
use maos_siem::export::{run_tick_cancellable, BatchSink, FileSink, Tick};
use maos_siem::SiemExporter;
use maos_sso::{OidcAlgorithm, OidcVerifier};
use rusqlite::OpenFlags;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubsystemState {
    Disabled,
    Available,
    ConfiguredDown,
}

impl SubsystemState {
    fn configured(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterpriseConfig {
    sso: SubsystemState,
    kms: SubsystemState,
    siem: SubsystemState,
}

impl EnterpriseConfig {
    pub fn empty() -> Self {
        Self {
            sso: SubsystemState::Disabled,
            kms: SubsystemState::Disabled,
            siem: SubsystemState::Disabled,
        }
    }

    /// Coarse env-presence probe. `EnterpriseRuntime::from_env` performs the
    /// REAL adapter construction + health check and may demote a probed
    /// `Available` to `ConfiguredDown`.
    pub fn from_env() -> Self {
        let mut config = Self::empty();
        if any_env_with_prefix("MAOS_SSO_") {
            config.sso = SubsystemState::Available;
        }
        if any_env_with_prefix("MAOS_KMS_") {
            config.kms = SubsystemState::Available;
        }
        if any_env_with_prefix("MAOS_SIEM_") {
            config.siem = SubsystemState::Available;
        }
        config
    }

    pub fn with_sso_down(mut self) -> Self {
        self.sso = SubsystemState::ConfiguredDown;
        self
    }

    pub fn with_kms_down(mut self) -> Self {
        self.kms = SubsystemState::ConfiguredDown;
        self
    }

    pub fn with_siem_down(mut self) -> Self {
        self.siem = SubsystemState::ConfiguredDown;
        self
    }

    /// Story 13.5a — declare a subsystem `Available` without probing process
    /// env. `from_env` is the production prober; the composition-root daemon
    /// proof needs "Available + this exact injected port" without mutating a
    /// shared test binary's environment.
    pub fn with_sso_available(mut self) -> Self {
        self.sso = SubsystemState::Available;
        self
    }

    /// Story 13.5a — see [`Self::with_sso_available`].
    pub fn with_kms_available(mut self) -> Self {
        self.kms = SubsystemState::Available;
        self
    }

    /// Story 13.5a — see [`Self::with_sso_available`].
    pub fn with_siem_available(mut self) -> Self {
        self.siem = SubsystemState::Available;
        self
    }

    pub fn sso_configured(&self) -> bool {
        self.sso.configured()
    }

    pub fn kms_configured(&self) -> bool {
        self.kms.configured()
    }

    pub fn siem_configured(&self) -> bool {
        self.siem.configured()
    }
}

pub struct EnterpriseRuntime {
    config: EnterpriseConfig,
    identity_port: Option<Arc<dyn IdentityAssertionPort>>,
    kms_port: Option<Arc<dyn KeyManagementPort>>,
    siem_port: Option<Arc<dyn SiemProjectionPort>>,
    /// Injected crypto provider (the same `Arc<dyn CryptoProvider>` the daemon
    /// holds for sealed-export). `None` on the test/posture `from_config` path.
    crypto: Option<Arc<dyn CryptoProvider>>,
    audit_db_path: Option<PathBuf>,
    siem_sink_path: Option<PathBuf>,
    /// Story 23-4 — the syslog-over-TLS sink `MAOS_SIEM_ENDPOINT` selected (its client
    /// key may still be pending).
    #[cfg(feature = "online-adapters")]
    siem_network: Option<Arc<NetworkSiem>>,
    /// One export tick at a time: the consumer is the only production caller, but a
    /// second concurrent tick would race the cursor.
    siem_tick: std::sync::Mutex<()>,
    /// Raised at shutdown: the running export stops at its next step.
    siem_cancel: Arc<std::sync::atomic::AtomicBool>,
    /// Acknowledged batches so far (D-23-4-09 A).
    siem_delivery: tokio::sync::watch::Sender<u64>,
    boot_nonce: u64,
    /// Probe of the external KMS endpoint (`None` for a local master key).
    kms_probe: Option<KmsProbeFn>,
}

impl EnterpriseRuntime {
    /// Test/posture constructor: no real adapters. Use ONLY for the
    /// zero-config / configured-but-down state-machine tests. The `Available`
    /// arms require [`Self::from_env`], which populates the real ports.
    pub fn from_config(config: &EnterpriseConfig) -> Result<Self, EnterpriseFailure> {
        Ok(Self {
            config: config.clone(),
            identity_port: None,
            kms_port: None,
            siem_port: None,
            crypto: None,
            audit_db_path: None,
            siem_sink_path: None,
            #[cfg(feature = "online-adapters")]
            siem_network: None,
            siem_tick: std::sync::Mutex::new(()),
            siem_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            siem_delivery: tokio::sync::watch::channel(0).0,
            boot_nonce: 0,
            kms_probe: None,
        })
    }

    /// Production constructor: builds the REAL adapters from env and demotes any
    /// subsystem that cannot be built or is unhealthy to `ConfiguredDown`
    /// (runtime-detected, not just a test fixture). The KMS is the one
    /// `configure_kms_from_env` selected at boot (Story 23-8, D-23-8-08 A).
    pub fn from_env(
        crypto: Arc<dyn CryptoProvider>,
        audit_db_path: PathBuf,
        boot_nonce: u64,
        kms: Option<&ConfiguredKms>,
        siem: Option<&ConfiguredSiem>,
    ) -> Result<Self, EnterpriseFailure> {
        let mut config = EnterpriseConfig::from_env();
        let mut identity_port: Option<Arc<dyn IdentityAssertionPort>> = None;
        let mut kms_port: Option<Arc<dyn KeyManagementPort>> = None;
        let mut siem_port: Option<Arc<dyn SiemProjectionPort>> = None;

        if config.sso_configured() {
            match build_sso_verifier() {
                Ok(verifier) if verifier.is_healthy() => {
                    identity_port = Some(Arc::new(verifier));
                }
                _ => config = config.with_sso_down(),
            }
        }

        // The KMS is selected and validated at boot (`configure_kms_from_env`);
        // a configured KMS with no adapter here is a latched ConfiguredDown. An
        // endpoint that is unhealthy is NOT demoted: health is per call.
        if config.kms_configured() {
            match kms {
                Some(kms) => kms_port = Some(kms.port()),
                None => config = config.with_kms_down(),
            }
        }

        if config.siem_configured() {
            // The sink is the one `configure_siem_from_env` selected at boot (Story 23-4,
            // D-23-4-12 A); with none selected (`MAOS_SIEM_HOSTNAME` alone) the posture
            // is a latched ConfiguredDown.
            if siem.is_some() {
                siem_port = Some(Arc::new(SiemExporter));
            } else {
                config = config.with_siem_down();
            }
        }

        let mut runtime = Self::from_ports(
            config,
            identity_port,
            kms_port,
            siem_port,
            Some(crypto),
            Some(audit_db_path),
            None,
            boot_nonce,
        );
        if let Some(siem) = siem.filter(|_| runtime.siem_port.is_some()) {
            runtime = runtime.with_configured_siem(siem);
        }
        Ok(match kms.and_then(ConfiguredKms::probe) {
            Some(probe) => runtime.with_kms_probe(probe),
            None => runtime,
        })
    }

    /// Route the SIEM export to the sink `configure_siem_from_env` selected (Story 23-4).
    #[must_use]
    pub fn with_configured_siem(mut self, siem: &ConfiguredSiem) -> Self {
        self.siem_sink_path = siem.file_path();
        #[cfg(feature = "online-adapters")]
        {
            self.siem_network = siem.network();
        }
        if let Some(db) = &self.audit_db_path {
            remove_stale_snapshots(db);
        }
        self
    }

    /// Ask a running export to stop at its next step (a bounded read slice, a batch boundary):
    /// shutdown cannot interrupt a blocking thread, so the export looks for this flag itself.
    pub fn cancel_siem_export(&self) {
        self.siem_cancel
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Story 13.5a — port-injection constructor. The production `from_env`
    /// above builds the real adapters and then lands here, so this is not a
    /// zero-caller test seam; the `cohort-a2a-daemon` lifecycle proof reuses it
    /// to drive the SAME governance chain against recording ports with no
    /// external SSO/KMS/SIEM substrate and no process-env mutation.
    #[allow(clippy::too_many_arguments)]
    pub fn from_ports(
        config: EnterpriseConfig,
        identity_port: Option<Arc<dyn IdentityAssertionPort>>,
        kms_port: Option<Arc<dyn KeyManagementPort>>,
        siem_port: Option<Arc<dyn SiemProjectionPort>>,
        crypto: Option<Arc<dyn CryptoProvider>>,
        audit_db_path: Option<PathBuf>,
        siem_sink_path: Option<PathBuf>,
        boot_nonce: u64,
    ) -> Self {
        Self {
            config,
            identity_port,
            kms_port,
            siem_port,
            crypto,
            audit_db_path,
            siem_sink_path,
            #[cfg(feature = "online-adapters")]
            siem_network: None,
            siem_tick: std::sync::Mutex::new(()),
            siem_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            siem_delivery: tokio::sync::watch::channel(0).0,
            boot_nonce,
            kms_probe: None,
        }
    }

    pub fn is_noop(&self) -> bool {
        !self.config.sso_configured()
            && !self.config.kms_configured()
            && !self.config.siem_configured()
            && self.identity_port.is_none()
            && self.kms_port.is_none()
            && self.siem_port.is_none()
    }

    pub fn sso_configured(&self) -> bool {
        self.config.sso_configured()
    }

    pub fn kms_configured(&self) -> bool {
        self.config.kms_configured()
    }

    pub fn siem_configured(&self) -> bool {
        self.config.siem_configured()
    }

    /// Seal a store row at rest. `Disabled` (no KMS) → byte-identical Option-A
    /// plaintext. `Available` + healthy KMS → real AEAD ciphertext.
    /// `ConfiguredDown` or unhealthy → sealed write REFUSED (never silent
    /// plaintext under an encryption posture — Vex's #1 at-rest defeat).
    pub fn seal_row_at_rest(&self, plaintext: &[u8]) -> Result<Vec<u8>, EnterpriseFailure> {
        match self.config.kms {
            SubsystemState::Disabled => Ok(plaintext.to_vec()),
            SubsystemState::Available => {
                let (kms, crypto) = match (&self.kms_port, &self.crypto) {
                    (Some(k), Some(c)) if k.is_healthy() => (k, c),
                    _ => {
                        return Err(EnterpriseFailure::KmsSealedWriteRefused {
                            reason: "MAOS_KMS_* configured but KMS is unavailable".to_string(),
                        })
                    }
                };
                seal_at_rest_opt(Some(kms.as_ref()), crypto.as_ref(), plaintext).map_err(|e| {
                    EnterpriseFailure::KmsSealedWriteRefused {
                        reason: e.to_string(),
                    }
                })
            }
            SubsystemState::ConfiguredDown => Err(EnterpriseFailure::KmsSealedWriteRefused {
                reason: "MAOS_KMS_* configured but KMS is unavailable".to_string(),
            }),
        }
    }

    /// Verify the SSO assertion for a capability issuance and return the
    /// authenticated principal to the composition root. `Disabled` means no SSO
    /// gate is active and returns `Ok(None)` (byte-identical default). Available
    /// SSO verifies the assertion and returns `Some(principal)`. Configured-down
    /// or verification failure denies issuance fail-closed.
    pub fn verify_principal_for_issuance(
        &self,
        spirit_pid: u32,
        assertion: &str,
    ) -> Result<Option<AuthenticatedPrincipal>, EnterpriseFailure> {
        match self.config.sso {
            SubsystemState::Disabled => Ok(None),
            SubsystemState::Available => {
                let port = match &self.identity_port {
                    Some(p) if p.is_healthy() => p,
                    _ => {
                        return Err(EnterpriseFailure::SsoIssuanceDenied {
                            spirit_pid,
                            reason: "MAOS_SSO_* configured but identity provider is unavailable"
                                .to_string(),
                        })
                    }
                };
                port.verify(assertion)
                    .map(Some)
                    .map_err(|e| EnterpriseFailure::SsoIssuanceDenied {
                        spirit_pid,
                        reason: e.to_string(),
                    })
            }
            SubsystemState::ConfiguredDown => Err(EnterpriseFailure::SsoIssuanceDenied {
                spirit_pid,
                reason: "MAOS_SSO_* configured but identity provider is unavailable".to_string(),
            }),
        }
    }

    /// Persist the out-of-kernel `identity.asserted` row after the composition
    /// root has allowed the issuance. No-op when audit DB is absent (test/posture
    /// constructor). This deliberately stays outside kernel-core: no token field,
    /// no kernel `FrameKind` mutation.
    pub fn persist_identity_asserted(
        &self,
        spirit_pid: u32,
        principal: &AuthenticatedPrincipal,
        capability_key: &str,
    ) -> Result<(), EnterpriseFailure> {
        let decision_time_ns = now_ns().map_err(|e| EnterpriseFailure::SsoIssuanceDenied {
            spirit_pid,
            reason: format!("system clock unavailable for identity.asserted: {e}"),
        })?;
        if let Some(db) = &self.audit_db_path {
            append_identity_asserted(
                db,
                spirit_pid,
                self.boot_nonce,
                &principal.subject,
                &principal.issuer,
                capability_key,
                decision_time_ns,
            )
            .map_err(|e| EnterpriseFailure::SsoIssuanceDenied {
                spirit_pid,
                reason: format!("identity.asserted persist failed: {e}"),
            })?;
        }
        Ok(())
    }

    /// Backward-compatible convenience for tests and posture probes: verify then
    /// persist without a PDP check. Live capability issuance uses the
    /// composition-root governed wrapper so principal attributes can feed the
    /// PDP before the token is minted.
    pub fn issue_under_principal(
        &self,
        spirit_pid: u32,
        assertion: &str,
        capability_key: &str,
    ) -> Result<(), EnterpriseFailure> {
        if let Some(principal) = self.verify_principal_for_issuance(spirit_pid, assertion)? {
            self.persist_identity_asserted(spirit_pid, &principal, capability_key)?;
        }
        Ok(())
    }

    /// One cursor-driven export tick to the configured SIEM sink (Story 23-4).
    ///
    /// `Disabled` → nothing. `Available` → `maos_siem::export::run_tick`: the redacted log
    /// is read after the durable cursor (insertion-id order, the sanctioned reader) and
    /// sent in bounded batches; the cursor moves only after the sink acknowledged a batch.
    /// Any other outcome, and `ConfiguredDown`, is `SiemSinkDown` with the reason code and
    /// the rows left after the cursor: the log itself is the spool, so nothing is dropped.
    /// Blocking: callers run it on a blocking thread, never on an async worker.
    pub fn forward_audit_to_siem(&self) -> Result<Tick, EnterpriseFailure> {
        let down = |reason: &'static str| EnterpriseFailure::SiemSinkDown {
            buffered: 0,
            reason,
            notes: Vec::new(),
        };
        match self.config.siem {
            SubsystemState::Disabled => Ok(Tick::default()),
            SubsystemState::Available => {
                let _one_tick_at_a_time = self
                    .siem_tick
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let port = self
                    .siem_port
                    .as_ref()
                    .filter(|port| port.is_healthy())
                    .ok_or(down("siem_sink_unconfigured"))?;
                let db = self
                    .audit_db_path
                    .as_ref()
                    .ok_or(down("siem_sink_unconfigured"))?;
                let mut sink = self.siem_sink(port).map_err(down)?;

                // `maos-audit` deliberately accepts only a quiesced database for
                // deterministic redaction/export. A running daemon necessarily
                // owns a live WAL, so take a transactionally consistent SQLite
                // snapshot and export that snapshot within this same daemon
                // lifecycle instead of silently buffering every live record.
                // `VACUUM INTO` copies each row's rowid (SQLite 3.45.0), so insertion
                // ids read from the snapshot are the live ones (the cursor's unit).
                static SNAPSHOT_SEQUENCE: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(0);
                let wal_path = db.with_extension("sqlite-wal");
                let mut snapshot = None;
                let source_db = if wal_path.exists() {
                    let sequence =
                        SNAPSHOT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let path = db.with_extension(format!(
                        "siem-snapshot-{}-{sequence}.sqlite",
                        std::process::id()
                    ));
                    for suffix in ["", "-wal", "-shm"] {
                        let mut sidecar = path.clone().into_os_string();
                        sidecar.push(suffix);
                        let _ = std::fs::remove_file(sidecar);
                    }
                    // The snapshot holds the whole redacted log: create it owner-only BEFORE
                    // SQLite writes into it (`VACUUM INTO` accepts an existing empty file).
                    let mut create = std::fs::OpenOptions::new();
                    create.write(true).create_new(true);
                    #[cfg(unix)]
                    std::os::unix::fs::OpenOptionsExt::mode(&mut create, 0o600);
                    if create.open(&path).is_err() {
                        return Err(down("siem_source_unavailable"));
                    }
                    let connection = rusqlite::Connection::open(db)
                        .map_err(|_| down("siem_source_unavailable"))?;
                    let path_text = path.to_string_lossy().into_owned();
                    if connection.execute("VACUUM INTO ?1", [&path_text]).is_err() {
                        let _ = std::fs::remove_file(&path);
                        return Err(down("siem_source_unavailable"));
                    }
                    snapshot = Some(path.clone());
                    path
                } else {
                    db.clone()
                };

                let store = CursorStore::beside(db, &sink.identity());
                let boot_nonce = self.boot_nonce;
                sink.set_cancel(Arc::clone(&self.siem_cancel));
                let outcome = run_tick_cancellable(
                    &source_db,
                    &store,
                    sink.as_mut(),
                    &|intent, payload| {
                        append_siem_event(db, boot_nonce, intent, &payload)
                            .map_err(|error| error.to_string())
                    },
                    &self.siem_cancel,
                );
                if let Some(path) = snapshot {
                    for suffix in ["", "-wal", "-shm"] {
                        let mut sidecar = path.clone().into_os_string();
                        sidecar.push(suffix);
                        let _ = std::fs::remove_file(sidecar);
                    }
                }
                match outcome {
                    Ok(tick) => {
                        if tick.batches > 0 {
                            self.siem_delivery
                                .send_modify(|batches| *batches += tick.batches as u64);
                        }
                        Ok(tick)
                    }
                    // The tick's completed work (a gap or reset it journaled before the sink
                    // failed) travels with the error, so its typed line always prints.
                    Err(failure) => Err(EnterpriseFailure::SiemSinkDown {
                        buffered: usize::try_from(failure.buffered()).unwrap_or(usize::MAX),
                        reason: failure.code(),
                        notes: failure.tick.notes,
                    }),
                }
            }
            SubsystemState::ConfiguredDown => Err(down("siem_sink_unconfigured")),
        }
    }

    /// Acknowledged batches so far (Story 23-4, D-23-4-09 A): the event the lifecycle proof
    /// waits on instead of sleeping.
    #[must_use]
    pub fn siem_delivery_events(&self) -> tokio::sync::watch::Receiver<u64> {
        self.siem_delivery.subscribe()
    }

    /// The sink this runtime exports to: the network sink when `MAOS_SIEM_ENDPOINT` selected
    /// one (refused until its client key is settled), else the file sink.
    fn siem_sink(
        &self,
        port: &Arc<dyn SiemProjectionPort>,
    ) -> Result<Box<dyn BatchSink>, &'static str> {
        #[cfg(feature = "online-adapters")]
        if let Some(network) = &self.siem_network {
            return network
                .sink()
                .map(|sink| Box::new(sink) as Box<dyn BatchSink>)
                .ok_or("siem_client_key_pending");
        }
        match &self.siem_sink_path {
            Some(path) => Ok(Box::new(FileSink::new(path.clone(), Arc::clone(port)))),
            None => Err("siem_sink_unconfigured"),
        }
    }

    /// Build the loom-lite at-rest seal hook from the configured KMS + crypto.
    ///
    /// `None` only when KMS is not configured (loom-lite then writes
    /// byte-identical Option-A plaintext). Whenever KMS IS configured the hook
    /// exists (Story 23-8, D-23-8-07 A): `Available` returns a closure that
    /// seals and fails closed on every write, including when the KMS is down
    /// or unhealthy (`seal_at_rest` checks `is_healthy` per call), and
    /// `ConfiguredDown` (or a missing port) returns a closure that always
    /// refuses. A configured posture therefore never falls back to plaintext.
    pub fn at_rest_seal_hook(&self) -> Option<AtRestSeal> {
        match self.config.kms {
            SubsystemState::Disabled => None,
            SubsystemState::ConfiguredDown => Some(refusing_seal()),
            SubsystemState::Available => match (self.kms_port.clone(), self.crypto.clone()) {
                (Some(kms), Some(crypto)) => Some(Arc::new(move |plaintext: &[u8]| {
                    seal_at_rest_opt(Some(kms.as_ref()), crypto.as_ref(), plaintext)
                        .map_err(|e| e.to_string())
                })),
                _ => Some(refusing_seal()),
            },
        }
    }

    /// KMS is configured but has no usable adapter or crypto provider (a
    /// latched `ConfiguredDown`, or `Available` with nothing behind it). The
    /// cohort daemon refuses to start in this state. An external endpoint that
    /// is merely unhealthy is not this: it recovers by probe.
    pub fn kms_unbuilt(&self) -> bool {
        match self.config.kms {
            SubsystemState::Disabled => false,
            SubsystemState::ConfiguredDown => true,
            SubsystemState::Available => self.kms_port.is_none() || self.crypto.is_none(),
        }
    }

    /// The probe of the external KMS endpoint, when there is one.
    pub fn kms_probe(&self) -> Option<KmsProbeFn> {
        self.kms_probe.clone()
    }

    /// Attach the endpoint probe (`ConfiguredKms::probe`).
    #[must_use]
    pub fn with_kms_probe(mut self, probe: KmsProbeFn) -> Self {
        self.kms_probe = Some(probe);
        self
    }

    /// Spawn the periodic endpoint probe (every [`KMS_PROBE_INTERVAL`]), if the
    /// KMS is an external endpoint. Stopped by `cancel`.
    pub fn spawn_kms_probe_task(
        &self,
        cancel: CancellationToken,
    ) -> Option<tokio::task::JoinHandle<()>> {
        self.kms_probe()
            .map(|probe| spawn_kms_probe(probe, KMS_PROBE_INTERVAL, cancel))
    }
}

fn refusing_seal() -> AtRestSeal {
    Arc::new(|_: &[u8]| Err("MAOS_KMS_* configured but KMS is unavailable".to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnterpriseFailure {
    #[error("SSO issuance denied for spirit_pid {spirit_pid}: {reason}")]
    SsoIssuanceDenied { spirit_pid: u32, reason: String },
    #[error("KMS sealed write refused: {reason}")]
    KmsSealedWriteRefused { reason: String },
    #[error("SIEM sink down ({reason}); {buffered} record(s) buffered")]
    SiemSinkDown {
        buffered: usize,
        reason: &'static str,
        /// Typed lines (gap, reset) the failed tick completed before it stopped.
        notes: Vec<String>,
    },
}

/// The operator lines of the SIEM export consumer (Story 23-4, AC5): the gap and reset notes
/// of every tick, one line per DISTINCT failure reason (a collector that stays down prints
/// once, not once a minute), one line when the sink answers again, and a summary line per
/// tick that delivered rows. Pure, so the dedupe is testable without a clock or a stderr.
#[derive(Debug, Default)]
pub struct SiemReport {
    last_failure: Option<&'static str>,
}

impl SiemReport {
    /// The lines to print for one tick's `outcome`.
    pub fn lines(&mut self, outcome: &Result<Tick, EnterpriseFailure>) -> Vec<String> {
        let mut lines = Vec::new();
        match outcome {
            Ok(tick) => {
                if self.last_failure.take().is_some() {
                    lines.push("maos: SIEM export is acknowledged again".to_owned());
                }
                lines.extend(tick.notes.iter().map(|note| format!("maos: {note}")));
                if tick.delivered > 0 {
                    lines.push(format!(
                        "maos: SIEM export delivered {} record(s) in {} acknowledged batch(es) (Story 23-4)",
                        tick.delivered, tick.batches
                    ));
                }
            }
            Err(EnterpriseFailure::SiemSinkDown {
                buffered,
                reason,
                notes,
            }) => {
                lines.extend(notes.iter().map(|note| format!("maos: {note}")));
                if self.last_failure != Some(reason) {
                    self.last_failure = Some(reason);
                    lines.push(format!(
                        "maos: SIEM export not acknowledged ({reason}) — {buffered} record(s) \
                         remain in the Transparency Log and are re-sent; operator action may be required"
                    ));
                }
            }
            Err(other) => lines.push(format!("maos: SIEM export failed: {other}")),
        }
        lines
    }
}

/// Remove `siem-snapshot-*` files (and their `-wal`/`-shm`) a crashed export left beside the
/// audit database. They hold a copy of the redacted log; the daemon removes them at boot, and
/// `maos purge` names any that remain. Called once while the single exporter is being built.
fn remove_stale_snapshots(db: &Path) {
    let (Some(dir), Some(stem)) = (db.parent(), db.file_stem()) else {
        return;
    };
    let prefix = format!("{}.siem-snapshot-", stem.to_string_lossy());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with(&prefix) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn any_env_with_prefix(prefix: &str) -> bool {
    std::env::vars_os().any(|(name, _)| name.to_string_lossy().starts_with(prefix))
}

// ── SIEM sink selection (Story 23-4, D-23-4-12 A) ───────────────────────────────────────

/// Why the SIEM configuration is refused at boot (exit 78). Each variant names the variables
/// involved and never a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiemBootRefusal {
    /// Both `MAOS_SIEM_ENDPOINT` and `MAOS_SIEM_FILE` are set: one sink per Host.
    Conflict,
    /// `MAOS_SIEM_ENDPOINT` is set in a build without `online-adapters`.
    UnsupportedBuild,
    /// The endpoint is not `host:port`, or failed the outbound policy (the code names which).
    EndpointRefused(&'static str),
    /// `MAOS_SIEM_ENDPOINT` is set without `MAOS_SIEM_CA_FILE`.
    CaMissing,
    /// The CA file is unreadable or holds no certificate.
    CaInvalid,
    /// `MAOS_SIEM_CLIENT_KEY` is set without `MAOS_SIEM_CLIENT_CERT_FILE`.
    ClientIncomplete,
    /// The client certificate file is unreadable or holds no certificate.
    ClientCertInvalid,
    /// `MAOS_SIEM_CLIENT_CERT_FILE` is set and the key is missing, unparsable, or not its key.
    ClientKeyInvalid,
}

impl SiemBootRefusal {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Conflict => "siem_config_conflict",
            Self::UnsupportedBuild => "siem_endpoint_unsupported_build",
            Self::EndpointRefused(_) => "siem_endpoint_refused",
            Self::CaMissing => "siem_ca_missing",
            Self::CaInvalid => "siem_ca_invalid",
            Self::ClientIncomplete => "siem_client_incomplete",
            Self::ClientCertInvalid => "siem_client_cert_invalid",
            Self::ClientKeyInvalid => "siem_client_key_invalid",
        }
    }
}

impl std::fmt::Display for SiemBootRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.code();
        match self {
            Self::Conflict => write!(
                f,
                "{code}: MAOS_SIEM_ENDPOINT and MAOS_SIEM_FILE are both set; set exactly one"
            ),
            Self::UnsupportedBuild => write!(
                f,
                "{code}: MAOS_SIEM_ENDPOINT is set but this build has no online adapters"
            ),
            Self::EndpointRefused(detail) => {
                write!(f, "{code}: MAOS_SIEM_ENDPOINT refused ({detail})")
            }
            Self::CaMissing => write!(
                f,
                "{code}: MAOS_SIEM_ENDPOINT needs MAOS_SIEM_CA_FILE (no system roots are trusted)"
            ),
            Self::CaInvalid => write!(
                f,
                "{code}: MAOS_SIEM_CA_FILE is unreadable or holds no PEM certificate"
            ),
            Self::ClientIncomplete => write!(
                f,
                "{code}: MAOS_SIEM_CLIENT_KEY is set but MAOS_SIEM_CLIENT_CERT_FILE is not"
            ),
            Self::ClientCertInvalid => write!(
                f,
                "{code}: MAOS_SIEM_CLIENT_CERT_FILE is unreadable or holds no PEM certificate"
            ),
            Self::ClientKeyInvalid => write!(
                f,
                "{code}: MAOS_SIEM_CLIENT_CERT_FILE is set but MAOS_SIEM_CLIENT_KEY is missing, \
                 not a PEM private key, or not the key of that certificate"
            ),
        }
    }
}

impl std::error::Error for SiemBootRefusal {}

/// The SIEM-related environment, read once (so tests drive selection without mutating the
/// process environment).
#[derive(Debug, Clone, Copy, Default)]
pub struct SiemEnv<'a> {
    pub endpoint: Option<&'a str>,
    pub file: Option<&'a str>,
    pub ca_file: Option<&'a str>,
    pub client_cert_file: Option<&'a str>,
    /// `MAOS_SIEM_CLIENT_KEY` is set in the environment (a key held only in the OS keyring is
    /// checked when the credential store is read).
    pub client_key_in_env: bool,
    /// `MAOS_SIEM_ALLOW_PRIVATE`: admit RFC 1918 / fc00::/7 and loopback endpoints.
    pub allow_private: bool,
}

/// The SIEM sink the composition root selected (one per Host): built once at boot and handed
/// to the enterprise runtime, so there is one sink and one cursor.
pub struct ConfiguredSiem {
    kind: SiemKind,
}

enum SiemKind {
    File(PathBuf),
    #[cfg(feature = "online-adapters")]
    Network(Arc<NetworkSiem>),
}

impl std::fmt::Debug for ConfiguredSiem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfiguredSiem")
            .field("kind", &self.kind_name())
            .finish_non_exhaustive()
    }
}

impl ConfiguredSiem {
    /// `"syslog over TLS"` or `"local file"`.
    #[must_use]
    pub fn kind_name(&self) -> &'static str {
        match self.kind {
            SiemKind::File(_) => "local file",
            #[cfg(feature = "online-adapters")]
            SiemKind::Network(_) => "syslog over TLS",
        }
    }

    #[must_use]
    pub fn file_path(&self) -> Option<PathBuf> {
        match &self.kind {
            SiemKind::File(path) => Some(path.clone()),
            #[cfg(feature = "online-adapters")]
            SiemKind::Network(_) => None,
        }
    }

    #[cfg(feature = "online-adapters")]
    #[must_use]
    pub fn network(&self) -> Option<Arc<NetworkSiem>> {
        match &self.kind {
            SiemKind::Network(network) => Some(Arc::clone(network)),
            SiemKind::File(_) => None,
        }
    }

    /// Whether the sink needs the client key from the credential store (a client certificate
    /// is configured).
    #[must_use]
    pub fn needs_client_key(&self) -> bool {
        match &self.kind {
            SiemKind::File(_) => false,
            #[cfg(feature = "online-adapters")]
            SiemKind::Network(network) => network.needs_client_key(),
        }
    }

    /// Settle the client key (`None` = the credential store holds none). With a client
    /// certificate configured a missing, unparsable or foreign key is a refusal; without
    /// one this is a no-op. Until it runs a network sink with a certificate sends nothing.
    ///
    /// # Errors
    /// [`SiemBootRefusal::ClientKeyInvalid`] (or the certificate / CA refusal).
    pub fn provision_client_key(&self, key: Option<&str>) -> Result<(), SiemBootRefusal> {
        match &self.kind {
            SiemKind::File(_) => Ok(()),
            #[cfg(feature = "online-adapters")]
            SiemKind::Network(network) => network.provision_client_key(key),
        }
    }
}

/// Select the SIEM sink (D-23-4-12 A). `Ok(None)` when neither `MAOS_SIEM_ENDPOINT` nor
/// `MAOS_SIEM_FILE` is set (a lone `MAOS_SIEM_HOSTNAME` keeps its configured-down posture).
/// An invalid configuration is a refusal, never a silent downgrade. The endpoint is
/// operator configuration: its host class is vetted here, once (a name that does not resolve
/// yet is not a refusal: the sink is simply down until it does), and again for every batch
/// connection.
///
/// # Errors
/// The [`SiemBootRefusal`] of the first check that fails.
pub fn configure_siem(
    env: &SiemEnv<'_>,
    resolver: Arc<dyn ResolveHost>,
) -> Result<Option<ConfiguredSiem>, SiemBootRefusal> {
    let endpoint = env
        .endpoint
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let file = env.file.filter(|value| !value.is_empty());
    match (endpoint, file) {
        (Some(_), Some(_)) => Err(SiemBootRefusal::Conflict),
        (Some(endpoint), None) => network_siem(env, endpoint, resolver).map(Some),
        (None, Some(path)) => Ok(Some(ConfiguredSiem {
            kind: SiemKind::File(PathBuf::from(path)),
        })),
        (None, None) => Ok(None),
    }
}

/// [`configure_siem`] over the process environment.
///
/// # Errors
/// As [`configure_siem`].
pub fn configure_siem_from_env() -> Result<Option<ConfiguredSiem>, SiemBootRefusal> {
    let var = |name: &str| std::env::var_os(name).map(|value| value.to_string_lossy().into_owned());
    let (endpoint, file) = (var("MAOS_SIEM_ENDPOINT"), var("MAOS_SIEM_FILE"));
    let (ca_file, client_cert_file) = (var("MAOS_SIEM_CA_FILE"), var("MAOS_SIEM_CLIENT_CERT_FILE"));
    let allow_private = var("MAOS_SIEM_ALLOW_PRIVATE")
        .is_some_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true"));
    configure_siem(
        &SiemEnv {
            endpoint: endpoint.as_deref(),
            file: file.as_deref(),
            ca_file: ca_file.as_deref(),
            client_cert_file: client_cert_file.as_deref(),
            client_key_in_env: var("MAOS_SIEM_CLIENT_KEY")
                .is_some_and(|value| !value.trim().is_empty()),
            allow_private,
        },
        Arc::new(SystemResolver),
    )
}

#[cfg(not(feature = "online-adapters"))]
fn network_siem(
    _env: &SiemEnv<'_>,
    _endpoint: &str,
    _resolver: Arc<dyn ResolveHost>,
) -> Result<ConfiguredSiem, SiemBootRefusal> {
    Err(SiemBootRefusal::UnsupportedBuild)
}

#[cfg(feature = "online-adapters")]
fn network_siem(
    env: &SiemEnv<'_>,
    endpoint: &str,
    resolver: Arc<dyn ResolveHost>,
) -> Result<ConfiguredSiem, SiemBootRefusal> {
    use maos_domain::outbound::OutboundRefusal;

    let (host, port) = parse_siem_endpoint(endpoint)?;
    let ca_path = env
        .ca_file
        .filter(|value| !value.is_empty())
        .ok_or(SiemBootRefusal::CaMissing)?;
    let mut policy = OutboundPolicy::strict();
    if env.allow_private {
        policy = policy.allow_private();
    }
    match policy.vet_host_port(&host, port, resolver.as_ref()) {
        Ok(_) | Err(OutboundRefusal::Unresolved) => {}
        Err(refusal) => return Err(SiemBootRefusal::EndpointRefused(refusal.code())),
    }
    let ca_pem = std::fs::read(ca_path).map_err(|_| SiemBootRefusal::CaInvalid)?;
    let client_cert = match env.client_cert_file.filter(|value| !value.is_empty()) {
        Some(path) => Some(std::fs::read(path).map_err(|_| SiemBootRefusal::ClientCertInvalid)?),
        None => None,
    };
    if client_cert.is_none() && env.client_key_in_env {
        return Err(SiemBootRefusal::ClientIncomplete);
    }
    let network = NetworkSiem::new(host, port, policy, ca_pem, client_cert, resolver)?;
    Ok(ConfiguredSiem {
        kind: SiemKind::Network(Arc::new(network)),
    })
}

/// `host:port`, an IPv6 literal in brackets, port default 6514 (RFC 5425 §4.1).
#[cfg(feature = "online-adapters")]
fn parse_siem_endpoint(raw: &str) -> Result<(String, u16), SiemBootRefusal> {
    const INVALID: SiemBootRefusal = SiemBootRefusal::EndpointRefused("endpoint_invalid");
    const DEFAULT_PORT: u16 = 6514;
    let raw = raw.trim();
    let (host, port_text) = if let Some(inner) = raw.strip_prefix('[') {
        let (host, rest) = inner.split_once(']').ok_or(INVALID)?;
        host.parse::<std::net::Ipv6Addr>().map_err(|_| INVALID)?;
        let port_text = if rest.is_empty() {
            ""
        } else {
            rest.strip_prefix(':').ok_or(INVALID)?
        };
        (host, port_text)
    } else {
        match raw.matches(':').count() {
            0 => (raw, ""),
            1 => raw.split_once(':').ok_or(INVALID)?,
            _ => return Err(INVALID),
        }
    };
    let host_chars_ok = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'));
    if !host_chars_ok {
        return Err(INVALID);
    }
    let port = if port_text.is_empty() {
        DEFAULT_PORT
    } else {
        port_text
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or(INVALID)?
    };
    Ok((host.to_owned(), port))
}

/// The syslog-over-TLS sink `MAOS_SIEM_ENDPOINT` selected (Story 23-4). Holds the CA bundle
/// and the client certificate (public); the client private key is settled once, after the
/// credential store exists, and lives only inside the TLS configuration (`Debug` shows
/// the endpoint only).
#[cfg(feature = "online-adapters")]
pub struct NetworkSiem {
    host: String,
    port: u16,
    policy: OutboundPolicy,
    resolver: Arc<dyn ResolveHost>,
    ca_pem: Vec<u8>,
    client_cert: Option<Vec<u8>>,
    tls: std::sync::OnceLock<maos_siem::tls_sink::ClientConfigHandle>,
}

#[cfg(feature = "online-adapters")]
impl std::fmt::Debug for NetworkSiem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkSiem")
            .field("host", &self.host)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "online-adapters")]
impl NetworkSiem {
    fn new(
        host: String,
        port: u16,
        policy: OutboundPolicy,
        ca_pem: Vec<u8>,
        client_cert: Option<Vec<u8>>,
        resolver: Arc<dyn ResolveHost>,
    ) -> Result<Self, SiemBootRefusal> {
        // The CA bundle is validated now, with or without a client certificate.
        let server_only = maos_siem::tls_sink::client_config(&ca_pem, None)
            .map_err(|_| SiemBootRefusal::CaInvalid)?;
        let tls = std::sync::OnceLock::new();
        if client_cert.is_none() {
            let _ = tls.set(server_only);
        }
        Ok(Self {
            host,
            port,
            policy,
            resolver,
            ca_pem,
            client_cert,
            tls,
        })
    }

    /// `host:port` for operator lines (never a credential).
    #[must_use]
    pub fn endpoint(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    #[must_use]
    pub fn needs_client_key(&self) -> bool {
        self.client_cert.is_some()
    }

    fn provision_client_key(&self, key: Option<&str>) -> Result<(), SiemBootRefusal> {
        let Some(cert) = &self.client_cert else {
            return Ok(());
        };
        let key = key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or(SiemBootRefusal::ClientKeyInvalid)?;
        let config = maos_siem::tls_sink::client_config(&self.ca_pem, Some((cert, key.as_bytes())))
            .map_err(|error| match error {
                maos_siem::tls_sink::SetupError::CaInvalid => SiemBootRefusal::CaInvalid,
                maos_siem::tls_sink::SetupError::ClientCertInvalid => {
                    SiemBootRefusal::ClientCertInvalid
                }
                maos_siem::tls_sink::SetupError::ClientKeyInvalid => {
                    SiemBootRefusal::ClientKeyInvalid
                }
            })?;
        let _ = self.tls.set(config);
        Ok(())
    }

    /// A sink for one tick; `None` until the client key is settled.
    fn sink(&self) -> Option<maos_siem::tls_sink::TlsSink> {
        Some(maos_siem::tls_sink::TlsSink::new(
            &self.host,
            self.port,
            self.tls.get()?.clone(),
            self.policy,
            Arc::clone(&self.resolver),
            maos_siem::tls_sink::Timeouts::default(),
        ))
    }
}

/// What one probe of the external KMS endpoint found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KmsProbeOutcome {
    /// `GET /health` failed or did not answer healthy.
    Unreachable,
    /// The endpoint answers, but no call can be sent yet: the bearer has not
    /// been provisioned from the secret store.
    ReachableBearerPending,
    /// The endpoint answers, but the bearer could not be read: every wrap and
    /// unwrap is refused until restart.
    ReachableBearerUnreadable,
    /// The endpoint answers, but the last wrap or unwrap was refused or failed.
    ReachableCallsFailing,
    /// The endpoint answers and calls are being sent and accepted.
    Ready,
}

impl KmsProbeOutcome {
    /// Whether `GET /health` answered.
    #[must_use]
    pub fn reachable(self) -> bool {
        !matches!(self, Self::Unreachable)
    }

    /// The log line for this outcome (never carries a value). `boot` is the
    /// first probe, which runs before the bearer is read from the secret store.
    #[must_use]
    pub fn line(self, boot: bool) -> &'static str {
        match (self, boot) {
            (Self::Unreachable, true) => {
                "maos: external KMS endpoint UNREACHABLE at boot; sealed writes are refused until a probe succeeds"
            }
            (Self::Unreachable, false) => {
                "maos: external KMS endpoint is now UNREACHABLE; sealed writes are refused until a probe succeeds"
            }
            (Self::ReachableBearerPending, true) => {
                "maos: external KMS endpoint reachable at boot; sealed writes stay refused until the bearer is read from the secret store"
            }
            (Self::ReachableBearerPending, false) => {
                "maos: external KMS endpoint is reachable, but the bearer is not provisioned; sealed writes stay refused"
            }
            (Self::ReachableBearerUnreadable, _) => {
                "maos: external KMS endpoint is reachable, but the bearer could not be read; every wrap and unwrap is refused until restart (check MAOS_KMS_BEARER and the secret store)"
            }
            (Self::ReachableCallsFailing, _) => {
                "maos: external KMS endpoint is reachable, but its wrap/unwrap calls are failing (see the earlier 'external KMS answered' line); sealed writes may be refused"
            }
            (Self::Ready, true) => "maos: external KMS endpoint reachable at boot",
            (Self::Ready, false) => {
                "maos: external KMS endpoint is now reachable; sealed writes resume"
            }
        }
    }
}

/// A probe of the external KMS endpoint.
pub type KmsProbeFn = Arc<dyn Fn() -> KmsProbeOutcome + Send + Sync>;

/// How often the composition root probes an external KMS endpoint (D-23-8-03 A).
pub const KMS_PROBE_INTERVAL: Duration = Duration::from_secs(10);

/// Why the KMS configuration is refused at boot (exit 78, Story 23-8). Each
/// variant names the variables involved and never a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KmsBootRefusal {
    /// Both `MAOS_KMS_ENDPOINT` and `MAOS_KMS_MASTER_KEY` are set (D-23-8-04 A).
    Conflict,
    /// `MAOS_KMS_ENDPOINT` is set in a build without `online-adapters` (D-23-8-09 A).
    UnsupportedBuild,
    /// The endpoint failed the outbound policy or is not a plain base URL.
    EndpointRefused(&'static str),
    /// `MAOS_KMS_MASTER_KEY` is not 64 hex characters, or is all zero.
    MasterKeyInvalid,
    /// Some `MAOS_KMS_*` variable is set but no key source is (D-23-8-07 A).
    Incomplete,
}

impl KmsBootRefusal {
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Conflict => "kms_config_conflict",
            Self::UnsupportedBuild => "kms_endpoint_unsupported_build",
            Self::EndpointRefused(_) => "kms_endpoint_refused",
            Self::MasterKeyInvalid => "kms_master_key_invalid",
            Self::Incomplete => "kms_config_incomplete",
        }
    }
}

impl std::fmt::Display for KmsBootRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let code = self.code();
        match self {
            Self::Conflict => write!(
                f,
                "{code}: MAOS_KMS_ENDPOINT and MAOS_KMS_MASTER_KEY are both set; set exactly one"
            ),
            Self::UnsupportedBuild => write!(
                f,
                "{code}: MAOS_KMS_ENDPOINT is set but this build has no online adapters"
            ),
            Self::EndpointRefused(detail) => {
                write!(f, "{code}: MAOS_KMS_ENDPOINT refused ({detail})")
            }
            Self::MasterKeyInvalid => write!(
                f,
                "{code}: MAOS_KMS_MASTER_KEY must be 64 hex characters (32 bytes) and not all zero"
            ),
            Self::Incomplete => write!(
                f,
                "{code}: a MAOS_KMS_* variable is set but neither MAOS_KMS_ENDPOINT nor \
                 MAOS_KMS_MASTER_KEY is"
            ),
        }
    }
}

impl std::error::Error for KmsBootRefusal {}

/// The KMS the composition root selected (Story 23-8, D-23-8-08 A): built once
/// at boot and handed to both the enterprise runtime and the secret store, so
/// there is one adapter and one health state.
pub struct ConfiguredKms {
    port: Arc<dyn KeyManagementPort>,
    probe: Option<KmsProbeFn>,
    #[cfg(feature = "online-adapters")]
    http: Option<Arc<maos_secrets::http_kms::HttpKms>>,
}

impl std::fmt::Debug for ConfiguredKms {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfiguredKms")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl ConfiguredKms {
    /// The port every seal and open goes through.
    #[must_use]
    pub fn port(&self) -> Arc<dyn KeyManagementPort> {
        Arc::clone(&self.port)
    }

    /// `Some` for an external endpoint: one `GET /health`.
    #[must_use]
    pub fn probe(&self) -> Option<KmsProbeFn> {
        self.probe.clone()
    }

    /// `"external endpoint"` or `"local master key"`.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        if self.probe.is_some() {
            "external endpoint"
        } else {
            "local master key"
        }
    }

    /// Settle the endpoint bearer (`None` = no bearer). The adapter refuses
    /// every call until this runs, so no request leaves without the operator's
    /// bearer decision. A local master key has no bearer: no-op.
    pub fn provision_bearer(&self, bearer: Option<String>) {
        #[cfg(feature = "online-adapters")]
        if let Some(http) = &self.http {
            http.provision_bearer(bearer);
        }
        #[cfg(not(feature = "online-adapters"))]
        let _ = bearer;
    }

    /// Record that the bearer could not be read: the adapter keeps refusing
    /// every call and the probe line says so. A local master key: no-op.
    pub fn bearer_unreadable(&self) {
        #[cfg(feature = "online-adapters")]
        if let Some(http) = &self.http {
            http.bearer_unreadable();
        }
    }
}

/// The KMS-related environment, read once (so tests drive selection without
/// mutating the process environment).
#[derive(Debug, Clone, Copy, Default)]
pub struct KmsEnv<'a> {
    pub endpoint: Option<&'a str>,
    pub master_key: Option<&'a str>,
    /// `MAOS_KMS_ALLOW_PRIVATE`: admit RFC 1918 / fc00::/7 endpoints.
    pub allow_private: bool,
    /// Whether any `MAOS_KMS_*` variable is set.
    pub any_kms_variable: bool,
}

/// Name resolution for the boot-time endpoint check, bounded by a 2 s deadline.
struct SystemResolver;

impl ResolveHost for SystemResolver {
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>, ResolveFailed> {
        let (sender, receiver) = std::sync::mpsc::channel();
        let host = host.to_owned();
        std::thread::spawn(move || {
            let answers = (host.as_str(), port)
                .to_socket_addrs()
                .map(|addresses| addresses.map(|address| address.ip()).collect::<Vec<_>>());
            let _ = sender.send(answers);
        });
        match receiver.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(addresses)) => Ok(addresses),
            _ => Err(ResolveFailed),
        }
    }
}

/// Select the KMS (D-23-8-04 A, D-23-8-07 A). `Ok(None)` only when no
/// `MAOS_KMS_*` variable is set (the byte-identical Option-A plaintext
/// default). An invalid configuration is a refusal, never a silent downgrade:
/// this is the Story 23-8 fix for the serve path that used to boot with an
/// unusable local key and write plaintext Collective rows. An endpoint that is
/// merely unreachable is not a refusal; its health recovers by probe.
///
/// The endpoint is operator configuration, never a value read from a peer. The
/// outbound policy vets it here, once; for a host NAME that does not bind the
/// connection to the vetted addresses (RELEASE-HOLDS 23-0.a), so this makes no
/// SSRF claim for names.
///
/// # Errors
/// The [`KmsBootRefusal`] of the first check that fails.
pub fn configure_kms(
    env: &KmsEnv<'_>,
    resolver: &dyn ResolveHost,
) -> Result<Option<ConfiguredKms>, KmsBootRefusal> {
    match (env.endpoint, env.master_key) {
        (Some(_), Some(_)) => Err(KmsBootRefusal::Conflict),
        (Some(endpoint), None) => external_kms(endpoint, env.allow_private, resolver).map(Some),
        (None, Some(key)) => local_kms(key).map(Some),
        (None, None) if env.any_kms_variable => Err(KmsBootRefusal::Incomplete),
        (None, None) => Ok(None),
    }
}

/// [`configure_kms`] over the process environment.
///
/// # Errors
/// As [`configure_kms`].
pub fn configure_kms_from_env() -> Result<Option<ConfiguredKms>, KmsBootRefusal> {
    let var = |name: &str| std::env::var_os(name).map(|value| value.to_string_lossy().into_owned());
    let endpoint = var("MAOS_KMS_ENDPOINT");
    let master_key = var("MAOS_KMS_MASTER_KEY");
    let allow_private = var("MAOS_KMS_ALLOW_PRIVATE")
        .is_some_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true"));
    configure_kms(
        &KmsEnv {
            endpoint: endpoint.as_deref(),
            master_key: master_key.as_deref(),
            allow_private,
            any_kms_variable: any_env_with_prefix("MAOS_KMS_"),
        },
        &SystemResolver,
    )
}

fn local_kms(raw: &str) -> Result<ConfiguredKms, KmsBootRefusal> {
    let bytes = hex::decode(raw.trim()).map_err(|_| KmsBootRefusal::MasterKeyInvalid)?;
    let kms =
        LocalMasterKeyKms::from_master_key(&bytes).map_err(|_| KmsBootRefusal::MasterKeyInvalid)?;
    if !kms.is_healthy() {
        return Err(KmsBootRefusal::MasterKeyInvalid);
    }
    Ok(ConfiguredKms {
        port: Arc::new(kms),
        probe: None,
        #[cfg(feature = "online-adapters")]
        http: None,
    })
}

#[cfg(feature = "online-adapters")]
fn external_kms(
    endpoint: &str,
    allow_private: bool,
    resolver: &dyn ResolveHost,
) -> Result<ConfiguredKms, KmsBootRefusal> {
    use maos_secrets::http_kms::{HttpKms, MonotonicClock, TimedHttp};

    // `http` only to a literal loopback address; any other host needs `https`.
    // Private ranges need the operator's explicit `MAOS_KMS_ALLOW_PRIVATE`.
    let mut policy = OutboundPolicy::strict().allow_loopback_http();
    if allow_private {
        policy = policy.allow_private();
    }
    let vetted = policy
        .vet_url(endpoint.trim(), resolver)
        .map_err(|refusal| KmsBootRefusal::EndpointRefused(refusal.code()))?;
    let kms = HttpKms::pending_bearer(
        Arc::new(TimedHttp::default()),
        vetted.as_str(),
        Arc::new(MonotonicClock::new()),
    )
    .map_err(|error| match error {
        KmsError::MalformedKey(detail) => KmsBootRefusal::EndpointRefused(detail),
        _ => KmsBootRefusal::EndpointRefused("endpoint unusable"),
    })?;
    let kms = Arc::new(kms);
    let probe_target = Arc::clone(&kms);
    Ok(ConfiguredKms {
        port: Arc::clone(&kms) as Arc<dyn KeyManagementPort>,
        probe: Some(Arc::new(move || {
            probe_outcome(
                probe_target.probe_checked(),
                probe_target.bearer_status(),
                probe_target.is_healthy(),
                probe_target.calls_failing(),
            )
        })),
        http: Some(kms),
    })
}

/// The probe outcome from what the probe did to the *applied* health state:
/// a healthy answer that was discarded (a call failed while it was in flight)
/// or that left the adapter unhealthy is never `Ready`.
#[cfg(feature = "online-adapters")]
#[must_use]
pub fn probe_outcome(
    result: maos_secrets::http_kms::ProbeResult,
    bearer: maos_secrets::http_kms::BearerStatus,
    healthy: bool,
    calls_failing: bool,
) -> KmsProbeOutcome {
    use maos_secrets::http_kms::{BearerStatus, ProbeResult};
    match (result, bearer) {
        (ProbeResult::Unreachable, _) => KmsProbeOutcome::Unreachable,
        (ProbeResult::Discarded, _) => KmsProbeOutcome::ReachableCallsFailing,
        (ProbeResult::Applied, BearerStatus::Pending) => KmsProbeOutcome::ReachableBearerPending,
        (ProbeResult::Applied, BearerStatus::Unreadable) => {
            KmsProbeOutcome::ReachableBearerUnreadable
        }
        (ProbeResult::Applied, BearerStatus::Settled) if calls_failing || !healthy => {
            KmsProbeOutcome::ReachableCallsFailing
        }
        (ProbeResult::Applied, BearerStatus::Settled) => KmsProbeOutcome::Ready,
    }
}

#[cfg(not(feature = "online-adapters"))]
fn external_kms(
    _endpoint: &str,
    _allow_private: bool,
    _resolver: &dyn ResolveHost,
) -> Result<ConfiguredKms, KmsBootRefusal> {
    Err(KmsBootRefusal::UnsupportedBuild)
}

/// Run `probe` on a blocking thread every `interval` until `cancel` fires
/// (Story 23-8 AC5). The boot path has already probed once, so the first tick
/// is skipped. A change of answer is logged; nothing else is.
pub fn spawn_kms_probe(
    probe: KmsProbeFn,
    interval: Duration,
    cancel: CancellationToken,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await;
        let mut last: Option<KmsProbeOutcome> = None;
        loop {
            tokio::select! {
                _ = cancel.cancelled() => break,
                _ = ticker.tick() => {
                    let probe = Arc::clone(&probe);
                    let outcome = tokio::task::spawn_blocking(move || probe())
                        .await
                        .unwrap_or(KmsProbeOutcome::Unreachable);
                    if last != Some(outcome) {
                        last = Some(outcome);
                        eprintln!("{}", outcome.line(false));
                    }
                }
            }
        }
    })
}

/// Build the static-JWKS OIDC verifier from `MAOS_SSO_JWKS` /
/// `MAOS_SSO_ISSUERS` (comma-separated) / `MAOS_SSO_AUDIENCE` /
/// `MAOS_SSO_ALGS` (optional, default RS256+ES256).
fn build_sso_verifier() -> Result<OidcVerifier, String> {
    let jwks = std::env::var("MAOS_SSO_JWKS").map_err(|_| "MAOS_SSO_JWKS absent")?;
    let issuers_raw = std::env::var("MAOS_SSO_ISSUERS").map_err(|_| "MAOS_SSO_ISSUERS absent")?;
    let audience = std::env::var("MAOS_SSO_AUDIENCE").map_err(|_| "MAOS_SSO_AUDIENCE absent")?;
    let algs_raw = std::env::var("MAOS_SSO_ALGS").unwrap_or_else(|_| "RS256,ES256".to_string());
    let algs: Vec<OidcAlgorithm> = algs_raw
        .split(',')
        .filter_map(|s| match s.trim() {
            "RS256" => Some(OidcAlgorithm::Rs256),
            "ES256" => Some(OidcAlgorithm::Es256),
            _ => None,
        })
        .collect();
    if algs.is_empty() {
        return Err("MAOS_SSO_ALGS resolved to no supported algorithms".to_string());
    }
    let issuers: Vec<&str> = issuers_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    OidcVerifier::from_static_jwks(&jwks, &algs, &issuers, &audience).map_err(|e| e.to_string())
}

fn now_ns() -> Result<u64, std::time::SystemTimeError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos().min(u128::from(u64::MAX)) as u64)
}

/// Append ONE out-of-kernel `identity.asserted` (kind = 30) row to the
/// Transparency Log (Story 11.4c AC2).
///
/// This write helper lives in `maos-bin` — the composition root / writer side —
/// NOT in `maos-audit`, which is read-only by design (Story 9.2 Decision A.2,
/// enforced by the `maos-audit-read-only` gate). Every OTHER Transparency-Log
/// row is written by the kernel's `TransparencyLogAdapter::insert_frame_event`,
/// which takes the kernel-core `FrameKind` enum; `IdentityAsserted` is
/// deliberately NOT a kernel-core `FrameKind` (adding it would be an L1
/// kernel-core delta — FORBIDDEN), so the `maos-sso` verified-principal flow
/// persists this provenance row directly from the composition root.
///
/// # Row shape
///
/// - `frame_id`         — 16 cryptographically-random bytes (`getrandom`); a PK
///                        collision (vanishing at 128 bits) surfaces as
///                        `AuditError::Query`, NEVER swallowed.
/// - `timestamp_ns`     — `decision_time_ns` (the IdP's assertion instant).
/// - `spirit_pid` /
///   `boot_nonce`       — the kernel context the assertion is bound to.
/// - `capability_token` — `NULL` (no capability mediates an identity assertion).
/// - `kind`             — `30` (renders as `"identity.asserted"` on read).
/// - `intent`           — `"identity.asserted"`.
/// - `payload_redacted` — compact JSON
///   `{"subject","issuer","capability_key","decision_time_ns"}`.
/// - `origin`           — `FrameOrigin::HumanAuthored = 0`: a NON-kernel origin.
///                        Reuses the domain enum so the discriminator cannot
///                        drift from `i3`.
///
/// Fail-closed: any SQLite, serialization, or entropy error returns
/// `AuditError` — no silent `INSERT OR IGNORE`. The `transparency_log` table is
/// created by the kernel; this helper does NOT create it and returns an error
/// if it is absent (single-sourced schema, no drift).
pub fn append_identity_asserted(
    db_path: &Path,
    spirit_pid: u32,
    boot_nonce: u64,
    subject: &str,
    issuer: &str,
    capability_key: &str,
    decision_time_ns: u64,
) -> Result<(), AuditError> {
    let payload = serde_json::json!({
        "subject": subject,
        "issuer": issuer,
        "capability_key": capability_key,
        "decision_time_ns": decision_time_ns,
    });
    append_raw_row(
        db_path,
        spirit_pid,
        boot_nonce,
        decision_time_ns,
        30, // kind — identity.asserted
        "identity.asserted",
        &serde_json::to_vec(&payload)?,
        None,
    )
}

/// Journal one `siem.gap` or `siem.cursor.reset` row (Story 23-4, D-23-4-06 A / -08 A) into
/// the LIVE Transparency Log: a raw kind-4 `telemetry.event` row (no kernel `FrameKind`
/// variant, no payload beyond the ids or reason code), written like `identity.asserted`.
///
/// # Errors
/// Any SQLite, serialization or clock error; the exporter then moves nothing and retries.
pub fn append_siem_event(
    db_path: &Path,
    boot_nonce: u64,
    intent: &str,
    payload: &serde_json::Value,
) -> Result<(), AuditError> {
    let now = now_ns().map_err(|error| AuditError::Io(std::io::Error::other(error)))?;
    append_raw_row(
        db_path,
        0,
        boot_nonce,
        now,
        4, // kind — telemetry.event
        intent,
        &serde_json::to_vec(payload)?,
        Some(Duration::from_secs(5)),
    )
}

#[allow(clippy::too_many_arguments)]
fn append_raw_row(
    db_path: &Path,
    spirit_pid: u32,
    boot_nonce: u64,
    timestamp_ns: u64,
    kind: i64,
    intent: &str,
    payload: &[u8],
    busy_wait: Option<Duration>,
) -> Result<(), AuditError> {
    let mut frame_id = [0u8; 16];
    getrandom::fill(&mut frame_id)
        .map_err(|e| AuditError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let spirit_pid_i = i64::try_from(spirit_pid).map_err(|_| AuditError::ValueOverflow {
        field: "spirit_pid",
        value: spirit_pid.into(),
    })?;
    let boot_nonce_i = i64::try_from(boot_nonce).map_err(|_| AuditError::ValueOverflow {
        field: "boot_nonce",
        value: boot_nonce,
    })?;
    let timestamp_ns_i = i64::try_from(timestamp_ns).map_err(|_| AuditError::ValueOverflow {
        field: "timestamp_ns",
        value: timestamp_ns,
    })?;

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AuditError::Open)?;
    // The SIEM journal waits for the kernel adapter's lock rather than lose a gap/reset row
    // to SQLITE_BUSY; `identity.asserted` keeps its original no-wait behaviour.
    if let Some(wait) = busy_wait {
        conn.busy_timeout(wait).map_err(AuditError::Open)?;
    }

    // Plain INSERT (NOT `INSERT OR IGNORE`): a frame_id PK collision surfaces as
    // SQLITE_CONSTRAINT_PRIMARYKEY → AuditError::Query rather than being silently
    // swallowed (Story 11.4c "fail closed" requirement).
    // The SIEM journal (`busy_wait` set) is idempotent: its payload is deterministic (sink
    // hash, range or bad-cursor fingerprint), so a row with the same intent and payload is
    // already the record of this transition and a retry after a cursor I/O failure, or a
    // crash between this write and the cursor write, adds nothing.
    if busy_wait.is_some() {
        let present: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transparency_log WHERE intent = ?1 \
                 AND payload_redacted = ?2)",
                rusqlite::params![intent, payload],
                |row| row.get(0),
            )
            .map_err(AuditError::Read)?;
        if present {
            return Ok(());
        }
    }
    conn.execute(
        "INSERT INTO transparency_log \
         (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, \
          kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &frame_id[..],                                                  // frame_id
            timestamp_ns_i,                                                 // timestamp_ns
            spirit_pid_i,                                                   // spirit_pid
            boot_nonce_i,                                                   // boot_nonce
            Option::<&[u8]>::None, // capability_token = NULL
            kind,                  // kind
            intent,                // intent
            payload,               // payload_redacted (compact JSON)
            maos_domain::invariants::i3::FrameOrigin::HumanAuthored as i64, // origin = 0 (non-kernel)
        ],
    )
    .map_err(AuditError::Query)?;

    Ok(())
}

#[cfg(test)]
mod available_arm_tests {
    //! Prove the `Available` arms route through the REAL adapters (not the stub).
    //! These are the falsifiers for the composition-root integration: a stub
    //! returns Ok-without-verifying / plaintext / Ok-without-forwarding; the
    //! real arms deny a forged assertion, emit ciphertext, and forward records.
    //!
    //! The offline RS256 OIDC fixtures are pulled byte-exact from
    //! `crates/maos-sso/tests/fixtures.rs` via `include_str!` + a tiny extractor,
    //! so this test can never drift from the trusted token material.
    use super::*;
    use maos_audit::{query, AuditEntry};
    use maos_kernel_core::security::RingCryptoProvider;
    use maos_secrets::LocalMasterKeyKms;
    use maos_siem::SiemExporter;
    use maos_sso::{OidcAlgorithm, OidcVerifier};
    use rusqlite::Connection;
    use tempfile::TempDir;

    const SSO_FIXTURES_SRC: &str = include_str!("../../maos-sso/tests/fixtures.rs");
    const ISS_GOOD: &str = "https://idp.maos.example";
    const AUD_EXPECTED: &str = "maos-deploy-alpha";

    /// Extract `pub const <NAME>: &str = "<value>";` byte-exact from the
    /// maos-sso fixture source (no hand-transcription of long base64 tokens).
    fn fixture(const_name: &str) -> String {
        let needle = format!("pub const {const_name}: &str = \"");
        let start = SSO_FIXTURES_SRC
            .find(&needle)
            .unwrap_or_else(|| panic!("fixture const {const_name} not found"))
            + needle.len();
        // Scan the Rust string literal, unescaping `\"` → `"` and `\\` → `\`,
        // stopping at the closing unescaped quote. (The JWKS constant embeds
        // escaped quotes; the base64 token constants have none.)
        let bytes = SSO_FIXTURES_SRC.as_bytes();
        let mut i = start;
        let mut out = String::new();
        while i < bytes.len() {
            match bytes[i] {
                b'\\' if i + 1 < bytes.len() => {
                    out.push(bytes[i + 1] as char);
                    i += 2;
                }
                b'"' => break,
                c => {
                    out.push(c as char);
                    i += 1;
                }
            }
        }
        out
    }

    const SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS transparency_log (
        frame_id BLOB NOT NULL PRIMARY KEY,
        timestamp_ns INTEGER NOT NULL,
        spirit_pid INTEGER NOT NULL,
        boot_nonce INTEGER NOT NULL,
        capability_token BLOB,
        kind INTEGER NOT NULL,
        intent TEXT NOT NULL,
        payload_redacted BLOB NOT NULL,
        origin INTEGER NOT NULL
    );";

    fn temp_tl() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("tl.sqlite");
        let conn = Connection::open(&path).expect("open");
        conn.execute_batch(SCHEMA_SQL).expect("schema");
        (dir, path)
    }

    fn verifier() -> Arc<dyn IdentityAssertionPort> {
        let jwks = fixture("JWKS_KEY_A");
        Arc::new(
            OidcVerifier::from_static_jwks(
                &jwks,
                &[OidcAlgorithm::Rs256],
                &[ISS_GOOD],
                AUD_EXPECTED,
            )
            .expect("verifier from fixture JWKS"),
        )
    }

    #[allow(clippy::type_complexity)]
    fn build_runtime(
        sso: SubsystemState,
        kms: SubsystemState,
        siem: SubsystemState,
        identity_port: Option<Arc<dyn IdentityAssertionPort>>,
        kms_port: Option<Arc<dyn KeyManagementPort>>,
        siem_port: Option<Arc<dyn SiemProjectionPort>>,
        crypto: Option<Arc<dyn CryptoProvider>>,
        audit_db_path: Option<PathBuf>,
        siem_sink_path: Option<PathBuf>,
    ) -> EnterpriseRuntime {
        EnterpriseRuntime {
            config: EnterpriseConfig { sso, kms, siem },
            identity_port,
            kms_port,
            siem_port,
            crypto,
            audit_db_path,
            siem_sink_path,
            #[cfg(feature = "online-adapters")]
            siem_network: None,
            siem_tick: std::sync::Mutex::new(()),
            siem_cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            siem_delivery: tokio::sync::watch::channel(0).0,
            boot_nonce: 7,
            kms_probe: None,
        }
    }

    #[cfg(not(feature = "kms-fault-inject"))]
    #[test]
    fn kms_available_seals_to_real_ciphertext() {
        let kms = LocalMasterKeyKms::from_master_key(&[0x42u8; 32]).expect("master key");
        let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
        let rt = build_runtime(
            SubsystemState::Disabled,
            SubsystemState::Available,
            SubsystemState::Disabled,
            None,
            Some(Arc::new(kms)),
            None,
            Some(crypto),
            None,
            None,
        );
        let plaintext: &[u8] = b"collective-row-option-a-plaintext";
        let sealed = rt
            .seal_row_at_rest(plaintext)
            .expect("Available + healthy KMS MUST seal (real AEAD)");
        assert_ne!(
            sealed.as_slice(),
            plaintext,
            "sealed output MUST differ from plaintext (real AEAD, not the stub passthrough)"
        );
        assert!(
            sealed.len() > plaintext.len(),
            "sealed output MUST carry AEAD overhead (wrapped DEK + nonce + tag)"
        );
    }

    #[cfg(not(feature = "sso-fault-inject"))]
    #[test]
    fn sso_available_denies_forged_assertions() {
        let rt = build_runtime(
            SubsystemState::Available,
            SubsystemState::Disabled,
            SubsystemState::Disabled,
            Some(verifier()),
            None,
            None,
            None,
            None,
            None,
        );
        // A stub would return Ok(()) for ANY assertion. The real Available arm
        // invokes OidcVerifier::verify and DENIES wrong-key / expired tokens.
        for forged in [fixture("TOKEN_WRONG_KEY"), fixture("TOKEN_EXPIRED")] {
            match rt.issue_under_principal(42, &forged, "cap-test") {
                Err(EnterpriseFailure::SsoIssuanceDenied { .. }) => {}
                other => panic!(
                    "Available SSO MUST deny a forged/expired assertion via real verify; got {other:?}"
                ),
            }
        }
    }

    #[test]
    fn sso_available_persists_identity_asserted_on_valid_assertion() {
        let (_dir, tl) = temp_tl();
        let rt = build_runtime(
            SubsystemState::Available,
            SubsystemState::Disabled,
            SubsystemState::Disabled,
            Some(verifier()),
            None,
            None,
            None,
            Some(tl.clone()),
            None,
        );
        let good = fixture("TOKEN_GOOD_RS256");
        rt.issue_under_principal(42, &good, "cap-test")
            .expect("a validly-signed, in-claim-window assertion MUST verify + issue");
        let rows = query(&tl, AuditFilter::default()).expect("read TL");
        let asserted: Vec<&AuditEntry> = rows
            .iter()
            .filter(|r| r.kind == "identity.asserted")
            .collect();
        assert_eq!(
            asserted.len(),
            1,
            "exactly one out-of-kernel identity.asserted row MUST be persisted on a verified issuance"
        );
        assert_eq!(asserted[0].spirit_pid, 42);
    }

    #[test]
    fn identity_asserted_does_not_mint_an_unvalidated_audit_shard() {
        let dir = TempDir::new().expect("dir");
        let unvalidated = dir.path().join("teams/security/transparency.sqlite");
        std::fs::create_dir_all(unvalidated.parent().unwrap()).expect("team directory");

        assert!(append_identity_asserted(
            &unvalidated,
            7,
            11,
            "subject",
            "issuer",
            "capability",
            1,
        )
        .is_err());
        assert!(
            !unvalidated.exists(),
            "the raw kind-30 writer must not create an unvalidated team shard"
        );
    }

    #[test]
    fn siem_available_forwards_redacted_records_to_file() {
        let (_dir, tl) = temp_tl();
        append_identity_asserted(&tl, 7, 0, "subj", "iss", "cap", 1)
            .expect("seed identity.asserted row");
        let sink_dir = TempDir::new().expect("sink dir");
        let sink = sink_dir.path().join("siem.log");
        let rt = build_runtime(
            SubsystemState::Disabled,
            SubsystemState::Disabled,
            SubsystemState::Available,
            None,
            None,
            Some(Arc::new(SiemExporter)),
            None,
            Some(tl.clone()),
            Some(sink.clone()),
        );
        let tick = rt
            .forward_audit_to_siem()
            .expect("Available SIEM MUST forward to the file sink");
        assert!(
            tick.delivered >= 1,
            "forwarded count MUST reflect the seeded record"
        );
        // The cursor is durable: a second tick re-sends nothing.
        assert_eq!(
            rt.forward_audit_to_siem().expect("second tick").delivered,
            0
        );
        let contents = std::fs::read_to_string(&sink).expect("sink readable");
        assert!(
            !contents.is_empty(),
            "the localhost file sink MUST contain the forwarded frame(s)"
        );
    }

    #[test]
    fn siem_sink_down_buffers_and_reports_a_real_count() {
        let dir = TempDir::new().expect("dir");
        let tl = dir.path().join("tl.sqlite");
        let conn = Connection::open(&tl).expect("open");
        conn.execute_batch(SCHEMA_SQL).expect("schema");
        append_identity_asserted(&tl, 1, 0, "a", "b", "c", 1).expect("seed row 1");
        append_identity_asserted(&tl, 2, 0, "d", "e", "f", 2).expect("seed row 2");
        // A directory is not openable as an append-file → forward_to_file I/O error.
        let unwritable_sink = dir.path().join("unwritable-dir-sink");
        std::fs::create_dir_all(&unwritable_sink).expect("dir sink");
        let rt = build_runtime(
            SubsystemState::Disabled,
            SubsystemState::Disabled,
            SubsystemState::Available,
            None,
            None,
            Some(Arc::new(SiemExporter)),
            None,
            Some(tl.clone()),
            Some(unwritable_sink),
        );
        match rt.forward_audit_to_siem() {
            Err(EnterpriseFailure::SiemSinkDown {
                buffered, reason, ..
            }) => {
                assert_eq!(reason, "siem_sink_file_io");
                assert_eq!(
                    buffered, 2,
                    "buffered MUST equal the 2 records the TL holds matching the filter (never silently dropped)"
                );
            }
            other => panic!(
                "sink-down MUST surface SiemSinkDown with the real buffered count; got {other:?}"
            ),
        }
    }
}
