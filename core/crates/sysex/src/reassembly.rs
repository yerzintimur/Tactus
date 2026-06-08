//! Reassembles complete SysEx messages (`F0 … F7`) from a stream of MIDI bytes
//! that may arrive fragmented across packets and interleaved with real-time
//! messages (`F8–FF`). See docs/PROTOCOL.md §1.

/// Default cap on an in-progress message — guards against a stray `F0` that never
/// gets an `F7`.
const DEFAULT_MAX_LEN: usize = 65_536;

/// Accumulates bytes and emits each complete `F0 … F7` message.
pub struct SysexReassembler {
    buf: Option<Vec<u8>>,
    max_len: usize,
}

impl Default for SysexReassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl SysexReassembler {
    pub fn new() -> Self {
        Self {
            buf: None,
            max_len: DEFAULT_MAX_LEN,
        }
    }

    /// Like [`new`](Self::new) but with a custom cap on an in-progress message.
    pub fn with_max_len(max_len: usize) -> Self {
        Self { buf: None, max_len }
    }

    /// Feed one byte; returns a complete message (including `F0`…`F7`) when one
    /// finishes, otherwise `None`.
    pub fn push(&mut self, byte: u8) -> Option<Vec<u8>> {
        match byte {
            0xF0 => {
                // Start (or restart, dropping any partial) a SysEx message.
                self.buf = Some(vec![0xF0]);
                None
            }
            0xF7 => {
                // End of Exclusive — emit the message if we were building one.
                let mut msg = self.buf.take()?;
                msg.push(0xF7);
                Some(msg)
            }
            0xF8..=0xFF => None, // real-time messages may interleave — ignore
            0x00..=0x7F => {
                if let Some(buf) = self.buf.as_mut() {
                    if buf.len() >= self.max_len {
                        self.buf = None; // overflow guard: abort this message
                    } else {
                        buf.push(byte);
                    }
                }
                None
            }
            // Any other status byte (80–EF, F1–F6) aborts an in-progress SysEx.
            _ => {
                self.buf = None;
                None
            }
        }
    }

    /// Feed many bytes; returns every message that completed within the slice.
    pub fn push_slice(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        for &b in bytes {
            if let Some(msg) = self.push(b) {
                out.push(msg);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn single_complete_message() {
        let mut r = SysexReassembler::new();
        assert_eq!(
            r.push_slice(&[0xF0, 0x41, 0x10, 0xF7]),
            vec![vec![0xF0, 0x41, 0x10, 0xF7]]
        );
    }

    #[test]
    fn fragmented_across_calls() {
        let mut r = SysexReassembler::new();
        assert!(r.push_slice(&[0xF0, 0x41]).is_empty());
        assert_eq!(
            r.push_slice(&[0x10, 0xF7]),
            vec![vec![0xF0, 0x41, 0x10, 0xF7]]
        );
    }

    #[test]
    fn two_messages_back_to_back() {
        let mut r = SysexReassembler::new();
        assert_eq!(
            r.push_slice(&[0xF0, 0x01, 0xF7, 0xF0, 0x02, 0xF7]),
            vec![vec![0xF0, 0x01, 0xF7], vec![0xF0, 0x02, 0xF7]]
        );
    }

    #[test]
    fn realtime_interleaved_is_ignored() {
        let mut r = SysexReassembler::new();
        // 0xF8 (timing clock) appears mid-stream and must not corrupt the message.
        assert_eq!(
            r.push_slice(&[0xF0, 0x01, 0xF8, 0x02, 0xF7]),
            vec![vec![0xF0, 0x01, 0x02, 0xF7]]
        );
    }

    #[test]
    fn leading_garbage_ignored() {
        let mut r = SysexReassembler::new();
        assert_eq!(
            r.push_slice(&[0x01, 0x02, 0xF0, 0x09, 0xF7]),
            vec![vec![0xF0, 0x09, 0xF7]]
        );
    }

    #[test]
    fn new_f0_drops_partial() {
        let mut r = SysexReassembler::new();
        assert_eq!(
            r.push_slice(&[0xF0, 0x01, 0xF0, 0x02, 0xF7]),
            vec![vec![0xF0, 0x02, 0xF7]]
        );
    }

    #[test]
    fn oversize_message_aborts() {
        let mut r = SysexReassembler::with_max_len(4);
        let mut data = vec![0xF0];
        data.extend_from_slice(&[0x01u8; 10]);
        data.push(0xF7);
        assert!(r.push_slice(&data).is_empty());
    }

    proptest! {
        #[test]
        fn never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let mut r = SysexReassembler::new();
            let _ = r.push_slice(&bytes);
        }
    }
}
