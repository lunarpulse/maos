//! Handler for `registry.artifact`.

use std::sync::Arc;

use crate::operations::ArtifactArgs;
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::SpiritId;

pub fn handle_artifact(
    storage: &Arc<dyn RegistryStorage>,
    args: &ArtifactArgs,
) -> Result<serde_json::Value, String> {
    let spirit_id = SpiritId::from(args.spirit_id.as_str());
    let artifact = storage
        .get_artifact(&spirit_id, &args.version)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(artifact).map_err(|e| format!("serialize error: {e}"))
}
