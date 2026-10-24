//! Story 17-11 AC2.2 — `Scope::entitles` vector table (D-17-11-FW-04/05/07/08, ruled A).

use maos_domain::invariants::i1::Scope;

fn fr(s: &str) -> Scope {
    Scope::FsRead { subtree: s.into() }
}

fn fw(s: &str) -> Scope {
    Scope::FsWrite { subtree: s.into() }
}

#[test]
fn fs_subtree_is_lexical_component_containment() {
    for mk in [fr as fn(&str) -> Scope, fw] {
        let d = mk("/data/project");
        for ok in [
            "/data/project",
            "/data/project/",
            "/data/project/a/b",
            "/data//project/./a",
            "/./data/project",
        ] {
            assert!(d.entitles(&mk(ok)), "beneath: {ok}");
        }
        for no in [
            "/data/projectX",
            "/data/projec",
            "/data",
            "/",
            "/data/project/../secret",
            "/data/project/a/../../x",
            "data/project",
            "/other",
            "",
        ] {
            assert!(!d.entitles(&mk(no)), "outside: {no}");
        }
        // `..` in the declaration entitles nothing, even its lexical target.
        assert!(!mk("/data/../etc").entitles(&mk("/etc/passwd")));
        assert!(!mk("/data/../etc").entitles(&mk("/data/../etc")));
        // Relative declarations entitle relative requests only.
        assert!(mk("rel").entitles(&mk("rel/x")));
        assert!(!mk("rel").entitles(&mk("/rel/x")));
        // Root entitles every absolute path, no relative one.
        assert!(mk("/").entitles(&mk("/any/where")));
        assert!(!mk("/").entitles(&mk("any")));
    }
    assert!(!fr("/data").entitles(&fw("/data")));
    assert!(!fw("/data").entitles(&fr("/data")));
}

/// 17-11 review: fail-closed hardening of the lexical containment.
#[test]
fn fs_subtree_fails_closed_on_ambiguous_components() {
    for mk in [fr as fn(&str) -> Scope, fw] {
        // A relative declaration with zero components entitles nothing.
        for empty in ["", ".", "./", "./.", ".//"] {
            for req in ["x", "a/b", "", "."] {
                assert!(!mk(empty).entitles(&mk(req)), "{empty:?} ⊉ {req:?}");
            }
        }
        // `\` is not a separator here, so any component holding it refuses.
        let d = mk("/data/project");
        assert!(!d.entitles(&mk("/data/project/..\\..\\x")));
        assert!(!d.entitles(&mk("/data/project/a\\b")));
        assert!(!mk("/data\\project").entitles(&mk("/data\\project/a")));
        // NUL refuses on either side.
        assert!(!d.entitles(&mk("/data/project/a\0b")));
        assert!(!mk("/data/pro\0ject").entitles(&mk("/data/pro\0ject/a")));
        // Trailing space or dot (Win32 resolves `".. "`/`"... "` to `..`).
        for no in [
            "/data/project/.. /x",
            "/data/project/... /x",
            "/data/project/.../x",
            "/data/project/a./b",
            "/data/project/a /b",
        ] {
            assert!(!d.entitles(&mk(no)), "trailing dot/space: {no:?}");
        }
        assert!(!mk("/data/project.").entitles(&mk("/data/project./a")));
        assert!(!mk("/data/project ").entitles(&mk("/data/project /a")));
        // The absolute root keeps its meaning.
        assert!(mk("/").entitles(&mk("/a")));
    }
}

#[test]
fn identifier_scopes_are_exact_on_every_field() {
    let one = |f: fn(String) -> Scope| {
        assert!(f("a".into()).entitles(&f("a".into())));
        assert!(!f("a".into()).entitles(&f("b".into())));
        assert!(!f("a".into()).entitles(&f("a:x".into())));
        assert!(!f("a:x".into()).entitles(&f("a".into())));
        assert!(!f("a".into()).entitles(&f("A".into())));
    };
    one(|s| Scope::ProcExec { binary: s });
    one(|s| Scope::SubSpiritSpawn { class: s });
    one(|s| Scope::ProviderInfer { provider: s });
    one(|s| Scope::IacSend { peer_class: s });
    one(|s| Scope::MemRead { scope: s });
    one(|s| Scope::MemWrite { scope: s });

    let m = |s: &str, t: &str| Scope::McpCall {
        server: s.into(),
        tool: t.into(),
    };
    assert!(m("web", "search").entitles(&m("web", "search")));
    assert!(!m("web", "search").entitles(&m("web", "fetch")));
    assert!(!m("web", "search").entitles(&m("github", "search")));

    let g = |id: &str, r: &str| Scope::GatewaySend {
        gateway_id: id.into(),
        recipient: r.into(),
    };
    assert!(g("gw", "r").entitles(&g("gw", "r")));
    assert!(!g("gw", "r").entitles(&g("gx", "r")));
    assert!(!g("gw", "r").entitles(&g("gw", "s")));

    let c = |p: &str, h: u8, v: &str| Scope::CliSubprocessSpawn {
        cli_binary_path: p.into(),
        argv_prefix_hash: [h; 32],
        output_shape_version: v.into(),
    };
    assert!(c("/usr/bin/git", 1, "1").entitles(&c("/usr/bin/git", 1, "1")));
    assert!(!c("/usr/bin/git", 1, "1").entitles(&c("/usr/bin/gh", 1, "1")));
    assert!(!c("/usr/bin/git", 1, "1").entitles(&c("/usr/bin/git", 2, "1")));
    assert!(!c("/usr/bin/git", 1, "1").entitles(&c("/usr/bin/git", 1, "2")));
}

#[test]
fn net_https_is_exact_bytes() {
    let n = |d: &str| Scope::NetHttps { domain: d.into() };
    assert!(n("api.example.com").entitles(&n("api.example.com")));
    assert!(!n("api.example.com").entitles(&n("example.com")));
    assert!(!n("example.com").entitles(&n("api.example.com")));
    assert!(!n("api.example.com").entitles(&n("evil.api.example.com")));
    assert!(!n("*.example.com").entitles(&n("api.example.com")));
    assert!(!n("api.example.com").entitles(&n("API.example.com")));
}

#[test]
fn unit_variants_by_kind_and_variants_never_cross() {
    let units = [
        Scope::SelfTelemetryRead,
        Scope::LogRecall,
        Scope::LogFetch,
        Scope::DistillateWrite,
        Scope::SkillAuthorSelf,
        Scope::LoomRead,
        Scope::LoomWrite,
        Scope::LoomScan,
    ];
    let all: Vec<Scope> = units
        .iter()
        .cloned()
        .chain([
            fr("/x"),
            fw("/x"),
            Scope::NetHttps { domain: "x".into() },
            Scope::ProcExec { binary: "x".into() },
            Scope::SubSpiritSpawn { class: "x".into() },
            Scope::ProviderInfer {
                provider: "x".into(),
            },
            Scope::IacSend {
                peer_class: "x".into(),
            },
            Scope::MemRead { scope: "x".into() },
            Scope::MemWrite { scope: "x".into() },
            Scope::McpCall {
                server: "x".into(),
                tool: "x".into(),
            },
            Scope::CliSubprocessSpawn {
                cli_binary_path: "x".into(),
                argv_prefix_hash: [0; 32],
                output_shape_version: "x".into(),
            },
            Scope::GatewaySend {
                gateway_id: "x".into(),
                recipient: "x".into(),
            },
        ])
        .collect();
    for (i, d) in all.iter().enumerate() {
        for (j, r) in all.iter().enumerate() {
            assert_eq!(d.entitles(r), i == j, "{d:?} vs {r:?}");
        }
    }
}
