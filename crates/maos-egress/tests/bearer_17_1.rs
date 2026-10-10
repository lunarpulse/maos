#![cfg(feature = "daemon")]

use maos_domain::host_grant::EgressDestination;
use maos_egress::{Allowlist, BearerTable};
use std::{sync::Arc, time::Duration};

fn destination(raw: &str) -> EgressDestination {
    raw.to_owned().try_into().unwrap()
}
fn policy() -> Arc<Allowlist> {
    Arc::new(Allowlist::intersect(
        &[
            destination("api.example.com:443"),
            destination("api.example.com:8443"),
        ],
        &[EgressDestination::parse_granted("api.example.com").unwrap()],
    ))
}

#[test]
fn intersecting_authorities_does_not_grant_another_port_or_host() {
    let policy = policy();
    assert!(policy.contains(&destination("api.example.com:443")));
    assert!(!policy.contains(&destination("api.example.com:8443")));
    assert!(!policy.contains(&destination("other.example.com:443")));
}

#[test]
fn token_is_bound_to_the_real_pid_spirit_and_destination() {
    let table = BearerTable::default();
    let token = table
        .mint(17, "worker", policy(), Duration::from_secs(60))
        .unwrap();
    let allowed = destination("api.example.com:443");
    assert!(table.verify(17, "worker", &allowed, &token));
    assert!(!table.verify(18, "worker", &allowed, &token));
    assert!(!table.verify(17, "worker-2", &allowed, &token));
    assert!(!table.verify(17, "worker", &destination("api.example.com:8443"), &token));
    assert!(!table.verify(17, "worker", &allowed, "wrong-scoped-token"));
}

#[test]
fn expiry_and_revocation_are_fail_closed() {
    let table = BearerTable::default();
    let allowed = destination("api.example.com:443");
    let expired = table.mint(17, "worker", policy(), Duration::ZERO).unwrap();
    assert!(!table.verify(17, "worker", &allowed, &expired));
    let live = table
        .mint(17, "worker", policy(), Duration::from_secs(60))
        .unwrap();
    assert!(table.verify(17, "worker", &allowed, &live));
    table.revoke(17);
    assert!(!table.verify(17, "worker", &allowed, &live));
}

#[test]
fn remint_invalidates_the_old_spawn_credential() {
    let table = BearerTable::default();
    let allowed = destination("api.example.com:443");
    let old = table
        .mint(17, "worker", policy(), Duration::from_secs(60))
        .unwrap();
    let current = table
        .mint(17, "worker", policy(), Duration::from_secs(60))
        .unwrap();
    assert!(!table.verify(17, "worker", &allowed, &old));
    assert!(table.verify(17, "worker", &allowed, &current));
}
