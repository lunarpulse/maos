#![forbid(unsafe_code)]

//! `maos-secrets` — secret-provider adapters (NFR-Sec-16) and the Story 11.4c
//! reference local-master-key KMS for opt-in at-rest AEAD.
//!
//! The local master-key adapter is **dev/CI-only**. Production KMS/keyring/cloud
//! adapters are additive per `KeyManagementPort` and deferred by ADR-051.

#[cfg(all(feature = "kms-fault-inject", not(debug_assertions)))]
compile_error!("kms-fault-inject is dev/CI-only and MUST NOT ship in release builds");

use maos_domain::ports::{CryptoProvider, KeyManagementPort, KmsError};
use ring::aead;

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
pub fn open_at_rest(
    kms: &dyn KeyManagementPort,
    sealed: &[u8],
) -> Result<Vec<u8>, KmsError> {
    if !kms.is_healthy() {
        return Err(KmsError::Unavailable("KMS is unhealthy".to_string()));
    }
    let parsed = ParsedSealedPayload::parse(sealed)?;
    let data_key = kms.unwrap_data_key(parsed.wrapped_data_key)?;
    open_aead(&data_key, parsed.row_nonce, b"maos.at-rest.v1", parsed.ciphertext)
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
