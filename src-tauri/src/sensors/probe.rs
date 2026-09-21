/// The only facts about an application that ever leave the OS boundary.
/// No window titles, no documents, no URLs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    pub bundle_id: String,
    pub name: String,
}

/// OS access, abstracted so the collection logic is testable with scripted fakes.
pub trait Probe: Send {
    /// The frontmost application, if any.
    fn frontmost_app(&self) -> Option<AppInfo>;
    /// Seconds since the last keyboard/mouse event. Only the *elapsed time* is
    /// read — never which keys or where the pointer was.
    fn idle_seconds(&self) -> f64;
}

/// Reports nothing. Used on platforms without an adapter yet.
pub struct NullProbe;

impl Probe for NullProbe {
    fn frontmost_app(&self) -> Option<AppInfo> {
        None
    }
    fn idle_seconds(&self) -> f64 {
        0.0
    }
}
