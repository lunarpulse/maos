use maos_domain::invariants::i1::{CapabilityToken, TokenId};
use maos_domain::ports::inference::{InferenceOptions, InferenceRequest};
use maos_domain::ports::io_subsystem::{HttpExchange, IoError};
use maos_domain::ports::{
    IoSubsystemPort, SecretDeleteStatus, SecretKey, SecretStore, SecretStoreError,
};
use std::sync::Mutex;

#[derive(Default)]
pub struct Recorded {
    pub url: String,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
}
pub struct MockTransport {
    pub recorded: Mutex<Recorded>,
    pub body: Vec<u8>,
    pub status: u16,
}
impl IoSubsystemPort for MockTransport {
    fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
        panic!("unexpected GET")
    }
    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        self.capture(url, body, headers);
        Ok(self.body.clone())
    }
    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        _: &str,
    ) -> Result<HttpExchange, IoError> {
        self.capture(url, body, headers);
        Ok(HttpExchange {
            status: self.status,
            body: self.body.clone(),
            header: None,
        })
    }
}
impl MockTransport {
    fn capture(&self, url: &str, body: &[u8], headers: &[(&str, &str)]) {
        *self.recorded.lock().unwrap_or_else(|e| e.into_inner()) = Recorded {
            url: url.into(),
            body: body.into(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
    }
}
pub struct Store(pub SecretKey, pub String);
impl SecretStore for Store {
    fn get(&self, key: SecretKey) -> Result<Option<String>, SecretStoreError> {
        assert_eq!(key, self.0);
        Ok(Some(self.1.clone()))
    }
    fn put(&self, _: SecretKey, _: &str) -> Result<(), SecretStoreError> {
        panic!("unexpected put")
    }
    fn delete(&self, _: SecretKey) -> Result<SecretDeleteStatus, SecretStoreError> {
        panic!("unexpected delete")
    }
    fn is_healthy(&self) -> bool {
        true
    }
}
pub fn request() -> InferenceRequest {
    InferenceRequest::new(
        1,
        CapabilityToken::new(TokenId::ZERO, 1, 0, [0; 64]),
        "Hello".into(),
        InferenceOptions {
            max_tokens: 42,
            temperature: Some(0.25),
            ..Default::default()
        },
        None,
        vec![],
    )
}
