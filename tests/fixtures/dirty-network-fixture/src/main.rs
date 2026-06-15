//! Dirty fixture: intentionally links `tokio::net::TcpStream` to prove the
//! `check-air-gap` gate can detect network surface in a binary's symbol table.
//! This fixture MUST be rejected by the gate; if it isn't, the gate is broken.

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Force the linker to include TcpStream::connect in the binary.
    match tokio::net::TcpStream::connect("127.0.0.1:0").await {
        Ok(_stream) => eprintln!("connected (should never happen on port 0)"),
        Err(e) => eprintln!("expected connect failure: {e}"),
    }
}
