// Fixture for Story 2.2 AC4 — pass scenario. NOT compiled by workspace.

pub mod capability {
    pub fn mediate(_path: &str) {}
}

pub mod api {
    pub fn do_something() {
        crate::capability::mediate("config.toml");
    }
}
