//! Story 20-2: real renderer controls, without spawning cargo.
use std::collections::BTreeMap;
use xtask::render_packages::{render, versions};

#[test]
fn ac2_unrendered_hash_token_is_refused() {
    assert!(render("sha256 '@HASH_maos-linux-amd64@'", &BTreeMap::new()).is_err());
    let values = BTreeMap::from([("HASH_maos-linux-amd64".into(), "a".repeat(64))]);
    assert_eq!(
        render("@HASH_maos-linux-amd64@", &values).unwrap(),
        "a".repeat(64)
    );
}

#[test]
fn ac2_versions_preserve_prerelease_ordering_shape() {
    assert_eq!(
        versions("1.0.0-rc.1").unwrap(),
        ("1.0.0~rc.1".into(), "1.0.0rc1".into())
    );
    assert_eq!(
        versions("0.1.0-alpha.1").unwrap(),
        ("0.1.0~alpha.1".into(), "0.1.0alpha1".into())
    );
    assert_eq!(versions("1.0.0").unwrap(), ("1.0.0".into(), "1.0.0".into()));
    for bad in [
        "1.0",
        "1.0.0-",
        "1.0.0+meta",
        "1.0.0-rc/1",
        "1.0.0-rc';exit",
        "1.0.0-1",
        "1.0.0-.",
        "1.0.0-rc..1",
        "1.0.0-rc.",
    ] {
        assert!(versions(bad).is_err(), "{bad}");
    }
}

#[test]
fn ac2_committed_recipes_are_templates_not_a_development_release() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    for path in [
        "deb/rules",
        "rpm/maos.spec",
        "aur/PKGBUILD",
        "homebrew/maos.rb",
    ] {
        let recipe = std::fs::read_to_string(root.join("packaging").join(path)).unwrap();
        assert!(
            !recipe.contains("bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7"),
            "{path}: dev key"
        );
        assert!(!recipe.contains("0.5.0"), "{path}: stale version");
        assert!(
            !recipe.contains("PLACEHOLDER_") && !recipe.contains("SCAFFOLD"),
            "{path}"
        );
        assert!(
            recipe.contains("@PUBKEY@"),
            "{path}: key provenance template missing"
        );
    }
}
