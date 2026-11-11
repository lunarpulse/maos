//! GPT round 3 P2: `invoke_halt` writes its halt row with a kernel-minted id
//! through `insert_frame_event_with_id`, whose `Duplicate` outcome means "not
//! written". I2 (log before deliver) requires the kernel to fail stop on it, as
//! the old `insert_frame_event` did, instead of journaling and registering a
//! halt whose row was never written.
//!
//! A minted id is `Ulid::new()` (random, nothing exposes it before the insert),
//! so a collision cannot be planted deterministically without a kernel test seam
//! the line budget does not allow; the Duplicate outcome itself is pinned at the
//! TL layer. This witness therefore reads the production source: the write's
//! result must be bound and compared with `FrameRowWrite::Written` in an assert.

#[test]
fn invoke_halt_fails_stop_unless_its_halt_row_was_written() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../maos-kernel-core/src/halt/mod.rs"),
    )
    .expect("kernel halt source");
    let syntax = syn::parse_file(&source).expect("kernel halt syntax");
    let function = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "invoke_halt" => Some(function),
            _ => None,
        })
        .expect("invoke_halt");
    let mut bound = false;
    let mut checked = false;
    for statement in &function.block.stmts {
        match statement {
            syn::Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    let mut expression = init.expr.as_ref();
                    let mut consumed = false;
                    while let syn::Expr::MethodCall(call) = expression {
                        consumed |= call.method == "into_inner";
                        if call.method == "insert_frame_event_with_id" {
                            bound |= consumed;
                        }
                        expression = call.receiver.as_ref();
                    }
                }
            }
            syn::Stmt::Macro(mac) => {
                let name = mac.mac.path.segments.last().map(|s| s.ident.to_string());
                let tokens = mac.mac.tokens.to_string();
                checked |= matches!(name.as_deref(), Some("assert" | "assert_eq"))
                    && tokens.contains("FrameRowWrite :: Written");
            }
            _ => {}
        }
    }
    assert!(
        bound,
        "the halt row write's FrameRowWrite result must be bound"
    );
    assert!(
        checked,
        "an assert must require FrameRowWrite::Written (I2 fail-stop)"
    );
}
