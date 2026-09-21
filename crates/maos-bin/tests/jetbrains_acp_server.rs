#![forbid(unsafe_code)]

//! Story 10.5 AC2 — JetBrains ACP bridge through the real `maos` binary.
//!
//! Spawns `maos` with `MAOS_ONE_SHOT=acp-server`, sends a scripted JetBrains
//! NDJSON conversation over stdio, and asserts the responses come from the
//! composition-root ACP server path rather than an in-process stub test.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn jetbrains_acp_server_binary_routes_real_resolvers() {
    let workspace_root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time must be monotonic")
        .as_nanos();
    let home = std::env::temp_dir().join(format!("maos-acp-jb-{nonce}"));
    std::fs::create_dir_all(&home).expect("temp HOME must be creatable");

    let mut child = Command::new(env!("CARGO_BIN_EXE_maos"))
        .env("MAOS_ONE_SHOT", "acp-server")
        .env("MAOS_OLLAMA_URL", "skip")
        // `HOME` is a POSIX convention; Windows home resolution reads
        // `USERPROFILE` (and `HOMEDRIVE`+`HOMEPATH`), so setting only `HOME`
        // left the child pointed at the runner's real profile. Every key that
        // can name a home is set, so the isolation this test claims is the
        // isolation it gets on every platform.
        //
        // ⚠ THIS WAS NOT THE CAUSE of the `windows-check` failure. I proposed
        // it as a hypothesis while the step log was unreadable, and the log
        // (job 106339757309) disproved it: the daemon never got far enough to
        // resolve a home, it refused at boot with exit 78 on the unix-only
        // store lock. Kept because unisolated HOME is a real defect on its own,
        // recorded as a hypothesis-then-disproof so the next reader does not
        // mistake it for the fix. See the platform-boundary assertion below.
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("HOMEDRIVE", "")
        .env("HOMEPATH", &home)
        .current_dir(workspace_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn maos acp-server");

    let input = concat!(
        r#"{"kind":"session_start","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"editor_id":"jetbrains","editor_version":"2024.3"}"#,
        "\n",
        r#"{"kind":"lifecycle_verb","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"decision_id":[172,2,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"verb":"load","spirit_id":"test-spirit-jb"}"#,
        "\n",
        r#"{"kind":"halt_resolve","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"decision_id":[172,2,0,0,0,0,0,0,0,0,0,0,0,0,0,1],"halt_id":"halt-jb-1","resolution":"approve"}"#,
        "\n",
        r#"{"kind":"session_end","session_id":[10,5,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#,
        "\n",
    );

    child
        .stdin
        .as_mut()
        .expect("stdin must be piped")
        .write_all(input.as_bytes())
        .expect("script must write to acp-server");
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("acp-server must exit on stdin EOF");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // ⚠ THE DAEMON CANNOT BOOT OFF UNIX, BY DESIGN — so asserting success on
    // every platform asserted something the product never promised. This was
    // `windows-check` step #8, red since Story 16-5, and the measured cause
    // (log of job 106339757309, read 2026-09-21) is:
    //
    //   status=exit code: 78
    //   maos: cannot lock store directory / (store locking requires a unix
    //   platform) — LockUnavailable
    //
    // `operator_door.rs:1861-1868` is `#[cfg(not(unix))]` and returns
    // `StoreLockError::LockUnavailable` with a HARDCODED `PathBuf::from("/")`,
    // which is why the message names `/` — it is a placeholder, not a resolved
    // path, and not a HOME-isolation problem. The `windows-check` build step
    // compiles `maos-kernel-core` and `maos-cli` only; `maos-bin` is not a
    // Windows target at all. So the platform boundary is the contract, and the
    // test now PROVES it on Windows instead of contradicting it. The e2e
    // routing assertions stay where the daemon can actually boot; they are also
    // covered on Linux by `workspace-test-suite`'s `cargo test --workspace`.
    #[cfg(not(unix))]
    {
        let _ = &stdout;
        assert_eq!(
            output.status.code(),
            Some(78),
            "off unix the daemon must REFUSE at boot with the config exit code, \
             not partially serve ACP: status={} stderr={stderr}",
            output.status
        );
        assert!(
            stderr.contains("store locking requires a unix platform"),
            "the refusal must name the unix-only store lock so an operator is \
             not left guessing why the daemon would not start: {stderr}"
        );
    }

    #[cfg(unix)]
    {
        assert!(
            output.status.success(),
            "maos acp-server failed: status={} stderr={stderr}",
            output.status
        );

        let responses: Vec<serde_json::Value> = stdout
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).expect("each ACP response must be JSON"))
            .collect();

        assert!(
            responses.len() >= 4,
            "expected ACP responses, got: {stdout}"
        );
        assert_eq!(responses[0]["kind"], "session_ready");
        assert_eq!(responses[1]["kind"], "lifecycle_receipt");
        assert_eq!(responses[2]["kind"], "halt_receipt");
        assert_eq!(responses[3]["kind"], "session_terminated");
        assert!(
            !stderr.contains("StubLifecycleResolver") && !stderr.contains("StubHaltResolver"),
            "binary ACP path must not announce stub resolvers: {stderr}"
        );
    }

    let _ = std::fs::remove_dir_all(home);
}
