//! UniFFI public API surface for the shared core (Session, callback interfaces,
//! records/enums). Kept thin — it wraps `engine`. Built as `libdrumcore`.
//!
//! See docs/DEVELOPMENT.md §6 (FFI contract). UniFFI scaffolding is added in the
//! "Define the UniFFI public API" task.
#![forbid(unsafe_code)]

/// Crate version, exposed for a binding smoke test.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn links_against_engine() {
        assert!(!engine::VERSION.is_empty());
    }
}
