//! Value encodings for Roland parameters: how a field's bytes map to a raw value.
//! The field's byte length comes from the device profile; the encoding says how to
//! interpret those bytes.
//!
//! Scope: pure bytes <-> raw value. Scaling (e.g. tempo ÷10), units and i18n live
//! in the model layer, not here. See docs/PROTOCOL.md §4.

/// How a parameter field's bytes encode its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Big-endian base-128: each byte carries 7 bits. `value = Σ b[i]·128^(n-1-i)`.
    Plain7,
    /// Nibble-packed big-endian base-16: each byte carries its low 4 bits.
    Nibble,
    /// Signed, Roland convention centred at 0: a plain7 value minus `64·128^(len-1)`
    /// (1-byte: 00=-64, 40=0, 7F=+63; 2-byte: 00 00=-8192, 40 00=0, 7F 7F=+8191).
    Signed,
    /// Nibble-packed two's complement over `4·len` bits — the V31 stores signed
    /// multi-nibble fields this way (e.g. volume raw −601 in 4 nibbles =
    /// 0xFDA7 → `0F 0D 0A 07`). Inferred from the negative range spans in the
    /// MIDI Implementation; every live edit is confirmed by read-back anyway
    /// (no blind writes).
    SignedNibble,
    /// ASCII text, one char per byte — use [`decode_ascii`] / [`encode_ascii`].
    Ascii,
}

impl Encoding {
    /// Decode a numeric field. Returns `None` for [`Encoding::Ascii`].
    pub fn decode_int(self, bytes: &[u8]) -> Option<i64> {
        match self {
            Encoding::Plain7 => Some(decode_base(bytes, 128) as i64),
            Encoding::Nibble => Some(decode_base(bytes, 16) as i64),
            Encoding::Signed => Some(decode_base(bytes, 128) as i64 - signed_centre(bytes.len())),
            Encoding::SignedNibble => {
                let raw = decode_base(bytes, 16) as i64;
                let span = 1i64 << (4 * bytes.len());
                Some(if raw >= span / 2 { raw - span } else { raw })
            }
            Encoding::Ascii => None,
        }
    }

    /// Encode a numeric value into exactly `len` bytes. Returns `None` for
    /// [`Encoding::Ascii`] or if the value doesn't fit in `len` bytes.
    pub fn encode_int(self, value: i64, len: usize) -> Option<Vec<u8>> {
        match self {
            Encoding::Plain7 => encode_base(u64::try_from(value).ok()?, 128, len),
            Encoding::Nibble => encode_base(u64::try_from(value).ok()?, 16, len),
            Encoding::Signed => {
                let raw = value.checked_add(signed_centre(len))?;
                encode_base(u64::try_from(raw).ok()?, 128, len)
            }
            Encoding::SignedNibble => {
                let span = 1i64 << (4 * len.min(15));
                if value < -span / 2 || value >= span / 2 {
                    return None;
                }
                let raw = if value < 0 { value + span } else { value };
                encode_base(raw as u64, 16, len)
            }
            Encoding::Ascii => None,
        }
    }
}

/// `64 · 128^(len-1)` — the zero point for a signed field of `len` bytes.
fn signed_centre(len: usize) -> i64 {
    64 * 128_i64.pow(len.saturating_sub(1) as u32)
}

fn decode_base(bytes: &[u8], base: u32) -> u64 {
    let base = u64::from(base);
    bytes
        .iter()
        .fold(0u64, |acc, &b| acc * base + (u64::from(b) % base))
}

fn encode_base(mut value: u64, base: u32, len: usize) -> Option<Vec<u8>> {
    let base = u64::from(base);
    let mut out = vec![0u8; len];
    for slot in out.iter_mut().rev() {
        *slot = (value % base) as u8;
        value /= base;
    }
    // Anything left over means the value didn't fit in `len` bytes.
    (value == 0).then_some(out)
}

/// Decode ASCII bytes to a `String`, dropping trailing spaces/NULs (Roland pads names).
pub fn decode_ascii(bytes: &[u8]) -> String {
    let s: String = bytes.iter().map(|&b| (b & 0x7F) as char).collect();
    s.trim_end_matches([' ', '\0']).to_string()
}

/// Encode a string to exactly `len` ASCII bytes, space-padded or truncated.
pub fn encode_ascii(s: &str, len: usize) -> Vec<u8> {
    let mut out = vec![b' '; len];
    for (slot, ch) in out.iter_mut().zip(s.chars()) {
        *slot = if ch.is_ascii() {
            (ch as u8) & 0x7F
        } else {
            b'?'
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn plain7_doc_example() {
        // docs/PROTOCOL.md §4: 12 34h (7-bit each) = 18*128 + 52 = 2356.
        assert_eq!(Encoding::Plain7.decode_int(&[0x12, 0x34]), Some(2356));
        assert_eq!(Encoding::Plain7.encode_int(2356, 2), Some(vec![0x12, 0x34]));
    }

    #[test]
    fn nibble_four_bytes() {
        // 1200 = 0x4B0 -> nibbles 0,4,B,0.
        assert_eq!(
            Encoding::Nibble.decode_int(&[0x00, 0x04, 0x0B, 0x00]),
            Some(1200)
        );
        assert_eq!(
            Encoding::Nibble.encode_int(1200, 4),
            Some(vec![0x00, 0x04, 0x0B, 0x00])
        );
    }

    #[test]
    fn signed_one_byte() {
        assert_eq!(Encoding::Signed.decode_int(&[0x00]), Some(-64));
        assert_eq!(Encoding::Signed.decode_int(&[0x40]), Some(0));
        assert_eq!(Encoding::Signed.decode_int(&[0x7F]), Some(63));
        assert_eq!(Encoding::Signed.encode_int(0, 1), Some(vec![0x40]));
        assert_eq!(Encoding::Signed.encode_int(-64, 1), Some(vec![0x00]));
    }

    #[test]
    fn signed_two_byte() {
        assert_eq!(Encoding::Signed.decode_int(&[0x00, 0x00]), Some(-8192));
        assert_eq!(Encoding::Signed.decode_int(&[0x40, 0x00]), Some(0));
        assert_eq!(Encoding::Signed.decode_int(&[0x7F, 0x7F]), Some(8191));
    }

    #[test]
    fn signed_nibble_twos_complement() {
        // Volume raw -601 = 0xFDA7 over 16 bits -> nibbles 0F 0D 0A 07.
        assert_eq!(
            Encoding::SignedNibble.decode_int(&[0x0F, 0x0D, 0x0A, 0x07]),
            Some(-601)
        );
        assert_eq!(
            Encoding::SignedNibble.encode_int(-601, 4),
            Some(vec![0x0F, 0x0D, 0x0A, 0x07])
        );
        // Positive values look like plain nibbles: +60 = 0x003C.
        assert_eq!(
            Encoding::SignedNibble.decode_int(&[0x00, 0x00, 0x03, 0x0C]),
            Some(60)
        );
        // 2-nibble: -5 = 0xFB -> 0F 0B.
        assert_eq!(Encoding::SignedNibble.decode_int(&[0x0F, 0x0B]), Some(-5));
        assert_eq!(
            Encoding::SignedNibble.encode_int(-5, 2),
            Some(vec![0x0F, 0x0B])
        );
        // Out of range for the width.
        assert_eq!(Encoding::SignedNibble.encode_int(128, 2), None);
        assert_eq!(Encoding::SignedNibble.encode_int(-129, 2), None);
    }

    #[test]
    fn ascii_roundtrip() {
        assert_eq!(decode_ascii(b"TR-808  "), "TR-808");
        assert_eq!(encode_ascii("TR-808", 8), b"TR-808  ".to_vec());
        assert_eq!(Encoding::Ascii.decode_int(b"x"), None);
        assert_eq!(Encoding::Ascii.encode_int(1, 1), None);
    }

    #[test]
    fn overflow_returns_none() {
        assert_eq!(Encoding::Plain7.encode_int(128, 1), None); // 128 needs 8 bits
        assert_eq!(Encoding::Nibble.encode_int(16, 1), None); // 16 needs > 1 nibble
        assert_eq!(Encoding::Plain7.encode_int(-1, 1), None); // negative not unsigned
    }

    proptest! {
        #[test]
        fn plain7_roundtrip(v in 0u32..(1 << 21)) {
            let bytes = Encoding::Plain7.encode_int(i64::from(v), 3).unwrap();
            prop_assert_eq!(Encoding::Plain7.decode_int(&bytes), Some(i64::from(v)));
        }

        #[test]
        fn nibble_roundtrip(v in 0u32..(1 << 16)) {
            let bytes = Encoding::Nibble.encode_int(i64::from(v), 4).unwrap();
            prop_assert_eq!(Encoding::Nibble.decode_int(&bytes), Some(i64::from(v)));
        }

        #[test]
        fn signed2_roundtrip(v in -8192i64..=8191) {
            let bytes = Encoding::Signed.encode_int(v, 2).unwrap();
            prop_assert_eq!(Encoding::Signed.decode_int(&bytes), Some(v));
        }

        #[test]
        fn signed_nibble4_roundtrip(v in -32768i64..=32767) {
            let bytes = Encoding::SignedNibble.encode_int(v, 4).unwrap();
            prop_assert_eq!(Encoding::SignedNibble.decode_int(&bytes), Some(v));
        }
    }
}
