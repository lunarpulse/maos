#![forbid(unsafe_code)]

//! `maos-bin` — the MAOS Host executable.
//!
//! At v0.1-α this is a placeholder stub. Story 1a.2 wires the
//! `#[tokio::main(flavor = "multi_thread")]` composition root.

fn main() {
    println!(
        "maos {} (v0.1-α scaffold; Story 1a.2 wires the composition root)",
        env!("CARGO_PKG_VERSION")
    );
}
