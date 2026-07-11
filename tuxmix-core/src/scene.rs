use serde::{Deserialize, Serialize};

use crate::channel::{InputChannel, OutputChannel, PlaybackChannel};
use crate::device::DeviceSettings;

/// A snapshot of the full device state, serializable for
/// save/restore (scenes / presets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Name of the scene (user-defined).
    pub name: String,

    /// Hardware input channels.
    pub inputs: Vec<InputChannel>,

    /// Software playback channels.
    pub playbacks: Vec<PlaybackChannel>,

    /// Physical output channels.
    pub outputs: Vec<OutputChannel>,

    /// Global device-level settings.
    pub settings: DeviceSettings,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            inputs: Vec::new(),
            playbacks: Vec::new(),
            outputs: Vec::new(),
            settings: DeviceSettings {
                clock_source: "Internal".into(),
                spdif_optical: false,
                spdif_emphasis: false,
                spdif_professional: false,
            },
        }
    }

    /// Serialize the scene to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a scene from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
