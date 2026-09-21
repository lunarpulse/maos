#[test]
fn serde_failure_termination_receipt_is_not_reported_as_raised() {
    let payload = r#"{"error":"serialization failed","halt_id":"01HALT"}"#;

    assert_eq!(
        maos_cli::subcommands::classify_halt_record(payload),
        ("termination_receipt", Some("01HALT".to_string()))
    );
}
