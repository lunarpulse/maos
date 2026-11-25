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

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use maos_audit::AuditError;
pub use maos_audit::AuditFilter;
#[cfg(feature = "online-adapters")]
use maos_domain::outbound::OutboundPolicy;
use maos_domain::outbound::ResolveHost;
use maos_domain::ports::identity_assertion::KeyFreshness;
#[cfg(feature = "online-adapters")]
use maos_domain::ports::KmsError;
use maos_domain::ports::{
    AuthenticatedPrincipal, CryptoProvider, IdentityAssertionPort, KeyManagementPort,
    SiemProjectionPort,
};
use maos_loom_lite::seal::AtRestSeal;
use maos_secrets::{seal_at_rest_opt, LocalMasterKeyKms};
use maos_siem::{export_report_from_tl, forward_to_file_with_port, SiemExporter};
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
    boot_nonce: u64,
    /// Probe of the external KMS endpoint (`None` for a local master key).
    kms_probe: Option<KmsProbeFn>,
    /// Why SSO was demoted to `ConfiguredDown` at build time (Story 23-2), so
    /// a denial can say which rule refused the configuration.
    sso_down_reason: Option<SsoDownReason>,
    /// The typed reason discovery last failed, named in an issuance denial while the verifier
    /// is unhealthy (Story 23-2 round 2, C8).
    sso_diagnostic: Option<SsoDiagnostic>,
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
            boot_nonce: 0,
            kms_probe: None,
            sso_down_reason: None,
            sso_diagnostic: None,
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
    ) -> Result<Self, EnterpriseFailure> {
        #[cfg(feature = "online-adapters")]
        let discovery = DiscoveryWiring::production;
        #[cfg(not(feature = "online-adapters"))]
        let discovery = || -> Result<DiscoveryWiring, SsoDownReason> {
            Err(SsoDownReason::DiscoveryUnsupportedBuild)
        };
        Self::from_env_with(
            &SsoEnv::from_process(),
            &discovery,
            crypto,
            audit_db_path,
            boot_nonce,
            kms,
        )
    }

    /// [`Self::from_env`] with the `MAOS_SSO_*` values and the discovery
    /// wiring injected (Story 23-2), so tests drive the real constructor
    /// without mutating the process environment. `discovery` is called only
    /// when a discovery issuer is configured.
    pub fn from_env_with(
        sso_env: &SsoEnv,
        discovery: &dyn Fn() -> Result<DiscoveryWiring, SsoDownReason>,
        crypto: Arc<dyn CryptoProvider>,
        audit_db_path: PathBuf,
        boot_nonce: u64,
        kms: Option<&ConfiguredKms>,
    ) -> Result<Self, EnterpriseFailure> {
        let mut config = EnterpriseConfig::from_env();
        if sso_env.any() {
            config = config.with_sso_available();
        }
        let mut identity_port: Option<Arc<dyn IdentityAssertionPort>> = None;
        let mut sso_down_reason: Option<SsoDownReason> = None;
        let mut sso_diagnostic: Option<SsoDiagnostic> = None;
        let mut kms_port: Option<Arc<dyn KeyManagementPort>> = None;
        let mut siem_port: Option<Arc<dyn SiemProjectionPort>> = None;
        let mut siem_sink_path: Option<PathBuf> = None;

        if config.sso_configured() {
            match build_sso_wired(sso_env, discovery) {
                Ok((port, diagnostic)) => {
                    identity_port = Some(port);
                    sso_diagnostic = diagnostic;
                }
                Err(reason) => {
                    eprintln!("maos: SSO is configured but down: {reason}");
                    sso_down_reason = Some(reason);
                    config = config.with_sso_down();
                }
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
            match std::env::var("MAOS_SIEM_FILE") {
                Ok(path) if !path.is_empty() => {
                    siem_sink_path = Some(PathBuf::from(path));
                    siem_port = Some(Arc::new(SiemExporter));
                }
                _ => config = config.with_siem_down(),
            }
        }

        let mut runtime = Self::from_ports(
            config,
            identity_port,
            kms_port,
            siem_port,
            Some(crypto),
            Some(audit_db_path),
            siem_sink_path,
            boot_nonce,
        );
        runtime.sso_down_reason = sso_down_reason;
        runtime.sso_diagnostic = sso_diagnostic;
        Ok(match kms.and_then(ConfiguredKms::probe) {
            Some(probe) => runtime.with_kms_probe(probe),
            None => runtime,
        })
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
            boot_nonce,
            kms_probe: None,
            sso_down_reason: None,
            sso_diagnostic: None,
        }
    }

    /// Why SSO is `ConfiguredDown` when the build refused its configuration.
    #[must_use]
    pub fn sso_down_reason(&self) -> Option<&SsoDownReason> {
        self.sso_down_reason.as_ref()
    }

    /// Record why SSO is down (a posture constructor that refused SSO).
    #[must_use]
    pub fn with_sso_down_reason(mut self, reason: SsoDownReason) -> Self {
        self.sso_down_reason = Some(reason);
        self
    }

    /// Story 23-2 (D-23-2-04 A) — verify an assertion a door request carries, at accept. `Ok`
    /// only when SSO is configured, healthy, and the assertion verifies; a Host without SSO
    /// refuses it (nothing could ever use it). Carries no reason: the door never echoes one.
    ///
    /// # Errors
    /// [`crate::directed_task::AssertionRefused`] for every refusal.
    pub fn check_carried_assertion(
        &self,
        assertion: &str,
    ) -> Result<(), crate::directed_task::AssertionRefused> {
        match self.verify_principal_for_issuance(0, assertion) {
            Ok(Some(_)) => Ok(()),
            _ => Err(crate::directed_task::AssertionRefused),
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
        self.verify_principal(spirit_pid, assertion, true)
            .map(|verified| verified.map(|(principal, _)| principal))
    }

    /// Story 23-2 — [`Self::verify_principal_for_issuance`] for a caller inside a critical
    /// section: the same verdicts, answered from the keys already held and NEVER waiting on
    /// discovery. An unknown `kid` or a stale key set is a denial (the same generic text) and
    /// fires the verifier's non-blocking refresh nudge, so a retry can succeed.
    pub fn verify_principal_for_issuance_no_wait(
        &self,
        spirit_pid: u32,
        assertion: &str,
    ) -> Result<Option<AuthenticatedPrincipal>, EnterpriseFailure> {
        self.verify_principal(spirit_pid, assertion, false)
            .map(|verified| verified.map(|(principal, _)| principal))
    }

    /// Story 23-2 (round 4) — [`Self::verify_principal_for_issuance`] (`wait`) or its no-wait
    /// form, and the [`KeyFreshness`] the verdict rests on (`None` for a static key set, which
    /// never expires or rotates). The caller hands the proof to [`Self::ensure_key_fresh`]
    /// right before it mints.
    pub fn verify_principal_with_freshness(
        &self,
        spirit_pid: u32,
        assertion: &str,
        wait: bool,
    ) -> Result<Option<(AuthenticatedPrincipal, Option<KeyFreshness>)>, EnterpriseFailure> {
        self.verify_principal(spirit_pid, assertion, wait)
    }

    /// Story 23-2 (round 4) — the issuance-point recheck: the key a verification used must still
    /// be published and unexpired. Never waits and holds no discovery lock afterwards; a key set
    /// that expired, or a `kid` retired or replaced since, is a denial with the same generic
    /// text as an unavailable identity provider.
    ///
    /// # Errors
    /// [`EnterpriseFailure::SsoIssuanceDenied`] when the proof no longer holds.
    pub fn ensure_key_fresh(
        &self,
        spirit_pid: u32,
        proof: &KeyFreshness,
    ) -> Result<(), EnterpriseFailure> {
        match &self.identity_port {
            Some(port) if !port.key_still_fresh(proof) => Err(self.idp_unavailable(spirit_pid)),
            _ => Ok(()),
        }
    }

    fn idp_unavailable(&self, spirit_pid: u32) -> EnterpriseFailure {
        let typed = self.sso_diagnostic.as_ref().and_then(|last| last());
        EnterpriseFailure::SsoIssuanceDenied {
            spirit_pid,
            reason: match typed {
                Some(code) => {
                    format!("MAOS_SSO_* configured but identity provider is unavailable: {code}")
                }
                None => "MAOS_SSO_* configured but identity provider is unavailable".to_string(),
            },
        }
    }

    fn verify_principal(
        &self,
        spirit_pid: u32,
        assertion: &str,
        wait: bool,
    ) -> Result<Option<(AuthenticatedPrincipal, Option<KeyFreshness>)>, EnterpriseFailure> {
        let unavailable = || self.idp_unavailable(spirit_pid);
        let denied = |reason: String| EnterpriseFailure::SsoIssuanceDenied { spirit_pid, reason };
        match self.config.sso {
            SubsystemState::Disabled => Ok(None),
            SubsystemState::Available => {
                let Some(port) = &self.identity_port else {
                    return Err(unavailable());
                };
                if wait {
                    if !port.is_healthy() {
                        return Err(unavailable());
                    }
                    return port
                        .verify_with_freshness(assertion, true)
                        .map(Some)
                        .map_err(|e| denied(e.to_string()));
                }
                // No wait: ask the keys held (this also nudges the refresher on a key refusal),
                // and say "unavailable" rather than the bare verdict when the set is unhealthy.
                port.verify_with_freshness(assertion, false)
                    .map(Some)
                    .map_err(|e| {
                        if port.is_healthy() {
                            denied(e.to_string())
                        } else {
                            unavailable()
                        }
                    })
            }
            SubsystemState::ConfiguredDown => Err(EnterpriseFailure::SsoIssuanceDenied {
                spirit_pid,
                reason: match &self.sso_down_reason {
                    Some(why) => {
                        format!("MAOS_SSO_* configured but identity provider is unavailable: {why}")
                    }
                    None => {
                        "MAOS_SSO_* configured but identity provider is unavailable".to_string()
                    }
                },
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

    /// Forward the (redacted) Transparency Log tail to the configured SIEM sink.
    /// `Disabled` → no forward. `Available` → `maos_siem::forward_to_file`
    /// (redaction applied before projection). On sink I/O error or
    /// `ConfiguredDown` → `SiemSinkDown` (buffered count surfaced to the
    /// operator); records are never silently dropped.
    pub fn forward_audit_to_siem(&self, filter: AuditFilter) -> Result<usize, EnterpriseFailure> {
        match self.config.siem {
            SubsystemState::Disabled => Ok(0),
            SubsystemState::Available => {
                let port = self
                    .siem_port
                    .as_ref()
                    .filter(|port| port.is_healthy())
                    .ok_or(EnterpriseFailure::SiemSinkDown { buffered: 0 })?;
                let (db, sink) = match (&self.audit_db_path, &self.siem_sink_path) {
                    (Some(db), Some(sink)) => (db, sink),
                    _ => return Err(EnterpriseFailure::SiemSinkDown { buffered: 0 }),
                };

                // `maos-audit` deliberately accepts only a quiesced database for
                // deterministic redaction/export. A running daemon necessarily
                // owns a live WAL, so take a transactionally consistent SQLite
                // snapshot and export that snapshot within this same daemon
                // lifecycle instead of silently buffering every live record.
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
                    let _ = std::fs::remove_file(&path);
                    let connection = rusqlite::Connection::open(db)
                        .map_err(|_| EnterpriseFailure::SiemSinkDown { buffered: 0 })?;
                    let path_text = path.to_string_lossy().into_owned();
                    if connection.execute("VACUUM INTO ?1", [&path_text]).is_err() {
                        let _ = std::fs::remove_file(&path);
                        return Err(EnterpriseFailure::SiemSinkDown { buffered: 0 });
                    }
                    snapshot = Some(path.clone());
                    path
                } else {
                    db.clone()
                };

                let outcome = match forward_to_file_with_port(
                    &source_db,
                    filter.clone(),
                    sink,
                    port.as_ref(),
                ) {
                    Ok(n) => Ok(n),
                    Err(_) => {
                        // On sink-down, surface how many records the
                        // consistent source snapshot holds.
                        let buffered = export_report_from_tl(&source_db, filter)
                            .ok()
                            .and_then(|report| report.forwarded_count)
                            .unwrap_or(0);
                        Err(EnterpriseFailure::SiemSinkDown { buffered })
                    }
                };
                if let Some(path) = snapshot {
                    let _ = std::fs::remove_file(path);
                }
                outcome
            }
            SubsystemState::ConfiguredDown => Err(EnterpriseFailure::SiemSinkDown { buffered: 0 }),
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

/// Story 23-2 — the verifier [`crate::directed_task::DirectedTasks`] checks a door-carried
/// assertion with at accept: this Host's own SSO verifier.
struct RuntimeAssertionCheck(Arc<EnterpriseRuntime>);

impl crate::directed_task::AssertionCheck for RuntimeAssertionCheck {
    fn check(
        &self,
        assertion: &maos_control::Assertion,
    ) -> Result<(), crate::directed_task::AssertionRefused> {
        self.0.check_carried_assertion(assertion.expose())
    }
}

/// Install the carried-assertion check on `tasks`. Without an enterprise runtime nothing is
/// installed, so a task that carries an assertion is refused (`AssertionInvalid`).
pub fn install_assertion_check(
    tasks: &crate::directed_task::DirectedTasks,
    enterprise: Option<&Arc<EnterpriseRuntime>>,
) {
    if let Some(runtime) = enterprise {
        tasks.set_assertion_check(Arc::new(RuntimeAssertionCheck(Arc::clone(runtime))));
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
    #[error("SIEM sink down; {buffered} record(s) buffered")]
    SiemSinkDown { buffered: usize },
}

fn any_env_with_prefix(prefix: &str) -> bool {
    std::env::vars_os().any(|(name, _)| name.to_string_lossy().starts_with(prefix))
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
        &crate::bounded_resolver::system(),
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

/// The `MAOS_SSO_*` configuration values (not the per-request assertion), read
/// once so tests drive the build without touching the process environment.
#[derive(Debug, Clone, Default)]
pub struct SsoEnv {
    pub jwks: Option<String>,
    pub issuers: Option<String>,
    pub audience: Option<String>,
    pub algs: Option<String>,
    /// Story 23-2 (D-23-2-06 A).
    pub discovery_issuer: Option<String>,
    pub jwks_ttl_secs: Option<String>,
    pub jwks_min_refresh_secs: Option<String>,
    /// The first `MAOS_SSO_*` variable that is PRESENT but not valid UTF-8 (Story 23-2 F13). Such
    /// a variable is configuration the operator meant: it is never read as absent (that would
    /// silently turn SSO off, or drop a tuning knob), so it makes the SSO configuration down.
    /// Only the variable's name is kept, never its value.
    pub invalid: Option<&'static str>,
}

impl SsoEnv {
    #[must_use]
    pub fn from_process() -> Self {
        Self::from_lookup(|name| std::env::var_os(name))
    }

    /// [`Self::from_process`] over any lookup (a test drives it without the process environment).
    #[must_use]
    pub fn from_lookup(get: impl Fn(&str) -> Option<std::ffi::OsString>) -> Self {
        let mut invalid = None;
        let mut var = |name: &'static str| match get(name)?.into_string() {
            Ok(value) => Some(value),
            Err(_) => {
                invalid.get_or_insert(name);
                None
            }
        };
        let jwks = var("MAOS_SSO_JWKS");
        let issuers = var("MAOS_SSO_ISSUERS");
        let audience = var("MAOS_SSO_AUDIENCE");
        let algs = var("MAOS_SSO_ALGS");
        let discovery_issuer = var("MAOS_SSO_DISCOVERY_ISSUER");
        let jwks_ttl_secs = var("MAOS_SSO_JWKS_TTL_SECS");
        let jwks_min_refresh_secs = var("MAOS_SSO_JWKS_MIN_REFRESH_SECS");
        Self {
            jwks,
            issuers,
            audience,
            algs,
            discovery_issuer,
            jwks_ttl_secs,
            jwks_min_refresh_secs,
            invalid,
        }
    }

    /// Whether any configuration value is present (a present but invalid one counts).
    #[must_use]
    pub fn any(&self) -> bool {
        self.jwks.is_some()
            || self.issuers.is_some()
            || self.audience.is_some()
            || self.algs.is_some()
            || self.discovery_issuer.is_some()
            || self.invalid.is_some()
            || self.tuning_set()
    }

    fn tuning_set(&self) -> bool {
        self.jwks_ttl_secs.is_some() || self.jwks_min_refresh_secs.is_some()
    }
}

/// Why the SSO configuration was refused. Carries no URL, key or assertion.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SsoDownReason {
    #[error("{0} absent")]
    Missing(&'static str),
    #[error("MAOS_SSO_ALGS resolved to no supported algorithms")]
    NoAlgorithms,
    #[error("{0}")]
    StaticVerifier(String),
    #[error("static verifier holds no usable key")]
    Unhealthy,
    /// `MAOS_SSO_JWKS` and `MAOS_SSO_DISCOVERY_ISSUER` are exclusive.
    #[error("sso_key_sources_exclusive")]
    KeySourcesConflict,
    /// A tuning knob without `MAOS_SSO_DISCOVERY_ISSUER`.
    #[error("sso_tuning_without_discovery")]
    TuningWithoutDiscovery,
    #[error("sso_tuning_invalid: {0}")]
    TuningInvalid(&'static str),
    #[error("sso_discovery_needs_online_adapters")]
    DiscoveryUnsupportedBuild,
    /// The outbound policy could not be armed (the port does not declare no-follow).
    #[error("sso_discovery_outbound_unavailable: {0}")]
    Outbound(&'static str),
    /// A `MAOS_SSO_*` variable is set to bytes that are not valid UTF-8. Names the variable
    /// only, never the value.
    #[error("sso_env_not_unicode: {0}")]
    EnvNotUnicode(&'static str),
    #[cfg(feature = "online-adapters")]
    #[error("{0}")]
    Discovery(maos_sso::discovery::DiscoveryError),
}

/// What discovery fetches through and reads time from. The production value
/// is [`Self::production`]; tests inject a scripted fetcher and a manual clock.
pub struct DiscoveryWiring {
    #[cfg(feature = "online-adapters")]
    fetcher: Arc<dyn maos_sso::discovery::Fetcher>,
    #[cfg(feature = "online-adapters")]
    clock: Arc<dyn maos_sso::discovery::Clock>,
}

#[cfg(feature = "online-adapters")]
impl DiscoveryWiring {
    #[must_use]
    pub fn new(
        fetcher: Arc<dyn maos_sso::discovery::Fetcher>,
        clock: Arc<dyn maos_sso::discovery::Clock>,
    ) -> Self {
        Self { fetcher, clock }
    }

    /// The kernel HTTP adapter behind the shared outbound policy (strict:
    /// public https only, no redirect, host names bound to the vetted
    /// addresses) and the bounded resolver.
    ///
    /// # Errors
    /// [`SsoDownReason::Outbound`] when the policy cannot wrap the adapter.
    pub fn production() -> Result<Self, SsoDownReason> {
        let policed = maos_domain::outbound::PolicedIo::new(
            maos_kernel_core::api::IoSubsystemAdapter::new(),
            crate::bounded_resolver::system(),
            OutboundPolicy::strict(),
        )
        .map_err(|refusal| SsoDownReason::Outbound(refusal.code()))?;
        Ok(Self::new(
            Arc::new(maos_sso::discovery::IoFetcher::new(policed)),
            Arc::new(maos_sso::discovery::MonotonicClock::new()),
        ))
    }
}

/// Build the SSO verifier from `MAOS_SSO_*` (D-23-2-06 A): the static
/// `MAOS_SSO_JWKS` path, or discovery of `MAOS_SSO_DISCOVERY_ISSUER`. Both
/// together, a tuning knob without discovery, and a discovery issuer missing
/// from `MAOS_SSO_ISSUERS` are refusals. A discovering verifier is returned
/// even though it holds no key yet (D-23-2-08 A): it reports unhealthy until
/// the first fetch succeeds, and issuance is denied meanwhile.
///
/// # Errors
/// The [`SsoDownReason`] of the first rule that fails.
pub fn build_sso_identity(
    env: &SsoEnv,
    discovery: &dyn Fn() -> Result<DiscoveryWiring, SsoDownReason>,
) -> Result<Arc<dyn IdentityAssertionPort>, SsoDownReason> {
    build_sso_wired(env, discovery).map(|(port, _)| port)
}

/// The typed reason discovery last failed (`sso_discovery_*`), when there is one. Fixed codes
/// only: no URL, userinfo or body.
pub type SsoDiagnostic = Arc<dyn Fn() -> Option<String> + Send + Sync>;

type SsoWired = (Arc<dyn IdentityAssertionPort>, Option<SsoDiagnostic>);

/// [`build_sso_identity`] plus the discovery diagnostic the runtime names in its denials.
fn build_sso_wired(
    env: &SsoEnv,
    discovery: &dyn Fn() -> Result<DiscoveryWiring, SsoDownReason>,
) -> Result<SsoWired, SsoDownReason> {
    if let Some(name) = env.invalid {
        return Err(SsoDownReason::EnvNotUnicode(name));
    }
    if env.jwks.is_some() && env.discovery_issuer.is_some() {
        return Err(SsoDownReason::KeySourcesConflict);
    }
    if env.discovery_issuer.is_none() && env.tuning_set() {
        return Err(SsoDownReason::TuningWithoutDiscovery);
    }
    let jwks = match env.discovery_issuer {
        Some(_) => None,
        None => Some(
            env.jwks
                .as_deref()
                .ok_or(SsoDownReason::Missing("MAOS_SSO_JWKS"))?,
        ),
    };
    let issuers_raw = env
        .issuers
        .as_deref()
        .ok_or(SsoDownReason::Missing("MAOS_SSO_ISSUERS"))?;
    let audience = env
        .audience
        .as_deref()
        .ok_or(SsoDownReason::Missing("MAOS_SSO_AUDIENCE"))?;
    let algs: Vec<OidcAlgorithm> = env
        .algs
        .as_deref()
        .unwrap_or("RS256,ES256")
        .split(',')
        .filter_map(|s| match s.trim() {
            "RS256" => Some(OidcAlgorithm::Rs256),
            "ES256" => Some(OidcAlgorithm::Es256),
            _ => None,
        })
        .collect();
    if algs.is_empty() {
        return Err(SsoDownReason::NoAlgorithms);
    }
    let issuers: Vec<&str> = issuers_raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    match (jwks, env.discovery_issuer.as_deref()) {
        (Some(jwks), _) => {
            let verifier = OidcVerifier::from_static_jwks(jwks, &algs, &issuers, audience)
                .map_err(|e| SsoDownReason::StaticVerifier(e.to_string()))?;
            if !verifier.is_healthy() {
                return Err(SsoDownReason::Unhealthy);
            }
            let port: Arc<dyn IdentityAssertionPort> = Arc::new(verifier);
            Ok((port, None))
        }
        (None, Some(issuer)) => {
            start_discovering(env, issuer, &algs, &issuers, audience, discovery)
        }
        (None, None) => Err(SsoDownReason::Missing("MAOS_SSO_JWKS")),
    }
}

#[cfg(feature = "online-adapters")]
fn start_discovering(
    env: &SsoEnv,
    issuer: &str,
    algs: &[OidcAlgorithm],
    issuers: &[&str],
    audience: &str,
    discovery: &dyn Fn() -> Result<DiscoveryWiring, SsoDownReason>,
) -> Result<SsoWired, SsoDownReason> {
    use maos_sso::discovery::{DiscoveringVerifier, DiscoveryConfig};

    let seconds = |raw: &str, name: &'static str| {
        raw.trim()
            .parse::<u64>()
            .map_err(|_| SsoDownReason::TuningInvalid(name))
    };
    let mut config = DiscoveryConfig::new(issuer);
    if let Some(raw) = env.jwks_ttl_secs.as_deref() {
        config = config.with_ttl_secs(seconds(raw, "MAOS_SSO_JWKS_TTL_SECS")?);
    }
    if let Some(raw) = env.jwks_min_refresh_secs.as_deref() {
        config = config.with_min_refresh_secs(seconds(raw, "MAOS_SSO_JWKS_MIN_REFRESH_SECS")?);
    }
    let wiring = discovery()?;
    let verifier = DiscoveringVerifier::start(
        config,
        algs,
        issuers,
        audience,
        wiring.fetcher,
        wiring.clock,
    )
    .map_err(SsoDownReason::Discovery)?;
    let verifier = Arc::new(verifier);
    let probe = Arc::clone(&verifier);
    let port: Arc<dyn IdentityAssertionPort> = verifier;
    let diagnostic: SsoDiagnostic = Arc::new(move || probe.last_error().map(|e| e.to_string()));
    Ok((port, Some(diagnostic)))
}

#[cfg(not(feature = "online-adapters"))]
fn start_discovering(
    _env: &SsoEnv,
    _issuer: &str,
    _algs: &[OidcAlgorithm],
    _issuers: &[&str],
    _audience: &str,
    _discovery: &dyn Fn() -> Result<DiscoveryWiring, SsoDownReason>,
) -> Result<SsoWired, SsoDownReason> {
    Err(SsoDownReason::DiscoveryUnsupportedBuild)
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
    let mut frame_id = [0u8; 16];
    getrandom::fill(&mut frame_id)
        .map_err(|e| AuditError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let payload = serde_json::json!({
        "subject": subject,
        "issuer": issuer,
        "capability_key": capability_key,
        "decision_time_ns": decision_time_ns,
    });
    let payload_bytes = serde_json::to_vec(&payload)?;

    let spirit_pid_i = i64::try_from(spirit_pid).map_err(|_| AuditError::ValueOverflow {
        field: "spirit_pid",
        value: spirit_pid.into(),
    })?;
    let boot_nonce_i = i64::try_from(boot_nonce).map_err(|_| AuditError::ValueOverflow {
        field: "boot_nonce",
        value: boot_nonce,
    })?;
    let decision_time_ns_i =
        i64::try_from(decision_time_ns).map_err(|_| AuditError::ValueOverflow {
            field: "decision_time_ns",
            value: decision_time_ns,
        })?;

    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(AuditError::Open)?;

    // Plain INSERT (NOT `INSERT OR IGNORE`): a frame_id PK collision surfaces as
    // SQLITE_CONSTRAINT_PRIMARYKEY → AuditError::Query rather than being silently
    // swallowed (Story 11.4c "fail closed" requirement).
    conn.execute(
        "INSERT INTO transparency_log \
         (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, \
          kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &frame_id[..],                                                  // frame_id
            decision_time_ns_i,                                             // timestamp_ns
            spirit_pid_i,                                                   // spirit_pid
            boot_nonce_i,                                                   // boot_nonce
            Option::<&[u8]>::None, // capability_token = NULL
            30i64,                 // kind — identity.asserted
            "identity.asserted",   // intent
            payload_bytes,         // payload_redacted (compact JSON)
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
            boot_nonce: 7,
            kms_probe: None,
            sso_down_reason: None,
            sso_diagnostic: None,
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
        let n = rt
            .forward_audit_to_siem(AuditFilter::default())
            .expect("Available SIEM MUST forward to the file sink");
        assert!(n >= 1, "forwarded count MUST reflect the seeded record");
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
        match rt.forward_audit_to_siem(AuditFilter::default()) {
            Err(EnterpriseFailure::SiemSinkDown { buffered }) => {
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

/// Story 23-2 round 2 (C8): a denial while discovery is down names the typed reason the
/// verifier recorded, and nothing of the issuer URL.
#[cfg(all(test, feature = "online-adapters"))]
mod discovery_denial_tests {
    use super::*;
    use maos_kernel_core::security::RingCryptoProvider;
    use maos_sso::discovery::{Clock, FetchError, Fetcher};
    use std::time::Instant;

    struct Down;
    impl Fetcher for Down {
        fn get(&self, _url: &str) -> Result<Vec<u8>, FetchError> {
            Err(FetchError::Unavailable)
        }
    }
    struct Still;
    impl Clock for Still {
        fn now_millis(&self) -> u64 {
            0
        }
    }

    #[test]
    fn a_denial_while_discovery_is_down_names_the_typed_reason_and_no_url() {
        const ISSUER: &str = "https://idp-canary.example";
        let env = SsoEnv {
            issuers: Some(ISSUER.to_owned()),
            audience: Some("aud".to_owned()),
            discovery_issuer: Some(ISSUER.to_owned()),
            ..SsoEnv::default()
        };
        let crypto: Arc<dyn CryptoProvider> = Arc::new(RingCryptoProvider);
        let runtime = EnterpriseRuntime::from_env_with(
            &env,
            &|| Ok(DiscoveryWiring::new(Arc::new(Down), Arc::new(Still))),
            crypto,
            std::env::temp_dir().join("maos-23-2-denial-never-written.sqlite"),
            1,
            None,
        )
        .expect("a runtime builds");
        // The first refresh runs on the refresher thread: wait for the observable, fail at the
        // deadline.
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            let Err(EnterpriseFailure::SsoIssuanceDenied { reason, .. }) =
                runtime.verify_principal_for_issuance(42, "a.b.c")
            else {
                panic!("an unhealthy discovery verifier must deny");
            };
            if reason.contains("sso_discovery_document_fetch_failed") {
                assert!(!reason.contains("idp-canary"), "{reason}");
                return;
            }
            assert!(
                Instant::now() < until,
                "the typed reason never surfaced: {reason}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}
