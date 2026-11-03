#![forbid(unsafe_code)]
//! 20-4 AC1/2/3/5: real roots, isolated homes, explicit replay, no paid key.
#[allow(dead_code)]
#[path = "support/provider204.rs"]
mod support;
use support::*;

#[test]
fn epic_gemini_shell_command_selects_and_routes() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = shell(
        home.path(),
        "gemini",
        "crates/maos-journey-test/cassettes/providers/gemini-shell-intro.json",
    )?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Hello from the Gemini routing seed"));
    assert!(intents(home.path())?
        .iter()
        .any(|s| s.starts_with("infer:gemini->replay:")));
    assert!(issuance_payloads(home.path())?
        .iter()
        .any(|s| s.contains("ProviderInfer") && s.contains("gemini")));
    Ok(())
}
#[test]
fn kimi_shell_selects_and_routes() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = shell(
        home.path(),
        "kimi",
        "crates/maos-journey-test/cassettes/providers/kimi-shell-intro.json",
    )?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("Hello from the Kimi routing seed"));
    assert!(intents(home.path())?
        .iter()
        .any(|s| s.starts_with("infer:kimi->replay:")));
    Ok(())
}
#[test]
fn unknown_default_refuses_with_registered_ids() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = command(home.path())
        .arg("shell")
        .env("MAOS_DEFAULT_PROVIDER", "bedrock")
        .output()?;
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(
        err.contains("MAOS_DEFAULT_PROVIDER") && err.contains("bedrock"),
        "{err}"
    );
    for id in ["anthropic", "openai", "gemini", "kimi", "ollama"] {
        assert!(err.contains(id), "{err}");
    }
    Ok(())
}
#[test]
fn invalid_credentials_refuse_without_echoing_bytes() -> TestResult {
    for key in [
        "MAOS_ANTHROPIC_API_KEY",
        "MAOS_OPENAI_API_KEY",
        "MAOS_GEMINI_API_KEY",
        "MAOS_KIMI_API_KEY",
    ] {
        let home = tempfile::tempdir()?;
        let secret = "sk-DO-NOT-ECHO\u{1}tail";
        let out = command(home.path())
            .arg("shell")
            .env(key, secret)
            .output()?;
        assert_eq!(out.status.code(), Some(78), "{key}: {}", stderr(&out));
        assert!(!stderr(&out).contains("DO-NOT-ECHO"));
        assert!(!String::from_utf8_lossy(&out.stdout).contains("DO-NOT-ECHO"));
    }
    Ok(())
}
#[test]
fn researcher_primary_overrides_declaration_order() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = researcher(
        home.path(),
        "openai",
        "[providers.primary]\nid = \"openai\"\n",
    )?;
    assert!(out.status.success(), "{}", stderr(&out));
    let rows = intents(home.path())?;
    assert!(
        rows.iter().any(|s| s.starts_with("infer:openai->")),
        "{rows:?}"
    );
    assert!(
        !rows.iter().any(|s| s.starts_with("infer:anthropic->")),
        "{rows:?}"
    );
    Ok(())
}
#[test]
fn fallback_is_refused_before_inference() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = researcher(
        home.path(),
        "openai",
        "[providers.primary]\nid = \"anthropic\"\n[[providers.fallback]]\nid = \"openai\"\n",
    )?;
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("fallback_unsupported"),
        "{}",
        stderr(&out)
    );
    assert!(!intents(home.path())?
        .iter()
        .any(|s| s.starts_with("infer:")));
    Ok(())
}

#[test]
fn new_keys_register_and_preserve_default_order_in_replay() -> TestResult {
    for (key, id) in [
        ("MAOS_GEMINI_API_KEY", "gemini"),
        ("MAOS_KIMI_API_KEY", "kimi"),
    ] {
        let home = tempfile::tempdir()?;
        let mut child = command(home.path())
            .arg("shell")
            .env(key, "test-key-visible-ascii")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;
        std::io::Write::write_all(
            &mut child.stdin.take().ok_or("stdin")?,
            b"@hello-spirit hello\n",
        )?;
        let out = child.wait_with_output()?;
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(intents(home.path())?
            .iter()
            .any(|s| s.starts_with(&format!("infer:{id}->"))));
    }
    Ok(())
}

#[test]
fn empty_default_preserves_existing_router_default() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = shell(
        home.path(),
        "",
        "crates/maos-journey-test/cassettes/j0/shell-intro.json",
    )?;
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(intents(home.path())?
        .iter()
        .any(|s| s.starts_with("infer:anthropic->")));
    Ok(())
}

#[test]
fn undeclared_primary_is_a_typed_refusal() -> TestResult {
    let home = tempfile::tempdir()?;
    let out = researcher(
        home.path(),
        "openai",
        "[providers.primary]\nid = \"gemini\"\n",
    )?;
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("provider_unavailable"),
        "{}",
        stderr(&out)
    );
    assert!(!intents(home.path())?
        .iter()
        .any(|s| s.starts_with("infer:")));
    Ok(())
}
