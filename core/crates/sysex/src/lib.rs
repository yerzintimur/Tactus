//! Device-agnostic Roland SysEx mechanics: RQ1/DT1 framing, checksum, and message
//! parsing. No I/O, no module specifics — the active device's Model ID is passed
//! in by the caller (the engine, which knows the loaded profile).
//!
//! Module layout (decomposed by concern; tests live next to the code they cover):
//! - [`checksum`] — the Roland checksum.
//! - [`message`] — building RQ1/DT1/Identity requests and parsing inbound messages.
//!
//! Black-box contract tests (the spec's golden vectors) are in `tests/`.
//!
//! See docs/PROTOCOL.md (derived facts + golden vectors) and
//! docs/DEVELOPMENT.md §4.1.
#![forbid(unsafe_code)]

mod checksum;
mod message;

pub use checksum::roland_checksum;
pub use message::{ParseError, SysexMessage, build_dt1, build_identity_request, build_rq1, parse};

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
