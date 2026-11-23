//! Story 22-1 (AC6; D-22-1-10 A) — ONE precedence for the Host's data stores.
//!
//! The private memory root, the registry store, the yank cursor and the
//! migration-plan directory used to resolve independently (memory: XDG then
//! `$HOME/.local/share`; registry and cursor: `$HOME/.local/share` only; plans:
//! `MAOS_HOME` only). They now share one ranking of data roots:
//!
//! 1. an **explicitly set** `MAOS_HOME` (the `$HOME/.maos` fallback of
//!    `operator_door::maos_home` is NOT a data root here),
//! 2. `$XDG_DATA_HOME/maos`,
//! 3. the legacy `$HOME/.local/share/maos`,
//!    with `/var/lib/maos` as the last resort when neither XDG nor HOME is set.
//!
//! A store lives under the highest root that already holds it; when none does, it
//! is created under the highest root. Data under a lower (legacy) root therefore
//! stays readable in place, and nothing is moved.

use std::ffi::OsStr;
use std::path::PathBuf;

/// The ranked data roots from explicit inputs (an empty value counts as unset).
pub fn roots_from(
    maos_home: Option<&OsStr>,
    xdg_data_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Vec<PathBuf> {
    let set = |value: Option<&OsStr>| value.filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut roots = Vec::with_capacity(3);
    roots.extend(set(maos_home));
    let xdg = set(xdg_data_home).map(|dir| dir.join("maos"));
    let legacy = set(home).map(|dir| dir.join(".local/share/maos"));
    if xdg.is_none() && legacy.is_none() {
        roots.push(PathBuf::from("/var/lib/maos"));
    }
    roots.extend(xdg);
    roots.extend(legacy);
    roots
}

/// The ranked data roots of this process's environment.
pub fn roots() -> Vec<PathBuf> {
    roots_from(
        std::env::var_os("MAOS_HOME").as_deref(),
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

/// Every location `rel` can resolve to, highest rank first.
pub fn locations(rel: &str) -> Vec<PathBuf> {
    roots().into_iter().map(|root| root.join(rel)).collect()
}

/// The store `rel` (for example `memory`) within the given ranked roots: the
/// highest location that exists, else the highest location.
pub fn resolve_in(roots: &[PathBuf], rel: &str) -> PathBuf {
    let mut first = None;
    for root in roots {
        let path = root.join(rel);
        if path.exists() {
            return path;
        }
        first.get_or_insert(path);
    }
    first.unwrap_or_else(|| PathBuf::from("/var/lib/maos").join(rel))
}

/// Whether the environment names any data root (an explicit `MAOS_HOME`, `XDG_DATA_HOME` or
/// `HOME`). Without one only the shared `/var/lib/maos` last resort is left, which a per-user
/// cursor must not select.
pub fn anchored() -> bool {
    ["MAOS_HOME", "XDG_DATA_HOME", "HOME"]
        .iter()
        .any(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
}

/// The store `rel` for this process's environment.
pub fn resolve(rel: &str) -> PathBuf {
    resolve_in(&roots(), rel)
}
