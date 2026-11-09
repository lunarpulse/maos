use clap::Parser;
use maos_cli::cli::{AuditQuery, Cli, Subcommand};

#[test]
fn audit_query_accepts_manifest_declaration_intent() {
    let cli = Cli::try_parse_from([
        "maosctl",
        "audit",
        "query",
        "--intent",
        "governance:manifest-declaration",
    ])
    .unwrap();
    match cli.command {
        Subcommand::Audit(args) => match args.query {
            Some(AuditQuery::Query {
                intent_contains, ..
            }) => assert_eq!(
                intent_contains.as_deref(),
                Some("governance:manifest-declaration")
            ),
            _ => panic!("expected audit query"),
        },
        _ => panic!("expected audit command"),
    }
}
