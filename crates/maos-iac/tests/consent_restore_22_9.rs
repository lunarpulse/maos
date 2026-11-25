//! Story 22-9 (D-22-9-03 A) — `ConsentPolicy::snapshot` / `restore_if_installed`: an
//! auto-revert puts the predecessor's declaration back, but only over the incarnation's own
//! entry, and never over one another pid installed.

use std::collections::BTreeMap;

use maos_domain::frame::ConsentRecall;
use maos_iac::adapter::ConsentPolicy;

fn grants(recall: ConsentRecall) -> BTreeMap<String, ConsentRecall> {
    BTreeMap::from([("peer".to_string(), recall)])
}

#[test]
fn restore_puts_the_predecessors_declaration_back_over_the_successors() {
    let policy = ConsentPolicy::default();
    policy.register("a", 1, Some(grants(ConsentRecall::Full)));
    let prior = policy.snapshot("a");
    policy.register("a", 1, Some(grants(ConsentRecall::Withheld)));

    policy.restore_if_installed("a", 1, prior);

    let (owner, restored) = policy.snapshot("a").expect("declared");
    assert_eq!((owner, &*restored), (1, &grants(ConsentRecall::Full)));
}

#[test]
fn restore_withdraws_a_declaration_the_predecessor_did_not_have() {
    let policy = ConsentPolicy::default();
    let prior = policy.snapshot("a");
    assert!(prior.is_none());
    policy.register("a", 1, Some(grants(ConsentRecall::Withheld)));

    policy.restore_if_installed("a", 1, prior);

    assert!(policy.snapshot("a").is_none());
}

#[test]
fn restore_never_overwrites_another_pids_declaration_or_the_entry_it_would_restore() {
    let policy = ConsentPolicy::default();
    policy.register("a", 1, Some(grants(ConsentRecall::Full)));
    let prior = policy.snapshot("a");

    // The predecessor's own entry is still installed (the successor registered nothing): no-op.
    policy.restore_if_installed("a", 1, prior.clone());
    assert!(std::sync::Arc::ptr_eq(
        &policy.snapshot("a").expect("declared").1,
        &prior.as_ref().expect("snapshot").1
    ));

    // Another incarnation now owns the id: its declaration survives the stale restore.
    policy.register("a", 2, Some(grants(ConsentRecall::HeaderOnly)));
    policy.restore_if_installed("a", 1, prior);
    let (owner, kept) = policy.snapshot("a").expect("declared");
    assert_eq!((owner, &*kept), (2, &grants(ConsentRecall::HeaderOnly)));

    // An id nobody declared is never conjured.
    policy.restore_if_installed("ghost", 1, Some((1, std::sync::Arc::new(BTreeMap::new()))));
    assert!(policy.snapshot("ghost").is_none());
}
