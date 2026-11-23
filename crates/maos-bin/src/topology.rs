#![forbid(unsafe_code)]

//! `[topology]` manifest parsing for `maos run`.
//!
//! Lives in the library rather than in `main.rs` so its behaviour is reachable
//! from `crates/maos-bin/tests/` — an in-`src` `#[cfg(test)]` module is charged
//! to the crate's KLOC budget and is **never executed by CI**, which is exactly
//! the "budget-charged code with no execution path" class filed as evidence
//! against decision D11 by story `j1-crosshost-1a`.

/// One parsed `[[topology.spirits]]` entry.
///
/// j1-crosshost-1a AC1.1 — typed, replacing the `Vec<String>` of manifest paths,
/// so the parser can carry the **existing** `host` key. `host` already ships in
/// `spirits/topologies/bilateral-2-host-mira-nash.toml`; this parser is the first
/// reader of it. It is NOT a new spelling — do not add `host_id`.
///
/// A `host`-bearing entry is a **cross-host delegation target**: the composition
/// root emits a frame-borne `task.assign` to it rather than loading it locally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyEntry {
    pub manifest: String,
    pub host: Option<String>,
}

/// Refuse a `host` declaration that this rung cannot route.
///
/// Story 1a implements remote delegation only for `[cli_wrapper]` workers. A
/// host-bearing class Spirit must never fall through to the local scheduler:
/// that would silently erase the topology's cross-host boundary.
pub fn validate_remote_topology_target(
    entry: &TopologyEntry,
    manifest_root: &toml::Value,
) -> Result<(), String> {
    let Some(host) = entry.host.as_deref() else {
        return Ok(());
    };
    if manifest_root.get("cli_wrapper").is_none() {
        return Err(format!(
            "maos run: topology entry '{}' targets host '{host}', but remote routing is only \
             implemented for [cli_wrapper] workers; refusing to load the entry locally",
            entry.manifest
        ));
    }
    Ok(())
}

/// The ONLY keys a `[[topology.spirits]]` entry may declare.
///
/// j1-crosshost-1a AC1.2 — unknown keys are **REJECTED**, not ignored. The parser
/// never read `priority_weight`, so topology ordering has always been file order,
/// which means the signed J1 wedge demo carried documented scheduling behaviour
/// that never once happened — and the comment explaining those weights to the
/// reader was written in good faith. Strict rejection is the control that catches
/// a false statement inside a signed artifact. Either consume a key or delete it;
/// never leave it.
pub const TOPOLOGY_SPIRIT_KEYS: &[&str] = &["manifest", "path", "host"];

/// Parse `[[topology.spirits]]` into typed entries. `Ok(None)` means the manifest
/// declares no `[topology]` section at all (a single-Spirit manifest).
pub fn topology_manifest_entries(root: &toml::Value) -> Result<Option<Vec<TopologyEntry>>, String> {
    let Some(topology) = root.get("topology") else {
        return Ok(None);
    };
    let spirits = topology
        .get("spirits")
        .and_then(toml::Value::as_array)
        .ok_or("maos run: [topology] manifest must declare [[topology.spirits]] entries")?;
    let mut entries = Vec::with_capacity(spirits.len());
    for (idx, spirit) in spirits.iter().enumerate() {
        if let Some(table) = spirit.as_table() {
            for key in table.keys() {
                if !TOPOLOGY_SPIRIT_KEYS.contains(&key.as_str()) {
                    return Err(format!(
                        "maos run: [[topology.spirits]] entry {idx} declares unknown key \
                         '{key}' (known: {TOPOLOGY_SPIRIT_KEYS:?}) — an unread key is a false \
                         claim about behavior; consume it or delete it"
                    ));
                }
            }
        }
        let manifest = spirit
            .get("manifest")
            .or_else(|| spirit.get("path"))
            .and_then(toml::Value::as_str)
            .ok_or_else(|| {
                format!("maos run: [[topology.spirits]] entry {idx} must declare manifest")
            })?;
        let host = match spirit.get("host") {
            None => None,
            Some(v) => Some(
                v.as_str()
                    .ok_or_else(|| {
                        format!(
                            "maos run: [[topology.spirits]] entry {idx} `host` must be a string"
                        )
                    })?
                    .to_string(),
            ),
        };
        entries.push(TopologyEntry {
            manifest: manifest.to_string(),
            host,
        });
    }
    if entries.is_empty() {
        return Err("maos run: [topology] manifest has no spirits".into());
    }
    Ok(Some(entries))
}

/// The ONLY keys a `[[topology.a2a_peers]]` entry may declare (Story 22-8, D-22-8-07 A).
pub const TOPOLOGY_A2A_PEER_KEYS: &[&str] = &["spirit", "send_allowlist", "accept_allowlist"];

/// One parsed `[[topology.a2a_peers]]` entry: the operator's ADR-012 consent for one member of
/// the in-process cast. `send_allowlist` is the typed intents the Host lets that Spirit send;
/// `accept_allowlist` the typed intents it accepts. An absent or empty list admits nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2aPeerEntry {
    pub spirit: String,
    pub send_allowlist: Vec<String>,
    pub accept_allowlist: Vec<String>,
}

/// Parse `[[topology.a2a_peers]]`. Unknown keys are refused, as for `[[topology.spirits]]`; a
/// spirit named twice, an empty name and an empty intent string are refused. No `[topology]`
/// section, or no `a2a_peers` array, is no peers (the Host builds no router).
pub fn topology_a2a_peers(root: &toml::Value) -> Result<Vec<A2aPeerEntry>, String> {
    let Some(peers) = root
        .get("topology")
        .and_then(|topology| topology.get("a2a_peers"))
    else {
        return Ok(Vec::new());
    };
    let peers = peers
        .as_array()
        .ok_or("maos run: [[topology.a2a_peers]] must be an array of tables")?;
    let mut entries: Vec<A2aPeerEntry> = Vec::with_capacity(peers.len());
    for (idx, peer) in peers.iter().enumerate() {
        let bad = |why: String| format!("maos run: [[topology.a2a_peers]] entry {idx}: {why}");
        let table = peer
            .as_table()
            .ok_or_else(|| bad("must be a table".into()))?;
        for key in table.keys() {
            if !TOPOLOGY_A2A_PEER_KEYS.contains(&key.as_str()) {
                return Err(bad(format!(
                    "unknown key '{key}' (known: {TOPOLOGY_A2A_PEER_KEYS:?})"
                )));
            }
        }
        let spirit = table
            .get("spirit")
            .and_then(toml::Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| bad("must name a `spirit`".into()))?;
        if entries.iter().any(|entry| entry.spirit == spirit) {
            return Err(bad(format!("spirit '{spirit}' is declared twice")));
        }
        let list = |key: &str| -> Result<Vec<String>, String> {
            let Some(value) = table.get(key) else {
                return Ok(Vec::new());
            };
            value
                .as_array()
                .ok_or_else(|| bad(format!("`{key}` must be an array of strings")))?
                .iter()
                .map(|intent| {
                    intent
                        .as_str()
                        .filter(|intent| !intent.trim().is_empty())
                        .map(str::to_owned)
                        .ok_or_else(|| bad(format!("`{key}` holds a non-string or empty intent")))
                })
                .collect()
        };
        entries.push(A2aPeerEntry {
            spirit: spirit.to_owned(),
            send_allowlist: list("send_allowlist")?,
            accept_allowlist: list("accept_allowlist")?,
        });
    }
    Ok(entries)
}
