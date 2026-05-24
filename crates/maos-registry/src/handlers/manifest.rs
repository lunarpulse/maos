//! Handler for `registry.manifest`.

use std::sync::Arc;

use crate::operations::ManifestArgs;
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::SpiritId;

pub fn handle_manifest(
    storage: &Arc<dyn RegistryStorage>,
    args: &ManifestArgs,
) -> Result<serde_json::Value, String> {
    let spirit_id = SpiritId::from(args.spirit_id.as_str());
    let manifest = storage
        .get_manifest(&spirit_id, &args.version)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(manifest).map_err(|e| format!("serialize error: {e}"))
}
