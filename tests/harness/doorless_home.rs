#![allow(dead_code)]

//! Story 16-1 / D-16-1-Q — an EMPTY scratch `HOME` for every test that spawns
//! a `maos` root.
//!
//! # The failure this prevents
//!
//! Before Story 16-1, `$HOME/.maos` held nothing a running root cared about,
//! so a test could spawn `maos run` on the developer's real `HOME` and nothing
//! collided. After this story, `<home>/control.json` names ONE loopback
//! endpoint and a second root on that endpoint fails fast with
//! `EndpointInUse` — by design, because two daemons quietly serving one
//! operator surface is the bug the door exists to remove.
//!
//! A root-spawning test that inherits the developer's `HOME` is therefore
//! green in a pristine CI image and red on any machine where `maos init` has
//! ever run. That is the worst shape of flake, because CI green is what
//! everyone believes. `.github/workflows/discipline.yml` makes CI hostile
//! instead (it seeds a real home and holds its endpoint), and
//! `crates/maos-bin/tests/root_spawn_home_isolation_16_1.rs` is the local
//! floor; this module is the fix both of them ask for.
//!
//! # Why the home is left EMPTY
//!
//! These vectors do not exercise the operator door — they boot roots to assert
//! on the Transparency Log, the Lifecycle Journal, cohort A2A or provider
//! polarity. No `control.json` in the home means NO door is configured, the
//! root announces that once on stderr and proceeds. That is both the cheapest
//! isolation and the most honest one: nothing here pretends to test a door.
//!
//! # Why `HOME` and not `MAOS_HOME`
//!
//! `MAOS_HOME` also redirects the Transparency Log, the Lifecycle Journal and
//! the memory root. Several of these files route those stores explicitly
//! through `MAOS_AUDIT_DB` / `MAOS_JOURNAL_PATH` and assert on them, and
//! `f4_pairing` / `two_host_delegation_2b` run two roots on one log ON PURPOSE.
//! Setting `MAOS_HOME` would silently move the very artifacts under assertion.
//! `HOME` moves only the door's discovery file, which is exactly the scope of
//! the problem.
//!
//! # Why it is a `#[path]`-included module and not a crate
//!
//! It is consumed by integration tests in three different packages
//! (`maos-bin`, `maos-cli`, `maos-journey-test`). A new crate would move
//! `check-workspace-count` off 55 for ten lines of test support, and a
//! `dev-dependency` on a support crate would show up in
//! `cargo tree -p maos-cli`, which `dep_kernel_core_free_test.rs` asserts on.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// A scratch directory, unique to this test binary and this process, holding
/// no `control.json`.
///
/// Per test BINARY rather than per test FUNCTION on purpose: the isolation
/// that matters is from the developer's real home, and an empty home
/// configures no door at all, so concurrent roots inside one binary cannot
/// collide on an endpoint either. A per-function directory would buy nothing
/// and would leave one temp directory per test behind.
pub fn doorless_home() -> &'static Path {
    static HOME: LazyLock<PathBuf> = LazyLock::new(|| {
        let path = std::env::temp_dir().join(format!(
            "maos-doorless-home-{}-{}",
            env!("CARGO_PKG_NAME"),
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("scratch HOME for a root-spawning test");
        // ⚠ Deliberately NOT `maos init`: an initialised home would mint a
        // `control.json`, every root in this binary would bind its single
        // endpoint, and the second one would fail `EndpointInUse` — exactly
        // the collision this module exists to remove.
        path
    });
    HOME.as_path()
}

/// A scratch XDG data root under the same home, for files that export
/// `XDG_DATA_HOME` (the memory root, journal and erasure proofs follow it, so
/// leaving it pointed at the developer's real one re-opens the split-brain
/// through a different door — §17 V-32).
pub fn doorless_xdg_data_home() -> PathBuf {
    let path = doorless_home().join("xdg");
    std::fs::create_dir_all(&path).expect("scratch XDG_DATA_HOME");
    path
}
