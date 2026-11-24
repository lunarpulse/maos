#![forbid(unsafe_code)]

//! Story 22-4 — CONTRACT tests for `maosctl ask` and `maosctl task show` (AC1, AC3, AC4).
//!
//! The wire (method, path, bearer, body) is what the client put on the loopback fixture door;
//! the mapping (exit code, stdout, stderr) is the production client's reading of the door's
//! scripted replies. The fixture is hand-rolled because `maos-cli` must not gain a
//! `maos-control` edge (`dep_kernel_core_free_test.rs`).

#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::{FixtureDoor, Reply};

const ID: &str = "01J9ZZZZZZZZZZZZZZZZZZZZZZ";

fn accepted() -> Reply {
    Reply::new(
        200,
        &format!(r#"{{"task_id":"{ID}","spirit_id":"probe","state":"queued","replayed":false}}"#),
    )
}

fn task(state: &str, extra: &str) -> Reply {
    Reply::new(
        200,
        &format!(
            r#"{{"task_id":"{ID}","spirit_id":"probe","origin":"door","state":"{state}","accepted_ns":1{extra}}}"#
        ),
    )
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn ask_posts_the_text_and_key_and_prints_only_the_task_id() {
    let door = FixtureDoor::spawn_scripted(vec![accepted()]);
    let out = door.run(&["ask", "probe", "summarise the queue", "--key", "retry-1"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(String::from_utf8_lossy(&out.stdout), format!("{ID}\n"));
    let requests = door.requests();
    assert_eq!(requests.len(), 1, "no polling without --wait");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/v1/spirits/probe/ask");
    assert_eq!(
        requests[0].authorization.as_deref(),
        Some(format!("Bearer {}", door.token()).as_str())
    );
    assert_eq!(
        requests[0].json_body(),
        serde_json::json!({"text": "summarise the queue", "idempotency_key": "retry-1"})
    );

    // Without --key the field is absent, not null.
    let door = FixtureDoor::spawn_scripted(vec![accepted()]);
    let out = door.run(&["ask", "probe", "plain"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(
        door.requests()[0].json_body(),
        serde_json::json!({"text": "plain"})
    );
}

#[test]
fn a_full_queue_exits_75_and_every_other_refusal_exits_1() {
    let door = FixtureDoor::spawn_replying(
        409,
        r#"{"error":"task_queue_full","detail":"the spirit task queue holds 32 queued tasks; retry later"}"#,
    );
    let out = door.run(&["ask", "probe", "x"]);
    assert_eq!(out.status.code(), Some(75), "{}", stderr(&out));
    assert!(out.stdout.is_empty(), "no task id was minted to print");
    assert!(stderr(&out).contains("task_queue_full"), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("nothing was accepted"),
        "{}",
        stderr(&out)
    );

    for (status, body) in [
        (404, r#"{"error":"spirit_not_loaded","detail":"gone"}"#),
        (409, r#"{"error":"spirit_not_running","detail":"Paused"}"#),
        (
            409,
            r#"{"error":"spirit_not_directable","detail":"native"}"#,
        ),
        (409, r#"{"error":"host_stopping","detail":"stopping"}"#),
        (400, r#"{"error":"invalid_task","detail":"empty"}"#),
        (413, r#"{"error":"payload_too_large","detail":"64 KiB"}"#),
    ] {
        let door = FixtureDoor::spawn_replying(status, body);
        let out = door.run(&["ask", "probe", "x"]);
        assert_eq!(
            out.status.code(),
            Some(1),
            "{status} {body}: {}",
            stderr(&out)
        );
        assert!(out.stdout.is_empty(), "{body}");
    }
    let door = FixtureDoor::spawn_replying(401, r#"{"error":"unauthorized"}"#);
    assert_eq!(door.run(&["ask", "probe", "x"]).status.code(), Some(77));
}

#[test]
fn a_bad_spirit_or_key_never_leaves_maosctl() {
    let door = FixtureDoor::spawn_scripted(vec![accepted()]);
    for args in [
        vec!["ask", "bad/spirit", "x"],
        vec!["ask", "probe", "x", "--key", "has space"],
        vec!["task", "show", "bad/id"],
    ] {
        let out = door.run(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
    }
    assert!(door.requests().is_empty(), "nothing reached the door");
}

#[test]
fn wait_polls_until_the_task_ends_and_prints_the_task_object() {
    let done = task("completed", r#","result":"recall-probe ok","ended_ns":2"#);
    let door = FixtureDoor::spawn_scripted(vec![
        accepted(),
        task("queued", ""),
        task("running", ""),
        done,
    ]);
    let out = door.run(&["ask", "probe", "hello", "--wait"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let printed: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("--wait prints the task object");
    assert_eq!(printed["state"], "completed");
    assert_eq!(printed["result"], "recall-probe ok");
    let requests = door.requests();
    assert_eq!(
        requests.len(),
        4,
        "one POST, then polling GETs: {requests:?}"
    );
    assert_eq!(requests[0].method, "POST");
    for poll in &requests[1..] {
        assert_eq!(poll.method, "GET");
        assert_eq!(poll.path, format!("/v1/tasks/{ID}"));
    }
}

#[test]
fn wait_on_a_task_that_ended_without_an_answer_exits_1_with_the_reason() {
    for (state, extra) in [
        ("failed", r#","reason":"deadline_exceeded","ended_ns":2"#),
        ("orphaned", r#","reason":"session_ended","ended_ns":2"#),
        ("interrupted", r#","ended_ns":2"#),
        (
            "halted",
            r#","halt":{"halt_id":"h1","state":"PendingResolution"},"ended_ns":2"#,
        ),
    ] {
        let door = FixtureDoor::spawn_scripted(vec![accepted(), task(state, extra)]);
        let out = door.run(&["ask", "probe", "x", "--wait=30"]);
        assert_eq!(out.status.code(), Some(1), "{state}: {}", stderr(&out));
        let printed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(printed["state"], state, "the object is still printed");
        assert!(stderr(&out).contains(state), "{state}: {}", stderr(&out));
    }
    let door = FixtureDoor::spawn_scripted(vec![
        accepted(),
        task("failed", r#","reason":"deadline_exceeded","ended_ns":2"#),
    ]);
    let out = door.run(&["ask", "probe", "x", "--wait"]);
    assert!(
        stderr(&out).contains("deadline_exceeded"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn a_wait_that_runs_out_exits_75_and_names_how_to_read_the_task_later() {
    let door = FixtureDoor::spawn_scripted(vec![accepted(), task("running", "")]);
    let out = door.run(&["ask", "probe", "slow", "--wait=0"]);
    assert_eq!(out.status.code(), Some(75), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("maosctl task show"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn task_show_gets_the_task_and_an_unknown_id_is_task_not_found() {
    let door =
        FixtureDoor::spawn_scripted(vec![task("completed", r#","result":"ok","ended_ns":2"#)]);
    let out = door.run(&["task", "show", ID]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let printed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(printed["task_id"], ID);
    let requests = door.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, format!("/v1/tasks/{ID}"));

    let door =
        FixtureDoor::spawn_replying(404, r#"{"error":"task_not_found","detail":"no such task"}"#);
    let out = door.run(&["task", "show", ID]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("task_not_found"), "{}", stderr(&out));
}
