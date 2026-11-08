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
    let mut list = storage.yanks_since(0).map_err(|e| e.to_string())?;
    let head = list.entries.last().map_or(0, |entry| entry.sequence);
    list.entries.retain(|entry| entry.sequence > args.since_ns);
    let mut value = serde_json::to_value(list).map_err(|e| format!("serialize error: {e}"))?;
    value
        .as_object_mut()
        .ok_or("invalid yank response")?
        .insert("head".into(), head.into());
    Ok(value)
}
