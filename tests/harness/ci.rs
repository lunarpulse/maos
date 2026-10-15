//! Shared test-harness capability declaration (the Story 17-6 T2 convention).

pub fn declared_ci() -> bool {
    std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some()
}
