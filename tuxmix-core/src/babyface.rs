//! Babyface Pro (FS) implementation of the [`RmeDevice`] trait.

use alsa::mixer::SelemChannelId;
use log::info;

use crate::channel::OutputChannel;
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
    outputs: Vec<OutputChannel>,
    settings: DeviceSettings,
}

impl BabyfacePro {
    fn build_inputs() -> Vec<InputChannel> {
        INPUT_NAMES
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let mut ch = InputChannel::new(i, name, INPUT_TYPES[i], OUTPUT_PAIRS);
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
        for (i, (l, r)) in OUTPUT_NAMES.iter().enumerate() {
            pbs.push(PlaybackChannel::new(
                i * 2,
                &format!("PCM {}", l),
                OUTPUT_PAIRS,
            ));
            pbs.push(PlaybackChannel::new(
                i * 2 + 1,
                &format!("PCM {}", r),
                OUTPUT_PAIRS,
            ));
        }
        pbs
    }

    fn build_outputs() -> Vec<OutputChannel> {
        let mut outs = Vec::new();
        for (i, (l, _r)) in OUTPUT_NAMES.iter().enumerate() {
            outs.push(OutputChannel::new(i * 2, &format!("OUT {}", l)));
            outs.push(OutputChannel::new(i * 2 + 1, &format!("OUT {}", _r)));
        }
        outs
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

            // ── Per-output volumes ──────────────────────────
            for i in 0..self.inputs.len() {
                let ct = ch_type_str(self.inputs[i].channel_type);
                for out_idx in 0..OUTPUT_PAIRS {
                    let (out_l, out_r) = OUTPUT_NAMES[out_idx];
                    let expected_l = selem_name(ct, &self.inputs[i].name, out_l);
                    let expected_r = selem_name(ct, &self.inputs[i].name, out_r);
                    if name == expected_l || name == expected_r {
                        if let Ok(v) = selem.get_playback_volume(mono) {
                            self.inputs[i].volumes[out_idx] = (v as f32) / 65536.0;
                        }
                        break;
                    }
                }
            }

            // ── Per-output volumes for playbacks ────────────
            for i in 0..self.playbacks.len() {
                let ch_name = &self.playbacks[i].name[4..]; // strip "PCM "
                for out_idx in 0..OUTPUT_PAIRS {
                    let (out_l, out_r) = OUTPUT_NAMES[out_idx];
                    let expected_l = selem_name("PCM", ch_name, out_l);
                    let expected_r = selem_name("PCM", ch_name, out_r);
                    if name == expected_l || name == expected_r {
                        if let Ok(v) = selem.get_playback_volume(mono) {
                            self.playbacks[i].volumes[out_idx] = (v as f32) / 65536.0;
                        }
                        break;
                    }
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

    fn channel(&self, ch: ChannelId) -> Result<(&bool, &bool), Error> {
        match ch {
            ChannelId::Input(idx) => self
                .inputs
                .get(idx)
                .map(|c| (&c.mute, &c.solo))
                .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx))),
            ChannelId::Playback(idx) => self
                .playbacks
                .get(idx)
                .map(|c| (&c.mute, &c.solo))
                .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx))),
            ChannelId::Output(idx) => self
                .outputs
                .get(idx)
                .map(|c| (&c.mute, &c.solo))
                .ok_or_else(|| Error::InvalidChannel(format!("Output {}", idx))),
        }
    }

    fn channel_mut(&mut self, ch: ChannelId) -> Result<(&mut bool, &mut bool), Error> {
        match ch {
            ChannelId::Input(idx) => self
                .inputs
                .get_mut(idx)
                .map(|c| (&mut c.mute, &mut c.solo))
                .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx))),
            ChannelId::Playback(idx) => self
                .playbacks
                .get_mut(idx)
                .map(|c| (&mut c.mute, &mut c.solo))
                .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx))),
            ChannelId::Output(idx) => self
                .outputs
                .get_mut(idx)
                .map(|c| (&mut c.mute, &mut c.solo))
                .ok_or_else(|| Error::InvalidChannel(format!("Output {}", idx))),
        }
    }
}

impl RmeDevice for BabyfacePro {
    fn model_name(&self) -> &str {
        "Babyface Pro FS"
    }

    fn output_pair_count(&self) -> usize {
        OUTPUT_PAIRS
    }

    fn open() -> Result<Self, Error> {
        info!("Searching for RME Babyface Pro...");
        let mixer = AlsaMixer::open_by_card_name(CARD_SUBSTRING)?;
        let mut device = Self {
            mixer,
            inputs: Self::build_inputs(),
            playbacks: Self::build_playbacks(),
            outputs: Self::build_outputs(),
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

    fn outputs(&self) -> &[OutputChannel] {
        &self.outputs
    }

    fn outputs_mut(&mut self) -> &mut [OutputChannel] {
        &mut self.outputs
    }

    fn settings(&self) -> &DeviceSettings {
        &self.settings
    }

    fn settings_mut(&mut self) -> &mut DeviceSettings {
        &mut self.settings
    }

    fn set_volume(&mut self, channel: ChannelId, output: usize, volume: f32) -> Result<(), Error> {
        if output >= OUTPUT_PAIRS {
            return Err(Error::InvalidChannel(format!("Output {}", output)));
        }

        let vol_clamped = volume.clamp(0.0, 1.0);
        let vol_raw = (vol_clamped * 65536.0) as i64;
        let mono = SelemChannelId::mono();

        let (ch_type, ch_name) = match channel {
            ChannelId::Input(idx) => {
                let inp = self
                    .inputs
                    .get(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Input {}", idx)))?;
                (ch_type_str(inp.channel_type), inp.name.clone())
            }
            ChannelId::Playback(idx) => {
                let pb = self
                    .playbacks
                    .get(idx)
                    .ok_or_else(|| Error::InvalidChannel(format!("Playback {}", idx)))?;
                ("PCM", pb.name[4..].to_string())
            }
            ChannelId::Output(_) => ("", String::new()),
        };

        if !ch_name.is_empty() {
            let (out_l, out_r) = OUTPUT_NAMES[output];
            for out_name in [out_l, out_r] {
                let elem_name = selem_name(ch_type, &ch_name, out_name);
                if let Some(selem) = self.mixer.find_selem(&elem_name, 0) {
                    selem.set_playback_volume(mono, vol_raw)?;
                }
            }
        }

        match channel {
            ChannelId::Input(idx) => self.inputs[idx].volumes[output] = vol_clamped,
            ChannelId::Playback(idx) => self.playbacks[idx].volumes[output] = vol_clamped,
            ChannelId::Output(idx) => self.outputs[idx].volume = vol_clamped,
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
            ChannelId::Output(idx) => self
                .outputs
                .get(idx)
                .map(|c| c.volume)
                .ok_or_else(|| Error::InvalidChannel(format!("Output {}", idx))),
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
            ChannelId::Output(_) => return Err(Error::InvalidChannel("Output has no pan".into())),
        }
        Ok(())
    }

    fn pan(&self, channel: ChannelId, output: usize) -> Result<i8, Error> {
        match channel {
            ChannelId::Input(idx) => self
                .inputs
                .get(idx)
                .and_then(|c| c.pans.get(output).copied())
                .ok_or_else(|| Error::InvalidChannel(format!("Channel {}", idx))),
            ChannelId::Playback(idx) => self
                .playbacks
                .get(idx)
                .and_then(|c| c.pans.get(output).copied())
                .ok_or_else(|| Error::InvalidChannel(format!("Channel {}", idx))),
            ChannelId::Output(_) => Err(Error::InvalidChannel("Output has no pan".into())),
        }
    }

    fn set_mute(&mut self, channel: ChannelId, mute: bool) -> Result<(), Error> {
        let ch = self.channel_mut(channel)?;
        *ch.0 = mute;
        Ok(())
    }

    fn mute(&self, channel: ChannelId) -> Result<bool, Error> {
        let ch = self.channel(channel)?;
        Ok(*ch.0)
    }

    fn set_solo(&mut self, channel: ChannelId, solo: bool) -> Result<(), Error> {
        let ch = self.channel_mut(channel)?;
        *ch.1 = solo;
        Ok(())
    }

    fn solo(&self, channel: ChannelId) -> Result<bool, Error> {
        let ch = self.channel(channel)?;
        Ok(*ch.1)
    }

    fn capture_scene(&self) -> Scene {
        Scene {
            name: "Untitled".into(),
            inputs: self.inputs.clone(),
            playbacks: self.playbacks.clone(),
            outputs: self.outputs.clone(),
            settings: self.settings.clone(),
        }
    }

    fn apply_scene(&mut self, scene: &Scene) -> Result<(), Error> {
        for (i, saved) in scene.inputs.iter().enumerate() {
            for (out, &v) in saved.volumes.iter().enumerate() {
                self.set_volume(ChannelId::Input(i), out, v)?;
            }
        }
        for (i, saved) in scene.playbacks.iter().enumerate() {
            for (out, &v) in saved.volumes.iter().enumerate() {
                self.set_volume(ChannelId::Playback(i), out, v)?;
            }
        }
        for (i, saved) in scene.outputs.iter().enumerate() {
            self.set_volume(ChannelId::Output(i), 0, saved.volume)?;
        }
        self.inputs = scene.inputs.clone();
        self.playbacks = scene.playbacks.clone();
        self.outputs = scene.outputs.clone();
        self.settings = scene.settings.clone();
        Ok(())
    }

    fn poll_events(&mut self) -> Result<(), Error> {
        let _ = self.mixer.handle_events()?;
        Ok(())
    }
}
