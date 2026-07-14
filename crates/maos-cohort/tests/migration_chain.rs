use maos_cohort::{
    resolve_migration_chain, CohortError, MigrationCandidate, MigrationChainNotLinearReason,
    MigrationPlan,
};

fn candidate(version: &str, migrates_from: &[&str]) -> MigrationCandidate {
    MigrationCandidate::new(
        "marcus-agent",
        version,
        migrates_from.iter().copied().map(str::to_owned).collect(),
    )
}

#[test]
fn resolves_a_linear_multi_hop_chain_in_order() {
    let chain = resolve_migration_chain(
        "1.0",
        "3.0",
        &[candidate("2.0", &["1.0"]), candidate("3.0", &["2.0"])],
    )
    .expect("a linear two-hop candidate set resolves");

    assert_eq!(chain.hops().len(), 2);
    assert_eq!(chain.hops()[0].predecessor_version, "1.0");
    assert_eq!(chain.hops()[0].successor.version, "2.0");
    assert_eq!(chain.hops()[1].predecessor_version, "2.0");
    assert_eq!(chain.hops()[1].successor.version, "3.0");
}

#[test]
fn rejects_wildcard_overlap_as_a_fork_at_the_concrete_source() {
    let error = resolve_migration_chain(
        "1.0",
        "2.0",
        &[candidate("2.0", &["1.0"]), candidate("2.1", &["1.x"])],
    )
    .expect_err("an exact and wildcard declaration both match concrete version 1.0");

    assert!(matches!(
        error,
        CohortError::ECohortMigrationChainNotLinear {
            reason: MigrationChainNotLinearReason::ForkAtSource,
            ref source_version,
        } if source_version == "1.0"
    ));
}

#[test]
fn rejects_a_two_cycle_even_when_every_source_has_one_outgoing_candidate() {
    let error = resolve_migration_chain(
        "1.0",
        "3.0",
        &[candidate("1.0", &["2.0"]), candidate("2.0", &["1.0"])],
    )
    .expect_err("a single-outgoing two-cycle must not be considered linear");

    assert!(matches!(
        error,
        CohortError::ECohortMigrationChainNotLinear {
            reason: MigrationChainNotLinearReason::Cycle,
            ..
        }
    ));
}

#[test]
fn rejects_a_self_loop_before_attempting_a_walk() {
    let error = resolve_migration_chain("1.0", "2.0", &[candidate("2.0", &["2.x"])])
        .expect_err("a successor that migrates from itself is malformed");

    assert!(matches!(
        error,
        CohortError::ECohortMigrationChainNotLinear {
            reason: MigrationChainNotLinearReason::SelfLoop,
            ref source_version,
        } if source_version == "2.0"
    ));
}

#[test]
fn permits_fan_in_and_reports_no_path_only_for_a_well_formed_set() {
    let candidates = [candidate("2.0", &["1.0", "1.1"])];

    let chain = resolve_migration_chain("1.0", "2.0", &candidates)
        .expect("fan-in gives each source exactly one outgoing candidate");
    assert_eq!(chain.hops().len(), 1);

    let error = resolve_migration_chain("1.0", "3.0", &candidates)
        .expect_err("a well-formed but disconnected set has no route");
    assert!(matches!(
        error,
        CohortError::ECohortNoMigrationPath { ref from, ref to }
            if from == "1.0" && to == "3.0"
    ));
}

#[test]
fn persisted_plan_hash_refuses_a_rederived_drifted_chain() {
    let approved = resolve_migration_chain(
        "1.0",
        "3.0",
        &[candidate("2.0", &["1.0"]), candidate("3.0", &["2.0"])],
    )
    .expect("approved chain resolves");
    let plan = MigrationPlan::new(
        "marcus-agent",
        "1.0",
        "3.0",
        vec!["v2.toml".into(), "v3.toml".into()],
        approved,
    );

    let drifted = resolve_migration_chain(
        "1.0",
        "4.0",
        &[candidate("2.0", &["1.0"]), candidate("4.0", &["2.0"])],
    )
    .expect("the changed candidate set remains structurally linear");

    assert!(matches!(
        plan.verify_live_chain(&drifted),
        Err(CohortError::EMigrationPlanDrift { .. })
    ));
}
