//! Signed, domain-separated publisher/vetter withdrawals. Never trust a response key.
use maos_compliance::VetterKeyring;
use maos_domain::ports::registry::{YankEntry, YankOrigin};
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YankAuthorization {
    pub origin: YankOrigin,
    pub public_key: [u8; 32],
    pub vetter_key_id: String,
    pub issued_at_unix_ms: u64,
    pub signature: Vec<u8>,
}

impl YankAuthorization {
    fn message(&self, name: &str, version: &str, reason: &str) -> Result<Vec<u8>, String> {
        serde_cbor::to_vec(&(
            "maos.registry.deprecate.v1",
            name,
            version,
            reason,
            self.origin,
            self.public_key,
            &self.vetter_key_id,
            self.issued_at_unix_ms,
        ))
        .map_err(|e| e.to_string())
    }

    pub fn sign(
        seed: &[u8; 32],
        name: &str,
        version: &str,
        reason: &str,
        origin: YankOrigin,
        vetter_key_id: String,
        now: u64,
    ) -> Result<Vec<u8>, String> {
        let key = Ed25519KeyPair::from_seed_unchecked(seed).map_err(|e| e.to_string())?;
        let mut auth = Self {
            origin,
            public_key: key
                .public_key()
                .as_ref()
                .try_into()
                .map_err(|_| "invalid Ed25519 public key length")?,
            vetter_key_id,
            issued_at_unix_ms: now,
            signature: Vec::new(),
        };
        auth.signature = key
            .sign(&auth.message(name, version, reason)?)
            .as_ref()
            .to_vec();
        serde_cbor::to_vec(&auth).map_err(|e| e.to_string())
    }

    pub fn verify(
        &self,
        name: &str,
        version: &str,
        reason: &str,
        publisher: &[u8; 32],
        vetters: Option<(&VetterKeyring, &[u8; 32])>,
        now: u64,
    ) -> Result<(), String> {
        UnparsedPublicKey::new(&ED25519, self.public_key)
            .verify(&self.message(name, version, reason)?, &self.signature)
            .map_err(|_| "unauthenticated registry.deprecate: invalid signature")?;
        if self.issued_at_unix_ms > now {
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
}

pub fn decode(bytes: &[u8]) -> Result<YankAuthorization, String> {
    serde_cbor::from_slice(bytes).map_err(|e| format!("invalid signed yank authorization: {e}"))
}

pub fn verify_entry(
    entry: &YankEntry,
    publisher: &[u8; 32],
    vetters: Option<(&VetterKeyring, &[u8; 32])>,
    now: u64,
) -> Result<(), String> {
    let auth = decode(&entry.authorization)?;
    if entry.origin != Some(auth.origin) {
        return Err("yank origin differs from signed actor".into());
    }
    auth.verify(
        entry.spirit_id.as_str(),
        &entry.version,
        &entry.reason,
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
