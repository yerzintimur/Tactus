//! The module's persistent memory.

use std::collections::HashMap;

/// A module's parameter memory: a **flat, byte-addressed** space, keyed by the
/// 28-bit linear form of Roland's 4-byte address.
///
/// This mirrors how a real module behaves — it doesn't "understand" tempo or kit
/// names, it stores the bytes a DT1 wrote and returns whatever bytes lie in the
/// range an RQ1 asks for. Flat rather than per-parameter cells, because a request
/// need not line up with a parameter: reading a whole set list in one RQ1 (33
/// values in one reply) is exactly how the app avoids a burst of requests, and a
/// cell-keyed store could not answer it. Bytes never written read back as 0, like
/// a factory-blank slot. Writes persist for the lifetime of the device, matching
/// the V31's no-separate-save behaviour (PROTOCOL §7).
#[derive(Debug, Clone, Default)]
pub struct DeviceState {
    bytes: HashMap<u32, u8>,
}

impl DeviceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `data` starting at `address`.
    pub fn write(&mut self, address: [u8; 4], data: Vec<u8>) {
        let at = sysex::address::to_linear(address);
        for (i, byte) in data.into_iter().enumerate() {
            self.bytes.insert(at.wrapping_add(i as u32), byte);
        }
    }

    /// The `len` bytes starting at `address`; never-written bytes read as 0.
    pub fn read(&self, address: [u8; 4], len: usize) -> Vec<u8> {
        let at = sysex::address::to_linear(address);
        (0..len)
            .map(|i| {
                self.bytes
                    .get(&at.wrapping_add(i as u32))
                    .copied()
                    .unwrap_or(0)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_read_spans_whatever_was_written_inside_it() {
        let mut state = DeviceState::new();
        state.write([0x03, 0, 0, 0], vec![1, 2]);
        state.write([0x03, 0, 0, 4], vec![9]);
        // One request covering both writes and the gap between them.
        assert_eq!(state.read([0x03, 0, 0, 0], 6), vec![1, 2, 0, 0, 9, 0]);
    }

    #[test]
    fn reads_carry_across_the_seven_bit_byte_boundary() {
        let mut state = DeviceState::new();
        state.write([0, 0, 0, 0x7F], vec![7, 8]);
        assert_eq!(state.read([0, 0, 0, 0x7F], 1), vec![7]);
        assert_eq!(state.read([0, 0, 1, 0x00], 1), vec![8]);
    }
}
