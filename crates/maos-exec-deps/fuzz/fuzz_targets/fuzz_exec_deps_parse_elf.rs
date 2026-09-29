#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = maos_exec_deps::parse_elf(Cursor::new(data));
});
