//! Device simulation for end-to-end tests: a profile-driven [`VirtualDevice`] that
//! speaks the same Roland SysEx as a real module (Identity Reply, RQ1 reads, DT1
//! writes, unsolicited hardware-edit pushes), plus a deterministic [`VirtualClock`]
//! and [`TimingProfile`] so reply latency and tick ordering are *modelled* rather
//! than discarded.
//!
//! The host state machine lives in `engine`; this crate knows nothing about it. It
//! deals only in raw bytes and a [`DeviceProfile`](device::DeviceProfile), which is
//! exactly what keeps it an honest stand-in for hardware — and lets the FFI expose
//! it (debug-only) without pulling the engine. The harness that couples a
//! `VirtualDevice` to an `engine::Session` lives in the `e2e` crate.
//!
//! See the device-mock plan and ADR-0008 (sans-I/O core).
#![forbid(unsafe_code)]

mod device;
mod state;
mod timing;

pub use device::{EditValue, VirtualDevice};
pub use state::DeviceState;
pub use timing::{TimingProfile, VirtualClock};

/// Crate version, exposed so higher layers can sanity-check linkage.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
