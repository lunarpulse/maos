//! NFR-Onb-2 AC4 and prepush use the same stripped default-on release limit.
//! 21-9 D-21-9-04 A: measure 37,208,592 bytes, add 5% and round up.

use std::path::PathBuf;

const LIMIT: u64 = 39_069_022;

fn read(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::read_to_string(root.join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

#[test]
fn the_measured_default_on_limit_matches_both_enforcers() {
    let shell = read("tests/integration/onb_nfr2_timing.sh");
    let line = shell
        .lines()
        .find(|l| l.starts_with("max_size="))
        .expect("the shell gate declares max_size");
    let value: u64 = line["max_size=".len()..]
        .split_whitespace()
        .next()
        .and_then(|v| v.parse().ok())
        .expect("max_size parses");
    assert_eq!(value, LIMIT, "{line}");

    let prepush = read("xtask/src/prepush.rs");
    for needle in [
        "release maos <= 39069022 B",
        "actual > 39_069_022",
        "expected <= 39069022 B",
    ] {
        assert!(prepush.contains(needle), "prepush leg lacks `{needle}`");
    }
}
