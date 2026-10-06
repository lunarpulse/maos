//! Bounded CBOR encoding and decoding for the ADR-032 frame body.
//! Byte framing is owned by the std-only `maos-frame-codec` leaf.
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

use maos_frame_codec::MAX_FRAME_BYTES;

/// Encode a serde-serializable value to CBOR, without allocating beyond the
/// ADR-032 frame limit. This is also used on guest-controlled output. Output
/// that [`decode_cbor`] would refuse (expansion budget, nesting) is refused here.
pub fn encode_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut buf = maos_frame_codec::BodyBuffer::default();
    ciborium::into_writer(value, &mut buf).map_err(|e| format!("CBOR encode error: {e}"))?;
    let bytes = buf.into_bytes();
    validate_decode_budget(&bytes).map_err(|error| format!("CBOR encode bound: {error}"))?;
    Ok(bytes)
}

/// Decode a definite-length CBOR body after allocation-free expansion validation.
/// Each encoded node consumes 1 KiB of the 16 MiB expansion budget; string/byte
/// payloads consume their length as well. Container hints are checked before
/// serde can reserve memory, and nesting is limited to 64 levels.
pub fn decode_cbor<T: serde::de::DeserializeOwned>(data: &[u8]) -> Result<T, String> {
    validate_decode_budget(data).map_err(|error| format!("CBOR decode bound: {error}"))?;
    ciborium::from_reader(data).map_err(|e| format!("CBOR decode error: {e}"))
}

fn validate_decode_budget(data: &[u8]) -> Result<(), &'static str> {
    fn item(
        data: &[u8],
        pos: &mut usize,
        budget: &mut usize,
        depth: u8,
    ) -> Result<(), &'static str> {
        if depth > 64 {
            return Err("nesting limit exceeded");
        }
        *budget = budget
            .checked_sub(1024)
            .ok_or("expanded item budget exceeded")?;
        let head = *data.get(*pos).ok_or("truncated item")?;
        *pos += 1;
        let additional = head & 31;
        let value = match additional {
            0..=23 => u64::from(additional),
            24..=27 => {
                let width = 1usize << (additional - 24);
                let end = pos.checked_add(width).ok_or("length overflow")?;
                let bytes = data.get(*pos..end).ok_or("truncated argument")?;
                *pos = end;
                bytes
                    .iter()
                    .fold(0u64, |value, byte| (value << 8) | u64::from(*byte))
            }
            _ => return Err("reserved or indefinite-length item"),
        };
        match head >> 5 {
            0 | 1 | 7 => {}
            2 | 3 => {
                let length = usize::try_from(value).map_err(|_| "length overflow")?;
                *budget = budget
                    .checked_sub(length)
                    .ok_or("expanded string budget exceeded")?;
                *pos = pos.checked_add(length).ok_or("length overflow")?;
                if *pos > data.len() {
                    return Err("truncated string");
                }
            }
            4 | 5 => {
                let count = if head >> 5 == 5 {
                    value.checked_mul(2).ok_or("map length overflow")?
                } else {
                    value
                };
                if count > (*budget / 1024) as u64 {
                    return Err("container expansion budget exceeded");
                }
                for _ in 0..count {
                    item(data, pos, budget, depth + 1)?;
                }
            }
            6 => item(data, pos, budget, depth + 1)?,
            _ => unreachable!(),
        }
        Ok(())
    }
    if data.len() > MAX_FRAME_BYTES {
        return Err("frame byte limit exceeded");
    }
    let mut pos = 0;
    let mut budget = MAX_FRAME_BYTES;
    item(data, &mut pos, &mut budget, 0)?;
    if pos != data.len() {
        return Err("trailing CBOR item");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
