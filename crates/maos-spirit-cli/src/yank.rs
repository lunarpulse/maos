//! Publisher/vetter signed withdrawal over the existing registry client.
use clap::Args;
use maos_domain::ports::registry::{SpiritId, SpiritRegistryClient, YankOrigin, YankReason};
#[derive(Debug, Args)]
pub struct YankArgs {
    pub spec: String,
    #[arg(long)]
    pub registry_uri: String,
    #[arg(long)]
    pub signing_key: std::path::PathBuf,
    #[arg(long)]
    pub reason: String,
    /// Present only for an enrolled vetter withdrawing a vetted release.
    #[arg(long)]
    pub vetter_key_id: Option<String>,
}
pub fn run(args: YankArgs) -> anyhow::Result<()> {
    let (name, version) = args
        .spec
        .rsplit_once('@')
        .ok_or_else(|| anyhow::anyhow!("expected name@version"))?;
    let seed = crate::signing::load_signing_seed(&Some(args.signing_key), &None)?;
    let origin = if args.vetter_key_id.is_some() {
        YankOrigin::Vetter
    } else {
        YankOrigin::Publisher
    };
    let mut reason = YankReason::new(args.reason);
    reason.authorization = Some(
        maos_registry::yank_auth::YankAuthorization::sign(
            &seed,
            maos_registry::yank_auth::YankScope {
                name,
                version,
                reason: &reason.summary,
                registry_uri: &args.registry_uri,
            },
            origin,
            args.vetter_key_id.unwrap_or_default(),
            maos_registry::yank_auth::now_unix_ms().map_err(anyhow::Error::msg)?,
        )
        .map_err(anyhow::Error::msg)?,
    );
    let client = maos_registry::McpSpiritRegistryClient::http(
        args.registry_uri,
        maos_registry::install::operator_config().map_err(anyhow::Error::from_boxed)?,
    )?;
    let receipt = client.deprecate(&SpiritId::from(name), version, &reason)?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
