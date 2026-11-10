//! FR48 crypto-provider selection, shared by all daemon consumers.
use std::path::Path;
use std::sync::Arc;

use maos_domain::ports::CryptoProvider;

pub struct ProviderRow {
    pub name: &'static str,
    pub version: &'static str,
    build: fn(&Path) -> Arc<dyn CryptoProvider>,
}

pub const PROVIDERS: &[ProviderRow] = &[
    ProviderRow {
        name: "ring",
        version: "0.17.14",
        build: |_| Arc::new(maos_kernel_core::security::crypto::RingCryptoProvider),
    },
    #[cfg(feature = "crypto-recording")]
    ProviderRow {
        name: "recording",
        version: "test",
        build: |home| Arc::new(recording::RecordingCryptoProvider::new(home)),
    },
];

#[derive(Clone)]
pub struct SelectedCrypto {
    pub name: &'static str,
    pub version: &'static str,
    pub defaulted: bool,
    pub provider: Arc<dyn CryptoProvider>,
}

pub fn known_names() -> String {
    PROVIDERS
        .iter()
        .map(|row| row.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn validate(name: &str) -> Result<&'static ProviderRow, String> {
    PROVIDERS
        .iter()
        .find(|row| row.name == name)
        .ok_or_else(|| {
            format!(
                "unknown crypto provider '{name}' (known: {})",
                known_names()
            )
        })
}

pub fn select(home: &Path) -> Result<SelectedCrypto, String> {
    let path = home.join("config.toml");
    let error = |detail: String| {
        format!(
            "{}: {detail} (known providers: {})",
            path.display(),
            known_names()
        )
    };
    let recorded = match std::fs::read_to_string(&path) {
        Ok(text) => {
            let doc: toml::Value = toml::from_str(&text).map_err(|e| error(format!("{e}")))?;
            match doc.get("crypto") {
                None => None,
                Some(table) => Some(
                    table
                        .get("provider")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| error("[crypto] provider must be a string".into()))?
                        .to_owned(),
                ),
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(error(e.to_string())),
    };
    let row = validate(recorded.as_deref().unwrap_or("ring"))?;
    Ok(SelectedCrypto {
        name: row.name,
        version: row.version,
        defaulted: recorded.is_none(),
        provider: (row.build)(home),
    })
}

pub fn journal(log: &maos_kernel_core::iac::TransparencyLogAdapter, selected: &SelectedCrypto) {
    let payload = serde_json::json!({
        "provider": selected.name,
        "version": selected.version,
        "source": if selected.defaulted { "default" } else { "config" },
    });
    let bytes = payload.to_string();
    // The I2 writer returns a proof token, not a Result: any storage failure
    // halts before this boot can expose consumers or an unjournaled selection.
    let _token = log.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::TelemetryEvent,
        0,
        None,
        "crypto.provider.selected",
        bytes.as_bytes(),
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );
}

#[cfg(feature = "crypto-recording")]
mod recording {
    #[cfg(not(debug_assertions))]
    compile_error!("crypto-recording is dev/CI-only and MUST NOT ship in release builds");

    use maos_domain::ports::crypto::{CryptoError, CryptoProvider};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

    pub struct RecordingCryptoProvider {
        log: PathBuf,
        identity_log: PathBuf,
        instance: u64,
        inner: maos_kernel_core::security::crypto::RingCryptoProvider,
    }
    impl RecordingCryptoProvider {
        pub fn new(home: &Path) -> Self {
            Self {
                log: home.join("crypto-recording.log"),
                identity_log: home.join("crypto-recording-identity.log"),
                instance: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed),
                inner: maos_kernel_core::security::crypto::RingCryptoProvider,
            }
        }
        fn record(&self, operation: &str) {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log)
            {
                let _ = writeln!(file, "{operation}");
            }
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.identity_log)
            {
                let _ = writeln!(file, "{}:{} {operation}", std::process::id(), self.instance);
            }
        }
    }
    impl CryptoProvider for RecordingCryptoProvider {
        fn verify_signature(
            &self,
            key: &[u8],
            message: &[u8],
            signature: &[u8],
        ) -> Result<(), CryptoError> {
            self.record("verify_signature");
            self.inner.verify_signature(key, message, signature)
        }
        fn seal_for_export(
            &self,
            key: &[u8],
            nonce: &[u8],
            aad: &[u8],
            plaintext: &[u8],
        ) -> Result<Vec<u8>, CryptoError> {
            self.record("seal_for_export");
            self.inner.seal_for_export(key, nonce, aad, plaintext)
        }
        fn sign_capability_token(&self, key: &[u8], token: &[u8]) -> Result<Vec<u8>, CryptoError> {
            self.record("sign_capability_token");
            self.inner.sign_capability_token(key, token)
        }
    }
}
