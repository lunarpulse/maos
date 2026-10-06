//! Bounded ADR-032 framing. Bodies are opaque bytes, including empty control records.
//! Subprocess stderr uses bounded LF-delimited chunks, never the frame protocol.
#![forbid(unsafe_code)]

use std::io::{self, BufRead, Read, Write};

pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_HEADER_LINE_BYTES: usize = 4096;
pub const MAX_DIAGNOSTIC_BYTES: usize = 4096;

/// Read a complete frame, or clean EOF at a header boundary.
/// Header and body limits are checked before body allocation.
pub fn read_frame<R: BufRead + ?Sized>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    for skipped in 0..=16 {
        let mut header = String::new();
        if read_header_line(reader, &mut header)? == 0 {
            return Ok(None);
        }
        let header = header.trim();
        if header.is_empty() {
            if skipped == 16 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "too many blank header lines",
                ));
            }
            continue;
        }
        let length = header
            .strip_prefix("Content-Length:")
            .or_else(|| header.strip_prefix("content-length:"))
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "expected Content-Length header")
            })?
            .trim();
        if length.is_empty() || !length.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Content-Length",
            ));
        }
        let length: usize = length
            .parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Content-Length overflow"))?;
        if length > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame exceeds ADR-032 body limit",
            ));
        }
        let mut separator = String::new();
        if read_header_line(reader, &mut separator)? == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "missing frame separator",
            ));
        }
        if separator != "\n" && separator != "\r\n" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected blank frame separator",
            ));
        }
        let mut body = Vec::new();
        body.try_reserve_exact(length)
            .map_err(|_| io::Error::new(io::ErrorKind::OutOfMemory, "frame allocation refused"))?;
        io::Read::take(reader, length as u64).read_to_end(&mut body)?;
        if body.len() != length {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated frame body",
            ));
        }
        return Ok(Some(body));
    }
    unreachable!("bounded blank-header loop either reads a frame or returns an error")
}

fn read_header_line<R: BufRead + ?Sized>(reader: &mut R, out: &mut String) -> io::Result<usize> {
    let count = io::Read::take(reader, MAX_HEADER_LINE_BYTES as u64).read_line(out)?;
    if count != 0 && !out.ends_with('\n') {
        let kind = if count == MAX_HEADER_LINE_BYTES {
            io::ErrorKind::InvalidData
        } else {
            io::ErrorKind::UnexpectedEof
        };
        return Err(io::Error::new(
            kind,
            "unterminated or oversized frame header",
        ));
    }
    Ok(count)
}

/// Write one complete bounded record without an allocated header string.
pub fn write_frame<W: Write + ?Sized>(writer: &mut W, body: &[u8]) -> io::Result<()> {
    if body.len() > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "outbound frame exceeds ADR-032 body limit",
        ));
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(body)?;
    writer.flush()
}

/// Appended to a diagnostic line cut at the bound; its length is reserved in the record.
pub const DIAGNOSTIC_TRUNCATION_MARKER: &[u8] = b" [maos: diagnostic truncated]";
/// Length at which a lowercase-hex run is a Transparency Log capability-token secret.
const SECRET_HEX_RUN: usize = 32;

/// Read one diagnostic line (content up to 4096 bytes, plus its LF) or `None` at EOF.
///
/// Secrets run to the next space/quote/LF and may be arbitrarily long, so a longer
/// line is never split into continuation records: a continuation would start
/// mid-token and could reach the journal unredacted. Instead the line is
/// **truncated** at `MAX_DIAGNOSTIC_BYTES` (marker included), the cut backed off to
/// a UTF-8 boundary and past any trailing lowercase-hex run shorter than a token,
/// and the rest of the line is discarded without buffering. A secret prefix kept
/// in the record is redacted to the end of the record by the log's redactor.
pub fn read_diagnostic<R: BufRead + ?Sized>(reader: &mut R) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    let count = io::Read::take(&mut *reader, MAX_DIAGNOSTIC_BYTES as u64 + 1)
        .read_until(b'\n', &mut bytes)?;
    if count == 0 {
        return Ok(None);
    }
    if count <= MAX_DIAGNOSTIC_BYTES || bytes.last() == Some(&b'\n') {
        return Ok(Some(bytes));
    }
    skip_line(reader)?;
    let mut cut = MAX_DIAGNOSTIC_BYTES - DIAGNOSTIC_TRUNCATION_MARKER.len();
    let partial = bytes[..=cut]
        .iter()
        .rev()
        .take_while(|byte| **byte & 0xC0 == 0x80)
        .take(4)
        .count();
    if partial <= 3 {
        cut -= partial;
    }
    let hex = bytes[..cut]
        .iter()
        .rev()
        .take_while(|byte| matches!(**byte, b'0'..=b'9' | b'a'..=b'f'))
        .count();
    if hex < SECRET_HEX_RUN {
        cut -= hex;
    }
    bytes.truncate(cut);
    bytes.extend_from_slice(DIAGNOSTIC_TRUNCATION_MARKER);
    Ok(Some(bytes))
}

/// Discard through the next LF (or EOF) without buffering the line.
fn skip_line<R: BufRead + ?Sized>(reader: &mut R) -> io::Result<()> {
    loop {
        let buffered = match reader.fill_buf() {
            Ok(buffered) => buffered,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if buffered.is_empty() {
            return Ok(());
        }
        match buffered.iter().position(|byte| *byte == b'\n') {
            Some(index) => {
                reader.consume(index + 1);
                return Ok(());
            }
            None => {
                let all = buffered.len();
                reader.consume(all);
            }
        }
    }
}

/// Serializer sink bounding both body length and geometric capacity growth.
/// Format-specific encoding remains outside this std-only transport crate.
#[derive(Default)]
pub struct BodyBuffer(Vec<u8>);

impl BodyBuffer {
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl Write for BodyBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_FRAME_BYTES - self.0.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "serialized body exceeds frame cap",
            ));
        }
        let required = self.0.len() + bytes.len();
        if required > self.0.capacity() {
            let capacity = self
                .0
                .capacity()
                .max(256)
                .saturating_mul(2)
                .max(required)
                .min(MAX_FRAME_BYTES);
            self.0
                .try_reserve_exact(capacity - self.0.len())
                .map_err(|error| io::Error::new(io::ErrorKind::OutOfMemory, error))?;
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn binary_and_empty_records_remain_separate_across_partial_reads() {
        let payload = [0xFF, 0, b'\r', b'\n', 0x80];
        let mut wire = Vec::new();
        write_frame(&mut wire, &[]).unwrap();
        write_frame(&mut wire, &payload).unwrap();
        write_frame(&mut wire, b"last").unwrap();
        let mut reader = io::BufReader::with_capacity(1, Cursor::new(wire));
        assert_eq!(read_frame(&mut reader).unwrap(), Some(Vec::new()));
        assert_eq!(read_frame(&mut reader).unwrap(), Some(payload.to_vec()));
        assert_eq!(read_frame(&mut reader).unwrap(), Some(b"last".to_vec()));
        assert_eq!(read_frame(&mut reader).unwrap(), None);
    }

    #[test]
    fn incomplete_headers_separators_and_bodies_never_become_records() {
        for truncated in [
            b"Content-Length: 0".as_slice(),
            b"Content-Length: 0\r\n".as_slice(),
            b"Content-Length: 0\r\n\r".as_slice(),
            b"Content-Length: 3\r\n\r\nab".as_slice(),
        ] {
            assert_eq!(
                read_frame(&mut Cursor::new(truncated)).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof
            );
        }
        for malformed in [
            b"Content-Length: +1\n\nx".as_slice(),
            b"Content-Length: 0\nnot-blank\n".as_slice(),
            b"Content-Length: 16777217\n\n".as_slice(),
        ] {
            assert_eq!(
                read_frame(&mut Cursor::new(malformed)).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn limits_refuse_before_body_or_header_can_escape() {
        let body = vec![0x80; MAX_FRAME_BYTES];
        let mut wire = Vec::new();
        write_frame(&mut wire, &body).unwrap();
        assert_eq!(read_frame(&mut Cursor::new(wire)).unwrap(), Some(body));
        let oversized = vec![0; MAX_FRAME_BYTES + 1];
        let mut untouched = Vec::new();
        assert_eq!(
            write_frame(&mut untouched, &oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert!(untouched.is_empty());
        assert_eq!(
            read_frame(&mut Cursor::new(vec![b'x'; MAX_HEADER_LINE_BYTES + 1]))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
