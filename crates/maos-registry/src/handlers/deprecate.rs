//! Handler for `registry.deprecate`.

use std::sync::Arc;

use crate::operations::DeprecateArgs;
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::{SpiritId, YankReason};

pub fn handle_deprecate(
    storage: &Arc<dyn RegistryStorage>,
    args: &DeprecateArgs,
    registry_uri: &str,
    vetters: Option<(&maos_compliance::VetterKeyring, &[u8; 32])>,
) -> Result<serde_json::Value, String> {
    let spirit_id = SpiritId::from(args.spirit_id.as_str());
    let mut reason = YankReason::new(args.reason.clone());
    let bytes = args
        .authorization
        .as_ref()
        .ok_or("unauthenticated registry.deprecate: signed authorization required")?;
    let auth = crate::yank_auth::decode(bytes)?;
    let manifest = storage
        .get_manifest(&spirit_id, &args.version)
        .map_err(|e| e.to_string())?;
    auth.verify(
        crate::yank_auth::YankScope {
            name: &args.spirit_id,
            version: &args.version,
            reason: &args.reason,
            registry_uri,
        },
        &manifest.signer_pubkey,
        vetters,
        crate::yank_auth::now_unix_ms()?,
    )?;
    reason.authorization = Some(bytes.clone());
    let receipt = storage
        .yank(&spirit_id, &args.version, &reason)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(receipt).map_err(|e| format!("serialize error: {e}"))
}
