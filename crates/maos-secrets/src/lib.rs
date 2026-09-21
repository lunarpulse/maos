#![forbid(unsafe_code)]

//! `maos-secrets` — secret-provider adapters (NFR-Sec-16) and the Story 11.4c
//! reference local-master-key KMS for opt-in at-rest AEAD.
//!
//! The local master-key adapter is **dev/CI-only**. Production KMS/keyring/cloud
//! adapters are additive per `KeyManagementPort` and deferred by ADR-051.

#[cfg(all(feature = "kms-fault-inject", not(debug_assertions)))]
compile_error!("kms-fault-inject is dev/CI-only and MUST NOT ship in release builds");

use maos_domain::ports::{
    CryptoProvider, KeyManagementPort, KmsError, SecretDeleteStatus, SecretKey, SecretStore,
    SecretStoreError,
};
use ring::aead;
use std::collections::HashMap;
use std::path::Path;
#[cfg(feature = "encrypted-file")]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Keyring,
    Env,
    EncryptedFile,
}

impl std::str::FromStr for Backend {
    type Err = SecretStoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "keyring" => Ok(Self::Keyring),
            "env" => Ok(Self::Env),
            "encrypted-file" => Ok(Self::EncryptedFile),
            other => Err(SecretStoreError::Unavailable(format!(
                "unknown secret backend '{other}'"
            ))),
        }
    }
}

/// Process-environment values captured at the composition root.
///
/// This adapter deliberately performs no environment reads of its own.
pub struct EnvSecretStore {
    values: Mutex<HashMap<SecretKey, String>>,
}

impl EnvSecretStore {
    pub fn new(values: impl IntoIterator<Item = (SecretKey, String)>) -> Self {
        Self {
            values: Mutex::new(values.into_iter().collect()),
        }
    }
}

impl SecretStore for EnvSecretStore {
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError> {
        Ok(self
            .values
            .lock()
            .map_err(|_| SecretStoreError::Access("environment store lock poisoned".to_string()))?
            .get(&key)
            .cloned())
    }
    fn put(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError> {
        self.values
            .lock()
            .map_err(|_| SecretStoreError::Access("environment store lock poisoned".to_string()))?
            .insert(key, value.to_string());
        Ok(())
    }

    fn delete(&self, key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError> {
        let removed = self
            .values
            .lock()
            .map_err(|_| SecretStoreError::Access("environment store lock poisoned".to_string()))?
            .remove(&key)
            .is_some();
        Ok(if removed {
            SecretDeleteStatus::Removed
        } else {
            SecretDeleteStatus::Absent
        })
    }

    fn is_healthy(&self) -> bool {
        self.values.lock().is_ok()
    }
}

pub type FallbackObserver = Arc<dyn Fn(SecretKey, &str) + Send + Sync>;

/// Resolve a primary store first, then a composition-root-captured fallback.
pub struct FallbackSecretStore {
    primary: Arc<dyn SecretStore>,
    fallback: Arc<dyn SecretStore>,
    on_fallback: FallbackObserver,
}

impl FallbackSecretStore {
    pub fn new(
        primary: Arc<dyn SecretStore>,
        fallback: Arc<dyn SecretStore>,
        on_fallback: FallbackObserver,
    ) -> Self {
        Self {
            primary,
            fallback,
            on_fallback,
        }
    }
}

impl SecretStore for FallbackSecretStore {
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError> {
        let (reason, primary_error) = match self.primary.get(key) {
            Ok(Some(value)) if !value.trim().is_empty() => return Ok(Some(value)),
            Ok(Some(_)) => ("primary entry present but blank".to_string(), None),
            Ok(None) => ("primary entry absent".to_string(), None),
            Err(error) => (error.to_string(), Some(error)),
        };
        match self.fallback.get(key) {
            Ok(Some(value)) if !value.trim().is_empty() => {
                (self.on_fallback)(key, &reason);
                Ok(Some(value))
            }
            Ok(_) => match primary_error {
                Some(error) => Err(error),
                None => Ok(None),
            },
            Err(fallback_error) => match primary_error {
                Some(primary_error) => {
                    // Both stores failed: this is the one terminal path where
                    // the observer has not yet journaled the primary failure,
                    // so record it before surfacing an error that still
                    // carries the primary's cause (the composition root turns
                    // it into a provider-less boot, never an exit).
                    (self.on_fallback)(key, &reason);
                    Err(SecretStoreError::Access(format!(
                        "primary lookup failed: {primary_error}; \
                         fallback lookup failed: {fallback_error}"
                    )))
                }
                None => Err(fallback_error),
            },
        }
    }

    fn put(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError> {
        self.primary.put(key, value)
    }

    fn delete(&self, key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError> {
        let primary = self.primary.delete(key);
        let fallback = self.fallback.delete(key);
        match (primary, fallback) {
            (Ok(primary), Ok(fallback)) => Ok(
                if primary == SecretDeleteStatus::Removed || fallback == SecretDeleteStatus::Removed
                {
                    SecretDeleteStatus::Removed
                } else {
                    SecretDeleteStatus::Absent
                },
            ),
            (Err(primary), Ok(_)) => Err(primary),
            (Ok(_), Err(fallback)) => Err(fallback),
            (Err(primary), Err(fallback)) => Err(SecretStoreError::Access(format!(
                "primary deletion failed: {primary}; fallback deletion failed: {fallback}"
            ))),
        }
    }

    fn is_healthy(&self) -> bool {
        self.primary.is_healthy() || self.fallback.is_healthy()
    }
}

#[cfg(feature = "keyring")]
pub struct KeyringSecretStore {
    namespace: String,
    timeout: std::time::Duration,
}

#[cfg(feature = "keyring")]
impl KeyringSecretStore {
    pub fn for_home(home: &Path) -> Self {
        use sha2::{Digest, Sha256};
        #[cfg(unix)]
        let digest = {
            use std::os::unix::ffi::OsStrExt;
            Sha256::digest(home.as_os_str().as_bytes())
        };
        #[cfg(windows)]
        let digest = {
            use std::os::windows::ffi::OsStrExt;
            let native: Vec<u8> = home
                .as_os_str()
                .encode_wide()
                .flat_map(u16::to_le_bytes)
                .collect();
            Sha256::digest(&native)
        };
        #[cfg(not(any(unix, windows)))]
        let digest = Sha256::digest(home.as_os_str().to_string_lossy().as_bytes());
        Self {
            namespace: hex_digest(&digest[..16]),
            timeout: std::time::Duration::from_millis(500),
        }
    }

    fn username(&self, key: SecretKey) -> String {
        format!("{}:{}", self.namespace, key.as_str())
    }

    fn bounded<T: Send + 'static>(
        &self,
        operation: impl FnOnce() -> Result<T, SecretStoreError> + Send + 'static,
    ) -> Result<T, SecretStoreError> {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("maos-keyring".to_string())
            .spawn(move || {
                let _ = sender.send(operation());
            })
            .map_err(|error| SecretStoreError::Unavailable(error.to_string()))?;
        match receiver.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(SecretStoreError::Unavailable(
                "credential-store operation timed out".to_string(),
            )),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(SecretStoreError::Access(
                "credential-store worker terminated".to_string(),
            )),
        }
    }
}

#[cfg(feature = "keyring")]
impl SecretStore for KeyringSecretStore {
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError> {
        let username = self.username(key);
        self.bounded(move || {
            let entry = keyring::Entry::new("dev.maos.credentials", &username)
                .map_err(|error| SecretStoreError::Unavailable(error.to_string()))?;
            match entry.get_password() {
                Ok(value) => Ok(Some(value)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(keyring::Error::BadEncoding(_)) => Err(SecretStoreError::InvalidEncoding),
                Err(error) => Err(SecretStoreError::Access(error.to_string())),
            }
        })
    }
    fn put(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError> {
        let username = self.username(key);
        let value = value.to_string();
        self.bounded(move || {
            let entry = keyring::Entry::new("dev.maos.credentials", &username)
                .map_err(|error| SecretStoreError::Unavailable(error.to_string()))?;
            entry
                .set_password(&value)
                .map_err(|error| SecretStoreError::Access(error.to_string()))
        })
    }

    fn delete(&self, key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError> {
        let username = self.username(key);
        self.bounded(move || {
            let entry = keyring::Entry::new("dev.maos.credentials", &username)
                .map_err(|error| SecretStoreError::Unavailable(error.to_string()))?;
            match entry.delete_credential() {
                Ok(()) => Ok(SecretDeleteStatus::Removed),
                Err(keyring::Error::NoEntry) => Ok(SecretDeleteStatus::Absent),
                Err(error) => Err(SecretStoreError::Access(error.to_string())),
            }
        })
    }

    fn is_healthy(&self) -> bool {
        self.bounded(|| {
            keyring::Entry::store_status()
                .as_ref()
                .map(|()| true)
                .map_err(|error| SecretStoreError::Unavailable(error.to_string()))
        })
        .unwrap_or(false)
    }
}

#[cfg(feature = "encrypted-file")]
pub struct EncryptedFileSecretStore {
    root: PathBuf,
    kms: Arc<dyn KeyManagementPort>,
    crypto: Arc<dyn CryptoProvider>,
}

#[cfg(feature = "encrypted-file")]
impl EncryptedFileSecretStore {
    pub fn new(
        root: PathBuf,
        kms: Arc<dyn KeyManagementPort>,
        crypto: Arc<dyn CryptoProvider>,
    ) -> Self {
        Self { root, kms, crypto }
    }

    pub fn store(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError> {
        use std::io::Write;
        create_private_directory(&self.root)?;
        let path = self.path(key);
        let sealed = seal_at_rest(self.kms.as_ref(), self.crypto.as_ref(), value.as_bytes())
            .map_err(|error| SecretStoreError::Access(error.to_string()))?;
        let mut temporary = tempfile::Builder::new()
            .prefix(".maos-secret-")
            .tempfile_in(&self.root)
            .map_err(|error| SecretStoreError::Access(error.to_string()))?;
        temporary
            .write_all(&sealed)
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|error| SecretStoreError::Access(error.to_string()))?;
        temporary
            .persist(&path)
            .map_err(|error| SecretStoreError::Access(error.error.to_string()))?;
        sync_directory(&self.root)
    }

    fn path(&self, key: SecretKey) -> PathBuf {
        self.root.join(format!("{}.sealed", key.as_str()))
    }
}

#[cfg(feature = "encrypted-file")]
impl SecretStore for EncryptedFileSecretStore {
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError> {
        let path = self.path(key);
        match std::fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(SecretStoreError::Access(error.to_string())),
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(SecretStoreError::Access(
                    "encrypted secret path is a symbolic link".to_string(),
                ));
            }
            Ok(_) => {}
        }
        let sealed =
            std::fs::read(&path).map_err(|error| SecretStoreError::Access(error.to_string()))?;
        let plaintext = open_at_rest(self.kms.as_ref(), &sealed)
            .map_err(|error| SecretStoreError::Access(error.to_string()))?;
        String::from_utf8(plaintext)
            .map(Some)
            .map_err(|_| SecretStoreError::InvalidEncoding)
    }
    fn put(&self, key: SecretKey, value: &str) -> Result<(), SecretStoreError> {
        self.store(key, value)
    }

    fn delete(&self, key: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError> {
        let path = self.path(key);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(SecretDeleteStatus::Removed),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(SecretDeleteStatus::Absent)
            }
            Err(error) => Err(SecretStoreError::Access(error.to_string())),
        }
    }

    fn is_healthy(&self) -> bool {
        self.kms.is_healthy()
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(feature = "encrypted-file")]
fn create_private_directory(path: &Path) -> Result<(), SecretStoreError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .map_err(|error| SecretStoreError::Access(error.to_string()))
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(path).map_err(|error| SecretStoreError::Access(error.to_string()))
    }
}

#[cfg(all(feature = "encrypted-file", unix))]
fn sync_directory(path: &Path) -> Result<(), SecretStoreError> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| SecretStoreError::Access(error.to_string()))
}

#[cfg(all(feature = "encrypted-file", not(unix)))]
fn sync_directory(_path: &Path) -> Result<(), SecretStoreError> {
    Ok(())
}

const MAGIC: &[u8; 8] = b"MAOSKMS1";
const NONCE_LEN: usize = 12;
const DATA_KEY_LEN: usize = 32;

#[derive(Debug, Clone)]
pub struct LocalMasterKeyKms {
    master_key: [u8; DATA_KEY_LEN],
}

impl LocalMasterKeyKms {
    pub fn from_master_key(master_key: &[u8]) -> Result<Self, KmsError> {
        let master_key: [u8; DATA_KEY_LEN] = master_key
            .try_into()
            .map_err(|_| KmsError::MalformedKey("local master key must be 32 bytes"))?;
        Ok(Self { master_key })
    }
}

impl KeyManagementPort for LocalMasterKeyKms {
    fn wrap_data_key(&self, data_key: &[u8]) -> Result<Vec<u8>, KmsError> {
        if data_key.len() != DATA_KEY_LEN {
            return Err(KmsError::MalformedKey("data key must be 32 bytes"));
        }
        let nonce = random_nonce()?;
        let ciphertext = seal_aead(&self.master_key, &nonce, b"maos.kms.wrap.v1", data_key)?;
        let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn unwrap_data_key(&self, wrapped_data_key: &[u8]) -> Result<Vec<u8>, KmsError> {
        if wrapped_data_key.len() <= NONCE_LEN {
            return Err(KmsError::MalformedPayload("wrapped data key too short"));
        }
        let (nonce, ciphertext) = wrapped_data_key.split_at(NONCE_LEN);
        open_aead(&self.master_key, nonce, b"maos.kms.wrap.v1", ciphertext)
    }

    fn is_healthy(&self) -> bool {
        self.master_key.iter().any(|byte| *byte != 0)
    }
}

/// Seal an adapter-store row with an envelope data key wrapped by KMS.
pub fn seal_at_rest(
    kms: &dyn KeyManagementPort,
    crypto: &dyn CryptoProvider,
    plaintext: &[u8],
) -> Result<Vec<u8>, KmsError> {
    #[cfg(feature = "kms-fault-inject")]
    {
        let _ = (kms, crypto);
        return Ok(plaintext.to_vec());
    }

    #[cfg(not(feature = "kms-fault-inject"))]
    {
        if !kms.is_healthy() {
            return Err(KmsError::Unavailable("KMS is unhealthy".to_string()));
        }
        let data_key = random_data_key()?;
        let wrapped_data_key = kms.wrap_data_key(&data_key)?;
        let row_nonce = random_nonce()?;
        let ciphertext = crypto
            .seal_for_export(&data_key, &row_nonce, b"maos.at-rest.v1", plaintext)
            .map_err(|e| KmsError::Crypto(e.to_string()))?;

        let wrapped_len = u16::try_from(wrapped_data_key.len())
            .map_err(|_| KmsError::MalformedPayload("wrapped key too large"))?;
        let mut out = Vec::with_capacity(
            MAGIC.len() + 2 + wrapped_data_key.len() + NONCE_LEN + ciphertext.len(),
        );
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&wrapped_len.to_be_bytes());
        out.extend_from_slice(&wrapped_data_key);
        out.extend_from_slice(&row_nonce);
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }
}

/// Opt-in at-rest seal posture.
///
/// `None` preserves the ratified Option-A plaintext default exactly; `Some`
/// delegates to the real envelope seal and therefore produces ciphertext.
pub fn seal_at_rest_opt(
    kms: Option<&dyn KeyManagementPort>,
    crypto: &dyn CryptoProvider,
    plaintext: &[u8],
) -> Result<Vec<u8>, KmsError> {
    match kms {
        Some(kms) => seal_at_rest(kms, crypto, plaintext),
        None => Ok(plaintext.to_vec()),
    }
}

/// Open an adapter-store row sealed by [`seal_at_rest`].
///
/// **Seal/open asymmetry (intentional, see review patch P2).** `seal_at_rest`
/// routes the AEAD through the injected `CryptoProvider::seal_for_export`
/// (polymorphic — a future FIPS/HSM provider substitutes at the seal site),
/// while `open_at_rest` pins the AEAD to ring `AES_256_GCM` directly via the
/// private `open_aead`. Routing open through the trait would require adding
/// `CryptoProvider::open_for_export` AND implementing it on `RingCryptoProvider`,
/// but `RingCryptoProvider` lives in `maos-kernel-core`, which is **ZERO-delta
/// (L1)** and MUST NOT be touched. The pin is safe today because the only
/// seal-time provider is the reference `RingCryptoProvider`, whose
/// `seal_for_export` uses exactly ring `AES_256_GCM` — so this open path is its
/// matching inverse. A future FIPS/HSM provider substitution would need a
/// matching additive open path (deferred, tracked by ADR-051).
pub fn open_at_rest(kms: &dyn KeyManagementPort, sealed: &[u8]) -> Result<Vec<u8>, KmsError> {
    if !kms.is_healthy() {
        return Err(KmsError::Unavailable("KMS is unhealthy".to_string()));
    }
    let parsed = ParsedSealedPayload::parse(sealed)?;
    let data_key = kms.unwrap_data_key(parsed.wrapped_data_key)?;
    open_aead(
        &data_key,
        parsed.row_nonce,
        b"maos.at-rest.v1",
        parsed.ciphertext,
    )
}

struct ParsedSealedPayload<'a> {
    wrapped_data_key: &'a [u8],
    row_nonce: &'a [u8],
    ciphertext: &'a [u8],
}

impl<'a> ParsedSealedPayload<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self, KmsError> {
        if bytes.len() < MAGIC.len() + 2 + NONCE_LEN + 1 {
            return Err(KmsError::MalformedPayload("sealed payload too short"));
        }
        if &bytes[..MAGIC.len()] != MAGIC {
            return Err(KmsError::MalformedPayload("sealed payload magic mismatch"));
        }
        let len_offset = MAGIC.len();
        let wrapped_len = u16::from_be_bytes([bytes[len_offset], bytes[len_offset + 1]]) as usize;
        let wrapped_start = len_offset + 2;
        let wrapped_end = wrapped_start + wrapped_len;
        let nonce_end = wrapped_end + NONCE_LEN;
        if wrapped_len == 0 || nonce_end >= bytes.len() {
            return Err(KmsError::MalformedPayload("sealed payload lengths invalid"));
        }
        Ok(Self {
            wrapped_data_key: &bytes[wrapped_start..wrapped_end],
            row_nonce: &bytes[wrapped_end..nonce_end],
            ciphertext: &bytes[nonce_end..],
        })
    }
}

fn random_data_key() -> Result<[u8; DATA_KEY_LEN], KmsError> {
    let mut key = [0u8; DATA_KEY_LEN];
    getrandom::fill(&mut key).map_err(|e| KmsError::Unavailable(e.to_string()))?;
    Ok(key)
}

fn random_nonce() -> Result<[u8; NONCE_LEN], KmsError> {
    let mut nonce = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|e| KmsError::Unavailable(e.to_string()))?;
    Ok(nonce)
}

fn seal_aead(key: &[u8], nonce: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, KmsError> {
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| KmsError::MalformedKey("AES-256-GCM key rejected"))?;
    let key = aead::LessSafeKey::new(unbound);
    let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
        .map_err(|_| KmsError::Crypto("nonce construction failed".to_string()))?;
    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, aead::Aad::from(aad), &mut in_out)
        .map_err(|_| KmsError::Crypto("AES-GCM seal failed".to_string()))?;
    Ok(in_out)
}

fn open_aead(key: &[u8], nonce: &[u8], aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, KmsError> {
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| KmsError::MalformedKey("AES-256-GCM key rejected"))?;
    let key = aead::LessSafeKey::new(unbound);
    let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
        .map_err(|_| KmsError::Crypto("nonce construction failed".to_string()))?;
    let mut in_out = ciphertext.to_vec();
    let plaintext = key
        .open_in_place(nonce, aead::Aad::from(aad), &mut in_out)
        .map_err(|_| KmsError::Crypto("AES-GCM open failed".to_string()))?;
    Ok(plaintext.to_vec())
}

#[cfg(all(test, feature = "keyring", unix))]
mod tests {
    use super::KeyringSecretStore;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;

    #[test]
    fn keyring_namespace_hashes_native_non_utf8_home_bytes() {
        let first = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/maos-\xff".to_vec()));
        let second = PathBuf::from(std::ffi::OsString::from_vec(b"/tmp/maos-\xfe".to_vec()));
        assert_ne!(
            KeyringSecretStore::for_home(&first).namespace,
            KeyringSecretStore::for_home(&second).namespace
        );
    }
}
