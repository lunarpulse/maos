//! Security review F6: a halt payload's Spirit-supplied strings are capped at
//! 4096 bytes in total (as `RetractPayload`'s reason is capped), so one
//! invocation cannot bloat the log or stall the process-wide halt fence.
use maos_domain::frame::{EpistemicHaltPayload, HaltPayloadError};

fn build(
    halt_id: &str,
    tag: &str,
    policy: &str,
    derived: &str,
) -> Result<EpistemicHaltPayload, HaltPayloadError> {
    EpistemicHaltPayload::new(
        halt_id.into(),
        tag.into(),
        1.0,
        None,
        policy.into(),
        derived.into(),
    )
}

#[test]
fn the_strings_are_capped_at_4096_bytes_in_total() {
    let long = "x".repeat(4097);
    assert!(
        build("h", "t", "p", &"x".repeat(4093)).is_ok(),
        "exactly the cap is accepted"
    );
    for (id, tag, policy, derived) in [
        (long.as_str(), "t", "p", "d"),
        ("h", long.as_str(), "p", "d"),
        ("h", "t", long.as_str(), "d"),
        ("h", "t", "p", long.as_str()),
    ] {
        assert_eq!(
            build(id, tag, policy, derived),
            Err(HaltPayloadError::FieldTooLong)
        );
    }
}
