//! Device-agnostic Roland SysEx mechanics: RQ1/DT1 framing, checksum, 4-byte
//! 7-bit address arithmetic, and value encodings. No I/O, no module specifics.
//!
//! See docs/PROTOCOL.md (derived facts + golden vectors) and
//! docs/DEVELOPMENT.md §4.1.
#![forbid(unsafe_code)]

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Roland checksum over the address+data byte slice (everything between the
/// command byte and the trailing checksum, exclusive of `F7`).
///
/// `checksum = (128 - (sum(bytes) mod 128)) mod 128`.
pub fn roland_checksum(addr_and_data: &[u8]) -> u8 {
    let sum: u32 = addr_and_data.iter().map(|&b| b as u32).sum();
    ((128 - (sum % 128)) % 128) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden vector G1 (docs/PROTOCOL.md §3): DT1 write, addr 04 00 52 21, data 01.
    #[test]
    fn checksum_golden_g1() {
        assert_eq!(roland_checksum(&[0x04, 0x00, 0x52, 0x21, 0x01]), 0x08);
    }

    // Golden vector G2: RQ1 read, addr 04 02 11 0D, size 00 00 00 01.
    #[test]
    fn checksum_golden_g2() {
        assert_eq!(
            roland_checksum(&[0x04, 0x02, 0x11, 0x0D, 0x00, 0x00, 0x00, 0x01]),
            0x5B
        );
    }

    #[test]
    fn checksum_zero_remainder_is_zero() {
        assert_eq!(roland_checksum(&[0x00]), 0x00);
    }
}
