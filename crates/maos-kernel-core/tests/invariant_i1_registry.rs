//! I1 ledger proof: no external effect happens without a registry-verified, in-scope token.
//!
//! Seeded attempts (issue, verify, and forged tokens) drive the real
//! `CapabilityRegistryAdapter`; every effect that the registry lets through is checked by the
//! domain predicate `invariants::i1::check`. The adversarial corpus holds observations the
//! predicate must refuse. Plant: make `issue` skip the manifest entitlement (or `verify` accept
//! an unknown token) and the generated leg fails with its seed.

use maos_domain::invariants::i1::{
    check, CapabilityToken, Effect, IntentClass, Observation, Scope, TokenId,
};
use maos_domain::invariants::i9::SandboxTier;
use maos_kernel_core::capability::{cap_tokens, CapabilityRegistryPort};

mod common;

const SEEDS: [u64; 4] = [1, 0xC0FFEE, 0xDEAD_BEEF, 42];
const ATTEMPTS: usize = 40;
const POSTURE: [u8; 32] = [1u8; 32];

fn fs_read(subtree: &str) -> Scope {
    Scope::FsRead {
        subtree: subtree.into(),
    }
}

fn manifests() -> Vec<(u32, Vec<Scope>)> {
    vec![
        (
            7,
            vec![
                fs_read("/tmp"),
                Scope::NetHttps {
                    domain: "api.example.com".into(),
                },
                Scope::SubSpiritSpawn {
                    class: "worker".into(),
                },
            ],
        ),
        (
            8,
            vec![Scope::ProcExec {
                binary: "/bin/ls".into(),
            }],
        ),
    ]
}

fn requests() -> Vec<(u32, Scope)> {
    let net = |d: &str| Scope::NetHttps { domain: d.into() };
    let spawn = |c: &str| Scope::SubSpiritSpawn { class: c.into() };
    let exec = |b: &str| Scope::ProcExec { binary: b.into() };
    vec![
        (7, fs_read("/tmp")),
        (7, fs_read("/tmp/work/a.txt")),
        (7, fs_read("/tmp/../etc/shadow")),
        (7, fs_read("/etc/passwd")),
        (
            7,
            Scope::FsWrite {
                subtree: "/tmp".into(),
            },
        ),
        (7, net("api.example.com")),
        (7, net("evil.example.com")),
        (7, spawn("worker")),
        (7, spawn("admin")),
        (7, exec("/bin/sh")),
        (8, exec("/bin/ls")),
        (8, exec("/bin/sh")),
        (8, fs_read("/tmp")),
    ]
}

/// One seeded run against the real registry: every attempt that the registry lets through
/// becomes an effect, tokens it never issued included.
fn generated(seed: u64) -> Vec<(u32, Observation)> {
    cap_tokens::init_monotonic_base();
    let adapter = common::registry::adapter(&manifests());
    let (pool, mut state) = (requests(), seed);
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut observed = Vec::new();
    for _ in 0..ATTEMPTS {
        let (pid, scope) = pool[(next() % pool.len() as u64) as usize].clone();
        let mut effects = Vec::new();
        if next() % 4 == 0 {
            let mut id = [0u8; 16];
            id[..8].copy_from_slice(&next().to_le_bytes());
            let forged = CapabilityToken::new(TokenId(id), pid, u64::MAX, [7u8; 64]);
            if adapter.verify(&forged, POSTURE, SandboxTier(2)).is_ok() {
                effects.push(Effect {
                    requested: scope.clone(),
                    verified: None,
                });
            }
        } else if let Ok(token) =
            adapter.issue(pid, scope.clone(), 60, POSTURE, IntentClass::Standard)
        {
            if adapter.verify(&token, POSTURE, SandboxTier(2)).is_ok() {
                effects.push(Effect {
                    requested: scope.clone(),
                    verified: Some(scope),
                });
            }
        }
        let declared = manifests()
            .into_iter()
            .find_map(|(p, d)| (p == pid).then_some(d))
            .unwrap_or_default();
        observed.push((pid, Observation { declared, effects }));
    }
    observed
}

fn run(corpus: &str) -> Vec<String> {
    let mut failures = Vec::new();
    for (n, line) in corpus.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        match serde_json::from_str::<Observation>(line) {
            Ok(obs) if check(&obs).is_ok() => failures.push(format!("corpus line {n} accepted")),
            Ok(_) => {}
            Err(e) => failures.push(format!("corpus line {n} unparsable: {e}")),
        }
    }
    for seed in SEEDS {
        let observed = generated(seed);
        if !observed.iter().any(|(_, o)| !o.effects.is_empty()) {
            failures.push(format!(
                "seed {seed}: no effect was ever allowed (vacuous run)"
            ));
        }
        for (pid, obs) in observed {
            if let Err(v) = check(&obs) {
                failures.push(format!("seed {seed} pid {pid}: {v}: {obs:?}"));
            }
        }
    }
    failures
}

#[test]
fn i1_no_effect_without_a_verified_in_scope_token() {
    let corpus = include_str!("fixtures/invariants/i1_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
