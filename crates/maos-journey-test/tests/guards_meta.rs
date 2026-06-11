#![forbid(unsafe_code)]

//! JB-7 / H-guard meta-tests: assert journey test sources contain no
//! wall-clock reads or fixed sleeps (H4 guard).

use maos_journey_test::guards;

fn test_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

// journey_butler.rs is exempt: JB-4 (driver integration) uses SystemTime::now()
// because TL rows carry real wall-clock timestamps and morning_digest's 24h
// window must contain them. The guard applies to the PTY journey tests.

#[test]
fn jb7_journey_researcher_no_wallclock() {
    guards::assert_no_wallclock_or_fixed_sleep(
        test_root().join("journey_researcher.rs").to_str().unwrap(),
    );
}

#[test]
fn jb7_journey_j1_no_wallclock() {
    guards::assert_no_wallclock_or_fixed_sleep(
        test_root().join("journey_j1.rs").to_str().unwrap(),
    );
}

#[test]
fn jb7_journey_j4_no_wallclock() {
    guards::assert_no_wallclock_or_fixed_sleep(
        test_root().join("journey_j4.rs").to_str().unwrap(),
    );
}

#[test]
fn jb7_journey_j0_no_wallclock() {
    guards::assert_no_wallclock_or_fixed_sleep(
        test_root().join("journey_j0.rs").to_str().unwrap(),
    );
}
