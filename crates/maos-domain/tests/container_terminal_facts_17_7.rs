use maos_domain::{
    sandbox::{
        cgroup_names_container, parse_container_id, parse_pidfile, ContainerExitRecord,
        ContainerResidue, ContainerState, ContainerTerminalFacts, PayloadExit, PayloadExitSource,
        TerminalUnavailable,
    },
    supervision::{CrashCause, FaultCause},
};
use serde_json::{json, Value};

fn state(code: Option<i32>, oom: bool) -> ContainerState {
    ContainerState {
        status: "exited".into(),
        exit_code: code,
        oom_marker: oom,
        error_reported: false,
        started_at: "0001-01-01T00:00:00Z".into(),
        finished_at: "0001-01-01T00:00:00Z".into(),
    }
}
fn facts(code: Option<i32>, oom: bool, payload: Option<PayloadExit>) -> ContainerTerminalFacts {
    ContainerTerminalFacts::new(Ok(state(code, oom)), payload)
}
fn fault(cause: CrashCause) -> FaultCause {
    match cause {
        CrashCause::Fault(f) => f,
        CrashCause::Voluntary => panic!("expected fault"),
        _ => panic!("unrecognized crash cause"),
    }
}

#[test]
fn unavailable_state_is_always_truncated_even_with_clean_runtime_client() {
    for reason in [
        TerminalUnavailable::InspectFailed,
        TerminalUnavailable::InspectTimedOut,
        TerminalUnavailable::MalformedState,
        TerminalUnavailable::NonTerminal,
        TerminalUnavailable::DeadlineExpired,
    ] {
        let f = ContainerTerminalFacts::new(Err(reason), None);
        assert!(matches!(
            fault(f.classify(None)),
            FaultCause::Truncated { .. }
        ));
        assert_eq!(f.completion_exit_code(), None);
    }
}
#[test]
fn payload_classification_obeys_oom_authority_and_fallback() {
    assert_eq!(
        facts(Some(0), false, Some(PayloadExit::Exited(0))).classify(None),
        CrashCause::Voluntary
    );
    assert_eq!(
        facts(Some(0), true, Some(PayloadExit::Exited(0))).classify(None),
        CrashCause::Voluntary
    );
    assert!(matches!(
        fault(facts(Some(9), false, Some(PayloadExit::Exited(9))).classify(None)),
        FaultCause::NonZeroExit { code: 9, .. }
    ));
    let oom = facts(Some(137), true, Some(PayloadExit::Signaled(9))).classify(None);
    assert_eq!(oom.exit_signal(), Some(9));
    assert_eq!(oom.exit_code(), None);
    assert_eq!(oom.as_str(), "fault.oom_killed");
    assert!(matches!(
        fault(facts(Some(143), false, Some(PayloadExit::Signaled(15))).classify(None)),
        FaultCause::SignaledByKernel { signal: 15, .. }
    ));
    assert!(matches!(
        fault(facts(Some(137), true, None).classify(None)),
        FaultCause::OomKilled { signal: None, .. }
    ));
    assert!(matches!(
        fault(facts(Some(129), false, None).classify(None)),
        FaultCause::NonZeroExit { code: 129, .. }
    ));
    assert!(matches!(
        fault(facts(Some(192), false, None).classify(None)),
        FaultCause::NonZeroExit { code: 192, .. }
    ));
    assert_eq!(
        facts(Some(0), true, Some(PayloadExit::Exited(0))).classify(None),
        CrashCause::Voluntary
    );
    assert!(matches!(
        fault(facts(None, true, None).classify(None)),
        FaultCause::Truncated { .. }
    ));
}
#[test]
fn decoding_boundaries_and_pidfd_authority() {
    for (code, expected, source) in [
        (0, PayloadExit::Exited(0), PayloadExitSource::Runtime),
        (128, PayloadExit::Exited(128), PayloadExitSource::Runtime),
        (129, PayloadExit::Unknown, PayloadExitSource::None),
        (192, PayloadExit::Unknown, PayloadExitSource::None),
        (193, PayloadExit::Exited(193), PayloadExitSource::Runtime),
        (255, PayloadExit::Exited(255), PayloadExitSource::Runtime),
        (-1, PayloadExit::Unknown, PayloadExitSource::None),
        (256, PayloadExit::Unknown, PayloadExitSource::None),
    ] {
        let f = ContainerTerminalFacts::new(Ok(state(Some(code), false)), None);
        assert_eq!((f.payload, f.source), (expected, source), "code={code}");
    }
    let f = ContainerTerminalFacts::new(Ok(state(Some(-1), false)), Some(PayloadExit::Exited(7)));
    assert_eq!(
        (f.payload, f.source),
        (PayloadExit::Exited(7), PayloadExitSource::Pidfd)
    );
    assert_eq!(
        ContainerTerminalFacts::new(Ok(state(None, false)), None).payload,
        PayloadExit::Unknown
    );
}

#[test]
fn invalid_runtime_exit_codes_are_truncated_without_invented_code_or_signal() {
    for code in [None, Some(-1), Some(-42), Some(256)] {
        let f = ContainerTerminalFacts::new(Ok(state(code, false)), None);
        let cause = f.classify(None);
        assert!(matches!(fault(cause.clone()), FaultCause::Truncated { .. }));
        assert_eq!(cause.exit_code(), None);
        assert_eq!(cause.exit_signal(), None);
    }
}
#[test]
fn wait_status_decode_is_terminal_only() {
    for (raw, expected) in [
        (0x8900, PayloadExit::Exited(137)),
        (9, PayloadExit::Signaled(9)),
        (0, PayloadExit::Exited(0)),
    ] {
        assert_eq!(PayloadExit::from_wait_status(raw), Some(expected));
    }
    assert_eq!(PayloadExit::from_wait_status(0x137f), None);
}
#[test]
fn completion_requires_available_exited_payload() {
    assert_eq!(
        facts(Some(0), false, Some(PayloadExit::Exited(0))).completion_exit_code(),
        Some(0)
    );
    assert_eq!(
        facts(Some(1), false, Some(PayloadExit::Exited(1))).completion_exit_code(),
        Some(1)
    );
    assert_eq!(facts(Some(129), false, None).completion_exit_code(), None);
    assert_eq!(
        facts(Some(0), false, Some(PayloadExit::Unknown)).completion_exit_code(),
        None
    );
    assert_eq!(
        facts(Some(0), false, Some(PayloadExit::Signaled(9))).completion_exit_code(),
        None
    );
    assert_eq!(
        ContainerTerminalFacts::new(
            Err(TerminalUnavailable::InspectFailed),
            Some(PayloadExit::Exited(0))
        )
        .completion_exit_code(),
        None
    );
}
fn inspect(status: &str, code: Value) -> Value {
    json!({"Status":status,"ExitCode":code,"OOMKilled":false,"Error":"","StartedAt":"0001-01-01T00:00:00Z","FinishedAt":"0001-01-01T00:00:00Z"})
}
#[test]
fn inspect_parser_requires_all_typed_state_fields_and_terminality_is_exact() {
    for status in ["exited", "stopped"] {
        let v = inspect(status, json!(0));
        assert!(ContainerState::from_inspect_json(
            &serde_json::to_vec(&json!({"State":v})).unwrap()
        )
        .is_none());
        let parsed = ContainerState::from_inspect_json(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(parsed.is_terminal());
        assert!(!parsed.error_reported);
    }
    for status in ["created", "running", "paused", "stopping"] {
        assert!(!ContainerState::from_inspect_json(
            &serde_json::to_vec(&inspect(status, json!(0))).unwrap()
        )
        .unwrap()
        .is_terminal());
    }
    let valid = inspect("exited", json!(0));
    for key in [
        "Status",
        "ExitCode",
        "OOMKilled",
        "Error",
        "StartedAt",
        "FinishedAt",
    ] {
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(key);
        assert!(
            ContainerState::from_inspect_json(&serde_json::to_vec(&missing).unwrap()).is_none(),
            "missing {key}"
        );
        let mut mistyped = valid.clone();
        mistyped[key] = json!(null);
        assert!(
            ContainerState::from_inspect_json(&serde_json::to_vec(&mistyped).unwrap()).is_none(),
            "mistyped {key}"
        );
    }
    let mut nonempty_error = valid.clone();
    nonempty_error["Error"] = json!("secret diagnostic");
    assert!(
        ContainerState::from_inspect_json(&serde_json::to_vec(&nonempty_error).unwrap())
            .unwrap()
            .error_reported
    );
    let mut huge = valid.clone();
    huge["ExitCode"] = json!(i64::MAX);
    assert_eq!(
        ContainerState::from_inspect_json(&serde_json::to_vec(&huge).unwrap())
            .unwrap()
            .exit_code,
        None
    );
    let mut float = valid;
    float["ExitCode"] = json!(1.0);
    assert!(ContainerState::from_inspect_json(&serde_json::to_vec(&float).unwrap()).is_none());
}
#[test]
fn pidfile_parser_accepts_only_positive_i32_decimal() {
    assert_eq!(parse_pidfile(b"42"), Some(42));
    assert_eq!(parse_pidfile(b"42\n"), Some(42));
    for bytes in [
        &b"0"[..],
        b"+1",
        b"2147483648",
        b"42\n7",
        b"abc",
        b"12345678901",
    ] {
        assert_eq!(parse_pidfile(bytes), None, "{bytes:?}");
    }
}
#[test]
fn cid_parser_and_cgroup_identity_are_exact() {
    let cid = "a".repeat(64);
    assert_eq!(parse_container_id(cid.as_bytes()), Some(cid.clone()));
    assert_eq!(
        parse_container_id(format!("{cid}\n").as_bytes()),
        Some(cid.clone())
    );
    assert_eq!(parse_container_id("A".repeat(64).as_bytes()), None);
    assert_eq!(parse_container_id("a".repeat(63).as_bytes()), None);
    assert_eq!(parse_container_id("a".repeat(65).as_bytes()), None);
    assert_eq!(parse_container_id(format!("{cid}x").as_bytes()), None);
    for cgroup in [
        format!("0::/user.slice/libpod-{cid}.scope"),
        format!("0::/libpod-{cid}"),
        format!("0::/libpod-payload-{cid}"),
    ] {
        assert!(cgroup_names_container(&cgroup, &cid));
    }
    assert!(!cgroup_names_container(
        &format!("0::/libpod-{}", "b".repeat(64)),
        &cid
    ));
    assert!(!cgroup_names_container(&format!("0::/libpod-{cid}x"), &cid));
}
fn record(
    code: Option<i32>,
    oom: bool,
    payload: PayloadExit,
    source: PayloadExitSource,
    verdict: Result<(), ContainerResidue>,
) -> ContainerExitRecord {
    let mut f = facts(code, oom, Some(payload));
    f.source = source;
    f.payload = payload;
    ContainerExitRecord::new(&f, verdict)
}
#[test]
fn exit_record_serializes_exact_six_keys_for_success_fallback_unavailable_and_residue() {
    let cases = [
        record(
            Some(0),
            false,
            PayloadExit::Exited(0),
            PayloadExitSource::Pidfd,
            Ok(()),
        ),
        // Fallback rows come from the real constructor, never a hand-set pair.
        ContainerExitRecord::new(&facts(Some(0), false, None), Ok(())),
        ContainerExitRecord::new(&facts(Some(137), false, None), Ok(())),
        record(
            None,
            false,
            PayloadExit::Unknown,
            PayloadExitSource::None,
            Ok(()),
        ),
        record(
            Some(137),
            true,
            PayloadExit::Signaled(9),
            PayloadExitSource::Pidfd,
            Err(ContainerResidue::StillExists),
        ),
    ];
    for item in cases {
        assert!(item.is_consistent(), "{item:?}");
        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 6);
        for key in [
            "runtime_exit_code",
            "oom_marker",
            "payload_exit",
            "payload_exit_source",
            "removed",
            "residue",
        ] {
            assert!(value.get(key).is_some());
        }
        assert_eq!(
            serde_json::from_value::<ContainerExitRecord>(value).unwrap(),
            item
        );
    }
    let mut unavailable =
        ContainerTerminalFacts::new(Err(TerminalUnavailable::InspectFailed), None);
    unavailable.source = PayloadExitSource::None;
    let row = ContainerExitRecord::new(&unavailable, Ok(()));
    assert_eq!(row.runtime_exit_code, None);
    assert_eq!(row.oom_marker, None);
    assert!(row.is_consistent());
}
#[test]
fn exit_record_rejects_each_cross_field_inconsistency() {
    let good = record(
        Some(0),
        false,
        PayloadExit::Exited(0),
        PayloadExitSource::Pidfd,
        Ok(()),
    );
    let mut bad = good.clone();
    bad.payload_exit_source = PayloadExitSource::None;
    assert!(!bad.is_consistent());
    let mut bad = good.clone();
    bad.payload_exit_source = PayloadExitSource::Runtime;
    bad.runtime_exit_code = Some(1);
    assert!(!bad.is_consistent());
    let mut bad = good.clone();
    bad.payload_exit = "garbage".into();
    assert!(!bad.is_consistent());
    let mut bad = good.clone();
    bad.payload_exit = "signaled(sig=0)".into();
    assert!(!bad.is_consistent());
    let mut bad = good.clone();
    bad.payload_exit = "signaled(sig=65)".into();
    bad.payload_exit_source = PayloadExitSource::Pidfd;
    assert!(!bad.is_consistent());
    let mut bad = good.clone();
    bad.removed = false;
    assert!(!bad.is_consistent());
    // 128+N runtime codes are never runtime-decoded payload exits.
    let mut bad = good.clone();
    bad.payload_exit = "exited(code=137)".into();
    bad.payload_exit_source = PayloadExitSource::Runtime;
    bad.runtime_exit_code = Some(137);
    assert!(!bad.is_consistent());
    let mut bad = good.clone();
    bad.payload_exit = "exited(code=+1)".into();
    assert!(!bad.is_consistent());
    let mut bad = good;
    bad.residue = Some(ContainerResidue::RemoveFailed);
    assert!(!bad.is_consistent());
}
