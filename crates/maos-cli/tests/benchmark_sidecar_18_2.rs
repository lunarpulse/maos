//! Story 18-2 AC5 (D-18-2-04 A) — `maosctl spirit benchmark <class>` reads
//! the signed halt benchmark sidecar from registry storage and prints its
//! body only after the Ed25519 signature verifies (against
//! `MAOS_EVAL_PRODUCER_PUBKEY`, else the documented dev pubkey). A missing
//! sidecar exits 5; a tampered body or a foreign key exits 1, typed.
//! Review (BH-1…BH-4): the signature covers the body's stored bytes (a 14/15
//! float that does not round-trip still verifies — run with `-p maos-cli`
//! alone, whose graph lacks serde_json's `float_roundtrip`); the signer label
//! is bound to the verifying key; class, version and schema must match; a
//! non-UTF-8 key refuses.
//! Process-level: the real `maosctl` binary under a scratch `HOME`.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

use maos_eval::halt_replay::{sidecar, BenchmarkBody, ClassFloors, HaltScore, BENCHMARK_SCHEMA};
use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage};

const PRODUCER_SEED: [u8; 32] = [0x42; 32];

fn body(spirit_version: &str, signer: &str) -> BenchmarkBody {
    BenchmarkBody {
        schema: BENCHMARK_SCHEMA.into(),
        class: "butler".into(),
        spirit_version: spirit_version.into(),
        maos_version: "0.0.0-test".into(),
        corpus: "spirits/butler/tests/fixtures/calendar-comms-v0.3.jsonl".into(),
        corpus_sha256: "ab".repeat(32),
        cassette: Some("crates/maos-journey-test/cassettes/j-butler/halt-corpus-v0.3.json".into()),
        cassette_sha256: Some("cd".repeat(32)),
        provenance: Some("seed".into()),
        completions_consumed: Some(30),
        model_driven: true,
        model_driven_reason: None,
        halt_recall_calendar_conflict_basis: Some(
            "arithmetic: belief_variance (18-1 AC1.1)".into(),
        ),
        score: HaltScore {
            rows: 30,
            expected_halts: 15,
            true_positives: 12,
            false_positives: 0,
            false_negatives: 3,
            halt_recall_calendar_conflict: Some(1.0),
            halt_recall_overall: 0.8,
            halt_precision_overall: 1.0,
        },
        floors: ClassFloors {
            halt_recall_calendar_conflict: Some(0.9),
            halt_recall_overall: 0.9,
            halt_precision_overall: 0.85,
        },
        missed_floors: vec!["halt_recall_overall 0.800 below floor 0.90".into()],
        pass: false,
        signer: signer.into(),
    }
}

fn store(home: &Path, version: &str, bytes: &[u8]) {
    let root = home.join(".local/share/maos/registry");
    LocalFsRegistryStorage::at_path(root)
        .unwrap()
        .put_benchmark(&"butler".into(), version, bytes)
        .unwrap();
}

fn benchmark(home: &Path, args: &[&str], pubkey: Option<&OsStr>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_maosctl"));
    command
        .args(["spirit", "benchmark"])
        .args(args)
        .env("HOME", home)
        .env_remove("MAOS_EVAL_PRODUCER_SEED")
        .env_remove("MAOS_EVAL_PRODUCER_PUBKEY");
    if let Some(pubkey) = pubkey {
        command.env("MAOS_EVAL_PRODUCER_PUBKEY", pubkey);
    }
    command.output().expect("run maosctl spirit benchmark")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_dev_seed_sidecar_verifies_and_prints_its_body() {
    let home = tempfile::TempDir::new().unwrap();
    let signed = sidecar::sign(body("0.3.0", "dev-seed"), &sidecar::DEV_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);

    let output = benchmark(home.path(), &["butler"], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let printed: BenchmarkBody = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(printed, body("0.3.0", "dev-seed"));
    assert!(stderr(&output).contains("signature verified"));
}

#[test]
fn a_tampered_body_exits_one_and_prints_nothing() {
    let home = tempfile::TempDir::new().unwrap();
    let signed = sidecar::sign(body("0.3.0", "dev-seed"), &sidecar::DEV_SEED).unwrap();
    let mut file: serde_json::Value = serde_json::from_slice(&signed).unwrap();
    // Lift the failed run to a pass without re-signing.
    file["body"]["pass"] = true.into();
    file["body"]["missed_floors"] = serde_json::json!([]);
    store(
        home.path(),
        "0.3.0",
        &serde_json::to_vec_pretty(&file).unwrap(),
    );

    let output = benchmark(home.path(), &["butler"], None);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        output.stdout.is_empty(),
        "an unverified body is never printed"
    );
    assert!(
        stderr(&output).contains("sidecar signature verification failed"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_producer_signature_verifies_only_against_the_producer_pubkey() {
    let home = tempfile::TempDir::new().unwrap();
    let signed = sidecar::sign(body("0.3.0", "producer"), &PRODUCER_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);
    let producer_pubkey = hex::encode(maos_audit::sealed_export::derive_pubkey(&PRODUCER_SEED));

    let trusted = benchmark(home.path(), &["butler"], Some(producer_pubkey.as_ref()));
    assert_eq!(trusted.status.code(), Some(0), "{}", stderr(&trusted));
    assert!(stderr(&trusted).contains("against MAOS_EVAL_PRODUCER_PUBKEY — the producer"));

    let dev_key = benchmark(home.path(), &["butler"], None);
    assert_eq!(dev_key.status.code(), Some(1), "{}", stderr(&dev_key));
    assert!(dev_key.stdout.is_empty());
}

#[test]
fn a_malformed_pubkey_refuses_without_echoing_it() {
    let home = tempfile::TempDir::new().unwrap();
    let signed = sidecar::sign(body("0.3.0", "dev-seed"), &sidecar::DEV_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);

    let output = benchmark(home.path(), &["butler"], Some("not-hex-secretish".as_ref()));
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("MAOS_EVAL_PRODUCER_PUBKEY is not 64 hex characters"));
    assert!(!stderr(&output).contains("not-hex-secretish"));
}

#[test]
fn a_missing_sidecar_exits_five_typed() {
    let home = tempfile::TempDir::new().unwrap();
    let output = benchmark(home.path(), &["butler"], None);
    assert_eq!(output.status.code(), Some(5), "{}", stderr(&output));
    assert!(stderr(&output).contains("no benchmark sidecar for spirit 'butler'"));

    let signed = sidecar::sign(body("0.3.0", "dev-seed"), &sidecar::DEV_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);
    let output = benchmark(home.path(), &["butler", "--version", "0.4.0"], None);
    assert_eq!(output.status.code(), Some(5), "{}", stderr(&output));
    assert!(stderr(&output).contains("version '0.4.0'"));
}

#[test]
fn without_a_version_the_highest_benchmarked_version_is_read() {
    let home = tempfile::TempDir::new().unwrap();
    for version in ["0.3.0", "0.10.0", "0.9.1"] {
        let signed = sidecar::sign(body(version, "dev-seed"), &sidecar::DEV_SEED).unwrap();
        store(home.path(), version, &signed);
    }
    let output = benchmark(home.path(), &["butler"], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let printed: BenchmarkBody = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(printed.spirit_version, "0.10.0");

    let pinned = benchmark(home.path(), &["butler", "--version", "0.9.1"], None);
    let printed: BenchmarkBody = serde_json::from_slice(&pinned.stdout).unwrap();
    assert_eq!(printed.spirit_version, "0.9.1");
}

#[test]
fn a_float_that_does_not_round_trip_verifies_byte_exact() {
    let home = tempfile::TempDir::new().unwrap();
    let mut passing = body("0.3.0", "dev-seed");
    passing.score.halt_recall_overall = 14.0 / 15.0;
    passing.missed_floors.clear();
    passing.pass = true;
    let signed = sidecar::sign(passing, &sidecar::DEV_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);

    let output = benchmark(home.path(), &["butler"], None);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"halt_recall_overall\":0.9333333333333333,"),
        "{stdout}"
    );
    assert!(stderr(&output).contains("development seed's key — integrity, not provenance"));
}

#[test]
fn a_dev_seed_signature_claiming_the_producer_exits_one() {
    let home = tempfile::TempDir::new().unwrap();
    let signed = sidecar::sign(body("0.3.0", "producer"), &sidecar::DEV_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);

    let output = benchmark(home.path(), &["butler"], None);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains("sidecar signer is 'producer', expected 'dev-seed'"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_sidecar_of_another_class_version_or_schema_exits_one() {
    let cases: [(fn(&mut BenchmarkBody), &str); 3] = [
        (
            |b| b.class = "researcher".into(),
            "sidecar class is 'researcher', expected 'butler'",
        ),
        (
            |b| b.spirit_version = "0.2.0".into(),
            "sidecar spirit_version is '0.2.0', expected '0.3.0'",
        ),
        (
            |b| b.schema = "maos.benchmark.halt/v0".into(),
            "sidecar schema is 'maos.benchmark.halt/v0'",
        ),
    ];
    for (edit, message) in cases {
        let home = tempfile::TempDir::new().unwrap();
        let mut moved = body("0.3.0", "dev-seed");
        edit(&mut moved);
        let signed = sidecar::sign(moved, &sidecar::DEV_SEED).unwrap();
        store(home.path(), "0.3.0", &signed);

        let output = benchmark(home.path(), &["butler"], None);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{message}: {}",
            stderr(&output)
        );
        assert!(output.stdout.is_empty());
        assert!(stderr(&output).contains(message), "{}", stderr(&output));
    }
}

#[test]
fn a_non_utf8_pubkey_refuses_and_never_falls_back() {
    use std::os::unix::ffi::OsStrExt;
    let home = tempfile::TempDir::new().unwrap();
    let signed = sidecar::sign(body("0.3.0", "dev-seed"), &sidecar::DEV_SEED).unwrap();
    store(home.path(), "0.3.0", &signed);

    let output = benchmark(
        home.path(),
        &["butler"],
        Some(OsStr::from_bytes(b"\xff\xfe")),
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty());
    assert!(stderr(&output).contains("MAOS_EVAL_PRODUCER_PUBKEY is not 64 hex characters"));
}
