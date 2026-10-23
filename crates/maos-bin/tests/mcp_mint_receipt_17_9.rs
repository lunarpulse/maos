#![cfg(feature = "network")]
#![forbid(unsafe_code)]

use maos_bin::{
    enterprise_identity::{EnterpriseConfig, EnterpriseRuntime},
    enterprise_pdp_runtime::EnterprisePdpRuntime,
    worker_spawn::{issue_governed_mcp_capability, GovernedMintError},
};
use maos_domain::{
    invariants::{i1::Scope, i9::SandboxTier},
    ports::{AuthenticatedPrincipal, IdentityAssertionPort, IdentityError},
};
use maos_iac::adapter::transparency_log::{FrameFilter, TransparencyLogAdapter};
use maos_kernel_core::capability::{
    cap_audit,
    cap_policy::{decision::TrustTier, ManifestCapabilityScope, PolicyTable, PolicyTableInner},
    cap_quota::{CapQuotaTracker, IssuanceQuota},
    cap_tokens::Ed25519SigningKey,
    CapabilityRegistryAdapter, WorkingMemoryStore,
};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

const PID: u32 = 179;
const NONCE: u64 = 179;
const ID: &str = "trusted-mcp-spirit";

#[derive(Debug, Default)]
struct ManualClock(AtomicU64);

impl maos_kernel_core::capability::cap_quota::QuotaClock for ManualClock {
    fn now_ns(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
}

fn fixture(
    grant: bool,
) -> (
    CapabilityRegistryAdapter,
    CapQuotaTracker,
    TransparencyLogAdapter,
    Arc<PolicyTable>,
) {
    maos_kernel_core::capability::cap_tokens::init_monotonic_base();
    let policy = Arc::new(PolicyTable::new());
    let mut inner = PolicyTableInner::default();
    if grant {
        inner.manifest_scopes.insert(
            PID,
            ManifestCapabilityScope {
                scopes: vec![Scope::McpCall {
                    server: "web".into(),
                    tool: "search".into(),
                }],
                declared_tier: SandboxTier(0),
                trust_tier: TrustTier::Verified,
            },
        );
    }
    policy.update(inner);
    let tracker = CapQuotaTracker::with_clock(
        IssuanceQuota {
            budget_per_window: 1,
            window: Duration::from_secs(3600),
        },
        Arc::new(ManualClock::default()),
    );
    let (tx, _rx) = cap_audit::channel();
    let cap = CapabilityRegistryAdapter::new(
        Arc::new(maos_kernel_core::api::RingCryptoProvider),
        Ed25519SigningKey::new([17; 32]),
        NONCE,
        policy.clone(),
        tx,
        tracker.clone(),
        Arc::new(WorkingMemoryStore::new()),
        Arc::new(maos_kernel_core::telemetry::TelemetryStreamAdapter::default()),
    );
    (
        cap,
        tracker,
        TransparencyLogAdapter::open_in_memory(NONCE),
        policy,
    )
}

fn mint(
    cap: &CapabilityRegistryAdapter,
    log: &TransparencyLogAdapter,
    enterprise: Option<&EnterpriseRuntime>,
    pdp: Option<&EnterprisePdpRuntime>,
) -> Result<maos_domain::invariants::i1::CapabilityToken, GovernedMintError> {
    issue_governed_mcp_capability(cap, enterprise, pdp, log, PID, ID, "web", "search", [0; 32])
}

fn receipt(log: &TransparencyLogAdapter) {
    let rows = log.query_frames(FrameFilter::default()).unwrap();
    assert_eq!(rows.len(), 1, "exactly one MCP refusal receipt");
    let row = &rows[0];
    assert_eq!(row.kind as i64, 28);
    assert_eq!(row.spirit_pid, PID);
    assert_eq!(row.from_spirit_id, ID);
    assert_eq!(row.intent, "mcp.call.refused:capability-denied");
    assert_eq!(row.capability_token, None);
    assert_eq!(std::str::from_utf8(&row.payload_redacted).unwrap(), format!("{{\"op\":\"mcp.call\",\"code\":\"capability-denied\",\"server\":\"web\",\"tool\":\"search\",\"boot_nonce\":{NONCE}}}"));
}

#[test]
fn quota_exhaustion_has_one_trusted_receipt() {
    let (cap, tracker, log, _) = fixture(true);
    let _reserved = tracker.charge(PID).unwrap();
    assert_eq!(
        mint(&cap, &log, None, None),
        Err(GovernedMintError::QuotaExhausted)
    );
    receipt(&log);
}

#[test]
fn kernel_policy_denial_has_one_trusted_receipt() {
    let (cap, _, log, _) = fixture(false);
    assert_eq!(
        mint(&cap, &log, None, None),
        Err(GovernedMintError::KernelPolicyDenied)
    );
    receipt(&log);
}

#[test]
fn successful_mint_returns_token_without_receipt() {
    let (cap, _, log, _) = fixture(true);
    let token = mint(&cap, &log, None, None).unwrap();
    assert_eq!(token.spirit_pid, PID);
    assert!(cap.has_active_tokens_for_pid(PID));
    assert!(log.query_frames(FrameFilter::default()).unwrap().is_empty());
}

struct AcceptSso;
impl IdentityAssertionPort for AcceptSso {
    fn verify(&self, _: &str) -> Result<AuthenticatedPrincipal, IdentityError> {
        Ok(AuthenticatedPrincipal {
            subject: "operator".into(),
            issuer: "test-idp".into(),
            audience: "maos".into(),
            attributes: Default::default(),
        })
    }
    fn is_healthy(&self) -> bool {
        true
    }
}

struct RestoreAssertion(Option<std::ffi::OsString>);
impl Drop for RestoreAssertion {
    fn drop(&mut self) {
        match &self.0 {
            Some(value) => std::env::set_var("MAOS_SSO_ASSERTION", value),
            None => std::env::remove_var("MAOS_SSO_ASSERTION"),
        }
    }
}

static SSO_ENV: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

#[test]
fn sso_refusal_has_typed_error_and_one_receipt() {
    let _lock = SSO_ENV.lock();
    let _restore = RestoreAssertion(std::env::var_os("MAOS_SSO_ASSERTION"));
    std::env::remove_var("MAOS_SSO_ASSERTION");
    let enterprise = EnterpriseRuntime::from_ports(
        EnterpriseConfig::empty().with_sso_available(),
        Some(Arc::new(AcceptSso)),
        None,
        None,
        None,
        None,
        None,
        NONCE,
    );
    let (cap, _tracker, log, _) = fixture(true);
    assert!(matches!(
        mint(&cap, &log, Some(&enterprise), None),
        Err(GovernedMintError::SsoAssertionMissing(_))
    ));
    receipt(&log);
    assert!(
        mint(&cap, &log, None, None).is_ok(),
        "SSO refusal must not charge"
    );
}

#[test]
fn pdp_refusal_has_typed_error_and_one_receipt() {
    let (cap, _tracker, log, policy) = fixture(true);
    let pdp = EnterprisePdpRuntime::new(
        Arc::new(
            maos_pdp::CedarPolicyAdapter::with_policy("forbid(principal, action, resource);")
                .unwrap(),
        ),
        policy,
        Duration::from_secs(30),
        Duration::from_secs(60),
    )
    .unwrap();
    assert!(matches!(
        mint(&cap, &log, None, Some(&pdp)),
        Err(GovernedMintError::PdpDenied(_))
    ));
    receipt(&log);
    assert!(
        mint(&cap, &log, None, None).is_ok(),
        "PDP refusal must not charge"
    );
}

#[test]
fn identity_persistence_rollback_has_typed_error_and_one_receipt() {
    let _lock = SSO_ENV.lock();
    let _restore = RestoreAssertion(std::env::var_os("MAOS_SSO_ASSERTION"));

    std::env::set_var("MAOS_SSO_ASSERTION", "accepted-by-test-port");
    let dir = tempfile::tempdir().unwrap();
    // SQLite cannot open a directory as a database: verification succeeds, but
    // the post-mint identity.asserted write fails and must revoke the token.
    let enterprise = EnterpriseRuntime::from_ports(
        EnterpriseConfig::empty().with_sso_available(),
        Some(Arc::new(AcceptSso)),
        None,
        None,
        None,
        Some(dir.path().to_owned()),
        None,
        NONCE,
    );
    let (cap, _tracker, log, _) = fixture(true);
    assert!(
        matches!(mint(&cap, &log, Some(&enterprise), None), Err(GovernedMintError::Mediation(message)) if message.contains("identity.asserted persist failed"))
    );
    assert!(
        !cap.has_active_tokens_for_pid(PID),
        "identity persistence failure rolls the minted token back"
    );
    receipt(&log);
    assert!(
        matches!(
            cap.issue_with_mediation(
                PID,
                Scope::McpCall {
                    server: "web".into(),
                    tool: "search".into()
                },
                60,
                [0; 32],
                maos_domain::invariants::i1::IntentClass::Standard,
            ),
            Err(
                maos_domain::ports::capability::CapError::IssuanceQuotaExhausted { spirit_id: PID }
            )
        ),
        "identity persistence rollback must retain its charge"
    );
}
