//! Wasmtime-generated host bindings for the `maos:spirit@2.4.0` worlds.
//!
//! Real component-model call path (Story 11.1a AC3) — NOT a hand-rolled
//! echo loop. The `bindgen!` macro reads `wit/spirit.wit` at compile time
//! and generates the `Spirit`/`SpiritPre` types + `call_handle_frame`/
//! `call_on_start`/`call_on_shutdown` typed methods the runner drives.

wasmtime::component::bindgen!({
    path: "../../wit/spirit.wit",
    world: "spirit",
});

/// The `spirit-lifecycle` world (Story 20-9): `spirit` plus the `lifecycle`
/// export. The shared interfaces stay the first world's types, so the frame
/// bridge and every host import exist once.
pub mod lifecycle {
    wasmtime::component::bindgen!({
        path: "../../wit/spirit.wit",
        world: "spirit-lifecycle",
        with: {
            "maos:spirit/frames": super::maos::spirit::frames,
            "maos:spirit/recall": super::maos::spirit::recall,
            "maos:spirit/context": super::maos::spirit::context,
            "maos:spirit/skills": super::maos::spirit::skills,
        },
    });
}

/// The 2.0/2.1 export schema has closed enums predating ADR-016. Import semver
/// compatibility alone cannot widen those guest-consumed export parameters.
pub mod legacy {
    wasmtime::component::bindgen!({
        path: "guests/compat-echo-2-0/wit/spirit.wit",
        world: "spirit",
    });
}
