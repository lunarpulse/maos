//! Story 17-3c — diagnostic, body and header bounds of the shared framing leaf.

use std::io::{self, BufRead, Cursor, Write};

use maos_frame_codec::{
    read_diagnostic, read_frame, BodyBuffer, DIAGNOSTIC_TRUNCATION_MARKER as MARKER,
    MAX_DIAGNOSTIC_BYTES, MAX_FRAME_BYTES,
};

/// The Transparency Log redactor's capability-token hex-run threshold.
const TOKEN_HEX_RUN: usize = 32;

fn diagnostic(reader: &mut impl BufRead) -> Option<Vec<u8>> {
    read_diagnostic(reader).unwrap()
}

#[test]
fn overlong_diagnostic_is_truncated_once_and_next_line_stays_intact() {
    let mut wire = vec![b'x'; MAX_DIAGNOSTIC_BYTES + 700];
    wire.push(b'\n');
    wire.extend_from_slice(b"next\n");
    // A tiny buffer forces the discard loop to cross many fill_buf calls.
    let mut reader = io::BufReader::with_capacity(7, Cursor::new(wire));
    let record = diagnostic(&mut reader).unwrap();
    assert_eq!(record.len(), MAX_DIAGNOSTIC_BYTES);
    assert!(record.ends_with(MARKER));
    assert!(record.starts_with(b"xxxx"));
    assert_eq!(diagnostic(&mut reader), Some(b"next\n".to_vec()));
    assert_eq!(diagnostic(&mut reader), None);
}

#[test]
fn bound_sized_line_is_one_record_without_a_spurious_empty_one() {
    let mut wire = vec![b'y'; MAX_DIAGNOSTIC_BYTES];
    wire.extend_from_slice(b"\nnext\n");
    let mut reader = Cursor::new(wire);
    let record = diagnostic(&mut reader).unwrap();
    assert_eq!(record.len(), MAX_DIAGNOSTIC_BYTES + 1);
    assert!(record.ends_with(b"y\n"));
    assert_eq!(diagnostic(&mut reader), Some(b"next\n".to_vec()));

    // One byte more is over the bound, with or without a terminating LF.
    for tail in [&b"\n"[..], &b""[..]] {
        let mut over = vec![b'y'; MAX_DIAGNOSTIC_BYTES + 1];
        over.extend_from_slice(tail);
        let mut reader = Cursor::new(over);
        assert!(diagnostic(&mut reader).unwrap().ends_with(MARKER));
        assert_eq!(diagnostic(&mut reader), None);
    }
}

#[test]
fn truncation_never_splits_a_utf8_sequence() {
    let limit = MAX_DIAGNOSTIC_BYTES - MARKER.len();
    // Shift the multi-byte run so the cut lands on each possible offset.
    for lead in 0..4 {
        let mut line = vec![b'z'; limit - 2 + lead];
        line.extend("\u{20AC}".repeat(40).bytes());
        line.push(b'\n');
        let record = diagnostic(&mut Cursor::new(line)).unwrap();
        let text = String::from_utf8(record).expect("cut stays on a char boundary");
        assert!(text.ends_with(std::str::from_utf8(MARKER).unwrap()));
    }
}

#[test]
fn truncation_withholds_a_partial_token_hex_tail_only() {
    let limit = MAX_DIAGNOSTIC_BYTES - MARKER.len();
    let cut_with_tail = |tail: usize| {
        let mut line = vec![b'-'; limit - tail];
        line.extend(std::iter::repeat(b'a').take(tail + 50));
        line.push(b'\n');
        let record = diagnostic(&mut Cursor::new(line)).unwrap();
        record.len() - MARKER.len()
    };
    // A hex tail too short to be redacted as a token is dropped whole ...
    assert_eq!(
        cut_with_tail(TOKEN_HEX_RUN - 1),
        limit - (TOKEN_HEX_RUN - 1)
    );
    // ... while a token-length one is kept for the redactor to replace.
    assert_eq!(cut_with_tail(TOKEN_HEX_RUN), limit);
}

#[test]
fn body_buffer_accepts_exactly_the_cap_and_refuses_one_more() {
    let mut exact = BodyBuffer::default();
    exact.write_all(&vec![7; MAX_FRAME_BYTES - 1]).unwrap();
    exact.write_all(&[7]).unwrap();
    assert_eq!(
        exact.write_all(&[7]).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(exact.into_bytes().len(), MAX_FRAME_BYTES);
    let mut over = BodyBuffer::default();
    assert_eq!(
        over.write_all(&vec![7; MAX_FRAME_BYTES + 1])
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert!(over.into_bytes().is_empty());
}

#[test]
fn blank_header_lines_are_bounded() {
    let frame = b"Content-Length: 2\r\n\r\nok";
    let with_blanks = |blanks: usize| [&b"\r\n".repeat(blanks)[..], frame].concat();
    assert_eq!(
        read_frame(&mut Cursor::new(with_blanks(16))).unwrap(),
        Some(b"ok".to_vec())
    );
    assert_eq!(
        read_frame(&mut Cursor::new(with_blanks(17)))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn lowercase_content_length_header_is_accepted_and_truncation_still_refused() {
    assert_eq!(
        read_frame(&mut Cursor::new(b"content-length: 3\r\n\r\nabc".as_slice())).unwrap(),
        Some(b"abc".to_vec())
    );
    assert_eq!(
        read_frame(&mut Cursor::new(b"content-length: 4\r\n\r\nabc".as_slice()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert_eq!(
        read_frame(&mut Cursor::new(b"Content-Length: 1\r\n\r\n".as_slice()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
}
