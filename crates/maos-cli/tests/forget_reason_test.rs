#[path = "support/fixture_door.rs"]
mod fixture_door;

use fixture_door::FixtureDoor;

#[test]
fn forget_reason_is_preserved_in_the_live_door_body() {
    let door = FixtureDoor::spawn();
    let output = door.run(&[
        "forget",
        "--principal",
        "held@example.org",
        "--reason",
        "legal-hold:CASE-7",
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "maosctl failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let requests = door.requests();
    assert_eq!(requests.len(), 1, "exactly one live-door request expected");
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].path, "/v1/memory/forget");
    let body: serde_json::Value =
        serde_json::from_slice(&requests[0].body).expect("forget JSON body");
    assert_eq!(body["principal"], "held@example.org");
    assert_eq!(body["reason"], "legal-hold:CASE-7");
}
