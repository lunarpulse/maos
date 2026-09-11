#![forbid(unsafe_code)]

use std::path::PathBuf;

use maos_journey_test::JourneyWorld;

const CHILD_ENV: &str = "MAOS_15_6_RECORDING_SOURCE_CHILD";
const SOURCE_ENV: &str = "MAOS_15_6_RECORDING_SOURCE";

#[test]
fn record_mode_routes_stub_output_to_the_source_cassette() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.json");
    std::fs::write(&source, b"original source bytes").unwrap();

    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "recording_source_child"])
        .env(CHILD_ENV, "1")
        .env(SOURCE_ENV, &source)
        .env("MAOS_INFERENCE_MODE", "record")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "recording child failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(&source).unwrap();
    assert_ne!(bytes, b"original source bytes");
    let cassette: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(cassette["provenance"], "live-record");
    assert_eq!(cassette["entries"][0]["response"]["text"], "stub response");
}

#[test]
#[ignore = "spawned by the parent with an isolated record-mode environment"]
fn recording_source_child() {
    assert_eq!(std::env::var(CHILD_ENV).as_deref(), Ok("1"));
    let source = PathBuf::from(std::env::var_os(SOURCE_ENV).unwrap());
    let world = JourneyWorld::builder()
        .cassette(source.to_str().unwrap())
        .build();
    let selected = PathBuf::from(world.env().get("MAOS_REPLAY_CASSETTE").unwrap());
    assert_eq!(
        selected, source,
        "record mode must target the source cassette"
    );

    // Hermetic stand-in for the paid provider leg: the harness-selected path is
    // the only contract under test here. Provider recording itself is covered
    // by maos-bin's CassetteRecordProvider round-trip test.
    let recorded = serde_json::json!({
        "schema_version": "maos.journey.cassette/v1",
        "provenance": "live-record",
        "entries": [{"response": {"text": "stub response"}}]
    });
    std::fs::write(selected, serde_json::to_vec_pretty(&recorded).unwrap()).unwrap();
}
