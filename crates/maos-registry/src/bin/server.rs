//! Standalone development registry (loopback only until 20-1b authenticates mutations).
use maos_registry::server::RegistryServerArgs;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1).peekable();
    if args
        .peek()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: maos-registry-server [--bind <addr>] [--root <dir>] [--signing-key <path>]"
        );
        println!("Loopback development server; mutation authentication is owned by story 20-1b.");
        return std::process::ExitCode::SUCCESS;
    }
    let result = async { RegistryServerArgs::parse(args)?.build()?.run().await }.await;
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("maos-registry-server: {error}");
            std::process::ExitCode::from(2)
        }
    }
}
