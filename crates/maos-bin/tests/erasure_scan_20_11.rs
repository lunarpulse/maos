#![forbid(unsafe_code)]
//! EC-13 real-store regression controls; no cargo subprocesses.
#[path = "../../../tests/harness/forget_18_6.rs"]
mod h;

#[test]
fn ac5_hold_between_forget_and_late_sweep_preserves_late_frame() {
    let home = tempfile::tempdir().expect("home");
    let stores = h::Stores::open(home.path(), 2011);
    stores.private(7, "late-held@example.org", "private record");
    let forgotten = stores
        .memory
        .forget_with_reason("late-held@example.org", None)
        .expect("first forget");
    stores
        .tl
        .place_legal_hold("late-held@example.org", "litigation", None, 1)
        .expect("intervening hold");
    let late = stores.frame(7, "late-held@example.org", "must survive");
    let before = h::rows(home.path());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("runtime");
    let report = rt.block_on(maos_bin::hook_pump::HookPump::finish_forget(
        &stores.tl, &forgotten, None,
    ));
    let after = h::rows(home.path());
    let id = hex::encode(late);
    assert_eq!(
        before.iter().find(|r| r.0 == id),
        after.iter().find(|r| r.0 == id),
        "late held frame scrubbed"
    );
    assert!(
        report["held_frame_ids"]
            .as_array()
            .is_some_and(|ids| ids.iter().any(|v| v == &id)),
        "hold must be reported: {report}"
    );
}
