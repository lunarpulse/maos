//! Handler for `registry.yanks_since` (kernel-internal op).

use std::sync::Arc;

use crate::operations::YanksSinceArgs;
use crate::storage::RegistryStorage;

pub fn handle_yanks_since(
    storage: &Arc<dyn RegistryStorage>,
    args: &YanksSinceArgs,
) -> Result<serde_json::Value, String> {
    // Head and entries share one durable snapshot; a concurrent append cannot
    // make a legitimate filtered response appear incomplete.
    let list = storage
        .yanks_since(args.since_ns)
        .map_err(|e| e.to_string())?;
    if list.head.is_none() {
        return Err("registry storage omitted the complete yank head".into());
    }
    serde_json::to_value(list).map_err(|e| format!("serialize error: {e}"))
}
