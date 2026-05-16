// Fixture for Story 2.2 AC4 — fail scenario. NOT compiled by workspace.

pub mod api {
    pub fn do_something() {
        let _ = std::fs::read("config.toml");
    }
}
