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
        assert!(
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
        assert!(
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
        assert_eq!(
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
        assert!(
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
        assert!(
            $self_check_report.warnings.is_empty(),
            "assert_manifest_well_formed!: manifest has warnings: {:?}",
            $self_check_report.warnings
        );
        assert!(
            !$self_check_report.class_name.is_empty(),
            "assert_manifest_well_formed!: class.name is empty"
        );
        assert!(
            !$self_check_report.forms.is_empty(),
            "assert_manifest_well_formed!: class.forms is empty"
        );
    }};
}
