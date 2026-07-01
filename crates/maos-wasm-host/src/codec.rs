//! ADR-032 wire protocol codec: Content-Length + CBOR.
//!
//! Implements the framing both Spirit forms speak. The WASM runner subprocess
//! reads inbound frames from stdin and writes outbound frames to stdout using
//! this exact codec — the kernel's `read_content_length` (runtime.rs:345) on
//! the other side of the pipe speaks the same protocol.
//!
//! # Canonical CBOR (RFC 8949 §4.2.1) — caller responsibility, NOT a codec guarantee
//!
//! Decision D5: canonical CBOR enforced on BOTH sides. `ciborium` does NOT
//! canonicalize on its own — `into_writer` preserves the serializer's
//! iteration/insertion order for maps (proven by
//! `tests/wit_corpus.rs::cbor_non_pre_sorted_container_reveals_insertion_order_not_canonical`).
//! Canonical output in this crate is achieved by every caller feeding a
//! pre-sorted container (`BTreeMap`, or a struct whose fields happen to be
//! declared in the desired order) — NEVER a `HashMap` or hand-ordered
//! `Vec<(K, V)>`. Preferred-length integer encoding and definite-length
//! items ARE genuine ciborium defaults; sorted map-key order is NOT.

use std::io::{self, BufRead, Write};

/// Hard cap on a single ADR-032 frame body. Guest-emitted `Content-Length`
/// values are untrusted input at this trust boundary (the runner speaks to
/// a WASM guest over stdio) — an unbounded allocation here is a
/// guest-triggerable denial of service.
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Hard cap on the length of a single header line before the blank
/// separator. Guards against an unbounded `read_line` buffer growth on a
/// hostile or broken stream with no newline.
const MAX_HEADER_LINE_BYTES: usize = 4096;

/// Read one ADR-032 frame from a reader.
///
/// Format: `Content-Length: <decimal>\r\n\r\n` followed by N bytes of CBOR.
/// Returns `None` on clean EOF (no partial header). Skips a bounded number
/// of leading blank lines before the header (never silently drops a real
/// frame — a run of blank lines followed by a header is still read).
pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    const MAX_BLANK_SKIPS: u32 = 16;
    let mut skipped = 0u32;
    loop {
        let mut header = String::new();
        let n = read_bounded_line(reader, &mut header)?;
        if n == 0 {
            return Ok(None); // Clean EOF.
        }
        let trimmed = header.trim();
        if !trimmed.is_empty() {
            return parse_and_read(trimmed, reader);
        }
        skipped += 1;
        if skipped > MAX_BLANK_SKIPS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("too many consecutive blank lines (> {MAX_BLANK_SKIPS}) before a header"),
            ));
        }
    }
}

/// `read_line` bounded by `MAX_HEADER_LINE_BYTES` so a newline-less hostile
/// stream cannot grow the buffer without limit.
fn read_bounded_line(reader: &mut impl BufRead, out: &mut String) -> io::Result<usize> {
    let mut limited = std::io::Read::take(reader, MAX_HEADER_LINE_BYTES as u64);
    let n = limited.read_line(out)?;
    if n > 0 && !out.ends_with('\n') && (n as u64) >= MAX_HEADER_LINE_BYTES as u64 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("header line exceeds {MAX_HEADER_LINE_BYTES}-byte cap with no newline"),
        ));
    }
    Ok(n)
}

fn parse_and_read(header_line: &str, reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    // Parse "Content-Length: <n>"
    let len_str = header_line
        .strip_prefix("Content-Length:")
        .or_else(|| header_line.strip_prefix("content-length:"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("expected Content-Length header, got: {header_line}"),
            )
        })?
        .trim();

    let content_len: usize = len_str.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid Content-Length value '{len_str}': {e}"),
        )
    })?;
    if content_len > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Content-Length {content_len} exceeds the {MAX_FRAME_BYTES}-byte frame cap"),
        ));
    }

    // Consume and VALIDATE the blank line separator (\r\n or \n). A
    // non-blank line here means the sender's framing disagrees with ours —
    // reading content_len bytes starting mid-body would silently misframe
    // every subsequent frame, so this fails closed instead.
    let mut blank = String::new();
    read_bounded_line(reader, &mut blank)?;
    if !blank.trim().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected a blank separator line after Content-Length, got: {blank:?}"),
        ));
    }

    // Read exactly content_len bytes
    let mut buf = vec![0u8; content_len];
    reader.read_exact(&mut buf)?;

    Ok(Some(buf))
}

/// Write one ADR-032 frame to a writer.
///
/// Format: `Content-Length: <decimal>\r\n\r\n` followed by N bytes of CBOR.
pub fn write_frame(writer: &mut impl Write, cbor_data: &[u8]) -> io::Result<()> {
    write!(writer, "Content-Length: {}\r\n\r\n", cbor_data.len())?;
    writer.write_all(cbor_data)?;
    writer.flush()
}

/// Encode a serde-serializable value to canonical CBOR bytes.
pub fn encode_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)
        .map_err(|e| format!("CBOR encode error: {e}"))?;
    Ok(buf)
}

/// Decode canonical CBOR bytes to a serde-deserializable value.
pub fn decode_cbor<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T, String> {
    ciborium::from_reader(data)
        .map_err(|e| format!("CBOR decode error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn roundtrip_frame() {
        let payload = b"hello world";
        let mut buf = Vec::new();
        write_frame(&mut buf, payload).unwrap();

        let mut reader = BufReader::new(Cursor::new(buf));
        let frame = read_frame(&mut reader).unwrap().unwrap();
        assert_eq!(frame, payload);
    }

    #[test]
    fn clean_eof_returns_none() {
        let mut reader = BufReader::new(Cursor::new(Vec::<u8>::new()));
        assert!(read_frame(&mut reader).unwrap().is_none());
    }

    #[test]
    fn cbor_roundtrip_string() {
        let original = "test value".to_string();
        let encoded = encode_cbor(&original).unwrap();
        let decoded: String = decode_cbor(&encoded).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn cbor_roundtrip_map() {
        let mut map = std::collections::BTreeMap::new();
        map.insert("key1".to_string(), 42u64);
        map.insert("key2".to_string(), 99u64);

        let encoded = encode_cbor(&map).unwrap();
        let decoded: std::collections::BTreeMap<String, u64> = decode_cbor(&encoded).unwrap();
        assert_eq!(map, decoded);
    }
}
