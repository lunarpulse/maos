//! Authenticated TLS registry, explicit loopback development, and local pin release.
use maos_registry::server::RegistryServerArgs;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1).peekable();
    if args
        .peek()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: maos-registry-server [--bind <addr>] [--root <dir>] --uri <https://host/mcp> --signing-key <hex-seed-path> --tls-cert <chain.pem> --tls-key <key.pem>\n       maos-registry-server [--bind <loopback>] [--root <dir>] --dev-http\n       maos-registry-server [--root <dir>] release-pin <name>"
        );
        println!("Installers configure the URI-bound signing public key and optional CA in operator.toml. HTTP requires explicit development opt-in at both ends.");
        return std::process::ExitCode::SUCCESS;
    }
    let result = async {
        let mut args = RegistryServerArgs::parse(args)?;
        if let Some(name) = args.release_pin.take() {
            if args.root.is_none() && std::env::var_os("HOME").is_none_or(|home| home.is_empty()) {
                return Err("release-pin requires --root or a nonempty HOME".into());
            }
            let storage = match args.root {
                Some(root) => maos_registry::storage::LocalFsRegistryStorage::at_path(root),
                None => maos_registry::storage::LocalFsRegistryStorage::new(),
            }
            .map_err(|e| e.to_string())?;
            let released = storage
                .release_installed_pin(&name)
                .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::json!({"outcome":"pin-release", "spirit_id":name, "released":released})
            );
            Ok(())
        } else {
            args.build()?.run().await
        }
    }
    .await;
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("maos-registry-server: {error}");
            std::process::ExitCode::from(2)
        }
    }
}
