#![forbid(unsafe_code)]

//! J-Researcher journey tests (JR-1 / JR-2).
//!
//! JR-1: PTY-level deterministic survey — researcher runs without --live,
//!       no network, exits 0 with "deterministic survey" confirmation.
//! JR-2: PTY-level live MCP — researcher runs --live --once with MockMcp
//!       servers for web/arxiv/github/citation-graph, exits 0.

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
fn jr1_deterministic_survey_via_pty() {
    let audit = AuditDb::temp();
    let world = JourneyWorld::builder()
        .audit(audit)
        .build();

    let manifest = workspace_root().join("spirits/researcher/manifest.toml");
    let cmd = format!("{} run {} --once", maos_bin(), manifest.display());
    let pty = Pty::spawn(&cmd, &world);

    let status = pty.wait();
    assert!(
        status.map(|s| s.success()).unwrap_or(false),
        "maos run researcher --once should exit 0"
    );

    let screen = pty.screen();
    assert!(
        screen.contains("deterministic survey"),
        "PTY screen should confirm deterministic survey mode, got:\n{}",
        screen.text()
    );
}

#[test]
fn jr2_live_mcp_fan_out_via_pty() {
    let web_fixture = workspace_root()
        .join("crates/maos-journey-test/fixtures/j-researcher/web-search.json");
    let arxiv_fixture = workspace_root()
        .join("crates/maos-journey-test/fixtures/j-researcher/arxiv-search.json");
    let github_fixture = workspace_root()
        .join("crates/maos-journey-test/fixtures/j-researcher/github-search.json");
    let citation_fixture = workspace_root()
        .join("crates/maos-journey-test/fixtures/j-researcher/citation-graph.json");

    let web_mock = MockMcp::from_fixture(web_fixture.to_str().unwrap());
    let arxiv_mock = MockMcp::from_fixture(arxiv_fixture.to_str().unwrap());
    let github_mock = MockMcp::from_fixture(github_fixture.to_str().unwrap());
    let citation_mock = MockMcp::from_fixture(citation_fixture.to_str().unwrap());

    let audit = AuditDb::temp();
    let cassette = workspace_root()
        .join("crates/maos-journey-test/cassettes/j-researcher/survey-distill.json");
    let world = JourneyWorld::builder()
        .mcp("web", web_mock)
        .mcp("arxiv", arxiv_mock)
        .mcp("github", github_mock)
        .mcp("citation-graph", citation_mock)
        .audit(audit)
        .cassette(cassette.to_str().unwrap())
        .build();

    let manifest = workspace_root().join("spirits/researcher/manifest.toml");
    let cmd = format!("{} run {} --live --once", maos_bin(), manifest.display());
    let pty = Pty::spawn(&cmd, &world);

    let status = pty.wait();
    let screen = pty.screen();
    assert!(
        status.map(|s| s.success()).unwrap_or(false),
        "maos run researcher --live --once should exit 0; screen:\n{}",
        screen.text()
    );

    assert!(
        screen.contains("researcher live MCP port wired"),
        "PTY screen should confirm live MCP wiring, got:\n{}",
        screen.text()
    );
}

#[test]
fn jr_zero_side_effect_deterministic_floor() {
    let audit_db = AuditDb::temp();
    let output = std::process::Command::new(maos_bin())
        .args(["run", &workspace_root().join("spirits/researcher/manifest.toml").to_string_lossy(), "--once"])
        .env("XDG_DATA_HOME", audit_db.path())
        .env("MAOS_HOME", audit_db.path())
        .current_dir(workspace_root())
        .output()
        .expect("failed to spawn researcher deterministic");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "researcher --once should exit 0; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("deterministic survey"),
        "should run in deterministic mode; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("McpInvocation"),
        "deterministic mode should produce zero McpInvocation frames; stderr:\n{stderr}"
    );

    // P9 — Also query the TL to confirm no McpInvocation frames were journaled.
    // Robust against a daemon that silently journals without printing to stderr.
    let tl_path = audit_db.transparency_log_path();
    if tl_path.exists() {
        let conn = rusqlite::Connection::open(&tl_path)
            .expect("open TL for McpInvocation query");
        // FrameKind::McpInvocation = 18 (Story 5.5c)
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM transparency_log WHERE kind = 18",
                [],
                |row| row.get(0),
            )
            .expect("query McpInvocation count");
        assert_eq!(
            count, 0,
            "deterministic mode should produce zero McpInvocation TL rows (kind=18)"
        );
    }
}
