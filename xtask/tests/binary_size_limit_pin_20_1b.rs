//! The stripped-binary size limit has two enforcers that must agree: the NFR-Onb-2 AC4
//! shell gate and the `prepush` release-size leg. Operator ruling `size` (2026-11-12)
//! sets both to 34 MiB = 35,651,584 B.

use std::path::PathBuf;

const LIMIT: u64 = 34 * 1024 * 1024;

fn read(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    std::fs::read_to_string(root.join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

#[test]
fn the_size_limit_is_34_mib_in_both_enforcers() {
    assert_eq!(LIMIT, 35_651_584);
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
        "release maos <= 35651584 B",
        "actual > 35_651_584",
        "expected <= 35651584 B",
    ] {
        assert!(prepush.contains(needle), "prepush leg lacks `{needle}`");
    }
    assert!(
        !prepush.contains("33554432") && !prepush.contains("33_554_432"),
        "the old 32 MiB literal survives in prepush"
    );
}
