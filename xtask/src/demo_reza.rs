//! `xtask demo-reza` — the one-command Reza Cortex scene.
//!
//! Story 13.6 proved the three-team / three-region journey, but the only way to
//! watch it was to hand-provision four databases, export seven variables, and
//! run an `#[ignore]`d integration test. The epic's demo-ability rung
//! (`epic-13-reza-cortex-v2-2.md:92`) asked for an executable scene and got
//! one; it never asked for an observable one.
//!
//! This runner is orchestration ONLY. It re-implements no oracle, asserts no
//! claim of its own, and owns no evidence: the journey test is still the test,
//! the four gates are still the judges, and the published ledgers are still the
//! verdict. What this adds is the narration around them and the substrate
//! bring-up in front of them.
//!
//! It deliberately shells out to `psql`/`createdb` rather than taking a
//! Postgres client dependency: those are the tools
//! `docs/testing/local-loom-substrate.md` already requires, and a demo runner
//! must not widen the workspace dependency surface.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// The four databases the substrate contract names.
/// Mirrors `check_loom_substrate_drift::CONTRACTS`; that gate remains the
/// authority, this is the provisioning list.
const DATABASES: [&str; 4] = ["maos_team_a", "maos_team_b", "maos_team_c", "maos_shared"];

const DEFAULT_PG: &str = "postgresql://postgres:postgres@127.0.0.1:5432";

/// The six production processes the journey drives, in execution order.
/// Text only — the assertions live in the test.
const PROCESSES: [(&str, &str); 6] = [
    (
        "P1  host-c daemon",
        "the tail of the chain: ACCEPTS only, never sends",
    ),
    (
        "P2  host-b daemon",
        "the middle: ACCEPTS from host-a AND SENDS to host-c (two peer entries)",
    ),
    (
        "P3  host-a daemon",
        "the head: SENDS only — A->B->C must land within 60s",
    ),
    (
        "P4  maos run researcher --once",
        "production Spirit path writes exactly one route row in team-a",
    ),
    (
        "P5  MAOS_ONE_SHOT=collective-erase",
        "destination erase reconciles the SOURCE: status erase_reconciled, both sides gone",
    ),
    (
        "P6  maos traceback --team team-b",
        "consented cross-wall read: outcome ok, exactly six disclosure fields, no payload bytes",
    ),
];

/// The two legs whose `PROVEN_LIVE_SIGNED` state is the whole product claim.
const KEY_LEGS: [&str; 2] = [
    "reza-three-team-three-region-journey",
    "cortex-fourteen-institution-isolation",
];

pub fn run(provision: bool, skip_gates: bool, journey_only: bool) -> Result<(), String> {
    let pg = std::env::var("MAOS_DEMO_PG").unwrap_or_else(|_| DEFAULT_PG.to_string());

    banner("Reza Cortex — three teams, three regions, one governed journey");

    let present = preflight(&pg, provision)?;
    let key = operator_key();
    report_key(&key);

    if !present {
        return Err(concat!(
            "demo-reza: substrate incomplete. Re-run with --provision, or bring it up by hand:\n",
            "  docker run -d --name maos-loom -p 5432:5432 \\\n",
            "    -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \\\n",
            "    -e POSTGRES_DB=maos_team_a pgvector/pgvector:pg16\n",
            "  (then `cargo run -p xtask -- demo-reza --provision`)\n",
            "Full runbook: docs/testing/local-loom-substrate.md"
        )
        .to_string());
    }

    let env = substrate_env(&pg);

    section("The scene");
    for (name, what) in PROCESSES {
        println!("  {name:<34} {what}");
    }
    println!();
    println!("  All six run inside ONE test against the live substrate. It is the");
    println!("  production entry points that execute — not a harness standing in for them.");

    section("Running the journey");
    let journey_ok = run_journey(&env, &key)?;
    if !journey_ok {
        return Err("demo-reza: the journey did not pass — see the test output above".to_string());
    }
    println!("  PASS — all six processes reached their production dispatch.");

    if journey_only {
        section("Done (--journey-only: gates and ledger skipped)");
        return Ok(());
    }

    section("Observable end state");
    observe(&pg);

    if skip_gates {
        section("Done (--skip-gates: the four judges were not run)");
        return Ok(());
    }

    section("Running the four substrate gates");
    let gates = run_gates(&env, &key);
    for (gate, ok) in &gates {
        println!("  {:<32} {}", gate, if *ok { "exit 0" } else { "FAILED" });
    }

    section("Verdict");
    summarize()?;

    if gates.iter().any(|(_, ok)| !ok) {
        return Err("demo-reza: at least one gate failed".to_string());
    }
    Ok(())
}

/// Probe the substrate and, when asked, create what is missing.
/// Returns whether all four databases are present afterwards.
fn preflight(pg: &str, provision: bool) -> Result<bool, String> {
    section("Substrate");
    let host_args = pg_host_args(pg);

    if !tool_exists("psql") {
        println!("  psql not on PATH — cannot probe or provision.");
        return Ok(false);
    }

    let mut existing = list_databases(&host_args);
    if existing.is_empty() {
        println!("  no PostgreSQL reachable at {pg}");
        return Ok(false);
    }

    for db in DATABASES {
        if !existing.contains(&db.to_string()) && provision && tool_exists("createdb") {
            let made = Command::new("createdb")
                .args(&host_args)
                .arg(db)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if made {
                existing.push(db.to_string());
                println!("  {db:<16} created");
                continue;
            }
        }
        println!(
            "  {db:<16} {}",
            if existing.contains(&db.to_string()) {
                "present"
            } else {
                "MISSING"
            }
        );
    }

    Ok(DATABASES
        .iter()
        .all(|db| existing.contains(&db.to_string())))
}

/// The operator audit key, if one is reachable. Its absence is not an error —
/// it changes what the run can prove, and the caller is told exactly how.
fn operator_key() -> Option<String> {
    if let Ok(explicit) = std::env::var("MAOS_AUDIT_KEY") {
        if Path::new(&explicit).exists() {
            return Some(explicit);
        }
    }
    let default = format!(
        "{}/.config/maos/audit-signing.key",
        std::env::var("HOME").unwrap_or_default()
    );
    Path::new(&default).exists().then_some(default)
}

fn report_key(key: &Option<String>) {
    match key {
        Some(path) => println!("  operator key    {path}"),
        None => {
            println!("  operator key    ABSENT");
            println!();
            println!("  Without it the journey leg reports ABSENT by design — the gate checks");
            println!("  for a key BEFORE launching and refuses to project a state it cannot");
            println!("  verify. CI holds no operator key on purpose. To sign this run:");
            println!("      maosctl audit keygen --output ~/.config/maos/audit-signing.key");
        }
    }
}

/// The seven-variable substrate contract, plus the one ratified alias:
/// `check-multi-tenant-loom` points the legacy singular at `maos_team_b`.
fn substrate_env(pg: &str) -> Vec<(String, String)> {
    let mut env = vec![
        ("MAOS_TEST_POSTGRES".into(), format!("{pg}/maos_team_b")),
        ("MAOS_TEST_POSTGRES_A".into(), format!("{pg}/maos_team_a")),
        ("MAOS_TEST_POSTGRES_B".into(), format!("{pg}/maos_team_b")),
        ("MAOS_TEST_POSTGRES_C".into(), format!("{pg}/maos_team_c")),
    ];
    for (suffix, db) in [
        ("A", "maos_team_a"),
        ("B", "maos_team_b"),
        ("C", "maos_team_c"),
    ] {
        env.push((
            format!("MAOS_TEST_POSTGRES_TEAM_{suffix}"),
            format!("{pg}/{db}"),
        ));
    }
    env
}

fn run_journey(env: &[(String, String)], key: &Option<String>) -> Result<bool, String> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "test",
        "-q",
        "-p",
        "maos-bin",
        "--features",
        "network",
        "--test",
        "cross_team_crossing_13_6b",
        "reza_three_team_three_region_production_journey",
        "--",
        "--exact",
        "--include-ignored",
    ]);
    apply_env(&mut cmd, env, key);
    cmd.status()
        .map(|s| s.success())
        .map_err(|e| format!("demo-reza: cargo test invocation failed: {e}"))
}

fn run_gates(env: &[(String, String)], key: &Option<String>) -> Vec<(&'static str, bool)> {
    // `check-multi-tenant-loom` is the exception the runbook names: its CI job
    // exports the legacy singular at team-b, which `substrate_env` already does.
    // The other three want the shared stand-in instead.
    [
        ("check-cross-region-consensus", true),
        ("check-multi-region-slo", true),
        ("check-multi-tenant-loom", false),
        ("check-reza-production-path", true),
    ]
    .iter()
    .map(|(gate, shared_standin)| {
        let mut cmd = Command::new("cargo");
        cmd.args(["run", "-q", "-p", "xtask", "--", gate]);
        apply_env(&mut cmd, env, key);
        if *shared_standin {
            if let Some((_, team_b)) = env.iter().find(|(k, _)| k == "MAOS_TEST_POSTGRES") {
                cmd.env(
                    "MAOS_TEST_POSTGRES",
                    team_b.replace("maos_team_b", "maos_shared"),
                );
            }
        }
        let ok = cmd
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        (*gate, ok)
    })
    .collect()
}

fn apply_env(cmd: &mut Command, env: &[(String, String)], key: &Option<String>) {
    for (k, v) in env {
        cmd.env(k, v);
    }
    if let Some(path) = key {
        cmd.env("MAOS_AUDIT_KEY", path);
    }
}

/// What is left in the databases after the scene: the erase must have taken the
/// crossed row from BOTH sides and left a tombstone where the delete happened.
fn observe(pg: &str) {
    let host = pg_host_args(pg);
    let crossed = scalar(
        &host,
        "maos_team_a",
        "SELECT count(*) FROM collective_memory WHERE namespace_detail LIKE 'xteam:%'",
    );
    let tombstones = scalar(
        &host,
        "maos_team_b",
        "SELECT count(*) FROM collective_erasure_tombstones",
    );
    match (crossed, tombstones) {
        (Some(c), Some(t)) => {
            println!("  team-a crossed rows remaining   {c}");
            println!("  team-b erasure tombstones       {t}");
            println!();
            println!("  The erase is two-sided: the destination row is gone, the source row it");
            println!("  was reconciled against is gone, and the tombstone records the deletion");
            println!("  so a replayed bundle cannot resurrect it.");
        }
        _ => println!("  (psql unavailable — skipping the direct database observation)"),
    }
}

fn summarize() -> Result<(), String> {
    let dir = Path::new("tests/reports");
    let mut claims: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut key_legs: BTreeMap<String, String> = BTreeMap::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("demo-reza: cannot read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("evidence-ledger-") && n.ends_with(".json"))
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let gate = json["gate"].as_str().unwrap_or("?").to_string();
        let claim = json["product_claim"].as_str().unwrap_or("?").to_string();
        let commit = json["commit"].as_str().unwrap_or("?");
        claims.insert(gate, (claim, commit.chars().take(8).collect::<String>()));
        if let Some(legs) = json["legs"].as_array() {
            for leg in legs {
                let name = leg["name"].as_str().unwrap_or_default();
                if KEY_LEGS.contains(&name) {
                    key_legs.insert(
                        name.to_string(),
                        leg["evidence_state"].as_str().unwrap_or("?").to_string(),
                    );
                }
            }
        }
    }

    if claims.is_empty() {
        return Err("demo-reza: no published ledgers in tests/reports".to_string());
    }
    for (gate, (claim, commit)) in &claims {
        println!("  {gate:<32} {claim:<12} @ {commit}");
    }
    println!();
    for leg in KEY_LEGS {
        println!(
            "  {leg:<40} {}",
            key_legs
                .get(leg)
                .map(String::as_str)
                .unwrap_or("NOT PRESENT")
        );
    }
    println!();

    let all_proven = claims.values().all(|(claim, _)| claim == "PROVEN");
    let legs_signed = KEY_LEGS.iter().all(|leg| {
        key_legs
            .get(*leg)
            .is_some_and(|s| s == "PROVEN_LIVE_SIGNED")
    });
    if all_proven && legs_signed {
        println!("  Reza/v2.2 product claim: PROVEN, both required legs signed on this lane.");
        println!("  This proves ISOLATION at fourteen institutions — not throughput, not soak,");
        println!("  not geo-distribution. See docs/release/v2.2-capacity-envelope.md.");
    } else {
        println!("  Claim NOT earned on this run. A development lane can exit zero while");
        println!("  product_claim is NOT_PROVEN — read the table, never the exit code.");
    }
    Ok(())
}

fn pg_host_args(pg: &str) -> Vec<String> {
    // postgresql://user:pass@host:port -> -h host -p port -U user
    let rest = pg.split("://").nth(1).unwrap_or(pg);
    let (creds, hostport) = rest.rsplit_once('@').unwrap_or(("postgres", rest));
    let user = creds.split(':').next().unwrap_or("postgres");
    let (host, port) = hostport
        .split_once(':')
        .unwrap_or((hostport.trim_end_matches('/'), "5432"));
    vec![
        "-h".into(),
        host.trim_end_matches('/').into(),
        "-p".into(),
        port.trim_end_matches('/').into(),
        "-U".into(),
        user.into(),
    ]
}

fn list_databases(host: &[String]) -> Vec<String> {
    Command::new("psql")
        .args(host)
        .args(["-lqtA", "-F", "|"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|line| line.split('|').next())
                .map(|name| name.trim().to_string())
                .filter(|name| !name.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn scalar(host: &[String], db: &str, sql: &str) -> Option<String> {
    let out = Command::new("psql")
        .args(host)
        .args(["-d", db, "-tAc", sql])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn tool_exists(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn banner(title: &str) {
    println!("\n=== {title} ===");
}

fn section(title: &str) {
    println!("\n-- {title}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_args_parse_a_full_connection_string() {
        assert_eq!(
            pg_host_args("postgresql://postgres:postgres@127.0.0.1:5432"),
            vec!["-h", "127.0.0.1", "-p", "5432", "-U", "postgres"]
        );
    }

    #[test]
    fn host_args_default_the_port_when_absent() {
        let args = pg_host_args("postgresql://alice@db.internal");
        assert_eq!(args[1], "db.internal");
        assert_eq!(args[3], "5432");
        assert_eq!(args[5], "alice");
    }

    #[test]
    fn substrate_env_points_the_legacy_singular_at_team_b() {
        // The one ratified cross-axis alias. If this drifts, the loom gate is
        // reproduced against the wrong database and its legs mean nothing.
        let env = substrate_env("pg://x");
        let singular = env
            .iter()
            .find(|(k, _)| k == "MAOS_TEST_POSTGRES")
            .expect("legacy singular is exported");
        assert_eq!(singular.1, "pg://x/maos_team_b");
    }

    #[test]
    fn substrate_env_keeps_both_axes_pairwise_distinct() {
        // check-loom-substrate-drift's topology-value-distinctness leg reds if
        // either axis aliases within itself; the demo must not be the thing
        // that fakes a wall.
        let env = substrate_env("pg://x");
        for prefix in ["MAOS_TEST_POSTGRES_TEAM_", "MAOS_TEST_POSTGRES_"] {
            let mut values: Vec<&str> = env
                .iter()
                .filter(|(k, _)| k.starts_with(prefix) && k.len() == prefix.len() + 1)
                .map(|(_, v)| v.as_str())
                .collect();
            assert_eq!(values.len(), 3, "three databases on the {prefix} axis");
            values.sort_unstable();
            values.dedup();
            assert_eq!(values.len(), 3, "{prefix} axis must be pairwise distinct");
        }
    }
}
