#![forbid(unsafe_code)]

//! The cassette the child gets follows the CHILD's effective inference mode
//! (live-leg run 43239794859): `record` → the source file (re-record target),
//! anything else → the world's own live copy.

use maos_journey_test::ReplayProvider;

#[test]
fn child_mode_selects_source_or_live_copy() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("seed.json");
    std::fs::write(&source, b"seed").unwrap();
    let provider = ReplayProvider::cassette(source.to_str().unwrap());

    assert_eq!(
        provider.cassette_path(Some("record")),
        Some(source.as_path())
    );
    for mode in [Some("replay"), Some(""), None] {
        let copy = provider
            .cassette_path(mode)
            .expect("a replay child gets a cassette");
        assert_ne!(copy, source.as_path(), "{mode:?} must not read the source");
        assert_eq!(std::fs::read(copy).unwrap(), b"seed", "{mode:?}: live copy");
    }
}

#[test]
fn a_cassette_free_provider_has_no_path_in_any_mode() {
    let provider = ReplayProvider::default();
    assert_eq!(provider.cassette_path(Some("record")), None);
    assert_eq!(provider.cassette_path(None), None);
}
