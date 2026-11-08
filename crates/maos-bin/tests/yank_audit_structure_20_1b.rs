#[path = "support/yank_audit.rs"]
mod audit;
fn rejected(source: &str) {
    let report = audit::inspect(source);
    assert!(
        !report.errors.is_empty(),
        "unsafe teardown escaped analyzer: {source}; {report:?}"
    );
}
#[test]
fn qualified_drop_requires_join_p6() {
    rejected("async fn main() { std::mem::drop(audit_tx); }");
}
#[test]
fn wildcard_sender_binding_is_not_a_release_p6() {
    rejected("async fn main() { let _ = audit_tx; }");
}
#[test]
fn helper_sender_move_requires_release_p6() {
    rejected("async fn main() { retain(audit_tx); }");
}
#[test]
fn async_task_cannot_retain_sender_clone_p6() {
    rejected("async fn main() { tokio::spawn(async move { let copy = audit_tx.clone(); }); }");
}
#[test]
fn parenthesized_drain_requires_join_p6() {
    rejected("async fn main() { (audit_writer).await; }");
}
#[test]
fn select_drain_requires_join_p6() {
    rejected("async fn main() { tokio::select! { _ = audit_writer => {} } }");
}
#[test]
fn sibling_join_does_not_authorize_another_arm_p6() {
    rejected("async fn main() { if first { yank_poller_task.shutdown().await; } if second { drop(audit_tx); } }");
}
#[test]
fn supervisor_stop_must_follow_join_p4() {
    rejected("async fn main() { root.stop_and_join().await; yank_poller_task.shutdown().await; drop(audit_tx); }");
}
#[test]
fn unload_must_follow_join_p4() {
    rejected("async fn main() { unload_all_loaded().await; yank_poller_task.shutdown().await; drop(audit_tx); }");
}
#[test]
fn joined_qualified_drop_and_select_are_accounted_p6() {
    let report = audit::inspect("async fn main() { yank_poller_task.shutdown().await; std::mem::drop((audit_tx)); tokio::select! { _ = (audit_writer) => {} } }");
    assert!(report.errors.is_empty(), "{report:?}");
    assert_eq!(report.releases.len(), 1);
    assert_eq!(report.drains.len(), 1);
}
#[test]
fn moving_a_drain_to_another_arm_cannot_hide_behind_equal_totals_p6() {
    let source = include_str!("../src/main.rs");
    let mut relocated = source.replacen("&mut audit_writer", "other_writer", 1);
    relocated
        .push_str("\nasync fn main() { yank_poller_task.shutdown().await; audit_writer.await; }\n");
    let report = audit::inspect(&relocated);
    assert!(report.errors.is_empty(), "{report:?}");
    assert_eq!(report.releases.len(), 7);
    assert_eq!(report.drains.len(), 11);
    assert!(
        !report.has_expected_main_arms(),
        "duplicate serving drain concealed missing shell arm: {report:?}"
    );
}
