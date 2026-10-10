use maos_manifest::CliWrapperConfig;

fn config(extra: &str, posture_extra: &str) -> String {
    format!(
        "command = \"worker-cli-fixture\"\noutput_shape_version = \"1.0.0\"\n{extra}\n\
         [posture]\nstdio_shape = \"raw\"\ncontrol_channel = \"signals\"\n{posture_extra}\n"
    )
}

#[test]
fn unknown_wrapper_and_posture_fields_are_refused() {
    for (extra, posture_extra) in [("egres = []", ""), ("", "control_channe = \"signals\"")] {
        let error = CliWrapperConfig::from_toml_str(&config(extra, posture_extra))
            .expect_err("unknown policy fields must not be silently ignored");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }
}

#[test]
fn destination_grammar_is_strict_at_the_manifest_boundary() {
    for destination in [
        "api.openai.com",
        "API.openai.com:443",
        "api.openai.com.:443",
        "https://api.openai.com:443",
        "*.openai.com:443",
        "user@api.openai.com:443",
        "api.openai.com:443/path",
        "[::1]:443",
        "api.openai.com:0",
        "api.openai.com:65536",
        "api.openai.com:+443",
        "-bad.example:443",
        "bad-.example:443",
        "bad..example:443",
    ] {
        assert!(
            CliWrapperConfig::from_toml_str(&config(&format!("egress = [\"{destination}\"]"), ""))
                .is_err(),
            "invalid destination accepted: {destination}"
        );
    }
}

#[test]
fn duplicate_destinations_are_refused() {
    let error = CliWrapperConfig::from_toml_str(&config(
        "egress = [\"api.openai.com:443\", \"api.openai.com:443\"]",
        "",
    ))
    .expect_err("duplicate destinations must be refused");
    assert!(error.to_string().contains("duplicate"), "{error}");
}

#[test]
fn absent_egress_denies_all_and_valid_destinations_keep_their_exact_ports() {
    let empty = CliWrapperConfig::from_toml_str(&config("", "")).unwrap();
    assert!(empty.egress.is_empty());
    let cfg = CliWrapperConfig::from_toml_str(&config(
        "egress = [\"api.openai.com:443\", \"127.0.0.1:18081\"]",
        "",
    ))
    .unwrap();
    assert_eq!(
        cfg.egress
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["api.openai.com:443", "127.0.0.1:18081"]
    );
}
