//! `echo-spirit` — minimal conformant `maos:spirit@1.0` guest.
//!
//! Crypto-free (D9) identity guest: `handle-frame` returns the inbound
//! frame unchanged (as the single emitted frame); `on-start`/`on-shutdown`
//! are no-ops. This is the AC3 byte-equal round-trip fixture — a REAL
//! component implementing the WIT world, not a host-side echo loop.

wit_bindgen::generate!({
    path: "../../../../wit/spirit.wit",
    world: "spirit",
});

struct EchoSpirit;

impl Guest for EchoSpirit {
    fn handle_frame(frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        Ok(vec![frame])
    }

    fn on_start() -> Result<(), Halt> {
        Ok(())
    }

    fn on_shutdown() {}
}

export!(EchoSpirit);
