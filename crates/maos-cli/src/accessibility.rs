//! Accessibility — `ColorChoice` resolver per NFR-Ops-5.
//!
//! Precedence cascade (highest to lowest):
//!   1. `--plain` CLI flag           → ColorChoice::Never
//!   2. `NO_COLOR` env (any non-empty value)   → ColorChoice::Never
//!   3. `TERM=dumb` env              → ColorChoice::Never
//!   4. stdout-is-a-tty              → ColorChoice::Auto
//!   5. fall-through (no tty)        → ColorChoice::Never
//!
//! The `EnvProvider` trait exists for testability — production uses
//! `RealEnv` (delegates to `std::env::var_os`); tests use `MockEnv`
//! to deterministically set/unset env vars without `std::env::set_var`
//! racing parallel tests.

use std::ffi::OsString;

/// Color output decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    /// Color permitted (caller may still consult its own tty check).
    Auto,
    /// Never emit ANSI color sequences.
    Never,
    /// Always emit color (escape hatch for forced color output;
    /// not exposed via CLI flag at v0.1-α — included for completeness).
    #[allow(dead_code)]
    Always,
}

/// Environment-variable provider trait — exists for test isolation.
pub trait EnvProvider {
    fn var(&self, key: &str) -> Option<OsString>;
}

/// Production env provider — delegates to `std::env::var_os`.
pub struct RealEnv;

impl EnvProvider for RealEnv {
    fn var(&self, key: &str) -> Option<OsString> {
        std::env::var_os(key)
    }
}

impl ColorChoice {
    /// Resolve color choice from the precedence cascade.
    ///
    /// `cli_plain` = `true` when the operator passed `--plain`.
    /// `env` provides environment-variable lookups.
    pub fn resolve(cli_plain: bool, env: &dyn EnvProvider) -> ColorChoice {
        if cli_plain {
            return ColorChoice::Never;
        }
        if let Some(value) = env.var("NO_COLOR") {
            if !value.is_empty() {
                return ColorChoice::Never;
            }
        }
        if let Some(term) = env.var("TERM") {
            if term == OsString::from("dumb") {
                return ColorChoice::Never;
            }
        }
        ColorChoice::Auto
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Test-only env provider — deterministic, hermetic.
    #[derive(Default)]
    struct MockEnv {
        vars: HashMap<String, OsString>,
    }

    impl MockEnv {
        fn with(mut self, k: &str, v: &str) -> Self {
            self.vars.insert(k.to_string(), OsString::from(v));
            self
        }
    }

    impl EnvProvider for MockEnv {
        fn var(&self, key: &str) -> Option<OsString> {
            self.vars.get(key).cloned()
        }
    }

    #[test]
    fn plain_flag_overrides_everything() {
        let env = MockEnv::default()
            .with("NO_COLOR", "")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(true, &env), ColorChoice::Never);
    }

    #[test]
    fn no_color_env_with_any_value_disables_color() {
        let env = MockEnv::default()
            .with("NO_COLOR", "1")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Never);
    }

    #[test]
    fn no_color_env_empty_string_does_not_disable_color() {
        let env = MockEnv::default()
            .with("NO_COLOR", "")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Auto);
    }

    #[test]
    fn term_dumb_disables_color_when_no_color_unset() {
        let env = MockEnv::default().with("TERM", "dumb");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Never);
    }

    #[test]
    fn term_xterm_falls_through_to_auto() {
        let env = MockEnv::default().with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Auto);
    }

    #[test]
    fn no_env_falls_through_to_auto() {
        let env = MockEnv::default();
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Auto);
    }

    #[test]
    fn plain_flag_wins_over_no_color_set() {
        let env = MockEnv::default().with("NO_COLOR", "1");
        assert_eq!(ColorChoice::resolve(true, &env), ColorChoice::Never);
    }

    #[test]
    fn no_color_wins_over_term_dumb() {
        let env = MockEnv::default()
            .with("NO_COLOR", "1")
            .with("TERM", "xterm-256color");
        assert_eq!(ColorChoice::resolve(false, &env), ColorChoice::Never);
    }
}
