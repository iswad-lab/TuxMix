use crate::channel::{ChannelId, InputChannel, PlaybackChannel};
use crate::error::Error;
use crate::scene::Scene;

/// Global device-level settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceSettings {
    /// Current clock source (e.g. "Internal", "SPDIF", "ADAT").
    pub clock_source: String,
    /// SPDIF optical mode (true = optical, false = coaxial).
    pub spdif_optical: bool,
    /// SPDIF emphasis.
    pub spdif_emphasis: bool,
    /// SPDIF professional flag.
    pub spdif_professional: bool,
}

/// A generic RME audio interface.
///
/// Each implementation maps to a specific hardware model and knows
/// how to discover and control its ALSA mixer elements.
pub trait RmeDevice {
    /// Human-readable model name (e.g. "Babyface Pro FS").
    fn model_name(&self) -> &str;

    /// Attempt to detect the device on the ALSA bus and open a mixer handle.
    fn open() -> Result<Self, Error>
    where
        Self: Sized;

    /// Returns a reference to all hardware input channels.
    fn inputs(&self) -> &[InputChannel];

    /// Returns a mutable reference to all hardware input channels.
    fn inputs_mut(&mut self) -> &mut [InputChannel];

    /// Returns a reference to all software playback channels.
    fn playbacks(&self) -> &[PlaybackChannel];

    /// Returns a mutable reference to all software playback channels.
    fn playbacks_mut(&mut self) -> &mut [PlaybackChannel];

    /// Returns the current global device settings.
    fn settings(&self) -> &DeviceSettings;

    /// Returns a mutable reference to the global device settings.
    fn settings_mut(&mut self) -> &mut DeviceSettings;

    // ── Control operations ──────────────────────────────────────

    /// Set the volume (0.0 – 1.0) for a given channel.
    fn set_volume(&mut self, channel: ChannelId, volume: f32) -> Result<(), Error>;

    /// Get the volume (0.0 – 1.0) for a given channel.
    fn volume(&self, channel: ChannelId) -> Result<f32, Error>;

    /// Set the pan (-100 .. 100) for a given channel.
    fn set_pan(&mut self, channel: ChannelId, pan: i8) -> Result<(), Error>;

    /// Get the pan (-100 .. 100) for a given channel.
    fn pan(&self, channel: ChannelId) -> Result<i8, Error>;

    /// Route the given channel to a physical hardware output.
    fn set_routing(&mut self, channel: ChannelId, output: usize) -> Result<(), Error>;

    /// Get the current routing destination for a channel.
    fn routing(&self, channel: ChannelId) -> Result<usize, Error>;

    // ── Scene / snapshot ────────────────────────────────────────

    /// Read the full hardware state into a [`Scene`].
    fn capture_scene(&self) -> Scene;

    /// Apply a previously captured [`Scene`] to the hardware.
    fn apply_scene(&mut self, scene: &Scene) -> Result<(), Error>;

    // ── Polling ─────────────────────────────────────────────────

    /// Process pending ALSA events (e.g. hardware state changes).
    /// Should be called periodically from the UI event loop.
    fn poll_events(&mut self) -> Result<(), Error>;
}
