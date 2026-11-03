use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn command(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    cmd.current_dir(root())
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("MAOS_HOME", home.join("maos"))
        .env("XDG_DATA_HOME", home.join("xdg"))
        .env("MAOS_SECRETS_BACKEND", "env")
        .env("MAOS_OLLAMA_URL", "skip")
        .env("MAOS_INFERENCE_MODE", "replay")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j0/shell-intro.json"),
        );
    cmd
}
pub fn shell(
    home: &Path,
    provider: &str,
    cassette: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let mut child = command(home)
        .arg("shell")
        .env("MAOS_DEFAULT_PROVIDER", provider)
        .env("MAOS_REPLAY_CASSETTE", root().join(cassette))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing stdin")?
        .write_all(b"@hello-spirit hello\n")?;
    Ok(child.wait_with_output()?)
}
pub fn intents(home: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let path = home.join("maos/audit/transparency.sqlite");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT intent FROM transparency_log ORDER BY rowid")?;
    let rows = stmt.query_map([], |r| r.get(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
pub fn issuance_payloads(home: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open_with_flags(
        home.join("maos/audit/transparency.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;
    let mut stmt = conn.prepare("SELECT payload_redacted FROM transparency_log WHERE intent LIKE 'cap.issue%' ORDER BY rowid")?;
    let rows = stmt.query_map([], |r| r.get::<_, Option<Vec<u8>>>(0))?;
    let mut payloads = Vec::new();
    for row in rows {
        payloads.push(String::from_utf8_lossy(&row?.unwrap_or_default()).into_owned());
    }
    Ok(payloads)
}
pub fn researcher(
    home: &Path,
    provider: &str,
    config: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(root().join("spirits/researcher/manifest.toml"))?;
    let mut doc: toml::Value = toml::from_str(&text)?;
    doc["capabilities"]["required"]["provider"]["complete"] = toml::Value::Array(vec![
        toml::Value::String("anthropic.claude-sonnet-5-5".into()),
        toml::Value::String(provider.into()),
    ]);
    let manifest = home.join("researcher.toml");
    std::fs::write(&manifest, format!("{}\n{config}", toml::to_string(&doc)?))?;
    Ok(command(home)
        .args(["run"])
        .arg(&manifest)
        .arg("--once")
        .env(
            "MAOS_REPLAY_CASSETTE",
            root().join("crates/maos-journey-test/cassettes/j-researcher/survey-distill.json"),
        )
        .output()?)
}
pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}
