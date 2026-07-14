#![forbid(unsafe_code)]

use maos_domain::frame::{FramePayload, IacFrame, TelemetryEventPayload};
use maos_spirit_abi::identity::FrameKind;
use serde::{Deserialize, Serialize};

use crate::error::CohortError;
use crate::RESERVED_INTENT_REISSUE;

pub const CONTROL_EVENT_TYPE: &str = "maos.cohort-manifest.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CohortManifestControl {
    Push {
        manifest_toml: String,
    },
    Pull {
        known_version: u64,
        known_hash: String,
    },
}

impl CohortManifestControl {
    pub fn from_frame(frame: &IacFrame) -> Result<Self, CohortError> {
        let intent = frame
            .consent_envelope
            .as_ref()
            .and_then(|envelope| envelope.intent_class.as_ref())
            .map(|intent| intent.as_str());
        if intent != Some(RESERVED_INTENT_REISSUE) || frame.kind != FrameKind::TelemetryEvent {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        let FramePayload::TelemetryEvent(payload) = &frame.payload else {
            return Err(CohortError::EControlEnvelopeInvalid);
        };
        if payload.event_type != CONTROL_EVENT_TYPE {
            return Err(CohortError::EControlEnvelopeInvalid);
        }
        serde_json::from_str(&payload.data)
            .map_err(|error| CohortError::EControlEnvelopeDecode(error.to_string()))
    }

    pub fn telemetry_payload(&self) -> Result<TelemetryEventPayload, CohortError> {
        Ok(TelemetryEventPayload {
            event_type: CONTROL_EVENT_TYPE.into(),
            data: serde_json::to_string(self)
                .map_err(|error| CohortError::EControlEnvelopeDecode(error.to_string()))?,
        })
    }
}
