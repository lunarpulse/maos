//! Story 22-2 (AC1, AC3): the service unit ships from ONE source file, the deb and the rpm
//! carry byte-equal copies, and the unit holds the keys the story names.
use std::collections::BTreeMap;
use std::path::Path;
use xtask::render_packages::{render, service_values};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The body the rpm `%install` heredoc writes to `dest`.
fn heredoc_for(spec: &str, dest: &str) -> String {
    let start = spec
        .lines()
        .position(|l| l.starts_with("cat > ") && l.contains(dest))
        .unwrap_or_else(|| panic!("spec writes no {dest}"));
    let mut body = String::new();
    for line in spec.lines().skip(start + 1) {
        if line == "MAOS_RENDERED_EOF" {
            return body;
        }
        body.push_str(line);
        body.push('\n');
    }
    panic!("heredoc for {dest} never ends");
}

/// The spec as `render-packages` renders it: every other token filled with a stand-in.
fn rendered_spec() -> String {
    let template = read("packaging/rpm/maos.spec");
    let mut values: BTreeMap<String, String> = regex::Regex::new(r"@([A-Z][A-Za-z0-9_-]*)@")
        .unwrap()
        .captures_iter(&template)
        .map(|c| (c[1].to_owned(), "x".to_owned()))
        .collect();
    values.extend(service_values(root()).unwrap());
    render(&template, &values).unwrap()
}

#[test]
fn deb_and_rpm_copies_of_the_unit_are_byte_equal_to_the_one_source() {
    let spec = rendered_spec();
    for (source, dest) in [
        ("maos.service", "/usr/lib/systemd/system/maos.service"),
        ("maos.sysusers", "/usr/lib/sysusers.d/maos.conf"),
        ("host.toml", "/usr/share/doc/maos/examples/host.toml"),
    ] {
        let original = read(&format!("packaging/systemd/{source}"));
        // The deb copy is the source rendered through the same `render` (it holds no token).
        assert_eq!(render(&original, &BTreeMap::new()).unwrap(), original);
        assert_eq!(heredoc_for(&spec, dest), original, "rpm copy of {source}");
        assert!(
            spec.lines().any(|l| l == dest),
            "%files lists {dest} as a literal path"
        );
    }
    let renderer = std::fs::read_to_string(root().join("xtask/src/render_packages.rs")).unwrap();
    for deb_file in [
        "(\"systemd/maos.service\", \"debian/maos.service\")",
        "(\"systemd/maos.sysusers\", \"debian/maos.sysusers\")",
        "(\"systemd/host.toml\", \"debian/host.toml\")",
        "(\"deb/maos.examples\", \"debian/maos.examples\")",
    ] {
        assert!(renderer.contains(deb_file), "render list lacks {deb_file}");
    }
    assert_eq!(read("packaging/deb/maos.examples"), "debian/host.toml\n");
}

#[test]
fn unit_holds_the_ac1_keys() {
    let unit = read("packaging/systemd/maos.service");
    for line in [
        "ExecStartPre=/usr/bin/maos init --plain",
        "ExecStart=/usr/bin/maos serve --config /etc/maos/host.toml",
        "Restart=on-failure",
        "RestartPreventExitStatus=78",
        "TimeoutStopSec=30",
        "User=maos",
        "Environment=MAOS_HOME=/var/lib/maos",
        "StateDirectory=maos",
    ] {
        assert!(unit.lines().any(|l| l == line), "unit lacks `{line}`");
    }
    assert!(
        !unit.contains('%'),
        "a literal % would be expanded by rpm in the heredoc"
    );
    assert!(read("packaging/systemd/maos.sysusers")
        .lines()
        .any(|l| l == "u maos - \"MAOS Host\" /var/lib/maos"));
}

#[test]
fn packages_install_the_unit_without_enabling_it_or_owning_a_conffile() {
    let rules = read("packaging/deb/rules");
    assert!(
        rules.contains("override_dh_installsystemd:\n\tdh_installsystemd --no-enable --no-start")
    );
    let spec = read("packaging/rpm/maos.spec");
    assert!(spec.contains("%pre\ngetent group maos"));
    for forbidden in ["systemctl enable", "%systemd_post", "/etc/maos/host.toml\n"] {
        assert!(
            !spec.contains(forbidden) && !rules.contains(forbidden),
            "{forbidden}: the package neither enables the unit nor ships a Host conffile"
        );
    }
}
