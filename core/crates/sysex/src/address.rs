//! 4-byte, 7-bit Roland address arithmetic. Each address byte carries 7 bits, so a
//! 4-byte address is a 28-bit number; we do the arithmetic in that linear space.
//! See docs/PROTOCOL.md §1, §5.

/// Pack a 4-byte 7-bit address into its 28-bit linear value.
pub fn to_linear(addr: [u8; 4]) -> u32 {
    addr.iter()
        .fold(0u32, |acc, &b| (acc << 7) | (u32::from(b) & 0x7F))
}

/// Unpack a 28-bit linear value into a 4-byte 7-bit address. Values beyond 28 bits
/// wrap (the top byte keeps only 7 bits).
pub fn from_linear(v: u32) -> [u8; 4] {
    [
        ((v >> 21) & 0x7F) as u8,
        ((v >> 14) & 0x7F) as u8,
        ((v >> 7) & 0x7F) as u8,
        (v & 0x7F) as u8,
    ]
}

/// Add two 4-byte addresses, with 7-bit carry.
pub fn add(a: [u8; 4], b: [u8; 4]) -> [u8; 4] {
    from_linear(to_linear(a).wrapping_add(to_linear(b)))
}

/// Add a right-aligned offset (1–4 bytes, 7-bit each) to a base address.
/// E.g. `add_offset([04,00,00,00], &[00,52,00])` -> `[04,00,52,00]`.
pub fn add_offset(base: [u8; 4], offset: &[u8]) -> [u8; 4] {
    let off = offset
        .iter()
        .fold(0u32, |acc, &b| (acc << 7) | (u32::from(b) & 0x7F));
    from_linear(to_linear(base).wrapping_add(off))
}

/// `base + stride * index` — e.g. the address of kit `index` (0-based).
pub fn with_stride(base: [u8; 4], stride: [u8; 4], index: u32) -> [u8; 4] {
    from_linear(to_linear(base).wrapping_add(to_linear(stride).wrapping_mul(index)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn g1_address_via_offsets() {
        // kit 1 base 04 00 00 00 + snare-head layer-A 00 52 00 + EQ switch 00 21.
        let a = add_offset([0x04, 0x00, 0x00, 0x00], &[0x00, 0x52, 0x00]);
        let a = add_offset(a, &[0x00, 0x21]);
        assert_eq!(a, [0x04, 0x00, 0x52, 0x21]); // matches golden vector G1
    }

    #[test]
    fn kit_200_via_stride() {
        // kit base 04 00 00 00, stride 00 04 00 00; kit 200 is index 199.
        assert_eq!(
            with_stride([0x04, 0, 0, 0], [0, 0x04, 0, 0], 199),
            [0x0A, 0x1C, 0, 0]
        );
    }

    #[test]
    fn seven_bit_carry() {
        assert_eq!(add([0, 0, 0, 0x7F], [0, 0, 0, 0x01]), [0, 0, 0x01, 0x00]);
    }

    proptest! {
        #[test]
        fn linear_roundtrip(a in 0u8..128, b in 0u8..128, c in 0u8..128, d in 0u8..128) {
            let addr = [a, b, c, d];
            prop_assert_eq!(from_linear(to_linear(addr)), addr);
        }
    }
}
