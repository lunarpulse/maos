//! The admitted-model gate (S1, operator ruling Q6): a driver serves any
//! admitted model of its provider — the configured default plus every model the
//! composition root admitted for it — and refuses any other requested model
//! before a transport call.

use maos_domain::ports::inference::{InferenceRequest, InferenceResponse};

use crate::provider::{Provider, ProviderError};

/// Wraps a driver so a request naming `options.model_id` outside `admitted` is
/// refused without touching the wire. A request naming no model is the
/// driver's configured default.
pub struct AdmittedModels<P> {
    inner: P,
    admitted: Vec<String>,
}

impl<P: Provider> AdmittedModels<P> {
    pub fn new(inner: P, admitted: Vec<String>) -> Self {
        Self { inner, admitted }
    }
}

impl<P: Provider> Provider for AdmittedModels<P> {
    fn complete(&self, req: &InferenceRequest) -> Result<InferenceResponse, ProviderError> {
        if let Some(model) = req.options.model_id.as_deref() {
            if !self.admitted.iter().any(|admitted| admitted == model) {
                return Err(ProviderError::Serde(format!(
                    "model `{model}` is not admitted for this provider"
                )));
            }
        }
        self.inner.complete(req)
    }

    fn credential_fingerprint(&self) -> u64 {
        self.inner.credential_fingerprint()
    }
}
