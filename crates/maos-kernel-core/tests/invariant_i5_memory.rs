//! I5 ledger proof (FR31 half): memory scopes are kernel-enforced.
//!
//! Seeded private writes, reads and principal-namespace writes by several Spirits run against
//! the real `MemoryManagerAdapter` (principal-write enforcement on); a second boot over the same
//! disk recycles pids onto other identities. Every observed read and write is judged by
//! `invariants::i5::check`, as is the adversarial corpus. The FR13 quarantine half is owned by
//! `later-registry-quarantine-closure`; the ledger entry stays `asserted` until both halves are
//! proven. Plant: key the private store directory by pid instead of the bound identity, or skip
//! the principal-write authorization check.

use std::path::Path;
use std::sync::Arc;

use maos_domain::invariants::i5::{check, Observation, Read, Write};
use maos_domain::memory::{MemoryNamespace, MemoryTier, MemoryValue};
use maos_domain::ports::MemoryManagerPort;
use maos_kernel_core::iac::TransparencyLogAdapter;
use maos_kernel_core::memory::{
    MemoryManagerAdapter, PrincipalNamespaceIndex, PrivateMemoryStore, SharedMemoryStore,
};

const SEEDS: [u64; 4] = [6, 0xFEED, 0x0DDBA11, 31];
const OPS: usize = 48;

/// One boot over the shared disk: `identities[i]` is bound to pid `i + 1`; the first two pids
/// may write principal namespaces.
fn boot(root: &Path, identities: [&str; 3]) -> MemoryManagerAdapter {
    let db = root.join("memory.sqlite");
    let memory = MemoryManagerAdapter::new(
        Arc::new(PrivateMemoryStore::new(root.join("private"), 4096)),
        Arc::new(SharedMemoryStore::open(&db).expect("shared store")),
        Arc::new(PrincipalNamespaceIndex::open(&db).expect("principal index")),
        Arc::new(TransparencyLogAdapter::open_in_memory(0x15)),
    )
    .with_principal_write_enforcement();
    for (i, identity) in identities.iter().enumerate() {
        memory
            .bind_storage_identity(i as u32 + 1, identity)
            .expect("bind");
        if i < 2 {
            memory.authorize_principal_writes(i as u32 + 1);
        }
    }
    memory
}

fn read_of(memory: &MemoryManagerAdapter, pid: u32, reader: &str, key: &str) -> Read {
    let got = memory
        .read(pid, MemoryTier::Private, &MemoryNamespace::Default, key)
        .expect("private read");
    let value_owner = match got {
        Some(MemoryValue::Text(t)) => t.strip_prefix("owner-").map(str::to_string),
        _ => None,
    };
    Read {
        reader: reader.into(),
        value_owner,
    }
}

fn generated(seed: u64) -> Observation {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = ["spirit-a", "spirit-b", "spirit-c"];
    let memory = boot(dir.path(), first);
    let principal = MemoryNamespace::principal("director-ada", "private/v1").expect("namespace");
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let (mut reads, mut writes) = (Vec::new(), Vec::new());
    for _ in 0..OPS {
        let (slot, key) = ((next() % 3) as usize, format!("k{}", next() % 4));
        let pid = slot as u32 + 1;
        match next() % 4 {
            0 => memory
                .write(
                    pid,
                    MemoryTier::Private,
                    &MemoryNamespace::Default,
                    &key,
                    MemoryValue::Text(format!("owner-{}", first[slot])),
                )
                .expect("private write"),
            2 => writes.push(Write {
                writer: first[slot].into(),
                authorized: slot < 2,
                landed: memory
                    .write(
                        pid,
                        MemoryTier::Private,
                        &principal,
                        &key,
                        MemoryValue::Text("p".into()),
                    )
                    .is_ok(),
            }),
            _ => reads.push(read_of(&memory, pid, first[slot], &key)),
        }
    }
    drop(memory);
    // Second boot: pids are recycled onto other identities over the same disk.
    let second = ["spirit-z", "spirit-a", "spirit-b"];
    let memory = boot(dir.path(), second);
    for (slot, reader) in second.iter().enumerate() {
        for key in 0..4 {
            reads.push(read_of(
                &memory,
                slot as u32 + 1,
                reader,
                &format!("k{key}"),
            ));
        }
    }
    Observation { reads, writes }
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
        let obs = generated(seed);
        if !obs.reads.iter().any(|r| r.value_owner.is_some()) {
            failures.push(format!(
                "seed {seed}: no read ever returned a value (vacuous)"
            ));
        }
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i5_private_memory_never_crosses_spirits_and_writes_stay_in_their_namespace() {
    let corpus = include_str!("fixtures/invariants/i5_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
