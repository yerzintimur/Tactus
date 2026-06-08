//! The Roland SysEx checksum.

/// Roland checksum over the address+data byte slice (everything between the
/// command byte and the trailing checksum, exclusive of `F7`).
///
/// `checksum = (128 - (sum(bytes) mod 128)) mod 128`.
///
/// # Examples
/// ```
/// use sysex::roland_checksum;
/// // Golden vector G1 (docs/PROTOCOL.md §3): address 04 00 52 21 + data 01.
/// assert_eq!(roland_checksum(&[0x04, 0x00, 0x52, 0x21, 0x01]), 0x08);
/// ```
pub fn roland_checksum(addr_and_data: &[u8]) -> u8 {
    let sum: u32 = addr_and_data.iter().map(|&b| b as u32).sum();
    ((128 - (sum % 128)) % 128) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    // Golden vectors from docs/PROTOCOL.md §3.
    #[test]
    fn golden_g1() {
        assert_eq!(roland_checksum(&[0x04, 0x00, 0x52, 0x21, 0x01]), 0x08);
    }

    #[test]
    fn golden_g2() {
        assert_eq!(
            roland_checksum(&[0x04, 0x02, 0x11, 0x0D, 0x00, 0x00, 0x00, 0x01]),
            0x5B
        );
    }

    #[test]
    fn zero_remainder_maps_to_zero() {
        assert_eq!(roland_checksum(&[0x00]), 0x00);
        assert_eq!(roland_checksum(&[]), 0x00);
    }
}
