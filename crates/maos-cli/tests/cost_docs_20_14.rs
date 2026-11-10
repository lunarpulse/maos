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
