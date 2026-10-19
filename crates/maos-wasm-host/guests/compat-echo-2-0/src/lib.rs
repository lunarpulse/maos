//! `compat-echo-2-0` — a `maos:spirit@2.0.0` echo guest frozen in time.
//!
//! Story 17.3d AC2.4: `wit/spirit.wit` here is a byte copy of the root WIT at
//! `39cd7966`, before `maos:spirit@2.1.0` added `interface recall`. It is never
//! regenerated: the test proves a guest built against the old contract still
//! runs, byte-identically, on the `@2.1` runner and never touches recall.

wit_bindgen::generate!({
    path: "wit/spirit.wit",
    world: "spirit",
});

struct CompatEcho;

impl Guest for CompatEcho {
    fn handle_frame(frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        Ok(vec![frame])
    }

    fn on_start() -> Result<(), Halt> {
        Ok(())
    }

    fn on_shutdown() {}
}

export!(CompatEcho);
