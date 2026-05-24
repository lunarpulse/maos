//! Handler for `registry.yanks_since` (kernel-internal op).

use std::sync::Arc;

use crate::operations::YanksSinceArgs;
use crate::storage::RegistryStorage;

pub fn handle_yanks_since(
    storage: &Arc<dyn RegistryStorage>,
    args: &YanksSinceArgs,
) -> Result<serde_json::Value, String> {
    let list = storage
        .yanks_since(args.since_ns)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(list).map_err(|e| format!("serialize error: {e}"))
}
