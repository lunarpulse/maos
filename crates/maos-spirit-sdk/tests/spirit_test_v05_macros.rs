#![forbid(unsafe_code)]
#![cfg(feature = "spirit_test")]

//! Story 7.1 v0.5 binding — integration tests for the 3 new assertion macros.

use maos_spirit_sdk::{
    local_runner::{MockBusFrame, MockBusFrameKind},
    spirit_test::{self, harness::ExtendedRunReport, HaltResolutionKind},
    Ctx, Spirit,
};

struct TestSpirit;

impl Spirit for TestSpirit {
    fn on_idle(&self, _ctx: &mut Ctx) {
        // noop
    }
}

#[test]
fn assert_passes_silently_on_true() {
    spirit_test::assert!(true, "should never fire");
}

#[test]
#[should_panic(expected = "spirit_test::assert! FAILED")]
fn assert_panics_on_false_with_diagnostics() {
    spirit_test::assert!(false, "diagnostic message");
}

#[test]
fn assert_panics_message_contains_file_line_and_suggested_fix() {
    let result = std::panic::catch_unwind(|| {
        spirit_test::assert!(false, "test diagnostic");
    });
    let err = result.unwrap_err();
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        String::new()
    };
    assert!(
        msg.contains("spirit_test::assert! FAILED"),
        "missing header"
    );
    assert!(msg.contains("condition:"), "missing condition");
    assert!(msg.contains("test diagnostic"), "missing diagnostic");
    assert!(msg.contains("suggested fix:"), "missing suggested fix");
}

#[test]
fn expect_frame_matches_kind_and_bytes() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"introduction: hello world".to_vec(),
        }],
    };
    spirit_test::expect_frame!(
        report,
        kind = MockBusFrameKind::Send,
        bytes_matches = b"introduction:"
    );
}

#[test]
#[should_panic(expected = "spirit_test::expect_frame! FAILED")]
fn expect_frame_panics_on_no_match() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"goodbye".to_vec(),
        }],
    };
    spirit_test::expect_frame!(
        report,
        kind = MockBusFrameKind::Send,
        bytes_matches = b"introduction:"
    );
}

#[test]
fn expect_frame_bytes_exact_matches() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"exact".to_vec(),
        }],
    };
    spirit_test::expect_frame!(report, bytes_exact = b"exact");
}

#[test]
#[should_panic(expected = "spirit_test::expect_frame! FAILED")]
fn expect_frame_bytes_exact_fails_on_mismatch() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"not exact".to_vec(),
        }],
    };
    spirit_test::expect_frame!(report, bytes_exact = b"exact");
}

#[test]
fn expect_frame_multiple_criteria_all_must_match() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::CapInvoke,
            bytes: b"cap:123".to_vec(),
        }],
    };
    spirit_test::expect_frame!(
        report,
        kind = MockBusFrameKind::CapInvoke,
        bytes_matches = b"cap:"
    );
}

#[test]
fn expect_halt_matches_halt_id_and_kind() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![maos_spirit_sdk::spirit_test::HaltResolutionRecord {
            halt_id: "id-1".to_string(),
            kind: HaltResolutionKind::AcceptedHalt,
        }],
        captured_frames: vec![],
    };
    spirit_test::expect_halt!(
        report,
        halt_id = "id-1",
        kind_matches = HaltResolutionKind::AcceptedHalt
    );
}

#[test]
#[should_panic(expected = "spirit_test::expect_halt! FAILED")]
fn expect_halt_panics_on_no_match() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![maos_spirit_sdk::spirit_test::HaltResolutionRecord {
            halt_id: "id-1".to_string(),
            kind: HaltResolutionKind::AcceptedHalt,
        }],
        captured_frames: vec![],
    };
    spirit_test::expect_halt!(
        report,
        halt_id = "id-2",
        kind_matches = HaltResolutionKind::AcceptedHalt
    );
}

#[test]
fn expect_halt_provided_context_discriminant_match() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![maos_spirit_sdk::spirit_test::HaltResolutionRecord {
            halt_id: "id-1".to_string(),
            kind: HaltResolutionKind::ProvidedContext {
                context_bytes: vec![1, 2, 3],
            },
        }],
        captured_frames: vec![],
    };
    // Discriminant match only — inner fields don't need to match at v0.5
    spirit_test::expect_halt!(
        report,
        halt_id = "id-1",
        kind_matches = HaltResolutionKind::ProvidedContext {
            context_bytes: vec![]
        }
    );
}

#[test]
fn macro_gated_behind_spirit_test_feature() {
    // This test file is already gated by #![cfg(feature = "spirit_test")]
    // at the top. Compilation success proves the macros are visible.
}

#[test]
fn alias_assert_resolves_to_underlying_macro() {
    // spirit_test::assert! is an alias for spirit_test_assert!
    // Both should produce identical behavior.
    spirit_test::assert!(true, "alias works");
    maos_spirit_sdk::spirit_test_assert!(true, "direct name works");
}

#[test]
fn v03_macros_still_compile() {
    // Verify the 5 Story 2.4 macros still exist and compile
    let report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![maos_spirit_sdk::spirit_test::HaltResolutionRecord {
            halt_id: "test".to_string(),
            kind: HaltResolutionKind::AcceptedHalt,
        }],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"dummy".to_vec(),
        }],
    };
    // Just verify compilation — these are the v0.3 macros
    maos_spirit_sdk::assert_emits_frame!(&report, |_f: &_| true);
    maos_spirit_sdk::assert_halts_with!(&report, |_k: &_| true);
    maos_spirit_sdk::assert_hook_fired!(&report, "on_idle", 0);
    maos_spirit_sdk::assert_no_capability_invocation!(&report, "scope");
}

#[test]
fn assert_diagnostic_stable_output() {
    let result = std::panic::catch_unwind(|| {
        spirit_test::assert!(1 + 1 == 3, "math is broken");
    });
    let err = result.unwrap_err();
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        String::new()
    };
    assert!(msg.contains("suggested fix:"), "missing suggested fix");
}

#[test]
fn expect_frame_closest_diff_byte_correct() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"abc".to_vec(),
        }],
    };
    let result = std::panic::catch_unwind(|| {
        spirit_test::expect_frame!(report, bytes_matches = b"abx");
    });
    let err = result.unwrap_err();
    let msg = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        String::new()
    };
    // closest_diff_byte should be 2 (0-indexed: 'c' vs 'x')
    assert!(
        msg.contains("last_mismatch_byte: Some(2)"),
        "expected last_mismatch_byte=2, got: {}",
        msg
    );
}

#[test]
fn cross_macro_composition() {
    let mut report = ExtendedRunReport {
        base: Default::default(),
        halt_resolutions: vec![maos_spirit_sdk::spirit_test::HaltResolutionRecord {
            halt_id: "id-1".to_string(),
            kind: HaltResolutionKind::AcceptedHalt,
        }],
        captured_frames: vec![MockBusFrame {
            kind: MockBusFrameKind::Send,
            bytes: b"intro".to_vec(),
        }],
    };
    spirit_test::expect_halt!(
        report,
        halt_id = "id-1",
        kind_matches = HaltResolutionKind::AcceptedHalt
    );
    spirit_test::expect_frame!(
        report,
        kind = MockBusFrameKind::Send,
        bytes_matches = b"intro"
    );
    spirit_test::assert!(report.captured_frames.len() == 1, "exactly one frame");
}
