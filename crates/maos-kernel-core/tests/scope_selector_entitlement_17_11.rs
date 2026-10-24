//! Story 17-11 AC1 — a capability is issued only for a selector a declared
//! manifest scope entitles (`Scope::entitles`), and a refused selector is
//! denied by policy before any quota charge (17-9 order policy → quota → tokens).
//!
//! The adapter is built as 17-9's harness builds it
//! (`issuance_quota_window_17_9.rs`): mock signer, `CapQuotaTracker::with_clock`
//! on a frozen manual clock, every audit event drained by a thread with a FIFO
//! progress barrier.
//!
//! `cargo nextest run -p maos-kernel-core --test scope_selector_entitlement_17_11`

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::invariants::i9::SandboxTier;
use maos_domain::ports::capability::CapError;
use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
use maos_kernel_core::capability::{
    cap_audit::{self, CapAuditEvent},
    cap_policy,
    cap_quota::{CapQuotaTracker, IssuanceQuota, QuotaClock, DEFAULT_ISSUANCE_QUOTA},
    cap_tokens, CapabilityRegistryAdapter, WorkingMemoryStore,
};
use maos_kernel_core::telemetry::TelemetryStreamAdapter;
use parking_lot::{Condvar, Mutex};

/// Frozen clock: every charge lands in window 0.
#[derive(Debug, Default)]
struct FrozenClock;

impl QuotaClock for FrozenClock {
    fn now_ns(&self) -> u64 {
        0
    }
}

/// Deterministic mock signer.
struct MockCrypto;

impl CryptoProvider for MockCrypto {
    fn verify_signature(&self, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> Result<(), CryptoError> {
        Ok(())
    }

    fn seal_for_export(
        &self,
        _key: &[u8],
        _nonce: &[u8],
        _aad: &[u8],
        plaintext: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        Ok(plaintext.to_vec())
    }

    fn sign_capability_token(
        &self,
        _key: &[u8],
        token_bytes: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        let mut signature = [0u8; 64];
        for (index, byte) in token_bytes.iter().enumerate() {
            signature[index % 64] ^= byte;
        }
        Ok(signature.to_vec())
    }
}

const BARRIER_SYSCALL: &str = "17-11.progress-barrier";

#[derive(Default)]
struct DrainState {
    issues: HashMap<u32, u64>,
    barriers: u64,
}

#[derive(Default)]
struct Drain {
    state: Mutex<DrainState>,
    progress: Condvar,
}

struct Harness {
    adapter: Arc<CapabilityRegistryAdapter>,
    drain: Arc<Drain>,
    drain_thread: Option<JoinHandle<()>>,
    barriers_sent: u64,
    started: Instant,
}

impl Harness {
    /// Each `(pid, scopes)` row is one Spirit's declared manifest scopes.
    fn new(budget: u64, rows: Vec<(u32, Vec<Scope>)>, org_deny: &[&str]) -> Self {
        cap_tokens::init_monotonic_base();
        let policy = cap_policy::PolicyTable::new();
        policy.set_operator_posture_ceiling(None);
        let mut inner = cap_policy::PolicyTableInner::default();
        for (pid, scopes) in rows {
            inner.manifest_scopes.insert(
                pid,
                cap_policy::ManifestCapabilityScope {
                    scopes,
                    declared_tier: SandboxTier(0),
                    trust_tier: cap_policy::decision::TrustTier::Verified,
                },
            );
        }
        inner.operator_policy.per_capability_deny = org_deny
            .iter()
            .map(|k| (*k).to_owned())
            .collect::<HashSet<_>>();
        policy.update(inner);
        let quota = IssuanceQuota {
            budget_per_window: budget,
            ..DEFAULT_ISSUANCE_QUOTA
        };
        let (audit, mut receiver) = cap_audit::channel();
        let adapter = Arc::new(CapabilityRegistryAdapter::new(
            Arc::new(MockCrypto),
            cap_tokens::Ed25519SigningKey::new([0x17; 32]),
            0x17_11,
            Arc::new(policy),
            audit,
            CapQuotaTracker::with_clock(quota, Arc::new(FrozenClock)),
            Arc::new(WorkingMemoryStore::new()),
            Arc::new(TelemetryStreamAdapter::default()),
        ));
        let drain = Arc::new(Drain::default());
        let sink = drain.clone();
        let drain_thread = Some(std::thread::spawn(move || {
            while let Some(event) = receiver.blocking_recv() {
                let mut state = sink.state.lock();
                match event {
                    CapAuditEvent::Issue { spirit_pid, .. } => {
                        *state.issues.entry(spirit_pid).or_default() += 1;
                    }
                    CapAuditEvent::SandboxBlock {
                        attempted_syscall, ..
                    } if attempted_syscall == BARRIER_SYSCALL => state.barriers += 1,
                    // Quota signals (limit/exhausted) are 17-9's subject.
                    CapAuditEvent::Quota { .. } => {}
                    other => panic!("unexpected audit event {other:?}"),
                }
                drop(state);
                sink.progress.notify_all();
            }
        }));
        Self {
            adapter,
            drain,
            drain_thread,
            barriers_sent: 0,
            started: Instant::now(),
        }
    }

    fn issue(&self, pid: u32, scope: Scope) -> Result<CapabilityToken, CapError> {
        self.adapter
            .issue_with_mediation(pid, scope, 60, [1u8; 32], IntentClass::Standard)
    }

    /// Issue events the drain has seen for `pid` once every prior event landed.
    fn issues(&mut self, pid: u32) -> u64 {
        self.adapter
            .audit_sender()
            .try_send(CapAuditEvent::SandboxBlock {
                spirit_pid: 0,
                attempted_syscall: BARRIER_SYSCALL.into(),
                sandbox_tier: SandboxTier(0),
            })
            .expect("the audit channel never fills here");
        self.barriers_sent += 1;
        let target = self.barriers_sent;
        let mut state = self.drain.state.lock();
        // A drain-thread panic must fail this test, not hang it until
        // nextest's terminate-after (17-11 review): bounded wait.
        let waited = self.drain.progress.wait_while_for(
            &mut state,
            |state| state.barriers < target,
            Duration::from_secs(5),
        );
        assert!(
            !waited.timed_out(),
            "the audit drain stalled (drain thread panicked or blocked)"
        );
        state.issues.get(&pid).copied().unwrap_or(0)
    }

    fn finish(self, name: &str) {
        let Self {
            adapter,
            drain_thread,
            started,
            ..
        } = self;
        drop(adapter);
        drain_thread.unwrap().join().unwrap();
        eprintln!("17-11 AC1 {name}: wall {:?}", started.elapsed());
    }
}

fn s(v: &str) -> String {
    v.to_owned()
}

fn cli(path: &str, hash: u8, version: &str) -> Scope {
    Scope::CliSubprocessSpawn {
        cli_binary_path: s(path),
        argv_prefix_hash: [hash; 32],
        output_shape_version: s(version),
    }
}

fn mcp(server: &str, tool: &str) -> Scope {
    Scope::McpCall {
        server: s(server),
        tool: s(tool),
    }
}

/// `(declared, undeclared requests)` — one row per mediated parameterized scope.
fn rows() -> Vec<(Scope, Vec<Scope>)> {
    let fs_undeclared = |mk: fn(String) -> Scope| {
        [
            "/data/projectX",
            "/data",
            "/data/project/../secret",
            "data/project",
        ]
        .into_iter()
        .map(|p| mk(s(p)))
        .collect::<Vec<_>>()
    };
    let fs_read = |subtree| Scope::FsRead { subtree };
    let fs_write = |subtree| Scope::FsWrite { subtree };
    let mut read_undeclared = fs_undeclared(fs_read);
    read_undeclared.push(fs_write(s("/data/project")));
    let mem = |mk: fn(String) -> Scope| {
        (
            mk(s("spirit:butler:digest")),
            vec![mk(s("spirit:butler")), mk(s("spirit:butler:digest:x"))],
        )
    };
    let net = |d: &str| Scope::NetHttps { domain: s(d) };
    let provider = |p: &str| Scope::ProviderInfer { provider: s(p) };
    let exec = |b: &str| Scope::ProcExec { binary: s(b) };
    let sub = |c: &str| Scope::SubSpiritSpawn { class: s(c) };
    let iac = |p: &str| Scope::IacSend { peer_class: s(p) };
    vec![
        (
            mcp("web", "search"),
            vec![mcp("web", "fetch"), mcp("github", "search")],
        ),
        (iac("spirit:peer"), vec![iac("broadcast")]),
        (
            provider("anthropic"),
            vec![provider("ollama"), provider("replay")],
        ),
        (
            exec("/usr/bin/git"),
            vec![exec("/usr/bin/curl"), exec("git")],
        ),
        (sub("researcher"), vec![sub("butler")]),
        mem(|scope| Scope::MemRead { scope }),
        mem(|scope| Scope::MemWrite { scope }),
        (
            net("api.example.com"),
            vec![
                net("example.com"),
                net("evil.api.example.com"),
                net("API.example.com"),
            ],
        ),
        (fs_read(s("/data/project")), read_undeclared),
        (fs_write(s("/data/project")), fs_undeclared(fs_write)),
        (
            cli("/usr/bin/git", 1, "1"),
            vec![
                cli("/usr/bin/gh", 1, "1"),
                cli("/usr/bin/git", 2, "1"),
                cli("/usr/bin/git", 1, "2"),
            ],
        ),
    ]
}

#[test]
fn declared_selectors_issue_and_undeclared_are_denied_uncharged() {
    let rows = rows();
    let mut harness = Harness::new(
        1,
        rows.iter()
            .enumerate()
            .map(|(i, (declared, _))| (100 + i as u32, vec![declared.clone()]))
            .collect(),
        &[],
    );
    for (i, (declared, undeclared)) in rows.into_iter().enumerate() {
        let pid = 100 + i as u32;
        let undeclared_again = undeclared[0].clone();
        for request in undeclared {
            assert_eq!(
                harness.issue(pid, request.clone()).map(|_| ()),
                Err(CapError::PolicyDenied),
                "declared {declared:?} must not entitle {request:?}"
            );
        }
        harness
            .issue(pid, declared.clone())
            .unwrap_or_else(|e| panic!("declared {declared:?} must issue: {e}"));
        assert_eq!(
            harness.issue(pid, declared.clone()).map(|_| ()),
            Err(CapError::IssuanceQuotaExhausted { spirit_id: pid }),
            "budget 1: the refusals for {declared:?} charged nothing"
        );
        // Policy precedes quota (17-9 order policy → quota → tokens): with
        // the budget exhausted an undeclared request is still `PolicyDenied`;
        // quota-first ordering would answer `IssuanceQuotaExhausted`.
        assert_eq!(
            harness.issue(pid, undeclared_again).map(|_| ()),
            Err(CapError::PolicyDenied),
            "policy is evaluated before any quota charge for {declared:?}"
        );
        assert_eq!(harness.issues(pid), 1, "one Issue event for {declared:?}");
    }
    harness.finish("declared_selectors_issue_and_undeclared_are_denied_uncharged");
}

#[test]
fn fs_subtree_containment_issues_beneath_the_declaration() {
    let fs_read = |p: &str| Scope::FsRead { subtree: s(p) };
    let mut harness = Harness::new(
        16,
        vec![
            (1, vec![fs_read("/data/project")]),
            (2, vec![fs_read("/data/../etc")]),
        ],
        &[],
    );
    for path in [
        "/data/project",
        "/data/project/",
        "/data/project/a/b",
        "/data//project/./a",
    ] {
        harness
            .issue(1, fs_read(path))
            .unwrap_or_else(|e| panic!("{path} is beneath /data/project: {e}"));
    }
    assert_eq!(harness.issues(1), 4);
    for path in ["/etc", "/etc/passwd", "/data/../etc", "/data"] {
        assert_eq!(
            harness.issue(2, fs_read(path)).map(|_| ()),
            Err(CapError::PolicyDenied),
            "declared /data/../etc entitles nothing ({path})"
        );
    }
    assert_eq!(harness.issues(2), 0);
    harness.finish("fs_subtree_containment_issues_beneath_the_declaration");
}

#[test]
fn kind_only_and_unmediated_scopes_are_unchanged() {
    let gateway = Scope::GatewaySend {
        gateway_id: s("gw"),
        recipient: s("r"),
    };
    let mut harness = Harness::new(
        16,
        vec![
            (
                1,
                vec![
                    Scope::LoomRead,
                    Scope::LogRecall,
                    Scope::DistillateWrite,
                    gateway.clone(),
                ],
            ),
            (2, vec![mcp("web", "search")]),
        ],
        &["mcp.call"],
    );
    harness.issue(1, Scope::LoomRead).expect("LoomRead by kind");
    harness
        .issue(1, Scope::LogRecall)
        .expect("LogRecall by kind");
    harness
        .issue(1, Scope::SelfTelemetryRead)
        .expect("SelfTelemetryRead is always-allow, even undeclared");
    assert_eq!(
        harness.issue(1, Scope::DistillateWrite).map(|_| ()),
        Err(CapError::PolicyDenied),
        "DistillateWrite still requires approval"
    );
    assert_eq!(
        harness.issue(1, gateway).map(|_| ()),
        Err(CapError::PolicyDenied),
        "a declared GatewaySend is still unmediated"
    );
    assert_eq!(harness.issues(1), 3);
    assert_eq!(
        harness.issue(2, mcp("web", "search")).map(|_| ()),
        Err(CapError::PolicyDenied),
        "an org deny on mcp.call beats a matching selector"
    );
    assert_eq!(harness.issues(2), 0);
    harness.finish("kind_only_and_unmediated_scopes_are_unchanged");
}
