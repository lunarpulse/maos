#![cfg(feature = "daemon")]

use std::{
    io,
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use maos_domain::host_grant::EgressDestination;
use maos_egress::{Allowlist, BearerTable};

fn destination(raw: &str) -> EgressDestination {
    raw.to_owned().try_into().unwrap()
}

fn policy(raw: &str) -> Arc<Allowlist> {
    let destination = destination(raw);
    Arc::new(Allowlist::intersect(
        std::slice::from_ref(&destination),
        std::slice::from_ref(&destination),
    ))
}

#[test]
fn minted_wire_format_is_exactly_32_bytes_of_lowercase_hex() {
    let table = BearerTable::default();
    let allowed = destination("api.example.com:443");
    let token = table
        .mint(
            1,
            "worker",
            policy("api.example.com:443"),
            Duration::from_secs(60),
        )
        .unwrap();
    assert_eq!(token.len(), 64);
    assert!(token
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
    assert!(table.verify(1, "worker", &allowed, &token));
    for malformed in [
        String::new(),
        "a".repeat(63),
        "a".repeat(65),
        "G".repeat(64),
    ] {
        assert!(!table.verify(1, "worker", &allowed, &malformed));
    }
}

#[test]
fn an_empty_side_or_disjoint_intersection_cannot_authorize_a_request() {
    let first = destination("api.example.com:443");
    let second = destination("other.example.com:443");
    let policies = [
        Allowlist::intersect(&[], std::slice::from_ref(&first)),
        Allowlist::intersect(std::slice::from_ref(&first), &[]),
        Allowlist::intersect(std::slice::from_ref(&first), std::slice::from_ref(&second)),
    ];
    let table = BearerTable::default();
    for allowlist in policies {
        assert!(!allowlist.contains(&first));
        assert!(!allowlist.contains(&second));
        let token = table
            .mint(1, "worker", Arc::new(allowlist), Duration::from_secs(60))
            .unwrap();
        assert!(!table.verify(1, "worker", &first, &token));
        assert!(!table.verify(1, "worker", &second, &token));
    }
}

#[test]
fn revocation_is_idempotent_and_does_not_touch_another_pid() {
    let table = BearerTable::default();
    let allowed = destination("api.example.com:443");
    let allowlist = policy("api.example.com:443");
    let first = table
        .mint(0, "worker", allowlist.clone(), Duration::from_secs(60))
        .unwrap();
    let second = table
        .mint(u32::MAX, "worker", allowlist, Duration::from_secs(60))
        .unwrap();
    table.revoke(17);
    table.revoke(0);
    table.revoke(0);
    assert!(!table.verify(0, "worker", &allowed, &first));
    assert!(table.verify(u32::MAX, "worker", &allowed, &second));
    assert!(!table.verify(u32::MAX, "worker", &allowed, &first));
}

#[test]
fn remint_replaces_the_spirit_and_destination_set_together() {
    let table = BearerTable::default();
    let old_destination = destination("api.example.com:443");
    let new_destination = destination("api.example.com:8443");
    let old = table
        .mint(
            1,
            "old-worker",
            policy("api.example.com:443"),
            Duration::from_secs(60),
        )
        .unwrap();
    let current = table
        .mint(
            1,
            "new-worker",
            policy("api.example.com:8443"),
            Duration::from_secs(60),
        )
        .unwrap();
    assert!(!table.verify(1, "old-worker", &old_destination, &old));
    assert!(!table.verify(1, "old-worker", &new_destination, &current));
    assert!(!table.verify(1, "new-worker", &old_destination, &current));
    assert!(table.verify(1, "new-worker", &new_destination, &current));
}

#[test]
fn an_unrepresentable_lifetime_is_an_error_without_replacing_the_current_token() {
    let table = BearerTable::default();
    let allowed = destination("api.example.com:443");
    let allowlist = policy("api.example.com:443");
    let current = table
        .mint(1, "worker", allowlist.clone(), Duration::from_secs(60))
        .unwrap();
    let error = table
        .mint(1, "replacement", allowlist, Duration::MAX)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!error.to_string().contains(&current));
    assert!(table.verify(1, "worker", &allowed, &current));
}

#[test]
fn a_nonzero_lifetime_expires_without_an_explicit_revoke() {
    let table = BearerTable::default();
    let allowed = destination("api.example.com:443");
    let token = table
        .mint(
            1,
            "worker",
            policy("api.example.com:443"),
            Duration::from_millis(1),
        )
        .unwrap();
    thread::sleep(Duration::from_millis(2));
    assert!(!table.verify(1, "worker", &allowed, &token));
}

#[test]
fn concurrent_remints_publish_only_one_current_credential() {
    let table = Arc::new(BearerTable::default());
    let allowlist = policy("api.example.com:443");
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let table = table.clone();
            let allowlist = allowlist.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                table
                    .mint(1, "worker", allowlist, Duration::from_secs(60))
                    .unwrap()
            })
        })
        .collect();
    barrier.wait();
    let tokens: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    let allowed = destination("api.example.com:443");
    assert_eq!(
        tokens
            .iter()
            .filter(|token| table.verify(1, "worker", &allowed, token))
            .count(),
        1,
    );
    table.revoke(1);
    assert!(tokens
        .iter()
        .all(|token| !table.verify(1, "worker", &allowed, token)));
}
