//! Domain model + localization: turns device-profile parameters and raw values
//! into localizable, spoken/UI [`Message`]s, and defines user [`Intent`]s.
//!
//! Localization is done here, in the core, via Fluent — one tested source of
//! phrasing for both platforms (ADR-0008). See docs/DEVELOPMENT.md §4.3, §8.
#![forbid(unsafe_code)]

mod catalog;
mod format;
mod i18n;
mod intent;

pub use catalog::InstrumentCatalog;
pub use format::{format_kit, format_parameter, format_parameter_label};
pub use i18n::{Arg, Localizer, Message};
pub use intent::Intent;

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
