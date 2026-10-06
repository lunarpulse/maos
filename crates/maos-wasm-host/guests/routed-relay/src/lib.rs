wit_bindgen::generate!({
    path: "../../../../wit/spirit.wit",
    world: "spirit",
});

struct RoutedRelay;

impl Guest for RoutedRelay {
    fn handle_frame(mut frame: IacFrame) -> Result<Vec<IacFrame>, Halt> {
        // The first address is the inbound guest; remaining addresses are the
        // delegated destinations. Do not feed replies into our own mailbox.
        if !frame.to.is_empty() {
            frame.to.remove(0);
        }
        Ok(vec![frame])
    }

    fn on_start() -> Result<(), Halt> { Ok(()) }
    fn on_shutdown() {}
}

export!(RoutedRelay);
