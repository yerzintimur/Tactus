//! Device profiles: everything specific to a particular Roland module lives here
//! as *data* (`DeviceProfile`), never hardcoded in logic. The registry resolves a
//! profile from the module's Identity Reply (Model ID); firmware compatibility is
//! detected and reported but never blocks.
//!
//! See ADR-0007 (profiles), ADR-0009 (firmware), ADR-0010 (instances) and
//! docs/DEVELOPMENT.md §3.
#![forbid(unsafe_code)]

pub mod firmware;
mod profile;
mod registry;

pub use firmware::{FirmwareSupport, FirmwareVersion};
pub use profile::{
    AreaDef, Capabilities, DeviceProfile, FirmwareConfig, Identity, ParameterDef, ValueRange,
};
pub use registry::ProfileRegistry;

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Profile-document schema version this build understands.
pub const SCHEMA_VERSION: u32 = 1;
