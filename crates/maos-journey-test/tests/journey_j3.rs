#![forbid(unsafe_code)]

use maos_journey_test::{AuditDb, JourneyWorld, MockMcp, Pty};

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
fn j3_day_30_digest_renders_and_persists_via_real_daemon() {
    let captured = workspace_root().join("crates/maos-journey-test/fixtures/j3/day-30-raw.json");
    let audit = AuditDb::temp();
    std::fs::copy(&captured, audit.path().join("j3-digest-inputs.json"))
        .expect("captured J3 raw-input fixture must exist");
    let tl_path = audit.transparency_log_path();
    let untouched_mcp = MockMcp::from_responses(vec!["{}".into()]);
    let world = JourneyWorld::builder()
        .mcp("calendar", untouched_mcp)
        .audit(audit)
        .build();

    let manifest = workspace_root().join("spirits/digest/manifest.toml");
    let cmd = format!("{} run {} --once", maos_bin(), manifest.display());
    let pty = Pty::spawn(&cmd, &world);
    let status = pty.wait();
    assert!(
        status.map(|value| value.success()).unwrap_or(false),
        "maos run digest --once should exit 0; screen:\n{}",
        pty.screen().text()
    );

    let screen = pty.screen();
    for clause in [
        "Overnight, 8 agents ran.",
        "47 IAC frames exchanged.",
        "3 agents halted, 0 acted invisibly.",
        "2 cross-agent consultations resolved without escalation.",
        "1 architectural conflict surfaced for review.",
    ] {
        assert!(
            screen.contains(clause),
            "PTY screen missing `{clause}`; got:\n{}",
            screen.text()
        );
    }

    assert!(
        world.mcp("calendar").unwrap().writes().is_empty(),
        "read-only Digest must not issue hidden MCP writes"
    );

    let rows = maos_audit::query(&tl_path, maos_audit::AuditFilter::default())
        .expect("J3 transparency log query");
    assert!(
        rows.iter().any(|row| row.kind == "distillate"),
        "J3 run must persist a real I11 distillate; rows={rows:?}"
    );
    assert!(
        rows.iter()
            .filter(|row| row.intent == "cohort:digest-ingestion")
            .count()
            >= 14,
        "J3 run must journal every consented raw input before citation"
    );
}

fn derive_captured(bytes: &[u8]) -> maos_digest::TeamDigest {
    let raw: maos_digest::RawDigestInputs =
        serde_json::from_slice(bytes).expect("captured fixture decodes as raw inputs");
    maos_digest::derive_team_digest(&raw).expect("captured raw inputs derive")
}

fn vt100_line(narrative: &str) -> String {
    let mut parser = vt100::Parser::new(3, 240, 0);
    parser.process(narrative.as_bytes());
    parser.screen().contents()
}

/// Render the J3 digest by spawning the real `maos run` daemon against the
/// supplied raw-input bytes, returning the vt100 screen text.
fn render_digest_screen(raw: &[u8]) -> String {
    let audit = AuditDb::temp();
    std::fs::write(audit.path().join("j3-digest-inputs.json"), raw)
        .expect("write J3 raw-input fixture for daemon render");
    let untouched_mcp = MockMcp::from_responses(vec!["{}".into()]);
    let world = JourneyWorld::builder()
        .mcp("calendar", untouched_mcp)
        .audit(audit)
        .build();
    let manifest = workspace_root().join("spirits/digest/manifest.toml");
    let cmd = format!("{} run {} --once", maos_bin(), manifest.display());
    let pty = Pty::spawn(&cmd, &world);
    let status = pty.wait();
    let screen = pty.screen();
    assert!(
        status.map(|value| value.success()).unwrap_or(false),
        "maos run digest --once should exit 0; screen:\n{}",
        screen.text()
    );
    screen.text().to_string()
}

#[test]
fn j3_anti_canned_blinded_raw_input_moves_rendered_line() {
    let captured_path =
        workspace_root().join("crates/maos-journey-test/fixtures/j3/day-30-raw.json");
    let captured = std::fs::read(captured_path).expect("captured J3 fixture");

    // AC5: the one derivation is input-sensitive — a blinded raw-input byte
    // moves the vt100 line it renders.
    let original_line = vt100_line(&derive_captured(&captured).narrative);
    let needle = b"\"frames\": 5";
    let offset = captured
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("fixture contains a single-byte-blindable summary count");
    let mut blinded = captured.clone();
    blinded[offset + needle.len() - 1] = b'6';
    let changed_line = vt100_line(&derive_captured(&blinded).narrative);
    assert!(original_line.contains("47 IAC frames exchanged."));
    assert!(changed_line.contains("48 IAC frames exchanged."));
    assert_ne!(
        original_line, changed_line,
        "one blinded captured raw-input byte must move the vt100 render"
    );

    // §A7 reflex: the same blinded byte must move the REAL daemon's render.
    // A static/canned daemon narrative decoupled from `derive_team_digest`
    // would not move and would red here.
    let original_screen = render_digest_screen(&captured);
    let changed_screen = render_digest_screen(&blinded);
    assert!(
        original_screen.contains("47 IAC frames exchanged."),
        "daemon must render the captured frame count; got:\n{original_screen}"
    );
    assert!(
        changed_screen.contains("48 IAC frames exchanged."),
        "blinded daemon render must reflect the moved frame count; got:\n{changed_screen}"
    );
    assert_ne!(
        original_screen, changed_screen,
        "one blinded byte must move the real daemon's vt100 render — a static digest reds"
    );
}
