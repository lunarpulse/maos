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
//!   are real ciphertext (not passthrough plaintext).
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
//! and `identity.asserted` is written through the maos-audit raw-kind helper,
//! not a kernel `FrameKind` variant.

use std::path::PathBuf;
use std::sync::Arc;

use maos_audit::append_identity_asserted;
pub use maos_audit::AuditFilter;
use maos_domain::ports::{
    AuthenticatedPrincipal, CryptoProvider, IdentityAssertionPort, KeyManagementPort,
    SiemProjectionPort,
};
use maos_loom_lite::seal::AtRestSeal;
use maos_secrets::{seal_at_rest_opt, LocalMasterKeyKms};
use maos_siem::{export_report_from_tl, forward_to_file, SiemExporter};
use maos_sso::{OidcAlgorithm, OidcVerifier};

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
        })
    }

    /// Production constructor: builds the REAL adapters from env and demotes any
    /// subsystem that cannot be built or is unhealthy to `ConfiguredDown`
    /// (runtime-detected, not just a test fixture).
    pub fn from_env(
        crypto: Arc<dyn CryptoProvider>,
        audit_db_path: PathBuf,
        boot_nonce: u64,
    ) -> Result<Self, EnterpriseFailure> {
        let mut config = EnterpriseConfig::from_env();
        let mut identity_port: Option<Arc<dyn IdentityAssertionPort>> = None;
        let mut kms_port: Option<Arc<dyn KeyManagementPort>> = None;
        let mut siem_port: Option<Arc<dyn SiemProjectionPort>> = None;
        let mut siem_sink_path: Option<PathBuf> = None;

        if config.sso_configured() {
            match build_sso_verifier() {
                Ok(verifier) if verifier.is_healthy() => {
                    identity_port = Some(Arc::new(verifier));
                }
                _ => config = config.with_sso_down(),
            }
        }

        if config.kms_configured() {
            match build_local_kms() {
                Ok(kms) if kms.is_healthy() => {
                    kms_port = Some(Arc::new(kms));
                }
                _ => config = config.with_kms_down(),
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

        Ok(Self {
            config,
            identity_port,
            kms_port,
            siem_port,
            crypto: Some(crypto),
            audit_db_path: Some(audit_db_path),
            siem_sink_path,
            boot_nonce,
        })
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

    /// Forward the (redacted) Transparency Log tail to the configured SIEM sink.
    /// `Disabled` → no forward. `Available` → `maos_siem::forward_to_file`
    /// (redaction applied before projection). On sink I/O error or
    /// `ConfiguredDown` → `SiemSinkDown` (buffered count surfaced to the
    /// operator); records are never silently dropped.
    pub fn forward_audit_to_siem(&self, filter: AuditFilter) -> Result<usize, EnterpriseFailure> {
        match self.config.siem {
            SubsystemState::Disabled => Ok(0),
            SubsystemState::Available => {
                if self
                    .siem_port
                    .as_ref()
                    .map(|p| p.is_healthy())
                    .unwrap_or(false)
                {
                    let (db, sink) = match (&self.audit_db_path, &self.siem_sink_path) {
                        (Some(d), Some(s)) => (d, s),
                        _ => return Err(EnterpriseFailure::SiemSinkDown { buffered: 0 }),
                    };
                    match forward_to_file(db, filter.clone(), sink) {
                        Ok(n) => Ok(n),
                        Err(_) => {
                            // On sink-down, surface how many records the TL holds
                            // matching the filter (the buffer the operator must drain).
                            let buffered = export_report_from_tl(db, filter)
                                .ok()
                                .and_then(|r| r.forwarded_count)
                                .unwrap_or(0);
                            Err(EnterpriseFailure::SiemSinkDown { buffered })
                        }
                    }
                } else {
                    Err(EnterpriseFailure::SiemSinkDown { buffered: 0 })
                }
            }
            SubsystemState::ConfiguredDown => Err(EnterpriseFailure::SiemSinkDown { buffered: 0 }),
        }
    }

    /// Build the loom-lite at-rest seal hook from the configured KMS + crypto.
    /// `None` when KMS is unconfigured/unhealthy (loom-lite then writes
    /// byte-identical Option-A plaintext). `Some(closure)` when KMS is
    /// `Available` + healthy — the closure produces real AEAD ciphertext and
    /// fails closed on any seal error (loom-lite refuses the write).
    pub fn at_rest_seal_hook(&self) -> Option<AtRestSeal> {
        if !matches!(self.config.kms, SubsystemState::Available) {
            return None;
        }
        let kms = self.kms_port.clone()?;
        let crypto = self.crypto.clone()?;
        if !kms.is_healthy() {
            return None;
        }
        Some(Arc::new(move |plaintext: &[u8]| {
            seal_at_rest_opt(Some(kms.as_ref()), crypto.as_ref(), plaintext)
                .map_err(|e| e.to_string())
        }))
    }
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

/// Decode the hex-encoded 32-byte org master key from `MAOS_KMS_MASTER_KEY`.
fn build_local_kms() -> Result<LocalMasterKeyKms, String> {
    let raw = std::env::var("MAOS_KMS_MASTER_KEY").map_err(|_| "MAOS_KMS_MASTER_KEY absent")?;
    let bytes =
        hex::decode(raw.trim()).map_err(|e| format!("master key hex decode failed: {e}"))?;
    LocalMasterKeyKms::from_master_key(&bytes).map_err(|e| e.to_string())
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
    use maos_audit::{append_identity_asserted, query, AuditEntry};
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
        }
    }

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
