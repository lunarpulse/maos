//! Story 9.2b — replay schema validation + determinism tests (AC2).
//!
//! Verifies that `replay()` output validates against `trace-shape.schema.json`
//! and that two replays of the same bundle are byte-identical.

#![forbid(unsafe_code)]

use std::collections::HashSet;
use std::sync::Arc;

use maos_domain::invariants::i3::FrameOrigin;
use tempfile::TempDir;
/// Open an isolated TL + write some frames for testing.
fn setup_test_db() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db_path, 1).unwrap(),
    );

    use maos_kernel_core::iac::transparency_log::FrameKind;
    let cap = [0xAAu8; 32];

    // Insert diverse frame kinds for shape-class coverage
    tl.insert_frame_event(
        FrameKind::TaskAssign,
        1,
        Some(&cap),
        "assign.work",
        b"task payload",
        FrameOrigin::Kernel,
    );
    tl.insert_frame_event(
        FrameKind::CapabilityInvocation,
        1,
        Some(&cap),
        "file.read",
        b"cap payload data",
        FrameOrigin::Kernel,
    );
    tl.insert_frame_event(
        FrameKind::EpistemicHalt,
        1,
        None,
        "halt.confidence",
        b"halt",
        FrameOrigin::Kernel,
    );
    tl.insert_frame_event(
        FrameKind::Decision,
        1,
        Some(&cap),
        "decide.route",
        b"decision data",
        FrameOrigin::Kernel,
    );
    tl.insert_frame_event(
        FrameKind::TelemetryEvent,
        1,
        None,
        "telemetry.ping",
        b"",
        FrameOrigin::Kernel,
    );

    drop(tl);
    (dir, db_path)
}

/// Read the committed trace-shape schema from the workspace `schemas/` dir.
fn trace_shape_schema_path() -> std::path::PathBuf {
    let manifest = std::env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("schemas")
        .join("trace-shape.schema.json")
}

/// Validate a JSON Value against the committed trace-shape schema.
///
/// Hand-rolled because the crate does not depend on a JSON Schema validator.
/// This is intentionally strict for the schema fields the story commits to.
fn validate_trace_shape(value: &serde_json::Value) -> Result<(), String> {
    let schema_bytes = std::fs::read(trace_shape_schema_path())
        .map_err(|e| format!("cannot read trace-shape.schema.json: {e}"))?;
    let schema: serde_json::Value = serde_json::from_slice(&schema_bytes)
        .map_err(|e| format!("cannot parse trace-shape.schema.json: {e}"))?;

    let obj = value.as_object().ok_or("trace-shape must be an object")?;
    let required = schema["required"]
        .as_array()
        .ok_or("schema missing required")?;
    for key in required {
        let key = key.as_str().ok_or("schema required key not string")?;
        if !obj.contains_key(key) {
            return Err(format!("missing required field: {key}"));
        }
    }

    if value["schema_version"] != schema["properties"]["schema_version"]["const"] {
        return Err("schema_version mismatch".to_string());
    }
    if value["determinism_scope"] != schema["properties"]["determinism_scope"]["const"] {
        return Err("determinism_scope mismatch".to_string());
    }

    let hash = value["source_bundle_hash"]
        .as_str()
        .ok_or("source_bundle_hash must be string")?;
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("source_bundle_hash must be 64 hex chars".to_string());
    }

    let frame_count = value["frame_count"]
        .as_u64()
        .ok_or("frame_count must be integer")?;
    let frames = value["frames"].as_array().ok_or("frames must be array")?;
    if frames.len() as u64 != frame_count {
        return Err("frame_count does not match frames.len()".to_string());
    }

    let defs = schema["$defs"].as_object().ok_or("schema missing $defs")?;
    let frame_schema = defs["frame"]
        .as_object()
        .ok_or("schema missing frame def")?;
    let frame_required: HashSet<&str> = frame_schema["required"]
        .as_array()
        .ok_or("frame schema missing required")?
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let valid_classes: HashSet<String> = frame_schema["properties"]["shape_class"]["enum"]
        .as_array()
        .ok_or("shape_class enum missing")?
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    for (i, frame) in frames.iter().enumerate() {
        let f = frame
            .as_object()
            .ok_or_else(|| format!("frame {i} not object"))?;
        for key in &frame_required {
            if !f.contains_key(*key) {
                return Err(format!("frame {i} missing required field {key}"));
            }
        }
        if f.len() != frame_required.len() {
            return Err(format!("frame {i} has extra fields"));
        }
        let class = f["shape_class"]
            .as_str()
            .ok_or_else(|| format!("frame {i} shape_class not string"))?;
        if !valid_classes.contains(class) {
            return Err(format!("frame {i} invalid shape_class: {class}"));
        }
        let ph = &f["placeholder"];
        if !(ph.is_string() || ph.is_null()) {
            return Err(format!("frame {i} placeholder must be string or null"));
        }
    }

    Ok(())
}

#[test]
fn replay_trace_shape_validates_against_schema() {
    let (_dir, db_path) = setup_test_db();

    let entries =
        maos_audit::query_with_redaction(&db_path, maos_audit::AuditFilter::default()).unwrap();

    assert!(!entries.is_empty());

    let shape = maos_audit::replay::replay(&entries, b"test-bundle-bytes").unwrap();
    let val = serde_json::to_value(&shape).unwrap();

    validate_trace_shape(&val).expect("trace-shape must validate against trace-shape.schema.json");
}

const REPLAY_BUNDLE: &str = "two-process-test-bundle";

/// Copy the pristine bundle directory (DB plus any sealed segments) into a
/// fresh temp dir so every replay opens an untouched log.
///
/// Opening the log leaves a `-wal` file beside it, and `query_with_redaction`
/// refuses a log with an active WAL (ADR-028 D6), so replays must not share
/// one directory.
fn pristine_copy(db_path: &std::path::Path) -> (TempDir, std::path::PathBuf) {
    fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let target = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).unwrap();
            }
        }
    }
    let copy = TempDir::new().unwrap();
    copy_dir(db_path.parent().unwrap(), copy.path());
    let copied_db = copy.path().join(db_path.file_name().unwrap());
    (copy, copied_db)
}

/// Run `replay_worker` in a fresh OS process of this very test binary.
///
/// The worker's canonical bytes travel through the file `out`, never through
/// libtest's stdout (which carries timing). `--exact` makes the filter select
/// exactly the `replay_worker` test. Returns the child's exit success.
fn run_replay_worker(db_path: &std::path::Path, out: &std::path::Path, set_db: bool) -> bool {
    let exe = std::env::current_exe().unwrap();
    let mut cmd = std::process::Command::new(exe);
    cmd.args([
        "--exact",
        "replay_worker",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("MAOS_REPLAY_INPUT", REPLAY_BUNDLE)
    .env("MAOS_REPLAY_OUT", out)
    .env_remove("MAOS_REPLAY_DB")
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::piped());
    if set_db {
        cmd.env("MAOS_REPLAY_DB", db_path);
    }
    let output = cmd.output().expect("spawn replay worker");
    if !output.status.success() {
        eprintln!(
            "replay worker failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output.status.success()
}

/// Spawn two OS processes and assert each produces the same replay bytes.
///
/// HashMap iteration order is stable within a process but NOT across processes,
/// so this is the binding determinism gate required by ADR-028 D6.
///
/// The compared bytes are the worker's canonical replay bytes (written to a
/// file), not libtest output. They must be non-empty and equal to an
/// in-process replay of the same bundle, so the test cannot pass vacuously.
#[test]
fn replay_determinism_two_process_byte_identical() {
    let (_dir, db_path) = setup_test_db();
    let out_dir = TempDir::new().unwrap();
    let out1 = out_dir.path().join("replay-out-1.bin");
    let out2 = out_dir.path().join("replay-out-2.bin");

    // A harness-style run (no MAOS_REPLAY_DB) must be a no-op: no output file.
    let noop_out = out_dir.path().join("replay-out-noop.bin");
    assert!(
        run_replay_worker(&db_path, &noop_out, false),
        "replay_worker without MAOS_REPLAY_DB must succeed as a no-op"
    );
    assert!(
        !noop_out.exists(),
        "replay_worker without MAOS_REPLAY_DB must not write output"
    );

    let (_copy1, db1) = pristine_copy(&db_path);
    assert!(run_replay_worker(&db1, &out1, true), "worker 1 failed");
    let (_copy2, db2) = pristine_copy(&db_path);
    assert!(run_replay_worker(&db2, &out2, true), "worker 2 failed");

    let bytes1 = std::fs::read(&out1).expect("worker 1 must write canonical bytes");
    let bytes2 = std::fs::read(&out2).expect("worker 2 must write canonical bytes");

    assert!(
        !bytes1.is_empty(),
        "worker 1 replay bytes must be non-empty"
    );
    assert_eq!(
        bytes1, bytes2,
        "two OS-process replays of the same bundle must produce byte-identical output"
    );

    let (_copy, copy_db) = pristine_copy(&db_path);
    let entries =
        maos_audit::query_with_redaction(&copy_db, maos_audit::AuditFilter::default()).unwrap();
    let shape = maos_audit::replay::replay(&entries, REPLAY_BUNDLE.as_bytes()).unwrap();
    let in_process = maos_audit::replay::runner::replay_to_canonical_bytes(&shape).unwrap();
    assert_eq!(
        bytes1, in_process,
        "worker output must equal the in-process canonical replay bytes"
    );
}

/// Worker entry point used by `replay_determinism_two_process_byte_identical`.
///
/// When run by the normal harness (no `MAOS_REPLAY_DB`) this is a no-op. When
/// spawned by the two-process test (`--exact replay_worker` with
/// `MAOS_REPLAY_DB`, `MAOS_REPLAY_INPUT`, `MAOS_REPLAY_OUT` set), it replays
/// the TL and writes the canonical bytes to the file named by `MAOS_REPLAY_OUT`.
#[test]
fn replay_worker() {
    let Ok(db_path) = std::env::var("MAOS_REPLAY_DB") else {
        return; // normal harness run: nothing to do
    };
    let out_path =
        std::env::var("MAOS_REPLAY_OUT").expect("MAOS_REPLAY_OUT must accompany MAOS_REPLAY_DB");
    let input = std::env::var("MAOS_REPLAY_INPUT").unwrap_or_default();
    let entries = maos_audit::query_with_redaction(
        std::path::Path::new(&db_path),
        maos_audit::AuditFilter::default(),
    )
    .unwrap();
    let shape = maos_audit::replay::replay(&entries, input.as_bytes()).unwrap();
    let bytes = maos_audit::replay::runner::replay_to_canonical_bytes(&shape).unwrap();
    std::fs::write(&out_path, &bytes).unwrap();
}

#[test]
fn replay_one_byte_tamper_diverges() {
    // Anti-tautology: tampering with bundle bytes must produce different hash
    let (_dir, db_path) = setup_test_db();

    let entries =
        maos_audit::query_with_redaction(&db_path, maos_audit::AuditFilter::default()).unwrap();

    let shape_original = maos_audit::replay::replay(&entries, b"original-bundle").unwrap();
    let bytes_original =
        maos_audit::replay::runner::replay_to_canonical_bytes(&shape_original).unwrap();

    // Tamper one byte
    let shape_tampered = maos_audit::replay::replay(&entries, b"original-bundlf").unwrap();
    let bytes_tampered =
        maos_audit::replay::runner::replay_to_canonical_bytes(&shape_tampered).unwrap();

    assert_ne!(
        bytes_original, bytes_tampered,
        "tampered bundle must produce different replay output"
    );

    // Specifically: source_bundle_hash should differ
    assert_ne!(
        shape_original.source_bundle_hash, shape_tampered.source_bundle_hash,
        "source_bundle_hash must differ after tamper"
    );
}

#[test]
fn replay_shape_classes_correct() {
    let (_dir, db_path) = setup_test_db();

    let entries =
        maos_audit::query_with_redaction(&db_path, maos_audit::AuditFilter::default()).unwrap();

    let shape = maos_audit::replay::replay(&entries, b"test").unwrap();

    // Check specific shape class assignments
    let structural: Vec<_> = shape
        .frames
        .iter()
        .filter(|f| f.shape_class == "structural")
        .collect();
    let capability: Vec<_> = shape
        .frames
        .iter()
        .filter(|f| f.shape_class == "capability")
        .collect();
    let halt: Vec<_> = shape
        .frames
        .iter()
        .filter(|f| f.shape_class == "halt")
        .collect();
    let decision: Vec<_> = shape
        .frames
        .iter()
        .filter(|f| f.shape_class == "decision")
        .collect();
    let telemetry: Vec<_> = shape
        .frames
        .iter()
        .filter(|f| f.shape_class == "telemetry")
        .collect();

    assert!(!structural.is_empty(), "should have structural frames");
    assert!(!capability.is_empty(), "should have capability frames");
    assert!(!halt.is_empty(), "should have halt frames");
    assert!(!decision.is_empty(), "should have decision frames");
    assert!(!telemetry.is_empty(), "should have telemetry frames");
}

#[test]
fn replay_with_redaction_produces_placeholders() {
    let (_dir, db_path) = setup_test_db();

    let entries =
        maos_audit::query_with_redaction(&db_path, maos_audit::AuditFilter::default()).unwrap();

    let shape = maos_audit::replay::replay(&entries, b"test").unwrap();

    // Frames whose payload was actually redacted get a placeholder; frames
    // with an empty payload keep a null placeholder.
    let mut saw_placeholder = false;
    for frame in &shape.frames {
        if let Some(ph) = frame.placeholder.as_ref() {
            saw_placeholder = true;
            assert!(ph.starts_with("<REDACTED:"), "placeholder format: {ph}");
            assert!(ph.ends_with('>'), "placeholder format: {ph}");
            assert!(
                !ph.contains("hash"),
                "placeholder must not contain hash: {ph}"
            );
        }
    }
    assert!(
        saw_placeholder,
        "at least one frame must have a placeholder"
    );
}

#[test]
fn verify_trajectory_rejects_open_writer() {
    // ADR-028 D6: deterministic replay/export requires a quiesced DB.
    // With the writer still alive, a SQLite WAL file exists; the read path
    // must reject rather than produce a potentially non-deterministic replay.
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.sqlite");

    let _tl = Arc::new(
        maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db_path, 1).unwrap(),
    );

    let err = maos_audit::query_with_redaction(&db_path, maos_audit::AuditFilter::default())
        .expect_err("query_with_redaction must reject an open writer");

    let msg = format!("{err}");
    assert!(
        msg.contains("WAL") || msg.contains("quiesce") || msg.contains("checkpoint"),
        "error should mention WAL/quiesce/checkpoint: {msg}"
    );
}
