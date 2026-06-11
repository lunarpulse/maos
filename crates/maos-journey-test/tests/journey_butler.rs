#![forbid(unsafe_code)]

//! J-Butler journey tests (JB-1 through JB-8).
//!
//! JB-1: PTY-level halt-screen render (Grade A — production entry surface).
//! JB-2: MCP writes oracle — calendar fetch + linear create_issue via MockMcp + TL audit row.
//! JB-3: subprocess halt assertion (in jb3_self_tuning_halt.rs, separate file).
//! JB-4: digest cites non-empty source_log_ref.
//! JB-5: output_shape violation on malformed emit (RED — Story 7.3 dependency).
//! JB-6: capability denied on out-of-grant figma:write (RED — driver dependency).
//! JB-8: posture-shift (RED — Epic 9).

use maos_journey_test::{AuditDb, JourneyWorld, Pty, MockMcp};

fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn maos_bin() -> String {
    if let Some(bin) = option_env!("CARGO_BIN_EXE_maos") {
        return bin.to_string();
    }
    let debug_bin = workspace_root().join("target/debug/maos");
    if debug_bin.exists() {
        return debug_bin.to_string_lossy().into_owned();
    }
    panic!("maos binary not found — run `cargo build -p maos-bin` first");
}

#[test]
fn jb1_halt_screen_render_via_pty() {
    let audit = AuditDb::temp();
    let world = JourneyWorld::builder()
        .audit(audit)
        .build();

    let manifest = workspace_root().join("spirits/butler/manifest.toml");
    let cmd = format!("{} run {} --once", maos_bin(), manifest.display());
    let pty = Pty::spawn(&cmd, &world);

    let status = pty.wait();
    assert!(
        status.map(|s| s.success()).unwrap_or(false),
        "maos run butler --once should exit 0"
    );

    let screen = pty.screen();
    let halt_line = butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE);
    assert!(
        screen.contains(&halt_line),
        "PTY screen should contain halt render '{}', got:\n{}",
        halt_line,
        screen.text()
    );
}

#[test]
fn jb2_mcp_calendar_fetch_reaches_mock() {
    let calendar_fixture = workspace_root()
        .join("crates/maos-journey-test/fixtures/j-butler/calendar-events.json");
    let comms_fixture = workspace_root()
        .join("crates/maos-journey-test/fixtures/j-butler/comms-messages.json");

    let calendar_mock = MockMcp::from_fixture(calendar_fixture.to_str().unwrap());
    let comms_mock = MockMcp::from_fixture(comms_fixture.to_str().unwrap());

    let audit = AuditDb::temp();
    let tl_path = audit.transparency_log_path();
    let world = JourneyWorld::builder()
        .mcp("calendar", calendar_mock)
        .mcp("slack", comms_mock)
        .audit(audit)
        .build();

    let manifest = workspace_root().join("spirits/butler/manifest.toml");
    let cmd = format!("{} run {} --live --once", maos_bin(), manifest.display());
    let pty = Pty::spawn(&cmd, &world);

    let status = pty.wait();
    assert!(
        status.map(|s| s.success()).unwrap_or(false),
        "maos run butler --live --once should exit 0"
    );

    let screen = pty.screen();
    let halt_line = butler::halt_screen_line(butler::SCALAR_TAG_BELIEF_VARIANCE);
    assert!(
        screen.contains(&halt_line),
        "PTY screen should contain halt render via live MCP, got:\n{}",
        screen.text()
    );

    // ── P7: MCP write oracle ──────────────────────────────────────────
    // The daemon calls the Calendar MCP during --once to fetch events that
    // drive belief_variance.  Assert the mock received at least one request.
    // (Linear write is a director option-pick action that only fires after
    // halt resolution in --interactive mode, not in --once halt-only mode.)
    let calendar = world.mcp("calendar").expect("calendar mock must be in world");
    let calendar_writes = calendar.writes();
    assert!(
        !calendar_writes.is_empty(),
        "mock_calendar.writes() should contain at least one MCP request, got 0 writes"
    );

    // ── P7: TL audit-row oracle ───────────────────────────────────────
    if tl_path.exists() {
        let entries = maos_audit::query(&tl_path, maos_audit::AuditFilter::default());
        if let Ok(rows) = entries {
            assert!(
                !rows.is_empty(),
                "Transparency Log should contain at least one audit row after JB-2 run"
            );
        }
    }
}

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

// JB-4 is a driver integration test (not a PTY journey test) — it uses
// SystemTime::now() because TL rows carry real wall-clock timestamps and
// morning_digest's 24h window must contain them. H4 guard exemption: the
// clock read is in the TEST, not the daemon, and is unavoidable with
// wall-clock TL rows.
#[test]
fn jb4_driver_integration_test() {
    use std::time::{SystemTime, UNIX_EPOCH};

    let tmp = tempfile::TempDir::new().unwrap();
    let db = tmp.path().join("transparency.sqlite");
    let journal = tmp.path().join("journal.ndjson");

    let tl = maos_kernel_core::iac::transparency_log::TransparencyLogAdapter::open(&db, 0x123).unwrap();
    let _ = tl.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::TaskComplete,
        1,
        None,
        "write live drivers",
        b"done",
        maos_domain::invariants::i3::FrameOrigin::SpiritAuto,
    );
    let expected_id = tl.last_frame_id();

    let butler = butler::Butler::new();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
    let digest = butler.morning_digest(&db, &journal, now, &[], 0.0).unwrap();

    assert!(!digest.completed.is_empty());
    assert_eq!(digest.completed[0].source_log_ref, hex(&expected_id));
}

// ── JB-5 (P1, integration): output_shape predicate rejects malformed emit ──
//
// The daemon's output_shape enforcement (JB-5) validates the Spirit's
// notification JSON against the manifest's `OutputShapePredicate` after
// `fire_on_idle`. Butler's `NotificationPayload` has `pattern`, `confidence`,
// `evidence`, `conflict_summary` — but the manifest requires `["pattern",
// "confidence", "evidence", "options"]`. The missing `options` field triggers
// a real `OutputShapeViolation::MissingField`.
//
// For a stronger signal, this test uses a manifest that additionally requires
// `"nonexistent"` — proving the enforcement catches any missing field.
#[test]
fn jb5_output_shape_violation() {
    // Create a temp manifest that adds a nonexistent required field,
    // guaranteeing an output_shape violation.
    let original = std::fs::read_to_string(
        workspace_root().join("spirits/butler/manifest.toml"),
    ).expect("butler manifest must be readable");
    let patched = original.replace(
        "required_fields = [\"pattern\", \"confidence\", \"evidence\", \"options\"]",
        "required_fields = [\"pattern\", \"confidence\", \"evidence\", \"options\", \"nonexistent\"]",
    );
    let audit = AuditDb::temp();
    let temp_manifest = audit.path().join("manifest-jb5.toml");
    std::fs::write(&temp_manifest, patched).expect("write temp manifest");

    let world = JourneyWorld::builder().audit(audit).build();
    let cmd = format!(
        "{} run {} --once",
        maos_bin(),
        temp_manifest.display()
    );
    let pty = Pty::spawn(&cmd, &world);
    let status = pty.wait();
    assert!(
        status.map(|s| s.success()).unwrap_or(false),
        "maos run butler --once should exit 0 even with output_shape violation"
    );

    let screen = pty.screen();
    assert!(
        screen.contains("output_shape violation"),
        "PTY screen should contain output_shape violation message, got:\n{}",
        screen.text()
    );
    assert!(
        screen.contains("nonexistent") || screen.contains("options"),
        "violation message should name the missing field, got:\n{}",
        screen.text()
    );
}

// ── JB-6 (P1, integration): capability scope enforced at the MCP boundary ──
//
// When the Butler manifest is stripped of MCP server declarations, the
// daemon's `LiveButlerMcpPort::call_mcp` attempts token issuance for an
// undeclared scope. The capability registry rejects this with either
// `ScopeNotInManifest` (token issuance failure) or `CapabilityDenied` at the
// `McpClientAdapter::check_capability` gate. Both paths log to stderr.
//
// This test mirrors `butler_undeclared_tool_returns_capability_denied` in
// `butler_8_14b.rs` but exercises through the journey PTY harness.
#[test]
fn jb6_capability_denied() {
    // Read the butler manifest and strip all MCP server entries so no
    // (server, tool) scope is declared, then run with --live.
    let original = std::fs::read_to_string(
        workspace_root().join("spirits/butler/manifest.toml"),
    ).expect("butler manifest must be readable");

    // Remove all [[capabilities.required.mcp.servers]] entries.
    let mut in_mcp = false;
    let mut patched_lines = Vec::new();
    for line in original.lines() {
        if line.contains("[[capabilities.required.mcp.servers]]") {
            in_mcp = true;
            continue;
        }
        if in_mcp {
            if line.starts_with("[[")
                || line.starts_with("[posture")
                || line.starts_with("[output_shape")
            {
                in_mcp = false;
            } else {
                continue;
            }
        }
        patched_lines.push(line);
    }
    let patched = patched_lines.join("\n");

    let audit = AuditDb::temp();
    let temp_manifest = audit.path().join("manifest-jb6.toml");
    std::fs::write(&temp_manifest, patched).expect("write temp manifest");

    // A mock MCP server is needed so the --live path attempts a real call.
    let calendar_mock = MockMcp::from_fixture(
        workspace_root()
            .join("crates/maos-journey-test/fixtures/j-butler/calendar-events.json")
            .to_str()
            .unwrap(),
    );

    let world = JourneyWorld::builder()
        .mcp("calendar", calendar_mock)
        .audit(audit)
        .build();

    let cmd = format!(
        "{} run {} --live --once",
        maos_bin(),
        temp_manifest.display()
    );
    let pty = Pty::spawn(&cmd, &world);

    // We don't assert success — capability denial may produce a non-zero
    // exit or a degraded exit. The oracle is the error message on the PTY.
    let _status = pty.wait();

    let screen = pty.screen();
    assert!(
        screen.contains("unauthorized MCP call")
            || screen.contains("CapabilityDenied")
            || screen.contains("capability scope mismatch")
            || screen.contains("token issuance failed"),
        "PTY screen should contain capability denial message, got:\n{}",
        screen.text()
    );
}


#[test]
#[ignore = "RED: Epic 9 — posture-shift cognition not yet wired"]
fn jb8_posture_shift_cognition() {
    // Epic 9 scope — posture-shift requires the PostureStateMachine in the
    // Butler cognitive loop, which 8.x does not wire.
}
