//! Handler for `registry.publish`.

use std::sync::Arc;

use crate::storage::RegistryStorage;

use maos_domain::ports::registry::{PublishReceipt, SignedPackage};

pub fn handle_publish(
    storage: &Arc<dyn RegistryStorage>,
    pkg: &SignedPackage,
) -> Result<serde_json::Value, String> {
    if !crate::admission::verify_publisher_sig(pkg) {
        return Err("unauthenticated registry.publish: publisher signature invalid".into());
    }
    let document =
        toml::from_str(std::str::from_utf8(&pkg.manifest_toml).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    crate::install::check_identity(&document, pkg.spirit_id.as_str(), &pkg.version)
        .map_err(|e| e.to_string())?;
    storage
        .put(&pkg.spirit_id, &pkg.version, pkg)
        .map_err(|e| e.to_string())?;
    let receipt = PublishReceipt::new(
        format!("pub-{}", crate::storage::monotonic_now_ns()),
        pkg.spirit_id.clone(),
        pkg.version.clone(),
    );
    serde_json::to_value(receipt).map_err(|e| format!("serialize error: {e}"))
}
