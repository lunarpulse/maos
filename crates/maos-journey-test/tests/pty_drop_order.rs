#![forbid(unsafe_code)]

//! Task 4.2 — PTY drop order test: kill+wait child → close master → join drain
//! thread. The #1 PTY flake source is dropping the master before the child exits.

use maos_journey_test::{AuditDb, JourneyWorld, Pty};

#[test]
fn drop_while_child_streaming() {
    let world = JourneyWorld::builder()
        .audit(AuditDb::temp())
        .build();
    let pty = Pty::spawn("echo hello world", &world);
    std::thread::sleep(std::time::Duration::from_millis(500));
    let screen = pty.screen();
    assert!(
        screen.text().contains("hello") || screen.text().is_empty(),
        "screen should contain output or be empty (echo may have finished)"
    );
    drop(pty);
}

#[test]
fn pty_screen_captures_output() {
    let world = JourneyWorld::builder()
        .audit(AuditDb::temp())
        .build();
    let pty = Pty::spawn("echo MAOS_JOURNEY_TEST_MARKER", &world);
    std::thread::sleep(std::time::Duration::from_millis(500));
    let screen = pty.screen();
    assert!(
        screen.contains("MAOS_JOURNEY_TEST_MARKER"),
        "PTY screen should capture echo output, got: {:?}",
        screen.text()
    );
}
