//! OS adapters. Only thin wrappers live here; decisions belong to the core.

#[cfg(target_os = "macos")]
pub mod macos;

use crate::sensors::probe::Probe;

/// The probe for the current platform.
pub fn default_probe() -> Box<dyn Probe> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacProbe)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(crate::sensors::probe::NullProbe)
    }
}
