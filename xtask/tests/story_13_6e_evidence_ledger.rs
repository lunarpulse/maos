#![forbid(unsafe_code)]

//! Story 13.6e (AC5, T10) — the oracle behind
//! `check-multi-tenant-loom`'s `kernel-collective-cause-distinguishable` leg.
//!
//! # Why this file exists
//!
//! Before this story, `check_multi_tenant_loom.rs` carried a 12-line doc comment
//! on an EMPTY `ABSENT_SUCCESSORS` const naming two controls "owned by Story
//! 13.6 to judge" — the only in-code record of that ownership, read by nothing.
//! Story 13.6e derives `ABSENT_SUCCESSORS` from the legs that came back
//! `ABSENT`, so deleting that comment without a leg would have deleted the
//! record. This is the leg's oracle.
//!
//! # The hand-off, mechanically
//!
//! The gate's substrate probe is "does the kernel distinguish the five
//! collective causes?" — answered by reading the kernel source. While the
//! collapse site below is still there the probe is FALSE, the leg is `ABSENT`,
//! and the derived successor list names the control and its owner. When Story
//! 13.6 lands the kernel widening the probe flips, the gate RUNS this test, and
//! 13.6 must make it pass. The successor then disappears because a leg proved
//! it — not because someone deleted a string.
//!
//! ⚠ ONE `#[test]` in this file, addressed `--exact` (trap 10): the gates' only
//! anti-vacuity oracle is `"running 1 test"` + `"1 passed"`, which a second test
//! behind the same leg name would defeat.

/// The kernel file that flattens every collective refusal cause.
const KERNEL_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../crates/maos-kernel-core/src/memory/mod.rs"
);

/// The exact mapping arm that erases the cause: every `Transport(_)` payload —
/// which is what a tenant-wall refusal surfaces as — collapses to one
/// `CollectiveErrorKind`, so "wrong team", "unregistered pid", "forged stamp",
/// "partitioned principal" and "erased row" become indistinguishable to the
/// operator. Kept as a literal on purpose: this string IS the substrate probe
/// `check_multi_tenant_loom::kernel_distinguishes_collective_causes` reads, and
/// the two must not drift.
const COLLAPSE_MARKER: &str = "CollectivePortError::Transport(_) => CollectiveErrorKind::Transport";

fn transport_cause_mapping_counts(source: &str) -> (usize, usize) {
    let compact: String = source.chars().filter(|ch| !ch.is_whitespace()).collect();
    let outer_marker = "CollectivePortError::Transport(";
    let next_outer_marker = "CollectivePortError::";
    let cause_marker = "TransportCause::";
    let output_marker = "=>CollectiveErrorKind::";
    let mut causes = std::collections::HashSet::new();
    let mut outputs = std::collections::HashSet::new();
    let mut outer_cursor = compact.as_str();

    while let Some(outer_start) = outer_cursor.find(outer_marker) {
        let arm = &outer_cursor[outer_start + outer_marker.len()..];
        let arm_end = arm.find(next_outer_marker).unwrap_or(arm.len());
        let mut cause_cursor = &arm[..arm_end];
        while let Some(cause_start) = cause_cursor.find(cause_marker) {
            let after_cause = &cause_cursor[cause_start + cause_marker.len()..];
            let cause = after_cause
                .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                .next()
                .unwrap_or_default();
            let next_cause = after_cause.find(cause_marker).unwrap_or(after_cause.len());
            let mapping = &after_cause[..next_cause];
            if let Some(output_start) = mapping.find(output_marker) {
                let output = &mapping[output_start + output_marker.len()..];
                let output = output
                    .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                    .next()
                    .unwrap_or_default();
                if !cause.is_empty() && !output.is_empty() {
                    causes.insert(cause.to_string());
                    outputs.insert(output.to_string());
                }
            }
            cause_cursor = if next_cause < after_cause.len() {
                &after_cause[next_cause..]
            } else {
                ""
            };
        }
        outer_cursor = &arm[arm_end..];
    }
    (causes.len(), outputs.len())
}

// RED BY DESIGN until Story 13.6 widens the kernel — it is the ABSENT leg's
// inverter, not a claim about HEAD. `#[ignore]` keeps it out of the default
// suite (the house idiom for every substrate-pending oracle in this repo); the
// gate's leg passes `--ignored --exact` and only invokes it once the substrate
// probe says the kernel distinguishes causes.
#[test]
#[ignore = "Blocking successor oracle: invoked exactly once the kernel source probe detects distinct causes"]
fn kernel_collective_cause_is_distinguishable() {
    assert_eq!(
        transport_cause_mapping_counts(
            "CollectivePortError::Transport(error) => CollectiveErrorKind::Transport"
        ),
        (0, 0),
        "binding the opaque payload without discriminating it must stay ABSENT"
    );
    assert_eq!(
        transport_cause_mapping_counts(
            "CollectivePortError::Transport(TransportCause::MapStale { .. }) => \
             CollectiveErrorKind::MapStale, \
             CollectivePortError::Transport(TransportCause::ConsentDenied { .. }) => \
             CollectiveErrorKind::ConsentDenied"
        ),
        (2, 2),
        "two concrete causes mapped to two outputs are discriminating"
    );
    assert_eq!(
        transport_cause_mapping_counts(
            "CollectivePortError::Transport(cause) => match cause { \
             TransportCause::MapStale { .. } => CollectiveErrorKind::MapStale, \
             TransportCause::ConsentDenied { .. } => CollectiveErrorKind::ConsentDenied, \
             }"
        ),
        (2, 2),
        "nested cause matches are discriminating too"
    );
    let source = std::fs::read_to_string(KERNEL_SOURCE)
        .unwrap_or_else(|e| panic!("cannot read {KERNEL_SOURCE}: {e}"));

    assert!(
        !source.contains(COLLAPSE_MARKER),
        "{KERNEL_SOURCE} still collapses every collective refusal cause:\n    \
         {COLLAPSE_MARKER}\n\
         The operator cannot see WHY the wall refused. Widening this is a \
         kernel-core edit plus a FLAG-Winston conversation, owned by Story 13.6. \
         This test is the inverter for `check-multi-tenant-loom`'s \
         `kernel-collective-cause-distinguishable` leg: while the collapse is \
         here the leg is ABSENT and the derived successor list says so; once it \
         is gone the gate runs this test and 13.6 owns making it pass."
    );

    let (cause_count, output_count) = transport_cause_mapping_counts(&source);
    assert!(
        cause_count >= 2 && output_count >= 2,
        "{KERNEL_SOURCE} no longer contains the exact collapse marker, but it \
         does not map at least two concrete `TransportCause` variants to at \
         least two distinct `CollectiveErrorKind` variants \
         (causes={cause_count}, outputs={output_count}). Binding the opaque \
         payload or routing every cause to one kind is still erasure."
    );
}
