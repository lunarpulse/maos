//! Story 17-3b AC5 — authored TypeScript projects receive the exact host WIT.

#[test]
fn root_template_and_example_wit_are_byte_identical() {
    let root = include_bytes!("../../../wit/spirit.wit");
    let template = include_bytes!("../../../templates/spirit-ts/wit/spirit.wit");
    let example = include_bytes!("../../../examples/example-spirit-ts/wit/spirit.wit");

    assert_eq!(
        root, template,
        "template WIT drifted from the host contract"
    );
    assert_eq!(root, example, "example WIT drifted from the host contract");
}
