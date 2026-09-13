//! Story 15-4 AC4 — build-time release public-key parser boundaries.

use maos_audit::release_verify::parse_release_pubkey_hex;

fn panic_text(result: Result<[u8; 32], Box<dyn std::any::Any + Send>>) -> String {
    let payload = result.expect_err("invalid key must be refused");
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic".to_string()
    }
}

#[test]
fn rejects_128_hex_chars_instead_of_truncating_them() {
    let pasted_seed_and_key = concat!(
        "bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7",
        "bedd2ba634da724027983f369149f108541f43e624a846438c01452ca7f469e7"
    );
    let message = panic_text(std::panic::catch_unwind(|| {
        parse_release_pubkey_hex(pasted_seed_and_key)
    }));
    assert!(message.contains("MAOS_RELEASE_PUBKEY"), "{message}");
    assert!(message.contains("64 lowercase hex"), "{message}");
}

#[test]
fn rejects_uppercase_to_match_the_lowercase_contract() {
    let message = panic_text(std::panic::catch_unwind(|| {
        parse_release_pubkey_hex("BEDD2BA634DA724027983F369149F108541F43E624A846438C01452CA7F469E7")
    }));
    assert!(message.contains("lowercase"), "{message}");
}

#[test]
fn rejects_empty_key_with_the_variable_name() {
    let message = panic_text(std::panic::catch_unwind(|| parse_release_pubkey_hex("")));
    assert!(message.contains("MAOS_RELEASE_PUBKEY"), "{message}");
}

/// Known-answer vector for the ACCEPTING branch. Without it the three rejection
/// vectors above would all still pass over a parser that returned a constant, and
/// every release would then embed a key that does not match the provisioned
/// `MAOS_RELEASE_PUBKEY` — signature verification fails only at install time.
/// Bytes are non-uniform and independently specified, not derived from the parser.
#[test]
fn decodes_a_valid_lowercase_key_to_its_exact_bytes() {
    let parsed = parse_release_pubkey_hex(
        "00112233445566778899aabbccddeeff102132435465768798a9bacbdcedfe0f",
    );
    assert_eq!(
        parsed,
        [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff, 0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87, 0x98, 0xa9, 0xba, 0xcb,
            0xdc, 0xed, 0xfe, 0x0f,
        ]
    );
}
