//! 20-11 L2 round 3: halt provenance is NOT size-capped at construction. A
//! policy halt must never be dropped because its `derived_from` is large (a
//! cold Butler assessment with many overlapping events exceeds 4096 bytes);
//! bounding it needs a durable reference in a later story.
use maos_domain::frame::EpistemicHaltPayload;

#[test]
fn an_oversized_derived_from_still_builds_a_halt() {
    let payload = EpistemicHaltPayload::new(
        "halt-big".into(),
        "belief_variance".into(),
        0.99,
        Some(0.7),
        "policy".into(),
        "x".repeat(200_000),
    )
    .expect("a large provenance never refuses the halt");
    assert_eq!(payload.derived_from.len(), 200_000);
}
