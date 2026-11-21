//! I9 ledger proof: the kernel keeps no persistent state outside the three holders and no
//! unregistered exemptions, persists no secret and holds no statistical state.
//!
//! Real detectors against the real tree and against planted violations: the `check-empty-kernel`
//! gate (persistent structs and the exemption register) on `crates/maos-kernel-core`, on its
//! planted fixtures and on the clean fixture; a structural scan of kernel-core field types for
//! statistical state; and secret canaries written through the real Transparency Log and read
//! back for plaintext. Each scan's report is judged by `invariants::i9::check`, as is the
//! adversarial corpus. No built-binary T3 daemon-admission witness (AC10) exists in this tree; I9 is not
//! claimed on it. Plant: add a `Mutex<HashMap<..>>` field to a kernel
//! struct outside the holders, a histogram field, or stop redacting a canary shape.

use std::path::Path;
use std::process::Command;

use maos_domain::invariants::i3::FrameOrigin;
use maos_domain::invariants::i9::{check, Observation, Scan};
use maos_iac::adapter::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use quote::ToTokens;

const SEEDS: [u64; 4] = [9, 0x9E9, 0xE0F1, 909];
const STATISTICAL: [&str; 8] = [
    "Histogram",
    "Ewma",
    "Percentile",
    "RunningMean",
    "Quantile",
    "Reservoir",
    "Welford",
    "TDigest",
];
const CANARIES: [&[u8]; 6] = [
    b"sk-ant-api03-abcdef1234567890abcdef1234567890",
    b"ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefgh1234",
    b"AKIAIOSFODNN7EXAMPLE",
    b"AIzaSyBd2bNf3eT6h8jK9lM0nP1qR4sU5vW7xY8z",
    b"a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
    b"-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\n-----END PRIVATE KEY-----",
];
const WRAPPERS: [(&str, &str); 4] = [
    ("{\"api_key\":\"", "\"}"),
    ("export API_KEY=", "\n"),
    ("Authorization: Bearer ", " HTTP/1.1"),
    ("", " — log entry"),
];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

/// `1` when the real `check-empty-kernel` gate refuses `path`.
fn empty_kernel(path: &str) -> u32 {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "check-empty-kernel",
            "--path",
            path,
            "--whitelist",
            "xtask/i9-whitelist.toml",
            "--denylist",
            "xtask/i9-denylist.toml",
            "--exemptions",
            "docs/invariants/i9-exemptions.md",
        ])
        .current_dir(root())
        .output()
        .expect("xtask should run");
    u32::from(!output.status.success())
}

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn statistical_fields(items: &[syn::Item], hits: &mut u32) {
    for item in items {
        match item {
            syn::Item::Struct(s) => {
                for field in &s.fields {
                    let ty = field.ty.to_token_stream().to_string();
                    *hits += u32::from(STATISTICAL.iter().any(|name| ty.contains(name)));
                }
            }
            syn::Item::Mod(m) => {
                statistical_fields(m.content.as_ref().map_or(&[][..], |c| c.1.as_slice()), hits)
            }
            _ => {}
        }
    }
}

/// Struct fields of statistical type in the production sources under `src`.
fn statistical_state(src: &Path) -> u32 {
    let mut files = Vec::new();
    rust_files(src, &mut files);
    let mut hits = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read source");
        let ast = syn::parse_file(&text).expect("parse source");
        statistical_fields(&ast.items, &mut hits);
    }
    hits
}

fn leaked(blob: &[u8]) -> u32 {
    CANARIES
        .iter()
        .filter(|secret| blob.windows(secret.len()).any(|w| w == **secret))
        .count() as u32
}

/// Seeded wrapped canaries written through the real Transparency Log; plaintext found on disk.
fn persisted_secrets(seed: u64) -> u32 {
    let log = TransparencyLogAdapter::open_in_memory(seed);
    let mut state = seed;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for _ in 0..12 {
        let secret = CANARIES[(next() % CANARIES.len() as u64) as usize];
        let (before, after) = WRAPPERS[(next() % WRAPPERS.len() as u64) as usize];
        let mut payload = before.as_bytes().to_vec();
        payload.extend_from_slice(secret);
        payload.extend_from_slice(after.as_bytes());
        log.insert_frame_event(
            FrameKind::TaskAssign,
            1,
            None,
            "canary",
            &payload,
            FrameOrigin::HumanAuthored,
        );
    }
    log.query_frames(FrameFilter::default())
        .expect("rows")
        .iter()
        .map(|row| leaked(&row.payload_redacted) + leaked(row.intent.as_bytes()))
        .sum()
}

fn scan(subject: &str, planted: u32, reported: u32) -> Scan {
    Scan {
        subject: subject.into(),
        planted,
        reported,
    }
}

fn generated(seed: u64) -> Observation {
    let planted_stats = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(planted_stats.path().join("src")).expect("src");
    std::fs::write(
        planted_stats.path().join("src/lib.rs"),
        "pub struct Latency { pub p99: std::sync::Mutex<Histogram> }\n",
    )
    .expect("planted source");
    Observation {
        scans: vec![
            scan(
                "kernel-core persistent state and exemptions",
                0,
                empty_kernel("crates/maos-kernel-core"),
            ),
            scan(
                "planted persistent state",
                1,
                empty_kernel("xtask/tests/fixtures/violation-i9"),
            ),
            scan(
                "planted unregistered exemption",
                1,
                empty_kernel("xtask/tests/fixtures/exempt-undocumented"),
            ),
            scan(
                "clean fixture",
                0,
                empty_kernel("xtask/tests/fixtures/clean-i9"),
            ),
            scan(
                "kernel-core statistical state",
                0,
                statistical_state(&root().join("crates/maos-kernel-core/src")),
            ),
            scan(
                "planted statistical state",
                1,
                statistical_state(&planted_stats.path().join("src")),
            ),
            scan(
                "secrets persisted through the log",
                0,
                persisted_secrets(seed),
            ),
            scan(
                "planted persisted secret",
                1,
                leaked(b"{\"api_key\":\"AKIAIOSFODNN7EXAMPLE\"}").min(1),
            ),
        ],
    }
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
        if let Err(v) = check(&obs) {
            failures.push(format!("seed {seed}: {v}: {obs:?}"));
        }
    }
    failures
}

#[test]
fn i9_the_kernel_holds_no_stray_state_secret_or_statistic_and_planted_ones_are_caught() {
    let corpus = include_str!("fixtures/invariants/i9_adversarial.jsonl");
    let failures = run(corpus);
    assert!(failures.is_empty(), "{failures:#?}");
}
