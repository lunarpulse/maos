//! Handler for `registry.search`.

use std::sync::Arc;

use crate::operations::SearchArgs;
use crate::storage::RegistryStorage;

use maos_domain::ports::registry::SearchQuery;

pub fn handle_search(
    storage: &Arc<dyn RegistryStorage>,
    args: &SearchArgs,
) -> Result<serde_json::Value, String> {
    let q = SearchQuery::new(args.text.clone(), args.include_yanked, args.limit);
    let results = storage.search(&q).map_err(|e| e.to_string())?;
    serde_json::to_value(results).map_err(|e| format!("serialize error: {e}"))
}
