#![forbid(unsafe_code)]

//! Story 20-9 AC7/AC8 — the suppression record and the poll outcome the schedule watchdog
//! reports. The row's FR4 disposition is `fr4_classifier_16_2`'s doorbell.

use maos_iac::payload::{PollOutcome, ScheduleSuppressedRecord};

#[test]
fn schedule_suppressed_record_round_trip() {
    for refused_scope in [Some("FsRead { subtree: \"/srv\" }".to_owned()), None] {
        let record = ScheduleSuppressedRecord {
            spirit_id: "butler".into(),
            schedule_id: "morning-digest".into(),
            refused_scope,
            reason: "quota exhausted".into(),
        };
        let json = serde_json::to_string(&record).expect("serialize ScheduleSuppressedRecord");
        let back: ScheduleSuppressedRecord =
            serde_json::from_str(&json).expect("deserialize ScheduleSuppressedRecord");
        assert_eq!(back, record);
    }
}

#[test]
fn poll_outcome_variants_are_distinct_per_pid_and_schedule() {
    let fired = PollOutcome::Fired(1, "a".into());
    assert_eq!(fired, PollOutcome::Fired(1, "a".into()));
    assert_ne!(fired, PollOutcome::RateRefused(1, "a".into()));
    assert_ne!(fired, PollOutcome::Fired(2, "a".into()));
    assert_ne!(
        PollOutcome::Suppressed(1, "a".into()),
        PollOutcome::PrincipalRevoked(1, "a".into())
    );
}
