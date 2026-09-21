use serde::{Deserialize, Serialize};

/// A neutral local fact. Never an interpretation ("overworking", "anxious"...).
/// Timestamps are RFC 3339 strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivityEvent {
    ActiveApplication {
        timestamp: String,
        #[serde(rename = "bundleId")]
        bundle_id: String,
        #[serde(rename = "applicationName")]
        application_name: String,
    },
    Idle {
        timestamp: String,
        seconds: u32,
    },
    ApplicationSwitch {
        timestamp: String,
        #[serde(rename = "fromBundleId")]
        from_bundle_id: String,
        #[serde(rename = "toBundleId")]
        to_bundle_id: String,
    },
}

impl ActivityEvent {
    pub fn timestamp(&self) -> &str {
        match self {
            Self::ActiveApplication { timestamp, .. }
            | Self::Idle { timestamp, .. }
            | Self::ApplicationSwitch { timestamp, .. } => timestamp,
        }
    }
}
