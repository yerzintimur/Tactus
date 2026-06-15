//! The module's persistent memory.

use std::collections::HashMap;

/// A module's parameter memory: raw bytes keyed by their absolute 4-byte address.
///
/// This mirrors how a real module behaves — it doesn't "understand" tempo or kit
/// names, it stores the bytes a DT1 wrote and returns them verbatim on an RQ1.
/// Keeping the simulator a dumb byte store (rather than re-implementing every
/// parameter's semantics) is what makes it profile-driven: a new device works by
/// adding profile data, with no code change here. Writes persist for the lifetime
/// of the device, matching the V31's no-separate-save behaviour (PROTOCOL §7).
#[derive(Debug, Clone, Default)]
pub struct DeviceState {
    cells: HashMap<[u8; 4], Vec<u8>>,
}

impl DeviceState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store `data` at `address` (overwriting any previous value).
    pub fn write(&mut self, address: [u8; 4], data: Vec<u8>) {
        self.cells.insert(address, data);
    }

    /// The bytes stored at `address`, if any has ever been written/seeded there.
    pub fn read(&self, address: [u8; 4]) -> Option<&[u8]> {
        self.cells.get(&address).map(Vec::as_slice)
    }
}
