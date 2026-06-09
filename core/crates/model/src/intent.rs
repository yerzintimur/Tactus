//! User intents — domain commands the engine executes against the device.

/// A user-initiated action. The engine turns these into MIDI and the
/// write→readback→verify cycle (tasks #8/#9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Switch the active kit (0-based number).
    SelectKit { number: u32 },
    /// Set a parameter to `value` (raw, pre-encoding). `indices` selects the
    /// kit/pad/etc. for indexed areas.
    SetParameter {
        param_id: String,
        indices: Vec<u32>,
        value: i64,
    },
    /// Rename a kit.
    RenameKit { number: u32, name: String },
    /// Re-read the current state from the device.
    RefreshAll,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intents_construct_and_compare() {
        let a = Intent::SelectKit { number: 4 };
        assert_eq!(a, Intent::SelectKit { number: 4 });
        assert_ne!(a, Intent::SelectKit { number: 5 });
    }
}
