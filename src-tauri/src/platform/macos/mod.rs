//! macOS probe. Neither call needs Accessibility or Screen Recording permission:
//! the frontmost app's identity comes from NSWorkspace, and idle time from the
//! elapsed-time counter of the session's event source — no event content.

use crate::sensors::probe::{AppInfo, Probe};
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;

pub struct MacProbe;

// CoreGraphics: seconds since the last input event of any type.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
}
const COMBINED_SESSION_STATE: i32 = 0; // kCGEventSourceStateCombinedSessionState
const ANY_INPUT_EVENT_TYPE: u32 = u32::MAX; // kCGAnyInputEventType

impl Probe for MacProbe {
    fn frontmost_app(&self) -> Option<AppInfo> {
        autoreleasepool(|_| {
            let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
            let name = app.localizedName().map(|s| s.to_string());
            // Unbundled processes (e.g. a dev binary) have no bundle id; fall back to the name.
            let bundle_id = app
                .bundleIdentifier()
                .map(|s| s.to_string())
                .or_else(|| name.as_ref().map(|n| format!("name:{n}")))?;
            Some(AppInfo {
                name: name.unwrap_or_else(|| bundle_id.clone()),
                bundle_id,
            })
        })
    }

    fn idle_seconds(&self) -> f64 {
        // SAFETY: plain C function taking two integers; no pointers, no preconditions.
        unsafe {
            CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT_EVENT_TYPE)
        }
    }
}
