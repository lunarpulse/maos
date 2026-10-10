//! Daemon-owned, per-spawn credentials scoped to the manifest/grant intersection.

use std::{
    collections::{HashMap, HashSet},
    io,
    sync::Arc,
    time::{Duration, Instant},
};

use maos_domain::host_grant::EgressDestination;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const TOKEN_BYTES: usize = 32;
const TOKEN_HEX_BYTES: usize = TOKEN_BYTES * 2;

/// Exact host-and-port authorities both requested by the manifest and granted.
///
/// Destinations arrive already parsed by the domain; no authority grammar or
/// wildcard interpretation is introduced here.
pub struct Allowlist {
    destinations: HashSet<EgressDestination>,
}

impl Allowlist {
    pub fn intersect(requested: &[EgressDestination], granted: &[EgressDestination]) -> Self {
        let mut destinations = HashSet::new();
        for destination in requested {
            if granted.contains(destination) && !destinations.contains(destination) {
                destinations.insert(destination.clone());
            }
        }
        Self { destinations }
    }

    pub fn contains(&self, destination: &EgressDestination) -> bool {
        self.destinations.contains(destination)
    }
}

struct Grant {
    digest: [u8; TOKEN_BYTES],
    spirit_id: String,
    allowlist: Arc<Allowlist>,
    expires_at: Instant,
}

/// One active credential per real process id, with private hash-only storage.
///
/// The same lock serializes publication, verification, and revocation, so a
/// verification cannot observe half of a remint or race past a completed revoke.
#[derive(Default)]
pub struct BearerTable {
    grants: Mutex<HashMap<u32, Grant>>,
}

impl BearerTable {
    /// Publish a new credential, replacing any previous credential for `pid`.
    ///
    /// Only the returned value contains the wire token. Entropy failures and
    /// unrepresentable lifetimes leave the existing grant unchanged. A zero
    /// lifetime publishes an immediately expired credential.
    pub fn mint(
        &self,
        pid: u32,
        spirit_id: &str,
        allowlist: Arc<Allowlist>,
        ttl: Duration,
    ) -> io::Result<String> {
        let mut entropy = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut entropy)
            .map_err(|_| io::Error::other("unable to obtain scoped credential entropy"))?;
        let bearer = hex::encode(entropy);
        let digest: [u8; TOKEN_BYTES] = Sha256::digest(bearer.as_bytes()).into();
        let spirit_id = spirit_id.to_owned();

        let mut grants = self.grants.lock();
        let expires_at = Instant::now().checked_add(ttl).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "scoped credential lifetime exceeds the monotonic clock range",
            )
        })?;
        grants.insert(
            pid,
            Grant {
                digest,
                spirit_id,
                allowlist,
                expires_at,
            },
        );
        Ok(bearer)
    }

    /// Authorize only the current, live credential and its exact spawn scope.
    pub fn verify(
        &self,
        pid: u32,
        spirit_id: &str,
        destination: &EgressDestination,
        bearer: &str,
    ) -> bool {
        // The public wire format bounds hashing work even for hostile input.
        if bearer.len() != TOKEN_HEX_BYTES
            || !bearer
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return false;
        }
        let digest: [u8; TOKEN_BYTES] = Sha256::digest(bearer.as_bytes()).into();
        let grants = self.grants.lock();
        let Some(grant) = grants.get(&pid) else {
            return false;
        };
        let token_matches = bool::from(grant.digest.ct_eq(&digest));
        token_matches
            && grant.expires_at > Instant::now()
            && grant.spirit_id == spirit_id
            && grant.allowlist.contains(destination)
    }

    /// Remove the credential for this process; repeated revocation is harmless.
    pub fn revoke(&self, pid: u32) {
        self.grants.lock().remove(&pid);
    }
}
