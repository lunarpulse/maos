//! V2 role-tagged withdrawals, scoped to one registry. A durable accepted yank
//! does not expire; the short authorization lifetime bounds mutation admission.
use maos_compliance::VetterKeyring;
use maos_domain::ports::registry::{YankEntry, YankOrigin};
use ring::rand::SecureRandom;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};

pub const MAX_CLOCK_SKEW_MS: u64 = 300_000;
pub const MAX_AUTHORIZATION_AGE_MS: u64 = 300_000;

#[derive(Clone, Copy)]
pub struct YankScope<'a> {
    pub name: &'a str,
    pub version: &'a str,
    pub reason: &'a str,
    pub registry_uri: &'a str,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YankAuthorization {
    pub format_version: u32,
    pub registry_uri: String,
    pub nonce: [u8; 16],
    pub max_age_ms: u64,
    pub origin: YankOrigin,
    pub public_key: [u8; 32],
    pub vetter_key_id: String,
    pub issued_at_unix_ms: u64,
    pub signature: Vec<u8>,
}

impl YankAuthorization {
    fn message(&self, scope: YankScope<'_>) -> Result<Vec<u8>, String> {
        let role = match self.origin {
            YankOrigin::Publisher => "maos.registry.deprecate.publisher.v2",
            YankOrigin::Vetter => "maos.registry.deprecate.vetter.v2",
        };
        serde_cbor::to_vec(&(
            role,
            self.format_version,
            &self.registry_uri,
            self.nonce,
            self.max_age_ms,
            scope.name,
            scope.version,
            scope.reason,
            self.origin,
            self.public_key,
            &self.vetter_key_id,
            self.issued_at_unix_ms,
        ))
        .map_err(|e| e.to_string())
    }

    pub fn sign(
        seed: &[u8; 32],
        scope: YankScope<'_>,
        origin: YankOrigin,
        vetter_key_id: String,
        now: u64,
    ) -> Result<Vec<u8>, String> {
        if scope.registry_uri.is_empty() {
            return Err("yank registry audience is empty".into());
        }
        let key = Ed25519KeyPair::from_seed_unchecked(seed).map_err(|e| e.to_string())?;
        let mut nonce = [0; 16];
        ring::rand::SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|e| e.to_string())?;
        let mut auth = Self {
            format_version: 2,
            registry_uri: scope.registry_uri.into(),
            nonce,
            max_age_ms: MAX_AUTHORIZATION_AGE_MS,
            origin,
            public_key: key
                .public_key()
                .as_ref()
                .try_into()
                .map_err(|_| "invalid public key length")?,
            vetter_key_id,
            issued_at_unix_ms: now,
            signature: Vec::new(),
        };
        auth.signature = key.sign(&auth.message(scope)?).as_ref().to_vec();
        serde_cbor::to_vec(&auth).map_err(|e| e.to_string())
    }

    fn verify_signature(
        &self,
        scope: YankScope<'_>,
        publisher: &[u8; 32],
        vetters: Option<(&VetterKeyring, &[u8; 32])>,
        now: u64,
    ) -> Result<(), String> {
        if self.format_version != 2
            || self.registry_uri != scope.registry_uri
            || self.max_age_ms == 0
            || self.max_age_ms > MAX_AUTHORIZATION_AGE_MS
        {
            return Err("yank format, registry audience or lifetime is invalid".into());
        }
        UnparsedPublicKey::new(&ED25519, self.public_key)
            .verify(&self.message(scope)?, &self.signature)
            .map_err(|_| "unauthenticated registry.deprecate: invalid signature")?;
        if self.issued_at_unix_ms > now.saturating_add(MAX_CLOCK_SKEW_MS) {
            return Err("yank authorization is not yet valid".into());
        }
        match self.origin {
            YankOrigin::Publisher if self.public_key == *publisher => Ok(()),
            YankOrigin::Publisher => {
                Err("unauthorized publisher yank: pinned owner differs".into())
            }
            YankOrigin::Vetter => {
                let (keyring, root) =
                    vetters.ok_or("unauthorized vetter yank: no enrolled keyring")?;
                keyring
                    .verify_enrollment(
                        root,
                        &self.vetter_key_id,
                        &self.public_key,
                        self.issued_at_unix_ms,
                        now,
                    )
                    .map_err(|e| e.to_string())
            }
        }
    }

    /// Only fresh requests may append durable withdrawal state.
    pub fn verify(
        &self,
        scope: YankScope<'_>,
        publisher: &[u8; 32],
        vetters: Option<(&VetterKeyring, &[u8; 32])>,
        now: u64,
    ) -> Result<(), String> {
        self.verify_signature(scope, publisher, vetters, now)?;
        if now
            > self
                .issued_at_unix_ms
                .saturating_add(self.max_age_ms)
                .saturating_add(MAX_CLOCK_SKEW_MS)
        {
            return Err("yank mutation authorization has expired".into());
        }
        Ok(())
    }
}

pub fn decode(bytes: &[u8]) -> Result<YankAuthorization, String> {
    serde_cbor::from_slice(bytes).map_err(|e| format!("invalid signed yank authorization: {e}"))
}

/// Verify historical authority, not request freshness: accepted withdrawals
/// remain effective after their mutation credential expires.
pub fn verify_entry(
    entry: &YankEntry,
    publisher: &[u8; 32],
    vetters: Option<(&VetterKeyring, &[u8; 32])>,
    registry_uri: &str,
    now: u64,
) -> Result<(), String> {
    let auth = decode(&entry.authorization)?;
    if entry.origin != Some(auth.origin) {
        return Err("yank origin differs from signed actor".into());
    }
    auth.verify_signature(
        YankScope {
            name: entry.spirit_id.as_str(),
            version: &entry.version,
            reason: &entry.reason,
            registry_uri,
        },
        publisher,
        vetters,
        now,
    )
}

pub fn now_unix_ms() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .map_err(|e| e.to_string())
}
