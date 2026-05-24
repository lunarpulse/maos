//! Handler for `registry.deprecate`.

use std::sync::Arc;

use crate::operations::DeprecateArgs;
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::{SpiritId, YankReason};

pub fn handle_deprecate(
    storage: &Arc<dyn RegistryStorage>,
    args: &DeprecateArgs,
) -> Result<serde_json::Value, String> {
    let spirit_id = SpiritId::from(args.spirit_id.as_str());
    let reason = YankReason::new(args.reason.clone());
    let receipt = storage
        .yank(&spirit_id, &args.version, &reason)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(receipt).map_err(|e| format!("serialize error: {e}"))
}
