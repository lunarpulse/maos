#[test]
fn operator_cost_guides_disclose_limits() {
    let guides = [
        (
            include_str!("../../../docs/maos.dev/run-maos.md"),
            "Cost attribution",
        ),
        (
            include_str!("../../../docs-site/docs/run-maos.md"),
            "Cost attribution",
        ),
        (
            include_str!(
                "../../../docs-site/i18n/ko/docusaurus-plugin-content-docs/current/run-maos.md"
            ),
            "비용 귀속",
        ),
    ];
    for (guide, heading) in guides {
        assert!(guide.contains(heading));
        for term in [
            "session_principal",
            "write_target_proxy",
            "host-unallocated",
            "task: none",
            "Worker CLI",
            "v1.1",
            "98%",
        ] {
            assert!(guide.contains(term), "{heading}: missing {term}");
        }
    }
    let adr = include_str!("../../../docs/adr/ADR-046-cost-attribution-and-reconciliation.md");
    assert!(adr.contains("root-bound `task_ref`"));
    assert!(adr.contains("landing is pending"));
}

#[test]
fn adr_retains_write_path_authorization_debt() {
    let adr = include_str!("../../../docs/adr/ADR-046-cost-attribution-and-reconciliation.md");
    let (_, followup) = adr.split_once("### 10. SR-5").unwrap();
    let (followup, _) = followup.split_once("## Gate").unwrap();
    assert!(followup.contains("SR-1"));
    assert!(followup.contains("validate_namespace_write"));
}
