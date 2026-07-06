//! Babyface Pro (FS) implementation of the [`RmeDevice`] trait.

use alsa::mixer::SelemChannelId;
use log::info;

use crate::channel::*;
use crate::device::{DeviceSettings, RmeDevice};
use crate::error::Error;
use crate::mixer::AlsaMixer;
use crate::scene::Scene;

// ── Constants ──────────────────────────────────────────────────

/// ALSA card name substring used for detection.
const CARD_SUBSTRING: &str = "Babyface Pro";

/// Number of hardware input channels.
const INPUT_COUNT: usize = 12;

/// Number of stereo output pairs.
const OUTPUT_PAIRS: usize = 6;

/// Names of the hardware input channels.
const INPUT_NAMES: [&str; INPUT_COUNT] = [
    "AN1", "AN2", "IN3", "IN4", "AS1", "AS2", "ADAT3", "ADAT4", "ADAT5", "ADAT6", "ADAT7", "ADAT8",
];

/// Types of each input channel.
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

/// Stereo output pair names.
const OUTPUT_NAMES: [(&str, &str); OUTPUT_PAIRS] = [
    ("AN1", "AN2"),
    ("PH3", "PH4"),
    ("AS1", "AS2"),
    ("ADAT3", "ADAT4"),
    ("ADAT5", "ADAT6"),
    ("ADAT7", "ADAT8"),
];

// ── Helpers ────────────────────────────────────────────────────

fn selem_name(ch_type: &str, ch_name: &str, out_name: &str) -> String {
    format!("{}-{}-{}", ch_type, ch_name, out_name)
}

fn ch_type_str(ct: ChannelType) -> &'static str {
    match ct {
        ChannelType::Mic => "Mic",
        ChannelType::Instrument => "Line",
        ChannelType::Line | ChannelType::SPDIF | ChannelType::ADAT => "Line",
    }
}

// ── Main struct ────────────────────────────────────────────────

/// Babyface Pro (FS) device controller.
pub struct BabyfacePro {
    mixer: AlsaMixer,
    inputs: Vec<InputChannel>,
    playbacks: Vec<PlaybackChannel>,
    settings: DeviceSettings,
}

impl BabyfacePro {
    fn build_inputs() -> Vec<InputChannel> {
        INPUT_NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| InputChannel {
                id: i,
                name: name.to_string(),
                channel_type: INPUT_TYPES[i],
                volume: 0.8,
                pan: 0,
                phantom: false,
                pad: false,
                sensitivity: if INPUT_TYPES[i] == ChannelType::Instrument {
                    Some(Sensitivity::Plus4dBu)
                } else {
                    None
                },
                routing: 0,
            })
            .collect()
    }

    fn build_playbacks() -> Vec<PlaybackChannel> {
        let mut pbs = Vec::new();
        for (i, (l, r)) in OUTPUT_NAMES.iter().enumerate() {
            pbs.push(PlaybackChannel {
                id: i * 2,
                name: format!("PCM {}", l),
                volume: 0.8,
                pan: 0,
                routing: i,
            });
            pbs.push(PlaybackChannel {
                id: i * 2 + 1,
                name: format!("PCM {}", r),
                volume: 0.8,
                pan: 0,
                routing: i,
            });
        }
        pbs
    }

    /// Match ALSA mixer elements to our channel model.
    fn attach_mixer_elements(&mut self) {
        let mono = SelemChannelId::mono();

        for (name, selem) in self.mixer.iter_selems() {
            // ── Global: Clock Mode ──────────────────────────
            if name == "Clock Mode" {
                if let Ok(current) = selem.get_enum_item(mono) {
                    self.settings.clock_source = format!("Mode {}", current);
                }
                continue;
            }

            // ── Phantom 48V & PAD for Mic inputs ────────────
            for i in 0..self.inputs.len() {
                if self.inputs[i].channel_type != ChannelType::Mic {
                    continue;
                }
                let expected_48v = format!("Mic-{} 48V", self.inputs[i].name);
                if name == expected_48v {
                    if let Ok(v) = selem.get_playback_switch(mono) {
                        self.inputs[i].phantom = v != 0;
                    }
                    break;
                }

                let expected_pad = format!("Mic-{} PAD", self.inputs[i].name);
                if name == expected_pad {
                    if let Ok(v) = selem.get_playback_switch(mono) {
                        self.inputs[i].pad = v != 0;
                    }
                    break;
                }
            }

            // ── Sensitivity for Instrument inputs ───────────
            for i in 0..self.inputs.len() {
                if self.inputs[i].channel_type != ChannelType::Instrument {
                    continue;
                }
                let expected_sens = format!("Line-{} Sens.", self.inputs[i].name);
                if name == expected_sens {
                    if let Ok(item) = selem.get_enum_item(mono) {
                        self.inputs[i].sensitivity = Some(if item == 0 {
                            Sensitivity::Minus10dBV
                        } else {
                            Sensitivity::Plus4dBu
                        });
                    }
                    break;
                }
            }

            // ── Volume for Inputs ───────────────────────────
            for i in 0..self.inputs.len() {
                let ct = ch_type_str(self.inputs[i].channel_type);
                for out_idx in 0..OUTPUT_PAIRS {
                    let (out_l, out_r) = OUTPUT_NAMES[out_idx];
                    let expected_l = selem_name(ct, &self.inputs[i].name, out_l);
                    let expected_r = selem_name(ct, &self.inputs[i].name, out_r);
                    if name == expected_l || name == expected_r {
                        if let Ok(v) = selem.get_playback_volume(mono) {
                            self.inputs[i].volume = (v as f32) / 65536.0;
                        }
                        break;
                    }
                }
            }

            // ── Volume for Playbacks ────────────────────────
            for i in 0..self.playbacks.len() {
                let ch_name = &self.playbacks[i].name[4..]; // strip "PCM " prefix
                let expected_l =
                    selem_name("PCM", ch_name, OUTPUT_NAMES[self.playbacks[i].routing].0);
                let expected_r =
                    selem_name("PCM", ch_name, OUTPUT_NAMES[self.playbacks[i].routing].1);
                if name == expected_l || name == expected_r {
                    if let Ok(v) = selem.get_playback_volume(mono) {
                        self.playbacks[i].volume = (v as f32) / 65536.0;
                    }
                    break;
                }
            }
        }

        info!(
            "Attached {} inputs, {} playbacks, clock: {}",
            self.inputs.len(),
            self.playbacks.len(),
            self.settings.clock_source
        );
    }
}

impl RmeDevice for BabyfacePro {
    fn model_name(&self) -> &str {
        "Babyface Pro FS"
    }

    fn open() -> Result<Self, Error> {
        info!("Searching for RME Babyface Pro...");
        let mixer = AlsaMixer::open_by_card_name(CARD_SUBSTRING)?;
        let mut device = Self {
            mixer,
            inputs: Self::build_inputs(),
            playbacks: Self::build_playbacks(),
            settings: DeviceSettings {
                clock_source: "Internal".into(),
                spdif_optical: false,
                spdif_emphasis: false,
                spdif_professional: false,
            },
        };
        device.attach_mixer_elements();
        Ok(device)
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

    fn set_volume(&mut self, channel: ChannelId, volume: f32) -> Result<(), Error> {
        let mono = SelemChannelId::mono();
        let vol_raw = (volume.clamp(0.0, 1.0) * 65536.0) as i64;
        let (ch_type, ch_name, routing) = match channel {
            ChannelId::Input(idx) => {
                let inp = &self.inputs[idx];
                (ch_type_str(inp.channel_type), inp.name.clone(), inp.routing)
            }
            ChannelId::Playback(idx) => {
                let pb = &self.playbacks[idx];
                let name = pb.name[4..].to_string(); // strip "PCM "
                ("PCM", name, pb.routing)
            }
        };

        let (out_l, out_r) = OUTPUT_NAMES[routing];
        for out_name in [out_l, out_r] {
            let elem_name = selem_name(ch_type, &ch_name, out_name);
            if let Some(selem) = self.mixer.find_selem(&elem_name, 0) {
                selem.set_playback_volume(mono, vol_raw)?;
            }
        }

        match channel {
            ChannelId::Input(idx) => self.inputs[idx].volume = volume,
            ChannelId::Playback(idx) => self.playbacks[idx].volume = volume,
        }
        Ok(())
    }

    fn volume(&self, channel: ChannelId) -> Result<f32, Error> {
        match channel {
            ChannelId::Input(idx) => Ok(self.inputs[idx].volume),
            ChannelId::Playback(idx) => Ok(self.playbacks[idx].volume),
        }
    }

    fn set_pan(&mut self, channel: ChannelId, pan: i8) -> Result<(), Error> {
        let pan = pan.clamp(-100, 100);
        match channel {
            ChannelId::Input(idx) => self.inputs[idx].pan = pan,
            ChannelId::Playback(idx) => self.playbacks[idx].pan = pan,
        }
        Ok(())
    }

    fn pan(&self, channel: ChannelId) -> Result<i8, Error> {
        match channel {
            ChannelId::Input(idx) => Ok(self.inputs[idx].pan),
            ChannelId::Playback(idx) => Ok(self.playbacks[idx].pan),
        }
    }

    fn set_routing(&mut self, channel: ChannelId, output: usize) -> Result<(), Error> {
        let output = output.min(OUTPUT_PAIRS - 1);
        match channel {
            ChannelId::Input(idx) => self.inputs[idx].routing = output,
            ChannelId::Playback(idx) => self.playbacks[idx].routing = output,
        }
        Ok(())
    }

    fn routing(&self, channel: ChannelId) -> Result<usize, Error> {
        match channel {
            ChannelId::Input(idx) => Ok(self.inputs[idx].routing),
            ChannelId::Playback(idx) => Ok(self.playbacks[idx].routing),
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
        let _ = self.mixer.handle_events()?;
        Ok(())
    }
}
