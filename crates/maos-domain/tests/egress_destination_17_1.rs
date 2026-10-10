use maos_domain::host_grant::EgressDestination;

#[test]
fn manifest_requires_a_port_while_bare_grants_mean_https() {
    assert!(EgressDestination::try_from("api.openai.com".to_string()).is_err());
    let grant = EgressDestination::parse_granted("api.openai.com").unwrap();
    assert_eq!(
        grant,
        EgressDestination::try_from("api.openai.com:443".to_string()).unwrap()
    );
    assert_eq!(grant.to_string(), "api.openai.com:443");
    let local = EgressDestination::parse_granted("127.0.0.1:8080").unwrap();
    assert_eq!(local.host, "127.0.0.1");
    assert_eq!(local.port, 8080);
    assert_ne!(
        local,
        EgressDestination::parse_granted("127.0.0.1:8081").unwrap()
    );
}

#[test]
fn invalid_authorities_are_refused_on_both_boundaries() {
    for value in [
        "",
        "API.example:443",
        "example.:443",
        "*.example:443",
        "https://example:443",
        "user@example:443",
        "example:443/path",
        "[::1]:443",
        "example:0",
        "example:65536",
        "example:+443",
        "example: 443",
        "-bad.example:443",
        "bad-.example:443",
        "bad..example:443",
        "127.0.0.999:443",
        "127.1:443",
        "01.2.3.4:443",
    ] {
        assert!(
            EgressDestination::try_from(value.to_string()).is_err(),
            "{value}"
        );
        assert!(EgressDestination::parse_granted(value).is_err(), "{value}");
    }
}

#[test]
fn serde_uses_the_same_strict_grammar_and_canonical_output() {
    let destination: EgressDestination = serde_json::from_str("\"api.openai.com:00443\"").unwrap();
    assert_eq!(
        serde_json::to_string(&destination).unwrap(),
        "\"api.openai.com:443\""
    );
    for value in [
        "\"api.openai.com\"",
        "42",
        "{\"host\":\"api.openai.com\",\"port\":443}",
    ] {
        assert!(
            serde_json::from_str::<EgressDestination>(value).is_err(),
            "{value}"
        );
    }
}

#[test]
fn dns_label_and_total_lengths_are_bounded() {
    assert!(EgressDestination::try_from(format!("{}:443", "a".repeat(64))).is_err());
    assert!(EgressDestination::try_from(format!("{}.example:65535", "a".repeat(63))).is_ok());
    let host = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(63),
    ]
    .join(".");
    assert!(EgressDestination::try_from(format!("{host}:443")).is_err());
}
