//! Mock implementation of [`RmeDevice`] for development and testing.
//!
//! [`MockBabyfacePro`] simulates a Babyface Pro FS entirely in memory,
//! without any ALSA or hardware interaction.
//!
//! Use `--mock` to run the TUI/GUI without a physical device:
//! ```bash
//! cargo run -p tinyface-gui -- --mock
//! ```

use rand::Rng;

use crate::channel::*;
use crate::device::{DeviceSettings, RmeDevice};
use crate::error::Error;
use crate::scene::Scene;

// ── Constants (mirrors babyface.rs) ──────────────────────────────

const MODEL_NAME: &str = "Babyface Pro FS (mock)";
const OUTPUT_PAIRS: usize = 6;
const INPUT_COUNT: usize = 12;

const INPUT_NAMES: [&str; INPUT_COUNT] = [
    "AN1", "AN2", "IN3", "IN4", "AS1", "AS2", "ADAT3", "ADAT4", "ADAT5", "ADAT6", "ADAT7", "ADAT8",
];

const INPUT_TYPES: [ChannelType; INPUT_COUNT] = [
    ChannelType::Mic,
    ChannelType::Mic,
    ChannelType::Instrument,
    ChannelType::Instrument,
    ChannelType::Line,
    ChannelType::Line,
    ChannelType::ADAT,
    ChannelType::ADAT,
    ChannelType::ADAT,
    ChannelType::ADAT,
    ChannelType::ADAT,
    ChannelType::ADAT,
];

const OUTPUT_NAMES: [(&str, &str); OUTPUT_PAIRS] = [
    ("AN1", "AN2"),
    ("PH3", "PH4"),
    ("AS1", "AS2"),
    ("ADAT3", "ADAT4"),
    ("ADAT5", "ADAT6"),
    ("ADAT7", "ADAT8"),
];

// ── Main struct ─────────────────────────────────────────────────

/// A simulated Babyface Pro FS that works without any hardware.
///
/// All state is kept in memory. Every operation succeeds immediately.
/// Use this to develop or test UI code without a physical RME device.
pub struct MockBabyfacePro {
    inputs: Vec<InputChannel>,
    playbacks: Vec<PlaybackChannel>,
    settings: DeviceSettings,
    /// Simulated VU levels for each input channel (0.0 – 1.0).
    input_meters: Vec<f32>,
    /// Simulated VU levels for each playback channel (0.0 – 1.0).
    playback_meters: Vec<f32>,
    tick: u64,
}

impl MockBabyfacePro {
    fn build_inputs() -> Vec<InputChannel> {
        INPUT_NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mut ch = InputChannel::new(i, name, INPUT_TYPES[i], OUTPUT_PAIRS);
                ch.phantom = i < 2;
                ch.sensitivity = if INPUT_TYPES[i] == ChannelType::Instrument {
                    Some(Sensitivity::Plus4dBu)
                } else {
                    None
                };
                ch
            })
            .collect()
    }

    fn build_playbacks() -> Vec<PlaybackChannel> {
        let mut pbs = Vec::new();
        for (i, (l, _r)) in OUTPUT_NAMES.iter().enumerate() {
            pbs.push(PlaybackChannel::new(
                i * 2,
                &format!("PCM {}", l),
                OUTPUT_PAIRS,
            ));
            pbs.push(PlaybackChannel::new(
                i * 2 + 1,
                &format!("PCM {}", _r),
                OUTPUT_PAIRS,
            ));
        }
        pbs
    }

    fn update_meters(&mut self) {
        let mut rng = rand::thread_rng();
        self.tick += 1;

        for v in &mut self.input_meters {
            let target = rng.gen_range(0.0..0.95);
            *v += (target - *v) * 0.05;
            *v = v.clamp(0.0, 1.0);
        }
        for v in &mut self.playback_meters {
            let target = rng.gen_range(0.0..0.85);
            *v += (target - *v) * 0.03;
            *v = v.clamp(0.0, 1.0);
        }
    }

    pub fn input_meter(&self, idx: usize) -> f32 {
        self.input_meters.get(idx).copied().unwrap_or(0.0)
    }

    pub fn playback_meter(&self, idx: usize) -> f32 {
        self.playback_meters.get(idx).copied().unwrap_or(0.0)
    }

    pub fn input_meters(&self) -> &[f32] {
        &self.input_meters
    }

    pub fn playback_meters(&self) -> &[f32] {
        &self.playback_meters
    }
}

impl RmeDevice for MockBabyfacePro {
    fn model_name(&self) -> &str {
        MODEL_NAME
    }

    fn output_pair_count(&self) -> usize {
        OUTPUT_PAIRS
    }

    fn open() -> Result<Self, Error> {
        Ok(Self {
            inputs: Self::build_inputs(),
            playbacks: Self::build_playbacks(),
            settings: DeviceSettings {
                clock_source: "Internal".into(),
                spdif_optical: false,
                spdif_emphasis: false,
                spdif_professional: false,
            },
            input_meters: vec![0.0; INPUT_COUNT],
            playback_meters: vec![0.0; OUTPUT_PAIRS * 2],
            tick: 0,
        })
    }

    fn inputs(&self) -> &[InputChannel] {
        &self.inputs
    }

    fn inputs_mut(&mut self) -> &mut [InputChannel] {
        &mut self.inputs
    }

    fn playbacks(&self) -> &[PlaybackChannel] {
        &self.playbacks
    }

    fn playbacks_mut(&mut self) -> &mut [PlaybackChannel] {
        &mut self.playbacks
    }

    fn settings(&self) -> &DeviceSettings {
        &self.settings
    }

    fn settings_mut(&mut self) -> &mut DeviceSettings {
        &mut self.settings
    }

    fn set_volume(&mut self, channel: ChannelId, output: usize, volume: f32) -> Result<(), Error> {
        let vol = volume.clamp(0.0, 1.0);
        match channel {
            ChannelId::Input(idx) => {
                let ch = self
                    .inputs
                    .get_mut(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx)))?;
                if output >= ch.volumes.len() {
                    return Err(Error::InvalidChannel(format!("Output {}", output)));
                }
                ch.volumes[output] = vol;
            }
            ChannelId::Playback(idx) => {
                let ch = self
                    .playbacks
                    .get_mut(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx)))?;
                if output >= ch.volumes.len() {
                    return Err(Error::InvalidChannel(format!("Output {}", output)));
                }
                ch.volumes[output] = vol;
            }
        }
        Ok(())
    }

    fn volume(&self, channel: ChannelId, output: usize) -> Result<f32, Error> {
        match channel {
            ChannelId::Input(idx) => {
                let ch = self
                    .inputs
                    .get(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx)))?;
                ch.volumes
                    .get(output)
                    .copied()
                    .ok_or_else(|| Error::InvalidChannel(format!("Output {}", output)))
            }
            ChannelId::Playback(idx) => {
                let ch = self
                    .playbacks
                    .get(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx)))?;
                ch.volumes
                    .get(output)
                    .copied()
                    .ok_or_else(|| Error::InvalidChannel(format!("Output {}", output)))
            }
        }
    }

    fn set_pan(&mut self, channel: ChannelId, output: usize, pan: i8) -> Result<(), Error> {
        let pan = pan.clamp(-100, 100);
        match channel {
            ChannelId::Input(idx) => {
                let ch = self
                    .inputs
                    .get_mut(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx)))?;
                if output >= ch.pans.len() {
                    return Err(Error::InvalidChannel(format!("Output {}", output)));
                }
                ch.pans[output] = pan;
            }
            ChannelId::Playback(idx) => {
                let ch = self
                    .playbacks
                    .get_mut(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx)))?;
                if output >= ch.pans.len() {
                    return Err(Error::InvalidChannel(format!("Output {}", output)));
                }
                ch.pans[output] = pan;
            }
        }
        Ok(())
    }

    fn pan(&self, channel: ChannelId, output: usize) -> Result<i8, Error> {
        match channel {
            ChannelId::Input(idx) => {
                let ch = self
                    .inputs
                    .get(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx)))?;
                ch.pans
                    .get(output)
                    .copied()
                    .ok_or_else(|| Error::InvalidChannel(format!("Output {}", output)))
            }
            ChannelId::Playback(idx) => {
                let ch = self
                    .playbacks
                    .get(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx)))?;
                ch.pans
                    .get(output)
                    .copied()
                    .ok_or_else(|| Error::InvalidChannel(format!("Output {}", output)))
            }
        }
    }

    fn capture_scene(&self) -> Scene {
        Scene {
            name: "Untitled".into(),
            inputs: self.inputs.clone(),
            playbacks: self.playbacks.clone(),
            settings: self.settings.clone(),
        }
    }

    fn apply_scene(&mut self, scene: &Scene) -> Result<(), Error> {
        self.inputs = scene.inputs.clone();
        self.playbacks = scene.playbacks.clone();
        self.settings = scene.settings.clone();
        Ok(())
    }

    fn poll_events(&mut self) -> Result<(), Error> {
        self.update_meters();
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_creates_correct_number_of_channels() {
        let dev = MockBabyfacePro::open().unwrap();
        assert_eq!(dev.inputs().len(), 12);
        assert_eq!(dev.playbacks().len(), 12);
    }

    #[test]
    fn test_output_pair_count() {
        let dev = MockBabyfacePro::open().unwrap();
        assert_eq!(dev.output_pair_count(), 6);
    }

    #[test]
    fn test_each_channel_has_per_output_volumes() {
        let dev = MockBabyfacePro::open().unwrap();
        for ch in dev.inputs() {
            assert_eq!(ch.volumes.len(), 6);
            assert_eq!(ch.pans.len(), 6);
        }
        for ch in dev.playbacks() {
            assert_eq!(ch.volumes.len(), 6);
            assert_eq!(ch.pans.len(), 6);
        }
    }

    #[test]
    fn test_per_output_volume() {
        let mut dev = MockBabyfacePro::open().unwrap();
        // Set volume for input 0 towards output 2 only
        dev.set_volume(ChannelId::Input(0), 2, 0.3).unwrap();
        assert!((dev.volume(ChannelId::Input(0), 2).unwrap() - 0.3).abs() < 1e-6);
        // Other outputs should be unchanged (default 0.75)
        assert!((dev.volume(ChannelId::Input(0), 0).unwrap() - 0.75).abs() < 1e-6);
        assert!((dev.volume(ChannelId::Input(0), 5).unwrap() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_per_output_pan() {
        let mut dev = MockBabyfacePro::open().unwrap();
        dev.set_pan(ChannelId::Playback(0), 3, 50).unwrap();
        assert_eq!(dev.pan(ChannelId::Playback(0), 3).unwrap(), 50);
        assert_eq!(dev.pan(ChannelId::Playback(0), 0).unwrap(), 0);
    }

    #[test]
    fn test_volume_clamps_to_range() {
        let mut dev = MockBabyfacePro::open().unwrap();
        dev.set_volume(ChannelId::Input(0), 0, 1.5).unwrap();
        assert!((dev.volume(ChannelId::Input(0), 0).unwrap() - 1.0).abs() < 1e-6);
        dev.set_volume(ChannelId::Input(0), 0, -0.5).unwrap();
        assert!((dev.volume(ChannelId::Input(0), 0).unwrap() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_invalid_channel_returns_error() {
        let mut dev = MockBabyfacePro::open().unwrap();
        let result = dev.set_volume(ChannelId::Input(99), 0, 0.5);
        assert!(result.is_err());
        let result = dev.set_volume(ChannelId::Input(0), 99, 0.5);
        assert!(result.is_err());
    }

    #[test]
    fn test_capture_and_apply_scene() {
        let mut dev = MockBabyfacePro::open().unwrap();
        dev.set_volume(ChannelId::Input(0), 0, 0.25).unwrap();
        dev.set_pan(ChannelId::Playback(0), 1, 50).unwrap();

        let scene = dev.capture_scene();
        assert!((scene.inputs[0].volumes[0] - 0.25).abs() < 1e-6);
        assert_eq!(scene.playbacks[0].pans[1], 50);

        let mut dev2 = MockBabyfacePro::open().unwrap();
        dev2.apply_scene(&scene).unwrap();
        assert!((dev2.volume(ChannelId::Input(0), 0).unwrap() - 0.25).abs() < 1e-6);
        assert_eq!(dev2.pan(ChannelId::Playback(0), 1).unwrap(), 50);
    }

    #[test]
    fn test_poll_events_updates_meters() {
        let mut dev = MockBabyfacePro::open().unwrap();
        for v in dev.input_meters() {
            assert_eq!(*v, 0.0);
        }
        dev.poll_events().unwrap();
        let has_movement = dev.input_meters().iter().any(|v| *v > 0.0);
        assert!(has_movement);
    }

    #[test]
    fn test_meter_ranges() {
        let mut dev = MockBabyfacePro::open().unwrap();
        for _ in 0..100 {
            dev.poll_events().unwrap();
        }
        for v in dev.input_meters() {
            assert!(*v >= 0.0 && *v <= 1.0, "Meter {} out of range", v);
        }
        for v in dev.playback_meters() {
            assert!(*v >= 0.0 && *v <= 1.0, "Meter {} out of range", v);
        }
    }

    #[test]
    fn test_serialize_scene_to_json() {
        let dev = MockBabyfacePro::open().unwrap();
        let scene = dev.capture_scene();
        let json = scene.to_json().unwrap();
        let restored = Scene::from_json(&json).unwrap();
        assert_eq!(restored.inputs.len(), scene.inputs.len());
    }

    #[test]
    fn test_first_inputs_have_phantom() {
        let dev = MockBabyfacePro::open().unwrap();
        assert!(dev.inputs()[0].phantom);
        assert!(dev.inputs()[1].phantom);
        assert!(!dev.inputs()[2].phantom);
    }
}
