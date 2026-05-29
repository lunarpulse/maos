#![forbid(unsafe_code)]

//! Assertion macros — panic with structured diagnostics on failure.
//!
//! Each macro is gated behind `#[cfg(feature = "spirit_test")]` at the
//! module level (callers must enable the feature to access).

/// Assert that the report's captured frames contain at least one frame
/// matching the predicate.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! assert_emits_frame {
    ($report:expr, $predicate:expr) => {{
        let matched: Vec<_> = $report
            .captured_frames
            .iter()
            .filter(|f| $predicate(f))
            .collect();
        ::core::assert!(
            !matched.is_empty(),
            "assert_emits_frame!: no captured frame matched the predicate. \
             captured_frames={:?}",
            $report.captured_frames
        );
    }};
}

/// Assert that the report's halt_resolutions contain at least one resolution
/// matching the kind predicate.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! assert_halts_with {
    ($report:expr, $kind_predicate:expr) => {{
        let matched: Vec<_> = $report
            .halt_resolutions
            .iter()
            .filter(|r| $kind_predicate(&r.kind))
            .collect();
        ::core::assert!(
            !matched.is_empty(),
            "assert_halts_with!: no halt resolution matched the predicate. \
             halt_resolutions={:?}",
            $report.halt_resolutions
        );
    }};
}

/// Assert that a specific hook fired the expected number of times.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! assert_hook_fired {
    ($report:expr, $hook_name:expr, $expected_count:expr) => {{
        let actual = $report
            .base
            .hooks_fired
            .get($hook_name)
            .copied()
            .unwrap_or(0);
        ::core::assert_eq!(
            actual, $expected_count,
            "assert_hook_fired!: hook '{}' fired {} times, expected {}",
            $hook_name, actual, $expected_count
        );
    }};
}

/// Assert that no frame was sent matching the CapInvoke scope.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! assert_no_capability_invocation {
    ($report:expr, $scope:expr) => {{
        use $crate::local_runner::MockBusFrameKind;
        let matched: Vec<_> = $report
            .captured_frames
            .iter()
            .filter(|f| {
                f.kind == MockBusFrameKind::CapInvoke && f.bytes.starts_with($scope.as_bytes())
            })
            .collect();
        ::core::assert!(
            matched.is_empty(),
            "assert_no_capability_invocation!: found {} capability invocations for scope '{}'. \
             matches={:?}",
            matched.len(),
            $scope,
            matched
        );
    }};
}

/// Assert that the manifest self-check report indicates a well-formed manifest.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! assert_manifest_well_formed {
    ($self_check_report:expr) => {{
        ::core::assert!(
            $self_check_report.warnings.is_empty(),
            "assert_manifest_well_formed!: manifest has warnings: {:?}",
            $self_check_report.warnings
        );
        ::core::assert!(
            !$self_check_report.class_name.is_empty(),
            "assert_manifest_well_formed!: class.name is empty"
        );
        ::core::assert!(
            !$self_check_report.forms.is_empty(),
            "assert_manifest_well_formed!: class.forms is empty"
        );
    }};
}

/// Story 7.1 v0.5 binding — compile-time-checked assertion against the Spirit ABI.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! spirit_test_assert {
    ($condition:expr, $diagnostic:expr $(,)?) => {{
        let cond = $condition;
        if !cond {
            panic!(
                "spirit_test::assert! FAILED at {}:{}\n  condition: {}\n  diagnostic: {}\n  suggested fix: read the failure context above; verify the expected hook fired AND emitted the expected frame BEFORE the condition was evaluated.",
                file!(),
                line!(),
                stringify!($condition),
                $diagnostic
            );
        }
    }};
}

/// Story 7.1 v0.5 binding — structured assertion that the report contains
/// at least one frame matching the named criteria.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! spirit_test_expect_frame {
    ($report:expr, kind = $kind:expr $(, bytes_matches = $bytes_matches:expr)? $(, bytes_exact = $bytes_exact:expr)? $(, from_spirit = $from_spirit:expr)? $(,)?) => {{
        let mut criteria = Vec::<String>::new();
        criteria.push(format!("kind={:?}", $kind));
        $( criteria.push(format!("bytes_matches={:?}", $bytes_matches)); )?
        $( criteria.push(format!("bytes_exact={:?}", $bytes_exact)); )?
        $( { let _ = $from_spirit; criteria.push(format!("from_spirit={:?}", $from_spirit)); } )?
        let mut last_mismatch_byte: Option<usize> = None;
        let matched = $report.captured_frames.iter().any(|f| {
            let mut all_match = true;
            if f.kind != $kind { all_match = false; }
            $( if !f.bytes.starts_with($bytes_matches) {
                all_match = false;
                let first_diff = f.bytes.iter().zip($bytes_matches.iter()).position(|(a,b)| a != b);
                if let Some(d) = first_diff { last_mismatch_byte = Some(d); }
            } )?
            $( if f.bytes.as_slice() != $bytes_exact { all_match = false; } )?
            all_match
        });
        if !matched {
            panic!(
                "spirit_test::expect_frame! FAILED at {}:{}\n  criteria: {}\n  captured: {} frames; last_mismatch_byte: {:?}\n  suggested fix: verify the Spirit emits a matching frame via ctx.send(...) BEFORE the hook returns; OR widen the criteria (e.g., use bytes_matches with a shorter prefix); OR call report.captured_frames in your test to inspect the actual frames.",
                file!(),
                line!(),
                criteria.join(" AND "),
                $report.captured_frames.len(),
                last_mismatch_byte
            );
        }
    }};
    ($report:expr, bytes_exact = $bytes_exact:expr $(, from_spirit = $from_spirit:expr)? $(,)?) => {{
        let mut criteria = Vec::<String>::new();
        criteria.push(format!("bytes_exact={:?}", $bytes_exact));
        $( { let _ = $from_spirit; criteria.push(format!("from_spirit={:?}", $from_spirit)); } )?
        let matched = $report.captured_frames.iter().any(|f| {
            f.bytes.as_slice() == $bytes_exact
        });
        if !matched {
            panic!(
                "spirit_test::expect_frame! FAILED at {}:{}\n  criteria: {}\n  captured: {} frames\n  suggested fix: verify the Spirit emits a matching frame via ctx.send(...) BEFORE the hook returns; OR widen the criteria; OR call report.captured_frames in your test to inspect the actual frames.",
                file!(),
                line!(),
                criteria.join(" AND "),
                $report.captured_frames.len()
            );
        }
    }};
    ($report:expr, bytes_matches = $bytes_matches:expr $(,)?) => {{
        let mut last_mismatch_byte: Option<usize> = None;
        let matched = $report.captured_frames.iter().any(|f| {
            if f.bytes.starts_with($bytes_matches) {
                true
            } else {
                let first_diff = f.bytes.iter().zip($bytes_matches.iter()).position(|(a,b)| a != b);
                if first_diff.is_some() {
                    last_mismatch_byte = first_diff;
                }
                false
            }
        });
        if !matched {
            panic!(
                "spirit_test::expect_frame! FAILED at {}:{}\n  criteria: bytes_matches={:?}\n  captured: {} frames; last_mismatch_byte: {:?}\n  suggested fix: verify the Spirit emits a matching frame via ctx.send(...) BEFORE the hook returns; OR widen the criteria (e.g., use bytes_matches with a shorter prefix); OR call report.captured_frames in your test to inspect the actual frames.",
                file!(),
                line!(),
                $bytes_matches,
                $report.captured_frames.len(),
                last_mismatch_byte
            );
        }
    }};
}

/// Story 7.1 v0.5 binding — structured assertion that the report contains
/// a halt resolution matching the named criteria.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! spirit_test_expect_halt {
    ($report:expr, halt_id = $halt_id:expr $(, kind_matches = $kind_matches:expr)? $(,)?) => {{
        use $crate::spirit_test::halt::HaltResolutionKind;
        let mut criteria = Vec::<String>::new();
        criteria.push(format!("halt_id={:?}", $halt_id));
        $( criteria.push(format!("kind_matches={:?}", $kind_matches)); )?
        let matched = $report.halt_resolutions.iter().any(|r| {
            let mut all_match = true;
            if r.halt_id != $halt_id { all_match = false; }
            $(
                match (&r.kind, &$kind_matches) {
                    (HaltResolutionKind::AcceptedHalt, HaltResolutionKind::AcceptedHalt) => {}
                    (HaltResolutionKind::ProvidedContext { .. }, HaltResolutionKind::ProvidedContext { .. }) => {}
                    (HaltResolutionKind::AuthorizedOverride { .. }, HaltResolutionKind::AuthorizedOverride { .. }) => {}
                    _ => { all_match = false; }
                }
            )?
            all_match
        });
        if !matched {
            panic!(
                "spirit_test::expect_halt! FAILED at {}:{}\n  criteria: {}\n  recorded resolutions: {} ({:?})\n  suggested fix: verify the test invokes harness.resolve_halt(halt_id, HaltResolutionKind::...) BEFORE harness.run(); OR widen the criteria.",
                file!(),
                line!(),
                criteria.join(" AND "),
                $report.halt_resolutions.len(),
                $report.halt_resolutions.iter().map(|r| &r.halt_id).collect::<Vec<_>>()
            );
        }
    }};
}

/// Story 7.1 v0.5 binding — assert the run report surfaced zero deprecation
/// warnings.
#[macro_export]
#[cfg(feature = "spirit_test")]
macro_rules! assert_no_deprecations {
    ($report:expr) => {{
        let warnings = &$report.base.deprecation_warnings_surfaced;
        ::core::assert!(
            warnings.is_empty(),
            "assert_no_deprecations! FAILED at {}:{}\n  surfaced: {} warning(s): {:?}\n  suggested fix: migrate off each deprecated surface per its migration_hint.",
            file!(),
            line!(),
            warnings.len(),
            warnings
        );
    }};
}
