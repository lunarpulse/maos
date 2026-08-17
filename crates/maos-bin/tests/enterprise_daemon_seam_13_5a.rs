#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! Story 13.5a — the source-inspection control over the `cohort-a2a-daemon`
//! enterprise wiring.
//!
//! The runtime legs (`main.rs` `story_13_5a_*`) prove the governance chain is
//! REACHED from a booted daemon. They boot the daemon through
//! `build_cohort_a2a_daemon_runtime` directly, so on their own they cannot see
//! the one line that decides whether a REAL `maos` process ever gets there: the
//! `MAOS_ONE_SHOT=cohort-a2a-daemon` dispatch. Deleting the enterprise argument
//! at that dispatch restores today's HEAD dead-wire while every runtime leg
//! stays green — this file is the control that reds.
//!
//! Modelled on `check_enterprise_identity.rs::run_issuance_bypass_absence_leg`
//! (source inspection that counts `.issue_with_mediation(` call sites) and the
//! 13.5d `story_10_4a_ac1_proven_red.rs` harness.

const MAIN: &str = include_str!("../src/main.rs");
/// j1-crosshost-2b AC1.1 — `issue_enterprise_governed_capability` (and with it the
/// single `.issue_with_mediation(` call) RELOCATED from `main.rs` into the library
/// so `crates/maos-bin/tests/` can name the worker-spawn surface. The
/// bypass-absence invariant is unchanged in substance — *exactly one* direct kernel
/// mint in the composition root — but the composition root is now this PAIR of
/// files, so the count is taken over both. Counting only `main.rs` would read 0 and
/// a second mint added in `worker_spawn.rs` would be invisible.
const WORKER_SPAWN: &str = include_str!("../src/worker_spawn.rs");

/// Return the source of the `{ … }` block that opens at `open_at`, matched by
/// brace depth. Rust string literals in the scanned regions carry no unbalanced
/// braces, so a depth counter is sufficient (the 13.5d harness idiom).
fn block_from(open_at: usize) -> &'static str {
    let bytes = MAIN.as_bytes();
    let start = open_at
        + MAIN[open_at..]
            .find('{')
            .expect("scanned region must open a block");
    let mut depth = 0usize;
    for (offset, byte) in bytes[start..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &MAIN[start..=start + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated block at byte {open_at}");
}

fn find(needle: &str) -> usize {
    MAIN.find(needle)
        .unwrap_or_else(|| panic!("composition root no longer contains `{needle}`"))
}

#[test]
fn story_13_5a_cohort_daemon_dispatch_threads_the_enterprise_runtime() {
    // ── 1. The dispatch. This is the line that was dead-wired at HEAD:
    // `return run_cohort_a2a_daemon(tl, boot_nonce, cohort_daemon).await;`
    // with no enterprise argument at all.
    let dispatch = block_from(find(r#"if mode == "cohort-a2a-daemon""#));
    for required in [
        "build_enterprise_daemon_governance(",
        "shared_journal.as_ref()",
        "enterprise_runtime.as_ref()",
        "enterprise_pdp_runtime.as_ref()",
        "run_cohort_a2a_daemon(",
        "enterprise_daemon_governance,",
        "enterprise_posture_required,",
    ] {
        assert!(
            dispatch.contains(required),
            "the cohort-a2a-daemon dispatch must thread `{required}`; \
             without it the EnterpriseRuntime is constructed and never reached:\n{dispatch}"
        );
    }

    // ── 2. Both daemon entry points carry the posture in their SIGNATURE, so
    // the thread cannot be quietly dropped to a local.
    for signature in [
        "async fn run_cohort_a2a_daemon(",
        "async fn build_cohort_a2a_daemon_runtime(",
    ] {
        let start = find(signature);
        let params = &MAIN[start..start + MAIN[start..].find(')').expect("signature end")];
        assert!(
            params.contains("enterprise_posture_required: bool")
                && params.contains(
                    "enterprise_daemon_governance: Option<Arc<EnterpriseDaemonGovernance>>",
                ),
            "`{signature}` must accept both the required-posture bit and governance; got:\n{params}"
        );
    }

    // ── 3. The builder INSTALLS the governed decorator on the collective-serve
    // port and hands that exact Arc to the transport — a retained-but-uninstalled
    // port would make the runtime legs green while production stayed dead-wired.
    let builder = block_from(find("async fn build_cohort_a2a_daemon_runtime("));
    assert!(
        builder.contains("EnterpriseGovernedDigestReadPort {"),
        "the daemon builder must decorate the digest-read port under an enterprise posture"
    );
    assert!(
        builder.contains("Some(std::sync::Arc::clone(&digest_port)),"),
        "the decorated port must be the one handed to bind_with_cohort_wiring_and_digest"
    );
    assert!(
        builder.contains("digest_port,"),
        "the daemon runtime must retain the installed collective-serve port"
    );

    // ── 4. The governed seam REUSES the 11.4c wrapper. The single
    // `.issue_with_mediation(` call site is the 11.4c bypass-absence invariant:
    // a second one would mean the daemon minted around SSO/PDP. Counted over the
    // composition-root PAIR (`main.rs` + the relocated `worker_spawn.rs`) — see
    // the `WORKER_SPAWN` doc.
    assert_eq!(
        MAIN.matches(".issue_with_mediation(").count()
            + WORKER_SPAWN.matches(".issue_with_mediation(").count(),
        1,
        "the composition root must keep exactly ONE direct kernel mint — the governed wrapper"
    );
    assert!(
        WORKER_SPAWN.contains("pub fn issue_enterprise_governed_capability("),
        "the governed mint wrapper must remain the one that owns that call site"
    );
    let seam = block_from(find("fn govern_collective_read("));
    for required in [
        "issue_enterprise_governed_capability(",
        "Scope::LoomRead",
        "self.at_rest.seal(",
        "forward_audit_to_siem_once(",
    ] {
        assert!(
            seam.contains(required),
            "the daemon governance seam must run `{required}`:\n{seam}"
        );
    }

    // ── 5. Admission reuses the process-wide append-only journal owner. A
    // second independent file cursor can overwrite entries from the first.
    let admission = block_from(find("fn admit_daemon_control_spirit("));
    assert!(
        MAIN.contains("journal: &maos_kernel_core::journal::JournalAdapter")
            && !admission.contains("JournalAdapter::open("),
        "daemon control-Spirit admission must reuse the shared lifecycle journal"
    );

    // ── 6. H3 — the proof path must be the production one. The two test-only
    // helpers with zero production callers stay uncalled by the composition root.
    for forbidden in ["seal_row_at_rest(", "issue_under_principal("] {
        assert_eq!(
            MAIN.matches(forbidden).count(),
            0,
            "the composition root must never reach the zero-production-caller helper `{forbidden}`"
        );
    }

    // ── 6. AC2 — the enterprise posture is NOT a Spirit. No new loaded-Spirit
    // kind, no new classifier arm, no enterprise Spirit crate.
    assert!(
        !MAIN.contains("LoadedSpiritKind::Enterprise"),
        "the enterprise daemon posture must not mint a LoadedSpiritKind variant"
    );
    let workspace_spirits = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spirits");
    let enterprise_crates: Vec<String> = std::fs::read_dir(&workspace_spirits)
        .expect("spirits directory")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("enterprise"))
        .collect();
    assert!(
        enterprise_crates.is_empty(),
        "13.5a must add no enterprise Spirit crate; found {enterprise_crates:?}"
    );

    // ── 7. AC6 / H6 — `identity.asserted` stays an out-of-kernel kind-30 row and
    // the governed record rides an EXISTING kernel FrameKind. A new variant here
    // would be a forbidden kernel-core delta.
    assert!(
        seam.contains("FrameKind::TelemetryEvent"),
        "the governed collective-read record must ride an existing kernel FrameKind"
    );
}
