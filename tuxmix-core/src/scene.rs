use serde::{Deserialize, Serialize};

use crate::channel::{InputChannel, OutputChannel, PlaybackChannel};
use crate::device::DeviceSettings;
use crate::error::Error;

/// A snapshot of the full device state, serializable for
/// save/restore (scenes / presets).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Name of the scene (user-defined).
    pub name: String,

    /// The `RmeDevice::model_name()` this scene was captured from.
    /// Empty = legacy scene, captured before this field existed —
    /// treated as "unknown, skip the compatibility check" rather than
    /// a hard mismatch. Applying a scene by blind positional index to
    /// a different model's channel layout would silently write wrong
    /// values, so this is checked before `apply_scene` mutates anything.
    #[serde(default)]
    pub model: String,

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
            model: String::new(),
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

    /// Returns an error if this scene was captured on a different
    /// model than `device_model`. A blank `self.model` (legacy scene,
    /// or one built via `Scene::new`) is always treated as compatible.
    pub fn check_compatible(&self, device_model: &str) -> Result<(), Error> {
        if !self.model.is_empty() && self.model != device_model {
            return Err(Error::SceneModelMismatch {
                scene_model: self.model.clone(),
                device_model: device_model.to_string(),
            });
        }
        Ok(())
    }
}
